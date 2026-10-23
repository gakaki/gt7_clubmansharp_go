#!/usr/bin/env python3
"""Rebuild GT Cafe scenes from the game's own conversation tables.

Unlike the old markup/window heuristic, this module joins:

* ``ConversationGTCafeMsg.dmsb``: one row per 1st/repeat/win conversation
  variant, including its exact start message id;
* ``GTCafeMessages.dmsb``: message id, localisation label, speaker and car key;
* decoded RT2 labels: exact message text in English and Simplified Chinese.

No text is assigned to a car by neighbouring rows.
"""

from __future__ import annotations

import csv
import json
import re
from collections import Counter, deque
from pathlib import Path

from car_texts import (
    PUBLIC_DIR,
    REPORT_DIR,
    _clean_text,
    _core_name,
    _find_dms_dir,
    _label_index,
    _norm,
    _parse_dms,
    _resolve_cars,
)
from textpick import load_best


VARIANT_ORDER = {4: 0, 3: 1, 2: 2, 1: 3}
VARIANT_NAMES = {
    4: ("初次见面", "First visit"),
    3: ("再次见面", "Repeat visit"),
    2: ("初次获胜后", "First visit after win"),
    1: ("再次获胜后", "Repeat visit after win"),
}

SPEAKER_TOKENS = {
    "CafeMaster": "Luca",
    "Sarah": "Sarah",
    "CarLover": "Stella",
    "ExpertOldMan1": "Chris",
    "RaceEnthusiast": "Jeremy",
    "CarDesignerProf1Kodama": "Hideo Kodama",
    "CarDesignerProf2Nakamura": "Shiro Nakamura",
    "CarDesignerProf3Ito": "Kunihisa Ito",
    "CarDesignerProf4Harada": "Hikonori Kagurazaka",
    "CarDesignerProf5Tom": "Tom Matano",
    "CarDesignerProf6Fabio": "Fabio Filippini",
    "CarDesignerProf7Ed": "Ed Welburn",
    "CarDesignerProf8Freeman": "Freeman Thomas",
    "CarDesignerProf9Maeda": "Ikuo Maeda",
    "DiorKimJones": "Kim Jones",
}

DETAIL_TEXT_CAR_OVERRIDES = {
    "0x0000964E": "Nissan GT-R LM NISMO '15",
    "0x0000D6DE": "Hyundai IONIQ 6 N '25",
    "0x0000D6D2": "Toyota Mark II Tourer V '97",
    "0x0000D6D8": "Toyota Chaser Tourer V '97",
    "0x0000D6CC": "Caterham Seven Superlight R500 '08",
}


def _speaker_from_label(label: str) -> str:
    prefix = "Buddy_GTCafe_"
    rest = label[len(prefix):] if label.startswith(prefix) else label
    if "_normal_" in rest:
        token = rest.split("_normal_", 1)[0]
    else:
        token = rest.split("__", 1)[0]
    return SPEAKER_TOKENS.get(token, token)


def _variant_from_key(key: str) -> int:
    if key.endswith("_Repeat_Win"):
        return 1
    if key.endswith("_1st_Win"):
        return 2
    if key.endswith("_Repeat"):
        return 3
    return 4


def _variant_from_message_id(message_id: int) -> int:
    suffix = message_id % 100
    if suffix == 11:
        return 3
    if suffix == 21:
        return 2
    if suffix == 31:
        return 1
    return 4


def _family_key(key: str) -> str:
    return re.sub(r"_(?:1st_Win|Repeat_Win|Repeat|1st)$", "", key)


def _message_targets(text: str) -> list[int]:
    targets = [
        int(value)
        for value in re.findall(r"<next\s+id=(\d+)\b", text)
    ]
    for attrs in re.findall(r"<yesno\s+([^>]+)>", text):
        values = dict(re.findall(r"(\w+)=(\d+)", attrs))
        for key in ("msgid", "nmsgid"):
            if key in values:
                targets.append(int(values[key]))
    return targets


