//! Native GT Cafe menu-book builder.
//!
//! Game-owned quest and collection rows are re-read from DMSB on every run.
//! Existing editorial fields and hand-audited car labels are retained from the
//! checked-in `menu_books.json` cache, then rejoined to the freshly extracted
//! car list. New rows discovered after a game update are emitted immediately
//! with their game keys and cover-container evidence.

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use crate::store::{cache_key, fnv1a};

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
            bail!("{}: invalid DMSB layout", path.display());
        }
        let row_size = (table - 0x30) / count;
        if row_size == 0 || 0x30 + row_size * count != table {
            bail!("{}: unsupported DMSB row size", path.display());
        }
        let rows = (0..count)
            .map(|i| data[0x30 + i * row_size..0x30 + (i + 1) * row_size].to_vec())
            .collect();
        Ok(Self { data, rows, table })
    }

    fn string(&self, row: &[u8], offset: usize) -> Result<String> {
        let start = self.table + u32le(row, offset)? as usize;
        if start >= self.data.len() {
            return Ok(String::new());
        }
        let end = self.data[start..]
            .iter()
            .position(|byte| *byte == 0)
            .map(|n| start + n)
            .unwrap_or(self.data.len());
        Ok(String::from_utf8_lossy(&self.data[start..end]).into_owned())
    }

    fn u32(&self, row: &[u8], offset: usize) -> Result<u32> {
        u32le(row, offset)
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

fn norm(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn bare_name<'a>(name: &'a str, brand: &str) -> &'a str {
    name.strip_prefix(brand)
        .and_then(|rest| rest.strip_prefix(' '))
        .unwrap_or(name)
}

fn score(query: &str, car: &Value) -> f64 {
    static YEAR: OnceLock<Regex> = OnceLock::new();
    let query = norm(query);
    let name = car.get("nameEn").and_then(Value::as_str).unwrap_or("");
    let brand = car.get("brand").and_then(Value::as_str).unwrap_or("");
    let full = norm(name);
    let core = norm(bare_name(name, brand));
    if query == full {
        return 1000.0;
    }
    if query == core {
        return 900.0;
    }
    let year = |value: &str| {
        YEAR.get_or_init(|| Regex::new(r"\b(?:19|20)\d{2}\b").expect("year regex"))
            .find(value)
            .map(|m| m.as_str().parse::<i32>().unwrap_or(0))
    };
    let year_bonus = match (year(&query), year(&full)) {
        (Some(a), Some(b)) if a == b => 60.0,
        (Some(a), Some(b)) if (a - b).abs() <= 1 => 18.0,
        (Some(_), Some(_)) => -50.0,
        _ => 0.0,
    };
    let stop: HashSet<&str> = ["the", "and", "edition", "package", "coupe", "car"]
        .into_iter()
        .collect();
    let query_tokens: HashSet<&str> = query
        .split_whitespace()
        .filter(|token| !stop.contains(token))
        .collect();
    let core_tokens: HashSet<&str> = core
        .split_whitespace()
        .filter(|token| !stop.contains(token))
        .collect();
    let overlap = query_tokens.intersection(&core_tokens).count();
    let coverage = overlap as f64 / query_tokens.len().max(1) as f64;
    year_bonus + coverage * 100.0 + overlap as f64 * 8.0
        - (query_tokens.len() as f64 - core_tokens.len() as f64).abs()
}

fn resolve_car<'a>(query: &str, cars: &'a [Value]) -> Option<&'a Value> {
    cars.iter()
        .map(|car| (score(query, car), car))
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .filter(|(score, _)| *score >= 55.0)
        .map(|(_, car)| car)
}

fn car_entry(query: &str, cars: &[Value], unmatched: &mut Vec<Value>) -> Value {
    if let Some(car) = resolve_car(query, cars) {
        json!({
            "id": car.get("id").cloned().unwrap_or(json!("")),
            "nameEn": car.get("nameEn").cloned().unwrap_or(json!(query)),
            "nameZh": car.get("nameZh").cloned().unwrap_or(json!(query)),
            "brand": car.get("brand").cloned().unwrap_or(json!("Unknown")),
            "image": car.get("image").cloned().unwrap_or(json!("")),
            "brandLogo": car.get("brandLogo").cloned().unwrap_or(json!("")),
            "matchMethod": "game-name-table",
        })
    } else {
        unmatched.push(json!({"query": query}));
        json!({
            "id": "",
            "nameEn": query,
            "nameZh": query,
            "brand": "Unknown",
            "image": "",
            "brandLogo": "",
            "matchMethod": "unresolved",
        })
    }
}

