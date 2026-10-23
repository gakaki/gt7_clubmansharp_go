#!/usr/bin/env python3
"""Extract the float32 polyline found inside a RADA file."""

import math
import struct
import sys
from pathlib import Path


def main() -> None:
    path = Path(sys.argv[1])
    start = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x11DC
    d = path.read_bytes()
    maxn = (len(d) - start) // 12
    pts = [struct.unpack_from("<3f", d, start + i * 12) for i in range(maxn)]
    print(f"{path.name}: {maxn} triples from 0x{start:X}")
    for p in pts[:8]:
        print("   ", tuple(round(v, 2) for v in p))

    dists = [math.dist(pts[i], pts[i - 1]) for i in range(1, len(pts))]
    # find where the smooth run ends
    end = 0
    for i, dist in enumerate(dists):
        if dist > 400 or not math.isfinite(dist):
            end = i
            break
    else:
        end = len(dists)
    print(f"smooth run ends at point {end} (step {dists[end] if end < len(dists) else -1:.1f})")

    # closure search within the smooth run
    best = None
    for n in range(50, end):
        close = math.dist(pts[0], pts[n])
        if best is None or close < best[1]:
            best = (n, close)
    print(f"closest-to-start: point {best[0]} distance {best[1]:.2f} m")

    n = best[0]
    loop = pts[:n + 1]
    total = sum(math.dist(loop[i], loop[i + 1]) for i in range(len(loop) - 1))
    xs = [p[0] for p in loop]
    ys = [p[1] for p in loop]
    zs = [p[2] for p in loop]
    print(f"loop points={n+1} length={total:.1f} m "
          f"x[{min(xs):.1f},{max(xs):.1f}] y[{min(ys):.1f},{max(ys):.1f}] z[{min(zs):.1f},{max(zs):.1f}]")

    out = Path(r"C:\gt7\gt7\_re\extracted\tracks")
    out.mkdir(parents=True, exist_ok=True)
    csv = out / (path.name + ".csv")
    with open(csv, "w", encoding="utf-8") as fh:
        fh.write("index,x,y,z\n")
        for i, (x, y, z) in enumerate(loop):
            fh.write(f"{i},{x:.4f},{y:.4f},{z:.4f}\n")
    print(f"wrote {csv}")


if __name__ == "__main__":
    main()
