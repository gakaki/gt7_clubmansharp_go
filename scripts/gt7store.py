#!/usr/bin/env python3
"""GT7 PS5 content-store toolkit.

The PS5 GT7 install ships a content store at:
    <dump>/contents/<d0>/<d1>/<XXXXX>
where the relative path is `compute_cache_key(fnv1a(asset_path))`.
Payloads are already decrypted; some are wrapped in Polyphony "PackedFile"
containers (zlib / zstd / chunked zstd) that must be decompressed.

Subcommands
-----------
  census                      classify every store file by magic, write report
  resolve <path> [path ...]   show which store file backs an asset path
  harvest <wordlist...>       brute-force candidate asset paths against the store
  extract <path> -o DIR       extract one asset path (decompressing packed files)
  extract-hits <hits.txt>     extract every path listed in a harvest output
"""

import argparse
import csv
import json
import os
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "repos" / "gt7tool"))

from gt7tool.constants import (  # noqa: E402
    CACHE_NAME_LOOKUP_TABLE,
    PACKED_FILE_ZLIB_MAGIC,
    PACKED_FILE_ZSTD_CHUNK_MAGIC,
    PACKED_FILE_ZSTD_REGULAR_MAGIC,
    PACKED_FILE_ZSTD_TINY_MAGIC,
    CHUNK_ALIGNMENT,
)
from gt7tool.crypto import fnv1a  # noqa: E402
from gt7tool.structs import compute_cache_key  # noqa: E402

DEFAULT_CONTENTS = Path(r"C:\gt7\gt7\contents")


# --------------------------------------------------------------------------- #
# packed-file containers
# --------------------------------------------------------------------------- #
def _zstd_decompress(data: bytes, expected: int = -1) -> bytes:
    import pyzstd

    out = pyzstd.decompress(data)
    if expected >= 0 and len(out) != expected:
        raise ValueError(f"zstd size mismatch: {len(out)} != {expected}")
    return out


def unpack_container(data: bytes) -> tuple[bytes, str]:
    """Return (payload, container_description). Handles Polyphony PackedFile wrappers."""
    if len(data) < 8:
        return data, "raw"
    magic = int.from_bytes(data[0:4], "little")

    if magic == PACKED_FILE_ZLIB_MAGIC:
        size = int.from_bytes(data[4:8], "little", signed=True)
        if size > 0:
            return data, "raw(magic-collision)"
        comp = -size
        return zlib.decompress(data[8:8 + comp]), "packed:zlib"

    if magic == PACKED_FILE_ZSTD_TINY_MAGIC:
        size = int.from_bytes(data[4:8], "little", signed=True)
        if size > 0:
            return data, "raw(magic-collision)"
        return _zstd_decompress(data[8:], -size), "packed:zstd-tiny"

    if magic == PACKED_FILE_ZSTD_REGULAR_MAGIC:
        usize = int.from_bytes(data[4:10], "big")
        csize = int.from_bytes(data[10:16], "big")
        if csize == 0 or 0x20 + csize > len(data) + 4096:
            # regular container, verify with actual length
            avail = len(data) - 0x20
            if not (0 < csize <= avail):
                raise ValueError(f"bad regular packed sizes {usize=} {csize=} {len(data)=}")
        return _zstd_decompress(data[0x20:0x20 + csize], usize), "packed:zstd-regular"

    # chunked zstd (KIND_FRAG): sequence of 0x10000-aligned chunks
    if len(data) > 0x10 and int.from_bytes(data[0x00:0x04], "little") == PACKED_FILE_ZSTD_CHUNK_MAGIC:
        out = bytearray()
        off = 0
        idx = 0
        while off + 0x10 <= len(data):
            if idx > 0:
                pad = (-off) % CHUNK_ALIGNMENT
                off += pad
                if off + 0x10 > len(data):
                    break
            if int.from_bytes(data[off:off + 4], "little") != PACKED_FILE_ZSTD_CHUNK_MAGIC:
                break
            usize = int.from_bytes(data[off + 4:off + 8], "little")
            csize = int.from_bytes(data[off + 8:off + 12], "little")
            body = data[off + 0x10:off + 0x10 + csize]
            if len(body) != csize:
                break
            out += _zstd_decompress(body, usize)
            off += 0x10 + csize
            idx += 1
        if out:
            return bytes(out), "packed:zstd-chunked"

    return data, "raw"


