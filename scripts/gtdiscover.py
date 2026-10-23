#!/usr/bin/env python3
"""Asset-path discovery engine for the GT7 PS5 content store.

Any asset path can be tested cheaply: file = contents/compute_cache_key(fnv1a(path)).
This tool grows the known-name set from three directions:

  probe    test raw candidate paths (from files or stdin)
  mine     pull path-like tokens out of already-named game files
  expand   derive sibling/parent candidates from known paths
  stats    report coverage of the store
"""

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))

from gt7store import Store  # noqa: E402

REPORTS = ROOT / "reports"
NAMED = REPORTS / "named.txt"          # canonical "asset_path<TAB>rel" map

TOKEN_RE = re.compile(rb"[\x20-\x7E]{4,200}")
PATHLIKE_RE = re.compile(r"^[A-Za-z0-9_\-\.\+/]{3,180}$")


def load_named() -> dict[str, str]:
    if not NAMED.exists():
        return {}
    out = {}
    for line in NAMED.read_text(encoding="utf-8", errors="replace").splitlines():
        if "\t" in line:
            a, b = line.split("\t", 1)
            out[a] = b
    return out


def save_named(mapping: dict[str, str]) -> None:
    NAMED.parent.mkdir(parents=True, exist_ok=True)
    with open(NAMED, "w", encoding="utf-8") as fh:
        for k in sorted(mapping):
            fh.write(f"{k}\t{mapping[k]}\n")


def probe_paths(store: Store, cands, mapping: dict[str, str], quiet: bool = False) -> int:
    hits = 0
    for cand in cands:
        c = cand.strip().strip('"').strip(",").strip("'")
        if not c or len(c) > 200 or c in mapping:
            continue
        for v in {c, c.lstrip("/")}:
            rel = store.rel_for_path(v)
            if store.rel_exists(rel):
                if v not in mapping:
                    mapping[v] = rel
                    hits += 1
                    if not quiet:
                        print(f"HIT  {v}  ->  {rel}")
                break
    return hits


def extract_strings(data: bytes, minlen: int = 4):
    for m in TOKEN_RE.finditer(data):
        s = m.group().decode("ascii", "ignore")
        yield s


COMMON_NAMES = """body body_lod0 lod0 lod1 lod2 hq lq race normal damage
engine engine_sound exhaust tyre tire wheel wheels rim brake disc caliper
meter meter_lod0 interior cockpit seat steering gauge needle
shadow ambient occlusion ao normalmap specular mask
texture tex txs image img thumbnail thumb icon logo
color colour paint livery decal stripe pattern skin
camera camera_motion replay replay_motion
model mdl mdl5 mdl4 mesh geometry vertices
data params parameter spec specs setting settings config
race_line raceline line curve path spline ai aipath
voice voiceover dialog dialogue movie cutscene video
ui menu window widget button font text string label
light lighting sun sky cloud weather rain night day
track course circuit layout scenery crowd
sound sfx bgm music se engine
collision col hit point
preview preview_0 preview_b preview_c preview_d
icon_0 icon_b icon_c icon_d
000 001 002 003 004 005 006 007 008 009
0 1 2 3 4 5 6 7 8 9
00 01 02 03 04 05 06 07 08 09
""".split()

EXTENSIONS = """mdl5 mdl4 mdl txs gpb adc mproject json bin dat sdb rt2 rtext
pac szd szd3 irf wav mp4 ivf dds png jpg jpeg tga bmp exr hdr xml txt csv
strb localdb mtl5 shd5 tex5 ani5 rig5 col5 csv5""".split()


def expand_known(mapping: dict[str, str]) -> list[str]:
    """Generate candidates related to already-known paths."""
    cands: list[str] = []
    paths = list(mapping.keys())
    dirs = set()
    for p in paths:
        d = p.rsplit("/", 1)[0] if "/" in p else ""
        dirs.add(d)

    # sibling basenames in known directories
    for d in dirs:
        for n in COMMON_NAMES:
            cands.append(f"{d}/{n}" if d else n)
            for e in EXTENSIONS:
                cands.append(f"{d}/{n}.{e}" if d else f"{n}.{e}")

    # other extensions of known paths (strip existing extension)
    for p in paths:
        base = p.rsplit(".", 1)[0] if "." in p.rsplit("/", 1)[-1] else p
        for e in EXTENSIONS:
            cands.append(f"{base}.{e}")
        for n in COMMON_NAMES:
            cands.append(f"{base}/{n}")
            for e in EXTENSIONS:
                cands.append(f"{base}/{n}.{e}")
    return cands


