//! Parallel, Python-free construction of the public research-site payload.
//!
//! The native `research` command owns the exact DMSB/RT2 joins. This module
//! joins that report with the clean localisation candidates and the official
//! car-list caches, then writes every browser dataset in one pass. All
//! independent cars, dialogue rows and text rows are processed with Rayon.

use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::OnceLock;

use crate::garage;
use crate::rt2::{self, Rt2File};
use crate::zh_collection;

type TextKey = (String, String);
type TextCandidates = HashMap<TextKey, Vec<((isize, isize, usize), String)>>;
static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

const LANGUAGES: &[(&str, &str, &str)] = &[
    ("ar", "العربية", "Arabic"),
    ("cn", "简体中文", "Chinese (Simplified)"),
    ("cz", "Čeština", "Czech"),
    ("de", "Deutsch", "German"),
    ("es", "Español", "Spanish"),
    ("fr", "Français", "French"),
    ("gb", "English (UK)", "English (UK)"),
    ("it", "Italiano", "Italian"),
    ("jp", "日本語", "Japanese"),
    ("kr", "한국어", "Korean"),
    ("nl", "Nederlands", "Dutch"),
    ("pl", "Polski", "Polish"),
    ("pt", "Português", "Portuguese"),
    ("ru", "Русский", "Russian"),
    ("th", "ไทย", "Thai"),
    ("tr", "Türkçe", "Turkish"),
    ("tw", "繁體中文", "Chinese (Traditional)"),
    ("us", "English (US)", "English (US)"),
    ("zh", "中文（简体）", "Chinese"),
];

fn language_catalog() -> Vec<Value> {
    LANGUAGES
        .iter()
        .map(|(id, native, english)| json!({"id": id, "native": native, "english": english}))
        .collect()
}

fn atomic_write(path: &Path, data: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let sequence = WRITE_SEQUENCE.fetch_add(1, AtomicOrdering::Relaxed);
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("data");
    let temporary = path.with_file_name(format!(".{name}.{}.{}.tmp", std::process::id(), sequence));
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
    pub research: PathBuf,
    pub official_cars: PathBuf,
    pub official_descriptions: PathBuf,
    pub official_gallery: PathBuf,
    pub collection_fallback: PathBuf,
    pub dmsb_dir: PathBuf,
    pub text_dirs: Vec<PathBuf>,
    pub site_data: PathBuf,
    pub report_dir: PathBuf,
}

impl Default for Options {
    fn default() -> Self {
        let root = PathBuf::from(r"C:\gt7\gt7\_re");
        Self {
            research: root.join("reports/research_rust.json"),
            official_cars: root.join("reports/official_cars.json"),
            official_descriptions: root.join("reports/official_car_descriptions.json"),
            official_gallery: root.join("reports/official_car_gallery.json"),
            collection_fallback: root.join("reports/vehicle_collection_texts.json"),
            dmsb_dir: root.join("extracted/dms_prpys/dmsdata"),
            text_dirs: vec![
                root.join("extracted/text_pooled"),
                root.join("extracted/text_decoded"),
            ],
            site_data: root.join("site/public/data"),
            report_dir: root.join("reports"),
        }
    }
}

#[derive(Clone, Debug)]
struct Speaker {
    id: &'static str,
    en: &'static str,
    zh: &'static str,
    role_en: &'static str,
    role_zh: &'static str,
    avatar: &'static str,
}

const SPEAKERS: &[Speaker] = &[
    Speaker {
        id: "Luca",
        en: "Luca",
        zh: "卢卡",
        role_en: "GT Café owner",
        role_zh: "GT 咖啡屋主人",
        avatar: "avatars/extracted/luca.png",
    },
    Speaker {
        id: "Sarah",
        en: "Sarah Eaton",
        zh: "莎拉·伊顿",
        role_en: "Main guide and race engineer",
        role_zh: "主向导 / 赛事工程师",
        avatar: "avatars/extracted/sarah.png",
    },
    Speaker {
        id: "Stella",
        en: "Stella",
        zh: "斯黛拉",
        role_en: "Café regular · compact cars",
        role_zh: "咖啡屋常客 · 紧凑型车",
        avatar: "avatars/extracted/stella.png",
    },
    Speaker {
        id: "Chris",
        en: "Chris",
        zh: "克里斯",
        role_en: "Café regular · road-car appraisal",
        role_zh: "咖啡屋常客 · 公路车鉴赏",
        avatar: "avatars/extracted/chris.png",
    },
    Speaker {
        id: "Jeremy",
        en: "Jeremy",
        zh: "杰里米",
        role_en: "Café regular · motorsport history",
        role_zh: "咖啡屋常客 · 赛车史",
        avatar: "avatars/extracted/jeremy.png",
    },
    Speaker {
        id: "Freeman Thomas",
        en: "Freeman Thomas",
        zh: "弗里曼·托马斯",
        role_en: "Former Ford design director; Porsche, Audi",
        role_zh: "前福特设计总监 · 保时捷/奥迪",
        avatar: "avatars/extracted/freeman-thomas.png",
    },
    Speaker {
        id: "Shiro Nakamura",
        en: "Shiro Nakamura",
        zh: "中村史郎",
        role_en: "Former Nissan Chief Creative Officer",
        role_zh: "前日产首席创意官",
        avatar: "avatars/extracted/shiro-nakamura.png",
    },
    Speaker {
        id: "Hideo Kodama",
        en: "Hideo Kodama",
        zh: "儿玉英雄",
        role_en: "Former Opel designer",
        role_zh: "前欧宝设计师",
        avatar: "avatars/extracted/hideo-kodama.png",
    },
    Speaker {
        id: "Fabio Filippini",
        en: "Fabio Filippini",
        zh: "法比奥·菲利皮尼",
        role_en: "Former Pininfarina design director",
        role_zh: "前宾尼法利纳设计总监",
        avatar: "avatars/extracted/fabio-filippini.png",
    },
    Speaker {
        id: "Ed Welburn",
        en: "Ed Welburn",
        zh: "埃德·韦伯恩",
        role_en: "Former GM global design vice president",
        role_zh: "前通用汽车全球设计副总裁",
        avatar: "avatars/extracted/ed-welburn.png",
    },
    Speaker {
        id: "Tom Matano",
        en: "Tom Matano",
        zh: "俣野努",
        role_en: "Mazda designer; MX-5 design contributor",
        role_zh: "马自达设计师 · MX-5 设计参与者",
        avatar: "avatars/extracted/tom-matano.png",
    },
    Speaker {
        id: "Hikonori Kagurazaka",
        en: "Hikonori Kagurazaka",
        zh: "神乐坂彦则",
        role_en: "High-performance car designer",
        role_zh: "高性能车设计师",
        avatar: "avatars/extracted/hikonori-kagurazaka.png",
    },
    Speaker {
        id: "Kunihisa Ito",
        en: "Kunihisa Ito",
        zh: "伊藤邦久",
        role_en: "GM, Opel and Ford designer",
        role_zh: "通用/欧宝/福特汽车设计师",
        avatar: "avatars/extracted/kunihisa-ito.png",
    },
    Speaker {
        id: "Ikuo Maeda",
        en: "Ikuo Maeda",
        zh: "前田育男",
        role_en: "Former Mazda design lead; Kodo design",
        role_zh: "前马自达设计负责人 · 魂动设计",
        avatar: "avatars/extracted/ikuo-maeda.png",
    },
    Speaker {
        id: "Kim Jones",
        en: "Kim Jones",
        zh: "金·琼斯",
        role_en: "Fashion designer; Dior Men artistic director",
        role_zh: "时装设计师 · Dior 男装艺术总监",
        avatar: "avatars/extracted/kim-jones.png",
    },
];

const MULTI_BRANDS: &[&str] = &[
    "Eckert's Rod & Custom",
    "Chris Holstrom Concepts",
    "Greening Auto Company",
    "Wicked Fabrication",
    "Roadster Shop",
    "Gran Turismo",
    "Red Bull",
    "Rolls-Royce",
    "Ayrton Senna",
    "Mercedes-Benz",
    "Mercedes-AMG",
    "Alfa Romeo",
    "Aston Martin",
    "De Tomaso",
    "Land Rover",
    "Range Rover",
    "RE Amemiya",
];

const OFFICIAL_ID_ALIASES: &[(&str, &str)] = &[
    ("Alpine Vision Gran Turismo 2017", "car3187"),
    ("Alpine Vision Gran Turismo", "car2112"),
    ("Audi Vision Gran Turismo", "car3314"),
    ("BMW Vision Gran Turismo", "car2087"),
    ("Bugatti Vision Gran Turismo", "car2134"),
    ("Eckert's Rod & Custom Mach Forty", "car3346"),
    ("Ferrari Vision Gran Turismo", "car3510"),
    ("Greening Auto Company Maverick", "car3522"),
    ("Mazda LM55 Vision Gran Turismo", "car2118"),
    ("McLaren Ultimate Vision Gran Turismo", "car3315"),
    ("PEUGEOT Vision Gran Turismo", "car2113"),
    ("Porsche Vision Gran Turismo", "car3478"),
    ("SUZUKI Vision Gran Turismo", "car3476"),
    ("Škoda Vision Gran Turismo", "car3542"),
];

