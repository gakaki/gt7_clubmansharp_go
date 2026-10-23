//! Consolidates the former Python inspection, packaging and dossier helpers.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

pub struct InspectOptions {
    pub file: PathBuf,
    pub offset: u64,
    pub length: usize,
    pub u32s: bool,
    pub strings: bool,
    pub utf16: bool,
    pub limit: usize,
}

pub fn inspect(options: &InspectOptions) -> Result<()> {
    let data = fs::read(&options.file)?;
    let start = (options.offset as usize).min(data.len());
    let end = (start + options.length).min(data.len());
    let slice = &data[start..end];
    if options.u32s {
        for (index, chunk) in slice.chunks_exact(4).take(options.limit).enumerate() {
            println!(
                "0x{:08X}  {}",
                start + index * 4,
                u32::from_le_bytes(chunk.try_into()?)
            );
        }
        return Ok(());
    }
    if options.strings {
        let mut from = 0usize;
        let mut shown = 0usize;
        while from < slice.len() && shown < options.limit {
            while from < slice.len() && !(0x20..0x7f).contains(&slice[from]) {
                from += 1;
            }
            let begin = from;
            while from < slice.len() && (0x20..0x7f).contains(&slice[from]) {
                from += 1;
            }
            if from - begin >= 4 {
                println!(
                    "0x{:08X}  {}",
                    start + begin,
                    String::from_utf8_lossy(&slice[begin..from])
                );
                shown += 1;
            }
        }
        return Ok(());
    }
    if options.utf16 {
        for (index, chunk) in slice.chunks_exact(2).take(options.limit).enumerate() {
            let value = u16::from_le_bytes(chunk.try_into()?);
            if value == 0 {
                println!("0x{:08X}  0x0000", start + index * 2);
            } else if (0x20..0x7f).contains(&value) || (0x3000..=0x9fff).contains(&value) {
                println!(
                    "0x{:08X}  0x{value:04X}  {}",
                    start + index * 2,
                    char::from_u32(value as u32).unwrap_or('?')
                );
            }
        }
        return Ok(());
    }
    for (row, chunk) in slice.chunks(16).enumerate() {
        let hex = chunk
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect::<Vec<_>>()
            .join(" ");
        let ascii = chunk
            .iter()
            .map(|byte| {
                if (0x20..0x7f).contains(byte) {
                    *byte as char
                } else {
                    '.'
                }
            })
            .collect::<String>();
        println!("{:08X}  {hex:<47}  {ascii}", start + row * 16);
    }
    Ok(())
}

pub fn manifest(root: &Path, out: &Path) -> Result<()> {
    let sections = [
        ("reports/named.txt", "真实资源路径 → store 文件映射"),
        ("reports/store_census_rust.csv", "全库 95,339 文件类型普查"),
        ("reports/mdl5_index.json", "MDL5 模型索引"),
        ("reports/txs5_index.json", "TXS5 纹理描述符"),
        ("reports/txs5_index_idp0.json", "IDP0 内嵌 TXS5"),
        ("reports/gt7_menu_scripts_diss.zip", "Adhoc 菜单脚本反汇编"),
        (
            "reports/menu_disassembly_manifest.csv",
            "菜单反汇编 SHA-256",
        ),
        ("reports/jxl_index.json", "JPEG XL 盒容器"),
        ("reports/sndz_index.json", "SNDZ 音频容器"),
        ("reports/research_rust.json", "Rust DMSB + RT2 + Cafe 研究"),
        ("reports/face_atlases_rust", "角色头像图集"),
        ("site/public/avatars/extracted", "站点角色头像"),
        ("site/public/cars", "车辆缩略图"),
        ("site/public/logos", "品牌标志"),
        ("site/public/data", "站点数据集"),
        ("extracted/text_decoded", "本地化文本"),
        ("extracted/textures", "DDS 贴图"),
        ("extracted/idp0", "IDP0 载荷"),
        ("extracted/audio", "SNDZ 音频"),
        ("extracted/video", "视频"),
        ("extracted/tracks", "赛道数据"),
        ("tree", "命名资源镜像"),
        ("rust/gt7re", "Rust 工具源码"),
    ];
    let rows = sections
        .into_iter()
        .map(|(relative, description)| {
            let path = root.join(relative);
            if !path.exists() {
                return json!({"path": relative, "desc": description, "exists": false});
            }
            if path.is_dir() {
                let stats = WalkDir::new(&path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|entry| entry.file_type().is_file())
                    .fold((0u64, 0u64), |(files, bytes), entry| {
                        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                        (files + 1, bytes + size)
                    });
                json!({"path": relative, "desc": description, "exists": true, "files": stats.0, "bytes": stats.1})
            } else {
                let bytes = path.metadata().map(|m| m.len()).unwrap_or(0);
                json!({"path": relative, "desc": description, "exists": true, "files": 1, "bytes": bytes})
            }
        })
        .collect::<Vec<_>>();
    fs::write(out, serde_json::to_vec_pretty(&rows)?)?;
    println!("manifest: {} sections -> {}", rows.len(), out.display());
    Ok(())
}

