//! SNDZ container inventory.
//!
//! GT7's SNDZ files are synthesis/program containers rather than ordinary PCM.
//! This index records their header and embedded section tags so the sound
//! database is searchable even when a native waveform renderer is unavailable.

use anyhow::{bail, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const TAGS: [&[u8; 4]; 10] = [
    b"HTBS", b"WAVS", b"RNDS", b"BRNS", b"BLNS", b"CHNS", b"TSKS", b"SMXS", b"REQS", b"WAVD",
];

#[derive(Clone, Debug, Serialize)]
pub struct SndzInfo {
    pub rel: String,
    pub size: u64,
    pub field_0x04: u32,
    pub field_0x08: u32,
    pub section_count: u32,
    pub tags: BTreeMap<String, usize>,
}

fn u32le(data: &[u8], offset: usize) -> u32 {
    data.get(offset..offset + 4)
        .and_then(|value| value.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0)
}

pub fn index_one(rel: &str, path: &Path) -> Result<SndzInfo> {
    let data = fs::read(path)?;
    if data.get(..4) != Some(b"SNDZ") {
        bail!("{}: not an SNDZ container", path.display());
    }
    let mut tags = BTreeMap::new();
    for tag in TAGS {
        let count = data.windows(4).filter(|window| *window == tag).count();
        if count > 0 {
            tags.insert(String::from_utf8_lossy(tag).to_string(), count);
        }
    }
    Ok(SndzInfo {
        rel: rel.into(),
        size: data.len() as u64,
        field_0x04: u32le(&data, 4),
        field_0x08: u32le(&data, 8),
        section_count: u32le(&data, 0x80),
        tags,
    })
}

pub fn index_many(entries: &[String], root: &Path) -> Vec<SndzInfo> {
    entries
        .par_iter()
        .filter_map(|rel| {
            let mut path = PathBuf::from(root);
            for part in rel.split('/') {
                path.push(part);
            }
            index_one(rel, &path).ok()
        })
        .collect()
}