def _recover_partial_next_targets(
    text: str,
    current_message_id: int,
    car_key: str,
    messages: dict[int, dict],
) -> list[int]:
    """Recover a next id when one middle digit was lost by RT2 decoding."""

    recovered = []
    for value in re.findall(r"next\s+id=(\d+)", text, re.IGNORECASE):
        if not 2 <= len(value) < 7:
            continue
        candidates = sorted(
            message_id
            for message_id, message in messages.items()
            if message["carKey"] == car_key
            and message_id > current_message_id
            and str(message_id).startswith(value)
        )
        if len(candidates) == 1:
            recovered.append(candidates[0])
    return recovered


def _scene_title(turns: list[dict]) -> tuple[str, str]:
    speakers = []
    for turn in turns:
        if turn["speaker"] not in speakers:
            speakers.append(turn["speaker"])
    guest = next(
        (speaker for speaker in speakers if speaker not in {"Sarah", "Luca"}),
        speakers[-1] if speakers else "GT Cafe",
    )
    if "Sarah" in speakers and guest != "Sarah":
        return f"{guest} 与 Sarah", f"{guest} and Sarah"
    return guest, guest


def build_exact_cafe(cars: list[dict], legacy_items: list[dict] | None = None) -> dict:
    dms_dir = _find_dms_dir()
    name_rows, name_string, name_u32 = _parse_dms(
        dms_dir / "CAR_NAME_ALPHABET.dmsb"
    )
    car_display = {
        name_string(name_u32(row, 0x08)): name_string(name_u32(row, 0x20))
        for row in name_rows
    }

    us = load_best("us")
    zh = load_best("zh")
    labels = _label_index("us")
    cafe_labels = {
        label: text_id
        for (section, label), text_id in labels.items()
        if section == "BuddyGTCafe"
    }
    cafe_text_labels = {
        text_id: label for label, text_id in cafe_labels.items()
    }

    message_rows, message_string, message_u32 = _parse_dms(
        dms_dir / "GTCafeMessages.dmsb"
    )
    messages = {}
    text_to_message = {}
    dms_labels = set()
    for row in message_rows:
        message_id = message_u32(row, 0x00)
        message_key = message_string(message_u32(row, 0x08))
        label = message_string(message_u32(row, 0x38))
        car_key = message_string(message_u32(row, 0x40))
        text_id = cafe_labels.get(label)
        if text_id is None:
            continue
        dms_labels.add(label)
        raw_en = us.get(("BuddyGTCafe", text_id), "")
        en = _clean_text(raw_en)
        zh_text = _clean_text(zh.get(("BuddyGTCafe", text_id), ""))
        if not en or en == label:
            continue
        message = {
            "id": text_id,
            "messageId": message_id,
            "messageKey": message_key,
            "label": label,
            "carKey": car_key,
            "raw": raw_en,
            "en": en,
            "zh": zh_text,
            "speaker": _speaker_from_label(label),
            "isCarDetail": bool(
                re.search(r"CarDetail", raw_en, re.IGNORECASE)
            ),
        }
        messages[message_id] = message
        text_to_message[text_id] = message

    # DLC cars added after the current GTCafeMessages DMSB still carry complete
    # RT2 labels.  Recover those rows directly from the stable label format.
    orphan_by_car = {}
    orphan_pattern = re.compile(
        r"^Buddy_GTCafe_(?P<speaker>.+?)_normal_(?P<car>.+)_(?P<message>\d+)$"
    )
    for label, text_id in cafe_labels.items():
        if label in dms_labels:
            continue
        match = orphan_pattern.match(label)
        if not match:
            continue
        message_id = int(match.group("message"))
        car_key = match.group("car")
        raw_en = us.get(("BuddyGTCafe", text_id), "")
        en = _clean_text(raw_en)
        zh_text = _clean_text(zh.get(("BuddyGTCafe", text_id), ""))
        if not en or en == label:
            continue
        message = {
            "id": text_id,
            "messageId": message_id,
            "messageKey": label,
            "label": label,
            "carKey": car_key,
            "raw": raw_en,
            "en": en,
            "zh": zh_text,
            "speaker": _speaker_from_label(label),
            "isCarDetail": bool(
                re.search(r"CarDetail", raw_en, re.IGNORECASE)
            ),
        }
        messages[message_id] = message
        text_to_message[text_id] = message
        orphan_by_car.setdefault(car_key, []).append(message)

    variant_rows, variant_string, variant_u32 = _parse_dms(
        dms_dir / "ConversationGTCafeMsg.dmsb"
    )
    variants = []
    for row in variant_rows:
        conversation_key = variant_string(variant_u32(row, 0x08))
        start_message = variant_u32(row, 0x1C)
        car_key = variant_string(variant_u32(row, 0x20))
        if start_message not in messages:
            continue
        variants.append({
            "conversationKey": conversation_key,
            "familyKey": _family_key(conversation_key),
            "variant": _variant_from_key(conversation_key),
            "startMessageId": start_message,
            "carKey": car_key,
        })

    # Build the missing DLC variants from roots (messages with no incoming
    # next/yesno reference) in each orphan car group.
    for car_key, orphan_messages in orphan_by_car.items():
        roots = sorted(
            message["messageId"]
            for message in orphan_messages
            if message["messageId"] % 100 in {1, 11, 21, 31}
        )
        for root in roots:
            variants.append({
                "conversationKey": f"RT2_{car_key}_{root}",
                "familyKey": f"RT2_{car_key}",
                "variant": _variant_from_message_id(root),
                "startMessageId": root,
                "carKey": car_key,
            })

    by_name: dict[str, list[dict]] = {}
    by_core: dict[str, list[dict]] = {}
    for car in cars:
        by_name.setdefault(_norm(car["nameEn"]), []).append(car)
        by_core.setdefault(_core_name(car), []).append(car)

    scenes = []
    unresolved_car_keys = {}
    for variant in variants:
        car_key = variant["carKey"]
        display = car_display.get(car_key, "")
        resolved, method, confidence, alternates = _resolve_cars(
            car_key, display, cars, by_name, by_core
        )
        if not resolved:
            unresolved_car_keys[car_key] = {
                "display": display,
                "alternates": alternates,
            }
            continue

        queue = deque([variant["startMessageId"]])
        visited = set()
        turns = []
        while queue:
            message_id = queue.popleft()
            if message_id in visited:
                continue
            message = messages.get(message_id)
            if message is None:
                continue
            visited.add(message_id)
            turn = dict(message)
            turn.update({
                "sceneId": variant["conversationKey"],
                "sceneKey": variant["conversationKey"],
                "sceneOrder": len(turns) + 1,
                "variant": variant["variant"],
                "variantZh": VARIANT_NAMES[variant["variant"]][0],
                "variantEn": VARIANT_NAMES[variant["variant"]][1],
            })
            turns.append(turn)
            targets = list(dict.fromkeys(_message_targets(message["raw"])))
            targets.extend(
                target
                for target in _recover_partial_next_targets(
                    message["raw"],
                    message_id,
                    message["carKey"],
                    messages,
                )
                if target not in targets
            )
            if (
                "next" in message["raw"].lower()
                and message_id + 1 in messages
                and messages[message_id + 1]["carKey"] == message["carKey"]
                and message_id + 1 not in targets
            ):
                targets.append(message_id + 1)
            for target in targets:
                if target not in visited:
                    queue.append(target)

        if not turns:
            continue
        title_zh, title_en = _scene_title(turns)
        scene = {
            **variant,
            "titleZh": title_zh,
            "titleEn": title_en,
            "turns": turns,
            "carIds": [car["id"] for car in resolved],
            "matchMethod": method,
            "confidence": confidence,
        }
        scenes.append(scene)

    scene_groups = {}
    for scene in scenes:
        key = (scene["carKey"], scene["familyKey"])
        scene_groups.setdefault(key, []).append(scene)
    for group in scene_groups.values():
        group.sort(key=lambda scene: VARIANT_ORDER.get(scene["variant"], 99))

    for car in cars:
        car["conversation"] = []
        car["dialogues"] = []
        car["cafeScenes"] = []

    primary_ids = set()
    for group in scene_groups.values():
        primary = next(
            (scene for scene in group if scene["variant"] == 4),
            group[0],
        )
        primary_ids.add(id(primary))

    for scene in scenes:
        if id(scene) not in primary_ids:
            continue
        for car in cars:
            if car["id"] not in scene["carIds"]:
                continue
            car["cafeScenes"].append({
                "familyKey": scene["familyKey"],
                "titleZh": scene["titleZh"],
                "titleEn": scene["titleEn"],
                "variants": [
                    {
                        "key": sibling["conversationKey"],
                        "variant": sibling["variant"],
                        "variantZh": VARIANT_NAMES[sibling["variant"]][0],
                        "variantEn": VARIANT_NAMES[sibling["variant"]][1],
                        "startMessageId": sibling["startMessageId"],
                        "turns": sibling["turns"],
                    }
                    for sibling in scene_groups[(scene["carKey"], scene["familyKey"])]
                ],
            })
            car["conversation"].extend(scene["turns"])

    detail_rows = []
    for message in messages.values():
        if not message["isCarDetail"]:
            continue
        display = car_display.get(message["carKey"], "")
        resolved, method, confidence, alternates = _resolve_cars(
            message["carKey"], display, cars, by_name, by_core
        )
        override_name = DETAIL_TEXT_CAR_OVERRIDES.get(message["id"])
        override_cars = (
            by_name.get(_norm(override_name), []) if override_name else []
        )
        if override_cars:
            resolved = override_cars
            method = "car-detail-text-id-override"
            confidence = 1.0
        row = {
            "id": message["id"],
            "messageId": message["messageId"],
            "messageKey": message["messageKey"],
            "carKey": message["carKey"],
            "sourceName": display,
            "speaker": message["speaker"],
            "en": message["en"],
            "zh": message["zh"],
            "carIds": [car["id"] for car in resolved],
            "matchMethod": method,
            "confidence": confidence,
        }
        detail_rows.append(row)
        for car in resolved:
            car["dialogues"].append(row)

    for text_id, car_name in DETAIL_TEXT_CAR_OVERRIDES.items():
        if any(row["id"] == text_id for row in detail_rows):
            continue
        resolved = by_name.get(_norm(car_name), [])
        if not resolved:
            continue
        label = cafe_text_labels.get(text_id, "")
        row = {
            "id": text_id,
            "messageId": None,
            "messageKey": "",
            "carKey": "",
            "sourceName": car_name,
            "speaker": _speaker_from_label(label) if label else "Luca",
            "en": _clean_text(us.get(("BuddyGTCafe", text_id), "")),
            "zh": _clean_text(zh.get(("BuddyGTCafe", text_id), "")),
            "carIds": [car["id"] for car in resolved],
            "matchMethod": "car-detail-text-id-override",
            "confidence": 1.0,
        }
        detail_rows.append(row)
        for car in resolved:
            car["dialogues"].append(row)

    resolved_by_text = {
        row["id"]: row
        for row in detail_rows
    }
    scene_keys_by_text = {}
    for scene in scenes:
        for turn in scene["turns"]:
            scene_keys_by_text.setdefault(turn["id"], set()).add(
                scene["conversationKey"]
            )
    for row in detail_rows:
        row["sceneId"] = next(
            iter(sorted(scene_keys_by_text.get(row["id"], []))),
            "",
        )
    review = []
    for item in legacy_items or []:
        exact = resolved_by_text.get(item["id"]) or text_to_message.get(item["id"])
        if exact is None:
            review.append({
                **item,
                "resolution": "missing-exact-label",
                "resolved": False,
            })
            continue
        resolved = exact.get("carIds") or []
        override_name = DETAIL_TEXT_CAR_OVERRIDES.get(item["id"])
        if not resolved and override_name:
            resolved = [car["id"] for car in by_name.get(_norm(override_name), [])]
        resolved_cars = [
            car for car in cars if car["id"] in set(resolved)
        ]
        review.append({
            **item,
            "resolution": "exact-rt2-label",
            "resolved": bool(resolved),
            "carId": resolved[0] if resolved else None,
            "carKey": exact["carKey"],
            "resolvedCar": exact.get("sourceName", ""),
            "speaker": exact.get("speaker", "Luca"),
            "resolvedCarNameEn": (
                resolved_cars[0]["nameEn"] if resolved_cars else ""
            ),
            "resolvedCarNameZh": (
                resolved_cars[0]["nameZh"] if resolved_cars else ""
            ),
            "sceneKeys": sorted(scene_keys_by_text.get(item["id"], [])),
            "sceneId": next(iter(sorted(scene_keys_by_text.get(item["id"], []))), ""),
        })

    for car in cars:
        deduped = {}
        for turn in car["conversation"]:
            deduped.setdefault(turn["id"], turn)
        car["conversation"] = list(deduped.values())
        dialog_deduped = {}
        for row in car["dialogues"]:
            dialog_deduped.setdefault(row["id"], row)
        car["dialogues"] = list(dialog_deduped.values())

    payload = {
        "scenes": scenes,
        "detailRows": detail_rows,
        "legacyUnmatchedReview": review,
        "stats": {
            "messages": len(messages),
            "variants": len(variants),
            "scenes": len(scenes),
            "carDetailRows": len(detail_rows),
            "carsWithConversation": sum(bool(car["conversation"]) for car in cars),
            "conversationLines": sum(len(car["conversation"]) for car in cars),
            "legacyReviewRows": len(review),
            "legacyReviewResolved": sum(row["resolved"] for row in review),
            "unresolvedCarKeys": len(unresolved_car_keys),
        },
        "unresolvedCarKeys": unresolved_car_keys,
    }

    payload_path = REPORT_DIR / "cafe_exact.json"
    payload_path.write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")),
        encoding="utf-8",
    )
    with (REPORT_DIR / "cafe_car_detail_exact.csv").open(
        "w", encoding="utf-8-sig", newline=""
    ) as handle:
        fields = [
            "id", "messageId", "carKey", "sourceName", "carIds",
            "speaker", "matchMethod", "confidence", "en", "zh",
        ]
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in detail_rows:
            writer.writerow({
                **{key: row.get(key, "") for key in fields},
                "carIds": ";".join(row["carIds"]),
            })
    with (REPORT_DIR / "legacy_unmatched_resolved.csv").open(
        "w", encoding="utf-8-sig", newline=""
    ) as handle:
        fields = [
            "id", "car", "speaker", "resolution", "resolved",
            "carId", "carKey", "resolvedCar", "resolvedCarNameEn",
            "resolvedCarNameZh", "sceneKeys", "en", "zh",
        ]
        writer = csv.DictWriter(handle, fieldnames=fields)
        writer.writeheader()
        for row in review:
            values = {key: row.get(key, "") for key in fields}
            values["sceneKeys"] = ";".join(row.get("sceneKeys", []))
            writer.writerow(values)
    variant_fields = [
        "sceneKey", "variant", "variantZh", "variantEn", "carKey",
        "carIds", "sceneOrder", "messageId", "textId", "speaker", "en", "zh",
    ]
    for target in (
        PUBLIC_DIR / "cafe_variants.csv",
        REPORT_DIR / "cafe_variants.csv",
    ):
        with target.open("w", encoding="utf-8-sig", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=variant_fields)
            writer.writeheader()
            for scene in scenes:
                for turn in scene["turns"]:
                    writer.writerow({
                        "sceneKey": scene["conversationKey"],
                        "variant": scene["variant"],
                        "variantZh": VARIANT_NAMES[scene["variant"]][0],
                        "variantEn": VARIANT_NAMES[scene["variant"]][1],
                        "carKey": scene["carKey"],
                        "carIds": ";".join(scene["carIds"]),
                        "sceneOrder": turn["sceneOrder"],
                        "messageId": turn["messageId"],
                        "textId": turn["id"],
                        "speaker": turn["speaker"],
                        "en": turn["en"],
                        "zh": turn["zh"],
                    })
    return payload


if __name__ == "__main__":
    site_path = Path(r"C:\gt7\gt7\_re\site\public\data\cars.json")
    site = json.loads(site_path.read_text(encoding="utf-8"))
    legacy = json.loads(
        Path(r"C:\gt7\gt7\_re\reports\cafe_dialogues.json").read_text(
            encoding="utf-8"
        )
    )
    report = build_exact_cafe(site["cars"], legacy)
    print(json.dumps(report["stats"], ensure_ascii=False, indent=2))
