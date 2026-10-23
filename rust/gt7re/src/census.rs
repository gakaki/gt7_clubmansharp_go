use crate::packed;
use crate::store::Store;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::Path;

#[derive(Default, Debug, Clone)]
pub struct CensusSummary {
    pub files: usize,
    pub bytes: u64,
    pub by_kind: HashMap<String, (usize, u64)>,
}

fn starts(head: &[u8], pat: &[u8]) -> bool {
    head.len() >= pat.len() && &head[..pat.len()] == pat
}

pub fn classify(head: &[u8]) -> String {
    if head.len() >= 4 {
        let m = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
        let packed = match m {
            packed::MAGIC_ZLIB => Some("packed:zlib"),
            packed::MAGIC_ZSTD_TINY => Some("packed:zstd-tiny"),
            packed::MAGIC_ZSTD_REGULAR => Some("packed:zstd-regular"),
            packed::MAGIC_ZSTD_CHUNK => Some("packed:zstd-chunk"),
            _ => None,
        };
        if let Some(k) = packed {
            return k.to_string();
        }
    }
    if head.len() >= 8 && &head[4..8] == b"JXL " {
        return "jpeg-xl".to_string();
    }
    let table: &[(&[u8], &str)] = &[
        (b"MDL5", "model-3d (MDL5)"),
        (b"MDL4", "model-3d (MDL4)"),
        (b"MDL3", "model-3d (MDL3)"),
        (b"PACL", "PAC archive (PACL)"),
        (b"PACC", "PAC archive (PACC)"),
        (b"PACS", "PAC archive (PACS)"),
        (b"IDP0", "IDP0 container"),
        (b"SCPS", "SCPS container"),
        (b"TXS5", "TXS5 texture-set"),
        (b"TXS\x00", "TXS texture-set"),
        (b"\x89PNG", "png image"),
        (b"\xff\xd8\xff", "jpeg image"),
        (b"RIFF", "riff (wav/webp)"),
        (b"OggS", "ogg audio"),
        (b"DDS ", "dds texture"),
        (b"PSP\x00", "PSP blob"),
        (b"PK\x03\x04", "zip"),
        (b"{\r\n", "json/text"),
        (b"{\n", "json/text"),
        (b"[\n", "json/text"),
        (b"GTFS", "GTFS"),
        (b"SDB\x00", "spec db"),
        (b"SNDZ", "sound (SNDZ)"),
        (b"STRB", "string bank (STRB)"),
        (b"MPRJ", "UI project (MPRJ)"),
        (b"CAR5", "car data (CAR5)"),
        (b"EBAF", "EBAF"),
        (b"EVSF", "EVSF"),
        (b"SCNB", "scene (SCNB)"),
        (b"SDEF", "SDEF"),
        (b"GTEB", "GTEB"),
        (b"GTED", "GTED"),
        (b"MAP4", "MAP4"),
        (b"ADSP", "ADSP"),
        (b"GCM7", "GCM7"),
        (b"RCMD", "RCMD"),
        (b"EXPD", "EXPD"),
    ];
    for (pat, name) in table {
        if starts(head, pat) {
            return (*name).to_string();
        }
    }
    if head
        .first()
        .map(|b| *b == b'{' || *b == b'[')
        .unwrap_or(false)
        || head
            .iter()
            .take(32)
            .all(|b| (0x09..0x7F).contains(b) || *b == 0x0A || *b == 0x0D)
    {
        return "text/unknown".to_string();
    }
    if head.len() >= 4 && head[..4] == [0, 0, 0, 0] {
        return "zero-filled".to_string();
    }
    if head.len() >= 4 {
        return format!(
            "unknown:{:02X}{:02X}{:02X}{:02X}",
            head[0], head[1], head[2], head[3]
        );
    }
    "empty".to_string()
}

