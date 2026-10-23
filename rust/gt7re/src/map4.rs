//! MAP4 (`course_map`) parser: the course centreline sampled every 2.5 m.
//!
//! Header (little endian):
//!   0x00 "MAP4", 0x04 u32 version (4), 0x0C u32 file size,
//!   0x10 u32 points-table offset (0x80), 0x14 u32 end of point table,
//!   0x20 f32 x,y,z min / 0x2C max / 0x38 cell size (2.5),
//!   0x40 u32 secondary table, 0x44 u32 edge-table offset,
//!   0x80 u32 point count, 0x84 count x 0x18-byte records
//!        {f32 x, f32 y, f32 z, u32 a, u32 b, f32 c}
//! Edge table: u32 count followed by count x (u16, u16) index pairs.

use anyhow::{bail, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Clone, Debug)]
pub struct Map4 {
    pub name: String,
    pub size: u64,
    pub point_count: u32,
    pub edge_count: u32,
    pub cell_size: f32,
    pub length_m: f32,
    pub closure_m: f32,
    pub points: Vec<[f32; 3]>,
    pub edges: Vec<[u16; 2]>,
}

fn u32le(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn f32le(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

pub fn parse(path: &Path) -> Result<Map4> {
    let data = fs::read(path)?;
    if data.len() < 0x90 || &data[0..4] != b"MAP4" {
        bail!("not a MAP4 file");
    }
    let count = u32le(&data, 0x80);
    let mut points = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let o = 0x84 + i * 0x18;
        if o + 0x18 > data.len() {
            break;
        }
        points.push([f32le(&data, o), f32le(&data, o + 4), f32le(&data, o + 8)]);
    }
    let mut length = 0.0f32;
    for w in points.windows(2) {
        length += ((w[1][0] - w[0][0]).powi(2)
            + (w[1][1] - w[0][1]).powi(2)
            + (w[1][2] - w[0][2]).powi(2))
        .sqrt();
    }
    let closure = points
        .first()
        .zip(points.last())
        .map(|(a, b)| {
            ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
        })
        .unwrap_or(0.0);

    let edge_off = u32le(&data, 0x44) as usize;
    let mut edges = Vec::new();
    let mut edge_count = 0u32;
    if edge_off + 4 <= data.len() {
        edge_count = u32le(&data, edge_off);
        for i in 0..edge_count as usize {
            let o = edge_off + 4 + i * 4;
            if o + 4 > data.len() {
                break;
            }
            edges.push([
                u16::from_le_bytes([data[o], data[o + 1]]),
                u16::from_le_bytes([data[o + 2], data[o + 3]]),
            ]);
        }
    }

    Ok(Map4 {
        name: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
        size: data.len() as u64,
        point_count: count,
        edge_count,
        cell_size: f32le(&data, 0x3C),
        length_m: length,
        closure_m: closure,
        points,
        edges,
    })
}

pub fn write_csv(m: &Map4, path: &Path) -> Result<()> {
    let mut s = String::with_capacity(m.points.len() * 40);
    s.push_str("index,x,y,z\n");
    for (i, p) in m.points.iter().enumerate() {
        s.push_str(&format!("{i},{:.3},{:.3},{:.3}\n", p[0], p[1], p[2]));
    }
    fs::write(path, s)?;
    Ok(())
}
