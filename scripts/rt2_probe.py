#!/usr/bin/env python3
"""Parse the Polyphony .rt2 localisation container and characterise its payloads."""

import sys
import zlib
import lzma
import bz2
from collections import Counter
from pathlib import Path


def parse(path: Path):
    data = path.read_bytes()
    count = int.from_bytes(data[4:8], "little")
    sections = []
    for i in range(count):
        off = 0x20 + i * 0x18
        name_off = int.from_bytes(data[off:off + 8], "little")
        n_entries = int.from_bytes(data[off + 8:off + 16], "little")
        entries_off = int.from_bytes(data[off + 16:off + 24], "little")
        name = data[name_off:data.find(b"\x00", name_off)].decode("ascii", "replace")
        entries = []
        for j in range(n_entries):
            e = entries_off + j * 0x18
            sid = int.from_bytes(data[e:e + 4], "little")
            f1 = int.from_bytes(data[e + 4:e + 6], "little")
            size = int.from_bytes(data[e + 6:e + 8], "little")
            b_off = int.from_bytes(data[e + 8:e + 16], "little")
            p_off = int.from_bytes(data[e + 16:e + 24], "little")
            entries.append((sid, f1, size, b_off, p_off))
        sections.append((name, n_entries, entries_off, entries))
    return data, count, sections


def try_decompress(payload: bytes):
    out = []
    try:
        d = zlib.decompress(payload)
        out.append(("zlib", len(d), d[:60]))
    except Exception:
        pass
    try:
        d = zlib.decompressobj(-15).decompress(payload)
        out.append(("raw-deflate", len(d), d[:60]))
    except Exception:
        pass
    try:
        d = lzma.decompress(payload)
        out.append(("lzma", len(d), d[:60]))
    except Exception:
        pass
    try:
        d = bz2.decompress(payload)
        out.append(("bz2", len(d), d[:60]))
    except Exception:
        pass
    try:
        import pyzstd
        d = pyzstd.decompress(payload)
        out.append(("zstd", len(d), d[:60]))
    except Exception:
        pass
    return out


def main() -> None:
    path = Path(sys.argv[1])
    data, count, sections = parse(path)
    print(f"{path.name}: {len(data):,} bytes, sections={count}")
    print("\nfirst 12 sections:")
    for name, n, eoff, entries in sections[:12]:
        print(f"  {name:34} entries={n:5} entries_off=0x{eoff:X} "
              f"blob0=0x{entries[0][3]:X} size0={entries[0][2]}")

    sec = sections[0]
    print(f"\nblob headers of section '{sec[0]}':")
    heads = Counter()
    for sid, f1, size, b_off, p_off in sec[3][:20]:
        hdr = data[b_off:b_off + 16]
        heads[hdr] += 1
        print(f"  id=0x{sid:08X} f1={f1} size={size:5} hdr={hdr.hex()} "
              f"payload[:8]={data[p_off:p_off+8].hex()}")

    # global header constant survey
    const12 = Counter()
    total = 0
    for name, n, eoff, entries in sections:
        for sid, f1, size, b_off, p_off in entries:
            const12[data[b_off:b_off + 12]] += 1
            total += 1
    print(f"\ntotal entries: {total}")
    print(f"distinct 12-byte blob headers: {len(const12)}")
    for h, c in const12.most_common(6):
        print(f"  {h.hex()}  x{c}")

    sid, f1, size, b_off, p_off = sec[3][0]
    payload = data[p_off:p_off + size]
    print(f"\npayload sample ({size} bytes) decompression attempts:")
    res = try_decompress(payload)
    if not res:
        print("  none succeeded")
    for name, n, head in res:
        print(f"  {name}: {n} bytes, head={head!r}")


if __name__ == "__main__":
    main()
