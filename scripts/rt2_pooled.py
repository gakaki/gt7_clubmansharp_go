#!/usr/bin/env python3
"""Recover the .rt2 keystream per SECTION, pooling every language file.

Rationale: sections with few strings give too few samples for per-position
frequency analysis when the file is processed alone, but the keystream appears
to be tied to the section (not to the language), so all 19 localisations can be
pooled to recover it.
"""

import argparse
import sys
from collections import defaultdict
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rt2_decode import build_model, recover_keystream, load_payloads  # noqa: E402


def printable_ratio(b: bytes) -> float:
    if not b:
        return 0.0
    return sum(1 for x in b if 0x20 <= x < 0x7F) / len(b)


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("files", nargs="+")
    ap.add_argument("--target", help="file to evaluate (defaults to first)")
    ap.add_argument("--scope", default="section",
                    choices=["section", "section_len", "global"])
    args = ap.parse_args()

    model_mat = build_model()
    target = Path(args.target) if args.target else Path(args.files[0])

    per_file: dict[str, list] = {}
    for f in args.files:
        p = Path(f)
        payloads, _ = load_payloads(p)
        per_file[p.stem] = payloads
        print(f"{p.stem}: {len(payloads):,} payloads")

    def key_of(item):
        if args.scope == "section":
            return item[0]
        if args.scope == "section_len":
            return (item[0], len(item[2]))
        return "ALL"

    pooled: dict[object, list] = defaultdict(list)
    for _stem, payloads in per_file.items():
        for item in payloads:
            pooled[key_of(item)].append(item)

    keystreams: dict[object, np.ndarray] = {}
    for k, items in pooled.items():
        maxlen = max(len(x[2]) for x in items)
        if len(items) < 4 or maxlen < 4:
            continue
        ks, _c = recover_keystream(items, maxlen, model_mat)
        keystreams[k] = ks
    print(f"groups: {len(pooled):,}, recovered: {len(keystreams):,}")

    # evaluate on the target file
    tgt = per_file[target.stem]
    ratios = []
    per_section = defaultdict(list)
    for name, sid, p in tgt:
        ks = keystreams.get(key_of((name, sid, p)))
        if ks is None:
            continue
        n = min(len(p), len(ks))
        d = bytes(b ^ ks[i] for i, b in enumerate(p[:n]))
        if len(d) > 1:
            d = d[1:]  # drop leading tag byte
        r = printable_ratio(d.rstrip(b"\x00"))
        ratios.append(r)
        per_section[name].append(r)

    ratios_np = np.array(ratios)
    print(f"\n{target.stem}: {len(ratios_np):,} payloads, "
          f"mean printable ratio {ratios_np.mean():.3f}")
    hist, edges = np.histogram(ratios_np, bins=10, range=(0, 1))
    for h, e in zip(hist, edges):
        print(f"  {e:.1f}-{e+0.1:.1f}: {h:,}")
    ranked = sorted(((sum(v) / len(v), len(v), k) for k, v in per_section.items()),
                    key=lambda t: t[0])
    print("\nworst 15 sections:")
    for r, n, k in ranked[:15]:
        print(f"  {r:.3f}  n={n:5}  {k}")
    print("\nbest 5 sections:")
    for r, n, k in ranked[-5:]:
        print(f"  {r:.3f}  n={n:5}  {k}")


if __name__ == "__main__":
    main()
