#!/usr/bin/env python3
"""Probe the Polyphony .rt2 localisation container layout."""

import sys
from pathlib import Path


def main() -> None:
    path = Path(sys.argv[1])
    data = path.read_bytes()
    print(f"{path.name}: {len(data):,} bytes")

    magic = data[0:4]
    print(f"magic raw={magic!r} reversed={magic[::-1]!r}")
    print(f"u32@0x04 = {int.from_bytes(data[4:8],'little')}")
    print(f"u32@0x08 = {int.from_bytes(data[8:12],'little')}")
    print(f"u32@0x0C = {int.from_bytes(data[12:16],'little')}")

    # scan for utf-16le ascii runs
    runs = []
    i = 0
    start = None
    while i + 1 < len(data):
        b0, b1 = data[i], data[i + 1]
        if 0x20 <= b0 < 0x7F and b1 == 0:
            if start is None:
                start = i
        else:
            if start is not None and i - start >= 8:
                runs.append((start, (i - start) // 2))
            start = None
        i += 2
    print(f"utf16le-ascii runs: {len(runs)} (first 20: {runs[:20]})")

    # 16-bit value histogram in the 'text' region
    from collections import Counter
    for region_start in (0x1210, 0x1400, 0x2000):
        if region_start + 0x400 > len(data):
            continue
        c = Counter(int.from_bytes(data[region_start + k:region_start + k + 2], "little")
                    for k in range(0, 0x400, 2))
        top = c.most_common(8)
        print(f"region 0x{region_start:X} top u16: {[(hex(v), n) for v, n in top]}")

    # dump the table as candidate (offset,u64) pairs after the header
    print("\ntable entries (u64 pairs) from 0x20:")
    for i in range(12):
        off = 0x20 + i * 16
        a = int.from_bytes(data[off:off + 8], "little")
        b = int.from_bytes(data[off + 8:off + 16], "little")
        print(f"  [{i:3}] @0x{off:04X}  a=0x{a:08X} ({a})  b=0x{b:08X} ({b})")


if __name__ == "__main__":
    main()
