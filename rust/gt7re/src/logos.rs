//! Download, transcode and rewrite the GT7 brand/tuner logo set.
//!
//! The command is intentionally idempotent: it discovers current JSON
//! references, fills missing PNG sources from the local mirror, encodes
//! lossless RGBA JPEG XL files, validates every output, rewrites JSON text
//! without reordering it, and only then removes successful PNG sources.

use anyhow::{anyhow, bail, Context, Result};
use image::GenericImageView;
use jxl_encoder::{LosslessConfig, PixelLayout};
use rayon::prelude::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use walkdir::WalkDir;

const LOCAL_LOGO_DIR: &str = "logos";
const TUNER_LOGO_DIR: &str = "common/dist/gt7/carlist/tuner_logos/light";

fn agent(proxy: Option<&str>) -> Result<ureq::Agent> {
    let mut config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(90)))
        .user_agent("gt7re-logo-jxl/1.0");
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

fn site_path(site: &Path, relative: &str) -> PathBuf {
    site.join(relative.trim_start_matches('/'))
}

fn normalize_reference(key: &str, value: &str) -> Option<String> {
    let clean = value
        .split(['?', '#'])
        .next()
        .unwrap_or(value)
        .trim_start_matches('/');
    if key == "brandLogo" {
        if clean.is_empty() {
            return None;
        }
        return Some(if clean.starts_with("logos/") {
            clean.to_string()
        } else {
            format!("{LOCAL_LOGO_DIR}/{clean}")
        });
    }
    if (clean.starts_with("logos/") || clean.contains("/tuner_logos/"))
        && matches!(
            Path::new(clean)
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.to_ascii_lowercase())
                .as_deref(),
            Some("png" | "jxl")
        )
    {
        Some(clean.to_string())
    } else {
        None
    }
}

fn collect_logo_references(value: &Value, key: &str, references: &mut BTreeMap<String, String>) {
    match value {
        Value::Object(map) => {
            for (child_key, child) in map {
                collect_logo_references(child, child_key, references);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_logo_references(item, key, references);
            }
        }
        Value::String(text) => {
            if let Some(path) = normalize_reference(key, text) {
                references.insert(path, key.to_string());
            }
        }
        _ => {}
    }
}

fn data_json_files(site: &Path) -> Result<Vec<PathBuf>> {
    let data = site.join("data");
    let mut files = WalkDir::new(&data)
        .max_depth(3)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn png_reference(path: &str) -> String {
    Path::new(path)
        .with_extension("png")
        .to_string_lossy()
        .replace('\\', "/")
}

fn jxl_reference(path: &str) -> String {
    Path::new(path)
        .with_extension("jxl")
        .to_string_lossy()
        .replace('\\', "/")
}

fn add_existing_logo_files(site: &Path, references: &mut BTreeMap<String, String>) {
    for directory in [LOCAL_LOGO_DIR, TUNER_LOGO_DIR] {
        let root = site_path(site, directory);
        let Ok(entries) = fs::read_dir(&root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let extension = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            if !matches!(extension.as_str(), "png" | "jxl") || !path.is_file() {
                continue;
            }
            let Ok(relative) = path.strip_prefix(site) else {
                continue;
            };
            references.insert(
                relative.to_string_lossy().replace('\\', "/"),
                "filesystem".to_string(),
            );
        }
    }
}

fn convert_one(site: &Path, relative: &str, refresh: bool) -> Result<Option<(u64, u64)>> {
    let source = site_path(site, &png_reference(relative));
    let output = site_path(site, &jxl_reference(relative));
    if !source.is_file() {
        if output.is_file() {
            return Ok(None);
        }
        bail!("missing PNG source and JXL output: {}", source.display());
    }
    if output.is_file() && !refresh {
        return Ok(None);
    }
    let before = fs::metadata(&source)?.len();
    let bytes = fs::read(&source)?;
    let image =
        image::load_from_memory(&bytes).with_context(|| format!("decode {}", source.display()))?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        bail!("empty logo image: {}", source.display());
    }
    let rgba = image.to_rgba8();
    let encoded = LosslessConfig::new()
        .encode(rgba.as_raw(), width, height, PixelLayout::Rgba8)
        .map_err(|error| anyhow!("JPEG XL encode failed for {}: {error}", source.display()))?;
    if encoded.is_empty() {
        bail!("empty JPEG XL output: {}", output.display());
    }
    atomic_write(&output, &encoded)?;
    let after = fs::metadata(&output)?.len();
    Ok(Some((before, after)))
}

