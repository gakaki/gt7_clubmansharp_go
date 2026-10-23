#!/usr/bin/env python3
"""Show decoded strings for chosen .rt2 sections using a per-section keystream."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rt2_decode import build_model, recover_keystream, load_payloads  # noqa: E402


def main() -> None:
    path = Path(sys.argv[1])
    wanted = set(sys.argv[2:])
    payloads, _ = load_payloads(path)
    model_mat = build_model()

    by_sec: dict[str, list] = {}
    for item in payloads:
        by_sec.setdefault(item[0], []).append(item)

    for name, items in by_sec.items():
        if wanted and name not in wanted:
            continue
        ks, _c = recover_keystream(items, max(len(x[2]) for x in items), model_mat)
        print(f"\n=== {name} (n={len(items)}) ===")
        for i, (n2, sid, p) in enumerate(items):
            if i >= 12:
                break
            d = bytes(b ^ ks[j] for j, b in enumerate(p)).rstrip(b"\x00")[1:]
            print(f"  0x{sid:08X} len={len(p):3}  {d.decode('utf-8','replace')!r}")


if __name__ == "__main__":
    main()
