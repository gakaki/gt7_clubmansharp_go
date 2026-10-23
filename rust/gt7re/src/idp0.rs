//! IDP0 container: 0x70-byte header followed by an embedded payload.
//!
//! Header:
//!   0x00 "IDP0", 0x04 u32 version (2),
//!   0x08..0x30 u64 offset table,
//!   0x30 u64 payload offset (0x70),
//!   0x38 u64 payload size,
//!   0x40 4-byte tag ("ecp"), 0x44 4-byte tag ("txs").
//! In every sampled file the payload is a TXS5 texture-set.

use anyhow::{bail, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Clone, Debug)]
pub struct Idp0Header {
    pub payload_offset: u64,
    pub payload_size: u64,
    pub tag1: String,
    pub tag2: String,
}

fn u64le(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes([
        b[o], b[o + 1], b[o + 2], b[o + 3], b[o + 4], b[o + 5], b[o + 6], b[o + 7],
    ])
}

fn tag(b: &[u8], o: usize) -> String {
    b[o..o + 4]
        .iter()
        .map(|c| if (0x20..0x7F).contains(c) { *c as char } else { '.' })
        .collect()
}

pub fn parse_header(data: &[u8]) -> Result<Idp0Header> {
    if data.len() < 0x48 || &data[0..4] != b"IDP0" {
        bail!("not an IDP0 container");
    }
    let payload_offset = u64le(data, 0x30);
    let payload_size = u64le(data, 0x38);
    if payload_offset + payload_size > data.len() as u64 {
        bail!("IDP0 payload out of range");
    }
    Ok(Idp0Header {
        payload_offset,
        payload_size,
        tag1: tag(data, 0x40),
        tag2: tag(data, 0x44),
    })
}

pub fn extract(path: &Path, dest: &Path) -> Result<Idp0Header> {
    let data = fs::read(path)?;
    let h = parse_header(&data)?;
    let start = h.payload_offset as usize;
    let end = start + h.payload_size as usize;
    if let Some(p) = dest.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(dest, &data[start..end])?;
    Ok(h)
}
