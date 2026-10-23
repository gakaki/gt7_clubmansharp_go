#!/usr/bin/env python3
"""Generate candidate GT7 asset paths from GTToolsSharp's MPH brute-forcer templates."""

import itertools
import re
import sys
from pathlib import Path

OUT = Path(r"C:\gt7\gt7\_re\reports\candidates_templates.txt")

PLACEHOLDER = re.compile(r"\{[^}]*\}")


def pad_width(expr: str) -> int:
    m = re.search(r"PadLeft\((\d+)", expr)
    if m:
        return int(m.group(1))
    return 0


def main() -> None:
    src = Path(sys.argv[1]).read_text(encoding="utf-8", errors="replace")
    templates: list[str] = []
    for line in src.splitlines():
        if "CheckFile" not in line:
            continue
        # every string literal on the line, including interpolated ones
        for lit in re.findall(r'\$?"((?:[^"\\]|\\.)*)"', line):
            if "/" not in lit and not lit.endswith((".img", ".txs", ".json", ".dat", ".adc")):
                continue
            templates.append(lit)

    seen = set()
    cands: list[str] = []
    max_id = 320
    for tpl in templates:
        if tpl in seen:
            continue
        seen.add(tpl)
        phs = list(PLACEHOLDER.finditer(tpl))
        if not phs:
            cands.append(tpl)
            continue
        widths = [pad_width(p.group()) for p in phs]
        n = len(phs)
        limit = 400 if n == 1 else 200
        ranges = [range(0, limit if w == 0 else min(limit, 10 ** w)) for w in widths]
        for combo in itertools.product(*ranges):
            out = tpl
            for p, val, w in zip(reversed(phs), reversed(combo), reversed(widths)):
                s = str(val).zfill(w) if w else str(val)
                out = out[:p.start()] + s + out[p.end():]
            cands.append(out)

    # extra families
    car_ids = [f"{a:04d}" for a in range(0, 300)]
    for a, b in itertools.product(range(0, 30), range(0, 200)):
        pass

    cands = list(dict.fromkeys(cands))
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text("\n".join(cands), encoding="utf-8")
    print(f"templates: {len(seen):,}  candidates: {len(cands):,} -> {OUT}")


if __name__ == "__main__":
    main()
