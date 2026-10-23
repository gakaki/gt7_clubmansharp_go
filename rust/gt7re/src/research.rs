//! DMSB + RT2 research pipeline for dealer descriptions and GT Cafe scenes.
//!
//! This is the native Rust counterpart of the Python car-text and exact-cafe
//! reconstruction passes. It never assigns a text row by file adjacency: car
//! keys are joined to `CAR_NAME_ALPHABET`, and Cafe scenes are traversed from
//! the exact message ids in `ConversationGTCafeMsg.dmsb`.

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::rt2::{self, Rt2File};

pub type LocalisationTexts = HashMap<(String, u32), String>;
pub type LocalisationLabels = HashMap<(String, String), u32>;

pub const LANGUAGE_CODES: &[&str] = &[
    "ar", "cn", "cz", "de", "es", "fr", "gb", "it", "jp", "kr", "nl", "pl", "pt", "ru", "th", "tr",
    "tw", "us", "zh",
];

fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, data)?;
    let mut last_error = None;
    for attempt in 0..8 {
        match fs::rename(&temporary, path) {
            Ok(()) => return Ok(()),
            Err(error) => {
                last_error = Some(error);
                std::thread::sleep(std::time::Duration::from_millis(50 * (attempt + 1)));
            }
        }
    }
    let _ = fs::remove_file(&temporary);
    Err(last_error.unwrap()).with_context(|| path.display().to_string())
}

