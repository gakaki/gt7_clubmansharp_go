#!/usr/bin/env python3
"""Broader swizzle search using the real R16G16 channels."""

import sys
from pathlib import Path

import numpy as np


def morton(n: int) -> tuple[int, int]:
    x = y = 0
    for i in range(16):
        x |= ((n >> (2 * i)) & 1) << i
        y |= ((n >> (2 * i + 1)) & 1) << i
    return x, y


def bitrev(n: int, bits: int) -> int:
    r = 0
    for i in range(bits):
        r = (r << 1) | ((n >> i) & 1)
    return r


def rebuild(raw: np.ndarray, w: int, h: int, tw: int, th: int, order: str, xor_row: bool):
    px = raw.reshape(-1, 2)
    out = np.zeros((h, w, 2), dtype=np.uint16)
    tx_n, ty_n = w // tw, h // th
    for t in range(tx_n * ty_n):
        if order == "morton":
            tx, ty = morton(t)
        elif order == "bitrev":
            bits_x = (tx_n - 1).bit_length()
            tx = bitrev(t % tx_n, bits_x)
            ty = t // tx_n
        else:
            tx, ty = t % tx_n, t // tx_n
        if xor_row:
            tx ^= (ty & 1)
        if tx >= tx_n or ty >= ty_n:
            continue
        block = px[t * tw * th:(t + 1) * tw * th].reshape(th, tw, 2)
        out[ty * th:(ty + 1) * th, tx * tw:(tx + 1) * tw] = block
    return out


def metric(img: np.ndarray) -> float:
    a = img[:, :, 0].astype(np.int32)
    b = img[:, :, 1].astype(np.int32)
    return float(
        np.abs(np.diff(a, axis=1)).mean() + np.abs(np.diff(b, axis=1)).mean()
        + np.abs(np.diff(a, axis=0)).mean() + np.abs(np.diff(b, axis=0)).mean()
    )


def main() -> None:
    p = Path(sys.argv[1])
    doff = int(sys.argv[2], 0)
    w = int(sys.argv[3])
    h = int(sys.argv[4])
    d = p.read_bytes()
    raw = np.frombuffer(d[doff:doff + w * h * 4], dtype="<u2")
    base = raw.reshape(h, w, 2)
    print(f"linear: {metric(base):.1f}")
    results = []
    for tw in (4, 8, 16, 32, 64):
        for th in (4, 8, 16, 32, 64):
            if w % tw or h % th:
                continue
            for order in ("morton", "bitrev", "raster"):
                for xr in (False, True):
                    img = rebuild(raw, w, h, tw, th, order, xr)
                    results.append((metric(img), tw, th, order, xr))
    results.sort()
    for r in results[:10]:
        print(f"  {r[0]:8.1f}  {r[1]}x{r[2]} {r[3]} xor_row={r[4]}")


if __name__ == "__main__":
    main()
