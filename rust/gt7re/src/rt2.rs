//! Polyphony `.rt2` localisation container parsing + keystream recovery.
//!
//! Every string payload is `cipher[i] = plaintext[i] XOR keystream[i]` where the
//! same keystream is reused for all payloads of a table.  The keystream is
//! recovered per byte offset by frequency analysis, which works because the
//! plaintext is natural-language UTF-8.  A payload of length `n` ends with a
//! NUL terminator at `n-1`; that is scored separately so terminators do not
//! pollute the text statistics.

use anyhow::{bail, Result};
use rayon::prelude::*;
use serde_json::{Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Entry {
    pub id: u32,
    pub tag: u16,
    pub size: u16,
    pub blob_off: u64,
    pub payload_off: u64,
}

#[derive(Clone, Debug)]
pub struct Section {
    pub name: String,
    pub entries: Vec<Entry>,
}

pub struct Rt2File {
    pub data: Vec<u8>,
    pub sections: Vec<Section>,
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

impl Rt2File {
    pub fn parse(path: &Path) -> Result<Self> {
        let data = fs::read(path)?;
        if data.len() < 0x20 {
            bail!("file too small for rt2");
        }
        let count = u32le(&data, 4) as usize;
        if count == 0 || count > 4096 || 0x20 + count * 0x18 > data.len() {
            bail!("invalid rt2/section count {count}");
        }
        let mut sections = Vec::with_capacity(count);
        for i in 0..count {
            let o = 0x20 + i * 0x18;
            if o + 0x18 > data.len() {
                break;
            }
            let name_off = u64le(&data, o) as usize;
            let n = u64le(&data, o + 8) as usize;
            let entries_off = u64le(&data, o + 16) as usize;
            let name = if name_off < data.len() {
                let end = data[name_off..]
                    .iter()
                    .position(|b| *b == 0)
                    .map(|p| name_off + p)
                    .unwrap_or(data.len());
                String::from_utf8_lossy(&data[name_off..end]).into_owned()
            } else {
                String::new()
            };
            let mut entries = Vec::with_capacity(n);
            for j in 0..n {
                let e = entries_off + j * 0x18;
                if e + 0x18 > data.len() {
                    break;
                }
                entries.push(Entry {
                    id: u32le(&data, e),
                    tag: u16le(&data, e + 4),
                    size: u16le(&data, e + 6),
                    blob_off: u64le(&data, e + 8),
                    payload_off: u64le(&data, e + 16),
                });
            }
            sections.push(Section { name, entries });
        }
        Ok(Self { data, sections })
    }

    pub fn payloads(&self) -> Vec<(usize, u32, &[u8])> {
        let mut out = Vec::new();
        for (si, sec) in self.sections.iter().enumerate() {
            for e in &sec.entries {
                let start = e.payload_off as usize;
                let size = e.size as usize;
                if start < self.data.len() {
                    let end = (start + size).min(self.data.len());
                    out.push((si, e.id, &self.data[start..end]));
                }
            }
        }
        out
    }

    pub fn max_payload_len(&self) -> usize {
        self.sections
            .iter()
            .flat_map(|s| s.entries.iter())
            .map(|e| e.size as usize)
            .max()
            .unwrap_or(0)
    }
}

fn text_model() -> [f32; 256] {
    let mut w = [0.002f32; 256];
    let mut add = |ch: u8, v: f32| w[ch as usize] += v;
    for (ch, v) in [
        (b' ', 18.0),
        (b'e', 10.2),
        (b't', 7.5),
        (b'a', 6.6),
        (b'o', 6.1),
        (b'i', 5.7),
        (b'n', 5.5),
        (b's', 5.2),
        (b'r', 5.0),
        (b'h', 5.0),
        (b'l', 3.3),
        (b'd', 3.3),
        (b'c', 2.3),
        (b'u', 2.3),
        (b'm', 2.0),
        (b'f', 1.8),
        (b'p', 1.6),
        (b'g', 1.6),
        (b'w', 1.6),
        (b'y', 1.5),
        (b'b', 1.2),
        (b'v', 0.8),
        (b'k', 0.6),
        (b'x', 0.15),
        (b'j', 0.12),
        (b'z', 0.07),
        (b'q', 0.08),
        (b'E', 1.0),
        (b'T', 0.9),
        (b'A', 0.9),
        (b'O', 0.7),
        (b'I', 0.6),
        (b'N', 0.6),
        (b'S', 0.6),
        (b'R', 0.6),
        (b'H', 0.5),
        (b'L', 0.4),
        (b'D', 0.4),
        (b'C', 0.4),
        (b'U', 0.3),
        (b'M', 0.3),
        (b'P', 0.3),
        (b'B', 0.25),
        (b'G', 0.25),
        (b'F', 0.2),
        (b'W', 0.2),
        (b'Y', 0.2),
        (b'V', 0.1),
        (b'K', 0.1),
        (b'J', 0.08),
        (b'X', 0.03),
        (b'Z', 0.03),
        (b'Q', 0.03),
        (b'.', 1.2),
        (b',', 1.2),
        (b'\'', 0.8),
        (b'-', 0.5),
        (b'"', 0.4),
        (b'(', 0.2),
        (b')', 0.2),
        (b':', 0.2),
        (b';', 0.15),
        (b'!', 0.15),
        (b'?', 0.15),
        (b'/', 0.15),
        (b'&', 0.1),
        (b'%', 0.1),
        (b'+', 0.08),
        (b'=', 0.08),
        (b'*', 0.05),
        (b'@', 0.03),
        (b'#', 0.03),
        (b'$', 0.03),
        (b'_', 0.05),
        (b'|', 0.02),
        (b'~', 0.02),
        (b'<', 0.02),
        (b'>', 0.02),
        (b'[', 0.02),
        (b']', 0.02),
        (b'{', 0.01),
        (b'}', 0.01),
        (b'^', 0.01),
        (b'`', 0.01),
        (b'\\', 0.01),
        (b'0', 0.3),
        (b'1', 0.3),
        (b'2', 0.25),
        (b'3', 0.2),
        (b'4', 0.15),
        (b'5', 0.15),
        (b'6', 0.15),
        (b'7', 0.13),
        (b'8', 0.13),
        (b'9', 0.13),
        (b'\n', 0.5),
        (b'\r', 0.3),
        (b'\t', 0.1),
    ] {
        add(ch, v);
    }
    for b in 0xC2..0xF5u16 {
        w[b as usize] = 2.0;
    }
    for b in 0x80..0xC0u16 {
        w[b as usize] = 3.0;
    }
    w.map(|v| v.ln())
}

fn first_model() -> [f32; 256] {
    let mut w = [0.02f32; 256];
    for b in b'A'..=b'Z' {
        w[b as usize] = 3.0;
    }
    for b in b'a'..=b'z' {
        w[b as usize] = 1.2;
    }
    for b in b'0'..=b'9' {
        w[b as usize] = 0.5;
    }
    for (ch, v) in [
        (b'<', 2.5),
        (b'"', 0.6),
        (b'\'', 0.4),
        (b'(', 0.3),
        (b'[', 0.2),
        (b'{', 0.1),
        (b'/', 0.2),
        (b'-', 0.2),
        (b' ', 0.05),
    ] {
        w[ch as usize] = v;
    }
    w[0] = 0.001;
    w.map(|v| v.ln())
}

/// Recover the per-offset keystream.  CPU-bound and parallelised over offsets.
pub fn recover_keystream(payloads: &[(usize, u32, &[u8])], maxlen: usize) -> Vec<u8> {
    let model = text_model();
    let first = first_model();
    recover_keystream_with_models(payloads, maxlen, &model, &first)
}

pub fn recover_keystream_with_counts(
    payloads: &[(usize, u32, &[u8])],
    maxlen: usize,
    counts: &[u64; 256],
    first_counts: &[u64; 256],
) -> Vec<u8> {
    let model = counts.map(|count| ((count + 1) as f32).ln());
    let first = first_counts.map(|count| ((count + 1) as f32).ln());
    recover_keystream_with_models(payloads, maxlen, &model, &first)
}

fn recover_keystream_with_models(
    payloads: &[(usize, u32, &[u8])],
    maxlen: usize,
    model: &[f32; 256],
    first: &[f32; 256],
) -> Vec<u8> {
    (0..maxlen)
        .into_par_iter()
        .map(|pos| {
            let mut counts = [0u32; 256];
            let mut term = 0u32;
            for (_, _, p) in payloads {
                if p.len() > pos + 1 {
                    counts[p[pos] as usize] += 1;
                } else if p.len() == pos + 1 {
                    term += 1;
                }
            }
            let m = if pos == 0 { first } else { model };
            let mut best_k = 0u8;
            let mut best = f32::NEG_INFINITY;
            for k in 0..256usize {
                let mut s = 0f32;
                for b in 0..256usize {
                    let c = counts[b];
                    if c != 0 {
                        s += c as f32 * m[b ^ k];
                    }
                }
                if term != 0 {
                    s += term as f32 * m[k];
                }
                if s > best {
                    best = s;
                    best_k = k as u8;
                }
            }
            best_k
        })
        .collect()
}

fn clean_text(mut v: Vec<u8>) -> String {
    while v.last() == Some(&0) {
        v.pop();
    }
    loop {
        match std::str::from_utf8(&v) {
            Ok(s) => return s.to_string(),
            Err(e) => {
                let at_end = e.valid_up_to() + 4 >= v.len();
                if at_end && e.valid_up_to() > 0 {
                    v.truncate(e.valid_up_to());
                } else {
                    return String::from_utf8_lossy(&v).into_owned();
                }
            }
        }
    }
}

pub struct DecodedString {
    pub section: String,
    pub id: u32,
    pub text: String,
}

pub fn decode(rt: &Rt2File, ks: &[u8]) -> Vec<DecodedString> {
    let mut out = Vec::new();
    for sec in &rt.sections {
        for e in &sec.entries {
            let start = e.payload_off as usize;
            let size = e.size as usize;
            if start >= rt.data.len() || size == 0 {
                continue;
            }
            let end = (start + size).min(rt.data.len());
            let n = (end - start).min(ks.len());
            let v: Vec<u8> = rt.data[start..start + n]
                .iter()
                .enumerate()
                .map(|(i, b)| b ^ ks[i])
                .collect();
            out.push(DecodedString {
                section: sec.name.clone(),
                id: e.id,
                text: clean_text(v),
            });
        }
    }
    out
}

pub fn write_outputs(
    strings: &[DecodedString],
    stem: &str,
    outdir: &Path,
) -> Result<(PathBuf, PathBuf)> {
    fs::create_dir_all(outdir)?;
    let mut root = Map::new();
    for s in strings {
        let sec = root
            .entry(s.section.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        if let Value::Object(m) = sec {
            m.insert(format!("0x{:08X}", s.id), Value::String(s.text.clone()));
        }
    }
    let json_path = outdir.join(format!("{stem}.json"));
    fs::write(&json_path, serde_json::to_vec_pretty(&Value::Object(root))?)?;

    let csv_path = outdir.join(format!("{stem}.csv"));
    let mut w = csv::Writer::from_path(&csv_path)?;
    w.write_record(["section", "id", "text"])?;
    for s in strings {
        w.write_record([
            s.section.as_str(),
            &format!("0x{:08X}", s.id),
            s.text.as_str(),
        ])?;
    }
    w.flush()?;
    Ok((json_path, csv_path))
}

pub fn auto_decode(path: &Path, outdir: &Path, keydir: Option<&Path>) -> Result<(usize, usize)> {
    let rt = Rt2File::parse(path)?;
    let payloads = rt.payloads();
    let maxlen = rt.max_payload_len();
    eprintln!(
        "{}: {} sections, {} strings, max payload {}",
        path.file_name().unwrap_or_default().to_string_lossy(),
        rt.sections.len(),
        payloads.len(),
        maxlen
    );
    let ks = recover_keystream(&payloads, maxlen);
    if let Some(kd) = keydir {
        fs::create_dir_all(kd)?;
        fs::write(
            kd.join(format!(
                "{}.ks.bin",
                path.file_stem().unwrap_or_default().to_string_lossy()
            )),
            &ks,
        )?;
    }
    let strings = decode(&rt, &ks);
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    write_outputs(&strings, &stem, outdir)?;
    Ok((ks.len(), strings.len()))
}

/// Decode one file using a keystream recovered elsewhere (the keystream is
/// shared by every localisation file).
pub fn decode_with_key(path: &Path, ks: &[u8], outdir: &Path) -> Result<usize> {
    let rt = Rt2File::parse(path)?;
    let strings = decode(&rt, ks);
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    write_outputs(&strings, &stem, outdir)?;
    Ok(strings.len())
}

pub fn tag_counts(path: &Path, key: &[u8], section: &str, limit: usize) -> Result<usize> {
    let rt = Rt2File::parse(path)?;
    let regex = regex::Regex::new(r"<[^>]+>").expect("tag regex");
    let attribute = regex::Regex::new(r#"\s+\w+="[^"]*""#).expect("attribute regex");
    let mut counts = std::collections::BTreeMap::<String, usize>::new();
    for item in decode(&rt, key) {
        if item.section != section {
            continue;
        }
        for tag in regex.find_iter(&item.text) {
            let normalized = attribute.replace_all(tag.as_str(), "").into_owned();
            *counts.entry(normalized).or_default() += 1;
        }
    }
    let mut values = counts.into_iter().collect::<Vec<_>>();
    values.sort_by_key(|item| std::cmp::Reverse(item.1));
    for (tag, count) in values.iter().take(limit) {
        println!("{count:6}  {tag}");
    }
    Ok(values.len())
}
