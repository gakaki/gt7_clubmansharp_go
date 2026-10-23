use crate::packed;
use crate::store::Store;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug)]
pub struct Named {
    pub asset: String,
    pub rel: String,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Copy,
    Link,
}

#[derive(Default, Debug)]
pub struct ExtractStats {
    pub extracted: usize,
    pub missing: usize,
    pub failed: usize,
    pub bytes: u64,
}

pub fn load_named(path: &Path) -> Result<Vec<Named>> {
    let text = fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        if let Some((a, r)) = line.split_once('\t') {
            let a = a.trim();
            let r = r.trim();
            if !a.is_empty() && !r.is_empty() {
                out.push(Named { asset: a.to_string(), rel: r.to_string() });
            }
        }
    }
    Ok(out)
}

fn safe_dest(root: &Path, asset: &str) -> PathBuf {
    let mut p = root.to_path_buf();
    for part in asset.trim_start_matches('/').split('/') {
        if part.is_empty() || part == "." || part == ".." {
            continue;
        }
        p.push(part);
    }
    p
}

pub fn extract_named(
    store: &Store,
    entries: &[Named],
    outdir: &Path,
    mode: Mode,
    limit: Option<usize>,
) -> Result<ExtractStats> {
    let slice = match limit {
        Some(n) if n < entries.len() => &entries[..n],
        _ => entries,
    };
    let done = AtomicUsize::new(0);
    let failures = AtomicUsize::new(0);
    let stats: Vec<(bool, u64)> = slice
        .par_iter()
        .map(|e| {
            let src = store.path_of_rel(&e.rel);
            if !src.is_file() {
                return (false, 0);
            }
            let dest = safe_dest(outdir, &e.asset);
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let res: Result<u64> = (|| {
                match mode {
                    Mode::Link => {
                        if dest.exists() {
                            let _ = fs::remove_file(&dest);
                        }
                        fs::hard_link(&src, &dest)
                            .with_context(|| format!("hardlink {} -> {}", src.display(), dest.display()))?;
                        Ok(fs::metadata(&dest)?.len())
                    }
                    Mode::Copy => {
                        let raw = fs::read(&src)?;
                        let unpacked = packed::unpack(&raw);
                        let (payload, _kind) = match unpacked {
                            Ok(v) => v,
                            Err(_) => (raw, "raw"),
                        };
                        fs::write(&dest, &payload)?;
                        Ok(payload.len() as u64)
                    }
                }
            })();
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 500 == 0 {
                eprintln!("  extracted {n}/{}", slice.len());
            }
            match res {
                Ok(sz) => (true, sz),
                Err(err) => {
                    let n = failures.fetch_add(1, Ordering::Relaxed);
                    if n < 10 {
                        eprintln!("FAIL {}: {err}", e.asset);
                    }
                    (false, 0)
                }
            }
        })
        .collect();

    let mut st = ExtractStats::default();
    for (ok, sz) in stats {
        if ok {
            st.extracted += 1;
            st.bytes += sz;
        } else {
            st.failed += 1;
        }
    }
    st.missing = slice.len().saturating_sub(st.extracted + st.failed);
    Ok(st)
}
