//! GPB (UI project) embedded IDP0 extraction.
//!
//! `.gpb` files start with "4bpg" and embed one or more IDP0 containers
//! (0x70-byte header + TXS5 payload) as UI image assets.

use anyhow::Result;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Embedded {
    pub offset: usize,
    pub payload_offset: u64,
    pub payload_size: u64,
}

fn u64le(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes([
        b[o], b[o + 1], b[o + 2], b[o + 3], b[o + 4], b[o + 5], b[o + 6], b[o + 7],
    ])
}

pub fn find_embedded(data: &[u8]) -> Vec<Embedded> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 0x48 <= data.len() {
        if &data[i..i + 4] == b"IDP0" {
            let ver = u32::from_le_bytes([data[i + 4], data[i + 5], data[i + 6], data[i + 7]]);
            let off = u64le(data, i + 0x30);
            let size = u64le(data, i + 0x38);
            if ver == 2
                && off >= 0x70
                && (i as u64 + off) < data.len() as u64
                && size > 0
                && (i as u64 + off + size) <= data.len() as u64
            {
                out.push(Embedded { offset: i, payload_offset: off, payload_size: size });
            }
            i += 0x40;
        } else {
            i += 4;
        }
    }
    out
}

pub fn extract_file(path: &Path, stem: &str, outdir: &Path) -> Result<usize> {
    let data = fs::read(path)?;
    let found = find_embedded(&data);
    fs::create_dir_all(outdir)?;
    for (i, e) in found.iter().enumerate() {
        let start = e.offset + e.payload_offset as usize;
        let end = start + e.payload_size as usize;
        fs::write(outdir.join(format!("{stem}_{i:02}.txs")), &data[start..end])?;
    }
    Ok(found.len())
}
