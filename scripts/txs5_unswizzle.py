#!/usr/bin/env python3
"""Try common PS4/PS5 block swizzles on a TXS5 RGBA8 texture."""

import struct
import sys
from pathlib import Path

from PIL import Image


def morton_decode(n: int) -> tuple[int, int]:
    x = y = 0
    for i in range(16):
        x |= ((n >> (2 * i)) & 1) << i
        y |= ((n >> (2 * i + 1)) & 1) << i
    return x, y


def unswizzle_tiles(raw: bytes, w: int, h: int, tw: int, th: int) -> bytes:
    out = bytearray(len(raw))
    tiles_x = w // tw
    tiles_y = h // th
    tile_bytes = tw * th * 4
    for t in range(tiles_x * tiles_y):
        tx, ty = morton_decode(t)
        if tx >= tiles_x or ty >= tiles_y:
            continue
        src = t * tile_bytes
        for row in range(th):
            dst = ((ty * th + row) * w + tx * tw) * 4
            out[dst:dst + tw * 4] = raw[src + row * tw * 4:src + (row + 1) * tw * 4]
    return bytes(out)


def main() -> None:
    p = Path(sys.argv[1])
    doff = int(sys.argv[2], 0)
    w = int(sys.argv[3])
    h = int(sys.argv[4])
    d = p.read_bytes()
    raw = d[doff:doff + w * h * 4]
    outdir = Path(r"C:\gt7\gt7\_re\extracted\tracks\preview")
    for tw, th in ((8, 8), (16, 16), (32, 32)):
        data = unswizzle_tiles(raw, w, h, tw, th)
        img = Image.frombytes("RGBA", (w, h), data)
        out = outdir / f"txs5_unsw_{tw}x{th}.png"
        img.save(out)
        print("saved", out)


if __name__ == "__main__":
    main()
