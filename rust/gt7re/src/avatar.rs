//! GT7 Cafe face-atlas extraction.
//!
//! Retail PS5 face files are `IDP0 -> TXS5`, using format `0x0700`, flags
//! `0x0109`, and 16-byte BC7 data.  The low five bits of the flags are GNM
//! swizzle mode 9: 64 KiB Standard with a within-block GFX10 address equation.

use anyhow::{bail, Context, Result};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use rayon::prelude::*;
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::idp0;
use crate::store::Store;

#[derive(Clone, Copy, Debug)]
pub struct AvatarSpec {
    pub key: &'static str,
    pub display: &'static str,
    pub character_id: &'static str,
    pub asset_path: &'static str,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarRecord {
    pub key: String,
    pub display: String,
    pub character_id: String,
    pub asset_path: String,
    pub source_path: String,
    pub source_size: u64,
    pub data_offset: u64,
    pub data_size: u32,
    pub image_format: u16,
    pub flags: u16,
    pub swizzle_mode: u16,
    pub width: u16,
    pub height: u16,
    pub atlas: String,
    pub portrait: String,
}

#[derive(Clone, Debug)]
pub struct RgbaImage {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl RgbaImage {
    fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width as usize * height as usize * 4],
        }
    }

    fn put_pixel(&mut self, x: u32, y: u32, rgba: [u8; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let offset = ((y as usize * self.width as usize) + x as usize) * 4;
        self.pixels[offset..offset + 4].copy_from_slice(&rgba);
    }

    fn crop(&self, x: u32, y: u32, width: u32, height: u32) -> Self {
        let mut output = Self::new(width, height);
        for row in 0..height {
            let source_y = y + row;
            if source_y >= self.height {
                break;
            }
            for column in 0..width {
                let source_x = x + column;
                if source_x >= self.width {
                    break;
                }
                let source = ((source_y as usize * self.width as usize) + source_x as usize) * 4;
                let target = ((row as usize * width as usize) + column as usize) * 4;
                output.pixels[target..target + 4].copy_from_slice(&self.pixels[source..source + 4]);
            }
        }
        output
    }

    fn resize_bilinear(&self, width: u32, height: u32) -> Self {
        let mut output = Self::new(width, height);
        if self.width == 0 || self.height == 0 || width == 0 || height == 0 {
            return output;
        }
        let sample = |x: usize, y: usize, channel: usize| -> f32 {
            let offset = (y * self.width as usize + x) * 4 + channel;
            self.pixels[offset] as f32
        };
        for y in 0..height as usize {
            let source_y = ((y as f32 + 0.5) * self.height as f32 / height as f32 - 0.5)
                .clamp(0.0, self.height.saturating_sub(1) as f32);
            let y0 = source_y.floor() as usize;
            let y1 = (y0 + 1).min(self.height as usize - 1);
            let fy = source_y - y0 as f32;
            for x in 0..width as usize {
                let source_x = ((x as f32 + 0.5) * self.width as f32 / width as f32 - 0.5)
                    .clamp(0.0, self.width.saturating_sub(1) as f32);
                let x0 = source_x.floor() as usize;
                let x1 = (x0 + 1).min(self.width as usize - 1);
                let fx = source_x - x0 as f32;
                let mut rgba = [0u8; 4];
                for (channel, output_channel) in rgba.iter_mut().enumerate() {
                    let top = sample(x0, y0, channel) * (1.0 - fx) + sample(x1, y0, channel) * fx;
                    let bottom =
                        sample(x0, y1, channel) * (1.0 - fx) + sample(x1, y1, channel) * fx;
                    *output_channel =
                        (top * (1.0 - fy) + bottom * fy).round().clamp(0.0, 255.0) as u8;
                }
                output.put_pixel(x as u32, y as u32, rgba);
            }
        }
        output
    }
}

