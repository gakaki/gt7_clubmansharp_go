#!/usr/bin/env python3
"""Search for the RADA vector semantics by testing integration variants."""

import math
import struct
import sys
from pathlib import Path


def read_section(d, s):
    count = struct.unpack_from("<I", d, s)[0]
    va = struct.unpack_from("<3f", d, s + 0x0C)
    vb = struct.unpack_from("<3f", d, s + 0x18)
    off = struct.unpack_from("<Q", d, s + 0x28)[0]
    base = struct.unpack_from("<Q", d, s + 0x30)[0] + 4
    vecs = [struct.unpack_from("<3h", d, off + i * 6) for i in range(count)]
    dist = [struct.unpack_from("<f", d, base + i * 4)[0] for i in range(count)]
    return count, va, vb, vecs, dist


def norm(v):
    L = math.sqrt(sum(c * c for c in v)) or 1.0
    return (v[0] / L, v[1] / L, v[2] / L)


def integrate_single(vecs, dist, ds_mode, t0, p0):
    t = norm(t0)
    p = list(p0)
    for i in range(1, len(vecs)):
        ds = (dist[i] - dist[i - 1]) if ds_mode == "dist" else 1.0
        if ds <= 0 or ds > 100:
            ds = 1.0
        v = norm(vecs[i - 1])
        p[0] += v[0] * ds
        p[1] += v[1] * ds
        p[2] += v[2] * ds
    return p


def integrate_double(vecs, dist, ds_mode, t0, p0, k):
    t = norm(t0)
    p = list(p0)
    for i in range(1, len(vecs)):
        ds = (dist[i] - dist[i - 1]) if ds_mode == "dist" else 1.0
        if ds <= 0 or ds > 100:
            ds = 1.0
        v = norm(vecs[i - 1])
        t = norm((t[0] + v[0] * k, t[1] + v[1] * k, t[2] + v[2] * k))
        p[0] += t[0] * ds
        p[1] += t[1] * ds
        p[2] += t[2] * ds
    return p


def main() -> None:
    d = Path(sys.argv[1]).read_bytes()
    s = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0xD0
    count, va, vb, vecs, dist = read_section(d, s)
    print(f"count={count} vecA={va} vecB={vb} dist=[{dist[0]:.1f}..{dist[-1]:.1f}]")

    for ds_mode in ("dist", "const"):
        p = integrate_single(vecs, dist, ds_mode, va, vb)
        print(f"single {ds_mode:5}: end={tuple(round(x,1) for x in p)} "
              f"closure={math.dist(p, vb):.1f}")
        for k in (1e-4, 5e-4, 1e-3, 5e-3, 1e-2, 5e-2, 1e-1, 5e-1, 1.0):
            p = integrate_double(vecs, dist, ds_mode, va, vb, k)
            print(f"  double {ds_mode:5} k={k:<7}: closure={math.dist(p, vb):9.1f} "
                  f"end={tuple(round(x,1) for x in p)}")


if __name__ == "__main__":
    main()