#[derive(Debug)]
pub struct Options {
    pub rt2_dir: PathBuf,
    pub dmsb_dir: PathBuf,
    pub key_dir: PathBuf,
    pub out: PathBuf,
    pub legacy_review: Option<PathBuf>,
    pub dealer_data: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CarText {
    pub title_zh: String,
    pub title_en: String,
    pub text_id: String,
    pub car_key: String,
    pub en: String,
    pub zh: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Turn {
    pub id: String,
    pub message_id: u32,
    pub message_key: String,
    pub car_key: String,
    pub speaker: String,
    pub en: String,
    pub zh: String,
    #[serde(default)]
    pub translations: BTreeMap<String, String>,
    pub is_car_detail: bool,
    pub scene_key: String,
    pub scene_order: usize,
    pub variant: u8,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TextEntry {
    pub category: String,
    pub title_zh: String,
    pub title_en: String,
    pub text_id: String,
    pub message_id: u32,
    pub message_key: String,
    pub car_key: String,
    pub source_name: String,
    pub en: String,
    pub zh: String,
    pub car_ids: Vec<String>,
    pub match_method: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CafeScene {
    pub conversation_key: String,
    pub family_key: String,
    pub variant: u8,
    pub start_message_id: u32,
    pub car_key: String,
    pub car_ids: Vec<String>,
    pub turns: Vec<Turn>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealerOffer {
    pub category: String,
    pub offer_id: u64,
    pub price: u64,
    pub discount_price: u64,
    pub rarity: u64,
    pub is_new: bool,
    pub is_hot: bool,
    pub is_pick: bool,
    pub car_key: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DealerOfferRecord {
    pub car_id: String,
    pub name_en: String,
    pub name_zh: String,
    #[serde(flatten)]
    pub offer: DealerOffer,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResearchCar {
    pub id: String,
    pub name_en: String,
    pub name_zh: String,
    pub texts: BTreeMap<String, CarText>,
    pub dialogues: Vec<Turn>,
    pub conversation: Vec<Turn>,
    pub cafe_scenes: Vec<CafeScene>,
    pub dealer: BTreeMap<String, DealerOffer>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Research {
    pub generated_from: &'static str,
    pub stats: serde_json::Value,
    pub languages: Vec<String>,
    pub cars: Vec<ResearchCar>,
    pub text_entries: Vec<TextEntry>,
    pub dealer_offers: Vec<DealerOfferRecord>,
    pub cafe_scenes: Vec<CafeScene>,
    pub detail_rows: Vec<Turn>,
    pub legacy_review: Vec<Turn>,
    pub unresolved_car_keys: BTreeMap<String, String>,
}

fn enrich_turn(turn: &mut Turn, language_texts: &BTreeMap<String, LocalisationTexts>) {
    let Ok(id) = u32::from_str_radix(turn.id.trim_start_matches("0x"), 16) else {
        return;
    };
    let mut translations = BTreeMap::new();
    for (language, texts) in language_texts {
        if let Some(text) = texts.get(&("BuddyGTCafe".to_string(), id)) {
            let text = clean_text(text);
            if !text.is_empty() {
                translations.insert(language.clone(), text);
            }
        }
    }
    turn.translations = translations;
}

fn enrich_multilingual(
    research: &mut Research,
    language_texts: &BTreeMap<String, LocalisationTexts>,
) {
    for turn in research
        .detail_rows
        .iter_mut()
        .chain(research.legacy_review.iter_mut())
        .chain(
            research
                .cafe_scenes
                .iter_mut()
                .flat_map(|scene| scene.turns.iter_mut()),
        )
        .chain(
            research
                .cars
                .iter_mut()
                .flat_map(|car| car.dialogues.iter_mut().chain(car.conversation.iter_mut())),
        )
    {
        enrich_turn(turn, language_texts);
    }
}

#[derive(Debug)]
struct Dmsb {
    data: Vec<u8>,
    rows: Vec<Vec<u8>>,
    table: usize,
}

impl Dmsb {
    fn parse(path: &Path) -> Result<Self> {
        let data = fs::read(path).with_context(|| path.display().to_string())?;
        if data.get(..4) != Some(b"dmsb") {
            bail!("{}: not a DMSB file", path.display());
        }
        let count = u32le(&data, 8)? as usize;
        let table = u64le(&data, 0x18)? as usize;
        if count == 0 || table <= 0x30 || table > data.len() {
            bail!("{}: invalid DMSB table layout", path.display());
        }
        let row_size = (table - 0x30) / count;
        if row_size == 0 || 0x30 + count * row_size != table {
            bail!("{}: unsupported DMSB row size", path.display());
        }
        let rows = (0..count)
            .map(|i| data[0x30 + i * row_size..0x30 + (i + 1) * row_size].to_vec())
            .collect();
        Ok(Self { data, rows, table })
    }

    fn string_at(&self, offset: u32) -> String {
        let start = self.table.saturating_add(offset as usize);
        if start >= self.data.len() {
            return String::new();
        }
        let end = self.data[start..]
            .iter()
            .position(|b| *b == 0)
            .map(|n| start + n)
            .unwrap_or(self.data.len());
        String::from_utf8_lossy(&self.data[start..end]).into_owned()
    }

    fn string(&self, row: &[u8], offset: usize) -> Result<String> {
        Ok(self.string_at(u32le(row, offset)?))
    }
}

fn u32le(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data
        .get(offset..offset + 4)
        .with_context(|| format!("missing u32 at 0x{offset:X}"))?;
    Ok(u32::from_le_bytes(bytes.try_into()?))
}

fn u64le(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data
        .get(offset..offset + 8)
        .with_context(|| format!("missing u64 at 0x{offset:X}"))?;
    Ok(u64::from_le_bytes(bytes.try_into()?))
}

fn clean_text(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut rest = value.replace("\r\n", "\n").replace('\r', "\n");
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(rel_end) = rest[start + 1..].find('>') else {
            let malformed = rest[start + 1..].trim_start().to_ascii_lowercase();
            if malformed.starts_with("p ") || malformed.starts_with("p/") {
                if let Some(newline) = rest[start..].find('\n') {
                    out.push('\n');
                    rest = rest[start + newline + 1..].to_string();
                } else {
                    rest.clear();
                }
                break;
            }
            out.push_str(&rest[start..]);
            rest.clear();
            break;
        };
        let end = start + 1 + rel_end;
        let inner = rest[start + 1..end].trim();
        let lower = inner.to_ascii_lowercase();
        if lower == "p" || lower == "p/" || lower.starts_with("p ") || lower.starts_with("br") {
            out.push('\n');
        } else if inner
            .chars()
            .next()
            .map(|c| c.is_ascii_alphabetic() || c == '/')
            .unwrap_or(false)
        {
            // Real markup tag.
        } else {
            // Inline localisation placeholder such as <660cc> or <1548PS>.
            out.push_str(inner);
        }
        rest = rest[end + 1..].to_string();
    }
    out.push_str(&rest);
    out.retain(|c| !c.is_control() || matches!(c, '\n' | '\t'));
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    out.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn norm(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn core_name(value: &str) -> String {
    let mut words = value.split_whitespace();
    words.next();
    norm(&words.collect::<Vec<_>>().join(" "))
}

fn normalize_car_name(value: &str, known: &HashSet<String>) -> String {
    let mut name = clean_text(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if let Some(pos) = name.rfind('\'') {
        let tail = &name[pos + 1..];
        if tail.len() >= 2 && tail.as_bytes()[..2].iter().all(u8::is_ascii_digit) {
            name.truncate(pos + 3);
            return name.trim().to_string();
        }
    }
    name = name.trim().to_string();
    for (suffix, replacement) in [
        ("Road Car8", "Road Car"),
        ("Road Cara", "Road Car"),
        ("Vantage Gr.4a", "Vantage Gr.4"),
        ("KATANA Edition Gr.47", "KATANA Edition Gr.4"),
        ("Anniversary[", "Anniversary"),
        ("Stealth ModelX", "Stealth Model"),
    ] {
        if name.ends_with(suffix) {
            name.truncate(name.len() - suffix.len());
            name.push_str(replacement);
            break;
        }
    }
    if known.contains(&norm(&name)) {
        return name;
    }
    let chars: Vec<char> = name.chars().collect();
    for trim in 1..=3 {
        if chars.len() <= trim {
            break;
        }
        let candidate: String = chars[..chars.len() - trim].iter().collect();
        let candidate = candidate.trim().to_string();
        if known.contains(&norm(&candidate)) {
            return candidate;
        }
    }
    name
}

fn orphan_text_id(message_key: &str) -> Option<u32> {
    match message_key {
        "Buddy_ContentCar_BrandCentralOwner_normal_caterham_seven_sl_r500_12_1020041" => {
            Some(0x00004F02)
        }
        "Buddy_ContentCar_BrandCentralOwner_normal_pininfarina_vision_gt_17_1020365" => {
            Some(0x00005046)
        }
        "Buddy_ContentCar_BrandCentralOwner_normal_f1_w08_eq_17_1020376" => Some(0x00005051),
        "Buddy_ContentCar_BrandCentralOwner_normal_f1_w08_eq_17_le_1020388" => Some(0x0000505D),
        "Buddy_ContentCar_BrandCentralOwner_normal_mercedes_amg_vision_gt_race_13_lhc_1020404" => {
            Some(0x0000506D)
        }
        "Buddy_ContentCar_BrandCentralOwner_normal_ioniq_6_n_25_1020780" => Some(0x0000D4C2),
        "Buddy_ContentCar_Hagerty_normal_rally_037_evo_2_84_1030455" => Some(0x00008E8E),
        "Buddy_ContentCar_UsedCarOwner_normal_chaser_v_97_1000555" => Some(0x00005495),
        "Buddy_ContentCar_UsedCarOwner_normal_mark_2_v_97_1000781" => Some(0x0000D4C3),
        _ => None,
    }
}

fn canonical_car_name(id: &str) -> Option<&'static str> {
    Some(match id {
        "0x0000428B" => "Alfa Romeo GIULIA TZ2 carrozzata da ZAGATO '65",
        "0x0000761E" => "BMW M4 Gr.4",
        "0x0000761D" => "BMW M4 Safety Car",
        "0x0000AB4D" => "Dallara SF23 Super Formula / Honda '23",
        "0x0000AB4C" => "Dallara SF23 Super Formula / Toyota '23",
        "0x00004243" => "Dodge SRT Tomahawk GTS-R Vision Gran Turismo",
        "0x00004251" => "Dodge Viper Gr.4",
        "0x00004220" => "Eckert's Rod & Custom Mach Forty",
        "0x0000425E" => "Ferrari 458 Italia Gr.4",
        "0x00004224" => "Ford Focus Gr.B Rally Car",
        "0x0000CD14" => "Ford Mustang 2015 American Racer",
        "0x0000423B" => "Ford Mustang Gr.3",
        "0x0000423C" => "Ford Mustang Gr.4",
        "0x0000423D" => "Ford Mustang Gr.B Rally Car",
        "0x00004234" => "Lamborghini Huracán Gr.4",
        "0x00004267" => "Mazda Atenza Gr.3",
        "0x00004269" => "Mazda Atenza Gr.4",
        "0x00004242" => "Mazda RX-VISION GT3 CONCEPT",
        "0x0000B1A5" => "MAZDA3 Gr.4",
        "0x000075D9" => "McLaren 650S Gr.4",
        "0x00007627" => "McLaren Ultimate Vision Gran Turismo (Gr.1)",
        "0x000053F0" => "Mitsubishi Lancer Evolution Final Edition Gr.3",
        "0x000053EF" => "Mitsubishi Lancer Evolution Final Edition Gr.4",
        "0x00006F51" => "Nissan GT-R Gr.4",
        "0x000084D1" => "Nissan Silvia spec-R Aero (S15) Touring Car",
        "0x00007DB6" => "Peugeot L500R HYbrid Vision Gran Turismo, 2017",
        "0x00007DB7" => "Peugeot L750R HYbrid Vision Gran Turismo, 2017",
        "0x00007644" => "Peugeot RCZ Gr.3",
        "0x00007645" => "Peugeot RCZ Gr.4",
        "0x00008050" => "Porsche 917 LIVING LEGEND",
        "0x0000763F" => "Porsche Vision Gran Turismo",
        "0x00007DB4" => "Renault Sport Mégane R.S. Trophy Safety Car",
        "0x0000427C" => "Subaru WRX Gr.B Rally Car",
        "0x0000CD16" => "Toyota AE86 Levin D-Tuned",
        "0x00004C39" => "Toyota FT-1",
        "0x0000CD13" => "Toyota Supra GT Road Car (JZA80)",
        "0x0000765D" => "Volkswagen GTI Roadster Vision Gran Turismo",
        "0x0000765C" => "Volkswagen GTI Vision Gran Turismo (Gr.3)",
        _ => return None,
    })
}

fn token_overlap(a: &str, b: &str) -> f32 {
    let a: HashSet<&str> = a.split_whitespace().collect();
    let b: HashSet<&str> = b.split_whitespace().collect();
    if a.is_empty() {
        return 0.0;
    }
    a.intersection(&b).count() as f32 / a.len() as f32
}

fn levenshtein_ratio(a: &str, b: &str) -> f32 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() || b.is_empty() {
        return f32::from(a.is_empty() && b.is_empty());
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        prev = cur;
    }
    1.0 - prev[b.len()] as f32 / a.len().max(b.len()) as f32
}

fn local_override(car_key: &str) -> &'static [&'static str] {
    match car_key {
        "r18_tdi_11" => &["Audi R18 TDI '11"],
        "gr_supra_racing_concept_18" => &["Toyota GR Supra Racing Concept '18"],
        "gr_supra_rz_20" => &["Toyota GR Supra RZ '20"],
        "gr010_hybrid_21" => &["TOYOTA GR010 HYBRID '21"],
        "peugeot_vision_gt_14" => &["PEUGEOT Vision Gran Turismo"],
        "bugatti_vision_gt_15" => &["Bugatti Vision Gran Turismo"],
        "porsche_vision_gt_coupe_21" => &["Porsche Vision Gran Turismo"],
        "v8_vantage_gt4m_xx" => &["Aston Martin Vantage Gr.4"],
        "vw3_vgt_gt3m_xx" => &["Volkswagen GTI Vision Gran Turismo (Gr.3)X"],
        "toyota_ft1_vgt_gt3m_xx" => &["Toyota FT-1 Vision Gran Turismo (Gr.3)7"],
        "peugeot_l500r_hv_vision_gt_race_17" => &["Peugeot L750R HYbrid Vision Gran Turismo, 2017"],
        "Bruce_levens_ford_gt51_coupe_xx" => &["Wicked Fabrication GT 51"],
        "bruce_levens_ford_gt51_coupe_xx" => &["Wicked Fabrication GT 51"],
        "rcr_civic_xx" => &["Garage RCR CivicX"],
        "roadster_shop_rampage_xx" => &["Roadster Shop Rampage"],
        "swift_sports_gt4m_xx" => &["Suzuki Swift Sport KATANA Edition Gr.4"],
        "mercedes_amg_gt_gt3_16" => &["Mercedes-AMG GT3 '16"],
        "mercedes_amg_gt_s_14_sc" => &["Mercedes-AMG GT Safety Car"],
        "gtr_17_sc" => &["Nissan GT-R Safety Car"],
        "mercedes_amg_gt_r_17" => &["Mercedes-AMG GT R '17"],
        "mercedes_amg_c63_s_15" => &["Mercedes-AMG C 63 S '15"],
        "corvette_zr1_90" => &["Chevrolet Corvette ZR-1 (C4) '89"],
        "gac_maverick_71_xx" => &["Greening Auto Company Maverick"],
        "mercedes_amg_gt3_evo_19" => &["Mercedes-AMG GT3 '20"],
        "bvlgari_vision_gt_23" => &["BVLGARI Aluminium Vision Gran Turismo"],
        "elantra_n_tcr_25" => &["Hyundai ELANTRA N TC '24"],
        "u9_24" => &["Yangwang U9 '24"],
        "_963_24" => &["Porsche 963 '24"],
        "_911_turbo_s_20_sc" => &["Porsche 911 Turbo S Safety Car (992)"],
        "_190e_evo2_91" => &["Mercedes-Benz 190 E 2.5-16 Evolution II '91"],
        "w196r_monopost_55" => &["Mercedes-Benz W 196 R '55"],
        "castrol_nsx_00" => &["Honda NSX GT500 '00"],
        _ => &[],
    }
}

/// Car-key aliases that cannot be recovered by name similarity alone because
/// the retail display name uses a different model spelling or year format.
/// These ids were verified against `CAR_NAME_ALPHABET.dmsb` joins.
fn explicit_car_ids(car_key: &str) -> &'static [&'static str] {
    match car_key {
        "_4_gtl_85" => &["0x0000B862"],
        "_911_gt3_r_22" => &["0x0000D1F3"],
        "_911_turbo_s_93" => &["0x0000D317"],
        "_959_87" => &["0x0000A9D7"],
        "_9x8_25" => &["0x0000D415"],
        "ambulance_himedic_21" => &["0x0000AF04"],
        "avantime_30_v6_bvm6_02" => &["0x0000CB03"],
        "bmw_vision_gt_14" => &["0x000075E9"],
        "bx_19_trs_87" => &["0x0000C7C0"],
        "captur_s_21" => &["0x0000D262"],
        "challenger_srt_demon_18" => &["0x0000A3F3"],
        "civic_si_extra_87" => &["0x0000C2FE"],
        "civic_sir2_93" => &["0x0000BB41"],
        "clio_rs_220_edc_trophy_15" => &["0x000075EF"],
        "clio_rs_220_edc_trophy_16" => &["0x000075F0"],
        "clk_lm_98" => &["0x000084C9"],
        "corvette_c1_58" => &["0x0000AF02"],
        "corvette_z06_c5_01" => &["0x0000C528"],
        "dallara_sf23_honda_23" => &["0x0000AB4D"],
        "delta_hf_integrale_wrc_92" => &["0x0000C7C1"],
        "eckert_mustang_mach_40_xx" => &["0x00004220"],
        "ferrari_vision_gt_22" => &["0x0000A5CD"],
        "ford_roadster_1932_xx" => &["0x00009D57"],
        "ft_1_14" => &["0x00004C39"],
        "giulia_gta_20" => &["0x00009D60"],
        "gt_one_race_99" => &["0x0000B9E0"],
        "honda_vision_gt_m_14" => &["0x00007610"],
        "impreza_wrc_98" => &["0x0000BCF1"],
        "ioniq_5_n_24" => &["0x0000C29F"],
        "kangoo_01" => &["0x0000C4BE"],
        "kpgc110_skyline_gtr_73" => &["0x0000A3F6"],
        "land_cruiser_fj40v_74" => &["0x0000CC62"],
        "m3_97" => &["0x0000BCF0"],
        "mazda3_x_burgundy_19" => &["0x0000A9D9"],
        "mines_r34_n1_base_06" => &["0x0000CDE6"],
        "panda_30_cl_85" => &["0x0000CDE8"],
        "r32_gtr_nismo_90" => &["0x0000AC82"],
        "rcz_gt4m_xx" => &["0x00007645"],
        "roadster_89" => &["0x0000426B"],
        "roadster_nra_22" => &["0x0000A3F9"],
        "rs_5_dtm_19" => &["0x0000A9D6"],
        "s14_ks_aero_96" => &["0x0000A5CB"],
        "sierra_cosworth_rs500_87" => &["0x0000A5CC"],
        "tesla_model_3_perf_23" => &["0x0000B2DB"],
        "toyota_vision_gt_14" => &["0x00004C3A"],
        "unimog_411_62" => &["0x0000C2FF"],
        "valkyrie_21" => &["0x0000AD40"],
        "wrx_sti_man_tt_16" => &["0x00004279"],
        "ioniq_6_n_25" => &["0x0000D4C8"],
        "mark_2_v_97" => &["0x0000D4C6"],
        "chaser_v_97" => &["0x0000D4C7"],
        "caterham_seven_sl_r500_12" => &["0x0000D4C5"],
        _ => &[],
    }
}

const DETAIL_TEXT_OVERRIDES: &[(&str, &[&str], &str)] = &[
    ("0x0000964E", &["0x00006F52"], "Jeremy"),
    ("0x0000D6DE", &["0x0000D4C8"], "Stella"),
    ("0x0000D6D2", &["0x0000D4C6"], "Chris"),
    ("0x0000D6D8", &["0x0000D4C7"], "Jeremy"),
    ("0x0000D6CC", &["0x0000D4C5"], "Chris"),
];

const ORPHAN_TEXT_OVERRIDES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "brandCentral",
        "品牌购物",
        "Brand Central",
        "BuddyContentCarBrandCentralOwner",
        "Buddy_ContentCar_BrandCentralOwner_normal_ioniq_6_n_25_1020780",
        "Hyundai IONIQ 6 N '25",
    ),
    (
        "brandCentral",
        "品牌购物",
        "Brand Central",
        "BuddyContentCarBrandCentralOwner",
        "Buddy_ContentCar_BrandCentralOwner_normal_caterham_seven_sl_r500_12_1020041",
        "Caterham Seven Superlight R500 '08",
    ),
    (
        "brandCentral",
        "品牌购物",
        "Brand Central",
        "BuddyContentCarBrandCentralOwner",
        "Buddy_ContentCar_BrandCentralOwner_normal_f1_w08_eq_17_1020376",
        "Mercedes-AMG F1 W08 EQ Power+ 2017",
    ),
    (
        "brandCentral",
        "品牌购物",
        "Brand Central",
        "BuddyContentCarBrandCentralOwner",
        "Buddy_ContentCar_BrandCentralOwner_normal_f1_w08_eq_17_le_1020388",
        "Mercedes-AMG F1 W08 EQ Power+ (Color Variation) 2017",
    ),
    (
        "brandCentral",
        "品牌购物",
        "Brand Central",
        "BuddyContentCarBrandCentralOwner",
        "Buddy_ContentCar_BrandCentralOwner_normal_pininfarina_vision_gt_17_1020365",
        "Fittipaldi EF7 Vision Gran Turismo by Pininfarina",
    ),
    (
        "brandCentral",
        "品牌购物",
        "Brand Central",
        "BuddyContentCarBrandCentralOwner",
        "Buddy_ContentCar_BrandCentralOwner_normal_mercedes_amg_vision_gt_race_13_lhc_1020404",
        "Mercedes-Benz AMG Vision Gran Turismo LH Edition",
    ),
    (
        "usedCar",
        "二手车",
        "Used Car Dealer",
        "BuddyContentCarUsedCarOwner",
        "Buddy_ContentCar_UsedCarOwner_normal_mark_2_v_97_1000781",
        "Toyota Mark II Tourer V '97",
    ),
    (
        "usedCar",
        "二手车",
        "Used Car Dealer",
        "BuddyContentCarUsedCarOwner",
        "Buddy_ContentCar_UsedCarOwner_normal_chaser_v_97_1000555",
        "Toyota Chaser Tourer V '97",
    ),
    (
        "collection",
        "收藏馆",
        "Hagerty Collection",
        "BuddyContentCarHagerty",
        "Buddy_ContentCar_Hagerty_normal_rally_037_evo_2_84_1030455",
        "Lancia Rally 037 Evoluzione 2 '84",
    ),
];

fn detail_override(text_id: &str) -> Option<(&'static [&'static str], &'static str)> {
    DETAIL_TEXT_OVERRIDES
        .iter()
        .find(|(id, _, _)| *id == text_id)
        .map(|(_, ids, speaker)| (*ids, *speaker))
}

fn car_index(cars: &[ResearchCar]) -> (HashMap<String, Vec<usize>>, HashMap<String, Vec<usize>>) {
    let mut by_name: HashMap<String, Vec<usize>> = HashMap::new();
    let mut by_core: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, car) in cars.iter().enumerate() {
        by_name.entry(norm(&car.name_en)).or_default().push(i);
        by_core.entry(core_name(&car.name_en)).or_default().push(i);
    }
    (by_name, by_core)
}

fn resolve_cars(
    car_key: &str,
    display: &str,
    cars: &[ResearchCar],
    by_name: &HashMap<String, Vec<usize>>,
    by_core: &HashMap<String, Vec<usize>>,
) -> (Vec<usize>, &'static str) {
    for name in local_override(car_key) {
        if let Some(found) = by_name.get(&norm(name)) {
            return (found.clone(), "dms-car-key-override");
        }
    }
    let explicit: Vec<usize> = explicit_car_ids(car_key)
        .iter()
        .filter_map(|id| cars.iter().position(|car| car.id == *id))
        .collect();
    if !explicit.is_empty() {
        return (explicit, "verified-car-key-alias");
    }
    if let Some(found) = by_name.get(&norm(display)) {
        return (found.clone(), "exact-display-name");
    }
    let shown = norm(display);
    if shown.len() >= 4 {
        let suffix_matches: Vec<usize> = cars
            .iter()
            .enumerate()
            .filter(|(_, car)| norm(&car.name_en).ends_with(&shown))
            .map(|(index, _)| index)
            .collect();
        if !suffix_matches.is_empty() {
            return (suffix_matches, "brand-prefix-display-name");
        }
    }
    if let Some(found) = by_core.get(&norm(display)) {
        return (found.clone(), "exact-core-name");
    }

    let mut ranked: Vec<(f32, usize)> = cars
        .iter()
        .enumerate()
        .map(|(i, car)| {
            let key = norm(car_key);
            let candidate = core_name(&car.name_en);
            let score =
                levenshtein_ratio(&shown, &candidate) + 0.35 * token_overlap(&key, &car.name_en);
            (score, i)
        })
        .collect();
    ranked.sort_by(|a, b| b.0.total_cmp(&a.0));
    if let (Some(&(best, idx)), Some(&(second, second_idx))) = (ranked.first(), ranked.get(1)) {
        let margin = if idx == second_idx {
            1.0
        } else {
            best - second
        };
        if best >= 0.72 && margin >= 0.06 {
            return (vec![idx], "fuzzy-name");
        }
    }
    (Vec::new(), "unresolved")
}

pub fn load_localisation(
    lang: &str,
    rt2_dir: &Path,
    key_dir: &Path,
) -> Result<(LocalisationTexts, LocalisationLabels)> {
    let quality = |text: &str| -> i64 {
        let readable = text.chars().filter(|c| c.is_alphanumeric()).count() as i64;
        let replacement = text.matches('\u{fffd}').count() as i64;
        let ids = text
            .split('<')
            .filter(|tag| {
                let lower = tag.to_ascii_lowercase();
                lower.contains("next") || lower.contains("yesno")
            })
            .filter(|tag| tag.contains('='))
            .count() as i64;
        readable - replacement * 80 + ids * 3 + text.len() as i64 / 100
    };
    let path = rt2_dir.join(format!("{lang}.rt2"));
    let rt = Rt2File::parse(&path)?;
    let key_path = key_dir.join(format!("{lang}.ks.bin"));
    let key = fs::read(&key_path).with_context(|| key_path.display().to_string())?;
    let mut key_candidates = vec![key.clone()];
    if lang == "us" {
        for name in ["rt2_all.ks.bin", "rt2_pooled.ks.bin", "rt2_hybrid.ks.bin"] {
            let candidate = key_dir.parent().unwrap_or(key_dir).join(name);
            if let Ok(bytes) = fs::read(candidate) {
                key_candidates.push(bytes);
            }
        }
    }
    let mut texts: HashMap<(String, u32), String> = HashMap::new();
    if let Some(extracted) = rt2_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
    {
        let cache_path = extracted.join("text_decoded").join(format!("{lang}.json"));
        if let Ok(bytes) = fs::read(&cache_path) {
            if let Ok(Value::Object(sections)) = serde_json::from_slice::<Value>(&bytes) {
                for (section, values) in sections {
                    let Some(values) = values.as_object() else {
                        continue;
                    };
                    for (id, value) in values {
                        let Some(value) = value.as_str() else {
                            continue;
                        };
                        let Ok(id) = u32::from_str_radix(id.trim_start_matches("0x"), 16) else {
                            continue;
                        };
                        texts.insert((section.clone(), id), value.to_string());
                    }
                }
            }
        }
    }
    for candidate_key in &key_candidates {
        for decoded in rt2::decode(&rt, candidate_key) {
            let entry = texts.entry((decoded.section, decoded.id)).or_default();
            if quality(&decoded.text) > quality(entry) {
                *entry = decoded.text;
            }
        }
    }
    let mut best_labels = HashMap::<(String, u32), String>::new();
    for section in &rt.sections {
        for entry in &section.entries {
            let len = entry.tag.saturating_sub(1) as usize;
            if len == 0 {
                continue;
            }
            let start = entry.blob_off as usize;
            let Some(encrypted) = rt.data.get(start..start + len) else {
                continue;
            };
            for candidate_key in &key_candidates {
                let plain: Vec<u8> = encrypted
                    .iter()
                    .enumerate()
                    .map(|(i, byte)| byte ^ candidate_key.get(i).copied().unwrap_or(0))
                    .collect();
                let value = String::from_utf8_lossy(&plain)
                    .trim_matches(|c: char| c == '\0' || c.is_whitespace())
                    .to_string();
                if value.is_empty() {
                    continue;
                }
                let key = (section.name.clone(), entry.id);
                let valid = (value.starts_with("Buddy_") || value.starts_with("RT2_"))
                    && value.chars().all(|c| {
                        c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | ' ')
                    });
                let score = if valid {
                    100_000 + value.len() as i64
                } else {
                    value.matches('\u{fffd}').count() as i64 * -80 + value.len() as i64 - 100_000
                };
                let old_score = best_labels
                    .get(&key)
                    .map(|old| {
                        let valid = (old.starts_with("Buddy_") || old.starts_with("RT2_"))
                            && old.chars().all(|c| {
                                c.is_ascii_alphanumeric()
                                    || matches!(c, '_' | '-' | '.' | '/' | ' ')
                            });
                        if valid {
                            100_000 + old.len() as i64
                        } else {
                            old.matches('\u{fffd}').count() as i64 * -80 + old.len() as i64
                                - 100_000
                        }
                    })
                    .unwrap_or(i64::MIN);
                if score > old_score {
                    best_labels.insert(key, value);
                }
            }
        }
    }
    let labels = {
        let mut result = HashMap::new();
        for ((section, text_id), label) in best_labels {
            result.insert((section, label), text_id);
        }
        result
    };
    Ok((texts, labels))
}

fn car_display_map(dmsb_dir: &Path) -> Result<HashMap<String, String>> {
    let alphabet = Dmsb::parse(&dmsb_dir.join("CAR_NAME_ALPHABET.dmsb"))?;
    let mut result = HashMap::new();
    for row in &alphabet.rows {
        let key = alphabet.string(row, 0x08)?;
        let display = alphabet.string(row, 0x20)?;
        if !key.is_empty() {
            result.insert(key, display);
        }
    }
    Ok(result)
}

fn find_dms_dir(root: &Path) -> Result<PathBuf> {
    if root.join("CAR_NAME_ALPHABET.dmsb").is_file() {
        return Ok(root.to_path_buf());
    }
    let mut candidates = WalkDir::new(root)
        .max_depth(6)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_dir())
        .map(|entry| entry.into_path())
        .filter(|path| path.join("CAR_NAME_ALPHABET.dmsb").is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .pop()
        .with_context(|| format!("no DMSB data directory under {}", root.display()))
}

fn build_cars(
    us: &HashMap<(String, u32), String>,
    zh: &HashMap<(String, u32), String>,
    known_names: &HashSet<String>,
) -> Vec<ResearchCar> {
    let mut cars: Vec<ResearchCar> = us
        .iter()
        .filter(|((section, _), _)| section == "CarNameLong")
        .map(|((_, id), en)| ResearchCar {
            id: format!("0x{id:08X}"),
            name_en: normalize_car_name(en, known_names),
            name_zh: normalize_car_name(
                zh.get(&("CarNameLong".to_string(), *id)).unwrap_or(en),
                known_names,
            ),
            texts: BTreeMap::new(),
            dialogues: Vec::new(),
            conversation: Vec::new(),
            cafe_scenes: Vec::new(),
            dealer: BTreeMap::new(),
        })
        .collect();
    cars.sort_by(|a, b| a.name_en.cmp(&b.name_en));
    for car in &mut cars {
        if let Some(name) = canonical_car_name(&car.id) {
            car.name_en = name.into();
        }
    }
    let canonical = HashMap::from([
        ("Audi R18 TDI '11", "0x00006F5C"),
        ("Lancia Delta HF Integrale Evoluzione '91", "0x00008149"),
        ("TOYOTA GR010 HYBRID '21", "0x00009962"),
        ("Toyota GR Supra RZ '20", "0x00004297"),
        ("Toyota GR Supra Racing Concept '18", "0x00004280"),
        ("Volkswagen 1200 '66", "0x0000765A"),
    ]);
    cars.into_iter()
        .filter(|car| {
            canonical
                .get(car.name_en.as_str())
                .map(|id| car.id == *id)
                .unwrap_or(true)
        })
        .collect()
}

fn attach_dealer_offers(
    path: Option<&Path>,
    cars: &mut [ResearchCar],
    texts: &[TextEntry],
) -> Result<Vec<DealerOfferRecord>> {
    let Some(path) = path else {
        return Ok(Vec::new());
    };
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let root: serde_json::Value =
        serde_json::from_slice(&fs::read(path).with_context(|| path.display().to_string())?)
            .with_context(|| format!("parsing dealer data {}", path.display()))?;
    let categories = [
        ("used_car_dealer", "usedCar"),
        ("legend_car_dealer", "collection"),
        ("brand_central_dealer", "brandCentral"),
    ];
    let mut offers_by_key: HashMap<String, BTreeMap<String, DealerOffer>> = HashMap::new();
    for (dealer_key, category) in categories {
        let Some(entries) = root
            .get(dealer_key)
            .and_then(|value| value.get("online_car_shop"))
            .and_then(|value| value.get("cars"))
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for entry in entries {
            let Some(car_key) = entry
                .get("car_parameter")
                .and_then(|value| value.get("car_label"))
                .and_then(serde_json::Value::as_str)
            else {
                continue;
            };
            let offer = DealerOffer {
                category: category.into(),
                offer_id: entry.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                price: entry.get("price").and_then(|v| v.as_u64()).unwrap_or(0),
                discount_price: entry
                    .get("discount_price")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                rarity: entry.get("rarity").and_then(|v| v.as_u64()).unwrap_or(0),
                is_new: entry
                    .get("is_new")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                is_hot: entry
                    .get("is_hot")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                is_pick: entry
                    .get("is_pick")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
                car_key: car_key.into(),
            };
            offers_by_key
                .entry(car_key.into())
                .or_default()
                .insert(category.into(), offer);
        }
    }

    let mut keys_by_car: HashMap<String, HashSet<String>> = HashMap::new();
    for entry in texts {
        for car_id in &entry.car_ids {
            keys_by_car
                .entry(car_id.clone())
                .or_default()
                .insert(entry.car_key.clone());
        }
    }
    for car in cars.iter() {
        for text in car.texts.values() {
            keys_by_car
                .entry(car.id.clone())
                .or_default()
                .insert(text.car_key.clone());
        }
    }

    let mut records = Vec::new();
    for car in cars {
        let Some(keys) = keys_by_car.get(&car.id) else {
            continue;
        };
        let mut matched = BTreeMap::new();
        for key in keys {
            if let Some(offers) = offers_by_key.get(key) {
                matched.extend(offers.clone());
            }
        }
        for offer in matched.values() {
            records.push(DealerOfferRecord {
                car_id: car.id.clone(),
                name_en: car.name_en.clone(),
                name_zh: car.name_zh.clone(),
                offer: offer.clone(),
            });
        }
        car.dealer = matched;
    }
    records.sort_by(|a, b| {
        a.offer
            .category
            .cmp(&b.offer.category)
            .then_with(|| a.name_en.cmp(&b.name_en))
    });
    Ok(records)
}

fn speaker_from_label(label: &str) -> &'static str {
    let rest = label.strip_prefix("Buddy_GTCafe_").unwrap_or(label);
    let token = rest
        .split_once("_normal_")
        .map(|v| v.0)
        .or_else(|| rest.split_once("__").map(|v| v.0))
        .unwrap_or(rest);
    match token {
        "CafeMaster" => "Luca",
        "Sarah" => "Sarah",
        "CarLover" => "Stella",
        "ExpertOldMan1" => "Chris",
        "RaceEnthusiast" => "Jeremy",
        "CarDesignerProf1Kodama" => "Hideo Kodama",
        "CarDesignerProf2Nakamura" => "Shiro Nakamura",
        "CarDesignerProf3Ito" => "Kunihisa Ito",
        "CarDesignerProf4Harada" => "Hikonori Kagurazaka",
        "CarDesignerProf5Tom" => "Tom Matano",
        "CarDesignerProf6Fabio" => "Fabio Filippini",
        "CarDesignerProf7Ed" => "Ed Welburn",
        "CarDesignerProf8Freeman" => "Freeman Thomas",
        "CarDesignerProf9Maeda" => "Ikuo Maeda",
        "DiorKimJones" => "Kim Jones",
        _ => "Luca",
    }
}