/// All Cafe members plus the three text/dealer owners whose portraits use the
/// same retail face-atlas layout.
pub const AVATARS: &[AvatarSpec] = &[
    AvatarSpec {
        key: "luca",
        display: "Luca",
        character_id: "CafeMaster",
        asset_path: "piece/4k/face/message_cafemaster_01.img",
    },
    AvatarSpec {
        key: "sarah",
        display: "Sarah Eaton",
        character_id: "Sarah",
        asset_path: "piece/4k/face/message_sarah_01.img",
    },
    AvatarSpec {
        key: "stella",
        display: "Stella",
        character_id: "CarLover",
        asset_path: "piece/4k/face/message_carlover_01.img",
    },
    AvatarSpec {
        key: "chris",
        display: "Chris",
        character_id: "ExpertOldMan1",
        asset_path: "piece/4k/face/message_expertoldman1_01.img",
    },
    AvatarSpec {
        key: "jeremy",
        display: "Jeremy",
        character_id: "RaceEnthusiast",
        asset_path: "piece/4k/face/message_raceenthusiast_01.img",
    },
    AvatarSpec {
        key: "hideo-kodama",
        display: "Hideo Kodama",
        character_id: "CarDesignerProf1Kodama",
        asset_path: "piece/4k/face/message_hideokodama_01.img",
    },
    AvatarSpec {
        key: "shiro-nakamura",
        display: "Shiro Nakamura",
        character_id: "CarDesignerProf2Nakamura",
        asset_path: "piece/4k/face/message_shironakamura_01.img",
    },
    AvatarSpec {
        key: "kunihisa-ito",
        display: "Kunihisa Ito",
        character_id: "CarDesignerProf3Ito",
        asset_path: "piece/4k/face/message_kunihisaito_01.img",
    },
    AvatarSpec {
        key: "hikonori-kagurazaka",
        display: "Hikonori Kagurazaka",
        character_id: "CarDesignerProf4Harada",
        asset_path: "piece/4k/face/message_cardesignerprof4harada_01.img",
    },
    AvatarSpec {
        key: "tom-matano",
        display: "Tom Matano",
        character_id: "CarDesignerProf5Tom",
        asset_path: "piece/4k/face/message_tommatano_01.img",
    },
    AvatarSpec {
        key: "fabio-filippini",
        display: "Fabio Filippini",
        character_id: "CarDesignerProf6Fabio",
        asset_path: "piece/4k/face/message_fabiofilippini_01.img",
    },
    AvatarSpec {
        key: "ed-welburn",
        display: "Ed Welburn",
        character_id: "CarDesignerProf7Ed",
        asset_path: "piece/4k/face/message_edwelburn_01.img",
    },
    AvatarSpec {
        key: "freeman-thomas",
        display: "Freeman Thomas",
        character_id: "CarDesignerProf8Freeman",
        asset_path: "piece/4k/face/message_freemanthomas_01.img",
    },
    AvatarSpec {
        key: "ikuo-maeda",
        display: "Ikuo Maeda",
        character_id: "CarDesignerProf9Maeda",
        asset_path: "piece/4k/face/message_ikuomaeda_01.img",
    },
    AvatarSpec {
        key: "kim-jones",
        display: "Kim Jones",
        character_id: "DiorKimJones",
        asset_path: "piece/4k/face/message_diorkimjones_01.img",
    },
    AvatarSpec {
        key: "hagerty",
        display: "McKeel Hagerty",
        character_id: "Hagerty",
        asset_path: "piece/4k/face/message_hagerty_01.img",
    },
    AvatarSpec {
        key: "used-car-owner",
        display: "Used Car Dealer",
        character_id: "UsedCarOwner",
        asset_path: "piece/4k/face/message_usedcar_01.img",
    },
    AvatarSpec {
        key: "brand-central-advisor",
        display: "Brand Central Advisor",
        character_id: "BrandCentralOwner",
        asset_path: "piece/4k/face/message_brandcentraladvisor_01.img",
    },
    AvatarSpec {
        key: "legend-car-owner",
        display: "Legend Car Owner",
        character_id: "LegendCarOwner",
        asset_path: "piece/4k/face/message_legendcarowner_01.img",
    },
];

