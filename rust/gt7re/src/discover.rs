//! Parallel asset-name discovery.

use crate::store::Store;
use anyhow::Result;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

pub fn load_named(path: &Path) -> HashMap<String, String> {
    let mut m = HashMap::new();
    if let Ok(text) = fs::read_to_string(path) {
        for line in text.lines() {
            if let Some((a, r)) = line.split_once('\t') {
                m.insert(a.trim().to_string(), r.trim().to_string());
            }
        }
    }
    m
}

pub fn save_named(path: &Path, named: &HashMap<String, String>) -> Result<()> {
    let mut v: Vec<(&String, &String)> = named.iter().collect();
    v.sort_by(|a, b| a.0.cmp(b.0));
    let mut s = String::with_capacity(v.len() * 40);
    for (a, r) in v {
        s.push_str(a);
        s.push('\t');
        s.push_str(r);
        s.push('\n');
    }
    if let Some(p) = path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(path, s)?;
    Ok(())
}

/// Test candidates in parallel; returns (candidate, rel) for hits not already known.
pub fn test_candidates(store: &Store, cands: &[String]) -> Vec<(String, String)> {
    cands
        .par_chunks(4096)
        .flat_map_iter(|chunk| {
            chunk.iter().filter_map(|c| {
                let rel = store.rel_for(c);
                if store.contains_rel(&rel) {
                    Some((c.clone(), rel))
                } else {
                    None
                }
            })
        })
        .collect()
}

pub const STATIC_NAMES: &[&str] = &[
    "body",
    "body_lod0",
    "lod0",
    "lod1",
    "lod2",
    "hq",
    "lq",
    "race",
    "normal",
    "damage",
    "interior",
    "cockpit",
    "steering",
    "wheel",
    "wheels",
    "tire",
    "tyre",
    "rim",
    "brake",
    "engine",
    "exhaust",
    "meter",
    "gauge",
    "needle",
    "seat",
    "mirror",
    "spoiler",
    "wing",
    "light",
    "headlight",
    "taillight",
    "shadow",
    "chassis",
    "suspension",
    "aero",
    "driver",
    "texture",
    "tex",
    "txs",
    "image",
    "img",
    "thumbnail",
    "thumb",
    "icon",
    "logo",
    "paint",
    "color",
    "colour",
    "livery",
    "decal",
    "stripe",
    "pattern",
    "skin",
    "normalmap",
    "spec",
    "mask",
    "preview",
    "data",
    "param",
    "params",
    "parameter",
    "specs",
    "setting",
    "config",
    "course",
    "track",
    "circuit",
    "layout",
    "scenery",
    "crowd",
    "road",
    "kerb",
    "curb",
    "race_line",
    "raceline",
    "rline",
    "line",
    "curve",
    "path",
    "spline",
    "ai",
    "aipath",
    "camera",
    "camera_motion",
    "replay",
    "ghost",
    "voice",
    "dialog",
    "movie",
    "cutscene",
    "video",
    "bgm",
    "music",
    "sound",
    "sfx",
    "ui",
    "menu",
    "window",
    "widget",
    "button",
    "font",
    "text",
    "string",
    "label",
    "message",
    "help",
    "title",
    "name",
    "description",
    "main",
    "boot",
    "root",
    "gpb",
    "adc",
    "mproject",
    "layout",
    "screen",
    "page",
    "panel",
];

pub const EXTENSIONS: &[&str] = &[
    "mdl5", "mdl4", "mdl", "txs", "txs5", "gpb", "adc", "mproject", "json", "bin", "dat", "sdb",
    "rt2", "pac", "szd", "szd3", "irf", "wav", "mp4", "dds", "png", "jpg", "xml", "csv", "strb",
    "localdb", "mtl5", "shd5", "tex5", "ani5", "rig5", "col5", "crs", "rline", "spline", "aipath",
    "txt", "ini", "fpc", "img", "svg",
];

pub fn expand_candidates(named: &HashMap<String, String>, depth2: bool) -> Vec<String> {
    let mut dirs: HashSet<&str> = HashSet::new();
    let mut vocab: HashSet<String> = STATIC_NAMES.iter().map(|s| s.to_string()).collect();
    for p in named.keys() {
        if let Some((d, _base)) = p.rsplit_once('/') {
            dirs.insert(d);
        }
        let base = p.rsplit('/').next().unwrap_or(p);
        vocab.insert(base.to_string());
        if let Some((stem, _)) = base.rsplit_once('.') {
            vocab.insert(stem.to_string());
            for e in EXTENSIONS {
                vocab.insert(format!("{stem}.{e}"));
            }
        }
    }
    let vocab: Vec<&String> = vocab.iter().collect();
    let mut out = Vec::new();
    for d in &dirs {
        for v in &vocab {
            out.push(format!("{d}/{v}"));
        }
    }
    if depth2 {
        let short: Vec<&&String> = vocab.iter().filter(|v| v.len() <= 24).take(80).collect();
        for d in dirs.iter().take(2000) {
            for a in &short {
                for b in &short {
                    out.push(format!("{d}/{a}/{b}"));
                }
            }
        }
    }
    out
}

/// Extract path-like ASCII tokens from already-named data files.
pub fn mine_named_files(store: &Store, named: &HashMap<String, String>) -> Vec<String> {
    let exts = [
        ".json",
        ".txt",
        ".csv",
        ".xml",
        ".ini",
        ".adc",
        ".gpb",
        ".mproject",
        ".dat",
        ".bin",
        ".fpc",
        ".strb",
    ];
    let entries: Vec<(&String, &String)> = named
        .iter()
        .filter(|(a, _)| exts.iter().any(|e| a.to_lowercase().ends_with(e)))
        .collect();
    let mut tokens: HashSet<String> = HashSet::new();
    let found: Vec<HashSet<String>> = entries
        .par_iter()
        .map(|(_, rel)| {
            let mut set = HashSet::new();
            let p = store.path_of_rel(rel.as_str());
            let Ok(meta) = fs::metadata(&p) else {
                return set;
            };
            if meta.len() > 8 << 20 {
                return set;
            }
            let Ok(data) = fs::read(&p) else { return set };
            let mut run = Vec::new();
            for &b in data.iter().chain(std::iter::once(&0)) {
                if (0x20..0x7F).contains(&b) {
                    run.push(b);
                } else {
                    if run.len() >= 3 {
                        let s = String::from_utf8_lossy(&run).to_string();
                        for piece in
                            s.split(|c: char| c.is_whitespace() || "\"',;:()[]{}=<>|".contains(c))
                        {
                            let piece = piece.trim();
                            if piece.len() >= 3
                                && piece.len() <= 160
                                && piece
                                    .chars()
                                    .all(|c| c.is_ascii_alphanumeric() || "_-.+/".contains(c))
                            {
                                set.insert(piece.to_string());
                                if let Some(last) = piece.rsplit('/').next() {
                                    set.insert(last.to_string());
                                }
                            }
                        }
                    }
                    run.clear();
                }
            }
            set
        })
        .collect();
    for s in found {
        tokens.extend(s);
    }
    tokens.into_iter().collect()
}