fn variant_from_key(key: &str) -> u8 {
    if key.ends_with("_Repeat_Win") {
        1
    } else if key.ends_with("_1st_Win") {
        2
    } else if key.ends_with("_Repeat") {
        3
    } else {
        4
    }
}

fn variant_rank(variant: u8) -> u8 {
    match variant {
        4 => 0,
        3 => 1,
        2 => 2,
        _ => 3,
    }
}

fn family_key(key: &str) -> String {
    key.trim_end_matches("_1st_Win")
        .trim_end_matches("_Repeat_Win")
        .trim_end_matches("_Repeat")
        .trim_end_matches("_1st")
        .to_string()
}

#[derive(Clone)]
struct CafeMessage {
    id: u32,
    message_id: u32,
    message_key: String,
    car_key: String,
    raw: String,
    en: String,
    zh: String,
    speaker: &'static str,
    is_detail: bool,
}

fn attr_u32(tag: &str, names: &[&str]) -> Vec<u32> {
    let lower = tag.to_ascii_lowercase();
    let mut out = Vec::new();
    for name in names {
        let needle = format!("{name}=");
        let mut offset = 0;
        while let Some(rel) = lower[offset..].find(&needle) {
            let word_start = offset + rel;
            let boundary = lower[..word_start]
                .chars()
                .next_back()
                .map(|c| c.is_ascii_whitespace() || c == '<')
                .unwrap_or(true);
            let start = word_start + needle.len();
            if !boundary {
                offset = start;
                continue;
            }
            let digits: String = lower[start..]
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            if let Ok(value) = digits.parse() {
                out.push(value);
                offset = start + digits.len();
            } else {
                offset = start;
            }
        }
    }
    out
}

