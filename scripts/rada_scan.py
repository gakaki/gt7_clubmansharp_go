#!/usr/bin/env python3
"""Scan a RADA file for plausible float32 / int16 track-point arrays."""

import math
import struct
import sys
from pathlib import Path


def scan_f32(d: bytes, n: int = 120):
    hits = []
    for off in range(0, len(d) - n * 12, 4):
        pts = struct.unpack_from(f"<{n*3}f", d, off)
        if any(not math.isfinite(v) for v in pts):
            continue
        if max(abs(v) for v in pts) > 50000:
            continue
        dists = [
            math.dist(pts[i * 3:i * 3 + 3], pts[(i + 1) * 3:(i + 1) * 3 + 3])
            for i in range(n - 1)
        ]
        if max(dists) < 300 and min(dists) > 0.01 and sum(dists) / len(dists) < 60:
            hits.append((off, sum(dists), max(dists)))
    return hits


def main() -> None:
    path = Path(sys.argv[1])
    d = path.read_bytes()
    hits = scan_f32(d)
    print(f"{path.name}: {len(d)} bytes, f32 candidate arrays: {len(hits)}")
    for off, total, mx in hits[:20]:
        print(f"  0x{off:05X}  path_len={total:.1f}  max_step={mx:.1f}")


if __name__ == "__main__":
    main()
