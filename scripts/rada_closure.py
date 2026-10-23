#!/usr/bin/env python3
"""Evaluate RADA reconstruction closure across all exported sections."""

import json
import math
from pathlib import Path

ROOT = Path(r"C:\gt7\gt7\_re\extracted\tracks\rada")


def norm(v):
    L = math.sqrt(sum(c * c for c in v)) or 1.0
    return [c / L for c in v]


def main() -> None:
    rows = []
    for p in ROOT.glob("*.json"):
        d = json.loads(p.read_text(encoding="utf-8"))
        for s in d["sections"]:
            pts = s["points"]
            dist = s["distances"]
            if len(pts) < 10 or len(dist) < 10:
                continue
            total = max(dist)
            sx = sy = sz = 0.0
            n = min(len(pts), len(dist))
            for i in range(1, n):
                ds = dist[i] - dist[i - 1]
                if ds <= 0 or ds > 100:
                    continue
                v = norm(pts[i - 1])
                sx += v[0] * ds
                sy += v[1] * ds
                sz += v[2] * ds
            closure = math.sqrt(sx * sx + sy * sy + sz * sz)
            rows.append((closure / total if total else 1.0, closure, total, d["name"], s["flags"]))

    rows.sort()
    good = [r for r in rows if r[0] < 0.02]
    print(f"sections={len(rows)}  closure<2%: {len(good)}  "
          f"median={rows[len(rows)//2][0]*100:.1f}%")
    for r in rows[:8]:
        print(f"  {r[0]*100:5.2f}%  closure={r[1]:7.1f} len={r[2]:8.1f} {r[3]} flags={r[4]}")
    print("  ...")
    for r in rows[-5:]:
        print(f"  {r[0]*100:5.2f}%  closure={r[1]:7.1f} len={r[2]:8.1f} {r[3]} flags={r[4]}")


if __name__ == "__main__":
    main()
