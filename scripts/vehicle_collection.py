#!/usr/bin/env python3
"""Recover GT7 Car Collection entries with exact car ownership.

The PS5 ``description`` RT2 files provide stable ``DSC_*`` (Catch) and
``DSL_*`` (Long) labels plus their text IDs. English decodes cleanly with the
verified description keystream. The Chinese payloads retain a few damaged byte
columns, so their display copy is cross-checked against the official GT7
car-list description modules, which contain the same Catch/Long pair for all
580 current cars.

The generated report keeps both evidence paths: local RT2 label/text IDs and
the official bilingual description used for display.
"""

from __future__ import annotations

import csv
import json
import re
from pathlib import Path

import numpy as np

from car_texts import (
    LOCAL_NAME_OVERRIDES,
    TEXT_VARIANT_ALIASES,
    _clean_text,
    _core_name,
    _find_dms_dir,
    _norm,
    _parse_dms,
)
from rt2_probe import parse as parse_rt2
from rt2_decode import load_payloads as load_rt2_payloads


ROOT = Path(r"C:\gt7\gt7\_re")
RAW_DIR = ROOT / "extracted" / "descriptions_raw"
KEY_DIR = ROOT / "keys" / "rt2_description"
PUBLIC_DIR = ROOT / "site" / "public" / "data"
REPORT_DIR = ROOT / "reports"
OFFICIAL_PATH = REPORT_DIR / "official_car_descriptions.json"

EN_FILE = "L_O_PWIJL"
CN_FALLBACK_FILE = "P_9_PS69N"
CN_CORPUS = ROOT / "extracted" / "text_decoded" / "cn.json"
SECTION_KIND = {
    "DescriptionCatch": "catch",
    "DescriptionLong": "long",
}

TITLE_ZH = "车辆收藏"
TITLE_EN = "Car Collection"

# These variants are absent from CAR_NAME_ALPHABET but are present in the
# current car list. The names come from the same DMSB orphan mappings already
# used by the Brand Central extractor; no fuzzy matching is involved.
EXACT_CAR_KEY_OVERRIDES = {
    "f1_w08_eq_17": ["Mercedes-AMG F1 W08 EQ Power+ 2017"],
    "f1_w08_eq_17_le": [
        "Mercedes-AMG F1 W08 EQ Power+ (Color Variation) 2017",
    ],
    "mercedes_amg_vision_gt_race_13_lhc": [
        "Mercedes-Benz AMG Vision Gran Turismo LH Edition",
    ],
}

# Only use a shared official description where the relationship is explicit
# in the car name. The legacy `officialId` values in cars.json were produced by
# a looser website match and are not safe ownership evidence by themselves.
OFFICIAL_NAME_ALIASES = {
    "De Tomaso Mangusta (Christian Dior)": "De Tomaso Mangusta '69",
    "Gran Turismo Red Bull X2019 25th Anniversary":
        "Gran Turismo Red Bull X2019 Competition",
    "MAZDA RX-VISION GT3 CONCEPT Stealth Model":
        "MAZDA RX-VISION GT3 CONCEPT",
    "Mercedes-Benz AMG Vision Gran Turismo LH Edition":
        "Mercedes-Benz AMG Vision Gran Turismo Racing Series",
}


def _xor(payload: bytes, key: bytes) -> bytes:
    return bytes(value ^ key[index] for index, value in enumerate(payload))


def _clean_description(value: str) -> str:
    value = value.replace("\r\n", "\n").replace("\r", "\n")
    value = _clean_text(value.lstrip("* \t"))
    value = re.sub(r"[\ufffd\ue000-\uf8ff]+", "", value)
    value = value.replace("\u3000", " ")
    value = re.sub(r"[ \t]+\n", "\n", value)
    value = re.sub(r"\n[ \t]+", "\n", value)
    value = re.sub(r"\n{3,}", "\n\n", value)
    return value.strip()


def _normalise_copy(value: str) -> str:
    value = value.replace("\r\n", "\n").replace("\r", "\n")
    value = re.sub(r"[\ufffd\ue000-\uf8ff]+", "", value)
    value = re.sub(r"[ \t]+", " ", value)
    value = re.sub(r"\s*\n\s*", " ", value)
    return value.strip(" .。!?！？")


