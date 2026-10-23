#!/usr/bin/env python3
"""Survey TXS5 header fields across many files."""

import csv
import struct
from pathlib import Path

CENSUS = Path(r"C:\gt7\gt7\_re\reports\store_census_rust.csv")
CONTENTS = Path(r"C:\gt7\gt7\contents")


def u32(d, o):
    return struct.unpack_from("<I", d, o)[0]


def u16(d, o):
    return struct.unpack_from("<H", d, o)[0]


def main() -> None:
    rows = []
    with open(CENSUS, encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            if r["kind"] == "TXS5 texture-set":
                rows.append((int(r["size"]), r["rel"]))
    rows.sort()
    picks = [rows[i] for i in (0, 1, 2, 5, 10, 50, 100, 500, 1000, 2000, 3000, 4000, len(rows) - 1)]
    print(f"{'size':>12} {'A20':>8} {'A28':>8} {'A58':>8} {'B8':>8} {'C0':>8} {'C4':>6} {'C6':>6} {'W':>6} {'H':>6} {'CC':>6}  name")
    for size, rel in picks:
        d = (CONTENTS / Path(*rel.split("/"))).read_bytes()
        if len(d) < 0xE0:
            continue
        print(f"{size:>12} {u32(d,0x20):>8} {u32(d,0x28):>8} {u32(d,0x58):>8} "
              f"{u32(d,0xB8):>8} {u32(d,0xC0):>8} {u16(d,0xC4):>6X} {u16(d,0xC6):>6X} "
              f"{u16(d,0xC8):>6} {u16(d,0xCA):>6} {u32(d,0xCC):>6}  {rel}")


if __name__ == "__main__":
    main()
