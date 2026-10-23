#!/usr/bin/env python3
"""Extract GT7 car descriptions from the local DMSB and RT2 tables.

The three dealer description tables are normal DMSB fixed-row records:

* ``+0x38`` points to the message key in the string table
* ``+0x40`` points to the stable car key in the string table

``CAR_NAME_ALPHABET.dmsb`` maps those stable car keys to display names.
The current retail text IDs are a fixed offset from the message number for
each table.  Only IDs that actually exist in the extracted RT2 section are
emitted; absent IDs are reported rather than invented.
"""

from __future__ import annotations

import csv
import difflib
import json
import re
import unicodedata
from collections import Counter
from pathlib import Path

from rt2_probe import parse as parse_rt2
from textpick import load_best


ROOT = Path(r"C:\gt7\gt7")
DMS_ROOT = ROOT / r"_re\extracted\dms_prpys\dmsdata"
RT2_ROOT = ROOT / r"_re\extracted\text\rtext\common"
KEY_ROOT = ROOT / r"_re\keys"
PUBLIC_DIR = ROOT / r"_re\site\public\data"
REPORT_DIR = ROOT / r"_re\reports"

CATEGORY_SPECS = [
    {
        "id": "brandCentral",
        "titleZh": "品牌购物",
        "titleEn": "Brand Central",
        "file": "BrandCentralOwnerCarDescriptionMessages.dmsb",
        "section": "BuddyContentCarBrandCentralOwner",
    },
    {
        "id": "usedCar",
        "titleZh": "二手车",
        "titleEn": "Used Car Dealer",
        "file": "UsedCarOwnerCarDescriptionMessages.dmsb",
        "section": "BuddyContentCarUsedCarOwner",
    },
    {
        "id": "collection",
        "titleZh": "收藏馆",
        "titleEn": "Hagerty Collection",
        "file": "HagertyCarDescriptionMessages.dmsb",
        "section": "BuddyContentCarHagerty",
    },
]

# Keys whose display name is abbreviated, contains a localisation artefact, or
# intentionally has more than one corresponding garage asset.  These mappings
# are name-based so they remain stable if numeric asset IDs change.
LOCAL_NAME_OVERRIDES = {
    "r18_tdi_11": ["Audi R18 TDI '11"],
    "gr_supra_racing_concept_18": ["Toyota GR Supra Racing Concept '18"],
    "gr_supra_rz_20": ["Toyota GR Supra RZ '20"],
    "gr010_hybrid_21": ["TOYOTA GR010 HYBRID '21"],
    "peugeot_vision_gt_14": ["PEUGEOT Vision Gran Turismo8"],
    "bugatti_vision_gt_15": ["Bugatti Vision Gran Turismo8"],
    "porsche_vision_gt_coupe_21": ["Porsche Vision Gran Turismo8"],
    "v8_vantage_gt4m_xx": ["Aston Martin Vantage Gr.4"],
    "vw3_vgt_gt3m_xx": ["Volkswagen GTI Vision Gran Turismo (Gr.3)X"],
    "toyota_ft1_vgt_gt3m_xx": ["Toyota FT-1 Vision Gran Turismo (Gr.3)7"],
    "peugeot_l500r_hv_vision_gt_race_17": [
        "Peugeot L750R HYbrid Vision Gran Turismo, 2017"
    ],
    "Bruce_levens_ford_gt51_coupe_xx": ["Wicked Fabrication GT 51"],
    "bruce_levens_ford_gt51_coupe_xx": ["Wicked Fabrication GT 51"],
    "rcr_civic_xx": ["Garage RCR CivicX"],
    "roadster_shop_rampage_xx": ["Roadster Shop Rampage"],
    "swift_sports_gt4m_xx": [
        "Suzuki Swift Sport KATANA Edition Gr.4"
    ],
    "mercedes_amg_gt_gt3_16": ["Mercedes-AMG GT3 '16"],
    "mercedes_amg_gt_s_14_sc": ["Mercedes-AMG GT Safety Car"],
    "gtr_17_sc": ["Nissan GT-R Safety Car"],
    "mercedes_amg_gt_r_17": ["Mercedes-AMG GT R '17"],
    "mercedes_amg_c63_s_15": ["Mercedes-AMG C 63 S '15"],
    "corvette_zr1_90": ["Chevrolet Corvette ZR-1 (C4) '89"],
    "gac_maverick_71_xx": ["Greening Auto Company Maverick"],
    "mercedes_amg_gt3_evo_19": ["Mercedes-AMG GT3 '20"],
    "bvlgari_vision_gt_23": ["BVLGARI Aluminium Vision Gran Turismo"],
    "elantra_n_tcr_25": ["Hyundai ELANTRA N TC '24"],
    "u9_24": ["Yangwang U9 '24"],
    "_963_24": ["Porsche 963 '24"],
    "_911_turbo_s_20_sc": ["Porsche 911 Turbo S Safety Car (992)"],
    "_190e_evo2_91": ["Mercedes-Benz 190 E 2.5-16 Evolution II '91"],
    "w196r_monopost_55": ["Mercedes-Benz W 196 R '55"],
    "castrol_nsx_00": ["Honda NSX GT500 '00"],
    "caterham_seven_sl_r500_12": ["Caterham Seven Superlight R500 '08"],
    "mark_2_v_97": ["Toyota Mark II Tourer V '97"],
    "chaser_v_97": ["Toyota Chaser Tourer V '97"],
    "ioniq_6_n_25": ["Hyundai IONIQ 6 N '25"],
}

