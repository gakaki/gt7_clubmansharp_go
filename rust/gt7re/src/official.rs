//! Official GT7 car-list and menu-cover cache refreshers.
//!
//! Network work is parallelised with Rayon. Cached outputs are plain
//! JSON/WebP files so later site builds remain deterministic and offline.

use anyhow::{anyhow, bail, Context, Result};
use image::imageops::FilterType;
use image::GenericImageView;
use rayon::prelude::*;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const GT_BASE: &str = "https://www.gran-turismo.com";
const MENU_BASE: &str = "https://www.skymac.org/";
const MENU_INDEX: &str = "https://www.skymac.org/GT7-Cafe.htm";

fn agent(proxy: Option<&str>) -> Result<ureq::Agent> {
    let mut config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .user_agent("gt7-research-rust/1.0");
    if let Some(proxy) = proxy.filter(|value| !value.is_empty()) {
        config = config.proxy(Some(ureq::Proxy::new(proxy)?));
    }
    Ok(config.build().into())
}

fn fetch_text(agent: &ureq::Agent, url: &str) -> Result<String> {
    let mut response = agent.get(url).call().with_context(|| url.to_string())?;
    response
        .body_mut()
        .read_to_string()
        .with_context(|| url.to_string())
}

fn fetch_bytes(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>> {
    let mut response = agent.get(url).call().with_context(|| url.to_string())?;
    response
        .body_mut()
        .read_to_vec()
        .with_context(|| url.to_string())
}

fn capture<'a>(pattern: &Regex, source: &'a str, label: &str) -> Result<&'a str> {
    pattern
        .captures(source)
        .and_then(|capture| capture.get(1))
        .map(|matched| matched.as_str())
        .ok_or_else(|| anyhow!("asset not found: {label}"))
}

fn parse_descriptions(source: &str) -> Result<HashMap<String, (String, String)>> {
    let pattern = Regex::new(r"(?s)car(\d+):\{desc:`(.*?)`,hero:`(.*?)`,id:`car\d+`\}")
        .expect("description regex");
    let mut result = HashMap::new();
    for capture in pattern.captures_iter(source) {
        result.insert(
            format!("car{}", &capture[1]),
            (capture[2].to_string(), capture[3].to_string()),
        );
    }
    Ok(result)
}

fn parse_cars(source: &str) -> Result<HashMap<String, Value>> {
    let starts = Regex::new(r"car(\d+):\{").expect("car start regex");
    let found = starts
        .captures_iter(source)
        .filter_map(|capture| {
            Some((
                capture.get(0)?.start(),
                capture.get(0)?.end(),
                capture.get(1)?.as_str().to_string(),
            ))
        })
        .collect::<Vec<_>>();
    let field =
        Regex::new(r"(?s)([A-Za-z][A-Za-z0-9_]*):(?:`([^`]*)`|(-?\d+(?:\.\d+)?)|(true|false))")
            .expect("car field regex");
    let mut result = HashMap::new();
    for (index, (_, start, number)) in found.iter().enumerate() {
        let end = found
            .get(index + 1)
            .map(|(next, _, _)| *next)
            .or_else(|| {
                source[*start..]
                    .find("};export{e as Cars}")
                    .map(|p| *start + p)
            })
            .unwrap_or(source.len());
        let body = &source[*start..end];
        let id = format!("car{number}");
        let mut fields = serde_json::Map::new();
        fields.insert("id".into(), Value::String(id.clone()));
        for capture in field.captures_iter(body) {
            let value = if let Some(text) = capture.get(2) {
                Value::String(text.as_str().to_string())
            } else if let Some(number) = capture.get(3) {
                number
                    .as_str()
                    .parse::<f64>()
                    .map(|value| json!(value))
                    .unwrap_or(Value::Null)
            } else {
                Value::Bool(&capture[4] == "true")
            };
            fields.insert(capture[1].to_string(), value);
        }
        result.insert(id, Value::Object(fields));
    }
    Ok(result)
}

