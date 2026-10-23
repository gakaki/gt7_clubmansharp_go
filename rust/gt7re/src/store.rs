use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Digit -> character table used by Polyphony's cache-key encoding.
pub const TABLE: &[u8; 36] = b"P6MH85C24L91ZTIYURQ7NKJ0OSBWDAVGX3FE";
const FLAG_BOUNDARY: u32 = 0x81BF_0FFF;

/// FNV-1a (32 bit) over the raw asset path bytes.
pub fn fnv1a(data: &[u8]) -> u32 {
    let mut h: u32 = 0x811C_9DC5;
    for &b in data {
        h ^= b as u32;
        h = h.wrapping_mul(16_777_619);
    }
    h
}

/// Encode a 32-bit node id as the on-disk relative path `a/b/XXXXX`.
pub fn cache_key(x: u32) -> String {
    let digit = |i: u32| -> char {
        let mut v = x;
        for _ in 0..i {
            v /= 36;
        }
        TABLE[(v % 36) as usize] as char
    };
    let flag = if x >= FLAG_BOUNDARY { TABLE[1] } else { TABLE[0] } as char;
    format!(
        "{}/{}/{}{}{}{}{}",
        digit(0),
        digit(1),
        flag,
        digit(5),
        digit(4),
        digit(3),
        digit(2)
    )
}

/// The asset content store (`<game>/contents`).
pub struct Store {
    pub root: PathBuf,
    pub rels: HashSet<String>,
}

impl Store {
    pub fn scan(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        let mut rels = HashSet::with_capacity(100_000);
        for e0 in fs::read_dir(&root)? {
            let e0 = e0?;
            if !e0.file_type()?.is_dir() {
                continue;
            }
            let n0 = e0.file_name();
            let s0 = n0.to_string_lossy();
            if s0.len() != 1 || !TABLE.contains(&s0.as_bytes()[0]) {
                continue;
            }
            for e1 in fs::read_dir(e0.path())? {
                let e1 = e1?;
                if !e1.file_type()?.is_dir() {
                    continue;
                }
                let s1 = e1.file_name().to_string_lossy().to_string();
                if s1.len() != 1 || !TABLE.contains(&s1.as_bytes()[0]) {
                    continue;
                }
                for f in fs::read_dir(e1.path())? {
                    let f = f?;
                    if !f.file_type()?.is_file() {
                        continue;
                    }
                    rels.insert(format!("{}/{}/{}", s0, s1, f.file_name().to_string_lossy()));
                }
            }
        }
        Ok(Self { root, rels })
    }

    pub fn contains_rel(&self, rel: &str) -> bool {
        self.rels.contains(rel)
    }

    pub fn rel_for(&self, asset_path: &str) -> String {
        cache_key(fnv1a(asset_path.as_bytes()))
    }

    /// Resolve an asset path (case / leading-slash tolerant) to a store-relative path.
    pub fn resolve(&self, asset_path: &str) -> Option<String> {
        let mut variants = Vec::with_capacity(6);
        let no_lead = asset_path.trim_start_matches('/');
        variants.push(asset_path.to_string());
        variants.push(no_lead.to_string());
        variants.push(asset_path.to_lowercase());
        variants.push(no_lead.to_lowercase());
        let mut seen = HashSet::new();
        for v in variants {
            if !seen.insert(v.clone()) {
                continue;
            }
            let rel = self.rel_for(&v);
            if self.rels.contains(&rel) {
                return Some(rel);
            }
        }
        None
    }

    pub fn file_for(&self, asset_path: &str) -> Option<PathBuf> {
        self.resolve(asset_path).map(|r| self.path_of_rel(&r))
    }

    pub fn path_of_rel(&self, rel: &str) -> PathBuf {
        let mut p = self.root.clone();
        for part in rel.split('/') {
            p.push(part);
        }
        p
    }

    pub fn stats(&self) -> (usize, u64) {
        let mut bytes = 0u64;
        for rel in &self.rels {
            if let Ok(m) = fs::metadata(self.path_of_rel(rel)) {
                bytes += m.len();
            }
        }
        (self.rels.len(), bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_hash_and_key() {
        let p = "common/render.txs";
        assert_eq!(fnv1a(p.as_bytes()), 0x8E7C_4EE6);
        assert_eq!(cache_key(fnv1a(p.as_bytes())), "J/Z/6H74F");
    }

    #[test]
    fn more_known_paths() {
        let cases = [
            ("sound_gt/etc/files.json", "5/7/6VIOL"),
            ("scripts/gt7/main.adc", "D/4/PM1EF"),
            ("rtext/common/us.rt2", "3/L/6AXPV"),
            ("font/ps4/fontset_jp.gpb", "M/P/PZZ8P"),
        ];
        for (p, expected) in cases {
            assert_eq!(cache_key(fnv1a(p.as_bytes())), expected, "{p}");
        }
    }
}