ORPHAN_MESSAGE_OVERRIDES = {
    "Buddy_ContentCar_BrandCentralOwner_normal_ioniq_6_n_25_1020780": [
        "Hyundai IONIQ 6 N '25",
    ],
    "Buddy_ContentCar_BrandCentralOwner_normal_caterham_seven_sl_r500_12_1020041": [
        "Caterham Seven Superlight R500 '08",
    ],
    "Buddy_ContentCar_BrandCentralOwner_normal_f1_w08_eq_17_1020376": [
        "Mercedes-AMG F1 W08 EQ Power+ 2017",
    ],
    "Buddy_ContentCar_BrandCentralOwner_normal_f1_w08_eq_17_le_1020388": [
        "Mercedes-AMG F1 W08 EQ Power+ (Color Variation) 2017",
    ],
    "Buddy_ContentCar_BrandCentralOwner_normal_pininfarina_vision_gt_17_1020365": [
        "Fittipaldi EF7 Vision Gran Turismo by Pininfarina",
    ],
    "Buddy_ContentCar_BrandCentralOwner_normal_mercedes_amg_vision_gt_race_13_lhc_1020404": [
        "Mercedes-Benz AMG Vision Gran Turismo LH Edition",
    ],
    "Buddy_ContentCar_UsedCarOwner_normal_mark_2_v_97_1000781": [
        "Toyota Mark II Tourer V '97",
    ],
    "Buddy_ContentCar_UsedCarOwner_normal_chaser_v_97_1000555": [
        "Toyota Chaser Tourer V '97",
    ],
    "Buddy_ContentCar_Hagerty_normal_rally_037_evo_2_84_1030455": [
        "Lancia Rally 037 Evoluzione 2 '84",
    ],
}

# Some cars are separate liveries or editions but the game reuses the base
# model's description.  These aliases are explicit so a fuzzy nearby match can
# never attach a different model's text.
TEXT_VARIANT_ALIASES = {
    "Gran Turismo Red Bull X2019 25th Anniversary": [
        "Gran Turismo Red Bull X2019 Competition",
    ],
    "MAZDA RX-VISION GT3 CONCEPT Stealth Model": [
        "MAZDA RX-VISION GT3 CONCEPT",
        "MAZDA RX-VISION GT3 CONCEPT8",
    ],
}


def _norm(value: str) -> str:
    value = unicodedata.normalize("NFKD", value)
    value = value.encode("ascii", "ignore").decode().lower()
    value = value.replace("･", " ")
    return re.sub(r"[^a-z0-9]+", " ", value).strip()


def _find_dms_dir() -> Path:
    candidates = sorted(
        p for p in DMS_ROOT.glob("*")
        if p.is_dir() and (p / "CAR_NAME_ALPHABET.dmsb").is_file()
    )
    if not candidates:
        raise FileNotFoundError(f"no extracted DMSB directory under {DMS_ROOT}")
    return candidates[-1]


