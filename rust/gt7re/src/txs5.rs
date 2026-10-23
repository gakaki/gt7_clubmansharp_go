//! TXS5 texture-set descriptor parsing.
//!
//! Header: "TXS5", u32@0x04 file size, u32@0x28 -> descriptor table,
//! u32@0x30 -> texture data start (0x1000).
//! Descriptor stride is 0x60; the first 0x20 bytes are used:
//!   0x00 u64 data offset
//!   0x08 u32 data size
//!   0x0C u16 format / tile mode
//!   0x0E u16 mip/flag
//!   0x10 u16 width
//!   0x12 u16 height
//!   0x14 u32 array count
//! Pixel payloads are block-swizzled; raw bytes are kept for future decoders.

use anyhow::{bail, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Clone, Debug)]
pub struct Txs5Entry {
    pub data_offset: u64,
    pub data_size: u32,
    pub format: u16,
    pub mip_flags: u16,
    pub width: u16,
    pub height: u16,
    pub count: u32,
    pub bytes_per_pixel: f32,
}

#[derive(Serialize, Clone, Debug)]
pub struct Txs5File {
    pub rel: String,
    pub file_size: u64,
    pub entries: Vec<Txs5Entry>,
}

fn u16le(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([b[o], b[o + 1]])
}
fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn u64le(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes([
        b[o],
        b[o + 1],
        b[o + 2],
        b[o + 3],
        b[o + 4],
        b[o + 5],
        b[o + 6],
        b[o + 7],
    ])
}

pub fn parse(rel: &str, path: &Path) -> Result<Txs5File> {
    let d = fs::read(path)?;
    if d.len() < 0x40 || &d[0..4] != b"TXS5" {
        bail!("not a TXS5 file");
    }
    let table = u32le(&d, 0x28) as usize;
    let mut entries = Vec::new();
    if table + 0x20 <= d.len() {
        let mut o = table;
        while o + 0x20 <= d.len() {
            let data_offset = u64le(&d, o);
            let data_size = u32le(&d, o + 8);
            let width = u16le(&d, o + 0x10);
            let height = u16le(&d, o + 0x12);
            if data_offset >= d.len() as u64
                || data_size == 0
                || data_size as u64 > d.len() as u64
                || width == 0
                || height == 0
                || width > 8192
                || height > 8192
            {
                break;
            }
            let count = u32le(&d, o + 0x14);
            let texels = (width as u64) * (height as u64) * count.max(1) as u64;
            entries.push(Txs5Entry {
                data_offset,
                data_size,
                format: u16le(&d, o + 0x0C),
                mip_flags: u16le(&d, o + 0x0E),
                width,
                height,
                count,
                bytes_per_pixel: if texels > 0 {
                    data_size as f32 / texels as f32
                } else {
                    0.0
                },
            });
            o += 0x60;
        }
    }
    Ok(Txs5File {
        rel: rel.to_string(),
        file_size: d.len() as u64,
        entries,
    })
}
