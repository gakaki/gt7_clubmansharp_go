#!/usr/bin/env python3
"""Measure per-payload / per-section decode quality for .rt2 with a given keystream."""

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
    path = Path(sys.argv[1])
    payloads, _ = load_payloads(path)
    model_mat = build_model()
    ks, conf = recover_keystream(payloads, max(len(p[2]) for p in payloads), model_mat)

    per_section = defaultdict(list)
    ratios = []
    for name, sid, p in payloads:
        d = bytearray(p)
        n = min(len(p), len(ks))
        for i in range(n):
            d[i] ^= ks[i]
        del d[0:1]  # leading prefix byte is not part of the text
        r = printable_ratio(bytes(d).rstrip(b"\x00"))
        ratios.append(r)
        per_section[name].append(r)

    ratios_np = np.array(ratios)
    print(f"{path.name}: {len(ratios_np):,} payloads")
    print(f"mean printable ratio: {ratios_np.mean():.3f}")
    hist, edges = np.histogram(ratios_np, bins=10, range=(0, 1))
    for h, e in zip(hist, edges):
        print(f"  {e:.1f}-{e+0.1:.1f}: {h:,}")

    ranked = sorted(((sum(v) / len(v), len(v), k) for k, v in per_section.items()),
                    key=lambda t: -t[0])
    print("\nbest sections (mean printable ratio, n):")
    for r, n, k in ranked[:25]:
        print(f"  {r:.3f}  n={n:5}  {k}")
    print("\nworst sections:")
    for r, n, k in ranked[-25:]:
        print(f"  {r:.3f}  n={n:5}  {k}")


if __name__ == "__main__":
    main()