# --------------------------------------------------------------------------- #
# store
# --------------------------------------------------------------------------- #
class Store:
    def __init__(self, contents: Path = DEFAULT_CONTENTS):
        self.root = Path(contents)
        self.files: set[str] = set()
        for entry in os.scandir(self.root):
            if entry.is_dir() and entry.name in CACHE_NAME_LOOKUP_TABLE_SET:
                for sub in os.scandir(entry.path):
                    if sub.is_dir():
                        for f in os.scandir(sub.path):
                            self.files.add(f"{entry.name}/{sub.name}/{f.name}")

    def rel_for_path(self, asset_path: str) -> str:
        return compute_cache_key(fnv1a(asset_path))

    def file_for_path(self, asset_path: str) -> Path | None:
        variants = [asset_path, asset_path.lstrip("/")]
        variants += [v.lower() for v in list(variants)]
        seen = set()
        for v in variants:
            if v in seen:
                continue
            seen.add(v)
            rel = self.rel_for_path(v)
            if rel in self.files:
                return self.root / Path(*rel.split("/"))
        return None

    def rel_exists(self, rel: str) -> bool:
        return rel in self.files

    def read(self, asset_path: str, raw: bool = False) -> tuple[bytes, str, Path] | None:
        p = self.file_for_path(asset_path)
        if p is None:
            return None
        data = p.read_bytes()
        if raw:
            return data, "raw", p
        payload, kind = unpack_container(data)
        return payload, kind, p


CACHE_NAME_LOOKUP_TABLE_SET = set(CACHE_NAME_LOOKUP_TABLE)


# --------------------------------------------------------------------------- #
# magic classification
# --------------------------------------------------------------------------- #
KNOWN_MAGICS = {
    b"MDL5": "model-3d (MDL5)",
    b"MDL4": "model-3d (MDL4)",
    b"MDL3": "model-3d (MDL3)",
    b"PACL": "PAC archive (PACL)",
    b"PACA": "PAC archive (PACA)",
    b"IDP0": "IDP0 container",
    b"SCPS": "SCPS container",
    b"\x89PNG": "png image",
    b"\xff\xd8\xff": "jpeg image",
    b"RIFF": "riff (wav/webp)",
    b"OggS": "ogg audio",
    b"DDS ": "dds texture",
    b"TXS\x00": "TXS texture-set",
    b"PSP\x00": "PSP blob",
    b"\x28\xb5\x2f\xfd": "zstd stream",
    b"PK\x03\x04": "zip",
    b"{\r\n": "json/text",
    b"{\n": "json/text",
    b"[\n": "json/text",
    b"<pre": "text",
    b"GTFS": "GTFS",
    b"SDB\x00": "spec db",
    b"\x00\x00\x00\x00": "zero-filled",
}

STRUCT_MAGICS = {
    PACKED_FILE_ZLIB_MAGIC: "packed:zlib",
    PACKED_FILE_ZSTD_TINY_MAGIC: "packed:zstd-tiny",
    PACKED_FILE_ZSTD_REGULAR_MAGIC: "packed:zstd-regular",
    PACKED_FILE_ZSTD_CHUNK_MAGIC: "packed:zstd-chunk",
}


def classify(head: bytes) -> str:
    if len(head) >= 4:
        m = int.from_bytes(head[0:4], "little")
        if m in STRUCT_MAGICS:
            return STRUCT_MAGICS[m]
    for magic, name in KNOWN_MAGICS.items():
        if head.startswith(magic):
            return name
    if head[:1] in (b"{", b"[") or all(
            0x09 <= b < 0x7F or b in (0x0A, 0x0D) for b in head[:32]):
        return "text/unknown"
    if head[:4] == b"\x00\x00\x00\x00":
        return "zero-filled"
    return "unknown:" + head[:4].hex().upper()


# --------------------------------------------------------------------------- #
# commands
# --------------------------------------------------------------------------- #
def cmd_census(args) -> None:
    store = Store(args.contents)
    rows = []
    counts: dict[str, int] = {}
    bytes_by: dict[str, int] = {}
    for rel in sorted(store.files):
        p = store.root / Path(*rel.split("/"))
        size = p.stat().st_size
        with open(p, "rb") as fh:
            head = fh.read(32)
        c = classify(head)
        counts[c] = counts.get(c, 0) + 1
        bytes_by[c] = bytes_by.get(c, 0) + size
        rows.append({"rel": rel, "size": size, "kind": c, "head": head.hex().upper()})

    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", newline="", encoding="utf-8") as fh:
        w = csv.DictWriter(fh, fieldnames=["rel", "size", "kind", "head"])
        w.writeheader()
        w.writerows(rows)

    print(f"files: {len(rows)}  report: {out}\n")
    print(f"{'kind':38} {'count':>8} {'size(GB)':>10}")
    for c, n in sorted(counts.items(), key=lambda kv: -kv[1])[:60]:
        print(f"{c:38} {n:>8} {bytes_by[c]/2**30:>10.2f}")