fn asset_source(contents: &Path, asset_path: &str) -> (String, &'static str) {
    let no_lead = asset_path.trim_start_matches('/');
    let variants = [
        asset_path.to_string(),
        no_lead.to_string(),
        asset_path.to_lowercase(),
        no_lead.to_lowercase(),
    ];
    for variant in &variants {
        let rel = cache_key(fnv1a(variant.as_bytes()));
        if contents.join(&rel).is_file() {
            return (rel, "present");
        }
    }
    (cache_key(fnv1a(asset_path.as_bytes())), "missing")
}

fn load_template(data_dir: &Path) -> Vec<Value> {
    fs::read(data_dir.join("menu_books.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|payload| payload.get("records").and_then(Value::as_array).cloned())
        .unwrap_or_default()
}

fn find_dms_dir(root: &Path) -> Result<std::path::PathBuf> {
    if root.join("Quests.dmsb").is_file() {
        return Ok(root.to_path_buf());
    }
    let mut candidates = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.join("Quests.dmsb").is_file())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .pop()
        .with_context(|| format!("no Quests.dmsb below {}", root.display()))
}

fn new_main_record(dmsb: &Dmsb, row: &[u8]) -> Result<Option<Value>> {
    let quest_id = dmsb.u32(row, 0)?;
    if quest_id < 600 || dmsb.u32(row, 0x10)? > 51 {
        return Ok(None);
    }
    let order = dmsb.u32(row, 0x10)?;
    let key = dmsb.string(row, 8)?;
    let pattern = Regex::new(r"(?i)MB(\d{3})").expect("MB regex");
    let Some(capture) = pattern.captures(&key) else {
        return Ok(None);
    };
    let category = if order <= 39 { "main" } else { "bonus" };
    let code = format!("MB{}", &capture[1]);
    Ok(Some(json!({
        "id": format!("{category}-{order:03}"),
        "category": category,
        "categoryZh": if category == "main" { "主线菜单书" } else { "奖励菜单书" },
        "categoryEn": if category == "main" { "Main Menu Book" } else { "Bonus Menu Book" },
        "number": order,
        "displayNumber": format!("No.{order}"),
        "internalCode": code,
        "titleEn": key,
        "titleZh": key,
        "collectionNameEn": "",
        "collectionNameZh": "",
        "descriptionEn": "",
        "descriptionZh": "",
        "requirementsEn": "",
        "requirementsZh": "",
        "rewardEn": "Unknown",
        "rewardZh": "未知",
        "collectorLevel": 0,
        "cars": [],
        "manufacturers": [],
        "coverAsset": "",
        "coverThumbAsset": "",
        "coverRel": "",
        "coverThumbRel": "",
        "coverStatus": "missing",
        "coverContainerStatus": "missing",
        "cover": "",
        "thumbnail": "",
        "storyEn": "",
        "storyZh": "",
        "sources": {"questId": quest_id, "questKey": key, "newNativeRow": true},
    })))
}

fn new_extra_record(collection: &Dmsb, row: &[u8], contents: &Path) -> Result<Option<Value>> {
    let id = collection.u32(row, 0)?;
    if id < 31 {
        return Ok(None);
    }
    let key = collection.string(row, 8)?;
    let pattern = Regex::new(r"(?i)CC(\d{3})").expect("CC regex");
    let Some(capture) = pattern.captures(&key) else {
        return Ok(None);
    };
    let order = collection.u32(row, 0x98)?;
    let level = collection.u32(row, 0x94)?;
    let cover_asset = format!("/piece/raw/collection_image/{key}.img");
    let thumb_asset = format!("/piece/raw/collection_thumbnail/{key}.img");
    let (cover_rel, status) = asset_source(contents, &cover_asset);
    let (thumb_rel, _) = asset_source(contents, &thumb_asset);
    Ok(Some(json!({
        "id": format!("extra-{order:03}"),
        "category": "extra",
        "categoryZh": "追加菜单 / 收藏菜单",
        "categoryEn": "Extra Menu",
        "number": order,
        "displayNumber": format!("EM {order}"),
        "internalCode": format!("CC{}", &capture[1]),
        "titleEn": format!("Collection: CC{}", &capture[1]),
        "titleZh": format!("收藏：CC{}", &capture[1]),
        "collectionNameEn": "",
        "collectionNameZh": "",
        "descriptionEn": "",
        "descriptionZh": "",
        "requirementsEn": "",
        "requirementsZh": "",
        "rewardEn": "Unknown",
        "rewardZh": "未知",
        "collectorLevel": level,
        "cars": [],
        "manufacturers": [],
        "coverAsset": cover_asset,
        "coverThumbAsset": thumb_asset,
        "coverRel": cover_rel,
        "coverThumbRel": thumb_rel,
        "coverStatus": if status == "present" { "container-only" } else { "missing" },
        "coverContainerStatus": status,
        "cover": "",
        "thumbnail": "",
        "storyEn": "",
        "storyZh": "",
        "sources": {"carCollectionId": id, "collectionKey": key, "newNativeRow": true},
    })))
}

fn join_names(values: Option<&Vec<Value>>, key: &str) -> String {
    values
        .into_iter()
        .flatten()
        .filter_map(|item| {
            item.get(key)
                .and_then(Value::as_str)
                .or_else(|| item.as_str())
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn build(
    data_dir: &Path,
    dmsb_dir: &Path,
    contents: &Path,
    report_dir: &Path,
) -> Result<Value> {
    let started = std::time::Instant::now();
    let cars_payload: Value = serde_json::from_slice(&fs::read(data_dir.join("cars.json"))?)?;
    let cars = cars_payload
        .get("cars")
        .and_then(Value::as_array)
        .cloned()
        .context("cars.json has no cars")?;
    let mut records = load_template(data_dir);
    let existing: HashSet<String> = records
        .iter()
        .filter_map(|row| row.get("id")?.as_str().map(str::to_string))
        .collect();
    let unmatched = std::sync::Mutex::new(Vec::<Value>::new());
    let dmsb_dir = find_dms_dir(dmsb_dir)?;

    records.par_iter_mut().for_each(|record| {
        let Some(object) = record.as_object_mut() else {
            return;
        };
        let original = object
            .get("cars")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if !original.is_empty() {
            let mut local_unmatched = Vec::new();
            let refreshed = original
                .iter()
                .map(|car| {
                    let query = car
                        .get("nameEn")
                        .or_else(|| car.get("nameZh"))
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    car_entry(query, &cars, &mut local_unmatched)
                })
                .collect::<Vec<_>>();
            unmatched.lock().expect("unmatched").extend(local_unmatched);
            object.insert("cars".into(), Value::Array(refreshed));
        }
        if let Some(cover_asset) = object.get("coverAsset").and_then(Value::as_str) {
            if !cover_asset.is_empty() {
                let (rel, status) = asset_source(contents, cover_asset);
                object.insert("coverRel".into(), json!(rel));
                object.insert("coverContainerStatus".into(), json!(status));
            }
        }
    });

    let quests = Dmsb::parse(&dmsb_dir.join("Quests.dmsb"))?;
    for row in &quests.rows {
        if let Some(value) = new_main_record(&quests, row)? {
            let id = value.get("id").and_then(Value::as_str).unwrap_or("");
            if !existing.contains(id) {
                records.push(value);
            }
        }
    }
    let collection = Dmsb::parse(&dmsb_dir.join("CarCollection.dmsb"))?;
    for row in &collection.rows {
        if let Some(value) = new_extra_record(&collection, row, contents)? {
            let id = value.get("id").and_then(Value::as_str).unwrap_or("");
            if !existing.contains(id) {
                records.push(value);
            }
        }
    }

    if let Ok(bytes) = fs::read(data_dir.join("menu_cover_sources.json")) {
        if let Ok(manifest) = serde_json::from_slice::<Value>(&bytes) {
            let screenshots: HashMap<&str, &Value> = manifest
                .get("entries")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|entry| Some((entry.get("recordId")?.as_str()?, entry)))
                .collect();
            for record in &mut records {
                let id = record.get("id").and_then(Value::as_str).unwrap_or("");
                if let Some(screenshot) = screenshots.get(id) {
                    let object = record.as_object_mut().expect("record");
                    object.insert(
                        "cover".into(),
                        screenshot.get("cover").cloned().unwrap_or(json!("")),
                    );
                    object.insert("coverStatus".into(), json!("screenshot"));
                }
            }
        }
    }

    records.sort_by(|a, b| {
        let rank = |record: &Value| match record.get("category").and_then(Value::as_str) {
            Some("main") => 0,
            Some("bonus") => 1,
            _ => 2,
        };
        (
            rank(a),
            a.get("number").and_then(Value::as_u64).unwrap_or(0),
        )
            .cmp(&(
                rank(b),
                b.get("number").and_then(Value::as_u64).unwrap_or(0),
            ))
    });
    let count_category = |category: &str| {
        records
            .iter()
            .filter(|r| r.get("category").and_then(Value::as_str) == Some(category))
            .count()
    };
    let counts = json!({
        "total": records.len(),
        "main": count_category("main"),
        "bonus": count_category("bonus"),
        "extra": count_category("extra"),
        "withCars": records.iter().filter(|r| r.get("cars").and_then(Value::as_array).map(|v| !v.is_empty()).unwrap_or(false)).count(),
        "withCoverContainer": records.iter().filter(|r| r.get("coverContainerStatus").and_then(Value::as_str) == Some("present")).count(),
        "withScreenshot": records.iter().filter(|r| r.get("coverStatus").and_then(Value::as_str) == Some("screenshot")).count(),
        "withAnyCover": records.iter().filter(|r| r.get("coverStatus").and_then(Value::as_str) == Some("screenshot") || r.get("coverStatus").and_then(Value::as_str) == Some("container-only") || r.get("cover").and_then(Value::as_str).map(|v| !v.is_empty()).unwrap_or(false)).count(),
        "withDisplayCover": records.iter().filter(|r| r.get("cover").and_then(Value::as_str).map(|v| !v.is_empty()).unwrap_or(false)).count(),
        "resolvedCars": records.iter().flat_map(|r| r.get("cars").and_then(Value::as_array).into_iter().flatten()).filter(|car| car.get("matchMethod").and_then(Value::as_str) != Some("unresolved")).count(),
    });
    let payload = json!({
        "generatedFrom": "GT7 PS5 DMSB + local content store (native Rust)",
        "gameVersion": "local DMSB + checked-in editorial cache",
        "counts": counts,
        "records": records,
        "unmatchedQueries": unmatched.into_inner().unwrap_or_default(),
    });
    fs::create_dir_all(data_dir)?;
    fs::create_dir_all(report_dir)?;
    let bytes = serde_json::to_vec(&payload)?;
    atomic_write(&data_dir.join("menu_books.json"), &bytes)?;
    atomic_write(&report_dir.join("menu_books.json"), &bytes)?;

    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record([
        "category",
        "number",
        "displayNumber",
        "internalCode",
        "collectionNameEn",
        "collectionNameZh",
        "titleEn",
        "titleZh",
        "manufacturers",
        "carsEn",
        "carsZh",
        "collectorLevel",
        "rewardEn",
        "rewardZh",
        "coverAsset",
        "coverStatus",
        "descriptionEn",
        "descriptionZh",
    ])?;
    for record in payload["records"].as_array().into_iter().flatten() {
        writer.write_record([
            record.get("category").and_then(Value::as_str).unwrap_or(""),
            &record
                .get("number")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            record
                .get("displayNumber")
                .and_then(Value::as_str)
                .unwrap_or(""),
            record
                .get("internalCode")
                .and_then(Value::as_str)
                .unwrap_or(""),
            record
                .get("collectionNameEn")
                .and_then(Value::as_str)
                .unwrap_or(""),
            record
                .get("collectionNameZh")
                .and_then(Value::as_str)
                .unwrap_or(""),
            record.get("titleEn").and_then(Value::as_str).unwrap_or(""),
            record.get("titleZh").and_then(Value::as_str).unwrap_or(""),
            &join_names(
                record.get("manufacturers").and_then(Value::as_array),
                "nameEn",
            ),
            &join_names(record.get("cars").and_then(Value::as_array), "nameEn"),
            &join_names(record.get("cars").and_then(Value::as_array), "nameZh"),
            &record
                .get("collectorLevel")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .to_string(),
            record.get("rewardEn").and_then(Value::as_str).unwrap_or(""),
            record.get("rewardZh").and_then(Value::as_str).unwrap_or(""),
            record
                .get("coverAsset")
                .and_then(Value::as_str)
                .unwrap_or(""),
            record
                .get("coverStatus")
                .and_then(Value::as_str)
                .unwrap_or(""),
            record
                .get("descriptionEn")
                .and_then(Value::as_str)
                .unwrap_or(""),
            record
                .get("descriptionZh")
                .and_then(Value::as_str)
                .unwrap_or(""),
        ])?;
    }
    writer.flush()?;
    let csv_bytes = writer.into_inner().map_err(|e| anyhow::anyhow!(e))?;
    atomic_write(&data_dir.join("menu_books.csv"), &csv_bytes)?;
    atomic_write(&report_dir.join("menu_books.csv"), &csv_bytes)?;
    println!(
        "menu books: {} records, unresolved car labels {} -> {} ({:.2}s)",
        counts["total"],
        payload["unmatchedQueries"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0),
        data_dir.join("menu_books.json").display(),
        started.elapsed().as_secs_f64()
    );
    Ok(payload)
}