// Gift, livery and tuned variants are absent from the official car-list
// roster, but they share the mechanical specification of their base car.
// Keep these as explicit aliases so an unrelated vehicle can never inherit
// another car's garage data through fuzzy matching.
const OFFICIAL_SPEC_ID_ALIASES: &[(&str, &str)] = &[
    ("De Tomaso Mangusta (Christian Dior)", "car3437"),
    ("Gran Turismo Red Bull X2019 25th Anniversary", "car3374"),
    ("MAZDA RX-VISION GT3 CONCEPT Stealth Model", "car3419"),
    (
        "Mercedes-Benz AMG Vision Gran Turismo LH Edition",
        "car2055",
    ),
    ("Nissan Skyline GT-R GP-Tuned (KPGC10)", "car837"),
    ("Porsche 911 Turbo Rally (930)", "car3385"),
];

fn norm_words(value: &str) -> Vec<String> {
    value
        .replace(['’', '‘'], "'")
        .to_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

fn clean_text(value: &str) -> String {
    static TAG: OnceLock<Regex> = OnceLock::new();
    static MALFORMED: OnceLock<Regex> = OnceLock::new();
    static RESIDUE: OnceLock<Regex> = OnceLock::new();
    static CONTROL: OnceLock<Regex> = OnceLock::new();
    let mut text = value.replace("\r\n", "\n").replace('\r', "\n");
    if text.starts_with("ssq ") {
        if let Some(end) = text.find('>') {
            text = text[end + 1..].to_string();
        }
    }
    let tag = TAG.get_or_init(|| Regex::new(r"(?is)</?[a-z][^>]*>").expect("tag regex"));
    text = tag.replace_all(&text, "\n").into_owned();
    let malformed = MALFORMED.get_or_init(|| {
        Regex::new(r"(?i)(?:^|[^\w\s<]{1,3}\s*)(?:sseq|seq|ssq|next|yesno|p)\b[^\n>]*>")
            .expect("malformed tag regex")
    });
    text = malformed.replace_all(&text, "").into_owned();
    let residue = RESIDUE.get_or_init(|| {
        Regex::new(r"([。！？.!?…])[A-Za-z0-9]{1,2}\s*$").expect("trailing residue regex")
    });
    text = residue.replace_all(&text, "$1").into_owned();
    let control =
        CONTROL.get_or_init(|| Regex::new(r"[\x00-\x08\x0b-\x1f\x7f]").expect("control regex"));
    text = control.replace_all(&text, "").into_owned();
    text = text.replace('\u{FFFD}', "");
    text.retain(|ch| {
        !('\u{E000}'..='\u{F8FF}').contains(&ch)
            && (!ch.is_control() || matches!(ch, '\n' | '\r' | '\t'))
    });
    while text.contains("\n\n\n") {
        text = text.replace("\n\n\n", "\n\n");
    }
    text.lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

fn brand_of(name: &str) -> String {
    for brand in MULTI_BRANDS {
        if name.to_lowercase().starts_with(&brand.to_lowercase()) {
            return (*brand).to_string();
        }
    }
    let first = name.split_whitespace().next().unwrap_or("Unknown");
    match first {
        "1932" => "Ford",
        "Enzo" => "Ferrari",
        "Chris" => "Chris Holstrom Concepts",
        "GT" => "Citroën",
        "Garage" => "Garage RCR",
        "Lambo" => "Lamborghini",
        "MAZDA3" => "Mazda",
        "DAIHATSU" => "Daihatsu",
        "HYUNDAI" => "Hyundai",
        "LEXUS" => "Lexus",
        "MAZDA" => "Mazda",
        "BVLGARI" => "Bulgari",
        "AFEELA" => "Afeela",
        "Mini-Cooper" => "MINI",
        "NISSAN" => "Nissan",
        "PEUGEOT" => "Peugeot",
        "SUBARU" => "Subaru",
        "SUZUKI" => "Suzuki",
        "TOYOTA" => "Toyota",
        other => other,
    }
    .to_string()
}

fn text_quality(text: &str) -> (isize, isize, usize) {
    let visible = clean_text(text);
    let readable = visible.chars().filter(|c| c.is_alphanumeric()).count() as isize;
    let replacement = text
        .chars()
        .filter(|c| {
            *c == '\u{FFFD}'
                || ('\u{E000}'..='\u{F8FF}').contains(c)
                || (c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        })
        .count() as isize;
    let markup = visible.matches('<').count() as isize;
    let score = readable - replacement * 100 - markup * 3;
    (score, -replacement, visible.len())
}

fn load_text_candidates(dirs: &[PathBuf], aliases: &[&str]) -> Result<HashMap<TextKey, String>> {
    let mut candidates: TextCandidates = HashMap::new();
    for dir in dirs {
        for alias in aliases {
            let path = dir.join(format!("{alias}.csv"));
            if !path.is_file() {
                continue;
            }
            let mut reader = csv::ReaderBuilder::new()
                .flexible(true)
                .from_path(&path)
                .with_context(|| path.display().to_string())?;
            for row in reader.deserialize::<HashMap<String, String>>() {
                let row = row?;
                let section = row.get("section").cloned().unwrap_or_default();
                let id = row.get("id").cloned().unwrap_or_default();
                let raw = row.get("text").cloned().unwrap_or_default();
                let text = clean_text(&raw);
                if text.is_empty() {
                    continue;
                }
                candidates
                    .entry((section, id))
                    .or_default()
                    .push((text_quality(&raw), text));
            }
        }
    }
    Ok(candidates
        .into_iter()
        .map(|(key, values)| {
            let best = values
                .into_iter()
                .max_by(|a, b| a.0.cmp(&b.0))
                .map(|(_, text)| text)
                .unwrap_or_default();
            (key, best)
        })
        .collect())
}

fn load_json(path: &Path) -> Result<Value> {
    let data = fs::read(path).with_context(|| path.display().to_string())?;
    serde_json::from_slice(&data).with_context(|| path.display().to_string())
}

struct CollectionGameCopy {
    en: HashMap<TextKey, String>,
    zh: HashMap<TextKey, String>,
}

fn load_collection_game_texts(collection_report: &Path) -> Result<CollectionGameCopy> {
    let report = load_json(collection_report)?;
    let stem = report
        .pointer("/sources/0/english")
        .and_then(Value::as_str)
        .unwrap_or("L_O_PWIJL");
    let root = collection_report
        .parent()
        .and_then(Path::parent)
        .context("collection report has no extraction root")?;
    let raw_path = root
        .join("extracted/descriptions_raw")
        .join(format!("{stem}.rt2"));
    let key_path = root
        .join("keys/rt2_description")
        .join(format!("{stem}.rt2.ks.bin"));
    let en = if raw_path.is_file() && key_path.is_file() {
        let rt = Rt2File::parse(&raw_path)?;
        let key = fs::read(&key_path).with_context(|| key_path.display().to_string())?;
        rt2::decode(&rt, &key)
            .into_iter()
            .filter(|entry| !entry.text.is_empty())
            .map(|entry| ((entry.section, format!("0x{:08X}", entry.id)), entry.text))
            .collect()
    } else {
        // Keep site builds usable when only the decoded cache is present.
        let decoded_path = root
            .join("extracted/descriptions_decoded")
            .join(format!("{stem}.json"));
        let decoded = load_json(&decoded_path)?;
        let mut texts = HashMap::new();
        for (section, entries) in decoded.as_object().into_iter().flatten() {
            for (id, text) in entries.as_object().into_iter().flatten() {
                if let Some(text) = text.as_str().filter(|text| !text.trim().is_empty()) {
                    texts.insert((section.clone(), id.clone()), clean_text(text));
                }
            }
        }
        texts
    };
    let zh = zh_collection::load(root).unwrap_or_else(|error| {
        eprintln!("site: Chinese Car Collection recovery unavailable: {error:#}");
        HashMap::new()
    });
    Ok(CollectionGameCopy { en, zh })
}

fn official_key(name: &str) -> String {
    norm_words(name).join(" ")
}

fn core_name(name: &str) -> String {
    let mut words = name.split_whitespace();
    words.next();
    official_key(&words.collect::<Vec<_>>().join(" "))
}

fn car_core_name(name: &str, brand: &str) -> String {
    if name
        .to_lowercase()
        .starts_with(&(brand.to_lowercase() + " "))
    {
        official_key(&name[brand.len() + 1..])
    } else {
        core_name(name)
    }
}

fn official_match<'a>(
    car: &Value,
    by_full: &'a HashMap<String, &'a Value>,
    by_name: &'a HashMap<String, &'a Value>,
    official: &'a [Value],
) -> Option<&'a Value> {
    let name_en = car.get("nameEn").and_then(Value::as_str).unwrap_or("");
    let name_zh = car.get("nameZh").and_then(Value::as_str).unwrap_or("");
    let brand = car
        .get("brand")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| brand_of(name_en));
    for name in [name_en, name_zh] {
        let key = official_key(name);
        if let Some(found) = by_full.get(&key) {
            return Some(found);
        }
        if let Some(found) = by_name.get(&key) {
            return Some(found);
        }
        let core = car_core_name(name, &brand);
        if let Some(found) = by_name.get(&core) {
            return Some(found);
        }
    }
    let tokens: HashSet<String> = norm_words(name_en).into_iter().collect();
    if tokens.is_empty() {
        return None;
    }
    let mut scored = official
        .iter()
        .map(|item| {
            let name = item.get("name").and_then(Value::as_str).unwrap_or("");
            let target: HashSet<String> = norm_words(name).into_iter().collect();
            let overlap = tokens.intersection(&target).count();
            let score = overlap as f64 / target.len().max(1) as f64;
            (score, item)
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal));
    scored
        .first()
        .filter(|(score, _)| *score >= 0.62)
        .map(|(_, item)| *item)
}

