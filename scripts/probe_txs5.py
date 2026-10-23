#!/usr/bin/env python3
"""Inspect Polyphony TXS5 texture-set containers."""

import csv
import struct
import sys
from pathlib import Path

CENSUS = Path(r"C:\gt7\gt7\_re\reports\store_census_rust.csv")
CONTENTS = Path(r"C:\gt7\gt7\contents")


def rows_of(kind: str):
    out = []
    with open(CENSUS, encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            if r["kind"] == kind:
                out.append((int(r["size"]), r["rel"]))
    out.sort()
    return out


def dump(path: Path, n: int = 0x60) -> None:
    d = path.read_bytes()
    print("=" * 70)
    print(f"{path.name}  size={len(d):,} (0x{len(d):X})")
    for off in range(0, min(n, len(d)), 4):
        v = struct.unpack_from("<I", d, off)[0]
        print(f"  0x{off:02X}: {v:>10}  0x{v:08X}")
    for m in [b"DDS ", b"TXS5", b"TXS", b"BC1", b"BC3", b"BC7", b"DXT", b"GNF", b"PMSK"]:
        i = d.find(m)
        if i >= 0:
            print(f"  found {m!r} at 0x{i:X}")


def main() -> None:
    rows = rows_of("TXS5 texture-set")
    print(f"{len(rows)} TXS5 files, {sum(s for s, _ in rows)/2**30:.2f} GB")
    picks = [rows[0], rows[len(rows) // 2], rows[-1]] if len(rows) >= 3 else rows
    for _s, rel in picks:
        dump(CONTENTS / Path(*rel.split("/")))


if __name__ == "__main__":
    main()
