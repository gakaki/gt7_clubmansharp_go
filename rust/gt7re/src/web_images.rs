//! Convert browser-owned raster images to JPEG XL and rewrite local references.

use anyhow::{anyhow, bail, Context, Result};
use image::GenericImageView;
use jxl_encoder::{LosslessConfig, LossyConfig, PixelLayout};
use rayon::prelude::*;
use regex::{Captures, Regex};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use walkdir::WalkDir;

#[derive(Debug)]
struct Conversion {
    source: PathBuf,
    output: PathBuf,
    before: u64,
    after: u64,
    reused: bool,
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

fn encode_jxl(path: &Path) -> Result<Vec<u8>> {
    let bytes = fs::read(path).with_context(|| format!("read {}", path.display()))?;
    let image =
        image::load_from_memory(&bytes).with_context(|| format!("decode {}", path.display()))?;
    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        bail!("empty image: {}", path.display());
    }
    let encoded = if image.color().has_alpha() {
        let rgba = image.to_rgba8();
        LosslessConfig::new().encode(rgba.as_raw(), width, height, PixelLayout::Rgba8)
    } else {
        let rgb = image.to_rgb8();
        LossyConfig::new(1.0).encode(rgb.as_raw(), width, height, PixelLayout::Rgb8)
    }
    .map_err(|error| anyhow!("JPEG XL encode failed for {}: {error}", path.display()))?;
    if encoded.len() <= 16 {
        bail!("empty JPEG XL output for {}", path.display());
    }
    Ok(encoded)
}

fn convert_one(source: &Path, refresh: bool) -> Result<Conversion> {
    let output = source.with_extension("jxl");
    let before = fs::metadata(source)?.len();
    if output.is_file() && !refresh {
        let after = fs::metadata(&output)?.len();
        if after > 16 {
            return Ok(Conversion {
                source: source.to_path_buf(),
                output,
                before,
                after,
                reused: true,
            });
        }
    }
    let encoded = encode_jxl(source)?;
    atomic_write(&output, &encoded)?;
    Ok(Conversion {
        source: source.to_path_buf(),
        output,
        before,
        after: encoded.len() as u64,
        reused: false,
    })
}

fn raster_sources(site: &Path) -> Vec<PathBuf> {
    WalkDir::new(site)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| {
            matches!(
                path.extension()
                    .and_then(|value| value.to_str())
                    .map(|value| value.to_ascii_lowercase())
                    .as_deref(),
                Some("png" | "webp")
            )
        })
        .collect()
}

fn slash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn rewrite_local_references(site: &Path, conversions: &[Conversion]) -> Result<usize> {
    let data = site.join("data");
    let replacements = conversions
        .iter()
        .map(|conversion| {
            let before = slash_path(
                conversion
                    .source
                    .strip_prefix(site)
                    .with_context(|| conversion.source.display().to_string())?,
            );
            let after = slash_path(
                conversion
                    .output
                    .strip_prefix(site)
                    .with_context(|| conversion.output.display().to_string())?,
            );
            Ok((before, after))
        })
        .collect::<Result<HashMap<_, _>>>()?;
    let reference = Regex::new(r#""(/?[^"\r\n]+\.(?:png|webp))""#)?;
    let mut changed_files = 0usize;
    for entry in WalkDir::new(&data)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
        })
    {
        let path = entry.path();
        let source = fs::read_to_string(path)?;
        let rewritten = reference
            .replace_all(&source, |captures: &Captures| {
                let value = &captures[1];
                let relative = value.trim_start_matches('/');
                let Some(replacement) = replacements.get(relative) else {
                    return captures[0].to_string();
                };
                if value.starts_with('/') {
                    format!("\"/{replacement}\"")
                } else {
                    format!("\"{replacement}\"")
                }
            })
            .into_owned();
        if rewritten != source {
            atomic_write(path, rewritten.as_bytes())?;
            changed_files += 1;
        }
    }
    Ok(changed_files)
}

fn official_image_index(site: &Path) -> Result<HashMap<String, Vec<String>>> {
    let mut index = HashMap::new();
    for entry in fs::read_dir(site.join("official"))? {
        let directory = entry?.path();
        if !directory.is_dir() {
            continue;
        }
        let Some(car_id) = directory
            .file_name()
            .and_then(|value| value.to_str())
            .map(str::to_string)
        else {
            continue;
        };
        let mut names = fs::read_dir(&directory)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("jxl")
            })
            .filter_map(|path| {
                path.file_name()
                    .map(|value| value.to_string_lossy().into_owned())
            })
            .collect::<Vec<_>>();
        names.sort();
        if !names.is_empty() {
            index.insert(
                car_id.clone(),
                names
                    .into_iter()
                    .map(|name| format!("/official/{car_id}/{name}"))
                    .collect(),
            );
        }
    }
    Ok(index)
}

