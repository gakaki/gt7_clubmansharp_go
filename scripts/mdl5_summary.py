#!/usr/bin/env python3
"""Summarise the MDL5 metadata index."""

import json
from collections import Counter
from pathlib import Path

INDEX = Path(r"C:\gt7\gt7\_re\reports\mdl5_index.json")
OUT = Path(r"C:\gt7\gt7\_re\reports\mdl5_summary.txt")


def main() -> None:
    data = json.loads(INDEX.read_text(encoding="utf-8"))
    counter = Counter()
    for m in data:
        counter.update(set(m["strings"]))
    total_strings = sum(len(m["strings"]) for m in data)

    lines = [
        f"models: {len(data):,}",
        f"strings: {total_strings:,}",
        f"distinct identifiers: {len(counter):,}",
        "",
        "identifiers present in 200..4000 models (model-specific vocabulary):",
    ]
    mid = [(k, n) for k, n in counter.items() if 200 <= n <= 4000]
    mid.sort(key=lambda kv: -kv[1])
    for name, n in mid[:300]:
        lines.append(f"{n:6}  {name}")

    OUT.write_text("\n".join(lines), encoding="utf-8")
    print("\n".join(lines[:35]))
    print(f"... {len(lines)} lines -> {OUT}")


if __name__ == "__main__":
    main()
