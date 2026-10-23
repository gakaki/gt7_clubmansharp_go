#!/usr/bin/env python3
"""Inspect the MAP4 course-map container."""

import struct
import sys
from pathlib import Path


def hexdump(d, base, n):
    for o in range(base, min(base + n, len(d)), 16):
        c = d[o:o + 16]
        print(f"{o:06X}  " + " ".join(f"{b:02X}" for b in c).ljust(47)
              + "  " + "".join(chr(b) if 32 <= b < 127 else "." for b in c))


def main() -> None:
    p = Path(sys.argv[1])
    d = p.read_bytes()
    print(f"{p.name}: {len(d):,} bytes")
    print("header u32:", [struct.unpack_from("<I", d, o)[0] for o in range(0, 0x50, 4)])
    print("header f32:", [round(struct.unpack_from("<f", d, o)[0], 3) for o in range(0x20, 0x40, 4)])
    print("\n[0x80..0x180]")
    hexdump(d, 0x80, 0x100)
    for off_name, off in [("0x14", struct.unpack_from("<I", d, 0x14)[0]),
                          ("0x18", struct.unpack_from("<I", d, 0x18)[0]),
                          ("0x40", struct.unpack_from("<I", d, 0x40)[0]),
                          ("0x44", struct.unpack_from("<I", d, 0x44)[0])]:
        if 0 < off < len(d):
            print(f"\n[{off_name} -> 0x{off:X}]")
            hexdump(d, off, 0x80)


if __name__ == "__main__":
    main()
