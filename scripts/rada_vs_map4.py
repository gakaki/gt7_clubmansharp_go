#!/usr/bin/env python3
"""Compare RADA per-point vectors with MAP4 centreline tangents/normals."""

import json
import math
from pathlib import Path

MAP4 = Path(r"C:\gt7\gt7\_re\extracted\tracks\map4\crs_c052_l01_course_map.json")
RADA = Path(r"C:\gt7\gt7\_re\extracted\tracks\rada\crs_c052_l01_course_line.json")


def norm(v):
    l = math.sqrt(sum(c * c for c in v)) or 1.0
    return [c / l for c in v]


def main() -> None:
    m = json.loads(MAP4.read_text(encoding="utf-8"))
    r = json.loads(RADA.read_text(encoding="utf-8"))
    pts = m["points"]
    segs = []
    acc = 0.0
    for i in range(1, len(pts)):
        d = math.dist(pts[i - 1], pts[i])
        segs.append((acc, acc + d, pts[i - 1], pts[i], d))
        acc += d
    total = acc
    print(f"MAP4 points={len(pts)} length={total:.1f}")

    def at(dist):
        dist = max(0.0, min(dist, total - 0.001))
        for a, b, p0, p1, d in segs:
            if dist <= b:
                t = norm([p1[k] - p0[k] for k in range(3)])
                n = [-t[2], 0.0, t[0]]
                return t, n
        return [0, 0, 1], [-1, 0, 0]

    for si, sec in enumerate(r["sections"][:4]):
        vecs = sec["points"]
        dist = sec["distances"]
        dots_t = []
        dots_n = []
        for i in range(1, min(len(vecs), len(dist))):
            if dist[i] <= 0:
                continue
            v = norm([float(c) for c in vecs[i]])
            t, n = at(dist[i])
            dots_t.append(sum(v[k] * t[k] for k in range(3)))
            dots_n.append(abs(sum(v[k] * n[k] for k in range(3))))
        print(f"section {si}: flags={sec['flags']} n={len(dots_t)} "
              f"mean|dot tangent|={sum(abs(x) for x in dots_t)/len(dots_t):.3f} "
              f"mean|dot normal|={sum(dots_n)/len(dots_n):.3f}")


if __name__ == "__main__":
    main()