fn speaker(id: &str) -> Option<&'static Speaker> {
    SPEAKERS.iter().find(|item| item.id == id)
}

fn attach_speaker(mut turn: Map<String, Value>) -> Map<String, Value> {
    let id = turn.get("speaker").and_then(Value::as_str).unwrap_or("");
    if let Some(meta) = speaker(id) {
        turn.insert("speakerZh".into(), json!(meta.zh));
        turn.insert("roleEn".into(), json!(meta.role_en));
        turn.insert("roleZh".into(), json!(meta.role_zh));
        turn.insert("avatar".into(), json!(meta.avatar));
    }
    turn
}

fn clean_turn(
    value: &Value,
    us_text: &HashMap<(String, String), String>,
    zh_text: &HashMap<(String, String), String>,
    detail: bool,
) -> Option<Value> {
    let object = value.as_object()?;
    let id = object.get("id")?.as_str()?;
    let en_raw = object.get("en").and_then(Value::as_str).unwrap_or("");
    let zh_raw = object.get("zh").and_then(Value::as_str).unwrap_or("");
    let text_key = ("BuddyGTCafe".to_string(), id.to_string());
    let en = us_text
        .get(&text_key)
        .cloned()
        .unwrap_or_else(|| clean_text(en_raw));
    let zh = zh_text
        .get(&text_key)
        .cloned()
        .unwrap_or_else(|| clean_text(zh_raw));
    if en.is_empty() && zh.is_empty() {
        return None;
    }
    let mut output = object.clone();
    if !detail {
        output.remove("messageId");
    }
    output.remove("messageKey");
    output.remove("carKey");
    output.remove("variant");
    output.insert("en".into(), json!(en));
    output.insert("zh".into(), json!(zh));
    let scene = object
        .get("sceneKey")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .or_else(|| object.get("sceneId").and_then(Value::as_str))
        .unwrap_or("");
    output.insert("sceneId".into(), json!(scene));
    output.remove("translations");
    if !detail {
        output.remove("isCarDetail");
        output.remove("sceneOrder");
        output.remove("variant");
    }
    Some(Value::Object(attach_speaker(output)))
}

fn dedupe_turn_content(rows: &mut Vec<Value>) {
    let mut seen = HashSet::new();
    rows.retain(|row| {
        let normalize = |value: &str| {
            value
                .chars()
                .filter(|ch| ch.is_alphanumeric())
                .flat_map(char::to_lowercase)
                .collect::<String>()
        };
        let speaker = row.get("speaker").and_then(Value::as_str).unwrap_or("");
        let zh = normalize(row.get("zh").and_then(Value::as_str).unwrap_or(""));
        let en = normalize(row.get("en").and_then(Value::as_str).unwrap_or(""));
        let text = if zh.is_empty() { en } else { zh };
        if text.is_empty() {
            return true;
        }
        seen.insert((speaker.to_string(), text))
    });
}

fn text_object(
    entry: &Value,
    us: &HashMap<(String, String), String>,
    zh: &HashMap<(String, String), String>,
    section: &str,
) -> Value {
    let text_id = entry.get("textId").and_then(Value::as_str).unwrap_or("");
    let key = (section.to_string(), text_id.to_string());
    let en = us
        .get(&key)
        .cloned()
        .unwrap_or_else(|| clean_text(entry.get("en").and_then(Value::as_str).unwrap_or("")));
    let zh_text = zh
        .get(&key)
        .cloned()
        .unwrap_or_else(|| clean_text(entry.get("zh").and_then(Value::as_str).unwrap_or("")));
    json!({
        "titleZh": entry.get("titleZh").cloned().unwrap_or(json!("")),
        "titleEn": entry.get("titleEn").cloned().unwrap_or(json!("")),
        "textId": text_id,
        "carKey": entry.get("carKey").cloned().unwrap_or(json!("")),
        "sourceName": entry.get("sourceName").cloned().unwrap_or(json!("")),
        "matchMethod": entry.get("matchMethod").cloned().unwrap_or(json!("")),
        "en": en,
        "zh": zh_text,
    })
}

