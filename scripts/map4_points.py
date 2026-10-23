#!/usr/bin/env python3
"""Parse the MAP4 point/edge tables."""

import math
import struct
import sys
from pathlib import Path


def main() -> None:
    p = Path(sys.argv[1])
    d = p.read_bytes()
    count = struct.unpack_from("<I", d, 0x80)[0]
    print(f"{p.name}: {len(d)} bytes, points={count}")
    pts = []
    for i in range(count):
        o = 0x84 + i * 0x18
        if o + 0x18 > len(d):
            break
        x, y, z = struct.unpack_from("<3f", d, o)
        a, b, c = struct.unpack_from("<IIf", d, o + 0x0C)
        pts.append((x, y, z, a, b, c))
    for q in pts[:5]:
        print("   ", tuple(round(v, 3) if isinstance(v, float) else v for v in q))
    xs = [q[0] for q in pts]
    ys = [q[1] for q in pts]
    zs = [q[2] for q in pts]
    print(f"bbox x[{min(xs):.1f},{max(xs):.1f}] y[{min(ys):.1f},{max(ys):.1f}] "
          f"z[{min(zs):.1f},{max(zs):.1f}]")
    d2 = [math.dist(pts[i][:3], pts[i - 1][:3]) for i in range(1, len(pts))]
    print(f"steps: mean={sum(d2)/len(d2):.2f} max={max(d2):.2f}")
    print(f"closure first->last: {math.dist(pts[0][:3], pts[-1][:3]):.2f}")

    eo = struct.unpack_from("<I", d, 0x44)[0]
    if eo + 4 <= len(d):
        n = struct.unpack_from("<I", d, eo)[0]
        print(f"edge table at 0x{eo:X}: n={n}")
        pairs = [struct.unpack_from("<2H", d, eo + 4 + i * 4) for i in range(min(n, 8))]
        print("   first pairs:", pairs)


if __name__ == "__main__":
    main()