pub fn write_census(store: &Store, out: &Path) -> Result<CensusSummary> {
    let mut rels: Vec<&String> = store.rels.iter().collect();
    rels.sort();

    let rows: Vec<(String, u64, String, Vec<u8>)> = rels
        .par_iter()
        .map(|rel| {
            let p = store.path_of_rel(rel);
            let size = fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            let mut head = Vec::with_capacity(32);
            if let Ok(mut f) = fs::File::open(&p) {
                let mut buf = [0u8; 32];
                if let Ok(n) = f.read(&mut buf) {
                    head.extend_from_slice(&buf[..n]);
                }
            }
            let kind = classify(&head);
            ((*rel).to_string(), size, kind, head)
        })
        .collect();

    let mut summary = CensusSummary::default();
    let mut w = csv::Writer::from_path(out)?;
    w.write_record(["rel", "size", "kind", "head"])?;
    for (rel, size, kind, head) in rows {
        w.write_record([
            rel.as_str(),
            &size.to_string(),
            kind.as_str(),
            &head.iter().map(|b| format!("{b:02X}")).collect::<String>(),
        ])?;
        summary.files += 1;
        summary.bytes += size;
        let e = summary.by_kind.entry(kind).or_insert((0, 0));
        e.0 += 1;
        e.1 += size;
    }
    w.flush()?;
    Ok(summary)
}

pub fn write_report(summary: &CensusSummary, out: &Path) -> Result<()> {
    let mut f = BufWriter::new(fs::File::create(out)?);
    writeln!(
        f,
        "files: {}\nbytes: {:.2} GB\n",
        summary.files,
        summary.bytes as f64 / 2f64.powi(30)
    )?;
    let mut v: Vec<_> = summary.by_kind.iter().collect();
    v.sort_by_key(|item| std::cmp::Reverse(item.1 .0));
    writeln!(f, "{:<40} {:>9} {:>10}", "kind", "count", "size(GB)")?;
    for (kind, (n, bytes)) in v {
        writeln!(
            f,
            "{:<40} {:>9} {:>10.2}",
            kind,
            n,
            *bytes as f64 / 2f64.powi(30)
        )?;
    }
    Ok(())
}

/// Read a census CSV back into `(rel, size, kind)` rows.
pub fn read_catalog(csv_path: &Path) -> Result<Vec<(String, u64, String)>> {
    let mut r = csv::Reader::from_path(csv_path)?;
    let mut out = Vec::new();
    for rec in r.records() {
        let rec = rec?;
        let rel = rec.get(0).unwrap_or("").to_string();
        let size = rec.get(1).and_then(|s| s.parse().ok()).unwrap_or(0u64);
        let kind = rec.get(2).unwrap_or("").to_string();
        if !rel.is_empty() {
            out.push((rel, size, kind));
        }
    }
    Ok(out)
}

pub fn sanitize_kind(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

/// Split a census into per-kind lists under `outdir/catalog/<kind>.txt`.
pub fn write_catalog(
    rows: &[(String, u64, String)],
    outdir: &Path,
) -> Result<Vec<(String, usize, u64)>> {
    fs::create_dir_all(outdir)?;
    let mut map: HashMap<String, Vec<(String, u64)>> = HashMap::new();
    for (rel, size, kind) in rows {
        map.entry(kind.clone())
            .or_default()
            .push((rel.clone(), *size));
    }
    let mut summary = Vec::new();
    for (kind, items) in &map {
        let path = outdir.join(format!("{}.txt", sanitize_kind(kind)));
        let mut s = String::new();
        let mut bytes = 0u64;
        for (rel, size) in items {
            s.push_str(rel);
            s.push('\t');
            s.push_str(&size.to_string());
            s.push('\n');
            bytes += *size;
        }
        fs::write(path, s)?;
        summary.push((kind.clone(), items.len(), bytes));
    }
    summary.sort_by_key(|item| std::cmp::Reverse(item.1));
    Ok(summary)
}
