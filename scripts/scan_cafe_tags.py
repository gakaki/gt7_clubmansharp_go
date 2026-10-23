#!/usr/bin/env python3
"""List all markup tags used by GT Cafe dialogue payloads."""

import re
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rt2_probe import parse  # noqa: E402

DATA = Path(r"C:\gt7\gt7\_re\extracted\text\rtext\common\us.rt2")
KS = Path(r"C:\gt7\gt7\_re\keys\rt2_pooled.ks.bin")


def main() -> None:
    d, _n, sections = parse(DATA)
    ks = KS.read_bytes()
    counter = Counter()
    samples = {}
    for sec in sections:
        if sec[0] != "BuddyGTCafe":
            continue
        for sid, f1, size, bo, po in sec[3]:
            payload = d[po:po + size]
            text = bytes(b ^ ks[i] for i, b in enumerate(payload)).decode("utf-8", "replace")
            for tag in re.findall(r"<[^>]+>", text):
                key = re.sub(r'="[^"]*"', '=""', tag)
                counter[key] += 1
                samples.setdefault(key, (sid, tag))
    for key, n in counter.most_common(40):
        sid, sample = samples[key]
        print(f"{n:5}  {key:48} e.g. {sample}  @0x{sid:08X}")


if __name__ == "__main__":
    main()
