#!/usr/bin/env python3
"""Generic binary inspector: hexdump + ASCII/UTF-16 string extraction.

usage: inspect.py <file> [--offset N] [--length N] [--strings] [--utf16]
"""

import argparse
import re
import sys
from pathlib import Path


def hexdump(data: bytes, base: int = 0, width: int = 16) -> None:
    for off in range(0, len(data), width):
        chunk = data[off:off + width]
        hexpart = " ".join(f"{b:02X}" for b in chunk).ljust(width * 3 - 1)
        asc = "".join(chr(b) if 0x20 <= b < 0x7F else "." for b in chunk)
        print(f"{base + off:08X}  {hexpart}  {asc}")


def u32s(data: bytes, count: int = 24) -> None:
    for i in range(min(count, len(data) // 4)):
        v = int.from_bytes(data[i * 4:i * 4 + 4], "little")
        print(f"  [{i:3}] 0x{v:08X}  {v}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("file")
    ap.add_argument("--offset", type=lambda s: int(s, 0), default=0)
    ap.add_argument("--length", type=lambda s: int(s, 0), default=0x100)
    ap.add_argument("--u32", action="store_true")
    ap.add_argument("--strings", action="store_true")
    ap.add_argument("--utf16", action="store_true")
    ap.add_argument("--limit", type=int, default=60)
    args = ap.parse_args()

    data = Path(args.file).read_bytes()
    print(f"{args.file}: {len(data):,} bytes (0x{len(data):X})")

    if args.strings:
        n = 0
        for m in re.finditer(rb"[\x20-\x7E]{4,}", data):
            print(f"  0x{m.start():08X}  {m.group().decode('ascii')}")
            n += 1
            if n >= args.limit:
                break
        return

    if args.utf16:
        n = 0
        i = args.offset
        start = None
        while i + 1 < len(data) and n < args.limit:
            ch = data[i] | (data[i + 1] << 8)
            printable = (0x20 <= ch < 0x7F) or (0xA0 <= ch <= 0xFFFF)
            if printable and data[i + 1] == 0 and ch < 0x7F:
                printable = True
            elif data[i + 1] == 0 and ch < 0x20:
                printable = False
            if printable:
                if start is None:
                    start = i
            else:
                if start is not None and i - start >= 6:
                    s = data[start:i].decode("utf-16le", "replace")
                    print(f"  0x{start:08X}  {s}")
                    n += 1
                start = None
            i += 2
        return

    seg = data[args.offset:args.offset + args.length]
    hexdump(seg, args.offset)
    if args.u32:
        print("\nu32le from offset:")
        u32s(seg)


if __name__ == "__main__":
    main()