pub fn descriptions(out: &Path, proxy: Option<&str>) -> Result<()> {
    let agent = agent(proxy)?;
    let page = fetch_text(&agent, &format!("{GT_BASE}/cn/gt7/carlist"))?;
    let app_pattern = Regex::new(r#"src="(/common/dist/gt7/carlist/assets/index-[^"]+\.js)""#)
        .expect("app regex");
    let app_path = capture(&app_pattern, &page, "car-list app")?;
    let app_js = fetch_text(&agent, &format!("{GT_BASE}{app_path}"))?;
    let asset_root = app_path.rsplit_once('/').map(|v| v.0).unwrap_or("");

    let names = |locale: &str, kind: &str| -> Result<String> {
        let pattern =
            Regex::new(&format!(r"\./{kind}\.{locale}-([^`]+)\.js")).expect("asset name regex");
        Ok(capture(&pattern, &app_js, locale)?.to_string())
    };
    let cn_desc_name = names("cn", "descriptions")?;
    let us_desc_name = names("us", "descriptions")?;
    let cn_cars_name = names("cn", "cars")?;
    let us_cars_name = names("us", "cars")?;
    let urls = [
        format!("{GT_BASE}{asset_root}/descriptions.cn-{cn_desc_name}.js"),
        format!("{GT_BASE}{asset_root}/descriptions.us-{us_desc_name}.js"),
        format!("{GT_BASE}{asset_root}/cars.cn-{cn_cars_name}.js"),
        format!("{GT_BASE}{asset_root}/cars.us-{us_cars_name}.js"),
    ];
    let ((cn_desc, us_desc), (cn_cars, us_cars)) = rayon::join(
        || {
            rayon::join(
                || fetch_text(&agent, &urls[0]),
                || fetch_text(&agent, &urls[1]),
            )
        },
        || {
            rayon::join(
                || fetch_text(&agent, &urls[2]),
                || fetch_text(&agent, &urls[3]),
            )
        },
    );
    let cn_desc = parse_descriptions(&cn_desc?)?;
    let us_desc = parse_descriptions(&us_desc?)?;
    let cn_cars = parse_cars(&cn_cars?)?;
    let us_cars = parse_cars(&us_cars?)?;

    let mut ids = cn_desc
        .keys()
        .filter(|id| us_desc.contains_key(*id))
        .cloned()
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| id[3..].parse::<u32>().unwrap_or(u32::MAX));
    if ids.len() != 580 {
        bail!("expected 580 bilingual descriptions, got {}", ids.len());
    }
    let records = ids
        .par_iter()
        .map(|id| {
            let (desc_zh, hero_zh) = &cn_desc[id];
            let (desc_en, hero_en) = &us_desc[id];
            json!({
                "id": id,
                "nameEn": us_cars.get(id).and_then(|v| v.get("nameLong")).and_then(Value::as_str).unwrap_or(""),
                "nameZh": cn_cars.get(id).and_then(|v| v.get("nameLong")).and_then(Value::as_str).unwrap_or(""),
                "manufacturerId": us_cars.get(id).and_then(|v| v.get("manufacturerId")).and_then(Value::as_str).unwrap_or(""),
                "specsEn": us_cars.get(id).cloned().unwrap_or(Value::Null),
                "specsZh": cn_cars.get(id).cloned().unwrap_or(Value::Null),
                "heroEn": hero_en,
                "descEn": desc_en,
                "heroZh": hero_zh,
                "descZh": desc_zh,
                "sourceUrlEn": format!("{GT_BASE}/us/gt7/carlist/id/{id}"),
                "sourceUrlZh": format!("{GT_BASE}/cn/gt7/carlist/id/{id}"),
            })
        })
        .collect::<Vec<_>>();
    let payload = json!({
        "generatedFrom": "official Gran Turismo 7 car-list description modules",
        "sources": {"cn": urls[0], "us": urls[1]},
        "count": records.len(),
        "cars": records,
    });
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, serde_json::to_vec_pretty(&payload)?)?;
    println!("official descriptions: {} -> {}", ids.len(), out.display());
    Ok(())
}

pub fn images(source: &Path, cars_out: &Path, logos_out: &Path) -> Result<()> {
    let cars: Vec<Value> =
        serde_json::from_slice(&fs::read(source).with_context(|| source.display().to_string())?)?;
    fs::create_dir_all(cars_out)?;
    fs::create_dir_all(logos_out)?;
    let agent = Arc::new(agent(None)?);
    let mut jobs: Vec<(String, PathBuf)> = cars
        .iter()
        .filter_map(|car| {
            Some((
                car.get("thumb")?.as_str()?.to_string(),
                cars_out.join(format!("{}.png", car.get("id")?.as_str()?)),
            ))
        })
        .collect();
    let mut logos: BTreeMap<String, String> = BTreeMap::new();
    for car in &cars {
        if let (Some(brand), Some(logo)) = (
            car.get("brand").and_then(Value::as_str),
            car.get("logo").and_then(Value::as_str),
        ) {
            logos
                .entry(brand.to_string())
                .or_insert_with(|| logo.to_string());
        }
    }
    jobs.extend(logos.into_iter().map(|(brand, url)| {
        (
            url,
            logos_out.join(format!("{}.png", brand.replace([' ', '/'], "_"))),
        )
    }));
    let results = jobs
        .par_iter()
        .map(|(url, path)| download_image(&agent, url, path))
        .collect::<Vec<_>>();
    let ok = results.iter().filter(|value| **value).count();
    println!("official images: ok={} failed={}", ok, results.len() - ok);
    Ok(())
}

