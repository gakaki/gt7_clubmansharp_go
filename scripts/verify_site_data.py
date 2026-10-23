#!/usr/bin/env python3
"""Validate the GT7 cafe site's dialogue, speaker, and image data."""

import csv
import json
from pathlib import Path

ROOT = Path(r"C:\gt7\gt7\_re\site")
DATA = ROOT / "public" / "data" / "cars.json"
CSV_DATA = ROOT / "public" / "data" / "cafe_dialogues.csv"
SCENE_CSV = ROOT / "public" / "data" / "cafe_scene_dialogues.csv"
TEXT_CSV = ROOT / "public" / "data" / "car_texts.csv"
UNMATCHED_CSV = ROOT / "public" / "data" / "unmatched_dialogues.csv"
UNMATCHED_MD = ROOT / "public" / "data" / "unmatched_dialogues.md"


def main() -> None:
    payload = json.loads(DATA.read_text(encoding="utf-8"))
    cars = payload["cars"]
    dialogues = [d for car in cars for d in car["dialogues"]]

    assert len({d["id"] for d in dialogues}) == 444
    assert len({d["id"] for car in cars for d in car["dialogues"]}) == 444
    assert payload["stats"]["dialogues"] == 444
    assert payload["stats"]["unmatched"] == 0
    assert payload["stats"]["legacyReview"] == 253
    assert len(payload["unmatched"]) == 253
    assert all(row.get("resolved") for row in payload["unmatched"])
    assert payload["stats"]["mappedDialogues"] == 444
    assert not [d for d in dialogues if d["speaker"] in {"", "Buddy", "Guest"}]
    assert not [d for d in dialogues if not d.get("speakerZh") or not d.get("roleZh")]

    text_stats = payload["carTextStats"]
    assert text_stats["entries"] == 634, text_stats
    assert text_stats["mappedEntries"] == 634, text_stats
    assert text_stats["unresolvedEntries"] == 0, text_stats
    assert text_stats["orphanTextRows"] == 9, text_stats
    assert text_stats["carsWithAnyText"] == 581, text_stats
    assert text_stats["byCategory"] == {
        "brandCentral": 401,
        "usedCar": 139,
        "collection": 94,
    }, text_stats
    assert all(isinstance(car.get("texts"), dict) for car in cars)
    assert sum(bool(car["texts"]) for car in cars) == 581
    allowed_text_categories = {"brandCentral", "usedCar", "collection"}
    assert all(
        set(car["texts"]) <= allowed_text_categories
        for car in cars
    )
    assert all(
        text.get("en") and text.get("zh") and text.get("textId")
        for car in cars
        for text in car["texts"].values()
    )
    assert payload["stats"]["legacyReview"] == len(payload["unmatched"]) == 253

    missing_images = []
    for car in cars:
        if not car.get("image"):
            continue
        path = ROOT / "public" / car["image"]
        if not path.exists():
            missing_images.append(str(path))
    assert not missing_images, missing_images[:10]

    with CSV_DATA.open(encoding="utf-8-sig", newline="") as fh:
        rows = list(csv.DictReader(fh))
    assert len(rows) == 444, len(rows)
    assert not [row for row in rows if not row["speaker"]]

    with TEXT_CSV.open(encoding="utf-8-sig", newline="") as fh:
        car_text_rows = list(csv.DictReader(fh))
    assert len(car_text_rows) == 634, len(car_text_rows)
    assert all(row["carIds"] and row["en"] and row["zh"] for row in car_text_rows)

    with UNMATCHED_CSV.open(encoding="utf-8-sig", newline="") as fh:
        unmatched_rows = list(csv.DictReader(fh))
    assert len(unmatched_rows) == 253, len(unmatched_rows)
    unmatched_markdown = UNMATCHED_MD.read_text(encoding="utf-8")
    assert "Legacy review records: **253**" in unmatched_markdown
    assert "Exact-resolved records: **253**" in unmatched_markdown
    assert "Distinct conversation scene keys: **956**" in unmatched_markdown
    assert all(row["resolved_car_en"] for row in unmatched_rows)
    assert sum(not row["scene_keys"] for row in unmatched_rows) == 7

    scene_turns = [turn for car in cars for turn in car["conversation"]]
    assert payload["stats"]["conversationLines"] == len({
        (turn["id"], turn.get("sceneId") or "")
        for turn in scene_turns
    })
    assert not [turn for turn in scene_turns if not turn["speaker"]]
    with SCENE_CSV.open(encoding="utf-8-sig", newline="") as fh:
        scene_rows = list(csv.DictReader(fh))
    assert len(scene_rows) == payload["stats"]["conversationLines"], (
        len(scene_rows),
        payload["stats"]["conversationLines"],
    )

    by_name = {car["nameEn"]: car for car in cars}
    assert len(cars) == 591
    assert len(by_name) == len(cars)
    assert by_name["Xiaomi SU7 Ultra '25"]["conversation"] == []
    assert by_name["Porsche 911 GT3 RS (992) '22"]["conversation"] == []

    text_sources = {source["id"] for source in payload["carTextSources"]}
    assert text_sources == {"brandCentral", "usedCar", "collection"}
    assert payload["carTextStats"]["mappedEntries"] >= 634
    assert payload["carTextStats"]["carsWithAnyText"] >= 581
    assert all(isinstance(car.get("texts"), dict) for car in cars)
    assert by_name["Xiaomi SU7 Ultra '25"]["texts"]["brandCentral"]["en"]
    assert by_name["Xiaomi SU7 Ultra '25"]["texts"]["brandCentral"]["zh"]
    assert by_name["Lamborghini Urus '18"]["texts"]["brandCentral"]["en"]
    for car in cars:
        for category, text in car["texts"].items():
            assert category in text_sources, (car["nameEn"], category)
            assert text.get("en") or text.get("zh"), (car["nameEn"], category)

    public_data = ROOT / "public" / "data"
    for name in (
        "car_texts.json",
        "car_texts.csv",
        "car_text_orphans.csv",
        "cafe_variants.csv",
        "unmatched_dialogues.csv",
        "unmatched_dialogues.md",
    ):
        assert (public_data / name).is_file(), name

    def ids(name: str) -> set[str]:
        return {turn["id"] for turn in by_name[name]["conversation"]}

    assert ids("Lamborghini Urus '18") == {
        "0x0000BA4F", "0x0000BA50", "0x0000BA51", "0x0000BA52",
    }
    assert "0x0000D6D8" in ids("Toyota Chaser Tourer V '97")
    assert {
        dialogue["id"]
        for dialogue in by_name["Toyota Chaser Tourer V '97"]["dialogues"]
    } == {"0x0000D6D8"}
    assert "0x0000B0FF" in ids("Abarth 1500 Biposto Bertone B.A.T 1 '52")
    assert "0x0000A794" in ids("Shelby Cobra Daytona Coupe '64")
    assert "0x0000A794" not in ids("Ferrari GTO '84")
    assert "0x00009684" not in ids("Chevrolet Corvette CX Concept '25")

    # Every conversation variant exposed on a car page must carry that exact
    # car id in the DMSB/RT2-derived scene graph.
    exact_report = json.loads(
        (Path(r"C:\gt7\gt7\_re\reports") / "cafe_exact.json").read_text(
            encoding="utf-8"
        )
    )
    scene_owners = {}
    for scene in exact_report["scenes"]:
        scene_owners[scene["conversationKey"]] = set(scene["carIds"])
    for car in cars:
        conversation_scenes = {
            turn["sceneKey"] for turn in car["conversation"]
        }
        for scene_key in conversation_scenes:
            assert scene_key in scene_owners, (car["nameEn"], scene_key)
            assert car["id"] in scene_owners[scene_key], (
                car["nameEn"],
                car["id"],
                scene_key,
                sorted(scene_owners[scene_key]),
            )
    assert sum(bool(car["dialogues"]) for car in cars) == 329
    assert sum(bool(car["conversation"]) for car in cars) == exact_report["stats"]["carsWithConversation"]
    assert "cafeScenes" not in cars[0]

    print(
        "PASS "
        f"cars={len(cars)} "
        f"with_dialogue={payload['stats']['withDialogue']} "
        f"dialogues={len({d['id'] for d in dialogues})} "
        f"scene_lines={len(scene_turns)} "
        f"mapped={payload['stats']['mappedDialogues']} "
        f"images={sum(bool(c.get('image')) for c in cars)} "
        f"speakers={len(payload['speakers'])} "
        f"car_texts={text_stats['mappedEntries']} "
        f"text_cars={text_stats['carsWithAnyText']} "
        f"review={len(unmatched_rows)}"
    )


if __name__ == "__main__":
    main()
