#!/usr/bin/env python3
"""Probe GT7 course_line (RADA) and car info (CAR5) structures."""

import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from gt7store import Store  # noqa: E402

REPORT = Path(r"C:\gt7\gt7\_re\reports\named.txt")


def named(group_substr: str, limit: int = 3):
    out = []
    for line in REPORT.read_text(encoding="utf-8", errors="replace").splitlines():
        if "\t" not in line:
            continue
        a, r = line.split("\t", 1)
        if group_substr in a:
            out.append((a, r))
            if len(out) >= limit:
                break
    return out


def dump(path: Path, n: int = 0x60) -> bytes:
    d = path.read_bytes()
    print(f"  {path.name}: {len(d):,} bytes")
    for off in range(0, min(n, len(d)), 16):
        chunk = d[off:off + 16]
        hx = " ".join(f"{b:02X}" for b in chunk).ljust(47)
        asc = "".join(chr(b) if 0x20 <= b < 0x7F else "." for b in chunk)
        print(f"    {off:04X}  {hx}  {asc}")
    return d


def try_points(d: bytes, header: int) -> None:
    body = d[header:]
    if len(body) < 12:
        return
    n = len(body) // 12
    pts = [struct.unpack_from("<3f", body, i * 12) for i in range(min(n, 4000))]
    xs = [p[0] for p in pts]
    ys = [p[1] for p in pts]
    zs = [p[2] for p in pts]
    print(f"    as float3 x{n}: x[{min(xs):.1f},{max(xs):.1f}] "
          f"y[{min(ys):.1f},{max(ys):.1f}] z[{min(zs):.1f},{max(zs):.1f}]")
    print(f"    first points: {[(round(a,2),round(b,2),round(c,2)) for a,b,c in pts[:4]]}")
    print(f"    remainder bytes: {len(body) % 12}")


def main() -> None:
    store = Store()
    print("== course_line ==")
    for a, rel in named("course_line", 2):
        print(f"  {a}")
        d = dump(store.root / Path(*rel.split("/")))
        for h in (0, 4, 8, 0xC, 0x10, 0x20, 0x40, 0x80):
            try_points(d, h)

    print("\n== car info (CAR5) ==")
    for a, rel in named("/info", 2):
        print(f"  {a}")
        d = dump(store.root / Path(*rel.split("/")))
        body = d[8:]
        n = len(body) // 4
        vals = struct.unpack_from(f"<{min(n,24)}I", body, 0)
        print(f"    u32s: {vals}")
        fs = struct.unpack_from(f"<{min(n,16)}f", body, 0)
        print(f"    floats: {[round(x,3) for x in fs]}")


if __name__ == "__main__":
    main()