def _decode_english_table() -> dict[tuple[str, int], dict]:
    key = (KEY_DIR / f"{EN_FILE}.rt2.ks.bin").read_bytes()
    data, _, sections = parse_rt2(RAW_DIR / f"{EN_FILE}.rt2")
    result: dict[tuple[str, int], dict] = {}
    for section, _, _, entries in sections:
        for text_id, label_len, size, label_off, payload_off in entries:
            encrypted_label = data[label_off:label_off + label_len - 1]
            label = _xor(encrypted_label, key).decode(
                "utf-8", "replace"
            ).strip()
            raw = _xor(
                data[payload_off:payload_off + size], key
            ).rstrip(b"\x00")
            result[(section, text_id)] = {
                "label": label,
                "text": _clean_description(
                    raw.decode("utf-8", "replace")
                ),
            }
    return result


def _recover_cn_key(stem: str) -> bytes:
    """Recover the compact Chinese fallback key from clean local text.

    Only a handful of cars (currently the two F1 W08 entries) are absent from
    the official 580-car description modules. They are short enough to decode
    reliably with a UTF-8 byte-frequency model. The cache avoids rebuilding the
    4 KB keystream on each site build.
    """

    cache = KEY_DIR / f"{stem}.cn.ks.bin"
    if cache.is_file():
        return cache.read_bytes()
    if not CN_CORPUS.is_file():
        raise FileNotFoundError(f"missing Chinese corpus: {CN_CORPUS}")
    corpus = json.loads(CN_CORPUS.read_text(encoding="utf-8"))
    counts = np.ones(256, dtype=np.float64) * 0.1
    for section in corpus.values():
        for text in section.values():
            for value in text.encode("utf-8", "ignore"):
                counts[value] += 1
    for value in (
        "。，！？、：；“”‘’（）《》0123456789"
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz <>/-.\r\n"
    ).encode("utf-8"):
        counts[value] += 3
    scores = np.log(counts)
    payloads, _ = load_rt2_payloads(
        RAW_DIR / f"{stem}.rt2"
    )
    maxlen = max(len(payload) for _, _, payload in payloads)
    candidates = np.arange(256, dtype=np.uint16)
    plain = (
        candidates[None, :]
        ^ np.arange(256, dtype=np.uint16)[:, None]
    )
    key = np.zeros(maxlen, dtype=np.uint8)
    for offset in range(maxlen):
        values = np.fromiter(
            (
                payload[offset]
                for _, _, payload in payloads
                if len(payload) > offset
            ),
            dtype=np.uint8,
        )
        histogram = np.bincount(values, minlength=256)
        byte_scores = (
            histogram[None, :] * scores[plain]
        ).sum(axis=1)
        key[offset] = byte_scores.argmax()
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_bytes(key.tobytes())
    return key.tobytes()


def _decode_local_chinese() -> dict[tuple[str, int], str]:
    key = _recover_cn_key(CN_FALLBACK_FILE)
    payloads, _ = load_rt2_payloads(
        RAW_DIR / f"{CN_FALLBACK_FILE}.rt2"
    )
    result = {}
    for section, text_id, payload in payloads:
        raw = _xor(payload, key).rstrip(b"\x00")
        result[(section, text_id)] = _clean_description(
            raw.decode("utf-8", "replace")
        )
    return result


def _dms_displays() -> dict[str, str]:
    dms_dir = _find_dms_dir()
    rows, string_at, u32 = _parse_dms(
        dms_dir / "CAR_NAME_ALPHABET.dmsb"
    )
    return {
        string_at(u32(row, 0x08)): string_at(u32(row, 0x20))
        for row in rows
    }


def _stable_key_index(cars: list[dict]) -> dict[str, list[dict]]:
    index: dict[str, list[dict]] = {}
    for car in cars:
        for text in car.get("texts", {}).values():
            key = text.get("carKey", "")
            if key:
                index.setdefault(key, [])
                if car not in index[key]:
                    index[key].append(car)
    return index