fn normalize_mirror_catalog_with_index(
    index: &HashMap<String, Vec<String>>,
    catalog: &mut Value,
) -> usize {
    let mut image_count = 0usize;
    let mut all_jxl = true;
    if let Some(cars) = catalog.get_mut("cars").and_then(Value::as_array_mut) {
        for car in cars {
            let car_id = car.get("id").and_then(Value::as_str).unwrap_or("");
            if let Some(images) = index.get(car_id) {
                image_count += images.len();
                if let Some(object) = car.as_object_mut() {
                    object.insert("images".into(), json!(images));
                }
            } else if let Some(images) = car.get("images").and_then(Value::as_array) {
                image_count += images.len();
                all_jxl &= images.iter().all(|image| {
                    image
                        .as_str()
                        .map(|value| value.ends_with(".jxl"))
                        .unwrap_or(false)
                });
            }
        }
    }
    if let Some(object) = catalog.as_object_mut() {
        object.insert(
            "image_format".into(),
            json!(if all_jxl { "jxl" } else { "mixed" }),
        );
        object.insert("image_count".into(), json!(image_count));
    }
    image_count
}

pub fn normalize_mirror_catalog(site: &Path, catalog: &mut Value) -> Result<usize> {
    let index = official_image_index(site)?;
    Ok(normalize_mirror_catalog_with_index(&index, catalog))
}

fn rewrite_mirror_catalogs(site: &Path) -> Result<usize> {
    let directory = site.join("data/mirror");
    let index = official_image_index(site)?;
    let mut changed = 0usize;
    for entry in fs::read_dir(&directory)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        if !name.starts_with("cars.") || !name.ends_with(".json") {
            continue;
        }
        let mut catalog: Value = serde_json::from_slice(&fs::read(&path)?)
            .with_context(|| path.display().to_string())?;
        normalize_mirror_catalog_with_index(&index, &mut catalog);
        atomic_write(&path, &serde_json::to_vec_pretty(&catalog)?)?;
        changed += 1;
    }
    Ok(changed)
}

fn references_avif(site: &Path) -> Result<bool> {
    for entry in WalkDir::new(site.join("data"))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry.path().extension().and_then(|value| value.to_str()) == Some("json")
        })
    {
        let source = fs::read_to_string(entry.path())?;
        if source.contains(".avif\"") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn prune_avifs(site: &Path) -> Result<usize> {
    if references_avif(site)? {
        return Ok(0);
    }
    let mut removed = 0usize;
    for entry in WalkDir::new(site)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .map(|value| value.eq_ignore_ascii_case("avif"))
                    .unwrap_or(false)
        })
    {
        fs::remove_file(entry.path())?;
        removed += 1;
    }
    Ok(removed)
}

pub fn convert(site: &Path, threads: usize, keep_sources: bool, refresh: bool) -> Result<Value> {
    let site = site
        .canonicalize()
        .with_context(|| format!("site directory: {}", site.display()))?;
    let sources = raster_sources(&site);
    let processed = AtomicUsize::new(0);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .build()?;
    let results = pool.install(|| {
        sources
            .par_iter()
            .map(|source| {
                let result = convert_one(source, refresh);
                processed.fetch_add(1, Ordering::Relaxed);
                result
            })
            .collect::<Vec<_>>()
    });
    let mut conversions = Vec::with_capacity(results.len());
    let mut errors = Vec::new();
    for result in results {
        match result {
            Ok(conversion) => conversions.push(conversion),
            Err(error) => errors.push(error.to_string()),
        }
    }
    if !errors.is_empty() {
        bail!(
            "{} image conversions failed: {}",
            errors.len(),
            errors.join(" | ")
        );
    }

    let rewritten_files = rewrite_local_references(&site, &conversions)?;
    let rewritten_catalogs = rewrite_mirror_catalogs(&site)?;
    let mut removed_sources = 0usize;
    if !keep_sources {
        for conversion in &conversions {
            if conversion.output.is_file() && fs::metadata(&conversion.output)?.len() > 16 {
                fs::remove_file(&conversion.source)?;
                removed_sources += 1;
            }
        }
    }
    let removed_avifs = prune_avifs(&site)?;
    let before = conversions.iter().map(|item| item.before).sum::<u64>();
    let after = conversions.iter().map(|item| item.after).sum::<u64>();
    Ok(json!({
        "site": site,
        "sources": conversions.len(),
        "converted": conversions.iter().filter(|item| !item.reused).count(),
        "reused": conversions.iter().filter(|item| item.reused).count(),
        "processed": processed.load(Ordering::Relaxed),
        "sourceBytes": before,
        "jxlBytes": after,
        "removedSources": removed_sources,
        "removedAvifs": removed_avifs,
        "rewrittenJsonFiles": rewritten_files,
        "rewrittenMirrorCatalogs": rewritten_catalogs,
    }))
}
