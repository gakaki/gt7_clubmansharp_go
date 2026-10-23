#!/usr/bin/env python3
"""For a section, XOR same-length payloads to test whether a shared keystream applies."""

import sys
from collections import defaultdict
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rt2_decode import load_payloads  # noqa: E402


def main() -> None:
    path = Path(sys.argv[1])
    sec = sys.argv[2]
    payloads, _ = load_payloads(path)
    items = [p for p in payloads if p[0] == sec]
    by_len = defaultdict(list)
    for _n, sid, p in items:
        by_len[len(p)].append((sid, p))
    print(f"{sec}: {len(items)} payloads, lengths: "
          f"{sorted((k, len(v)) for k, v in by_len.items())}")

    for length, group in sorted(by_len.items()):
        if len(group) < 2:
            continue
        print(f"\n--- length {length}, {len(group)} payloads")
        sid0, p0 = group[0]
        print(f"  c0[0x{sid0:08X}] = {p0.hex()}")
        for sid1, p1 in group[1:4]:
            x = bytes(a ^ b for a, b in zip(p0, p1))
            printable = "".join(chr(b) if 0x20 <= b < 0x7F else "." for b in x)
            print(f"  xor with 0x{sid1:08X}: {x.hex()}  |{printable}|")


if __name__ == "__main__":
    main()