def _parse_dms(path: Path):
    data = path.read_bytes()
    if data[:4] != b"dmsb":
        raise ValueError(f"{path}: not a DMSB file")
    rows = int.from_bytes(data[8:12], "little")
    table = int.from_bytes(data[0x18:0x20], "little")
    row_size = (table - 0x30) // rows
    if row_size <= 0 or 0x30 + rows * row_size != table:
        raise ValueError(f"{path}: unsupported DMSB row layout")

    def string_at(offset: int) -> str:
        offset &= 0xFFFFFFFF
        start = table + offset
        if start >= len(data):
            return ""
        end = data.find(b"\0", start)
        if end < 0:
            end = len(data)
        return data[start:end].decode("utf-8", "replace")

    def u32(record: bytes, offset: int) -> int:
        return int.from_bytes(record[offset:offset + 4], "little")

    return [
        data[0x30 + i * row_size:0x30 + (i + 1) * row_size]
        for i in range(rows)
    ], string_at, u32


def _clean_text(value: str) -> str:
    value = value.replace("\r\n", "\n").replace("\r", "\n")
    value = re.sub(r"<p\s*/?>", "\n", value, flags=re.IGNORECASE)
    value = re.sub(r"<br\s*/?>", "\n", value, flags=re.IGNORECASE)
    # The localisation payload uses ordinary tags for paragraphs, but also
    # angle-bracket placeholders for inline values such as ``<660cc>`` and
    # ``<700PS>``.  Strip real tag names while preserving those values.
    value = re.sub(
        r"</?[A-Za-z][A-Za-z0-9_-]*(?:\s[^>]*)?>",
        "",
        value,
    )
    value = re.sub(r"<([^<>/]+)>", r"\1", value)
    value = re.sub(r"[\x00-\x08\x0b-\x1f\x7f]", "", value)
    value = re.sub(r"[ \t]+\n", "\n", value)
    value = re.sub(r"([。！？.!?…])[A-Za-z]{1,2}\s*$", r"\1", value)
    value = re.sub(r"\n{3,}", "\n\n", value).strip()
    return value.rstrip("\ufffd").rstrip()


def _rt2_key_path(language: str) -> Path:
    candidates = [
        KEY_ROOT / "rt2_rust" / f"{language}.ks.bin",
        KEY_ROOT / "ktmp" / f"{language}.rt2.ks.bin",
        KEY_ROOT / "rt2" / f"{language}.rt2.ks.bin",
    ]
    for path in candidates:
        if path.is_file():
            return path
    raise FileNotFoundError(f"no RT2 keystream for {language}")


def _label_index(language: str) -> dict[tuple[str, str], str]:
    """Return ``(section, decrypted label) -> text-id`` from a raw RT2."""

    data, _, sections = parse_rt2(RT2_ROOT / f"{language}.rt2")
    key = _rt2_key_path(language).read_bytes()
    index = {}
    for section, _, _, entries in sections:
        for text_id, label_len, _, label_off, _ in entries:
            if not label_len:
                continue
            encrypted = data[label_off:label_off + label_len - 1]
            plain = bytes(
                value ^ key[offset]
                for offset, value in enumerate(encrypted)
                if offset < len(key)
            )
            label = plain.decode("utf-8", "replace").strip("\0\r\n\t ")
            if label:
                index[(section, label)] = f"0x{text_id:08X}"
    return index


def _core_name(car: dict) -> str:
    full = _norm(car["nameEn"])
    brand = _norm(car["brand"])
    if brand and full.startswith(f"{brand} "):
        return full[len(brand) + 1:]
    return full


def _mapping_score(car_key: str, display: str, car: dict) -> float:
    key = _norm(car_key.replace("_", " "))
    shown = _norm(display)
    core = _core_name(car)
    brand = _norm(car["brand"])
    similarity = difflib.SequenceMatcher(None, shown, core).ratio()
    key_tokens = set(key.split())
    core_tokens = set(core.split())
    overlap = len(key_tokens & core_tokens) / max(1, len(key_tokens))
    brand_bonus = 0.3 if brand and brand in key_tokens else 0.0
    return similarity + 0.35 * overlap + brand_bonus


