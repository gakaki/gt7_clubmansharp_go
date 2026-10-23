#!/usr/bin/env python3
"""Vectorised GT7 asset-name discovery.

Fully vectorises FNV-1a + the base-36 cache-key encoding so millions of
candidate asset paths can be tested per second against the content store.

usage:
  gtfast.py harvest --lists <file...> [--expand] [--rounds N] [--mine]
"""

import argparse
import json
import os
import re
import sys
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
from gt7store import DEFAULT_CONTENTS, Store  # noqa: E402

REPORTS = ROOT / "reports"
NAMED = REPORTS / "named.txt"

TABLE = b"P6MH85C24L91ZTIYURQ7NKJ0OSBWDAVGX3FE"
TABLE_NP = np.frombuffer(TABLE, dtype=np.uint8)
FNV_OFFSET = 0x811C9DC5
FNV_PRIME = 16777619
FLAG_BOUNDARY = 0x81BF0FFF


def encode_rel(rel: bytes) -> np.uint64:
    v = np.uint64(0)
    for i, b in enumerate(rel):
        v += np.uint64(b) * (np.uint64(128) ** np.uint64(i))
    return v


class FastStore:
    def __init__(self, contents: Path):
        store = Store(contents)
        self.store = store
        self.files = store.files
        rels = np.array([r.encode() for r in store.files], dtype="S9")
        self.targets = np.array([encode_rel(r) for r in rels], dtype=np.uint64)
        self.targets.sort()

    def probe(self, cands, chunk: int = 1_000_000):
        """Return list of (candidate, rel) for candidates that exist in the store."""
        uniq = list(dict.fromkeys(c for c in cands if c and len(c) < 200))
        found = []
        for start in range(0, len(uniq), chunk):
            part = uniq[start:start + chunk]
            found.extend(self._probe_chunk(part))
        return found

    def _probe_chunk(self, cands):
        n = len(cands)
        if n == 0:
            return []
        lens = np.array([len(c) for c in cands], dtype=np.int64)
        maxlen = int(lens.max())
        arr = np.zeros((n, maxlen), dtype=np.uint8)
        for i, c in enumerate(cands):
            b = np.frombuffer(c.encode("ascii", "ignore"), dtype=np.uint8)
            arr[i, :len(b)] = b

        h = np.full(n, FNV_OFFSET, dtype=np.uint64)
        mask32 = np.uint64(0xFFFFFFFF)
        res = np.zeros(n, dtype=np.uint64)
        need = lens - 1
        for i in range(maxlen):
            h = ((h ^ arr[:, i].astype(np.uint64)) * np.uint64(FNV_PRIME)) & mask32
            res = np.where(need == i, h, res)

        x = res
        digits = np.empty((n, 6), dtype=np.uint64)
        for i in range(6):
            digits[:, i] = (x // np.uint64(36 ** i)) % np.uint64(36)

        out = np.empty((n, 9), dtype=np.uint8)
        idx = digits.astype(np.int64)
        out[:, 0] = TABLE_NP[idx[:, 0]]
        out[:, 1] = ord("/")
        out[:, 2] = TABLE_NP[idx[:, 1]]
        out[:, 3] = ord("/")
        out[:, 4] = np.where(x >= np.uint64(FLAG_BOUNDARY), TABLE_NP[1], TABLE_NP[0])
        out[:, 5] = TABLE_NP[idx[:, 5]]
        out[:, 6] = TABLE_NP[idx[:, 4]]
        out[:, 7] = TABLE_NP[idx[:, 3]]
        out[:, 8] = TABLE_NP[idx[:, 2]]

        keys = np.zeros(n, dtype=np.uint64)
        for j in range(9):
            keys += out[:, j].astype(np.uint64) * (np.uint64(128) ** np.uint64(j))

        pos = np.searchsorted(self.targets, keys)
        pos_clamped = np.clip(pos, 0, len(self.targets) - 1)
        hit = self.targets[pos_clamped] == keys
        rels = out[hit].view("S9").ravel()
        out_list = []
        for cand, rel in zip(np.array(cands, dtype=object)[hit], rels):
            out_list.append((cand, rel.decode()))
        return out_list


# --------------------------------------------------------------------------- #
def load_named() -> dict[str, str]:
    if not NAMED.exists():
        return {}
    m = {}
    for line in NAMED.read_text(encoding="utf-8", errors="replace").splitlines():
        if "\t" in line:
            a, b = line.split("\t", 1)
            m[a] = b
    return m


def save_named(mapping: dict[str, str]) -> None:
    NAMED.parent.mkdir(parents=True, exist_ok=True)
    with open(NAMED, "w", encoding="utf-8") as fh:
        for k in sorted(mapping):
            fh.write(f"{k}\t{mapping[k]}\n")


CURATED_NAMES = [
    # geometry / models
    "body", "body_lod0", "lod0", "lod1", "lod2", "lod3", "hq", "lq", "race", "normal",
    "damage", "dmg", "wreck", "interior", "cockpit", "cockpit_lod0", "steering", "wheel",
    "wheels", "tire", "tyre", "rim", "brake", "disc", "caliper", "engine", "exhaust",
    "meter", "gauge", "needle", "seat", "mirror", "spoiler", "wing", "bonnet", "hood",
    "door", "light", "headlight", "taillight", "shadow", "chassis", "suspension", "aero",
    "driver", "pilot", "helmet", "suit", "skeleton", "rig", "animation", "ani",
    # textures / images
    "texture", "tex", "txs", "image", "img", "thumbnail", "thumb", "icon", "logo", "paint",
    "color", "colour", "livery", "decal", "stripe", "pattern", "skin", "normalmap", "spec",
    "specular", "mask", "diffuse", "albedo", "roughness", "metallic", "ao", "preview",
    "preview_0", "preview_b", "preview_c", "preview_d", "icon_0", "icon_b", "icon_c", "icon_d",
    # data
    "data", "param", "params", "parameter", "spec", "specs", "setting", "settings", "config",
    "common", "default", "index", "list", "table", "info", "definition", "define", "master",
    # track / course
    "course", "track", "circuit", "layout", "scenery", "crowd", "road", "kerb", "curb",
    "race_line", "raceline", "rline", "line", "curve", "path", "spline", "ai", "aipath",
    "checkpoint", "start", "goal", "pit", "tunnel", "bridge", "wall", "fence", "tree",
    "sky", "cloud", "sun", "light", "weather", "rain", "night", "day", "env", "environment",
    "camera", "camera_motion", "cameramotion", "replay", "replay_motion", "ghost", "motion",
    # audio / video / text
    "voice", "voiceover", "dialog", "dialogue", "movie", "cutscene", "video", "bgm", "music",
    "sound", "sfx", "se", "ui", "menu", "window", "widget", "button", "font", "text", "string",
    "label", "message", "help", "tutorial", "title", "name", "description", "comment",
    # ui / script
    "main", "boot", "root", "top", "dialog_", "cursor", "components", "residentmenu", "hud",
    "gpb", "adc", "mproject", "project", "layout", "screen", "page", "panel", "item",
]

EXTENSIONS = [
    "mdl5", "mdl4", "mdl", "txs", "txs5", "gpb", "adc", "mproject", "json", "bin", "dat",
    "sdb", "rt2", "pac", "szd", "szd3", "irf", "wav", "mp4", "ivf", "dds", "png", "jpg",
    "jpeg", "xml", "csv", "strb", "localdb", "mtl5", "shd5", "tex5", "ani5", "rig5", "col5",
    "crs", "rline", "spline", "aipath", "txt", "ini", "fpc", "gpb4", "sndz", "idl", "xls",
]


def basename_vocab(mapping) -> set[str]:
    vocab = set()
    for p in mapping:
        base = p.rsplit("/", 1)[-1]
        if not base:
            continue
        vocab.add(base)
        if "." in base:
            vocab.add(base.rsplit(".", 1)[0])
    return vocab


def dir_vocab(mapping) -> list[str]:
    dirs = set()
    for p in mapping:
        if "/" in p:
            dirs.add(p.rsplit("/", 1)[0])
    return sorted(dirs)


def generate_expand(mapping, names, exts, depth: int = 1):
    vocab = basename_vocab(mapping) | set(names)
    dirs = dir_vocab(mapping)
    cands = []
    for d in dirs:
        for n in vocab:
            cands.append(f"{d}/{n}")
    if depth >= 2:
        for d in dirs:
            for n in set(names):
                for n2 in set(names):
                    cands.append(f"{d}/{n}/{n2}")
    for p in mapping:
        if "." in p.rsplit("/", 1)[-1]:
            stem = p.rsplit(".", 1)[0]
            for e in exts:
                if not stem.endswith("." + e):
                    cands.append(f"{stem}.{e}")
    return cands


TOKEN_RE = re.compile(rb"[\x20-\x7E]{3,240}")


def mine_from_named(store: Store, mapping, min_size=0, max_size=8 << 20):
    tokens = set()
    exts = (".json", ".txt", ".csv", ".xml", ".ini", ".adc", ".gpb", ".mproject",
            ".strb", ".fpc", ".crs", ".dat", ".bin", ".localdb", ".idl", ".mp4")
    for asset, rel in mapping.items():
        if not asset.lower().endswith(exts):
            continue
        p = store.root / Path(*rel.split("/"))
        try:
            if not (min_size <= p.stat().st_size <= max_size):
                continue
            data = p.read_bytes()
        except OSError:
            continue
        for m in TOKEN_RE.finditer(data):
            s = m.group().decode("ascii", "ignore")
            for piece in re.split(r"[\s\"',;:()\[\]{}=<>|]+", s):
                piece = piece.strip()
                if len(piece) < 3 or len(piece) > 160:
                    continue
                if not re.match(r"^[A-Za-z0-9_\-\.\+/]+$", piece):
                    continue
                tokens.add(piece)
                if "/" in piece:
                    tokens.add(piece.lstrip("/"))
                    tokens.add(piece.rsplit("/", 1)[-1])
    return tokens


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--contents", default=str(DEFAULT_CONTENTS))
    ap.add_argument("--lists", nargs="*", default=[])
    ap.add_argument("--rounds", type=int, default=2)
    ap.add_argument("--expand", action="store_true")
    ap.add_argument("--mine", action="store_true")
    ap.add_argument("--depth2", action="store_true")
    ap.add_argument("--static", action="store_true", help="also use built-in name/extension vocabulary")
    args = ap.parse_args()

    fs = FastStore(Path(args.contents))
    mapping = load_named()
    print(f"start: {len(mapping):,} named files")

    base_cands = []
    for f in args.lists:
        for line in Path(f).read_text(encoding="utf-8", errors="replace").splitlines():
            s = line.strip().strip('"').strip(",")
            if s:
                base_cands.append(s)
                base_cands.append(s.lower())
    print(f"list candidates: {len(base_cands):,}")

    hits = fs.probe(base_cands)
    added = 0
    for cand, rel in hits:
        c = cand.lstrip("/")
        if c not in mapping:
            mapping[c] = rel
            added += 1
    save_named(mapping)
    print(f"after lists: +{added} -> {len(mapping):,}")

    if args.mine:
        tokens = mine_from_named(fs.store, mapping)
        print(f"mined tokens: {len(tokens):,}")
        tokens_all = []
        for t in tokens:
            tokens_all.append(t)
            tokens_all.append(t.lower())
        hits = fs.probe(tokens_all)
        added = 0
        for cand, rel in hits:
            if cand not in mapping:
                mapping[cand] = rel
                added += 1
        save_named(mapping)
        print(f"after mine: +{added} -> {len(mapping):,}")
        (REPORTS / "mined_tokens.txt").write_text("\n".join(sorted(tokens)), encoding="utf-8")

    if args.expand:
        names = CURATED_NAMES if args.static else []
        for rnd in range(args.rounds):
            cands = generate_expand(mapping, names, EXTENSIONS, 2 if args.depth2 else 1)
            cands = list(dict.fromkeys(cands))
            print(f"round {rnd+1}: probing {len(cands):,} candidates ...")
            hits = fs.probe(cands)
            added = 0
            for cand, rel in hits:
                if cand not in mapping:
                    mapping[cand] = rel
                    added += 1
            save_named(mapping)
            print(f"round {rnd+1}: +{added} -> {len(mapping):,}")
            if added == 0:
                break

    print(f"final: {len(mapping):,} named")


if __name__ == "__main__":
    main()