def _resolve_exact(
    car_key: str,
    display: str,
    by_name: dict[str, list[dict]],
    by_core: dict[str, list[dict]],
) -> list[dict]:
    override_names = LOCAL_NAME_OVERRIDES.get(car_key, [])
    override_names = EXACT_CAR_KEY_OVERRIDES.get(car_key, override_names)
    names = override_names or [display]
    found = []
    for name in names:
        found.extend(by_name.get(_norm(name), []))
        if not override_names:
            found.extend(by_core.get(_norm(name), []))
    return list({car["id"]: car for car in found}.values())


def _load_official() -> tuple[dict[str, dict], dict[str, str]]:
    if not OFFICIAL_PATH.is_file():
        raise FileNotFoundError(
            f"{OFFICIAL_PATH} is missing; run "
            "fetch_official_descriptions.py --proxy <proxy>"
        )
    payload = json.loads(OFFICIAL_PATH.read_text(encoding="utf-8"))
    records = {row["id"]: row for row in payload["cars"]}
    names: dict[str, str] = {}
    for official_id, row in records.items():
        for name in (row.get("nameEn"), row.get("nameZh")):
            normalised = _norm(name or "")
            if normalised:
                names.setdefault(normalised, official_id)
    return records, names


def _official_for_car(
    car: dict,
    official: dict[str, dict],
    official_names: dict[str, str],
) -> dict | None:
    for name in (car.get("nameEn"), car.get("nameZh")):
        official_id = official_names.get(_norm(name or ""))
        if official_id:
            return official[official_id]
    alias = OFFICIAL_NAME_ALIASES.get(car.get("nameEn", ""))
    if alias:
        official_id = official_names.get(_norm(alias))
        if official_id:
            return official[official_id]
    return None


def _value_from_record(
    car: dict,
    record: dict,
    row: dict | None,
    match_method: str,
) -> dict:
    catch_id = (
        row.get("catch", {}).get("textId", "") if row else ""
    )
    long_id = (
        row.get("long", {}).get("textId", "") if row else ""
    )
    return {
        "titleZh": TITLE_ZH,
        "titleEn": TITLE_EN,
        "textId": catch_id,
        "textIds": {"catch": catch_id, "long": long_id},
        "carKey": row.get("carKey", "") if row else "",
        "sourceName": row.get("display", "") if row else record["nameEn"],
        "matchMethod": match_method,
        "officialId": record["id"],
        "translationSource": "official-gt7-carlist",
        "sourceUrlEn": record["sourceUrlEn"],
        "sourceUrlZh": record["sourceUrlZh"],
        "en": f"{record['heroEn']}\n\n{record['descEn']}",
        "zh": f"{record['heroZh']}\n\n{record['descZh']}",
        "catch": {
            "textId": catch_id,
            "en": record["heroEn"],
            "zh": record["heroZh"],
        },
        "long": {
            "textId": long_id,
            "en": record["descEn"],
            "zh": record["descZh"],
        },
    }


