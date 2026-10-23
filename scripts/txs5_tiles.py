#!/usr/bin/env python3
"""Square-tile swizzle search with full-coverage validation (R16G16)."""

import sys
from pathlib import Path

import numpy as np
from PIL import Image


def morton(n: int, bits: int) -> tuple[int, int]:
    x = y = 0
    for i in range(bits):
        x |= ((n >> (2 * i)) & 1) << i
        y |= ((n >> (2 * i + 1)) & 1) << i
    return x, y


def rebuild(raw: np.ndarray, w: int, h: int, tile: int, transposed: bool):
    px = raw.reshape(-1, 2)
    out = np.zeros((h, w, 2), dtype=np.uint16)
    n_x, n_y = w // tile, h // tile
    if n_x & (n_x - 1) or n_y & (n_y - 1):
        return None, 0
    bits = max(n_x, n_y).bit_length() - 1
    filled = 0
    for t in range(n_x * n_y):
        tx, ty = morton(t, bits)
        if tx >= n_x or ty >= n_y:
            continue
        block = px[t * tile * tile:(t + 1) * tile * tile].reshape(tile, tile, 2)
        if transposed:
            block = block.transpose(1, 0, 2)
        out[ty * tile:(ty + 1) * tile, tx * tile:(tx + 1) * tile] = block
        filled += tile * tile
    return out, filled


def metric(img: np.ndarray) -> float:
    a = img[:, :, 0].astype(np.int32)
    b = img[:, :, 1].astype(np.int32)
    # second derivative highlights block seams less than random noise
    la = np.abs(np.diff(a, 2, axis=0)).mean() + np.abs(np.diff(a, 2, axis=1)).mean()
    lb = np.abs(np.diff(b, 2, axis=0)).mean() + np.abs(np.diff(b, 2, axis=1)).mean()
    return float(la + lb)


def main() -> None:
    p = Path(sys.argv[1])
    doff = int(sys.argv[2], 0)
    w = int(sys.argv[3])
    h = int(sys.argv[4])
    d = p.read_bytes()
    raw = np.frombuffer(d[doff:doff + w * h * 4], dtype="<u2")
    base = raw.reshape(h, w, 2)
    print(f"linear: metric={metric(base):.2f}")
    results = []
    for tile in (2, 4, 8, 16, 32, 64, 128):
        if w % tile or h % tile:
            continue
        for tr in (False, True):
            img, filled = rebuild(raw, w, h, tile, tr)
            if img is None or filled != w * h:
                continue
            results.append((metric(img), tile, tr))
    results.sort()
    for r in results:
        print(f"  {r[0]:8.2f}  tile={r[1]:3} transposed={r[2]}")
    if results:
        m, tile, tr = results[0]
        img, _ = rebuild(raw, w, h, tile, tr)
        u = img.astype(np.float32) / 65535.0
        x = (u[:, :, 0] - 0.5) * 2
        y = (u[:, :, 1] - 0.5) * 2
        rgb = np.stack([
            np.clip(0.5 + x * 6, 0, 1),
            np.clip(0.5 + y * 6, 0, 1),
            np.full_like(x, 0.75),
        ], axis=2)
        out = Path(r"C:\gt7\gt7\_re\extracted\tracks\preview\txs5_best_tile.png")
        Image.fromarray((rgb * 255).astype(np.uint8), "RGB").save(out)
        print(f"best tile={tile} transposed={tr} -> {out}")


if __name__ == "__main__":
    main()