fn u16le(data: &[u8], offset: usize) -> Result<u16> {
    let bytes = data.get(offset..offset + 2).context("u16 out of range")?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
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

fn parity(value: u32) -> u32 {
    value.count_ones() & 1
}

fn pattern_axis(value: usize, pattern: &[(u32, u32)], use_x: bool) -> usize {
    let mut result = 0usize;
    for (bit, &(x_mask, y_mask)) in pattern.iter().enumerate() {
        let mask = if use_x { x_mask } else { y_mask };
        result |= (parity(value as u32 & mask) as usize) << bit;
    }
    result
}

/// Exact GFX10_SW_64K_S address equation from AMD AddrLib.
fn gnm_mode9_bc7(raw: &[u8], width: u16, height: u16) -> Result<Vec<u8>> {
    let x = |bit: u32| (1u32 << bit, 0u32);
    let y = |bit: u32| (0u32, 1u32 << bit);
    let pattern = [
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        y(0),
        y(1),
        x(0),
        x(1),
        y(2),
        x(2),
        y(3),
        x(3),
        y(4),
        x(4),
        y(5),
        x(5),
    ];
    let blocks_w = (width as usize).div_ceil(4);
    let blocks_h = (height as usize).div_ceil(4);
    let blocks_per_row = blocks_w.div_ceil(64);
    let x_terms: Vec<usize> = (0..64).map(|v| pattern_axis(v, &pattern, true)).collect();
    let y_terms: Vec<usize> = (0..64).map(|v| pattern_axis(v, &pattern, false)).collect();
    let mut linear = vec![0u8; blocks_w * blocks_h * 16];
    for by in 0..blocks_h {
        for bx in 0..blocks_w {
            let block_index = (by / 64) * blocks_per_row + bx / 64;
            let source = block_index * 65_536 + (x_terms[bx % 64] ^ y_terms[by % 64]);
            if let Some(block) = raw.get(source..source + 16) {
                let target = (by * blocks_w + bx) * 16;
                linear[target..target + 16].copy_from_slice(block);
            }
        }
    }
    Ok(linear)
}

fn decode_bc7(raw: &[u8], width: u16, height: u16) -> Result<RgbaImage> {
    let width = width as usize;
    let height = height as usize;
    let blocks_w = width.div_ceil(4);
    let blocks_h = height.div_ceil(4);
    let mut image = RgbaImage::new(width as u32, height as u32);
    for by in 0..blocks_h {
        for bx in 0..blocks_w {
            let source = (by * blocks_w + bx) * 16;
            let block = &raw[source..source + 16];
            let mut rgba = [0u8; 64];
            bcdec_rs::bc7(block, &mut rgba, 16);
            for py in 0..4 {
                for px in 0..4 {
                    let x = bx * 4 + px;
                    let y = by * 4 + py;
                    if x < width && y < height {
                        let src = (py * 4 + px) * 4;
                        image.put_pixel(
                            x as u32,
                            y as u32,
                            [rgba[src], rgba[src + 1], rgba[src + 2], rgba[src + 3]],
                        );
                    }
                }
            }
        }
    }
    Ok(image)
}

fn png_chunk(output: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]) {
    output.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    output.extend_from_slice(kind);
    output.extend_from_slice(payload);
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(kind);
    hasher.update(payload);
    output.extend_from_slice(&hasher.finalize().to_be_bytes());
}

fn save_png(image: &RgbaImage, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fn paeth(left: u8, above: u8, upper_left: u8) -> u8 {
        let p = left as i32 + above as i32 - upper_left as i32;
        let pa = (p - left as i32).abs();
        let pb = (p - above as i32).abs();
        let pc = (p - upper_left as i32).abs();
        if pa <= pb && pa <= pc {
            left
        } else if pb <= pc {
            above
        } else {
            upper_left
        }
    }

    let row_bytes = image.width as usize * 4;
    let mut scanlines = Vec::with_capacity(image.height as usize * (1 + row_bytes));
    let mut previous = vec![0u8; row_bytes];
    for row in image.pixels.chunks_exact(row_bytes) {
        scanlines.push(4);
        for index in 0..row_bytes {
            let left = if index >= 4 { row[index - 4] } else { 0 };
            let above = previous[index];
            let upper_left = if index >= 4 { previous[index - 4] } else { 0 };
            scanlines.push(row[index].wrapping_sub(paeth(left, above, upper_left)));
        }
        previous.copy_from_slice(row);
    }
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&scanlines)?;
    let compressed = encoder.finish()?;

    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&image.width.to_be_bytes());
    ihdr.extend_from_slice(&image.height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    png_chunk(&mut png, b"IHDR", &ihdr);
    png_chunk(&mut png, b"IDAT", &compressed);
    png_chunk(&mut png, b"IEND", &[]);
    fs::write(path, png).with_context(|| format!("writing {}", path.display()))
}