def cmd_resolve(args) -> None:
    store = Store(args.contents)
    for p in args.paths:
        f = store.file_for_path(p)
        print(f"{p} -> {f.relative_to(store.root) if f else 'NOT FOUND'}")


def _iter_candidates(paths: list[Path], from_store: bool, store: Store):
    seen = set()
    for wl in paths:
        for line in wl.read_text(encoding="utf-8", errors="replace").splitlines():
            s = line.strip().strip('"').strip(",")
            if not s or s.startswith("#"):
                continue
            if s not in seen:
                seen.add(s)
                yield s
    if from_store:
        for rel in sorted(store.files):
            yield "\x00store\x00" + rel


def cmd_harvest(args) -> None:
    store = Store(args.contents)
    hits = []
    tried = 0
    for cand in _iter_candidates([Path(p) for p in args.wordlists], args.from_store, store):
        tried += 1
        variants = {cand, cand.lstrip("/"), cand.replace("\\", "/")}
        variants |= {v.lower() for v in list(variants)}
        for v in variants:
            rel = store.rel_for_path(v)
            if store.rel_exists(rel):
                hits.append((v, rel))
                break
    print(f"candidates tried: {tried}")
    print(f"hits: {len(hits)} ({100.0*len(hits)/max(tried,1):.1f}%)")
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with open(out, "w", encoding="utf-8") as fh:
        for p, rel in hits:
            fh.write(f"{p}\t{rel}\n")
    print(f"mapping written: {out}")
    for p, rel in hits[:15]:
        print(f"  {p}  ->  {rel}")


def cmd_extract(args) -> None:
    store = Store(args.contents)
    outdir = Path(args.out)
    got, miss, failed = 0, 0, 0
    for asset in args.paths:
        try:
            r = store.read(asset, raw=args.raw)
        except Exception as exc:  # container lied about its magic; fall back to raw
            try:
                r = store.read(asset, raw=True)
                print(f"WARN  {asset}: container decode failed ({exc}); wrote raw bytes")
            except Exception as exc2:
                print(f"FAIL  {asset}: {exc2}")
                failed += 1
                continue
        if r is None:
            print(f"MISS  {asset}")
            miss += 1
            continue
        payload, kind, src = r
        dest = outdir / asset.lstrip("/")
        dest.parent.mkdir(parents=True, exist_ok=True)
        try:
            dest.write_bytes(payload)
        except OSError:
            dest = outdir / (asset.lstrip("/").replace("/", "__"))
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(payload)
        print(f"OK    {asset}  [{kind}] {len(payload):,} bytes")
        got += 1
    print(f"\nextracted {got}, missing {miss}, failed {failed} -> {outdir}")


def cmd_extract_hits(args) -> None:
    entries = []
    for line in Path(args.hits).read_text(encoding="utf-8", errors="replace").splitlines():
        if "\t" in line:
            entries.append(line.split("\t", 1)[0])
    ns = argparse.Namespace(contents=args.contents, out=args.out, paths=entries, raw=args.raw)
    cmd_extract(ns)


def main() -> None:
    ap = argparse.ArgumentParser(description="GT7 PS5 content store toolkit")
    ap.add_argument("--contents", default=str(DEFAULT_CONTENTS))
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("census")
    p.add_argument("--out", default=str(ROOT / "reports" / "store_census.csv"))
    p.set_defaults(func=cmd_census)

    p = sub.add_parser("resolve")
    p.add_argument("paths", nargs="+")
    p.set_defaults(func=cmd_resolve)

    p = sub.add_parser("harvest")
    p.add_argument("wordlists", nargs="+")
    p.add_argument("--from-store", action="store_true")
    p.add_argument("--out", default=str(ROOT / "reports" / "harvest_hits.txt"))
    p.set_defaults(func=cmd_harvest)

    p = sub.add_parser("extract")
    p.add_argument("paths", nargs="+")
    p.add_argument("-o", "--out", required=True)
    p.add_argument("--raw", action="store_true")
    p.set_defaults(func=cmd_extract)

    p = sub.add_parser("extract-hits")
    p.add_argument("hits")
    p.add_argument("-o", "--out", required=True)
    p.add_argument("--raw", action="store_true")
    p.set_defaults(func=cmd_extract_hits)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
