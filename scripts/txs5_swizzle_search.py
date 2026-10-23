#!/usr/bin/env python3
"""Quantitatively search TXS5 block layouts by neighbour smoothness."""

import struct
import sys
from pathlib import Path

import numpy as np


def morton_decode(n: int) -> tuple[int, int]:
    x = y = 0
    for i in range(16):
        x |= ((n >> (2 * i)) & 1) << i
        y |= ((n >> (2 * i + 1)) & 1) << i
    return x, y


def tiles_to_image(raw: np.ndarray, w: int, h: int, tw: int, th: int, morton: bool) -> np.ndarray:
    px = raw.reshape(-1, 4)
    out = np.zeros((h, w, 4), dtype=np.uint8)
    tiles_x, tiles_y = w // tw, h // th
    tbytes = tw * th
    for t in range(tiles_x * tiles_y):
        if morton:
            tx, ty = morton_decode(t)
        else:
            tx, ty = t % tiles_x, t // tiles_x
        if tx >= tiles_x or ty >= tiles_y:
            continue
        block = px[t * tbytes:(t + 1) * tbytes].reshape(th, tw, 4)
        out[ty * th:(ty + 1) * th, tx * tw:(tx + 1) * tw] = block
    return out


def smoothness(img: np.ndarray) -> float:
    a = img[:, :, 0].astype(np.int16)
    b = img[:, :, 1].astype(np.int16)
    dx = np.abs(np.diff(a, axis=1)).mean() + np.abs(np.diff(b, axis=1)).mean()
    dy = np.abs(np.diff(a, axis=0)).mean() + np.abs(np.diff(b, axis=0)).mean()
    return float(dx + dy)


def main() -> None:
    p = Path(sys.argv[1])
    doff = int(sys.argv[2], 0)
    w = int(sys.argv[3])
    h = int(sys.argv[4])
    d = p.read_bytes()
    raw = np.frombuffer(d[doff:doff + w * h * 4], dtype=np.uint8)
    print(f"{p.name}: {w}x{h}")
    base = raw.reshape(-1, 4).reshape(h, w, 4)
    print(f"  linear       smoothness={smoothness(base):.2f}")
    best = (smoothness(base), "linear")
    for tw, th in ((4, 4), (8, 8), (16, 16), (32, 32), (64, 64), (8, 16), (16, 8)):
        if w % tw or h % th:
            continue
        img = tiles_to_image(raw, w, h, tw, th, True)
        s = smoothness(img)
        print(f"  morton {tw:2}x{th:<2}  smoothness={s:.2f}")
        if s < best[0]:
            best = (s, f"morton{tw}x{th}")
    print(f"best: {best[1]} ({best[0]:.2f})")


if __name__ == "__main__":
    main()