def _resolve_cars(
    car_key: str,
    display: str,
    cars: list[dict],
    by_name: dict[str, list[dict]],
    by_core: dict[str, list[dict]],
) -> tuple[list[dict], str, float, list[dict]]:
    overridden = LOCAL_NAME_OVERRIDES.get(car_key, [])
    if overridden:
        found = []
        for name in overridden:
            found.extend(by_name.get(_norm(name), []))
        found = list({car["id"]: car for car in found}.values())
        if found:
            return found, "dms-car-key-override", 1.0, []

    exact = by_name.get(_norm(display), [])
    if not exact:
        exact = by_core.get(_norm(display), [])
    if exact:
        exact = list({car["id"]: car for car in exact}.values())
        return exact, "exact-display-name", 1.0, []

    ranked = sorted(
        ((_mapping_score(car_key, display, car), car) for car in cars),
        key=lambda item: item[0],
        reverse=True,
    )
    best_score, best = ranked[0]
    second_score = next(
        score for score, car in ranked[1:] if car["id"] != best["id"]
    )
    if best_score >= 0.72 and best_score - second_score >= 0.06:
        return [best], "fuzzy-name", round(best_score, 4), []
    alternates = [
        {
            "id": car["id"],
            "nameEn": car["nameEn"],
            "score": round(score, 4),
        }
        for score, car in ranked[:3]
    ]
    return [], "unresolved", round(best_score, 4), alternates