fn rewrite_json_files(files: &[PathBuf], replacements: &BTreeMap<String, String>) -> Result<usize> {
    let mut changed = 0usize;
    for path in files {
        let source =
            fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
        let mut rewritten = source.clone();
        for (before, after) in replacements {
            rewritten = rewritten.replace(&format!("\"{before}\""), &format!("\"{after}\""));
            if before.contains('/') {
                rewritten = rewritten.replace(&format!("\"/{before}\""), &format!("\"/{after}\""));
            }
            if before.starts_with("logos/") {
                let filename = before.trim_start_matches("logos/");
                let next_name = after.trim_start_matches("logos/");
                rewritten =
                    rewritten.replace(&format!("\"{filename}\""), &format!("\"{next_name}\""));
            }
        }
        if rewritten != source {
            let _: Value = serde_json::from_str(&rewritten)
                .with_context(|| format!("invalid JSON after rewrite: {}", path.display()))?;
            atomic_write(path, rewritten.as_bytes())?;
            changed += 1;
        }
    }
    Ok(changed)
}

pub fn convert(
    site: &Path,
    base: &str,
    proxy: Option<&str>,
    threads: usize,
    refresh: bool,
    keep_png: bool,
) -> Result<Value> {
    let json_files = data_json_files(site)?;
    let mut references = BTreeMap::<String, String>::new();
    for path in &json_files {
        let value: Value = serde_json::from_slice(&fs::read(path)?)
            .with_context(|| format!("parse {}", path.display()))?;
        collect_logo_references(&value, "", &mut references);
    }
    add_existing_logo_files(site, &mut references);
    if references.is_empty() {
        bail!(
            "no logo references or logo files found under {}",
            site.display()
        );
    }

    let agent = Arc::new(agent(proxy)?);
    let downloaded = AtomicUsize::new(0);
    let download_errors = references
        .keys()
        .collect::<Vec<_>>()
        .par_iter()
        .filter_map(|relative| {
            let png_relative = png_reference(relative);
            let source = site_path(site, &png_relative);
            let output = site_path(site, &jxl_reference(relative));
            if source.is_file() || (output.is_file() && !refresh) {
                return None;
            }
            let url = format!(
                "{}/{}",
                base.trim_end_matches('/'),
                png_relative.trim_start_matches('/')
            );
            match fetch(&agent, &url).and_then(|bytes| atomic_write(&source, &bytes)) {
                Ok(()) => {
                    downloaded.fetch_add(1, Ordering::Relaxed);
                    None
                }
                Err(error) => Some(json!({
                    "stage": "download",
                    "path": relative,
                    "url": url,
                    "error": error.to_string(),
                })),
            }
        })
        .collect::<Vec<_>>();

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .build()?;
    let conversions = pool.install(|| {
        references
            .keys()
            .collect::<Vec<_>>()
            .par_iter()
            .map(|relative| match convert_one(site, relative, refresh) {
                Ok(Some((before, after))) => Ok(Some((before, after))),
                Ok(None) => Ok(None),
                Err(error) => Err(json!({
                    "stage": "convert",
                    "path": relative,
                    "error": error.to_string(),
                })),
            })
            .collect::<Vec<_>>()
    });
    let converted = conversions
        .iter()
        .filter(|result| matches!(result, Ok(Some(_))))
        .count();
    let converted_bytes: (u64, u64) = conversions
        .iter()
        .filter_map(|result| result.as_ref().ok().and_then(|value| *value))
        .fold((0, 0), |(before, after), (next_before, next_after)| {
            (before + next_before, after + next_after)
        });
    let conversion_errors = conversions
        .into_iter()
        .filter_map(Result::err)
        .collect::<Vec<_>>();
    let errors = download_errors
        .into_iter()
        .chain(conversion_errors)
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        bail!(
            "logo conversion stopped before link replacement because {} assets failed: {}",
            errors.len(),
            serde_json::to_string(&errors)?
        );
    }

    let mut replacements = BTreeMap::new();
    let mut missing_outputs = Vec::new();
    for relative in references.keys() {
        let before = png_reference(relative);
        let after = jxl_reference(relative);
        if site_path(site, &after).is_file() {
            if before != after {
                replacements.insert(before, after);
            }
        } else {
            missing_outputs.push(relative.clone());
        }
    }
    if !missing_outputs.is_empty() {
        bail!(
            "missing JPEG XL logo outputs: {}",
            missing_outputs.join(", ")
        );
    }
    let changed_json_files = rewrite_json_files(&json_files, &replacements)?;

    let mut removed_png = 0usize;
    if !keep_png {
        for relative in references.keys() {
            let source = site_path(site, &png_reference(relative));
            if source.is_file() {
                fs::remove_file(&source).with_context(|| format!("remove {}", source.display()))?;
                removed_png += 1;
            }
        }
    }

    Ok(json!({
        "references": references.len(),
        "downloaded": downloaded.load(Ordering::Relaxed),
        "converted": converted,
        "skipped": references.len().saturating_sub(converted),
        "changedJsonFiles": changed_json_files,
        "removedPng": removed_png,
        "pngBytes": converted_bytes.0,
        "jxlBytes": converted_bytes.1,
        "threads": threads.max(1),
        "keepPng": keep_png,
    }))
}
