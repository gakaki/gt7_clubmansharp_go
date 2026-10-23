#!/usr/bin/env python3
"""Try block-level permutations for a BC3 TXS5 texture."""

import struct
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def dds(payload: bytes, w: int, h: int, fourcc=b"DXT5") -> bytes:
    hdr = bytearray(128)
    hdr[0:4] = b"DDS "
    struct.pack_into("<I", hdr, 4, 124)
    struct.pack_into("<I", hdr, 8, 0x81007)
    struct.pack_into("<I", hdr, 12, h)
    struct.pack_into("<I", hdr, 16, w)
    struct.pack_into("<I", hdr, 20, w * h)
    struct.pack_into("<I", hdr, 76, 32)
    struct.pack_into("<I", hdr, 80, 0x4)
    hdr[84:88] = fourcc
    return bytes(hdr) + payload


def morton(n: int, bits: int) -> tuple[int, int]:
    x = y = 0
    for i in range(bits):
        x |= ((n >> (2 * i)) & 1) << i
        y |= ((n >> (2 * i + 1)) & 1) << i
    return x, y


def main() -> None:
    p = Path(sys.argv[1])
    doff = int(sys.argv[2], 0)
    w = int(sys.argv[3])
    h = int(sys.argv[4])
    d = p.read_bytes()
    payload = d[doff:doff + w * h]  # BC3 = 1 byte/px
    bw, bh = w // 4, h // 4
    blocks = np.frombuffer(payload, dtype=np.uint8).reshape(bh, bw, 16)
    outdir = Path(r"C:\gt7\gt7\_re\extracted\tracks\preview")

    variants = {}
    variants["identity"] = blocks
    variants["transpose"] = blocks.transpose(1, 0, 2)
    for n in (4, 8, 16, 32):
        if bh % n:
            continue
        idx = []
        for y in range(bh):
            idx.append((y // n) * n + (n - 1 - (y % n)))
        variants[f"rows_rev_{n}"] = blocks[idx, :, :]
    for tile in (4, 8, 16):
        if bw % tile or bh % tile:
            continue
        nx, ny = bw // tile, bh // tile
        if nx & (nx - 1) or ny & (ny - 1):
            continue
        bits = max(nx, ny).bit_length() - 1
        src = blocks.reshape(ny, tile, nx, tile, 16)
        out = np.zeros((ny, tile, nx, tile, 16), dtype=np.uint8)
        for t in range(nx * ny):
            tx, ty = morton(t, bits)
            if tx >= nx or ty >= ny:
                continue
            sy, sx = divmod(t, nx)
            out[ty, :, tx, :, :] = src[sy, :, sx, :, :]
        variants[f"morton_{tile}"] = out.reshape(bh, bw, 16)

    for name, b in variants.items():
        data = np.ascontiguousarray(b).tobytes()
        path = outdir / f"bc3_{name}.dds"
        path.write_bytes(dds(data, w, h))
        try:
            im = Image.open(path).convert("RGB")
            im.save(outdir / f"bc3_{name}.png")
            arr = np.asarray(im, dtype=np.int16)
            v = np.abs(np.diff(arr, axis=0)).mean()
            print(f"{name:14} vertical_gradient={v:.2f}")
        except Exception as exc:
            print(name, "decode failed", exc)


if __name__ == "__main__":
    main()
