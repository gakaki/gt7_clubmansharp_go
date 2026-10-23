#!/usr/bin/env python3
"""Remove undecodable replacement glyphs from public-facing research data.

The RT2 decoder can recover the surrounding sentence while one or more glyph
slots remain undecodable. U+FFFD is useful in the raw extraction, but it is not
useful in the public browser. This script keeps the text and removes only the
replacement code point, its redundant spacing, and adjacent replacement
separators. It records a deterministic quality report for later auditing.
"""

from __future__ import annotations

import json
import re
from pathlib import Path


ROOT = Path(r"C:\gt7\gt7\_re")
DATA_DIR = ROOT / "site" / "public" / "data"
REPORT = ROOT / "reports" / "text_quality.json"
TEXT_EXTENSIONS = {".csv", ".json", ".md"}


def sanitize(text: str) -> tuple[str, int, int]:
    count = text.count("\ufffd")
    controls = len(
        re.findall(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]", text)
    )
    text = re.sub(r"[\x00-\x08\x0b\x0c\x0e-\x1f\x7f]", "", text)
    text = re.sub(r"\ufffd\s*;\s*\ufffd", "", text)
    text = text.replace("\ufffd", "")
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"[ \t]{2,}", " ", text)
    return text, count, controls


def main() -> None:
    files = []
    total = 0
    control_total = 0
    for path in sorted(DATA_DIR.rglob("*")):
        if not path.is_file() or path.suffix.lower() not in TEXT_EXTENSIONS:
            continue
        original = path.read_text(encoding="utf-8-sig")
        repaired, count, controls = sanitize(original)
        if not count and not controls:
            continue
        path.write_text(repaired, encoding="utf-8")
        files.append(
            {
                "path": path.relative_to(ROOT).as_posix(),
                "removed": count,
                "controlCharactersRemoved": controls,
            }
        )
        total += count
        control_total += controls

    REPORT.parent.mkdir(parents=True, exist_ok=True)
    REPORT.write_text(
        json.dumps(
            {
                "generatedFrom": "site/public/data",
                "policy": "remove U+FFFD only; never invent a missing glyph",
                "removed": total,
                "controlCharactersRemoved": control_total,
                "files": files,
            },
            ensure_ascii=False,
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )
    print(
        json.dumps(
            {
                "removed": total,
                "controlCharactersRemoved": control_total,
                "files": len(files),
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
