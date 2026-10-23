#!/usr/bin/env python3
"""Validate the public Cafe Menu Book payload before deployment."""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path


ROOT = Path(r"C:\gt7\gt7\_re")
PUBLIC = ROOT / "site" / "public"
DATA = PUBLIC / "data"


def fail(message: str) -> None:
    raise SystemExit(message)


def main() -> None:
    payload = json.loads((DATA / "menu_books.json").read_text(encoding="utf-8"))
    records = payload.get("records", [])
    if not records:
        fail("menu_books.json has no records")

    ids = [record["id"] for record in records]
    if len(ids) != len(set(ids)):
        fail("duplicate menu-book IDs")

    categories = Counter(record["category"] for record in records)
    expected = {"main": 39, "bonus": 12, "extra": 57}
    if dict(categories) != expected:
        fail(f"unexpected category counts: {dict(categories)}; expected {expected}")

    screenshot_covers = []
    container_only = []
    no_independent_cover = []
    bad_covers = []
    replacement_glyphs = 0
    for record in records:
        for field in (
            "titleEn",
            "titleZh",
            "collectionNameEn",
            "collectionNameZh",
            "descriptionEn",
            "descriptionZh",
            "rewardEn",
            "rewardZh",
        ):
            value = record.get(field, "")
            if isinstance(value, str):
                replacement_glyphs += value.count("\ufffd")
        cover = record.get("cover", "")
        if not cover:
            if record.get("coverContainerStatus") == "present":
                container_only.append(record["id"])
            else:
                no_independent_cover.append(record["id"])
            continue
        path = PUBLIC / cover
        if not path.is_file() or path.stat().st_size == 0:
            bad_covers.append((record["id"], cover))
        else:
            screenshot_covers.append(record["id"])

        cars = record.get("cars", [])
        brands = sorted(
            {
                car.get("brand", "")
                for car in cars
                if car.get("brand") and car.get("brand") != "Unknown"
            }
        )
        if brands and sorted(record.get("manufacturers", [])) != brands:
            fail(f"{record['id']}: manufacturer/car-brand mismatch")

    if replacement_glyphs:
        fail(f"replacement glyph U+FFFD remains in {replacement_glyphs} fields")
    if bad_covers:
        fail(f"cover paths do not resolve: {bad_covers[:8]}")

    for path in DATA.iterdir():
        if path.suffix.lower() not in {".csv", ".json", ".md"}:
            continue
        if "\ufffd" in path.read_text(encoding="utf-8"):
            fail(f"replacement glyph remains in {path}")

    print(
        json.dumps(
            {
                "records": len(records),
                "categories": dict(categories),
                "withScreenshotCover": len(screenshot_covers),
                "containerOnly": len(container_only),
                "containerOnlyIds": container_only,
                "withoutIndependentCover": len(no_independent_cover),
                "withoutIndependentCoverIds": no_independent_cover,
                "withAnyCoverSource": len(records) - len(no_independent_cover),
            },
            ensure_ascii=False,
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
