#!/usr/bin/env python3
"""Try several keystream scopes for .rt2 payloads and report which decodes best."""

import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rt2_decode import build_model, recover_keystream, load_payloads  # noqa: E402


def valid_rate(items, ks):
    ok = 0
    for _n, _s, p in items:
        d = bytearray(p)
        n = min(len(p), len(ks))
        for i in range(n):
            d[i] ^= ks[i]
        try:
            bytes(d).rstrip(b"\x00").decode("utf-8")
            ok += 1
        except UnicodeDecodeError:
            pass
    return ok, len(items)


def eval_scope(path, keyfunc, label, min_group=4):
    payloads, _ = load_payloads(path)
    model_mat = build_model()
    groups = defaultdict(list)
    for item in payloads:
        groups[keyfunc(item)].append(item)
    ok = tot = 0
    sizes = []
    for _k, items in sorted(groups.items(), key=lambda kv: -len(kv[1])):
        if len(items) < min_group:
            tot += len(items)
            continue
        maxlen = max(len(x[2]) for x in items)
        if maxlen < 4:
            tot += len(items)
            continue
        ks, _c = recover_keystream(items, maxlen, model_mat)
        o, n = valid_rate(items, ks)
        ok += o
        tot += n
        sizes.append((o / n, len(items)))
    big = [s for s in sizes if s[1] >= 30]
    avg_big = sum(r * n for r, n in big) / max(sum(n for _r, n in big), 1)
    print(f"{label:28} groups={len(groups):5} valid={100.0*ok/max(tot,1):5.1f}% "
          f"(groups>=30 entries: {100.0*avg_big:.1f}%)")
    return ok / max(tot, 1)


def main() -> None:
    path = Path(sys.argv[1])
    payloads, _ = load_payloads(path)
    print(f"{path.name}: {len(payloads):,} payloads\n")

    eval_scope(path, lambda it: "ALL", "global (one group)")
    eval_scope(path, lambda it: it[0], "per section name")
    eval_scope(path, lambda it: len(it[2]), "per payload length")
    eval_scope(path, lambda it: (it[0], len(it[2])), "per (section, length)")
    eval_scope(path, lambda it: it[1] >> 8, "per (id >> 8)")
    eval_scope(path, lambda it: it[1] & 0xFF, "per (id & 0xFF)")
    eval_scope(path, lambda it: it[0].split("_")[-1], "per section suffix")
    eval_scope(path, lambda it: len(it[2]) // 16, "per length/16 bucket")


if __name__ == "__main__":
    main()
