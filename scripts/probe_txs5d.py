#!/usr/bin/env python3
"""Align TXS5 texture descriptors across files."""

import csv
import struct
from pathlib import Path

CENSUS = Path(r"C:\gt7\gt7\_re\reports\store_census_rust.csv")
CONTENTS = Path(r"C:\gt7\gt7\contents")


def main() -> None:
    rows = []
    with open(CENSUS, encoding="utf-8") as fh:
        for r in csv.DictReader(fh):
            if r["kind"] == "TXS5 texture-set":
                rows.append((int(r["size"]), r["rel"]))
    rows.sort()
    picks = [rows[i] for i in (5, 40, 200, 800, 1500, 2500, 3500, len(rows) - 1)]
    print(f"{'size':>11} {'off':>6} {'data':>8} {'dsize':>10} {'fmt':>6} {'mip':>6} {'w':>6} {'h':>6} {'cnt':>4} {'bpp':>7}")
    for size, rel in picks:
        d = (CONTENTS / Path(*rel.split("/"))).read_bytes()
        off = struct.unpack_from("<I", d, 0x28)[0]
        if off + 0x20 > len(d):
            continue
        data_off = struct.unpack_from("<Q", d, off)[0]
        dsize = struct.unpack_from("<I", d, off + 8)[0]
        fmt = struct.unpack_from("<H", d, off + 0x0C)[0]
        mip = struct.unpack_from("<H", d, off + 0x0E)[0]
        w = struct.unpack_from("<H", d, off + 0x10)[0]
        h = struct.unpack_from("<H", d, off + 0x12)[0]
        cnt = struct.unpack_from("<I", d, off + 0x14)[0]
        bpp = dsize / (w * h) if w and h else 0
        print(f"{size:>11} {off:>6X} {data_off:>8X} {dsize:>10} {fmt:>6X} {mip:>6X} "
              f"{w:>6} {h:>6} {cnt:>4} {bpp:>7.3f}")


if __name__ == "__main__":
    main()