def build_vehicle_collection_texts(cars: list[dict]) -> dict:
    english = _decode_english_table()
    official, official_names = _load_official()
    displays = _dms_displays()
    stable_keys = _stable_key_index(cars)

    by_name: dict[str, list[dict]] = {}
    by_core: dict[str, list[dict]] = {}
    for car in cars:
        by_name.setdefault(_norm(car["nameEn"]), []).append(car)
        by_core.setdefault(_core_name(car), []).append(car)

    rows: dict[str, dict] = {}
    for (section, text_id), entry in english.items():
        kind = SECTION_KIND.get(section)
        label = entry["label"]
        prefix = "DSC_" if kind == "catch" else "DSL_" if kind == "long" else ""
        if not kind or not label.startswith(prefix):
            continue
        car_key = label[len(prefix):]
        row = rows.setdefault(car_key, {"carKey": car_key})
        row[kind] = {
            "textId": f"0x{text_id:08X}",
            "en": entry["text"],
        }

    unresolved = []
    entries = []
    rows_by_car: dict[str, list[dict]] = {}
    english_matches = 0
    local_chinese = None
    for car_key, row in sorted(rows.items()):
        display = displays.get(car_key, "")
        row["display"] = display
        override_names = EXACT_CAR_KEY_OVERRIDES.get(car_key, [])
        if override_names:
            resolved = list({
                car["id"]: car
                for name in override_names
                for car in by_name.get(_norm(name), [])
            }.values())
            method = "explicit-car-key-override"
        else:
            resolved = stable_keys.get(car_key, [])
            method = "stable-car-key"
        if not resolved:
            resolved = _resolve_exact(
                car_key, display, by_name, by_core
            )
            method = "exact-display-name"

        if not resolved:
            unresolved.append({
                "carKey": car_key,
                "sourceName": display,
                "catchTextId": row.get("catch", {}).get("textId", ""),
                "longTextId": row.get("long", {}).get("textId", ""),
            })
            continue

        for car in resolved:
            record = _official_for_car(car, official, official_names)
            if record:
                if (
                    _normalise_copy(row.get("catch", {}).get("en", ""))
                    == _normalise_copy(record["heroEn"])
                ):
                    english_matches += 1
                value = _value_from_record(car, record, row, method)
            else:
                if local_chinese is None:
                    local_chinese = _decode_local_chinese()
                catch_id = row.get("catch", {}).get("textId", "")
                long_id = row.get("long", {}).get("textId", "")
                catch_zh = local_chinese.get(
                    (
                        "DescriptionCatch",
                        int(catch_id, 16) if catch_id else -1,
                    ),
                    "",
                )
                long_zh = local_chinese.get(
                    (
                        "DescriptionLong",
                        int(long_id, 16) if long_id else -1,
                    ),
                    "",
                )
                # Keep the local evidence if an edge-case variant is not in
                # the current official 580-car description set.
                value = {
                    "titleZh": TITLE_ZH,
                    "titleEn": TITLE_EN,
                    "textId": row.get("catch", {}).get("textId", ""),
                    "textIds": {
                        "catch": row.get("catch", {}).get("textId", ""),
                        "long": row.get("long", {}).get("textId", ""),
                    },
                    "carKey": car_key,
                    "sourceName": display,
                    "matchMethod": method,
                    "officialId": "",
                    "translationSource": "local-rt2-recovered",
                    "sourceUrlEn": "",
                    "sourceUrlZh": "",
                    "en": "\n\n".join(
                        part for part in (
                            row.get("catch", {}).get("en", ""),
                            row.get("long", {}).get("en", ""),
                        ) if part
                    ),
                    "zh": "\n\n".join(
                        part for part in (catch_zh, long_zh) if part
                    ),
                    "catch": {
                        "textId": catch_id,
                        "en": row.get("catch", {}).get("en", ""),
                        "zh": catch_zh,
                    },
                    "long": {
                        "textId": long_id,
                        "en": row.get("long", {}).get("en", ""),
                        "zh": long_zh,
                    },
                }
            car.setdefault("texts", {})["vehicleCollection"] = value
            rows_by_car.setdefault(car["id"], []).append(row)
            entries.append({
                "category": "vehicleCollection",
                "carId": car["id"],
                "carNameEn": car["nameEn"],
                **value,
            })

    # The official list is canonical for the current 580-car roster. Attach
    # the same description to every local CarNameLong alias sharing that
    # exact official id, even when no dealer text established a car key.
    for car in cars:
        if "vehicleCollection" in car.get("texts", {}):
            continue
        record = _official_for_car(car, official, official_names)
        if record is None:
            continue
        matched_rows = rows_by_car.get(car["id"], [])
        row = matched_rows[0] if matched_rows else None
        value = _value_from_record(car, record, row, "official-id")
        car.setdefault("texts", {})["vehicleCollection"] = value
        entries.append({
            "category": "vehicleCollection",
            "carId": car["id"],
            "carNameEn": car["nameEn"],
            **value,
        })

    # Explicit variants can reuse a base model's collection entry only where
    # the existing text pipeline already documents that relationship.
    cars_by_name = {_norm(car["nameEn"]): car for car in cars}
    for target_name, source_names in TEXT_VARIANT_ALIASES.items():
        target = cars_by_name.get(_norm(target_name))
        if target is None:
            continue
        for source_name in source_names:
            source = cars_by_name.get(_norm(source_name))
            if source and "vehicleCollection" in source.get("texts", {}):
                target.setdefault("texts", {}).setdefault(
                    "vehicleCollection",
                    {
                        **source["texts"]["vehicleCollection"],
                        "variantOf": source["nameEn"],
                    },
                )

    collection_cars = [
        car for car in cars
        if "vehicleCollection" in car.get("texts", {})
    ]
    stats = {
        "descriptionCars": len(rows),
        "rt2RowsResolved": len(rows) - len(unresolved),
        "officialDescriptions": len(official),
        "officialNameMatches": sum(
            bool(_official_for_car(car, official, official_names))
            for car in cars
        ),
        "rt2EnglishCatchMatchesOfficial": english_matches,
        "matchedCars": len({car["id"] for car in collection_cars}),
        "matchedEntries": len(entries),
        "unresolvedCars": len(unresolved),
        "catchTexts": sum("catch" in row for row in rows.values()),
        "longTexts": sum("long" in row for row in rows.values()),
        "carsWithCollection": len(collection_cars),
        "carsWithBilingualCollection": sum(
            bool(
                car["texts"]["vehicleCollection"].get("en")
                and car["texts"]["vehicleCollection"].get("zh")
            )
            for car in collection_cars
        ),
        "localRt2FallbackCars": sum(
            car["texts"]["vehicleCollection"].get("translationSource")
            == "local-rt2-recovered"
            for car in collection_cars
        ),
        "replacementAfterClean": sum(
            "\ufffd" in car["texts"]["vehicleCollection"].get("zh", "")
            for car in collection_cars
        ),
    }
    report = {
        "generatedFrom": (
            "PS5 DescriptionCatch/DescriptionLong RT2 labels and text IDs, "
            "cross-checked with official GT7 car-list descriptions"
        ),
        "sources": [
            {
                "id": "vehicleCollection",
                "titleZh": TITLE_ZH,
                "titleEn": TITLE_EN,
                "rt2Sections": ["DescriptionCatch", "DescriptionLong"],
                "english": EN_FILE,
                "officialCache": OFFICIAL_PATH.name,
                "officialSources": json.loads(
                    OFFICIAL_PATH.read_text(encoding="utf-8")
                )["sources"],
            }
        ],
        "entries": entries,
        "unresolved": unresolved,
        "stats": stats,
    }

    PUBLIC_DIR.mkdir(parents=True, exist_ok=True)
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(report, ensure_ascii=False, indent=1)
    (REPORT_DIR / "vehicle_collection_texts.json").write_text(
        payload, encoding="utf-8"
    )
    csv_fields = [
        "carId", "carNameEn", "carKey", "sourceName", "matchMethod",
        "officialId", "translationSource", "catchTextId", "longTextId",
        "en", "zh",
    ]
    csv_rows = [
        {
            "carId": entry["carId"],
            "carNameEn": entry["carNameEn"],
            "carKey": entry["carKey"],
            "sourceName": entry["sourceName"],
            "matchMethod": entry["matchMethod"],
            "officialId": entry["officialId"],
            "translationSource": entry["translationSource"],
            "catchTextId": entry["catch"]["textId"],
            "longTextId": entry["long"]["textId"],
            "en": entry["en"],
            "zh": entry["zh"],
        }
        for entry in entries
    ]
    for target in (
        PUBLIC_DIR / "vehicle_collection_texts.csv",
        REPORT_DIR / "vehicle_collection_texts.csv",
    ):
        with target.open("w", encoding="utf-8-sig", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=csv_fields)
            writer.writeheader()
            writer.writerows(csv_rows)
    return report


if __name__ == "__main__":
    site = json.loads(
        (PUBLIC_DIR / "cars.json").read_text(encoding="utf-8")
    )
    result = build_vehicle_collection_texts(site["cars"])
    print(json.dumps(result["stats"], ensure_ascii=False, indent=2))
    print("\nUnresolved:")
    for row in result["unresolved"]:
        print(json.dumps(row, ensure_ascii=False))
