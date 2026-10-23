//! Polyphony GPB4 bundle extractor used by the GT7 PS5 DMS database.
//!
//! `contents/3/S/PRPYS` starts with `4bpg`, stores an entry count at `0x10`,
//! and uses a 0x20-byte entry table starting at `0x14`:
//!   +0x00 u64 name offset (usually `/dmsdata/<hash>/<name>.dmsb`)
//!   +0x08 u64 payload offset
//!   +0x10 u64 unknown / flags
//!   +0x18 u64 payload size

use anyhow::{bail, Context, Result};
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpb4Entry {
    pub index: usize,
    pub name: String,
    pub name_offset: u64,
    pub data_offset: u64,
    pub size: u64,
    pub flags: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Gpb4Archive {
    pub source: String,
    pub file_size: usize,
    pub count: usize,
    pub entries: Vec<Gpb4Entry>,
}

fn u32le(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = data.get(offset..offset + 4).context("u32 out of range")?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn u64le(data: &[u8], offset: usize) -> Result<u64> {
    let bytes = data.get(offset..offset + 8).context("u64 out of range")?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn c_string(data: &[u8], offset: usize) -> Result<String> {
    let tail = data.get(offset..).context("string offset out of range")?;
    let end = tail
        .iter()
        .position(|&byte| byte == 0)
        .context("unterminated GPB4 entry name")?;
    Ok(String::from_utf8_lossy(&tail[..end]).into_owned())
}

pub fn parse(source: &Path) -> Result<Gpb4Archive> {
    let data = fs::read(source)?;
    if data.get(0..4) != Some(b"4bpg") {
        bail!("not a GPB4 file");
    }
    let count = u32le(&data, 0x10)? as usize;
    let table = u32le(&data, 0x14)? as usize;
    if count > 100_000 || table >= data.len() {
        bail!("invalid GPB4 table: count={count} offset=0x{table:X}");
    }
    let mut entries = Vec::with_capacity(count);
    for index in 0..count {
        let offset = table + index * 0x20;
        let name_offset = u64le(&data, offset)?;
        let data_offset = u64le(&data, offset + 8)?;
        let flags = u64le(&data, offset + 0x10)?;
        let size = u64le(&data, offset + 0x18)?;
        if data_offset > data.len() as u64 || size > data.len() as u64 - data_offset {
            bail!("GPB4 entry {index} out of range: offset=0x{data_offset:X} size=0x{size:X}");
        }
        entries.push(Gpb4Entry {
            index,
            name: c_string(&data, name_offset as usize)?,
            name_offset,
            data_offset,
            size,
            flags,
        });
    }
    Ok(Gpb4Archive {
        source: source.display().to_string(),
        file_size: data.len(),
        count,
        entries,
    })
}

fn safe_relative(name: &str) -> Result<PathBuf> {
    let trimmed = name.trim_start_matches('/');
    let mut output = PathBuf::new();
    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(part) => output.push(part),
            _ => bail!("unsafe GPB4 entry name: {name}"),
        }
    }
    if output.as_os_str().is_empty() {
        bail!("empty GPB4 entry name");
    }
    Ok(output)
}

pub fn extract(source: &Path, output: &Path) -> Result<(Gpb4Archive, usize)> {
    let archive = parse(source)?;
    let data = fs::read(source)?;
    fs::create_dir_all(output)?;
    let written = archive
        .entries
        .par_iter()
        .try_fold(
            || 0usize,
            |count, entry| -> Result<usize> {
                let relative = safe_relative(&entry.name)?;
                let destination = output.join(relative);
                if let Some(parent) = destination.parent() {
                    fs::create_dir_all(parent)?;
                }
                let start = entry.data_offset as usize;
                let end = start + entry.size as usize;
                fs::write(&destination, &data[start..end])
                    .with_context(|| format!("writing {}", destination.display()))?;
                Ok(count + 1)
            },
        )
        .try_reduce(|| 0usize, |left, right| Ok(left + right))?;
    fs::write(
        output.join("gpb4_manifest.json"),
        serde_json::to_vec_pretty(&archive)?,
    )?;
    Ok((archive, written))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_parent_traversal() {
        assert!(safe_relative("../../escape.dmsb").is_err());
        assert!(safe_relative("/dmsdata/hash/file.dmsb").is_ok());
    }
}
