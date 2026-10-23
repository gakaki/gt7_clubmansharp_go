#!/usr/bin/env python3
"""Assert that the native Rust research pipeline matches the Python pipeline."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(r"C:\gt7\gt7")
REPORTS = ROOT / r"_re\reports"
SITE_DATA = ROOT / r"_re\site\public\data\cars.json"


def load(path: Path):
    return json.loads(path.read_text(encoding="utf-8"))


def car_signature(car: dict) -> tuple:
    return (
        car["id"],
        car["nameEn"].strip(),
        tuple(sorted(car.get("texts", {}))),
    )


def conversation_signature(car: dict) -> tuple:
    turns = car.get("conversation", [])
    return tuple(sorted((turn["id"], turn.get("sceneId") or turn.get("sceneKey") or "") for turn in turns))


def main() -> None:
    python_site = load(SITE_DATA)
    rust = load(REPORTS / "research_rust.json")
    python_texts = load(REPORTS / "car_texts.json")
    python_cafe = load(REPORTS / "cafe_exact.json")

    py_cars = {car["id"]: car for car in python_site["cars"]}
    rs_cars = {car["id"]: car for car in rust["cars"]}
    assert py_cars.keys() == rs_cars.keys(), (
        sorted(py_cars.keys() - rs_cars.keys()),
        sorted(rs_cars.keys() - py_cars.keys()),
    )
    assert {key: car_signature(car) for key, car in py_cars.items()} == {
        key: car_signature(car) for key, car in rs_cars.items()
    }
    assert {
        key: conversation_signature(car) for key, car in py_cars.items()
    } == {
        key: conversation_signature(car) for key, car in rs_cars.items()
    }

    py_entries = {
        entry["textId"]: (
            entry["category"],
            entry["carKey"],
            tuple(entry["carIds"]),
        )
        for entry in python_texts["entries"]
    }
    rs_entries = {
        entry["textId"]: (
            entry["category"],
            entry["carKey"],
            tuple(entry["carIds"]),
        )
        for entry in rust["textEntries"]
    }
    assert py_entries == rs_entries

    py_scenes = {
        scene["conversationKey"]: (
            tuple(scene["carIds"]),
            tuple(turn["id"] for turn in scene["turns"]),
        )
        for scene in python_cafe["scenes"]
    }
    rs_scenes = {
        scene["conversationKey"]: (
            tuple(scene["carIds"]),
            tuple(turn["id"] for turn in scene["turns"]),
        )
        for scene in rust["cafeScenes"]
    }
    assert py_scenes == rs_scenes

    assert rust["stats"]["cars"] == python_site["stats"]["cars"] == 591
    assert rust["stats"]["textEntries"] == len(python_texts["entries"]) == 634
    assert rust["stats"]["cafeVariants"] == len(python_cafe["scenes"]) == 1698
    assert rust["stats"]["carDetailRows"] == 444
    assert rust["stats"]["conversationLines"] == python_site["stats"]["conversationLines"] == 2620
    assert len(rust["legacyReview"]) == 253
    assert not rust["unresolvedCarKeys"]

    print(
        "PASS "
        f"cars={len(py_cars)} "
        f"texts={len(py_entries)} "
        f"scenes={len(py_scenes)} "
        f"conversation={rust['stats']['conversationLines']} "
        f"review={len(rust['legacyReview'])}"
    )


if __name__ == "__main__":
    main()
