#!/usr/bin/env python3
"""Test hypotheses for how on-disk cache-key names map to real asset paths."""

import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "repos" / "gt7tool"))

from gt7tool.structs import compute_cache_key
from gt7tool.crypto import fnv1a

CONTENTS = Path(r"C:\gt7\gt7\contents")


def decode_name(rel: str) -> int:
    """Invert compute_cache_key: rel is like 'A/B/P1234' -> the 32-bit value."""
    from gt7tool.constants import CACHE_NAME_LOOKUP_TABLE as T

    d0c, d1c, fname = rel.split("/")
    assert len(fname) == 5, fname
    d0 = T.index(d0c)
    d1 = T.index(d1c)
    hi = [T.index(c) for c in fname[1:]]
    assert fname[0] in (T[0], T[1]), fname
    x = d0 + d1 * 36
    for i, d in enumerate(hi):
        x += d * (36 ** (2 + (3 - i)))
    return x


def main() -> None:
    # sanity: round-trip on real paths
    sample = "0/0/P0RLS"
    x = decode_name(sample)
    print(f"decode('{sample}') = {x} = 0x{x:08X}")
    print(f"re-encode -> {compute_cache_key(x)}   (expect {sample})")
    print()

    paths = [p.strip() for p in (ROOT / "repos" / "gt7tool" / "filelist.txt").read_text(
        encoding="utf-8", errors="replace").splitlines() if p.strip()]
    print(f"testing {len(paths)} known paths from filelist.txt\n")

    hits = 0
    for p in paths:
        cands = {
            "fnv1a": fnv1a(p),
            "fnv1a_plain": fnv1a(p.lstrip("/")),
            "fnv1a_lower": fnv1a(p.lower()),
            "crc32": zlib.crc32(p.encode()) & 0xFFFFFFFF,
            "crc32_lower": zlib.crc32(p.lower().encode()) & 0xFFFFFFFF,
        }
        found = []
        for label, v in cands.items():
            rel = compute_cache_key(v)
            if (CONTENTS / Path(*rel.split("/"))).exists():
                found.append(f"{label}={rel}")
                hits += 1
        if found:
            print(f"HIT {p} -> {found}")
    print(f"\ntotal hits: {hits}")

    # also: how many distinct real files exist for a couple of sample paths
    for p in paths[:5]:
        v = fnv1a(p)
        print(f"  {p}: fnv1a=0x{v:08X} -> {compute_cache_key(v)}")


if __name__ == "__main__":
    main()
