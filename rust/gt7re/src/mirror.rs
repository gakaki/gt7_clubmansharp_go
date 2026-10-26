//! Parallel mirror of the public GT7 car-list catalogue and its local assets.

use anyhow::{Context, Result};
use rayon::prelude::*;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

pub const LOCALES: &[&str] = &[
    "us", "gb", "de", "fr", "it", "nl", "es", "ms", "pt", "bp", "pl", "ru", "cz", "tr", "el", "ar",
    "cn", "tw", "jp", "kr", "th",
];

fn agent(proxy: Option<&str>) -> Result<ureq::Agent> {
    let mut config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(90)))
        .user_agent("gt7-car-list-mirror-rust/1.0");
    if let Some(proxy) = proxy.filter(|value| !value.is_empty()) {
        config = config.proxy(Some(ureq::Proxy::new(proxy)?));
    }
    Ok(config.build().into())
}

fn fetch(agent: &ureq::Agent, url: &str) -> Result<Vec<u8>> {
    let mut response = agent.get(url).call().with_context(|| url.to_string())?;
    response
        .body_mut()
        .read_to_vec()
        .with_context(|| url.to_string())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    fs::write(&temporary, bytes)?;
    fs::rename(&temporary, path).or_else(|_| {
        let _ = fs::remove_file(path);
        fs::rename(&temporary, path)
    })?;
    Ok(())
}

fn local_path(root: &Path, remote: &str) -> PathBuf {
    let clean = remote
        .split(['?', '#'])
        .next()
        .unwrap_or(remote)
        .trim_start_matches('/');
    root.join(clean)
}

fn add_referenced_assets(catalog: &Value, assets: &mut BTreeSet<String>) {
    if let Some(object) = catalog.as_object() {
        for key in ["logo", "thumbnail", "country_flag"] {
            if let Some(path) = object.get(key).and_then(Value::as_str) {
                assets.insert(path.to_string());
            }
        }
        if let Some(images) = object.get("images").and_then(Value::as_array) {
            for image in images.iter().filter_map(Value::as_str) {
                assets.insert(image.to_string());
            }
        }
    }
    if let Some(cars) = catalog.get("cars").and_then(Value::as_array) {
        for car in cars {
            add_referenced_assets(car, assets);
        }
    }
    if let Some(flags) = catalog.get("country_flags").and_then(Value::as_object) {
        for flag in flags.values().filter_map(Value::as_str) {
            assets.insert(flag.to_string());
        }
    }
}

pub fn sync(
    base: &str,
    site: &Path,
    proxy: Option<&str>,
    refresh: bool,
    threads: usize,
) -> Result<Value> {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .build()?;
    let agent = agent(proxy)?;
    let base = base.trim_end_matches('/');
    let data_dir = site.join("data/mirror");
    fs::create_dir_all(&data_dir)?;
    let mut assets = BTreeSet::new();
    let mut catalog_errors = Vec::new();

    for locale in LOCALES {
        let url = format!("{base}/common/dist/gt7/carlist/data/cars.{locale}.json");
        match fetch(&agent, &url).and_then(|bytes| {
            let mut value: Value = serde_json::from_slice(&bytes)?;
            crate::web_images::normalize_mirror_catalog(site, &mut value)?;
            add_referenced_assets(&value, &mut assets);
            atomic_write(
                &data_dir.join(format!("cars.{locale}.json")),
                &serde_json::to_vec_pretty(&value)?,
            )?;
            Ok(value)
        }) {
            Ok(_) => {}
            Err(error) => {
                catalog_errors.push(json!({"locale": locale, "error": error.to_string()}))
            }
        }
    }

    for fixed in [
        "/common/dist/gt7/carlist/assets/logo_gt7-BQSHSLc9.png",
        "/common/dist/gt7/carlist/assets/logo_gtcom-kY3Al7KV.svg",
    ] {
        assets.insert(fixed.to_string());
    }
    for css_path in ["/_offline/google-fonts.css", "/_offline/typekit.css"] {
        let css = fetch(&agent, &format!("{base}{css_path}"))?;
        let css_text = String::from_utf8_lossy(&css);
        let url_pattern = Regex::new(r#"url\(([^)]+)\)"#)?;
        for capture in url_pattern.captures_iter(&css_text) {
            let value = capture[1].trim().trim_matches(['\'', '"']);
            if value.starts_with('/') {
                assets.insert(value.to_string());
            }
        }
        atomic_write(&local_path(site, css_path), &css)?;
    }

    let processed = AtomicUsize::new(0);
    let errors = pool.install(|| {
        assets
            .par_iter()
            .filter_map(|remote| {
                let path = local_path(site, remote);
                if path.is_file() && !refresh {
                    processed.fetch_add(1, Ordering::Relaxed);
                    return None;
                }
                let url = format!("{base}{remote}");
                let result = fetch(&agent, &url).and_then(|bytes| atomic_write(&path, &bytes));
                processed.fetch_add(1, Ordering::Relaxed);
                result
                    .err()
                    .map(|error| json!({"asset": remote, "error": error.to_string()}))
            })
            .collect::<Vec<_>>()
    });

    let manifest = json!({
        "generatedFrom": base,
        "locales": LOCALES,
        "catalogCount": LOCALES.len() - catalog_errors.len(),
        "assetCount": assets.len(),
        "processedAssets": processed.load(Ordering::Relaxed),
        "catalogErrors": catalog_errors,
        "assetErrors": errors,
    });
    atomic_write(
        &site.join("data/mirror/manifest.json"),
        serde_json::to_vec_pretty(&manifest)?.as_slice(),
    )?;
    Ok(manifest)
}