def build_car_texts(cars: list[dict]) -> dict:
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
    us_labels = _label_index("us")
    by_name: dict[str, list[dict]] = {}
    by_core: dict[str, list[dict]] = {}
    by_id = {}
    for car in cars:
        by_id[car["id"]] = car
        by_name.setdefault(_norm(car["nameEn"]), []).append(car)
        by_core.setdefault(_core_name(car), []).append(car)

    entries: list[dict] = []
    unresolved: list[dict] = []
    orphan_texts: list[dict] = []
    matched_by_category = Counter()
    text_coverage = {}

    for spec in CATEGORY_SPECS:
        rows, string_at, u32 = _parse_dms(dms_dir / spec["file"])
        category_hits = 0
        missing_labels = []
        matched_text_ids = set()
        for row in rows:
            car_key = string_at(u32(row, 0x40))
            message_key = string_at(u32(row, 0x38))
            text_id = us_labels.get((spec["section"], message_key))
            if text_id is None:
                missing_labels.append(message_key)
                continue
            matched_text_ids.add(text_id)
            message_id = u32(row, 0x00)
            display = car_display.get(car_key, "")
            resolved, method, confidence, alternates = _resolve_cars(
                car_key, display, cars, by_name, by_core
            )
            entry = {
                "category": spec["id"],
                "titleZh": spec["titleZh"],
                "titleEn": spec["titleEn"],
                "textId": text_id,
                "messageId": message_id,
                "messageKey": message_key,
                "carKey": car_key,
                "sourceName": display,
                "en": _clean_text(us.get((spec["section"], text_id), "")),
                "zh": _clean_text(zh.get((spec["section"], text_id), "")),
                "matchMethod": method,
                "confidence": confidence,
                "carIds": [car["id"] for car in resolved],
            }
            if resolved:
                category_hits += 1
                matched_by_category[spec["id"]] += 1
                for car in resolved:
                    car.setdefault("texts", {})[spec["id"]] = {
                        "titleZh": spec["titleZh"],
                        "titleEn": spec["titleEn"],
                        "textId": text_id,
                        "carKey": car_key,
                        "en": entry["en"],
                        "zh": entry["zh"],
                    }
            else:
                entry["alternates"] = alternates
                unresolved.append(entry)
            entries.append(entry)
        text_coverage[spec["id"]] = {
            "textRows": sum(
                1 for (section, _) in us_labels if section == spec["section"]
            ),
            "mapped": category_hits,
            "dmsRowsWithoutText": len(missing_labels),
        }
        for (section, message_key), text_id in us_labels.items():
            if section != spec["section"] or text_id in matched_text_ids:
                continue
            orphan_texts.append({
                "category": spec["id"],
                "titleZh": spec["titleZh"],
                "titleEn": spec["titleEn"],
                "textId": text_id,
                "messageKey": message_key,
                "en": _clean_text(us.get((section, text_id), "")),
                "zh": _clean_text(zh.get((section, text_id), "")),
                "reason": "RT2 label has no current DMSB vehicle row",
            })

    specs_by_id = {spec["id"]: spec for spec in CATEGORY_SPECS}
    for orphan in list(orphan_texts):
        names = ORPHAN_MESSAGE_OVERRIDES.get(orphan["messageKey"], [])
        resolved = list({
            car["id"]: car
            for name in names
            for car in by_name.get(_norm(name), [])
        }.values())
        if not resolved:
            continue

        spec = specs_by_id[orphan["category"]]
        entry = {
            "category": spec["id"],
            "titleZh": spec["titleZh"],
            "titleEn": spec["titleEn"],
            "textId": orphan["textId"],
            "messageKey": orphan["messageKey"],
            "carKey": "",
            "sourceName": ", ".join(car["nameEn"] for car in resolved),
            "en": orphan["en"],
            "zh": orphan["zh"],
            "matchMethod": "message-key-override",
            "confidence": 1.0,
            "carIds": [car["id"] for car in resolved],
        }
        entries.append(entry)
        matched_by_category[spec["id"]] += 1
        for car in resolved:
            car.setdefault("texts", {})[spec["id"]] = {
                "titleZh": spec["titleZh"],
                "titleEn": spec["titleEn"],
                "textId": orphan["textId"],
                "carKey": "",
                "en": orphan["en"],
                "zh": orphan["zh"],
            }
        orphan_texts.remove(orphan)

    cars_by_name = {
        _norm(car["nameEn"]): car
        for car in cars
    }
    for target_name, source_names in TEXT_VARIANT_ALIASES.items():
        target = cars_by_name.get(_norm(target_name))
        if target is None:
            continue
        for source_name in source_names:
            source = cars_by_name.get(_norm(source_name))
            if source is None:
                continue
            for category, text in source.get("texts", {}).items():
                target.setdefault("texts", {}).setdefault(
                    category,
                    {
                        **text,
                        "carKey": text.get("carKey", ""),
                        "variantOf": source["nameEn"],
                    },
                )

    for car in cars:
        car.setdefault("texts", {})

    entries.sort(key=lambda item: (item["category"], item["textId"]))
    result = {
        "generatedFrom": "PS5 DMSB + RT2",
        "sources": [
            {
                "id": spec["id"],
                "titleZh": spec["titleZh"],
                "titleEn": spec["titleEn"],
                "dmsb": spec["file"],
                "rt2Section": spec["section"],
            }
            for spec in CATEGORY_SPECS
        ],
        "entries": entries,
        "unresolved": unresolved,
        "orphanTexts": orphan_texts,
        "stats": {
            "entries": len(entries),
            "mappedEntries": len(entries) - len(unresolved),
            "unresolvedEntries": len(unresolved),
            "orphanTextRows": len(orphan_texts),
            "carsWithAnyText": sum(bool(car["texts"]) for car in cars),
            "byCategory": dict(matched_by_category),
            "textCoverage": text_coverage,
        },
    }

    PUBLIC_DIR.mkdir(parents=True, exist_ok=True)
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    payload = json.dumps(result, ensure_ascii=False, separators=(",", ":"))
    (PUBLIC_DIR / "car_texts.json").write_text(payload, encoding="utf-8")
    (REPORT_DIR / "car_texts.json").write_text(payload, encoding="utf-8")

    csv_fields = [
        "category", "textId", "messageId", "carKey", "sourceName",
        "carIds", "matchMethod", "confidence", "en", "zh",
    ]
    for target in (
        PUBLIC_DIR / "car_texts.csv",
        REPORT_DIR / "car_texts.csv",
    ):
        with target.open("w", encoding="utf-8-sig", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=csv_fields)
            writer.writeheader()
            for entry in entries:
                writer.writerow({
                    **{key: entry.get(key, "") for key in csv_fields},
                    "carIds": ";".join(entry["carIds"]),
                })

    review_fields = [
        "category", "textId", "messageId", "carKey", "sourceName",
        "confidence", "en", "zh", "alternates",
    ]
    with (REPORT_DIR / "car_text_mapping_review.csv").open(
        "w", encoding="utf-8-sig", newline=""
    ) as handle:
        writer = csv.DictWriter(handle, fieldnames=review_fields)
        writer.writeheader()
        for entry in unresolved:
            writer.writerow({
                **{key: entry.get(key, "") for key in review_fields},
                "alternates": json.dumps(
                    entry.get("alternates", []), ensure_ascii=False
                ),
            })
    orphan_fields = [
        "category", "textId", "messageKey", "en", "zh", "reason",
    ]
    for target in (
        PUBLIC_DIR / "car_text_orphans.csv",
        REPORT_DIR / "car_text_orphans.csv",
    ):
        with target.open("w", encoding="utf-8-sig", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=orphan_fields)
            writer.writeheader()
            writer.writerows(
                {key: entry.get(key, "") for key in orphan_fields}
                for entry in orphan_texts
            )
    return result


if __name__ == "__main__":
    site = json.loads(
        (PUBLIC_DIR / "cars.json").read_text(encoding="utf-8")
    )
    report = build_car_texts(site["cars"])
    print(json.dumps(report["stats"], ensure_ascii=False, indent=2))
