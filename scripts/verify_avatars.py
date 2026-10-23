#!/usr/bin/env python3
"""Verify the native Rust avatar extraction and detect accidental duplicates."""

from __future__ import annotations

import json
from pathlib import Path

from PIL import Image


ROOT = Path(r"C:\gt7\gt7\_re")
MANIFEST = ROOT / r"reports\face_atlases_rust\manifest.json"
EXPECTED_CAFE = {
    "luca", "sarah", "stella", "chris", "jeremy", "hideo-kodama",
    "shiro-nakamura", "kunihisa-ito", "hikonori-kagurazaka", "tom-matano",
    "fabio-filippini", "ed-welburn", "freeman-thomas", "ikuo-maeda",
    "kim-jones",
}


def average_hash(path: Path) -> int:
    image = Image.open(path).convert("L").resize((8, 8), Image.Resampling.LANCZOS)
    getter = getattr(image, "get_flattened_data", image.getdata)
    values = list(getter())
    mean = sum(values) / len(values)
    result = 0
    for index, value in enumerate(values):
        if value >= mean:
            result |= 1 << index
    return result


def main() -> None:
    payload = json.loads(MANIFEST.read_text(encoding="utf-8"))
    entries = payload["entries"]
    assert payload["count"] == len(entries) == 19
    assert {entry["key"] for entry in entries} >= EXPECTED_CAFE
    assert all(Path(entry["sourcePath"]).is_file() for entry in entries)
    assert all(Path(entry["atlas"]).is_file() for entry in entries)
    assert all(Path(entry["portrait"]).is_file() for entry in entries)
    assert all(entry["swizzleMode"] == 9 for entry in entries)
    assert all(entry["imageFormat"] == 0x0700 for entry in entries)
    assert all(entry["width"] == 352 and entry["height"] == 440 for entry in entries)

    hashes = {entry["key"]: average_hash(Path(entry["portrait"])) for entry in entries}
    assert len(set(hashes.values())) == 19, "avatar crops contain duplicates"
    distances = []
    keys = sorted(hashes)
    for left_index, left in enumerate(keys):
        for right in keys[left_index + 1:]:
            distance = (hashes[left] ^ hashes[right]).bit_count()
            distances.append((distance, left, right))
    minimum = min(distances)
    assert minimum[0] >= 4, minimum

    print(
        "PASS "
        f"avatars={len(entries)} cafe={len(EXPECTED_CAFE)} "
        f"unique_hashes={len(set(hashes.values()))} "
        f"min_pair_distance={minimum[0]} ({minimum[1]}, {minimum[2]})"
    )


if __name__ == "__main__":
    main()
