//! MDL5 (model) header + identifier metadata extraction.
//!
//! Full geometry decoding is a separate undertaking; this module extracts the
//! parts that are directly verifiable: header fields and the identifier strings
//! (bones, animation channels, material/node names) that live near the start of
//! every model file.

use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize, Clone, Debug)]
pub struct ModelInfo {
    pub rel: String,
    pub size: u64,
    pub version: u32,
    pub field_0x08: u32,
    pub field_0x0c: u32,
    pub model_count: u32,
    pub models_offset: u64,
    pub models_end: u64,
    pub texture_set_offset: u64,
    pub shaders_offset: u64,
    pub strings: Vec<String>,
}

fn is_ident(s: &str) -> bool {
    s.len() >= 4
        && s.len() <= 96
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-' | '/' | ' '))
        && s.chars().any(|c| c.is_ascii_alphabetic())
}

fn extract_strings(data: &[u8], limit: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut run = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for &b in data.iter().chain(std::iter::once(&0u8)) {
        if (0x20..0x7F).contains(&b) {
            run.push(b);
        } else {
            if run.len() >= 4 {
                let s = String::from_utf8_lossy(&run).trim().to_string();
                if is_ident(&s) && seen.insert(s.clone()) {
                    out.push(s);
                    if out.len() >= limit {
                        break;
                    }
                }
            }
            run.clear();
        }
    }
    out
}

pub fn index_one(
    rel: &str,
    path: &Path,
    head_bytes: u64,
    string_limit: usize,
) -> Result<ModelInfo> {
    let meta = fs::metadata(path)?;
    let take = meta.len().min(head_bytes) as usize;
    let data = {
        use std::io::Read;
        let mut f = fs::File::open(path)?;
        let mut buf = vec![0u8; take];
        let n = f.read(&mut buf)?;
        buf.truncate(n);
        buf
    };
    let u32_at = |o: usize| -> u32 {
        if data.len() >= o + 4 {
            u32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]])
        } else {
            0
        }
    };
    let u64_at = |o: usize| -> u64 {
        if data.len() >= o + 8 {
            u64::from_le_bytes([
                data[o],
                data[o + 1],
                data[o + 2],
                data[o + 3],
                data[o + 4],
                data[o + 5],
                data[o + 6],
                data[o + 7],
            ])
        } else {
            0
        }
    };
    let model_count = u32_at(0x2C);
    let models_offset = u64_at(0xD8);
    Ok(ModelInfo {
        rel: rel.to_string(),
        size: meta.len(),
        version: u32_at(4),
        field_0x08: u32_at(8),
        field_0x0c: u32_at(0x0C),
        model_count,
        models_offset,
        models_end: models_offset.saturating_add(u64::from(model_count).saturating_mul(0x58)),
        texture_set_offset: u64_at(0x108),
        shaders_offset: u64_at(0x110),
        strings: extract_strings(&data[8..], string_limit),
    })
}

pub fn index_many(
    entries: &[(String, u64)],
    store_root: &Path,
    head_bytes: u64,
    string_limit: usize,
) -> Vec<ModelInfo> {
    entries
        .par_iter()
        .filter_map(|(rel, _size)| {
            let mut p: PathBuf = store_root.to_path_buf();
            for part in rel.split('/') {
                p.push(part);
            }
            index_one(rel, &p, head_bytes, string_limit).ok()
        })
        .collect()
}
