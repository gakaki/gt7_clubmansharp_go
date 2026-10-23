//! Fail-fast integrity checks for all public research datasets.

use anyhow::{bail, Context, Result};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct Options {
    pub data_dir: PathBuf,
    pub reports_dir: PathBuf,
    pub research: PathBuf,
}

impl Default for Options {
    fn default() -> Self {
        let root = PathBuf::from(r"C:\gt7\gt7\_re");
        Self {
            data_dir: root.join("site/public/data"),
            reports_dir: root.join("reports"),
            research: root.join("reports/research_rust.json"),
        }
    }
}

fn load(path: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(path).with_context(|| path.display().to_string())?)
        .with_context(|| path.display().to_string())
}

fn check_clean(value: &Value, path: &str) -> Result<()> {
    match value {
        Value::String(text) => {
            if text.contains('\u{FFFD}') {
                bail!("{path}: contains U+FFFD");
            }
            if text
                .chars()
                .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
            {
                bail!("{path}: contains control characters");
            }
            if text
                .chars()
                .any(|ch| ('\u{E000}'..='\u{F8FF}').contains(&ch))
            {
                bail!("{path}: contains private-use characters");
            }
        }
        Value::Array(values) => {
            for (index, item) in values.iter().enumerate() {
                check_clean(item, &format!("{path}[{index}]"))?;
            }
        }
        Value::Object(values) => {
            for (key, item) in values {
                check_clean(item, &format!("{path}.{key}"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

pub fn run(options: &Options) -> Result<Value> {
    let cars = load(&options.data_dir.join("cars.json"))?;
    let garage = load(&options.data_dir.join("garage_details.json"))?;
    let menu = load(&options.data_dir.join("menu_books.json"))?;
    let research = load(&options.research)?;
    if research.get("cars").and_then(Value::as_array).map(Vec::len) != Some(591) {
        bail!("research_rust.json car count mismatch");
    }
    let car_rows = cars
        .get("cars")
        .and_then(Value::as_array)
        .context("cars.json.cars")?;
    if car_rows.len() != 591 {
        bail!("expected 591 cars, got {}", car_rows.len());
    }
    let ids = car_rows
        .iter()
        .filter_map(|car| car.get("id")?.as_str())
        .collect::<HashSet<_>>();
    if ids.len() != car_rows.len() {
        bail!("duplicate car ids detected");
    }
    let garage_counts = garage.get("counts").context("garage_details.json.counts")?;
    if garage_counts
        .get("withSpecifications")
        .and_then(Value::as_u64)
        != Some(580)
    {
        bail!("garage_details.json specification count mismatch");
    }
    if garage_counts
        .get("withVehicleTypes")
        .and_then(Value::as_u64)
        != Some(576)
    {
        bail!("garage_details.json vehicle-type count mismatch");
    }
    if garage.get("cars").and_then(Value::as_array).map(Vec::len) != Some(580) {
        bail!("garage_details.json car record count mismatch");
    }
    let languages = cars
        .get("languages")
        .and_then(Value::as_array)
        .context("languages")?;
    if languages.len() != 19 {
        bail!("expected 19 game languages, got {}", languages.len());
    }
    let language_ids = languages
        .iter()
        .filter_map(|item| item.get("id")?.as_str())
        .collect::<HashSet<_>>();
    let allowed: HashSet<&str> = ["brandCentral", "vehicleCollection", "usedCar", "collection"]
        .into_iter()
        .collect();
    let mut collection = 0usize;
    let mut spec_cars = 0usize;
    let mut text_count = 0usize;
    let mut dialogue_cars = 0usize;
    let mut dialogue_lines = HashSet::new();
    let mut translation_cells = 0usize;
    for car in car_rows {
        let id = car.get("id").and_then(Value::as_str).unwrap_or("");
        let texts = car
            .get("texts")
            .and_then(Value::as_object)
            .with_context(|| format!("{id}: texts"))?;
        if let Some(category) = texts.keys().find(|value| !allowed.contains(value.as_str())) {
            bail!("{id}: invalid text category {category}");
        }
        for (category, text) in texts {
            text_count += 1 + text
                .get("variants")
                .and_then(Value::as_array)
                .map(Vec::len)
                .unwrap_or(0);
            if category == "vehicleCollection" {
                collection += 1;
                if !text
                    .get("catch")
                    .and_then(|v| v.get("en"))
                    .and_then(Value::as_str)
                    .map(|v| !v.is_empty())
                    .unwrap_or(false)
                    || !text
                        .get("catch")
                        .and_then(|v| v.get("zh"))
                        .and_then(Value::as_str)
                        .map(|v| !v.is_empty())
                        .unwrap_or(false)
                    || !text
                        .get("long")
                        .and_then(|v| v.get("en"))
                        .and_then(Value::as_str)
                        .map(|v| !v.is_empty())
                        .unwrap_or(false)
                    || !text
                        .get("long")
                        .and_then(|v| v.get("zh"))
                        .and_then(Value::as_str)
                        .map(|v| !v.is_empty())
                        .unwrap_or(false)
                {
                    bail!("{id}: incomplete bilingual vehicleCollection");
                }
            }
        }
        if let Some(specs) = car.get("specs") {
            let en = specs.get("en").and_then(Value::as_object);
            let zh = specs.get("zh").and_then(Value::as_object);
            if en.map(|value| value.is_empty()).unwrap_or(true)
                || zh.map(|value| value.is_empty()).unwrap_or(true)
            {
                bail!("{id}: incomplete bilingual garage specifications");
            }
            for key in [
                "performancePoint",
                "power",
                "torque",
                "weight",
                "driveTrain",
            ] {
                if en.and_then(|value| value.get(key)).is_none()
                    || zh.and_then(|value| value.get(key)).is_none()
                {
                    bail!("{id}: missing garage specification {key}");
                }
            }
            spec_cars += 1;
        }
        if car
            .get("dialogues")
            .and_then(Value::as_array)
            .map(|v| !v.is_empty())
            .unwrap_or(false)
            || car
                .get("conversation")
                .and_then(Value::as_array)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
        {
            dialogue_cars += 1;
        }
        for turn in car
            .get("conversation")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(id) = turn.get("id").and_then(Value::as_str) {
                let scene = turn.get("sceneId").and_then(Value::as_str).unwrap_or("");
                dialogue_lines.insert((id.to_string(), scene.to_string()));
            }
        }
    }
    if collection != 580 {
        bail!("expected 580 Car Collection cars, got {collection}");
    }
    if spec_cars != 580 {
        bail!("expected 580 cars with garage specifications, got {spec_cars}");
    }
    if dialogue_cars != 332 {
        bail!("expected 332 cars with cafe content, got {dialogue_cars}");
    }
    if dialogue_lines.len() != 2620 {
        bail!(
            "expected 2620 unique conversation lines, got {}",
            dialogue_lines.len()
        );
    }
    let mut language_hits = HashSet::new();
    for language in &language_ids {
        let path = options
            .data_dir
            .join("dialogue_translations")
            .join(format!("{language}.json"));
        let payload = load(&path)?;
        let rows = payload
            .as_object()
            .with_context(|| format!("{}: expected object", path.display()))?;
        if rows.is_empty() {
            bail!("{}: empty language dictionary", path.display());
        }
        translation_cells += rows.len();
        language_hits.insert(*language);
    }
    if language_hits != language_ids {
        bail!("dialogue language coverage mismatch");
    }
    let stats = cars
        .get("stats")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if stats.get("dialogues").and_then(Value::as_u64) != Some(444) {
        bail!("expected 444 CarDetail rows, got {stats:?}");
    }
    let dealer = cars
        .get("dealerOffers")
        .and_then(Value::as_array)
        .context("dealerOffers")?;
    if dealer.len() != 65 {
        bail!("expected 65 dealer offers, got {}", dealer.len());
    }

    let menu_counts = menu
        .get("counts")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if menu_counts.get("total").and_then(Value::as_u64) != Some(108) {
        bail!("expected 108 menu books, got {menu_counts:?}");
    }
    if menu_counts.get("extra").and_then(Value::as_u64) != Some(57) {
        bail!("expected 57 Extra Menus");
    }
    if menu.get("records").and_then(Value::as_array).map(Vec::len) != Some(108) {
        bail!("menu_books.json record count mismatch");
    }
    let avatar_dir = options.data_dir.join("../avatars/extracted");
    let avatars = fs::read_dir(&avatar_dir)
        .with_context(|| avatar_dir.display().to_string())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().and_then(|value| value.to_str()) == Some("png"))
        .count();
    if avatars != 19 {
        bail!("expected 19 decoded character portraits, got {avatars}");
    }

    check_clean(&cars, "cars.json")?;
    check_clean(&garage, "garage_details.json")?;
    check_clean(&menu, "menu_books.json")?;

    // Verify every public CSV parses and has a stable header. This catches
    // interrupted parallel writers before they reach Cloudflare.
    let csv_paths = [
        "cafe_dialogues.csv",
        "cafe_scene_dialogues.csv",
        "cafe_variants.csv",
        "car_texts.csv",
        "vehicle_collection_texts.csv",
        "garage_specs.csv",
        "dealer_offers.csv",
        "menu_books.csv",
        "unmatched_dialogues.csv",
    ];
    for name in csv_paths {
        let path = options.data_dir.join(name);
        if !path.is_file() {
            bail!("missing dataset {}", path.display());
        }
        let mut reader = csv::Reader::from_path(&path)?;
        reader
            .headers()
            .with_context(|| path.display().to_string())?;
        if reader.headers()?.is_empty() {
            bail!("{}: empty CSV header", path.display());
        }
    }
    let text_glyph = Regex::new(r"[�\u{E000}-\u{F8FF}]")?;
    for name in ["cars.json", "menu_books.json", "car_texts.json"] {
        let path = options.data_dir.join(name);
        if !path.is_file() {
            continue;
        }
        let text = fs::read_to_string(&path)?;
        if text_glyph.is_match(&text) {
            bail!(
                "{}: contains replacement/private-use glyphs",
                path.display()
            );
        }
    }

    let result = serde_json::json!({
        "cars": car_rows.len(),
        "withDialogue": dialogue_cars,
        "dialogues": 444,
        "conversationLines": dialogue_lines.len(),
        "languages": language_hits.len(),
        "translationCells": translation_cells,
        "carTextEntries": text_count,
        "vehicleCollectionCars": collection,
        "garageSpecCars": spec_cars,
        "garageVehicleTypeCars": garage_counts.get("withVehicleTypes").cloned().unwrap_or(Value::Null),
        "garageCountryCars": garage_counts.get("withCountry").cloned().unwrap_or(Value::Null),
        "dealerOffers": dealer.len(),
        "menuBooks": 108,
        "extraMenus": 57,
        "avatars": avatars,
        "reportsDir": options.reports_dir,
    });
    println!("PASS {}", serde_json::to_string(&result)?);
    Ok(result)
}