pub fn package_disassembly(source: &Path, archive: &Path, manifest: &Path) -> Result<()> {
    let mut files = WalkDir::new(source)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().and_then(|v| v.to_str()) == Some("diss"))
        .map(|entry| entry.into_path())
        .collect::<Vec<_>>();
    files.sort();
    if files.is_empty() {
        bail!("no .diss files under {}", source.display());
    }
    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let output = File::create(archive)?;
    let mut zip = ZipWriter::new(output);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));
    let mut manifest_writer = csv::Writer::from_path(manifest)?;
    manifest_writer.write_record(["path", "bytes", "lines", "sha256"])?;
    let mut total_bytes = 0usize;
    let mut total_lines = 0usize;
    for path in &files {
        let data = fs::read(path)?;
        let relative = path
            .strip_prefix(source)?
            .to_string_lossy()
            .replace('\\', "/");
        let name = format!("gt7/menu/{relative}");
        zip.start_file(&name, options)?;
        zip.write_all(&data)?;
        let hash = format!("{:x}", Sha256::digest(&data));
        let lines = data.iter().filter(|byte| **byte == b'\n').count();
        manifest_writer.write_record([name, data.len().to_string(), lines.to_string(), hash])?;
        total_bytes += data.len();
        total_lines += lines;
    }
    zip.finish()?;
    manifest_writer.flush()?;
    println!(
        "packaged {} files, {} bytes, {} lines -> {}",
        files.len(),
        total_bytes,
        total_lines,
        archive.display()
    );
    Ok(())
}

pub fn dossier(cars_json: &Path, query: &str, out: &Path) -> Result<()> {
    let payload: Value = serde_json::from_slice(&fs::read(cars_json)?)?;
    let query_lower = query.to_lowercase();
    let mut markdown = format!("# {query} — 游戏内中英文文本档案\n\n");
    let mut hits = 0usize;
    for car in payload
        .get("cars")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let name = car.get("nameEn").and_then(Value::as_str).unwrap_or("");
        let name_zh = car.get("nameZh").and_then(Value::as_str).unwrap_or("");
        let corpus = format!("{name} {name_zh}");
        if !corpus.to_lowercase().contains(&query_lower) {
            continue;
        }
        hits += 1;
        markdown.push_str(&format!(
            "## {} ({})\n\n",
            name,
            car.get("id").and_then(Value::as_str).unwrap_or("")
        ));
        for (category, text) in car
            .get("texts")
            .and_then(Value::as_object)
            .into_iter()
            .flatten()
        {
            markdown.push_str(&format!("### {category}\n\n"));
            for part in std::iter::once(text).chain(
                text.get("variants")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten(),
            ) {
                markdown.push_str("**EN**\n\n");
                markdown.push_str(part.get("en").and_then(Value::as_str).unwrap_or(""));
                markdown.push_str("\n\n**中文**\n\n");
                markdown.push_str(part.get("zh").and_then(Value::as_str).unwrap_or(""));
                markdown.push_str("\n\n");
            }
        }
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(out, markdown)?;
    println!("dossier: {hits} cars -> {}", out.display());
    Ok(())
}
