//! Polyphony "PackedFile" containers (zlib / zstd / chunked zstd).

use anyhow::{bail, Result};
use std::io::Read;

pub const MAGIC_ZLIB: u32 = 0xFFF7_EEC5;
pub const MAGIC_ZSTD_TINY: u32 = 0xFFF7_ED85;
pub const MAGIC_ZSTD_REGULAR: u32 = 0xFFF7_972F;
pub const MAGIC_ZSTD_CHUNK: u32 = 0xFFF7_F32E;
const CHUNK_ALIGNMENT: usize = 0x10000;

fn u32le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn be6(b: &[u8], off: usize) -> usize {
    let mut v: usize = 0;
    for i in 0..6 {
        v = (v << 8) | b[off + i] as usize;
    }
    v
}

/// Return `(payload, container description)`.
pub fn unpack(data: &[u8]) -> Result<(Vec<u8>, &'static str)> {
    if data.len() < 8 {
        return Ok((data.to_vec(), "raw"));
    }
    match u32le(data, 0) {
        MAGIC_ZLIB => {
            let size = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            if size >= 0 {
                return Ok((data.to_vec(), "raw(magic-collision)"));
            }
            // PS5 stores the *uncompressed* size here and the stream itself is
            // raw DEFLATE (no zlib wrapper).  PS4 files used a zlib wrapper, so
            // try both.
            let declared = (-size) as usize;
            let mut out = Vec::new();
            let body = &data[8..];
            let zlib_ok = flate2::read::ZlibDecoder::new(body).read_to_end(&mut out).is_ok();
            if !zlib_ok {
                out.clear();
                flate2::read::DeflateDecoder::new(body).read_to_end(&mut out)?;
            }
            if declared != 0 && out.len() != declared {
                // tolerate truncated/renamed variants but keep the payload
                eprintln!("warning: deflate size {} != declared {}", out.len(), declared);
            }
            Ok((out, "packed:zlib"))
        }
        MAGIC_ZSTD_TINY => {
            let size = i32::from_le_bytes([data[4], data[5], data[6], data[7]]);
            if size >= 0 {
                return Ok((data.to_vec(), "raw(magic-collision)"));
            }
            let out = zstd::stream::decode_all(&data[8..])?;
            Ok((out, "packed:zstd-tiny"))
        }
        MAGIC_ZSTD_REGULAR => {
            if data.len() < 0x20 {
                bail!("regular packed file too small");
            }
            let _usize = be6(data, 4);
            let csize = be6(data, 0x0A);
            if csize == 0 || 0x20 + csize > data.len() {
                bail!("regular packed size {csize} exceeds input {}", data.len());
            }
            let out = zstd::stream::decode_all(&data[0x20..0x20 + csize])?;
            Ok((out, "packed:zstd-regular"))
        }
        MAGIC_ZSTD_CHUNK => {
            let mut out = Vec::new();
            let mut off = 0usize;
            let mut idx = 0usize;
            while off + 0x10 <= data.len() {
                if idx > 0 {
                    let pad = (CHUNK_ALIGNMENT - (off % CHUNK_ALIGNMENT)) % CHUNK_ALIGNMENT;
                    off += pad;
                    if off + 0x10 > data.len() {
                        break;
                    }
                }
                if u32le(data, off) != MAGIC_ZSTD_CHUNK {
                    break;
                }
                let usize_ = u32le(data, off + 4) as usize;
                let csize = u32le(data, off + 8) as usize;
                let start = off + 0x10;
                if start + csize > data.len() {
                    break;
                }
                let mut dec = zstd::stream::decode_all(&data[start..start + csize])?;
                if usize_ != 0 && dec.len() != usize_ {
                    bail!("chunk size mismatch {} != {}", dec.len(), usize_);
                }
                out.append(&mut dec);
                off = start + csize;
                idx += 1;
            }
            if out.is_empty() {
                Ok((data.to_vec(), "raw"))
            } else {
                Ok((out, "packed:zstd-chunked"))
            }
        }
        _ => Ok((data.to_vec(), "raw")),
    }
}
