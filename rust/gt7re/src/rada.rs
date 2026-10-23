//! RADA (`course_line`) container parsing.
//!
//! Confirmed layout of every section header (0x30 bytes):
//!   0x00 u32 count
//!   0x04 u16 flags
//!   0x06 u16 0x2A06 (constant)
//!   0x08 f32 step / scale (0.1 in observed files)
//!   0x0C f32 x,y,z  (section vector A - start tangent/normal)
//!   0x18 f32 x,y,z  (section vector B - start position)
//!   0x24 u32 0
//!   0x28 u64 data offset
//!   0x30 u64 distance-array offset (array starts at +4)
//! The first data region holds `count` records of 3 x i16 (per-point vector).
//! The second holds `count` f32 cumulative arc lengths (observed lap: 5.8 km).
//! Semantics of the per-point vectors (normal vs delta) are not fully resolved;
//! they are exported verbatim so downstream work can test hypotheses.

use anyhow::{bail, Result};
use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Serialize, Clone, Debug)]
pub struct RadaSection {
    pub header_offset: u64,
    pub count: u32,
    pub flags: u16,
    pub step: f32,
    pub vector_a: [f32; 3],
    pub vector_b: [f32; 3],
    pub data_offset: u64,
    pub data_end: u64,
    pub points: Vec<[i16; 3]>,
    pub distances: Vec<f32>,
}

#[derive(Serialize, Clone, Debug)]
pub struct RadaFile {
    pub name: String,
    pub size: u64,
    pub sections: Vec<RadaSection>,
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
fn f32le(b: &[u8], o: usize) -> f32 {
    f32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}

pub fn parse(path: &Path) -> Result<RadaFile> {
    let data = fs::read(path)?;
    if data.len() < 0x40 || &data[0..4] != b"RADA" {
        bail!("not a RADA file");
    }
    let mut sections = Vec::new();
    let mut off = 0x10usize;
    while off + 0x38 <= data.len() {
        // section signature: constant 0x2A06 and plausible step
        if u16le(&data, off + 6) == 0x2A06 {
            let count = u32le(&data, off);
            let step = f32le(&data, off + 8);
            let data_off = u64le(&data, off + 0x28);
            let data_end = u64le(&data, off + 0x30);
            if (0.0001..1000.0).contains(&step)
                && count > 0
                && count < 100_000
                && data_off >= 0x38
                && data_off < data.len() as u64
            {
                let n = count as usize;
                let avail = ((data.len() as u64 - data_off) / 6) as usize;
                let n = n.min(avail);
                let mut points = Vec::with_capacity(n);
                for i in 0..n {
                    let o = data_off as usize + i * 6;
                    points.push([
                        i16::from_le_bytes([data[o], data[o + 1]]),
                        i16::from_le_bytes([data[o + 2], data[o + 3]]),
                        i16::from_le_bytes([data[o + 4], data[o + 5]]),
                    ]);
                }
                let dist_base = (data_end as usize) + 4;
                let mut distances = Vec::with_capacity(n);
                for i in 0..n {
                    let o = dist_base + i * 4;
                    if o + 4 > data.len() {
                        break;
                    }
                    distances.push(f32le(&data, o));
                }
                sections.push(RadaSection {
                    header_offset: off as u64,
                    count,
                    flags: u16le(&data, off + 4),
                    step,
                    vector_a: [
                        f32le(&data, off + 0x0C),
                        f32le(&data, off + 0x10),
                        f32le(&data, off + 0x14),
                    ],
                    vector_b: [
                        f32le(&data, off + 0x18),
                        f32le(&data, off + 0x1C),
                        f32le(&data, off + 0x20),
                    ],
                    data_offset: data_off,
                    data_end,
                    points,
                    distances,
                });
            }
        }
        off += 4;
    }
    Ok(RadaFile {
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        size: data.len() as u64,
        sections,
    })
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l <= f32::EPSILON {
        [0.0, 0.0, 0.0]
    } else {
        [v[0] / l, v[1] / l, v[2] / l]
    }
}

/// Reconstruct a polyline: start at `vector_b` and advance along each unit
/// direction by the arc-length delta.  The resulting shape and scale match the
/// recorded lap length, but closure is approximate (the exact frame convention
/// of the per-point vectors is still under investigation).
pub fn reconstruct(section: &RadaSection) -> Vec<[f32; 3]> {
    let n = section.points.len().min(section.distances.len());
    let mut out = Vec::with_capacity(n);
    let mut p = section.vector_b;
    out.push(p);
    for i in 1..n {
        let mut ds = section.distances[i] - section.distances[i - 1];
        if !(0.0..=100.0).contains(&ds) {
            ds = 1.0;
        }
        let v = norm([
            section.points[i - 1][0] as f32,
            section.points[i - 1][1] as f32,
            section.points[i - 1][2] as f32,
        ]);
        p = [p[0] + v[0] * ds, p[1] + v[1] * ds, p[2] + v[2] * ds];
        out.push(p);
    }
    out
}

pub fn write_polyline_csv(section: &RadaSection, path: &Path) -> Result<()> {
    let pts = reconstruct(section);
    let mut s = String::with_capacity(pts.len() * 40);
    s.push_str("index,x,y,z\n");
    for (i, p) in pts.iter().enumerate() {
        s.push_str(&format!("{i},{:.4},{:.4},{:.4}\n", p[0], p[1], p[2]));
    }
    fs::write(path, s)?;
    Ok(())
}