fn extract_one(
    store: &Store,
    spec: AvatarSpec,
    atlas_dir: &Path,
    portrait_dir: &Path,
) -> Result<AvatarRecord> {
    let source_path = store
        .file_for(spec.asset_path)
        .with_context(|| format!("{} not present in content store", spec.asset_path))?;
    let data = fs::read(&source_path)?;
    let header = idp0::parse_header(&data)?;
    let payload_start = header.payload_offset as usize;
    let payload_end = payload_start + header.payload_size as usize;
    let payload = data
        .get(payload_start..payload_end)
        .context("IDP0 payload out of range")?;
    if payload.get(0..4) != Some(b"TXS5") {
        bail!("{}: missing TXS5 payload", spec.asset_path);
    }
    let table = u32le(payload, 0x28)? as usize;
    let data_offset = u64le(payload, table)?;
    let data_size = u32le(payload, table + 8)?;
    let format_flags = u32le(payload, table + 0x0c)?;
    let image_format = (format_flags & 0xffff) as u16;
    let flags = (format_flags >> 16) as u16;
    let swizzle_mode = flags & 0x1f;
    let width = u16le(payload, table + 0x10)?;
    let height = u16le(payload, table + 0x12)?;
    if image_format != 0x0700 {
        bail!(
            "{}: unsupported TXS5 format 0x{image_format:04X}",
            spec.asset_path
        );
    }
    if swizzle_mode != 9 {
        bail!(
            "{}: unsupported GNM swizzle mode {swizzle_mode}",
            spec.asset_path
        );
    }
    let raw_start = data_offset as usize;
    let raw_end = raw_start + data_size as usize;
    let raw = payload
        .get(raw_start..raw_end)
        .context("TXS5 texture bytes out of range")?;
    let blocks = gnm_mode9_bc7(raw, width, height)?;
    let atlas = decode_bc7(&blocks, width, height)?;

    let atlas_path = atlas_dir.join(format!("{}.png", spec.key));
    save_png(&atlas, &atlas_path)?;

    // The primary 4x5 atlas cell contains a postage-stamp face image with the
    // useful silhouette inside this 64x64 inset.  Keeping the inset avoids the
    // neighbouring expression strip that otherwise cuts across the hair.
    let portrait = atlas.crop(84, 0, 64, 64).resize_bilinear(192, 192);
    let portrait_path = portrait_dir.join(format!("{}.png", spec.key));
    save_png(&portrait, &portrait_path)?;

    Ok(AvatarRecord {
        key: spec.key.to_string(),
        display: spec.display.to_string(),
        character_id: spec.character_id.to_string(),
        asset_path: spec.asset_path.to_string(),
        source_path: source_path.display().to_string(),
        source_size: data.len() as u64,
        data_offset,
        data_size,
        image_format,
        flags,
        swizzle_mode,
        width,
        height,
        atlas: atlas_path.display().to_string(),
        portrait: portrait_path.display().to_string(),
    })
}

/// Extract every real Cafe portrait in parallel.
pub fn extract_all(
    store: &Store,
    atlas_dir: &Path,
    portrait_dir: &Path,
) -> Result<Vec<AvatarRecord>> {
    fs::create_dir_all(atlas_dir)?;
    fs::create_dir_all(portrait_dir)?;
    let mut records: Vec<AvatarRecord> = AVATARS
        .par_iter()
        .map(|spec| extract_one(store, *spec, atlas_dir, portrait_dir))
        .collect::<Result<Vec<_>>>()?;
    records.sort_by_key(|record| record.display.to_lowercase());
    Ok(records)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarManifest {
    pub generated_from: &'static str,
    pub count: usize,
    pub entries: Vec<AvatarRecord>,
}

pub fn write_manifest(records: &[AvatarRecord], path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let manifest = AvatarManifest {
        generated_from: "GT7 PS5 IDP0/TXS5 + GNM mode 9 + BC7 (native Rust)",
        count: records.len(),
        entries: records.to_vec(),
    };
    fs::write(path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

pub fn default_atlas_dir() -> PathBuf {
    PathBuf::from(r"C:\gt7\gt7\_re\reports\face_atlases_rust")
}

pub fn default_portrait_dir() -> PathBuf {
    PathBuf::from(r"C:\gt7\gt7\_re\site\public\avatars\extracted")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode9_pattern_is_bijective_for_one_macro_block() {
        let raw = vec![0u8; 65_536];
        let linear = gnm_mode9_bc7(&raw, 256, 256).unwrap();
        assert_eq!(linear.len(), 65_536);
    }

    #[test]
    fn cafe_cast_has_no_duplicate_ids() {
        let mut keys: Vec<_> = AVATARS.iter().map(|spec| spec.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), AVATARS.len());
    }
}