def cmd_probe(args) -> None:
    store = Store(args.contents)
    mapping = load_named()
    before = len(mapping)
    cands = []
    for f in args.files:
        if f == "-":
            cands.extend(sys.stdin.read().splitlines())
        else:
            cands.extend(Path(f).read_text(encoding="utf-8", errors="replace").splitlines())
    hits = probe_paths(store, cands, mapping, quiet=args.quiet)
    save_named(mapping)
    print(f"tried {len(cands)} candidates, new hits {hits}, total named {len(mapping)} (+{len(mapping)-before})")


def cmd_mine(args) -> None:
    store = Store(args.contents)
    mapping = load_named()
    before = len(mapping)
    tokens: set[str] = set()
    scanned = 0
    for asset in args.paths:
        r = store.read(asset)
        if r is None:
            continue
        payload, kind, _ = r
        scanned += 1
        for s in extract_strings(payload):
            for piece in s.split():
                piece = piece.strip('"\'`,;:()[]{}=<>|')
                if PATHLIKE_RE.match(piece) and ("/" in piece or "." in piece):
                    tokens.add(piece)
                if "/" in piece:
                    tokens.add(piece.split("/")[-1])
                    tokens.add(piece)
            if "/" in s:
                tokens.add(s)
    cands = sorted(tokens)
    print(f"scanned {scanned} files -> {len(cands)} candidate tokens")
    hits = probe_paths(store, cands, mapping, quiet=False)
    save_named(mapping)
    out = REPORTS / "mined_tokens.txt"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(cands), encoding="utf-8")
    print(f"new hits {hits}, total named {len(mapping)} (+{len(mapping)-before}); tokens -> {out}")


def cmd_expand(args) -> None:
    store = Store(args.contents)
    mapping = load_named()
    before = len(mapping)
    for round_no in range(args.rounds):
        cands = expand_known(mapping)
        hits = probe_paths(store, cands, mapping, quiet=True)
        save_named(mapping)
        print(f"round {round_no+1}: tried {len(set(cands))}, hits {hits}, total {len(mapping)}")
        if hits == 0:
            break


def cmd_stats(args) -> None:
    store = Store(args.contents)
    mapping = load_named()
    named_rel = set(mapping.values())
    total = len(store.files)
    size_total = 0
    size_named = 0
    for rel in store.files:
        sz = (store.root / Path(*rel.split("/"))).stat().st_size
        size_total += sz
        if rel in named_rel:
            size_named += sz
    print(f"store files : {total:,}")
    print(f"named files : {len(named_rel):,} ({100.0*len(named_rel)/total:.1f}%)")
    print(f"store size  : {size_total/2**30:.2f} GB")
    print(f"named size  : {size_named/2**30:.2f} GB ({100.0*size_named/size_total:.1f}%)")
    dist: dict[str, int] = {}
    for p in mapping:
        dist[p.split("/")[0]] = dist.get(p.split("/")[0], 0) + 1
    print("\ntop-level groups:")
    for k, v in sorted(dist.items(), key=lambda kv: -kv[1])[:40]:
        print(f"  {k:24} {v}")


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--contents", default=None)
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("probe")
    p.add_argument("files", nargs="+")
    p.add_argument("--quiet", action="store_true")
    p.set_defaults(func=cmd_probe)

    p = sub.add_parser("mine")
    p.add_argument("paths", nargs="+")
    p.set_defaults(func=cmd_mine)

    p = sub.add_parser("expand")
    p.add_argument("--rounds", type=int, default=3)
    p.set_defaults(func=cmd_expand)

    p = sub.add_parser("stats")
    p.set_defaults(func=cmd_stats)

    args = ap.parse_args()
    if args.contents is None:
        from gt7store import DEFAULT_CONTENTS
        args.contents = str(DEFAULT_CONTENTS)
    args.func(args)


if __name__ == "__main__":
    main()