fn targets(raw: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let lower = raw.to_ascii_lowercase();
    for keyword in ["next", "yesno"] {
        let mut offset = 0;
        while let Some(rel) = lower[offset..].find(keyword) {
            let start = offset + rel;
            let segment = lower[start..].split('>').next().unwrap_or(&lower[start..]);
            if keyword == "next" {
                out.extend(attr_u32(segment, &["id"]));
            } else {
                out.extend(attr_u32(segment, &["msgid", "nmsgid"]));
            }
            offset = start + keyword.len();
        }
    }
    out
}

fn recover_partial_next_targets(
    raw: &str,
    current_message_id: u32,
    car_key: &str,
    messages: &BTreeMap<u32, CafeMessage>,
) -> Vec<u32> {
    let lower = raw.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut offset = 0;
    while let Some(rel) = lower[offset..].find("next") {
        let start = offset + rel;
        let segment = lower[start..].split('>').next().unwrap_or(&lower[start..]);
        let Some(id_pos) = segment.find("id=") else {
            offset = start + 4;
            continue;
        };
        let prefix: String = segment[id_pos + 3..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if (2..7).contains(&prefix.len()) {
            let matches: Vec<u32> = messages
                .values()
                .filter(|message| {
                    message.car_key == car_key
                        && message.message_id > current_message_id
                        && message.message_id.to_string().starts_with(&prefix)
                })
                .map(|message| message.message_id)
                .collect();
            if matches.len() == 1 {
                out.push(matches[0]);
            }
        }
        offset = start + 4;
    }
    out
}

fn write_csvs(out: &Path, texts: &[TextEntry], details: &[Turn]) -> Result<()> {
    let parent = out.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let text_path = parent.join("car_texts_rust.csv");
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer.write_record([
        "category",
        "textId",
        "messageId",
        "carKey",
        "sourceName",
        "carIds",
        "matchMethod",
        "en",
        "zh",
    ])?;
    for row in texts {
        writer.write_record([
            row.category.as_str(),
            row.text_id.as_str(),
            &row.message_id.to_string(),
            row.car_key.as_str(),
            row.source_name.as_str(),
            &row.car_ids.join(";"),
            row.match_method.as_str(),
            row.en.as_str(),
            row.zh.as_str(),
        ])?;
    }
    writer.flush()?;
    atomic_write(
        &text_path,
        &writer.into_inner().map_err(|e| anyhow::anyhow!(e))?,
    )?;

    let detail_path = parent.join("cafe_car_detail_rust.csv");
    let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
    writer.write_record(["id", "messageId", "carKey", "speaker", "en", "zh"])?;
    for row in details {
        writer.write_record([
            row.id.as_str(),
            &row.message_id.to_string(),
            row.car_key.as_str(),
            row.speaker.as_str(),
            row.en.as_str(),
            row.zh.as_str(),
        ])?;
    }
    writer.flush()?;
    atomic_write(
        &detail_path,
        &writer.into_inner().map_err(|e| anyhow::anyhow!(e))?,
    )?;
    Ok(())
}

fn write_dealer_csv(out: &Path, offers: &[DealerOfferRecord]) -> Result<()> {
    let path = out
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("dealer_offers_rust.csv");
    let mut writer = csv::Writer::from_writer(Vec::new());
    writer.write_record([
        "carId",
        "nameEn",
        "nameZh",
        "category",
        "offerId",
        "price",
        "discountPrice",
        "rarity",
        "isNew",
        "isHot",
        "isPick",
        "carKey",
    ])?;
    for row in offers {
        writer.write_record([
            row.car_id.as_str(),
            row.name_en.as_str(),
            row.name_zh.as_str(),
            row.offer.category.as_str(),
            &row.offer.offer_id.to_string(),
            &row.offer.price.to_string(),
            &row.offer.discount_price.to_string(),
            &row.offer.rarity.to_string(),
            &row.offer.is_new.to_string(),
            &row.offer.is_hot.to_string(),
            &row.offer.is_pick.to_string(),
            row.offer.car_key.as_str(),
        ])?;
    }
    writer.flush()?;
    atomic_write(&path, &writer.into_inner().map_err(|e| anyhow::anyhow!(e))?)
}

pub fn build(options: &Options) -> Result<Research> {
    let dmsb_dir = find_dms_dir(&options.dmsb_dir)?;
    let loaded = LANGUAGE_CODES
        .par_iter()
        .map(|language| {
            load_localisation(language, &options.rt2_dir, &options.key_dir)
                .map(|(texts, labels)| ((*language).to_string(), texts, labels))
        })
        .collect::<Vec<_>>();
    let mut language_texts = BTreeMap::new();
    let mut us_labels = HashMap::new();
    for result in loaded {
        let (language, texts, labels) = result?;
        if language == "us" {
            us_labels = labels;
        }
        language_texts.insert(language, texts);
    }
    let us_texts = language_texts
        .get("us")
        .context("US localisation was not decoded")?;
    let zh_texts = language_texts
        .get("zh")
        .context("Chinese localisation was not decoded")?;
    let display = car_display_map(&dmsb_dir)?;
    let known_names: HashSet<String> = display.values().map(|name| norm(name)).collect();
    let mut cars = build_cars(us_texts, zh_texts, &known_names);
    let (by_name, by_core) = car_index(&cars);

    let specs = [
        (
            "brandCentral",
            "品牌购物",
            "Brand Central",
            "BrandCentralOwnerCarDescriptionMessages.dmsb",
            "BuddyContentCarBrandCentralOwner",
        ),
        (
            "usedCar",
            "二手车",
            "Used Car Dealer",
            "UsedCarOwnerCarDescriptionMessages.dmsb",
            "BuddyContentCarUsedCarOwner",
        ),
        (
            "collection",
            "收藏馆",
            "Hagerty Collection",
            "HagertyCarDescriptionMessages.dmsb",
            "BuddyContentCarHagerty",
        ),
    ];
    let mut text_entries: Vec<TextEntry> = specs
        .par_iter()
        .map(|spec| -> Result<Vec<TextEntry>> {
            let table = Dmsb::parse(&dmsb_dir.join(spec.3))?;
            let mut rows = Vec::new();
            for row in &table.rows {
                let message_key = table.string(row, 0x38)?;
                let Some(text_id) = us_labels.get(&(spec.4.to_string(), message_key.clone()))
                else {
                    continue;
                };
                let car_key = table.string(row, 0x40)?;
                let source_name = display.get(&car_key).cloned().unwrap_or_default();
                let (indices, method) =
                    resolve_cars(&car_key, &source_name, &cars, &by_name, &by_core);
                let car_ids = indices
                    .iter()
                    .map(|i| cars[*i].id.clone())
                    .collect::<Vec<_>>();
                rows.push(TextEntry {
                    category: spec.0.into(),
                    title_zh: spec.1.into(),
                    title_en: spec.2.into(),
                    text_id: format!("0x{text_id:08X}"),
                    message_id: u32le(row, 0)?,
                    message_key,
                    car_key,
                    source_name,
                    en: clean_text(
                        us_texts
                            .get(&(spec.4.into(), *text_id))
                            .map(String::as_str)
                            .unwrap_or(""),
                    ),
                    zh: clean_text(
                        zh_texts
                            .get(&(spec.4.into(), *text_id))
                            .map(String::as_str)
                            .unwrap_or(""),
                    ),
                    car_ids,
                    match_method: method.into(),
                });
            }
            Ok(rows)
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();

    for (category, title_zh, title_en, section, message_key, car_name) in ORPHAN_TEXT_OVERRIDES {
        let Some(text_id) = us_labels
            .get(&((*section).to_string(), (*message_key).to_string()))
            .copied()
            .or_else(|| orphan_text_id(message_key))
        else {
            continue;
        };
        let Some(car_index) = cars.iter().position(|car| car.name_en == *car_name) else {
            continue;
        };
        if text_entries
            .iter()
            .any(|entry| entry.text_id == format!("0x{text_id:08X}"))
        {
            continue;
        }
        text_entries.push(TextEntry {
            category: (*category).into(),
            title_zh: (*title_zh).into(),
            title_en: (*title_en).into(),
            text_id: format!("0x{text_id:08X}"),
            message_id: 0,
            message_key: (*message_key).into(),
            car_key: String::new(),
            source_name: cars[car_index].name_en.clone(),
            en: clean_text(
                us_texts
                    .get(&((*section).into(), text_id))
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            zh: clean_text(
                zh_texts
                    .get(&((*section).into(), text_id))
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            car_ids: vec![cars[car_index].id.clone()],
            match_method: "message-key-override".into(),
        });
    }

    for entry in &text_entries {
        for car_id in &entry.car_ids {
            if let Some(car) = cars.iter_mut().find(|car| &car.id == car_id) {
                car.texts.insert(
                    entry.category.clone(),
                    CarText {
                        title_zh: entry.title_zh.clone(),
                        title_en: entry.title_en.clone(),
                        text_id: entry.text_id.clone(),
                        car_key: entry.car_key.clone(),
                        en: entry.en.clone(),
                        zh: entry.zh.clone(),
                    },
                );
            }
        }
    }
    for (target_name, source_name) in [
        (
            "Gran Turismo Red Bull X2019 25th Anniversary",
            "Gran Turismo Red Bull X2019 Competition",
        ),
        (
            "MAZDA RX-VISION GT3 CONCEPT Stealth Model",
            "Mazda RX-VISION GT3 CONCEPT",
        ),
    ] {
        let source_text = cars
            .iter()
            .find(|car| norm(&car.name_en) == norm(source_name))
            .and_then(|car| car.texts.get("brandCentral").cloned());
        if let (Some(text), Some(target)) = (
            source_text,
            cars.iter_mut()
                .find(|car| norm(&car.name_en) == norm(target_name)),
        ) {
            target.texts.insert("brandCentral".into(), text);
        }
    }
    text_entries.sort_by(|a, b| {
        a.category
            .cmp(&b.category)
            .then_with(|| a.text_id.cmp(&b.text_id))
    });
    let dealer_offers =
        attach_dealer_offers(options.dealer_data.as_deref(), &mut cars, &text_entries)?;

    let cafe_labels: HashMap<String, u32> = us_labels
        .iter()
        .filter(|((section, _), _)| section == "BuddyGTCafe")
        .map(|((_, label), id)| (label.clone(), *id))
        .collect();
    let messages_table = Dmsb::parse(&dmsb_dir.join("GTCafeMessages.dmsb"))?;
    let mut messages_by_id = BTreeMap::new();
    let mut dms_labels = HashSet::new();
    for row in &messages_table.rows {
        let message_id = u32le(row, 0)?;
        let message_key = messages_table.string(row, 0x08)?;
        let label = messages_table.string(row, 0x38)?;
        let car_key = messages_table.string(row, 0x40)?;
        let Some(text_id) = cafe_labels.get(&label).copied() else {
            continue;
        };
        dms_labels.insert(label.clone());
        let raw = us_texts
            .get(&("BuddyGTCafe".into(), text_id))
            .cloned()
            .unwrap_or_default();
        let en = clean_text(&raw);
        if en.is_empty() || en == label {
            continue;
        }
        messages_by_id.insert(
            message_id,
            CafeMessage {
                id: text_id,
                message_id,
                message_key,
                car_key: car_key.clone(),
                raw,
                en,
                zh: clean_text(
                    zh_texts
                        .get(&("BuddyGTCafe".into(), text_id))
                        .map(String::as_str)
                        .unwrap_or(""),
                ),
                speaker: speaker_from_label(&label),
                is_detail: false,
            },
        );
    }
    // Post-launch DLC labels can exist in RT2 before their row is present in
    // the retail GTCafeMessages table. Recover the stable label components.
    for ((section, label), text_id) in &us_labels {
        if section != "BuddyGTCafe"
            || dms_labels.contains(label)
            || !label.starts_with("Buddy_GTCafe_")
        {
            continue;
        }
        let Some((_, remainder)) = label.split_once("_normal_") else {
            continue;
        };
        let Some((car_key, number)) = remainder.rsplit_once('_') else {
            continue;
        };
        let Ok(message_id) = number.parse::<u32>() else {
            continue;
        };
        let raw = us_texts
            .get(&("BuddyGTCafe".into(), *text_id))
            .cloned()
            .unwrap_or_default();
        let en = clean_text(&raw);
        if en.is_empty() || en == *label {
            continue;
        }
        messages_by_id.insert(
            message_id,
            CafeMessage {
                id: *text_id,
                message_id,
                message_key: label.clone(),
                car_key: car_key.to_string(),
                raw,
                en,
                zh: clean_text(
                    zh_texts
                        .get(&("BuddyGTCafe".into(), *text_id))
                        .map(String::as_str)
                        .unwrap_or(""),
                ),
                speaker: speaker_from_label(label),
                is_detail: false,
            },
        );
    }
    for message in messages_by_id.values_mut() {
        message.is_detail = message.raw.to_ascii_lowercase().contains("cardetail");
    }
    let mut orphan_by_car: BTreeMap<String, Vec<u32>> = BTreeMap::new();
    for message in messages_by_id.values() {
        if matches!(
            message.car_key.as_str(),
            "caterham_seven_sl_r500_12" | "chaser_v_97" | "ioniq_6_n_25" | "mark_2_v_97"
        ) {
            orphan_by_car
                .entry(message.car_key.clone())
                .or_default()
                .push(message.message_id);
        }
    }

    let variants_table = Dmsb::parse(&dmsb_dir.join("ConversationGTCafeMsg.dmsb"))?;
    let variant_row_count = variants_table.rows.len();
    let variant_starts_missing = variants_table
        .rows
        .iter()
        .filter(|row| {
            u32le(row, 0x1C)
                .map(|start| !messages_by_id.contains_key(&start))
                .unwrap_or(true)
        })
        .count();
    let mut variants: Vec<(String, u32, String)> = variants_table
        .rows
        .iter()
        .filter_map(|row| {
            let key = variants_table.string(row, 0x08).ok()?;
            let start = u32le(row, 0x1C).ok()?;
            let car_key = variants_table.string(row, 0x20).ok()?;
            messages_by_id
                .contains_key(&start)
                .then_some((key, start, car_key))
        })
        .collect();
    let orphan_car_keys = orphan_by_car.len();
    let orphan_message_count: usize = orphan_by_car.values().map(Vec::len).sum();
    for (car_key, message_ids) in orphan_by_car {
        let mut roots = message_ids
            .into_iter()
            .filter(|id| matches!(id % 100, 1 | 11 | 21 | 31))
            .collect::<Vec<_>>();
        roots.sort_unstable();
        roots.dedup();
        for root in roots {
            variants.push((format!("RT2_{car_key}_{root}"), root, car_key.clone()));
        }
    }

    let built_scenes: Vec<Result<Option<CafeScene>>> = variants
        .par_iter()
        .map(|(key, start, car_key)| {
            let source_name = display.get(car_key).cloned().unwrap_or_default();
            let (indices, _) = resolve_cars(car_key, &source_name, &cars, &by_name, &by_core);
            if indices.is_empty() {
                return Ok(None);
            }
            let mut queue = VecDeque::from([*start]);
            let mut seen = HashSet::new();
            let mut turns = Vec::new();
            while let Some(message_id) = queue.pop_front() {
                if !seen.insert(message_id) {
                    continue;
                }
                let Some(message) = messages_by_id.get(&message_id) else {
                    continue;
                };
                turns.push(Turn {
                    id: format!("0x{:08X}", message.id),
                    message_id: message.message_id,
                    message_key: message.message_key.clone(),
                    car_key: message.car_key.clone(),
                    speaker: message.speaker.into(),
                    en: message.en.clone(),
                    zh: message.zh.clone(),
                    translations: BTreeMap::new(),
                    is_car_detail: message.is_detail,
                    scene_key: key.clone(),
                    scene_order: turns.len() + 1,
                    variant: variant_from_key(key),
                });
                let mut next_targets = targets(&message.raw);
                next_targets.extend(recover_partial_next_targets(
                    &message.raw,
                    message.message_id,
                    &message.car_key,
                    &messages_by_id,
                ));
                if message.raw.to_ascii_lowercase().contains("next") {
                    if let Some(next) = messages_by_id.get(&(message.message_id + 1)) {
                        if next.car_key == message.car_key {
                            next_targets.push(message.message_id + 1);
                        }
                    }
                }
                for target in next_targets {
                    queue.push_back(target);
                }
            }
            if turns.is_empty() {
                return Ok(None);
            }
            Ok(Some(CafeScene {
                conversation_key: key.clone(),
                family_key: if key.starts_with("RT2_") {
                    format!("RT2_{car_key}")
                } else {
                    family_key(key)
                },
                variant: if key.starts_with("RT2_") {
                    match start % 100 {
                        11 => 3,
                        21 => 2,
                        31 => 1,
                        _ => 4,
                    }
                } else {
                    variant_from_key(key)
                },
                start_message_id: *start,
                car_key: car_key.clone(),
                car_ids: indices.iter().map(|i| cars[*i].id.clone()).collect(),
                turns,
            }))
        })
        .collect();
    let mut cafe_scenes = Vec::new();
    let mut unresolved_car_keys = BTreeMap::new();
    for scene in built_scenes {
        if let Some(scene) = scene? {
            cafe_scenes.push(scene);
        }
    }
    let mut family_primary = HashMap::<(String, String), (u8, usize)>::new();
    for (index, scene) in cafe_scenes.iter().enumerate() {
        let key = (scene.car_key.clone(), scene.family_key.clone());
        family_primary
            .entry(key)
            .and_modify(|(rank, primary)| {
                if variant_rank(scene.variant) < *rank {
                    *rank = variant_rank(scene.variant);
                    *primary = index;
                }
            })
            .or_insert((variant_rank(scene.variant), index));
    }
    for (index, scene) in cafe_scenes.iter().enumerate() {
        if family_primary
            .get(&(scene.car_key.clone(), scene.family_key.clone()))
            .map(|(_, primary)| *primary == index)
            .unwrap_or(false)
        {
            for car_id in &scene.car_ids {
                if let Some(car) = cars.iter_mut().find(|car| &car.id == car_id) {
                    car.conversation.extend(scene.turns.clone());
                    car.cafe_scenes.push(scene.clone());
                }
            }
        }
    }
    cafe_scenes.sort_by(|a, b| {
        a.conversation_key
            .cmp(&b.conversation_key)
            .then_with(|| a.car_key.cmp(&b.car_key))
    });

    let mut detail_rows: Vec<Turn> = messages_by_id
        .values()
        .filter(|message| message.is_detail)
        .map(|message| Turn {
            id: format!("0x{:08X}", message.id),
            message_id: message.message_id,
            message_key: message.message_key.clone(),
            car_key: message.car_key.clone(),
            speaker: message.speaker.into(),
            en: message.en.clone(),
            zh: message.zh.clone(),
            translations: BTreeMap::new(),
            is_car_detail: true,
            scene_key: String::new(),
            scene_order: 0,
            variant: 0,
        })
        .collect();
    for (text_id, _, speaker) in DETAIL_TEXT_OVERRIDES {
        if detail_rows.iter().any(|row| row.id == *text_id) {
            continue;
        }
        let Ok(id) = u32::from_str_radix(text_id.trim_start_matches("0x"), 16) else {
            continue;
        };
        detail_rows.push(Turn {
            id: (*text_id).to_string(),
            message_id: 0,
            message_key: String::new(),
            car_key: String::new(),
            speaker: (*speaker).to_string(),
            en: clean_text(
                us_texts
                    .get(&("BuddyGTCafe".into(), id))
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            zh: clean_text(
                zh_texts
                    .get(&("BuddyGTCafe".into(), id))
                    .map(String::as_str)
                    .unwrap_or(""),
            ),
            translations: BTreeMap::new(),
            is_car_detail: true,
            scene_key: String::new(),
            scene_order: 0,
            variant: 0,
        });
    }
    detail_rows.sort_by(|a, b| a.id.cmp(&b.id));
    for row in &detail_rows {
        let source_name = display.get(&row.car_key).cloned().unwrap_or_default();
        let indices = if let Some((ids, _)) = detail_override(&row.id) {
            ids.iter()
                .filter_map(|id| cars.iter().position(|car| car.id == *id))
                .collect()
        } else {
            resolve_cars(&row.car_key, &source_name, &cars, &by_name, &by_core).0
        };
        if indices.is_empty() {
            unresolved_car_keys.insert(row.car_key.clone(), row.id.clone());
        }
        for index in indices {
            cars[index].dialogues.push(Turn {
                scene_key: cafe_scenes
                    .iter()
                    .find(|scene| scene.turns.iter().any(|turn| turn.id == row.id))
                    .map(|scene| scene.conversation_key.clone())
                    .unwrap_or_default(),
                ..row.clone()
            });
        }
    }
    for car in &mut cars {
        let mut seen_turns = HashSet::new();
        car.conversation
            .retain(|turn| seen_turns.insert(turn.id.clone()));
        let mut seen_dialogues = HashSet::new();
        car.dialogues
            .retain(|row| seen_dialogues.insert(row.id.clone()));
        car.conversation.sort_by(|a, b| {
            a.scene_key
                .cmp(&b.scene_key)
                .then_with(|| a.scene_order.cmp(&b.scene_order))
                .then_with(|| a.id.cmp(&b.id))
        });
        car.dialogues.sort_by(|a, b| a.id.cmp(&b.id));
    }

    let legacy_ids: HashSet<String> = options
        .legacy_review
        .as_ref()
        .and_then(|path| fs::read(path).ok())
        .and_then(|data| serde_json::from_slice::<Vec<String>>(&data).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();
    let legacy_review = detail_rows
        .iter()
        .filter(|row| legacy_ids.is_empty() || legacy_ids.contains(&row.id))
        .cloned()
        .collect::<Vec<_>>();

    let mapped_texts = text_entries
        .iter()
        .filter(|entry| !entry.car_ids.is_empty())
        .count();
    let unresolved_texts = text_entries.len() - mapped_texts;
    let cars_with_texts = cars.iter().filter(|car| !car.texts.is_empty()).count();
    let stats = json!({
        "cars": cars.len(),
        "languages": language_texts.len(),
        "textEntries": text_entries.len(),
        "dealerOffers": dealer_offers.len(),
        "mappedTextEntries": mapped_texts,
        "unresolvedTextEntries": unresolved_texts,
        "carsWithTexts": cars_with_texts,
        "cafeMessages": messages_by_id.len(),
        "cafeVariantRows": variant_row_count,
        "cafeVariantStartsMissing": variant_starts_missing,
        "orphanCarKeys": orphan_car_keys,
        "orphanMessages": orphan_message_count,
        "cafeVariants": variants.len(),
        "cafeScenes": cafe_scenes.len(),
        "carDetailRows": detail_rows.len(),
        "legacyReviewRows": legacy_review.len(),
        "carsWithConversation": cars.iter().filter(|car| !car.conversation.is_empty()).count(),
        "conversationLines": cars.iter().map(|car| car.conversation.len()).sum::<usize>(),
        "unresolvedCafeCarKeys": unresolved_car_keys.len(),
    });
    let mut research = Research {
        generated_from: "PS5 DMSB + RT2 (native Rust)",
        stats,
        languages: LANGUAGE_CODES
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        cars,
        text_entries,
        dealer_offers,
        cafe_scenes,
        detail_rows,
        legacy_review,
        unresolved_car_keys,
    };
    enrich_multilingual(&mut research, &language_texts);
    let translated_turns = research
        .detail_rows
        .iter()
        .chain(
            research
                .cafe_scenes
                .iter()
                .flat_map(|scene| scene.turns.iter()),
        )
        .filter(|turn| !turn.translations.is_empty())
        .count();
    let language_cells = research
        .detail_rows
        .iter()
        .chain(
            research
                .cafe_scenes
                .iter()
                .flat_map(|scene| scene.turns.iter()),
        )
        .map(|turn| turn.translations.len())
        .sum::<usize>();
    if let Some(stats) = research.stats.as_object_mut() {
        stats.insert("translatedTurns".into(), json!(translated_turns));
        stats.insert("languageCells".into(), json!(language_cells));
    }

    if let Some(parent) = options.out.parent() {
        fs::create_dir_all(parent)?;
    }
    atomic_write(&options.out, &serde_json::to_vec(&research)?)?;
    write_csvs(&options.out, &research.text_entries, &research.detail_rows)?;
    write_dealer_csv(&options.out, &research.dealer_offers)?;
    Ok(research)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_placeholders_survive_text_cleanup() {
        let text = "<sseq scmd=\"GTCafe_Test\">Engine <660cc> and <1548PS>.<p/>Done";
        assert_eq!(clean_text(text), "Engine 660cc and 1548PS.\nDone");
    }

    #[test]
    fn message_targets_support_next_and_yesno() {
        let targets =
            targets("Continue<next id=1702206 type=9>\\n<yesno msgid=1702207 nmsgid=1702208>");
        assert_eq!(targets, vec![1702206, 1702207, 1702208]);
    }

    #[test]
    fn car_name_cleanup_uses_known_display_names() {
        let known = HashSet::from([
            norm("Gran Turismo Red Bull X2019 25th Anniversary"),
            norm("Suzuki Swift Sport KATANA Edition Gr.4"),
        ]);
        assert_eq!(
            normalize_car_name("Gran Turismo Red Bull X2019 25th Anniversary[", &known),
            "Gran Turismo Red Bull X2019 25th Anniversary"
        );
        assert_eq!(
            normalize_car_name("Suzuki Swift Sport KATANA Edition Gr.47", &known),
            "Suzuki Swift Sport KATANA Edition Gr.4"
        );
        assert_eq!(
            normalize_car_name("Alfa Romeo GIULIA TZ2  carrozzata da ZAGATO '65", &known),
            "Alfa Romeo GIULIA TZ2 carrozzata da ZAGATO '65"
        );
    }

    #[test]
    fn cafe_family_and_variant_decode_exactly() {
        assert_eq!(
            family_key("Harada_014_sileighty_xx_Repeat_Win"),
            "Harada_014_sileighty_xx"
        );
        assert_eq!(variant_from_key("Harada_014_sileighty_xx_Repeat_Win"), 1);
        assert_eq!(variant_from_key("Harada_014_sileighty_xx_1st_Win"), 2);
        assert_eq!(variant_from_key("Harada_014_sileighty_xx_Repeat"), 3);
        assert_eq!(variant_from_key("Harada_014_sileighty_xx_1st"), 4);
    }
}
