#!/usr/bin/env python3
"""Test whether the .rt2 keystream is scoped per section, per file, or global."""

import sys
from collections import defaultdict
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rt2_decode import build_model, recover_keystream, load_payloads  # noqa: E402


def decode_group(payloads, ks):
    """Return (mean printable ratio, n) for a group decoded with ks."""
    total = 0.0
    for _n, _s, p in payloads:
        d = bytearray(p)
        n = min(len(p), len(ks))
        for i in range(n):
            d[i] ^= ks[i]
        body = bytes(d[1:]).rstrip(b"\x00") if len(d) > 1 else b""
        if body:
            total += sum(1 for x in body if 0x20 <= x < 0x7F) / len(body)
    return (total / max(len(payloads), 1), len(payloads))


def main() -> None:
    path = Path(sys.argv[1])
    payloads, _ = load_payloads(path)
    model_mat = build_model()

    groups = defaultdict(list)
    for item in payloads:
        groups[item[0]].append(item)
    print(f"{path.name}: {len(payloads):,} payloads in {len(groups)} sections")

    total_valid = 0.0
    total_n = 0
    per_section = []
    for name, items in groups.items():
        maxlen = max(len(x[2]) for x in items)
        if len(items) < 4 or maxlen < 8:
            continue
        ks, _conf = recover_keystream(items, maxlen, model_mat)
        ratio, n = decode_group(items, ks)
        per_section.append((ratio, n, maxlen, name, items, ks))
        total_valid += ratio * n
        total_n += n

    per_section.sort(key=lambda t: -t[0])
    print(f"\noverall mean printable ratio with per-section keystream: "
          f"{total_valid/max(total_n,1):.3f}")
    print("\nbest sections:")
    for rate, n, ml, name, items, ks in per_section[:10]:
        print(f"  {rate:5.3f}  n={n:5} maxlen={ml:5}  {name}")
    print("\nworst sections:")
    for rate, n, ml, name, items, ks in per_section[-10:]:
        print(f"  {rate:5.3f}  n={n:5} maxlen={ml:5}  {name}")

    # keystream cross-section agreement
    print("\ncross-section keystream comparison (first 512 offsets):")
    ref_name, ref_ks = per_section[0][3], per_section[0][5]
    for rate, n, ml, name, items, ks in per_section[1:6]:
        m = min(512, len(ks), len(ref_ks))
        same = int((ks[:m] == ref_ks[:m]).sum())
        print(f"  {name:28} vs {ref_name:28} {same}/{m} identical")


if __name__ == "__main__":
    main()