pub fn trim_logos(dir: &Path) -> Result<()> {
    let mut trimmed = 0usize;
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) != Some("png") {
            continue;
        }
        let Ok(image) = image::open(&path) else {
            continue;
        };
        let (width, height) = image.dimensions();
        let rgba = image.to_rgba8();
        let mut min_x = width;
        let mut min_y = height;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        for (x, y, pixel) in rgba.enumerate_pixels() {
            let [r, g, b, a] = pixel.0;
            if a > 12 && (r < 243 || g < 243 || b < 243) {
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        let cropped = if max_x >= min_x && max_y >= min_y {
            image.crop_imm(min_x, min_y, max_x - min_x + 1, max_y - min_y + 1)
        } else {
            image
        };
        cropped.resize(320, 160, FilterType::Lanczos3).save(&path)?;
        trimmed += 1;
    }
    println!("trimmed {} brand logos -> {}", trimmed, dir.display());
    Ok(())
}

fn download_image(agent: &ureq::Agent, url: &str, path: &Path) -> bool {
    if path.metadata().map(|m| m.len() > 500).unwrap_or(false) {
        return true;
    }
    for attempt in 0..3 {
        if let Ok(bytes) = fetch_bytes(agent, url) {
            if bytes.len() > 500 {
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                if fs::write(path, bytes).is_ok() {
                    return true;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(500 * (attempt + 1)));
    }
    false
}

fn decode_html(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

pub fn cars(source: &Path, out: &Path) -> Result<()> {
    let html = fs::read_to_string(source).with_context(|| source.display().to_string())?;
    let heading = Regex::new(r#"(?s)<h3 class="css-16jmhlm">(.*?)</h3>"#).expect("heading regex");
    let tags = Regex::new(r"<[^>]+>").expect("tag regex");
    let car_start = Regex::new(r#"data-car-id="(car\d+)""#).expect("car id regex");
    let name = Regex::new(r#"(?s)<span class="css-af5abj">(.*?)</span>"#).expect("name regex");
    let thumbnail = Regex::new(r#"src="(/common/dist/gt7/carlist/car_thumbnails/[^"]+)""#)
        .expect("thumbnail regex");
    let logo_re = Regex::new(r#"/common/dist/gt7/carlist/tuner_logos/[^"']+"#).expect("logo regex");
    let headings = heading
        .captures_iter(&html)
        .filter_map(|capture| {
            Some((
                capture.get(0)?.start(),
                decode_html(&tags.replace_all(&capture[1], ""))
                    .trim()
                    .to_string(),
            ))
        })
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    for (index, (start, brand)) in headings.iter().enumerate() {
        let end = headings
            .get(index + 1)
            .map(|(next, _)| *next)
            .unwrap_or(html.len());
        let segment = &html[*start..end];
        let mut before_start = start.saturating_sub(4000);
        while before_start < html.len() && !html.is_char_boundary(before_start) {
            before_start += 1;
        }
        let before = &html[before_start..*start];
        let logo = logo_re
            .find_iter(before)
            .last()
            .map(|value| format!("{GT_BASE}{}", value.as_str()));
        let ids = car_start
            .captures_iter(segment)
            .filter_map(|capture| Some((capture.get(0)?.start(), capture[1].to_string())))
            .collect::<Vec<_>>();
        for (car_index, (car_start, id)) in ids.iter().enumerate() {
            let car_end = ids
                .get(car_index + 1)
                .map(|(next, _)| *next)
                .unwrap_or(segment.len());
            let block = &segment[*car_start..car_end];
            let Some(name_match) = name.captures(block) else {
                continue;
            };
            let thumb = thumbnail
                .captures(block)
                .map(|capture| format!("{GT_BASE}{}", &capture[1]))
                .unwrap_or_else(|| {
                    format!("{GT_BASE}/common/dist/gt7/carlist/car_thumbnails/{id}.png")
                });
            output.push(json!({
                "brand": brand,
                "id": id,
                "name": decode_html(&tags.replace_all(&name_match[1], "")).trim(),
                "thumb": thumb,
                "logo": logo,
            }));
        }
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, serde_json::to_vec_pretty(&output)?)?;
    println!("official cars: {} -> {}", output.len(), out.display());
    Ok(())
}

fn classify_heading(heading: &str) -> Option<(&'static str, u32)> {
    let spaces = Regex::new(r"\s+").expect("space regex");
    let decoded = decode_html(heading);
    let heading = spaces.replace_all(&decoded, " ");
    let main = Regex::new(r"(?i)Livre des menus\s*(?:n[°º]\s*)?(\d+)").expect("main heading regex");
    if let Some(capture) = main.captures(&heading) {
        let number = capture[1].parse().ok()?;
        return Some((if number <= 39 { "main" } else { "bonus" }, number));
    }
    let extra = Regex::new(r"(?i)Menu\s+suppl[eé]mentaire\s*(?:n[°º]\s*)?(\d+)")
        .expect("extra heading regex");
    extra
        .captures(&heading)
        .and_then(|capture| Some(("extra", capture[1].parse().ok()?)))
}

fn scrape_menu(agent: &ureq::Agent, page_id: u32, page_url: &str, out: &Path) -> Result<Value> {
    let page = fetch_text(agent, page_url)?;
    let h1 = Regex::new(r"(?is)<h1[^>]*>(.*?)</h1>").expect("h1 regex");
    let tags = Regex::new(r"<[^>]+>").expect("tag regex");
    let spaces = Regex::new(r"\s+").expect("space regex");
    let heading = h1
        .captures(&page)
        .map(|capture| {
            spaces
                .replace_all(&decode_html(&tags.replace_all(&capture[1], " ")), " ")
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    if page_url.to_lowercase().contains("saisonnier") {
        return Ok(
            json!({"pageId": page_id, "pageUrl": page_url, "heading": heading, "error": "seasonal menu skipped"}),
        );
    }
    let Some((category, number)) = classify_heading(&heading) else {
        return Ok(
            json!({"pageId": page_id, "pageUrl": page_url, "heading": heading, "error": "unrecognized heading"}),
        );
    };
    let image = Regex::new(r#"(?i)(?:src|href)=["']([^"']*GTCafePictures/picture(\d+)\.jpg)"#)
        .expect("cover regex");
    let Some(capture) = image.captures(&page) else {
        return Ok(json!({
            "pageId": page_id,
            "pageUrl": page_url,
            "heading": heading,
            "category": category,
            "number": number,
            "recordId": format!("{category}-{number:03}"),
            "error": "no cover screenshot on guide page"
        }));
    };
    let picture_url = format!("{MENU_BASE}{}", capture[1].trim_start_matches('/'));
    let bytes = fetch_bytes(agent, &picture_url)?;
    let image = image::load_from_memory(&bytes)?.thumbnail(1280, 720);
    fs::create_dir_all(out)?;
    let record_id = format!("{category}-{number:03}");
    let output = out.join(format!("{record_id}.webp"));
    image.save_with_format(&output, image::ImageFormat::WebP)?;
    Ok(json!({
        "pageId": page_id,
        "pageUrl": page_url,
        "heading": heading,
        "category": category,
        "number": number,
        "recordId": record_id,
        "pictureNumber": capture[2].parse::<u32>().unwrap_or(0),
        "pictureUrl": picture_url,
        "cover": format!("menu_covers/{record_id}.webp"),
        "source": "skymac.org GT7 Cafe guide screenshots (research use)",
    }))
}

pub fn menu_covers(out: &Path, manifest: &Path, proxy: Option<&str>) -> Result<()> {
    let agent = agent(proxy)?;
    let index = fetch_text(&agent, MENU_INDEX)?;
    let links_re = Regex::new(r#"(?i)href=["'](GT7-Cafe/Menu-(\d+)-[^"']+\.htm)["']"#)
        .expect("menu links regex");
    let mut links: BTreeMap<u32, String> = BTreeMap::new();
    for capture in links_re.captures_iter(&index) {
        links.insert(
            capture[2].parse().unwrap_or(0),
            format!("{MENU_BASE}{}", &capture[1]),
        );
    }
    let results = links
        .into_par_iter()
        .map(|(page_id, url)| scrape_menu(&agent, page_id, &url, out))
        .collect::<Vec<_>>();
    let mut entries = BTreeMap::new();
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(value) if value.get("error").is_none() => {
                entries.insert(value["recordId"].as_str().unwrap_or("").to_string(), value);
            }
            Ok(value) => errors.push(value),
            Err(error) => errors.push(json!({"error": error.to_string()})),
        }
    }
    let entries = entries.into_values().collect::<Vec<_>>();
    let payload = json!({
        "generatedFrom": "skymac.org GT7 Cafe guide menu screenshots",
        "indexUrl": MENU_INDEX,
        "count": entries.len(),
        "errors": errors,
        "entries": entries,
    });
    if let Some(parent) = manifest.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(manifest, serde_json::to_vec_pretty(&payload)?)?;
    println!(
        "menu covers: {} -> {}",
        payload["count"],
        manifest.display()
    );
    Ok(())
}
