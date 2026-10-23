#!/usr/bin/env python3
"""Package the recovered GT7 menu disassembly with an audit manifest."""

from __future__ import annotations

import csv
import hashlib
import zipfile
from pathlib import Path


ROOT = Path(r"C:\gt7\gt7\_re")
SOURCE = ROOT / r"tree\scripts\gt7\menu"
ARCHIVE = ROOT / r"reports\gt7_menu_scripts_diss.zip"
MANIFEST = ROOT / r"reports\menu_disassembly_manifest.csv"


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def main() -> None:
    files = sorted(SOURCE.rglob("*.diss"))
    if not files:
        raise SystemExit(f"no .diss files under {SOURCE}")

    ARCHIVE.parent.mkdir(parents=True, exist_ok=True)
    rows = []
    with zipfile.ZipFile(
        ARCHIVE, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9
    ) as archive:
        for path in files:
            data = path.read_bytes()
            relative = path.relative_to(SOURCE).as_posix()
            archive.writestr(f"gt7/menu/{relative}", data)
            rows.append({
                "path": f"gt7/menu/{relative}",
                "bytes": len(data),
                "lines": data.count(b"\n"),
                "sha256": sha256(data),
            })

    with MANIFEST.open("w", encoding="utf-8-sig", newline="") as handle:
        writer = csv.DictWriter(
            handle, fieldnames=["path", "bytes", "lines", "sha256"]
        )
        writer.writeheader()
        writer.writerows(rows)

    print(
        f"packaged {len(rows)} disassembly files, "
        f"{sum(row['bytes'] for row in rows)} bytes, "
        f"{sum(row['lines'] for row in rows)} lines -> {ARCHIVE}"
    )


if __name__ == "__main__":
    main()