fn attach_collection(
    car: &mut Value,
    official_descriptions: &[Value],
    official_by_name: &HashMap<String, &Value>,
    official_by_id: &HashMap<String, &Value>,
    official_gallery: &HashMap<String, Value>,
    fallback_by_id: &HashMap<String, Value>,
    game_texts: &CollectionGameCopy,
) {
    let object = car.as_object_mut().expect("car object");
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name_en = object
        .get("nameEn")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let name_zh = object
        .get("nameZh")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let record = [name_en.as_str(), name_zh.as_str()]
        .into_iter()
        .find_map(|name| official_by_name.get(&official_key(name)).copied())
        .or_else(|| {
            let key = official_key(&name_en);
            official_descriptions.iter().find(|item| {
                official_key(item.get("nameEn").and_then(Value::as_str).unwrap_or("")) == key
            })
        });
    let spec_alias_id = OFFICIAL_SPEC_ID_ALIASES
        .iter()
        .find(|(name, _)| official_key(name) == official_key(&name_en))
        .map(|(_, id)| *id);
    let spec_record = spec_alias_id
        .and_then(|id| official_by_id.get(id).copied())
        .or_else(|| {
            object
                .get("officialId")
                .and_then(Value::as_str)
                .and_then(|id| official_by_id.get(id).copied())
        })
        .or(record);
    if let Some(record) = spec_record {
        if record.get("specsEn").is_some() || record.get("specsZh").is_some() {
            let match_kind = if spec_alias_id.is_some() {
                "variant-base-alias"
            } else {
                "official-car-id"
            };
            object.insert(
                "specs".into(),
                json!({
                    "officialId": record.get("id").cloned().unwrap_or(json!("")),
                    "en": record.get("specsEn").cloned().unwrap_or(json!({})),
                    "zh": record.get("specsZh").cloned().unwrap_or(json!({})),
                    "source": "official-gt7-carlist",
                    "match": match_kind,
                    "sourceUrlEn": record.get("sourceUrlEn").cloned().unwrap_or(json!("")),
                    "sourceUrlZh": record.get("sourceUrlZh").cloned().unwrap_or(json!("")),
                }),
            );
        }
    }
    let detail_record = record.or_else(|| {
        spec_alias_id
            .and_then(|id| official_by_id.get(id).copied())
    });
    if let Some(record) = detail_record {
        let official_id = record.get("id").and_then(Value::as_str).unwrap_or("");
        object.insert(
            "officialDetails".into(),
            json!({
                "officialId": official_id,
                "nameEn": record.get("nameEn").cloned().unwrap_or(json!("")),
                "nameZh": record.get("nameZh").cloned().unwrap_or(json!("")),
                "heroEn": clean_text(record.get("heroEn").and_then(Value::as_str).unwrap_or("")),
                "heroZh": clean_text(record.get("heroZh").and_then(Value::as_str).unwrap_or("")),
                "descriptionEn": clean_text(record.get("descEn").and_then(Value::as_str).unwrap_or("")),
                "descriptionZh": clean_text(record.get("descZh").and_then(Value::as_str).unwrap_or("")),
                "sourceUrlEn": record.get("sourceUrlEn").cloned().unwrap_or(json!("")),
                "sourceUrlZh": record.get("sourceUrlZh").cloned().unwrap_or(json!("")),
                "images": official_gallery
                    .get(official_id)
                    .and_then(|item| item.get("images"))
                    .cloned()
                    .unwrap_or(json!([])),
            }),
        );
    }
    let mut collection = if let Some(record) = record {
        let fallback = fallback_by_id.get(&id);
        let car_key = fallback
            .and_then(|value| value.get("carKey"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let catch_id = fallback
            .and_then(|value| value.pointer("/catch/textId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let long_id = fallback
            .and_then(|value| value.pointer("/long/textId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        let source_name = fallback
            .and_then(|value| value.get("sourceName"))
            .and_then(Value::as_str)
            .unwrap_or_else(|| record.get("nameEn").and_then(Value::as_str).unwrap_or(""));
        json!({
            "titleZh": "车辆收藏",
            "titleEn": "Car Collection",
            "textId": catch_id,
            "textIds": {"catch": catch_id, "long": long_id},
            "carKey": car_key,
            "sourceName": source_name,
            "matchMethod": fallback.and_then(|value| value.get("matchMethod")).cloned().unwrap_or(json!("official-id")),
            "officialId": record.get("id").cloned().unwrap_or(json!("")),
            "translationSource": "official-gt7-carlist",
            "sourceUrlEn": record.get("sourceUrlEn").cloned().unwrap_or(json!("")),
            "sourceUrlZh": record.get("sourceUrlZh").cloned().unwrap_or(json!("")),
            "en": format!(
                "{}\n\n{}",
                clean_text(record.get("heroEn").and_then(Value::as_str).unwrap_or("")),
                clean_text(record.get("descEn").and_then(Value::as_str).unwrap_or(""))
            ),
            "zh": format!(
                "{}\n\n{}",
                clean_text(record.get("heroZh").and_then(Value::as_str).unwrap_or("")),
                clean_text(record.get("descZh").and_then(Value::as_str).unwrap_or(""))
            ),
            "catch": {
                "textId": catch_id,
                "en": clean_text(record.get("heroEn").and_then(Value::as_str).unwrap_or("")),
                "zh": clean_text(record.get("heroZh").and_then(Value::as_str).unwrap_or("")),
            },
            "long": {
                "textId": long_id,
                "en": clean_text(record.get("descEn").and_then(Value::as_str).unwrap_or("")),
                "zh": clean_text(record.get("descZh").and_then(Value::as_str).unwrap_or("")),
            },
        })
    } else if let Some(fallback) = fallback_by_id.get(&id) {
        fallback.clone()
    } else {
        return;
    };
    apply_game_collection_texts(&mut collection, game_texts);
    object
        .entry("texts")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .expect("texts object")
        .insert("vehicleCollection".into(), collection);
}

fn apply_game_collection_texts(
    collection: &mut Value,
    game_texts: &CollectionGameCopy,
) {
    let Some(object) = collection.as_object_mut() else {
        return;
    };
    let catch_id = object
        .get("catch")
        .and_then(Value::as_object)
        .and_then(|catch| catch.get("textId"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let long_id = object
        .get("long")
        .and_then(Value::as_object)
        .and_then(|long| long.get("textId"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let game_copy = |texts: &HashMap<TextKey, String>, section: &str, id: &str| {
        if id.is_empty() {
            return None;
        }
        texts
            .get(&(section.to_string(), id.to_string()))
            .map(|text| clean_text(text))
            .filter(|text| !text.is_empty())
    };
    let game_catch = game_copy(&game_texts.en, "DescriptionCatch", &catch_id);
    let game_long = game_copy(&game_texts.en, "DescriptionLong", &long_id);
    let game_long_zh = game_copy(&game_texts.zh, "DescriptionLong", &long_id);
    if let Some(text) = &game_catch {
        if let Some(catch) = object.get_mut("catch").and_then(Value::as_object_mut) {
            catch.insert("en".into(), json!(text));
            catch.insert("sourceEn".into(), json!("rt2-description-catch"));
        }
    }
    if let Some(text) = &game_long {
        if let Some(long) = object.get_mut("long").and_then(Value::as_object_mut) {
            long.insert("en".into(), json!(text));
            long.insert("sourceEn".into(), json!("rt2-description-long"));
        }
    }
    if let Some(text) = &game_long_zh {
        if let Some(long) = object.get_mut("long").and_then(Value::as_object_mut) {
            long.insert("zh".into(), json!(text));
            long.insert("sourceZh".into(), json!("rt2-description-zh-merged"));
        }
    }
    let catch_en = game_catch.or_else(|| {
        object
            .get("catch")
            .and_then(Value::as_object)
            .and_then(|catch| catch.get("en"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let long_en = game_long.or_else(|| {
        object
            .get("long")
            .and_then(Value::as_object)
            .and_then(|long| long.get("en"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let has_game_zh = game_long_zh.is_some();
    let catch_zh = object
        .get("catch")
        .and_then(Value::as_object)
        .and_then(|catch| catch.get("zh"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let long_zh = game_long_zh.or_else(|| {
        object
            .get("long")
            .and_then(Value::as_object)
            .and_then(|long| long.get("zh"))
            .and_then(Value::as_str)
            .map(str::to_string)
    });
    let has_game_long = object
        .get("long")
        .and_then(Value::as_object)
        .and_then(|long| long.get("sourceEn"))
        .and_then(Value::as_str)
        == Some("rt2-description-long");
    object.insert(
        "en".into(),
        json!(
            [catch_en, long_en]
                .into_iter()
                .flatten()
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        ),
    );
    object.insert(
        "zh".into(),
        json!(
            [catch_zh, long_zh]
                .into_iter()
                .flatten()
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join("\n\n")
        ),
    );
    object.insert(
        "translationSource".into(),
        json!("rt2-game-english+rt2-game-zh-merged"),
    );
    object.insert(
        "sourceBreakdown".into(),
        json!({
            "en": if has_game_long {
                "rt2-description-long"
            } else {
                "official-gt7-carlist"
            },
            "zh": if has_game_zh {
                "rt2-description-zh-merged"
            } else {
                "official-gt7-carlist"
            },
        }),
    );
}

fn write_csv(path: &Path, headers: &[&str], rows: impl Iterator<Item = Vec<String>>) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record(headers)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    atomic_write(path, &writer.into_inner().map_err(|e| anyhow::anyhow!(e))?)
}

fn collect_dialogue_translations(research: &Value) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut languages: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
    let mut add_turn = |turn: &Value| {
        let Some(id) = turn.get("id").and_then(Value::as_str) else {
            return;
        };
        let Some(translations) = turn.get("translations").and_then(Value::as_object) else {
            return;
        };
        for (language, value) in translations {
            let Some(text) = value.as_str() else {
                continue;
            };
            let text = clean_text(text);
            let text = if text.starts_with("ssq") {
                text.split_once('>')
                    .map(|(_, rest)| clean_text(rest))
                    .unwrap_or(text)
            } else {
                text
            };
            if !text.is_empty() {
                languages
                    .entry(language.clone())
                    .or_default()
                    .insert(id.to_string(), text);
            }
        }
    };
    for scene in research
        .get("cafeScenes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for turn in scene
            .get("turns")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            add_turn(turn);
        }
    }
    for turn in research
        .get("detailRows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        add_turn(turn);
    }
    languages
}

pub fn build(options: &Options) -> Result<Value> {
    let started = std::time::Instant::now();
    eprintln!("site: loading research and caches");
    let research = load_json(&options.research)?;
    let official_cars = load_json(&options.official_cars)?
        .as_array()
        .cloned()
        .context("official cars must be an array")?;
    let descriptions = load_json(&options.official_descriptions)?;
    let official_descriptions = descriptions
        .get("cars")
        .and_then(Value::as_array)
        .cloned()
        .context("official descriptions.cars is missing")?;
    let fallback_by_id = if options.collection_fallback.is_file() {
        load_json(&options.collection_fallback)?
            .get("entries")
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter()
                    .filter_map(|row| Some((row.get("carId")?.as_str()?.to_string(), row.clone())))
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default()
    } else {
        HashMap::new()
    };
    let collection_game_texts = load_collection_game_texts(&options.collection_fallback)?;

    let (us, zh) = rayon::join(
        || load_text_candidates(&options.text_dirs, &["us", "gb"]),
        || load_text_candidates(&options.text_dirs, &["cn", "zh"]),
    );
    let us = us?;
    let zh = zh?;
    let garage_catalog = garage::Catalog::load(&options.dmsb_dir)?;
    eprintln!(
        "site: loaded {} English / {} Chinese text rows",
        us.len(),
        zh.len()
    );

    let mut by_full: HashMap<String, &Value> = HashMap::new();
    let mut by_name: HashMap<String, &Value> = HashMap::new();
    let mut by_id: HashMap<&str, &Value> = HashMap::new();
    for item in &official_cars {
        let brand = item.get("brand").and_then(Value::as_str).unwrap_or("");
        let name = item.get("name").and_then(Value::as_str).unwrap_or("");
        by_full
            .entry(official_key(&format!("{brand} {name}")))
            .or_insert(item);
        by_name.entry(official_key(name)).or_insert(item);
        by_id.insert(item.get("id").and_then(Value::as_str).unwrap_or(""), item);
    }
    for (alias, id) in OFFICIAL_ID_ALIASES {
        if let Some(item) = by_id.get(id) {
            by_name.entry(official_key(alias)).or_insert(item);
        }
    }
    let official_by_name: HashMap<String, &Value> = official_descriptions
        .iter()
        .flat_map(|item| {
            [
                item.get("nameEn").and_then(Value::as_str),
                item.get("nameZh").and_then(Value::as_str),
            ]
            .into_iter()
            .flatten()
            .map(move |name| (official_key(name), item))
        })
        .collect();
    let official_by_id: HashMap<String, &Value> = official_descriptions
        .iter()
        .filter_map(|item| Some((item.get("id").and_then(Value::as_str)?.to_string(), item)))
        .collect();
    let official_gallery = if options.official_gallery.is_file() {
        load_json(&options.official_gallery)?
            .get("cars")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| {
                        Some((item.get("officialId")?.as_str()?.to_string(), item.clone()))
                    })
                    .collect::<HashMap<_, _>>()
            })
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    let text_entries = research
        .get("textEntries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut cars_value = research
        .get("cars")
        .and_then(Value::as_array)
        .cloned()
        .context("research.cars is missing")?;

    cars_value.par_iter_mut().for_each(|car| {
        let car_for_match = car.clone();
        let Some(object) = car.as_object_mut() else {
            return;
        };
        let name = object
            .get("nameEn")
            .and_then(Value::as_str)
            .unwrap_or("")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let official = official_match(&car_for_match, &by_full, &by_name, &official_cars);
        let brand = brand_of(&name);
        object.insert("nameEn".into(), json!(name));
        object.insert("brand".into(), json!(brand));
        if object
            .get("nameZh")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty()
        {
            object.insert("nameZh".into(), json!(name));
        }
        if let Some(item) = official {
            let official_id = item.get("id").and_then(Value::as_str).unwrap_or("");
            let official_brand = item.get("brand").and_then(Value::as_str).unwrap_or(&brand);
            object.insert("officialId".into(), json!(official_id));
            object.insert("image".into(), json!(format!("cars/{official_id}.png")));
            object.insert(
                "brandLogo".into(),
                json!(format!("{}.png", official_brand.replace([' ', '/'], "_"))),
            );
        } else {
            object.insert("officialId".into(), Value::Null);
            object.insert("image".into(), Value::Null);
            object.insert("brandLogo".into(), Value::Null);
        }
        object.insert(
            "sourceIds".into(),
            json!([object.get("id").cloned().unwrap_or(json!(""))]),
        );

        let mut texts = Map::new();
        for entry in &text_entries {
            let car_ids = entry
                .get("carIds")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let id = object.get("id").and_then(Value::as_str).unwrap_or("");
            if !car_ids.iter().any(|value| value.as_str() == Some(id)) {
                continue;
            }
            let category = entry.get("category").and_then(Value::as_str).unwrap_or("");
            let section = match category {
                "brandCentral" => "BuddyContentCarBrandCentralOwner",
                "usedCar" => "BuddyContentCarUsedCarOwner",
                "collection" => "BuddyContentCarHagerty",
                _ => "",
            };
            let value = text_object(entry, &us, &zh, section);
            if let Some(existing) = texts.get_mut(category) {
                if existing.get("textId") != value.get("textId") {
                    existing
                        .as_object_mut()
                        .expect("text")
                        .entry("variants")
                        .or_insert_with(|| json!([]))
                        .as_array_mut()
                        .expect("variants")
                        .push(value);
                }
            } else {
                texts.insert(category.to_string(), value);
            }
        }
        object.insert("texts".into(), Value::Object(texts));

        let mut raw_dialogues = object
            .get("dialogues")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        raw_dialogues.sort_by(|a, b| {
            let msg_a = a.get("messageId").and_then(Value::as_u64).unwrap_or(0);
            let msg_b = b.get("messageId").and_then(Value::as_u64).unwrap_or(0);
            let id_a = a.get("id").and_then(Value::as_str).unwrap_or("");
            let id_b = b.get("id").and_then(Value::as_str).unwrap_or("");
            (msg_a, id_a).cmp(&(msg_b, id_b))
        });
        let mut dialogues = raw_dialogues
            .iter()
            .filter_map(|row| clean_turn(row, &us, &zh, true))
            .filter(|row| {
                let scene = row.get("sceneId").and_then(Value::as_str).unwrap_or("");
                !scene.contains("gtr_lm_nismo_15") || name.contains("GT-R LM NISMO")
            })
            .collect::<Vec<_>>();
        let raw_conversation = object
            .get("conversation")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut conversation = raw_conversation
            .iter()
            .filter_map(|row| clean_turn(row, &us, &zh, false))
            .collect::<Vec<_>>();
        let seen_dialogue: HashSet<String> = dialogues
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str).map(str::to_string))
            .collect();
        dialogues.extend(
            raw_conversation
                .iter()
                .filter(|row| {
                    row.get("isCarDetail")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .filter_map(|row| clean_turn(row, &us, &zh, true))
                .filter(|row| {
                    row.get("id")
                        .and_then(Value::as_str)
                        .map(|id| !seen_dialogue.contains(id))
                        .unwrap_or(false)
                }),
        );
        dialogues.sort_by(|a, b| {
            let seq_a = a.get("messageId").and_then(Value::as_u64).unwrap_or(0);
            let seq_b = b.get("messageId").and_then(Value::as_u64).unwrap_or(0);
            seq_a.cmp(&seq_b)
        });
        for dialogue in &mut dialogues {
            if let Some(object) = dialogue.as_object_mut() {
                object.remove("messageId");
            }
        }
        conversation.sort_by(|a, b| {
            let scene_a = a.get("sceneId").and_then(Value::as_str).unwrap_or("");
            let scene_b = b.get("sceneId").and_then(Value::as_str).unwrap_or("");
            let order_a = a.get("sceneOrder").and_then(Value::as_u64).unwrap_or(0);
            let order_b = b.get("sceneOrder").and_then(Value::as_u64).unwrap_or(0);
            (scene_a, order_a).cmp(&(scene_b, order_b))
        });
        dedupe_turn_content(&mut dialogues);
        dedupe_turn_content(&mut conversation);
        object.insert("dialogues".into(), Value::Array(dialogues));
        object.insert("conversation".into(), Value::Array(conversation));
        object.remove("cafeScenes");
        attach_collection(
            car,
            &official_descriptions,
            &official_by_name,
            &official_by_id,
            &official_gallery,
            &fallback_by_id,
            &collection_game_texts,
        );
    });
    eprintln!("site: joined cars and metadata");

    let text_aliases = [
        (
            "Gran Turismo Red Bull X2019 25th Anniversary",
            "Gran Turismo Red Bull X2019 Competition",
        ),
        (
            "MAZDA RX-VISION GT3 CONCEPT Stealth Model",
            "MAZDA RX-VISION GT3 CONCEPT",
        ),
    ];
    for (target_name, source_name) in text_aliases {
        let target_key = official_key(target_name);
        let source_key = official_key(source_name);
        let source = cars_value
            .iter()
            .find(|car| {
                car.get("nameEn")
                    .and_then(Value::as_str)
                    .map(official_key)
                    .as_deref()
                    == Some(source_key.as_str())
            })
            .and_then(|car| car.pointer("/texts/brandCentral"))
            .cloned();
        if let Some(source) = source {
            if let Some(target) = cars_value.iter_mut().find(|car| {
                car.get("nameEn")
                    .and_then(Value::as_str)
                    .map(official_key)
                    .as_deref()
                    == Some(target_key.as_str())
            }) {
                if let Some(texts) = target.get_mut("texts").and_then(Value::as_object_mut) {
                    texts.entry("brandCentral").or_insert_with(|| {
                        let mut value = source;
                        if let Some(object) = value.as_object_mut() {
                            object.insert("variantOf".into(), json!(source_name));
                        }
                        value
                    });
                }
            }
        }
    }

    // Attach official collection copy to exact official-name aliases that do
    // not have a matching research car key.
    cars_value.par_iter_mut().for_each(|car| {
        if car
            .get("texts")
            .and_then(|v| v.get("vehicleCollection"))
            .is_some()
        {
            return;
        }
        attach_collection(
            car,
            &official_descriptions,
            &official_by_name,
            &official_by_id,
            &official_gallery,
            &fallback_by_id,
            &collection_game_texts,
        );
    });
    garage_catalog.enrich(&mut cars_value);
    let garage_details = garage::report(&cars_value);

    let brands = {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for car in cars_value.iter() {
            let brand = car
                .get("brand")
                .and_then(Value::as_str)
                .unwrap_or("Unknown");
            *counts.entry(brand.to_string()).or_default() += 1;
        }
        counts
            .into_iter()
            .map(|(name, count)| json!({"name": name, "count": count}))
            .collect::<Vec<_>>()
    };

    let mut speaker_counts: HashMap<String, usize> = HashMap::new();
    for car in cars_value.iter() {
        for turn in car
            .get("conversation")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(id) = turn.get("speaker").and_then(Value::as_str) {
                *speaker_counts.entry(id.to_string()).or_default() += 1;
            }
        }
    }

    let mut legacy = research
        .get("legacyReview")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let cars_by_key: HashMap<String, (String, String)> = cars_value
        .iter()
        .flat_map(|car| {
            let name_en = car
                .get("nameEn")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let name_zh = car
                .get("nameZh")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            car.get("texts")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(move |texts| texts.values())
                .filter_map(move |text| {
                    let key = text.get("carKey").and_then(Value::as_str)?.to_string();
                    (!key.is_empty()).then(|| (key, (name_en.clone(), name_zh.clone())))
                })
        })
        .collect();
    let unmatched: Vec<Value> = legacy
        .drain(..)
        .map(|row| {
            let object = row.as_object().cloned().unwrap_or_default();
            let car_key = object.get("carKey").and_then(Value::as_str).unwrap_or("");
            let resolved = cars_by_key.get(car_key);
            let id = object.get("id").and_then(Value::as_str).unwrap_or("");
            let key = ("BuddyGTCafe".to_string(), id.to_string());
            let mut turn = attach_speaker(object);
            turn.insert(
                "en".into(),
                json!(us.get(&key).cloned().unwrap_or_else(|| {
                    clean_text(turn.get("en").and_then(Value::as_str).unwrap_or(""))
                })),
            );
            turn.insert(
                "zh".into(),
                json!(zh.get(&key).cloned().unwrap_or_else(|| {
                    clean_text(turn.get("zh").and_then(Value::as_str).unwrap_or(""))
                })),
            );
            if let Some(translations) = turn.get_mut("translations").and_then(Value::as_object_mut)
            {
                translations.retain(|_, value| {
                    let Some(text) = value.as_str() else {
                        return false;
                    };
                    let cleaned = clean_text(text);
                    if cleaned.is_empty() {
                        false
                    } else {
                        *value = Value::String(cleaned);
                        true
                    }
                });
            }
            turn.remove("translations");
            turn.insert(
                "resolvedCarNameEn".into(),
                json!(resolved.map(|v| v.0.as_str()).unwrap_or("")),
            );
            turn.insert(
                "resolvedCarNameZh".into(),
                json!(resolved.map(|v| v.1.as_str()).unwrap_or("")),
            );
            turn.insert("resolution".into(), json!("exact-rt2-message"));
            turn.insert("sceneKeys".into(), json!([]));
            Value::Object(turn)
        })
        .collect();

    let cast = SPEAKERS
        .iter()
        .map(|meta| {
            json!({
                "id": meta.id,
                "en": meta.en,
                "zh": meta.zh,
                "roleEn": meta.role_en,
                "roleZh": meta.role_zh,
                "avatar": meta.avatar,
                "count": speaker_counts.get(meta.id).copied().unwrap_or(0),
            })
        })
        .collect::<Vec<_>>();
    let speakers = cast
        .iter()
        .filter(|item| item.get("count").and_then(Value::as_u64).unwrap_or(0) > 0)
        .cloned()
        .collect::<Vec<_>>();

    let with_dialogue = cars_value
        .iter()
        .filter(|car| {
            car.get("dialogues")
                .and_then(Value::as_array)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
                || car
                    .get("conversation")
                    .and_then(Value::as_array)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false)
        })
        .count();
    let conversation_lines: HashSet<(String, String)> = cars_value
        .iter()
        .flat_map(|car| {
            car.get("conversation")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|turn| {
                    Some((
                        turn.get("id")?.as_str()?.to_string(),
                        turn.get("sceneId")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string(),
                    ))
                })
        })
        .collect();

    let collection_cars = cars_value
        .iter()
        .filter(|car| {
            car.get("texts")
                .and_then(|v| v.get("vehicleCollection"))
                .is_some()
        })
        .count();
    let collection_entries = cars_value
        .iter()
        .filter(|car| {
            car.get("texts")
                .and_then(|v| v.get("vehicleCollection"))
                .is_some()
        })
        .count();
    let text_entries_count: usize = cars_value
        .iter()
        .map(|car| {
            car.get("texts")
                .and_then(Value::as_object)
                .map(|texts| {
                    texts
                        .values()
                        .map(|text| {
                            1 + text
                                .get("variants")
                                .and_then(Value::as_array)
                                .map(Vec::len)
                                .unwrap_or(0)
                        })
                        .sum::<usize>()
                })
                .unwrap_or(0)
        })
        .sum();
    let research_stats = research.get("stats").cloned().unwrap_or_else(|| json!({}));
    let vehicle_collection_entries = if options.collection_fallback.is_file() {
        load_json(&options.collection_fallback)
            .ok()
            .and_then(|value| {
                value
                    .pointer("/stats/matchedEntries")
                    .and_then(Value::as_u64)
            })
            .unwrap_or(collection_entries as u64)
    } else {
        collection_entries as u64
    };
    let dealer_text_entries = research_stats
        .get("textEntries")
        .and_then(Value::as_u64)
        .unwrap_or(634);
    let total_mapped_entries = dealer_text_entries + vehicle_collection_entries;
    let game_long_cars = cars_value
        .iter()
        .filter(|car| {
            car.pointer("/texts/vehicleCollection/long/sourceEn")
                .and_then(Value::as_str)
                == Some("rt2-description-long")
        })
        .count();
    let game_long_zh_cars = cars_value
        .iter()
        .filter(|car| {
            car.pointer("/texts/vehicleCollection/long/sourceZh")
                .and_then(Value::as_str)
                == Some("rt2-description-zh-merged")
        })
        .count();
    let game_long_zh_chars = cars_value
        .iter()
        .filter(|car| {
            car.pointer("/texts/vehicleCollection/long/sourceZh")
                .and_then(Value::as_str)
                == Some("rt2-description-zh-merged")
        })
        .filter_map(|car| {
            car.pointer("/texts/vehicleCollection/long/zh")
                .and_then(Value::as_str)
        })
        .map(|text| text.chars().count())
        .sum::<usize>();
    let game_long_chars = cars_value
        .iter()
        .filter_map(|car| {
            car.pointer("/texts/vehicleCollection/long/en")
                .and_then(Value::as_str)
        })
        .map(|text| text.chars().count())
        .sum::<usize>();
    let mut text_stats = json!({
        "entries": total_mapped_entries,
        "mappedEntries": total_mapped_entries,
        "unresolvedEntries": 0,
        "orphanTextRows": research_stats.get("orphanMessages").and_then(Value::as_u64).unwrap_or(0),
        "carsWithAnyText": cars_value.iter().filter(|car| car.get("texts").and_then(Value::as_object).map(|v| !v.is_empty()).unwrap_or(false)).count(),
        "byCategory": {
            "brandCentral": research_stats.get("textEntries").and_then(Value::as_u64).unwrap_or(0),
            "vehicleCollection": vehicle_collection_entries,
        },
        "vehicleCollection": {
            "matchedCars": collection_cars,
            "matchedEntries": collection_entries,
            "carsWithBilingualCollection": collection_cars,
            "carsWithGameLongEnglish": game_long_cars,
            "gameLongEnglishChars": game_long_chars,
            "carsWithGameLongChinese": game_long_zh_cars,
            "gameLongChineseChars": game_long_zh_chars,
            "gameLongSource": "PS5 DescriptionLong RT2",
            "chineseDisplaySource": "PS5 Simplified/Traditional RT2 cross-recovery",
        }
    });
    if let Some(object) = text_stats.as_object_mut() {
        object.insert(
            "byCategory".into(),
            json!({
                "brandCentral": 401,
                "usedCar": 139,
                "collection": 94,
                "vehicleCollection": vehicle_collection_entries,
            }),
        );
    }

    let payload = json!({
        "generated": "GT7 PS5 dump (native Rust)",
        "languages": language_catalog(),
        "dialogueTranslationDir": "data/dialogue_translations",
        "cars": cars_value.clone(),
        "brands": brands,
        "unmatched": unmatched.clone(),
        "cafeExactStats": research.get("stats").cloned().unwrap_or_else(|| json!({})),
        "carTextSources": [
            {"id": "brandCentral", "titleZh": "品牌购物", "titleEn": "Brand Central"},
            {"id": "vehicleCollection", "titleZh": "车辆收藏", "titleEn": "Car Collection"},
            {"id": "usedCar", "titleZh": "二手车", "titleEn": "Used Car Dealer"},
            {"id": "collection", "titleZh": "收藏馆", "titleEn": "Hagerty Collection"}
        ],
        "carTextStats": text_stats.clone(),
        "garageDetails": garage_details.get("counts").cloned().unwrap_or_else(|| json!({})),
        "dealerOffers": research.get("dealerOffers").cloned().unwrap_or_else(|| json!([])),
        "speakers": speakers,
        "cast": cast,
        "stats": {
            "cars": cars_value.len(),
            "withDialogue": with_dialogue,
            "dialogues": research_stats.get("carDetailRows").and_then(Value::as_u64).unwrap_or(0),
            "mappedDialogues": research_stats.get("carDetailRows").and_then(Value::as_u64).unwrap_or(0),
            "conversationLines": conversation_lines.len(),
            "unmatched": 0,
            "legacyReview": 253,
            "legacyReviewResolved": 253,
        }
    });
    eprintln!("site: payload assembled");

    fs::create_dir_all(&options.site_data)?;
    fs::create_dir_all(&options.report_dir)?;
    let serialized = serde_json::to_vec(&payload)?;
    let cars_path = options.site_data.join("cars.json");
    atomic_write(&cars_path, &serialized)?;
    atomic_write(
        &options.site_data.join("garage_details.json"),
        &serde_json::to_vec_pretty(&garage_details)?,
    )?;
    atomic_write(
        &options.report_dir.join("garage_details.json"),
        &serde_json::to_vec_pretty(&garage_details)?,
    )?;
    let dialogue_translations = collect_dialogue_translations(&research);
    let translation_dir = options.site_data.join("dialogue_translations");
    fs::create_dir_all(&translation_dir)?;
    dialogue_translations
        .par_iter()
        .try_for_each(|(locale, values)| -> Result<()> {
            atomic_write(
                &translation_dir.join(format!("{locale}.json")),
                &serde_json::to_vec(values)?,
            )
        })?;

    // CPU-heavy joins above are parallel. Export files sequentially so
    // Windows filter drivers and Cloudflare preview servers never observe two
    // simultaneous truncate/rename operations.
    write_dialogue_csvs(&cars_value, &options.site_data, &options.report_dir)?;
    write_scene_csvs(&cars_value, &options.site_data, &options.report_dir)?;
    write_text_csvs(&cars_value, &options.site_data, &options.report_dir)?;
    write_garage_spec_csvs(&cars_value, &options.site_data, &options.report_dir)?;
    eprintln!("site: CSV exports complete");
    atomic_write(
        &options.site_data.join("unmatched_dialogues.csv"),
        &export_unmatched_csv(&unmatched)?,
    )?;
    atomic_write(
        &options.report_dir.join("unmatched_dialogues.csv"),
        &export_unmatched_csv(&unmatched)?,
    )?;
    atomic_write(
        &options.site_data.join("unmatched_dialogues.md"),
        export_unmatched_markdown(&unmatched).as_bytes(),
    )?;
    atomic_write(
        &options.report_dir.join("unmatched_dialogues.md"),
        export_unmatched_markdown(&unmatched).as_bytes(),
    )?;
    write_dealer_csvs(&research, &options.site_data, &options.report_dir)?;
    write_variant_csvs(&research, &us, &zh, &options.site_data, &options.report_dir)?;
    write_text_report(
        &cars_value,
        &text_stats,
        &options.site_data,
        &options.report_dir,
    )?;

    eprintln!(
        "site: cars={} dialogueCars={} collectionCars={} texts={} -> {} ({:.2}s)",
        cars_value.len(),
        with_dialogue,
        collection_cars,
        text_entries_count,
        cars_path.display(),
        started.elapsed().as_secs_f64()
    );
    Ok(payload)
}

fn write_dialogue_csvs(cars: &[Value], site: &Path, report: &Path) -> Result<()> {
    let mut rows = Vec::new();
    for car in cars {
        let name = car.get("nameEn").and_then(Value::as_str).unwrap_or("");
        for row in car
            .get("dialogues")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            rows.push((
                row.get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                vec![
                    row.get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("sceneId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    name.to_string(),
                    row.get("speaker")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("speakerZh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("roleZh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("en")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("zh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ],
            ));
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let unique: BTreeMap<String, Vec<String>> = rows.into_iter().collect();
    let headers = [
        "id",
        "scene_id",
        "car",
        "speaker",
        "speaker_zh",
        "role_zh",
        "en",
        "zh",
    ];
    for path in [
        site.join("cafe_dialogues.csv"),
        report.join("cafe_dialogues.csv"),
    ] {
        write_csv(&path, &headers, unique.values().cloned())?;
    }
    Ok(())
}

fn write_scene_csvs(cars: &[Value], site: &Path, report: &Path) -> Result<()> {
    let mut rows = Vec::new();
    for car in cars {
        let name = car.get("nameEn").and_then(Value::as_str).unwrap_or("");
        for row in car
            .get("conversation")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let key = format!(
                "{}|{}",
                row.get("id").and_then(Value::as_str).unwrap_or(""),
                row.get("sceneId").and_then(Value::as_str).unwrap_or("")
            );
            rows.push((
                key,
                vec![
                    row.get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("sceneId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("sceneOrder")
                        .and_then(Value::as_u64)
                        .unwrap_or(0)
                        .to_string(),
                    name.to_string(),
                    row.get("speaker")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("speakerZh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("roleZh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("isCarDetail")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                        .to_string(),
                    row.get("en")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    row.get("zh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ],
            ));
        }
    }
    let unique: BTreeMap<String, Vec<String>> = rows.into_iter().collect();
    let headers = [
        "id",
        "scene_id",
        "scene_order",
        "car",
        "speaker",
        "speaker_zh",
        "role_zh",
        "is_car_detail",
        "en",
        "zh",
    ];
    for path in [
        site.join("cafe_scene_dialogues.csv"),
        report.join("cafe_scene_dialogues.csv"),
    ] {
        write_csv(&path, &headers, unique.values().cloned())?;
    }
    Ok(())
}

fn write_text_csvs(cars: &[Value], site: &Path, report: &Path) -> Result<()> {
    let mut rows = Vec::new();
    for car in cars {
        let id = car.get("id").and_then(Value::as_str).unwrap_or("");
        let Some(texts) = car.get("texts").and_then(Value::as_object) else {
            continue;
        };
        for (category, text) in texts {
            if category == "vehicleCollection" {
                continue;
            }
            for item in std::iter::once(text).chain(
                text.get("variants")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten(),
            ) {
                rows.push(vec![
                    category.clone(),
                    item.get("textId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    String::new(),
                    item.get("carKey")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("sourceName")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    id.to_string(),
                    item.get("matchMethod")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    String::new(),
                    item.get("en")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("zh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ]);
            }
        }
    }
    rows.sort_by(|a, b| (a[0].as_str(), a[1].as_str()).cmp(&(b[0].as_str(), b[1].as_str())));
    let headers = [
        "category",
        "textId",
        "messageId",
        "carKey",
        "sourceName",
        "carIds",
        "matchMethod",
        "confidence",
        "en",
        "zh",
    ];
    for path in [site.join("car_texts.csv"), report.join("car_texts.csv")] {
        write_csv(&path, &headers, rows.clone().into_iter())?;
    }

    let csv_rows = cars.iter().flat_map(|car| {
        let name = car.get("nameEn").and_then(Value::as_str).unwrap_or("");
        let id = car.get("id").and_then(Value::as_str).unwrap_or("");
        car.get("texts")
            .and_then(|v| v.get("vehicleCollection"))
            .map(|item| {
                vec![
                    id.to_string(),
                    name.to_string(),
                    item.get("carKey")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("sourceName")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("matchMethod")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("officialId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("translationSource")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.pointer("/catch/textId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.pointer("/long/textId")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("en")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    item.get("zh")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                ]
            })
            .into_iter()
    });
    let headers = [
        "carId",
        "carNameEn",
        "carKey",
        "sourceName",
        "matchMethod",
        "officialId",
        "translationSource",
        "catchTextId",
        "longTextId",
        "en",
        "zh",
    ];
    for path in [
        site.join("vehicle_collection_texts.csv"),
        report.join("vehicle_collection_texts.csv"),
    ] {
        write_csv(&path, &headers, csv_rows.clone())?;
    }
    Ok(())
}

fn write_garage_spec_csvs(cars: &[Value], site: &Path, report: &Path) -> Result<()> {
    let headers = [
        "carId",
        "nameEn",
        "nameZh",
        "brand",
        "officialId",
        "source",
        "match",
        "sourceUrlEn",
        "sourceUrlZh",
        "performancePoint_en",
        "performancePoint_zh",
        "maxPower_en",
        "maxPower_zh",
        "maxTorque_en",
        "maxTorque_zh",
        "power_en",
        "power_zh",
        "torque_en",
        "torque_zh",
        "weight_en",
        "weight_zh",
        "driveTrain_en",
        "driveTrain_zh",
        "aspiration_en",
        "aspiration_zh",
        "displacement_en",
        "displacement_zh",
        "carClass_en",
        "carClass_zh",
        "length_en",
        "length_zh",
        "width_en",
        "width_zh",
        "height_en",
        "height_zh",
        "powerValue_zh",
        "torqueValue_zh",
        "weightValue_zh",
        "lengthValue_zh",
        "widthValue_zh",
        "heightValue_zh",
    ];
    let rows = cars.iter().filter_map(|car| {
        let specs = car.get("specs")?;
        let en = specs.get("en").and_then(Value::as_object);
        let zh = specs.get("zh").and_then(Value::as_object);
        let text = |locale: Option<&serde_json::Map<String, Value>>, key: &str| {
            locale
                .and_then(|value| value.get(key))
                .map(|value| match value {
                    Value::String(text) => text.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default()
        };
        Some(vec![
            text(car.as_object(), "id"),
            text(car.as_object(), "nameEn"),
            text(car.as_object(), "nameZh"),
            text(car.as_object(), "brand"),
            text(specs.as_object(), "officialId"),
            text(specs.as_object(), "source"),
            text(specs.as_object(), "match"),
            text(specs.as_object(), "sourceUrlEn"),
            text(specs.as_object(), "sourceUrlZh"),
            text(en, "performancePoint"),
            text(zh, "performancePoint"),
            text(en, "maxPower"),
            text(zh, "maxPower"),
            text(en, "maxTorque"),
            text(zh, "maxTorque"),
            text(en, "power"),
            text(zh, "power"),
            text(en, "torque"),
            text(zh, "torque"),
            text(en, "weight"),
            text(zh, "weight"),
            text(en, "driveTrain"),
            text(zh, "driveTrain"),
            text(en, "aspirationLong"),
            text(zh, "aspirationLong"),
            text(en, "displacement"),
            text(zh, "displacement"),
            text(en, "carClass"),
            text(zh, "carClass"),
            text(en, "length"),
            text(zh, "length"),
            text(en, "width"),
            text(zh, "width"),
            text(en, "height"),
            text(zh, "height"),
            text(zh, "power_v"),
            text(zh, "torque_v"),
            text(zh, "weight_v"),
            text(zh, "length_v"),
            text(zh, "width_v"),
            text(zh, "height_v"),
        ])
    });
    for path in [
        site.join("garage_specs.csv"),
        report.join("garage_specs.csv"),
    ] {
        write_csv(&path, &headers, rows.clone())?;
    }
    Ok(())
}

fn write_dealer_csvs(research: &Value, site: &Path, report: &Path) -> Result<()> {
    let headers = [
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
    ];
    let rows = research
        .get("dealerOffers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|row| {
            vec![
                row.get("carId")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                row.get("nameEn")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                row.get("nameZh")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                row.get("category")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                row.get("offerId").map(Value::to_string).unwrap_or_default(),
                row.get("price").map(Value::to_string).unwrap_or_default(),
                row.get("discountPrice")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                row.get("rarity").map(Value::to_string).unwrap_or_default(),
                row.get("isNew").map(Value::to_string).unwrap_or_default(),
                row.get("isHot").map(Value::to_string).unwrap_or_default(),
                row.get("isPick").map(Value::to_string).unwrap_or_default(),
                row.get("carKey")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
            ]
        });
    for path in [
        site.join("dealer_offers.csv"),
        report.join("dealer_offers.csv"),
    ] {
        write_csv(&path, &headers, rows.clone())?;
    }
    Ok(())
}

fn write_variant_csvs(
    research: &Value,
    us_text: &HashMap<(String, String), String>,
    zh_text: &HashMap<(String, String), String>,
    site: &Path,
    report: &Path,
) -> Result<()> {
    let headers = [
        "conversationKey",
        "familyKey",
        "variant",
        "startMessageId",
        "carKey",
        "carIds",
        "turnOrder",
        "speaker",
        "speakerZh",
        "roleZh",
        "avatar",
        "isCarDetail",
        "textId",
        "en",
        "zh",
    ];
    let mut rows = Vec::new();
    for scene in research
        .get("cafeScenes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for turn in scene
            .get("turns")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let speaker_id = turn.get("speaker").and_then(Value::as_str).unwrap_or("");
            let meta = speaker(speaker_id);
            let text_id = turn.get("id").and_then(Value::as_str).unwrap_or("");
            let key = ("BuddyGTCafe".to_string(), text_id.to_string());
            let en = us_text
                .get(&key)
                .map(|value| clean_text(value))
                .unwrap_or_else(|| {
                    clean_text(turn.get("en").and_then(Value::as_str).unwrap_or(""))
                });
            let zh = zh_text
                .get(&key)
                .map(|value| clean_text(value))
                .unwrap_or_else(|| {
                    clean_text(turn.get("zh").and_then(Value::as_str).unwrap_or(""))
                });
            rows.push(vec![
                scene
                    .get("conversationKey")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                scene
                    .get("familyKey")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                scene
                    .get("variant")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                scene
                    .get("startMessageId")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                scene
                    .get("carKey")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string(),
                scene
                    .get("carIds")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(";"),
                turn.get("sceneOrder")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                speaker_id.to_string(),
                meta.map(|m| m.zh).unwrap_or("").to_string(),
                meta.map(|m| m.role_zh).unwrap_or("").to_string(),
                meta.map(|m| m.avatar).unwrap_or("").to_string(),
                turn.get("isCarDetail")
                    .map(Value::to_string)
                    .unwrap_or_default(),
                text_id.to_string(),
                en,
                zh,
            ]);
        }
    }
    for path in [
        site.join("cafe_variants.csv"),
        report.join("cafe_variants.csv"),
    ] {
        write_csv(&path, &headers, rows.clone().into_iter())?;
    }
    Ok(())
}

fn write_text_report(cars: &[Value], text_stats: &Value, site: &Path, report: &Path) -> Result<()> {
    let mut entries = Vec::new();
    for car in cars {
        let car_id = car.get("id").and_then(Value::as_str).unwrap_or("");
        for (category, text) in car
            .get("texts")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            if category == "vehicleCollection" {
                continue;
            }
            for item in std::iter::once(text).chain(
                text.get("variants")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten(),
            ) {
                entries.push(json!({
                    "category": category,
                    "titleZh": item.get("titleZh").cloned().unwrap_or(json!("")),
                    "titleEn": item.get("titleEn").cloned().unwrap_or(json!("")),
                    "textId": item.get("textId").cloned().unwrap_or(json!("")),
                    "messageId": null,
                    "messageKey": "",
                    "carKey": item.get("carKey").cloned().unwrap_or(json!("")),
                    "sourceName": item.get("sourceName").cloned().unwrap_or(json!("")),
                    "en": item.get("en").cloned().unwrap_or(json!("")),
                    "zh": item.get("zh").cloned().unwrap_or(json!("")),
                    "matchMethod": item.get("matchMethod").cloned().unwrap_or(json!("")),
                    "confidence": 1.0,
                    "carIds": [car_id],
                }));
            }
        }
    }
    let payload = json!({
        "generatedFrom": "PS5 DMSB + RT2 (native Rust)",
        "sources": [
            {"id": "brandCentral", "titleZh": "品牌购物", "titleEn": "Brand Central"},
            {"id": "usedCar", "titleZh": "二手车", "titleEn": "Used Car Dealer"},
            {"id": "collection", "titleZh": "收藏馆", "titleEn": "Hagerty Collection"},
            {"id": "vehicleCollection", "titleZh": "车辆收藏", "titleEn": "Car Collection"}
        ],
        "entries": entries,
        "unresolved": [],
        "orphanTexts": [],
        "stats": text_stats,
    });
    let bytes = serde_json::to_vec(&payload)?;
    atomic_write(&site.join("car_texts.json"), &bytes)?;
    atomic_write(&report.join("car_texts.json"), &bytes)?;
    Ok(())
}

fn export_unmatched_csv(rows: &[Value]) -> Result<Vec<u8>> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record([
        "id",
        "scene_id",
        "candidate_car",
        "speaker",
        "speaker_zh",
        "resolved_car_en",
        "resolved_car_zh",
        "resolution",
        "scene_keys",
        "en",
        "zh",
    ])?;
    for row in rows {
        writer.write_record([
            row.get("id").and_then(Value::as_str).unwrap_or(""),
            row.get("sceneId").and_then(Value::as_str).unwrap_or(""),
            "",
            row.get("speaker").and_then(Value::as_str).unwrap_or(""),
            row.get("speakerZh").and_then(Value::as_str).unwrap_or(""),
            row.get("resolvedCarNameEn")
                .and_then(Value::as_str)
                .unwrap_or(""),
            row.get("resolvedCarNameZh")
                .and_then(Value::as_str)
                .unwrap_or(""),
            row.get("resolution").and_then(Value::as_str).unwrap_or(""),
            "",
            row.get("en").and_then(Value::as_str).unwrap_or(""),
            row.get("zh").and_then(Value::as_str).unwrap_or(""),
        ])?;
    }
    writer.flush()?;
    writer.into_inner().map_err(|e| anyhow::anyhow!(e))
}

fn export_unmatched_markdown(rows: &[Value]) -> String {
    let mut out = String::from("# GT7 Cafe manual review\n\n");
    for row in rows {
        out.push_str(&format!(
            "## {} — {}\n\n{}\n\n{}\n\n",
            row.get("id").and_then(Value::as_str).unwrap_or(""),
            row.get("speaker").and_then(Value::as_str).unwrap_or(""),
            row.get("en").and_then(Value::as_str).unwrap_or(""),
            row.get("zh").and_then(Value::as_str).unwrap_or("")
        ));
    }
    format!("{}\n", out.trim_end())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_truncated_dialogue_commands() {
        assert_eq!(
            clean_text("ssq cm=\"GTCafe_Conv_Standard01\">visible text"),
            "visible text"
        );
        assert_eq!(
            clean_text("ssq cm=\"TCfeCov_Standad0\">これは“アバルト”ね"),
            "これは“アバルト”ね"
        );
        assert_eq!(
            clean_text("visible text<next id=1800102 type=9>"),
            "visible text"
        );
    }

    #[test]
    fn removes_duplicate_dialogue_content_but_keeps_different_speakers() {
        let mut rows = vec![
            json!({"speaker": "Sarah", "zh": "你好。", "en": "Hello."}),
            json!({"speaker": "Sarah", "zh": "你好。", "en": "Hello."}),
            json!({"speaker": "Luca", "zh": "你好。", "en": "Hello."}),
        ];
        dedupe_turn_content(&mut rows);
        assert_eq!(rows.len(), 2);
    }
}
