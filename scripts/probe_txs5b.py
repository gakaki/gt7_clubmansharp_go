#!/usr/bin/env python3
"""Detailed TXS5 header inspection."""

import re
import struct
import sys
from pathlib import Path


def hexdump(d: bytes, base: int = 0, width: int = 16) -> None:
    for off in range(0, len(d), width):
        chunk = d[off:off + width]
        hx = " ".join(f"{b:02X}" for b in chunk).ljust(width * 3 - 1)
        asc = "".join(chr(b) if 0x20 <= b < 0x7F else "." for b in chunk)
        print(f"{base+off:06X}  {hx}  {asc}")


def main() -> None:
    path = Path(sys.argv[1])
    d = path.read_bytes()
    print(f"{path.name}: {len(d):,} bytes")
    print("\n--- 0x60..0x260 ---")
    hexdump(d[0x60:0x260], 0x60)
    print("\n--- ascii strings in first 0x1000 ---")
    for m in re.finditer(rb"[\x20-\x7E]{4,}", d[:0x1000]):
        print(f"  0x{m.start():04X}  {m.group().decode('ascii')}")
    print("\n--- u16 array 0x60..0x140 ---")
    for i in range(0x60, 0x140, 2):
        v = struct.unpack_from("<H", d, i)[0]
        print(f"  0x{i:04X}: {v:6}  0x{v:04X}")
    print("\n--- first 64 bytes after 0x1000 ---")
    hexdump(d[0x1000:0x1040], 0x1000)


if __name__ == "__main__":
    main()
