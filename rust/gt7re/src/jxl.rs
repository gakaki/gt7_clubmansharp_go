//! JPEG XL container indexer for GT7 preview assets.
//!
//! The retail files are encrypted/custom decoded by the game, but their ISO
//! BMFF box layout is intact. This command records the exact `jxlc` extent so
//! a future codec implementation can work from a reproducible inventory.

use anyhow::{bail, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct JxlBox {
    pub kind: String,
    pub offset: u64,
    pub size: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct JxlInfo {
    pub rel: String,
    pub size: u64,
    pub boxes: Vec<JxlBox>,
    pub codestream_bytes: u64,
    pub has_exif: bool,
}

pub fn index_one(rel: &str, path: &Path) -> Result<JxlInfo> {
    let data = fs::read(path)?;
    if data.len() < 12 || data.get(4..8) != Some(b"JXL ") {
        bail!("{}: not a JXL box container", path.display());
    }
    let mut boxes = Vec::new();
    let mut offset = 0usize;
    while offset + 8 <= data.len() {
        let size32 = u32::from_be_bytes(data[offset..offset + 4].try_into()?);
        let kind = String::from_utf8_lossy(&data[offset + 4..offset + 8]).to_string();
        let mut header = 8u64;
        let box_size = match size32 {
            0 => (data.len() - offset) as u64,
            1 => {
                if offset + 16 > data.len() {
                    bail!("{}: truncated extended box", path.display());
                }
                header = 16;
                u64::from_be_bytes(data[offset + 8..offset + 16].try_into()?)
            }
            value => u64::from(value),
        };
        if box_size < header || box_size > (data.len() - offset) as u64 {
            bail!("{}: invalid box at 0x{offset:X}", path.display());
        }
        boxes.push(JxlBox {
            kind,
            offset: (offset + header as usize) as u64,
            size: box_size - header,
        });
        offset += box_size as usize;
    }
    let codestream_bytes = boxes
        .iter()
        .filter(|item| item.kind == "jxlc" || item.kind == "jxlp")
        .map(|item| item.size)
        .sum();
    Ok(JxlInfo {
        rel: rel.into(),
        size: data.len() as u64,
        has_exif: boxes.iter().any(|item| item.kind == "exif"),
        codestream_bytes,
        boxes,
    })
}

pub fn index_many(entries: &[String], root: &Path) -> Vec<JxlInfo> {
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
