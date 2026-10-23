#!/usr/bin/env python3
"""Build the static-site dataset (cars + GT Cafe dialogues, EN/ZH)."""

import csv
import json
import re
from collections import Counter, defaultdict
from pathlib import Path

import sys
if hasattr(sys.stdout, "reconfigure"):
    sys.stdout.reconfigure(encoding="utf-8")
sys.path.insert(0, str(Path(__file__).resolve().parent))
from textpick import load_best  # noqa: E402
from car_texts import build_car_texts  # noqa: E402
from cafe_exact import build_exact_cafe  # noqa: E402
from export_unmatched_review import main as export_unmatched_review  # noqa: E402

OUT = Path(r"C:\gt7\gt7\_re\site\public\data\cars.json")
CSV_OUT = Path(r"C:\gt7\gt7\_re\site\public\data\cafe_dialogues.csv")
REPORT_CSV = Path(r"C:\gt7\gt7\_re\reports\cafe_dialogues.csv")
SCENE_CSV_OUT = Path(r"C:\gt7\gt7\_re\site\public\data\cafe_scene_dialogues.csv")
SCENE_REPORT_CSV = Path(r"C:\gt7\gt7\_re\reports\cafe_scene_dialogues.csv")
TEXT_DIRS = [
    Path(r"C:\gt7\gt7\_re\extracted\text_pooled"),
    Path(r"C:\gt7\gt7\_re\extracted\text_decoded"),
]
CAFE = Path(r"C:\gt7\gt7\_re\reports\cafe_dialogues.json")
OFFICIAL = Path(r"C:\gt7\gt7\_re\reports\official_cars.json")

MULTI = [
    "Eckert's Rod & Custom", "Chris Holstrom Concepts", "Greening Auto Company",
    "Wicked Fabrication", "Roadster Shop", "RE Amemiya", "Mercedes-Benz", "Mercedes-AMG",
    "Alfa Romeo", "Aston Martin", "De Tomaso", "Land Rover", "Range Rover",
    "Gran Turismo", "Red Bull", "Rolls-Royce", "Ayrton Senna",
]

BRAND_FIX = {
    "1932": "Ford",
    "Enzo": "Ferrari",
    "Chris": "Chris Holstrom Concepts",
    "GT": "Citroën",
    "Garage": "Garage RCR",
    "Lambo": "Lamborghini",
    "MAZDA3": "Mazda",
    "DAIHATSU": "Daihatsu",
    "HYUNDAI": "Hyundai",
    "LEXUS": "Lexus",
    "MAZDA": "Mazda",
    "BVLGARI": "Bulgari",
    "AFEELA": "Afeela",
}

CAR_TEXT_ALIASES = {
    "Caterham Seven Superlight R500 '08": [
        "the seven is a legend",
        "superlight r500",
    ],
    "Lamborghini Urus '18": [
        "the urus was the first suv designed by lamborghini",
    ],
    "Toyota Chaser Tourer V '97": [
        "sixth-generation chaser",
        "the toyota chaser",
    ],
}

# A small set of Cafe entries mentions multiple cars while discussing one
# subject.  Those entries cannot be resolved safely by substring matching, so
# their scene-level car is fixed explicitly after auditing the surrounding
# control-flow block.  ``None`` means the subject is not determinable from the
# recovered text and the scene must stay out of every car page.
DIALOGUE_CAR_OVERRIDES = {
    "0x000095B2": "Alfa Romeo 4C Launch Edition '14",
    "0x000095BE": "Alfa Romeo 8C Competizione '08",
    "0x00009B33": "Alfa Romeo 155 2.5 V6 TI '93",
    "0x0000A03A": "Alfa Romeo 4C Launch Edition '14",
    "0x0000A1F7": "Alfa Romeo 8C 2900B Touring Berlinetta '38",
    "0x0000A6DB": "Alfa Romeo Giulia GTAm '20",
    "0x0000ADB7": "Alfa Romeo Giulia Sprint GT Veloce '67",
    "0x0000B8D1": "Mazda Demio XD Touring '15",
    "0x0000BAD8": "Alfa Romeo 8C Competizione '08",
    "0x00009660": "Alpine A110 1600S '72",
    "0x00009F68": "Aston Martin V8 Vantage S '15",
    "0x0000A7AD": "Aston Martin Vulcan '16",
    "0x0000B924": "Aston Martin V12 Vantage GT3 '12",
    "0x00009684": "Aston Martin DBR9 GT1 '10",
    "0x00009618": "Lexus RC F GT500 '16",
    "0x00009F4D": "Nissan GT-R LM NISMO '15",
    "0x00009FE9": "Audi TT Cup '16",
    "0x00009B5D": "Audi R8 LMS '15",
    "0x0000B909": "Toyota TS050 - Hybrid '16",
    "0x0000D385": "Porsche 911 Turbo S Leichtbau (964) '93",
    "0x00009B7B": "Ford GT Race Car '18",
    "0x0000AF51": "Ford Mark IV Race Car '67",
    "0x000095C4": "Ford GT '06",
    "0x00009624": "Ferrari 330 P4 '67",
    "0x0000A06C": "Lamborghini Miura P400 Bertone Prototype '67",
    "0x00009576": "Ford Mustang GT Premium Fastback '15",
    "0x00007273": "Toyota Supra RZ '97",
    "0x000072CC": "Nissan Silvia spec-R Aero (S15) '02",
    "0x00007259": "Mitsubishi Lancer Evolution V GSR '98",
    "0x00009DB6": "MINI Cooper S '05",
    "0x000095A0": "Mitsubishi Lancer Evolution Final Edition '15",
    "0x0000ADCF": "Subaru Impreza Sedan WRX STi '04",
    "0x000074CA": "Toyota Supra GT500 '97",
    "0x0000BBBF": "Honda NSX GT500 '00",
    "0x0000C95A": "Nissan Skyline Hard Top 2000GT-R (KPGC10) '70",
    "0x0000A00A": "Honda NSX Type R '92",
    "0x0000A075": "McLaren F1 '94",
    "0x0000A74C": "Plymouth XNR Ghia Roadster '60",
    "0x0000AF8F": "Chevrolet Corvette (C1) '58",
    "0x0000CC7D": "Chevrolet Corvette C8 Stingray '20",
    "0x0000A794": "Shelby Cobra Daytona Coupe '64",
    "0x0000A798": "Shelby Cobra Daytona Coupe '64",
    "0x00007477": "Abarth 500 '09",
    "0x00009BAC": "FIAT Abarth 595 Esseesse '70",
    "0x00009CCC": None,
    "0x00009DA2": "FIAT 500 1.2 8V Lounge SS '08",
    "0x0000A7F3": None,
    "0x0000AF5A": "Dodge Viper SRT GT3-R '15",
    "0x000071FA": "Nissan Skyline GT-R V･spec II Nür (R34) '02",
    "0x00009B75": "Ford GT LM Race Car Spec II8",
    "0x00009B81": "Gran Turismo F1500T-A",
    "0x00009CA5": "IsoRivolta Zagato Vision Gran Turismo",
    "0x00009F3B": "INFINITI CONCEPT Vision Gran Turismo",
    "0x0000A87F": "RE Amemiya FD3S RX-7",
    "0x0000B17B": "Maserati GranTurismo S '08",
    "0x0000B892": "MAZDA RX-VISION GT3 CONCEPT8",
    "0x0000BFDE": "Porsche 962 C '88",
    "0x0000D2D1": "Chevrolet Camaro 1969 Race-Mod",
    "0x00007445": "Porsche 911 Carrera RS Clubsport (993) '95",
    "0x00009840": "Volkswagen 1200 '66",
    "0x0000B1DA": "Toyota GR Corolla MORIZO Edition '22",
    "0x0000B1CE": "Honda Civic Type R Limited Edition (FK8) '20",
    "0x0000954C": "Honda Civic Type R (EK) '97",
}

OFFICIAL_ID_OVERRIDES = {
    "Alpine Vision Gran Turismo 2017": "car3187",
    "Alpine Vision Gran Turismo": "car2112",
    "Audi Vision Gran Turismo": "car3314",
    "BMW Vision Gran Turismo": "car2087",
    "Bugatti Vision Gran Turismo": "car2134",
    "Eckert's Rod & Custom Mach Forty": "car3346",
    "Ferrari Vision Gran Turismo": "car3510",
    "Greening Auto Company Maverick": "car3522",
    "Mazda LM55 Vision Gran Turismo": "car2118",
    "McLaren Ultimate Vision Gran Turismo": "car3315",
    "PEUGEOT Vision Gran Turismo": "car2113",
    "Porsche Vision Gran Turismo": "car3478",
    "SUZUKI Vision Gran Turismo": "car3476",
    "Škoda Vision Gran Turismo": "car3542",
}

CANON = {k.lower(): v for k, v in {
    "bmw": "BMW", "mini": "MINI", "mini-cooper": "MINI", "tvr": "TVR", "ruf": "RUF", "bac": "BAC",
    "ktm": "KTM", "ds": "DS", "fiat": "FIAT", "dmc": "DMC", "amg": "AMG",
    "srt": "SRT", "hks": "HKS", "trd": "TRD", "fpv": "FPV", "aem": "AEM",
    "bbs": "BBS", "greddy": "GReddy", "hyundai": "Hyundai", "daihatsu": "Daihatsu",
    "lexus": "Lexus", "mazda": "Mazda", "nissan": "Nissan", "honda": "Honda",
    "toyota": "Toyota", "subaru": "Subaru", "suzuki": "Suzuki", "audi": "Audi",
    "porsche": "Porsche", "ferrari": "Ferrari", "lamborghini": "Lamborghini",
    "maserati": "Maserati", "mclaren": "McLaren", "bugatti": "Bugatti",
    "chevrolet": "Chevrolet", "dodge": "Dodge", "ford": "Ford", "jaguar": "Jaguar",
    "jeep": "Jeep", "ktm": "KTM", "lancia": "Lancia", "lotus": "Lotus",
    "peugeot": "Peugeot", "renault": "Renault", "tesla": "Tesla", "volvo": "Volvo",
    "volkswagen": "Volkswagen", "abarth": "Abarth", "alpine": "Alpine",
    "aston martin": "Aston Martin", "alfa romeo": "Alfa Romeo",
}.items()}


def clean(s: str) -> str:
    # Markup recovery sometimes leaves one or two payload bytes after the final
    # tag (for example ``...<next id=...>h``).  They are container artefacts,
    # not dialogue.
    last_tag = s.rfind(">")
    if last_tag >= 0:
        tail = s[last_tag + 1:].strip()
        if 0 < len(tail) <= 2:
            punctuation = "".join(ch for ch in tail if ch in ".!?…")
            s = s[:last_tag + 1] + punctuation

    s = re.sub(r"<p[^>]*/>", "\n", s)
    s = re.sub(r"<p[^>]*>", "\n", s)
    s = re.sub(r"<next[^>]*>", "", s)
    s = re.sub(r"<yesno[^>]*>", "", s)
    s = re.sub(r"<sseq[^>]*>", "", s)
    # Some recovered RT2 strings retain the visible payload but lose the
    # opening bracket of a tag (for example, "\ufffdsseq ...").
    s = re.sub(
        r"[^\w\s<]{1,3}\s*(?:sseq|seq|next|yesno|p)\b[^>]*>",
        "",
        s,
        flags=re.IGNORECASE,
    )
    s = re.sub(r"<([^<>/]+)>", r"\1", s)
    s = re.sub(r"^[^\w]{0,3}(?:sseq|seq)[^>]*>", "", s)
    s = re.sub(r"^[^\w]{0,3}p[^\n]{0,80}>", "", s)
    s = s.replace("\r\n", "\n").replace("\r", "\n")
    s = re.sub(r"[\x00-\x08\x0b-\x1f\x7f]", "", s)
    s = re.sub(r"([。！？.!?…])[A-Za-z]{1,2}\s*$", r"\1", s)
    return re.sub(r"\n{3,}", "\n\n", s).strip()


def load(lang: str):
    return load_best(lang)


def norm_name(s: str) -> str:
    s = clean(s)
    m = re.search(r"'\d\d", s)
    if m:
        s = s[:m.end()]
    else:
        s = re.sub(r"(\d{4})\d$", r"\1", s)
        s = s.rstrip(" >/")
    return s.strip()


def brand_of(name: str) -> str:
    for b in MULTI:
        if name.lower().startswith(b.lower()):
            return b
    b = name.split()[0]
    b = BRAND_FIX.get(b, b)
    return CANON.get(b.lower(), b)


SPEAKERS = {
    "Luca": {
        "speaker": "Luca",
        "speakerZh": "卢卡",
        "roleEn": "GT Café owner",
        "roleZh": "GT 咖啡屋主人",
        "avatar": "avatars/luca.jpg",
        "aliases": ["Luca"],
    },
    "Sarah": {
        "speaker": "Sarah Eaton",
        "speakerZh": "莎拉·伊顿",
        "roleEn": "Main guide and race engineer",
        "roleZh": "主向导 / 赛事工程师",
        "avatar": "avatars/sarah.jpg",
        "aliases": ["Sarah Eaton", "Sarah"],
    },
    "Stella": {
        "speaker": "Stella",
        "speakerZh": "斯黛拉",
        "roleEn": "Café regular · compact cars",
        "roleZh": "咖啡屋常客 · 紧凑型车",
        "avatar": "avatars/stella.svg",
        "aliases": ["Stella"],
    },
    "Chris": {
        "speaker": "Chris",
        "speakerZh": "克里斯",
        "roleEn": "Café regular · road-car appraisal",
        "roleZh": "咖啡屋常客 · 公路车鉴赏",
        "avatar": "avatars/chris.svg",
        "aliases": ["Chris"],
    },
    "Jeremy": {
        "speaker": "Jeremy",
        "speakerZh": "杰里米",
        "roleEn": "Café regular · motorsport history",
        "roleZh": "咖啡屋常客 · 赛车史",
        "avatar": "avatars/jeremy.svg",
        "aliases": ["Jeremy"],
    },
    "Freeman Thomas": {
        "speaker": "Freeman Thomas",
        "speakerZh": "弗里曼·托马斯",
        "roleEn": "Former Ford design director; Porsche, Audi",
        "roleZh": "前福特设计总监 · 保时捷/奥迪",
        "avatar": "avatars/designer.svg",
        "aliases": ["Freeman Thomas", "Freeman"],
    },
    "Shiro Nakamura": {
        "speaker": "Shiro Nakamura",
        "speakerZh": "中村史郎",
        "roleEn": "Former Nissan Chief Creative Officer",
        "roleZh": "前日产首席创意官",
        "avatar": "avatars/designer.svg",
        "aliases": ["Shiro Nakamura", "Nakamura"],
    },
    "Hideo Kodama": {
        "speaker": "Hideo Kodama",
        "speakerZh": "儿玉英雄",
        "roleEn": "Former Opel designer",
        "roleZh": "前欧宝设计师",
        "avatar": "avatars/designer.svg",
        "aliases": ["Hideo Kodama", "Kodama"],
    },
    "Fabio Filippini": {
        "speaker": "Fabio Filippini",
        "speakerZh": "法比奥·菲利皮尼",
        "roleEn": "Former Pininfarina design director",
        "roleZh": "前宾尼法利纳设计总监",
        "avatar": "avatars/designer.svg",
        "aliases": ["Fabio Filippini", "Fabio", "Filippini"],
    },
    "Ed Welburn": {
        "speaker": "Ed Welburn",
        "speakerZh": "埃德·韦伯恩",
        "roleEn": "Former GM global design vice president",
        "roleZh": "前通用汽车全球设计副总裁",
        "avatar": "avatars/designer.svg",
        "aliases": ["Ed Welburn", "Welburn"],
    },
    "Tom Matano": {
        "speaker": "Tom Matano",
        "speakerZh": "俣野努",
        "roleEn": "Mazda designer; MX-5 design contributor",
        "roleZh": "马自达设计师 · MX-5 设计参与者",
        "avatar": "avatars/designer.svg",
        "aliases": ["Tom Matano", "Matano"],
    },
    "Hikonori Kagurazaka": {
        "speaker": "Hikonori Kagurazaka",
        "speakerZh": "神乐坂彦则",
        "roleEn": "High-performance car designer",
        "roleZh": "高性能车设计师",
        "avatar": "avatars/designer.svg",
        "aliases": ["Hikonori Kagurazaka", "Kagurazaka"],
    },
    "Kunihisa Ito": {
        "speaker": "Kunihisa Ito",
        "speakerZh": "伊藤邦久",
        "roleEn": "GM, Opel and Ford designer",
        "roleZh": "通用/欧宝/福特汽车设计师",
        "avatar": "avatars/designer.svg",
        "aliases": ["Kunihisa Ito", "Kunihisa"],
    },
    "Ikuo Maeda": {
        "speaker": "Ikuo Maeda",
        "speakerZh": "前田育男",
        "roleEn": "Former Mazda design lead; Kodo design",
        "roleZh": "前马自达设计负责人 · 魂动设计",
        "avatar": "avatars/designer.svg",
        "aliases": ["Ikuo Maeda", "Maeda"],
    },
    "Kim Jones": {
        "speaker": "Kim Jones",
        "speakerZh": "金·琼斯",
        "roleEn": "Fashion designer; Dior Men artistic director",
        "roleZh": "时装设计师 · Dior 男装艺术总监",
        "avatar": "avatars/designer.svg",
        "aliases": ["Kim Jones"],
    },
}


RECURRING = {"Stella", "Chris", "Jeremy"}


def turn_speaker(text: str, guest: dict, is_car_detail: bool) -> dict:
    """Assign a speaker using the dialogue's role in a single scene."""

    body = clean(text)
    guest_name = guest["speaker"]
    if is_car_detail:
        return guest

    if guest_name in RECURRING:
        guest_cue = re.search(
            r"^(?:hi[,!.]?\s+)?(?:nice to meet you|i noticed your cool car|"
            r"i love talking)|^(?:\.\.\.)?(?:th-)?this car|"
            r"^(?:oh man|wow!|why, this is|good to see you both)|"
            r"^about this car here",
            body,
            re.IGNORECASE,
        )
        if guest_cue:
            return guest
        return dict(SPEAKERS["Luca"])

    self_intro = re.search(
        r"\b(?:I'm|I am|My name is|Yes, my name is|Yes! I'm|"
        r"Yes, sorry to surprise|That's right\. But you can call me)\b",
        body,
        re.IGNORECASE,
    )
    if self_intro:
        return guest
    if re.search(
        r"^(?:sir|wait|no no|we're|it's|the pleasure is ours|"
        r"wow, mr\.|mr\. [^,]+, we're)",
        body,
        re.IGNORECASE,
    ):
        return dict(SPEAKERS["Sarah"])
    if re.search(
        r"(?:thank you for (?:that|the|this|sharing)|"
        r"(?:that|this) was (?:a )?(?:very )?(?:interesting|fascinating|impressive))",
        body,
        re.IGNORECASE,
    ):
        return dict(SPEAKERS["Sarah"])
    if re.search(
        r"(?:^(?:no,? )?my pleasure|no need to thank me|don't mention it|"
        r"see you again|until next time|take (?:a|another) good look|"
        r"i hope we can discuss|i look forward to discussing|"
        r"the pleasure was all mine)",
        body,
        re.IGNORECASE,
    ):
        return guest
    if re.search(
        r"(?:congratulations|you just won|won that race|won the race|"
        r"race the other day|rumors of your|news of your)",
        body,
        re.IGNORECASE,
    ):
        return dict(SPEAKERS["Luca"])
    return dict(SPEAKERS["Luca"])


NEXT_ID_RE = re.compile(r"next\s+id=(\d{7})\b", re.IGNORECASE)
CAR_DETAIL_RE = re.compile(
    r"(?:CarDetail|GTCafe_Conv_CarDetail|GTCafe_Walk(?:Side|Front|Rear))",
    re.IGNORECASE,
)


def cafe_rows(us: dict, zh: dict) -> list[dict]:
    rows = []
    for (section, sid), raw in us.items():
        if section != "BuddyGTCafe":
            continue
        rows.append({
            "id": f"0x{int(sid, 16):08X}",
            "number": int(sid, 16),
            "raw": raw,
            "en": clean(raw),
            "zh": clean(zh.get((section, sid), "")),
        })
    rows.sort(key=lambda row: row["number"])
    return rows


def scene_runs(rows: list[dict]) -> list[dict]:
    """Rebuild Cafe scenes from consecutive control-flow ids.

    Cafe text used to be sliced with a fixed ``idx - 6 .. idx + 3`` window.
    That necessarily leaks the previous/next car into a scene.  Each GT7 Cafe
    exchange carries a seven-digit target such as ``1702206``; removing the
    final action number yields one stable scene key for every line in the
    exchange.  Missing targets are filled only inside the same contiguous run.
    """

    labels = []
    for row in rows:
        ids = NEXT_ID_RE.findall(row["raw"])
        labels.append(int(ids[-1]) // 100 if ids else None)

    for idx, label in enumerate(labels):
        if label is not None:
            continue
        previous = next(
            (labels[j] for j in range(idx - 1, -1, -1) if labels[j] is not None),
            None,
        )
        following = next(
            (labels[j] for j in range(idx + 1, len(labels)) if labels[j] is not None),
            None,
        )
        labels[idx] = previous if previous is not None else following

    runs = []
    start = 0
    for idx in range(1, len(rows) + 1):
        if idx == len(rows) or labels[idx] != labels[start]:
            runs.append({
                "sceneId": (
                    f"{labels[start]:05d}-{rows[start]['id'][2:]}"
                    if labels[start] is not None
                    else None
                ),
                "rows": rows[start:idx],
                "position": len(runs),
            })
            start = idx
    return runs


def is_detail_row(row: dict) -> bool:
    text = row["en"]
    return bool(
        CAR_DETAIL_RE.search(row["raw"])
        or (
            re.match(r"^\s*(?:\ufffd)?sseq\b", row["raw"], re.IGNORECASE)
            and len(text) >= 180
        )
    )


def guest_for_run(run: dict) -> dict:
    combined = "\n".join(clean(row["raw"]) for row in run["rows"][:7])
    hits = []
    for meta in SPEAKERS.values():
        if meta["speaker"] in {"Luca", "Sarah"}:
            continue
        for alias in meta["aliases"]:
            match = re.search(
                rf"(?<![A-Za-z]){re.escape(alias)}(?![A-Za-z])",
                combined,
                re.IGNORECASE,
            )
            if match:
                hits.append((match.start(), -len(alias), meta))
    if hits:
        hits.sort(key=lambda hit: (hit[0], hit[1]))
        return dict(hits[0][2])
    return dict(SPEAKERS["Luca"])


def scene_sort_key(row: dict) -> tuple[int, int, int]:
    text = row["en"]
    target_ids = NEXT_ID_RE.findall(row["raw"])
    is_victory_entry = bool(
        target_ids
        and int(target_ids[-1]) % 100 == 1
        and re.search(
            r"(?:congratulations|you (?:just )?won|won (?:a|that|the) race|"
            r"race the other day|rumors of your|news of your|championship victory)",
            text,
            re.IGNORECASE,
        )
    )
    is_closing = bool(re.search(
        r"^(?:thank you|that was|wow, thank you|it was nice talking|"
        r"(?:no,? )?my pleasure|no need to thank|don't mention it|"
        r"i hope we can discuss|i look forward to discussing)",
        text,
        re.IGNORECASE,
    ))
    return (0 if is_victory_entry else 2 if is_closing else 1, row["number"], row["number"])


def run_turns(run: dict, guest: dict) -> list[dict]:
    turns = []
    seen = set()
    for row in sorted(run["rows"], key=scene_sort_key):
        if not row["en"]:
            continue
        dedupe = re.sub(r"\W+", "", row["en"]).lower()
        if dedupe in seen:
            continue
        seen.add(dedupe)
        detail = is_detail_row(row)
        turns.append({
            "id": row["id"],
            "en": row["en"],
            "zh": row["zh"],
            "isCarDetail": detail,
            "sceneId": run["sceneId"],
            "sceneOrder": len(turns) + 1,
            **turn_speaker(row["raw"], guest, detail),
        })
    return turns


def norm_key(s: str) -> str:
    s = s.lower()
    s = re.sub(r"'\d\d", " ", s)
    s = re.sub(r"[^a-z0-9]+", " ", s)
    return " ".join(s.split())


def merge_duplicate_cars(cars: list[dict]) -> list[dict]:
    """Collapse duplicate CarNameLong rows for the same display vehicle.

    The retail tables contain six duplicated names with different internal
    IDs.  Keeping both cards would duplicate the same car page and every
    dialogue attached to it.  The canonical row keeps all source IDs so the
    merge remains auditable.
    """

    groups: dict[tuple[str, str], list[dict]] = {}
    for car in cars:
        groups.setdefault(
            (
                car["brand"].strip().casefold(),
                re.sub(r"\s+", " ", car["nameEn"].strip()).casefold(),
            ),
            [],
        ).append(car)

    merged = []
    for group in groups.values():
        primary = group[0]
        primary["sourceIds"] = sorted(car["id"] for car in group)
        merged.append(primary)
    merged.sort(key=lambda car: (car["brand"].lower(), car["nameEn"].lower()))
    return merged


def choose_scene_car(
    run: dict,
    cars_by_id: dict[str, dict],
    cars_by_name: dict[str, dict],
    legacy_by_id: dict[str, str],
) -> dict | None:
    """Map a scene from its own explanatory text, never from its neighbours."""

    overridden = {
        DIALOGUE_CAR_OVERRIDES[row["id"]]
        for row in run["rows"]
        if row["id"] in DIALOGUE_CAR_OVERRIDES
    }
    if len(overridden) > 1:
        raise ValueError(
            f"conflicting car overrides in scene {run['sceneId']}: {overridden}"
        )
    if overridden:
        car_name = next(iter(overridden))
        if car_name is None:
            return None
        car = cars_by_name.get(norm_key(car_name))
        if car is None:
            raise ValueError(f"unknown car override: {car_name}")
        return car

    run_body = norm_key(" ".join(row["en"] for row in run["rows"]))
    explicit_hits = []
    for car in cars_by_id.values():
        for alias in CAR_TEXT_ALIASES.get(car["nameEn"], []):
            alias_key = norm_key(alias)
            if alias_key and f" {alias_key} " in f" {run_body} ":
                explicit_hits.append((len(alias_key), car))
    if explicit_hits:
        explicit_hits.sort(key=lambda hit: hit[0], reverse=True)
        if len(explicit_hits) == 1 or explicit_hits[0][0] > explicit_hits[1][0]:
            return explicit_hits[0][1]

    detail_rows = [row for row in run["rows"] if is_detail_row(row)]
    if not detail_rows:
        detail_rows = [max(run["rows"], key=lambda row: len(row["en"]))]

    legacy_cars = {
        legacy_by_id[row["id"]]
        for row in detail_rows
        if row["id"] in legacy_by_id
    }
    if len(legacy_cars) == 1:
        car_id = next(iter(legacy_cars))
        return cars_by_id.get(car_id)
    return None


def load_official():
    items = json.loads(OFFICIAL.read_text(encoding="utf-8"))
    by_full = {}
    by_name = {}
    by_id = {}
    logos = {}
    for it in items:
        by_id[it["id"]] = it
        full = norm_key(f"{it['brand']} {it['name']}")
        name = norm_key(it["name"])
        by_full.setdefault(full, it)
        by_name.setdefault(name, it)
        if it.get("logo"):
            logos.setdefault(it["brand"], it["brand"].replace(" ", "_").replace("/", "_") + ".png")
    return items, by_full, by_name, by_id, logos


CAR_NAME_FIXES = {
    "Alpine Vision Gran Turismo 2017a": "Alpine Vision Gran Turismo 2017",
    "Amuse NISMO 380RS Super Leggeraa": "Amuse NISMO 380RS Super Leggera",
    "Aston Martin Vantage Gr.4a": "Aston Martin Vantage Gr.4",
    "Audi e-tron Vision Gran Turismoa": "Audi e-tron Vision Gran Turismo",
    "BMW Vision Gran TurismoC": "BMW Vision Gran Turismo",
    "Bugatti Vision Gran Turismo8": "Bugatti Vision Gran Turismo",
    "GT by Citroën Road CarC": "GT by Citroën Road Car",
    "Dodge SRT Tomahawk GTS-R Vision Gran Turismo[": (
        "Dodge SRT Tomahawk GTS-R Vision Gran Turismo"
    ),
    "Gran Turismo Red Bull X2019 25th Anniversary[": (
        "Gran Turismo Red Bull X2019 25th Anniversary"
    ),
    "Eckert's Rod & Custom Mach FortyT": "Eckert's Rod & Custom Mach Forty",
    "Ferrari Vision Gran Turismo8": "Ferrari Vision Gran Turismo",
    "Ford GT LM Race Car Spec II8": "Ford GT LM Race Car Spec II",
    "Ford GT LM Spec II Test Car8": "Ford GT LM Spec II Test Car",
    "Ford Mustang 2015 American RacerT": "Ford Mustang 2015 American Racer",
    "Garage RCR CivicX": "Garage RCR Civic",
    "Genesis X Gran Berlinetta Vision Gran Turismo ConceptW": (
        "Genesis X Gran Berlinetta Vision Gran Turismo Concept"
    ),
    "Honda Sports Vision Gran TurismoT": "Honda Sports Vision Gran Turismo",
    "HYUNDAI N 2025 Vision Gran Turismo (Gr.1)X": (
        "HYUNDAI N 2025 Vision Gran Turismo (Gr.1)"
    ),
    "McLaren Ultimate Vision Gran Turismo (Gr.1)}": (
        "McLaren Ultimate Vision Gran Turismo (Gr.1)"
    ),
    "MINI Clubman Vision Gran TurismoT": "MINI Clubman Vision Gran Turismo",
    "PEUGEOT Vision Gran Turismo8": "PEUGEOT Vision Gran Turismo",
    "Porsche Vision Gran Turismo8": "Porsche Vision Gran Turismo",
    "Renault Sport Mégane R.S. Trophy Safety Car[": (
        "Renault Sport Mégane R.S. Trophy Safety Car"
    ),
    "Toyota FT-1 Vision Gran Turismoa": "Toyota FT-1 Vision Gran Turismo",
    "Toyota FT-1 Vision Gran Turismo (Gr.3)7": (
        "Toyota FT-1 Vision Gran Turismo (Gr.3)"
    ),
    "SUZUKI Vision Gran Turismo (Gr.3 Version)X": (
        "SUZUKI Vision Gran Turismo (Gr.3 Version)"
    ),
    "Suzuki Swift Sport KATANA Edition Gr.47": (
        "Suzuki Swift Sport KATANA Edition Gr.4"
    ),
    "Volkswagen GTI Roadster Vision Gran Turismo}": (
        "Volkswagen GTI Roadster Vision Gran Turismo"
    ),
    "Volkswagen GTI Vision Gran Turismo (Gr.3)X": (
        "Volkswagen GTI Vision Gran Turismo (Gr.3)"
    ),
    "MAZDA RX-VISION GT3 CONCEPT Stealth ModelX": (
        "MAZDA RX-VISION GT3 CONCEPT Stealth Model"
    ),
    "Toyota AE86 Levin D-Tuneda": "Toyota AE86 Levin D-Tuned",
    "Toyota Supra GT Road Car (JZA80)T": "Toyota Supra GT Road Car (JZA80)",
    "Nissan Silvia spec-R Aero (S15) Touring Car}": (
        "Nissan Silvia spec-R Aero (S15) Touring Car"
    ),
}

CAR_NAME_ZH_FIXES = {
    "Bugatti Veyron Gr.4": "布加迪Veyron组别4",
    "Peugeot L500R HYbrid Vision Gran Turismo, 2017": (
        "标致L500R HYbrid Vision Gran Turismo, 2017"
    ),
    "Peugeot L750R HYbrid Vision Gran Turismo, 2017": (
        "标致L750R HYbrid Vision Gran Turismo, 2017"
    ),
    "RE Amemiya FD3S RX-7": "RE雨宫FD3S RX-7",
}


def repair_car_name(car: dict, by_full: dict, by_name: dict) -> None:
    """Remove one-character localisation artefacts without touching real names."""

    original = car["nameEn"]
    fixed = CAR_NAME_FIXES.get(original)
    if fixed:
        car["nameEn"] = fixed
        car["nameZh"] = CAR_NAME_ZH_FIXES.get(fixed, car.get("nameZh", ""))
        if car["nameZh"][-1:] in "aCT[]}vX":
            car["nameZh"] = car["nameZh"][:-1].rstrip()
        return

    car["nameZh"] = CAR_NAME_ZH_FIXES.get(original, car.get("nameZh", ""))
    if car["nameZh"][-1:] in "[]}":
        car["nameZh"] = car["nameZh"][:-1].rstrip()
    if norm_key(original) in by_full:
        return
    brand_key = norm_key(car["brand"])
    rest = norm_key(original)
    if brand_key and rest.startswith(brand_key + " "):
        rest = rest[len(brand_key) + 1:]
    if norm_key(rest) in by_name:
        return
    if original[-1:] not in "aCT8[]}vWX7":
        return

    stripped = original[:-1].rstrip()
    stripped_rest = norm_key(stripped)
    if brand_key and stripped_rest.startswith(brand_key + " "):
        stripped_rest = stripped_rest[len(brand_key) + 1:]
    hit = by_full.get(norm_key(stripped)) or by_name.get(stripped_rest)
    if hit:
        official = hit["name"]
        if not norm_key(official).startswith(brand_key + " "):
            official = f"{car['brand']} {official}"
        car["nameEn"] = official
        car["nameZh"] = CAR_NAME_ZH_FIXES.get(official, car["nameZh"])
        if car["nameZh"][-1:] in "aCT[]}vX":
            car["nameZh"] = car["nameZh"][:-1].rstrip()


DIALOGUE_SITE_FIELDS = (
    "id", "sceneId", "speaker", "speakerZh", "roleEn", "roleZh",
    "avatar", "en", "zh",
)
SCENE_TURN_SITE_FIELDS = (
    "id", "sceneId", "sceneKey", "sceneOrder", "isCarDetail", "speaker",
    "speakerZh", "roleEn", "roleZh", "avatar", "en", "zh",
)


def select_fields(row: dict, fields: tuple[str, ...]) -> dict:
    return {key: row[key] for key in fields if key in row}


def main() -> None:
    us, zh = load("us"), load("zh")
    rows = cafe_rows(us, zh)
    runs = scene_runs(rows)
    row_to_run = {
        row["id"]: run
        for run in runs
        for row in run["rows"]
    }
    cars = []
    for (sec, sid), text in us.items():
        if sec != "CarNameLong":
            continue
        en = norm_name(text)
        cn = norm_name(zh.get((sec, sid), "")) or en
        cars.append({
            "id": sid,
            "nameEn": en,
            "nameZh": cn,
            "brand": brand_of(en),
            "dialogues": [],
            "conversation": [],
        })
    cars.sort(key=lambda c: (c["brand"].lower(), c["nameEn"].lower()))
    cars = merge_duplicate_cars(cars)

    guest_patterns = [
        r"aren'?t you (?:the )?(?:former )?[^,.]*?([A-Z][a-zA-Z'.-]+(?: [A-Z][a-zA-Z'.-]+)?)\?",
        r"(?:you'?re|you are|this is|meet) ([A-Z][a-zA-Z'.-]+(?: [A-Z][a-zA-Z'.-]+)?)",
        r"([A-Z][a-zA-Z'.-]+)-san",
        r"(?:Mr\.|Ms\.|Mrs\.) ([A-Z][a-zA-Z'.-]+)",
    ]
    skip = {"The", "This", "That", "You", "Your", "Cafe", "Café", "Gran", "Turismo"}

    official, by_full, by_name, by_id, logos = load_official()
    matched = 0
    for c in cars:
        repair_car_name(c, by_full, by_name)
        key = norm_key(c["nameEn"])
        brand_key = norm_key(c["brand"])
        rest = key[len(brand_key) + 1:] if key.startswith(brand_key + " ") else key
        hit = by_full.get(key) or by_name.get(rest) or by_name.get(key)
        if hit is None:
            for prefix, official_id in OFFICIAL_ID_OVERRIDES.items():
                if c["nameEn"].startswith(prefix):
                    hit = by_id.get(official_id)
                    break
        if hit is None:
            toks = set(key.split())
            best = None
            best_score = 0.0
            for it in official:
                t = set(norm_key(it["name"]).split())
                if not t:
                    continue
                score = len(toks & t) / len(t)
                if score > best_score:
                    best, best_score = it, score
            if best is not None and best_score >= 0.62:
                hit = best
        if hit:
            matched += 1
            c["officialId"] = hit["id"]
            c["image"] = f"cars/{hit['id']}.png"
            c["brandLogo"] = logos.get(hit["brand"])
        else:
            c["officialId"] = None
            c["image"] = None
            c["brandLogo"] = None
    print(f"official matches: {matched}/{len(cars)}")

    text_report = build_car_texts(cars)
    print("car text stats:", json.dumps(text_report["stats"], ensure_ascii=False))

    items = json.loads(CAFE.read_text(encoding="utf-8"))
    previous_unmatched_ids = set()
    if OUT.is_file():
        try:
            previous = json.loads(OUT.read_text(encoding="utf-8"))
            previous_unmatched_ids = {
                row["id"] for row in previous.get("unmatched", [])
            }
        except (OSError, ValueError, KeyError):
            previous_unmatched_ids = set()

    exact = build_exact_cafe(cars, items)
    exact_stats = exact["stats"]
    print("exact cafe stats:", json.dumps(exact_stats, ensure_ascii=False))

    def attach_speaker(turn: dict) -> dict:
        metadata = SPEAKERS.get(turn.get("speaker"))
        if metadata is None:
            return turn
        return {
            **turn,
            **{
                key: value
                for key, value in metadata.items()
                if key not in {"aliases", "speaker"}
            },
        }

    for car in cars:
        car["dialogues"] = [
            attach_speaker(row) for row in car.get("dialogues", [])
        ]
        car["conversation"] = [
            attach_speaker(turn) for turn in car.get("conversation", [])
        ]
        for scene in car.get("cafeScenes", []):
            for variant in scene.get("variants", []):
                variant["turns"] = [
                    attach_speaker(turn) for turn in variant.get("turns", [])
                ]

    review_unmatched = [
        row for row in exact["legacyUnmatchedReview"]
        if not previous_unmatched_ids or row["id"] in previous_unmatched_ids
    ]
    unmatched = [attach_speaker(row) for row in review_unmatched]
    mapped_dialogues = len(exact["detailRows"])

    for car in cars:
        car["dialogues"].sort(
            key=lambda d: (
                d.get("messageId") is None,
                d.get("messageId") or 0,
                d["id"],
            )
        )
        turns_by_id = {
            (turn.get("sceneId") or "", turn["id"]): turn
            for turn in car["conversation"]
        }
        car["conversation"] = sorted(
            turns_by_id.values(),
            key=lambda turn: (
                turn.get("sceneId") or "",
                turn.get("sceneOrder", 0),
            ),
        )

    brands = Counter(c["brand"] for c in cars)
    speaker_counts = Counter(
        turn["speaker"] for c in cars for turn in c["conversation"]
    )
    speaker_counts.update(d["speaker"] for d in unmatched)
    unique_conversation_lines = len({
        (turn["id"], turn.get("sceneId") or "")
        for car in cars
        for turn in car["conversation"]
    })
    # cafeScenes duplicate every conversation variant and are already exported
    # in reports/cafe_exact.json and the scene CSV.  Keep them out of the
    # initial site payload so the car index stays responsive.
    site_cars = []
    for car in cars:
        site_car = {
            key: value
            for key, value in car.items()
            if key != "cafeScenes"
        }
        site_car["dialogues"] = [
            select_fields(row, DIALOGUE_SITE_FIELDS)
            for row in car["dialogues"]
        ]
        site_car["conversation"] = [
            select_fields(row, SCENE_TURN_SITE_FIELDS)
            for row in car["conversation"]
        ]
        site_cars.append(site_car)
    payload = {
        "generated": "GT7 PS5 dump",
        "cars": site_cars,
        "brands": [{"name": b, "count": n} for b, n in sorted(brands.items())],
        "unmatched": unmatched,
        "cafeExactStats": exact_stats,
        "carTextSources": text_report["sources"],
        "carTextStats": text_report["stats"],
        "speakers": sorted(
            (
                {
                    "id": key,
                    "en": meta["speaker"],
                    "zh": meta["speakerZh"],
                    "roleEn": meta["roleEn"],
                    "roleZh": meta["roleZh"],
                    "avatar": meta["avatar"],
                    "count": speaker_counts.get(key, 0),
                }
                for key, meta in SPEAKERS.items()
                if speaker_counts.get(key, 0)
            ),
            key=lambda speaker: (-speaker["count"], speaker["en"]),
        ),
        "cast": [
            {
                "id": key,
                "en": SPEAKERS[key]["speaker"],
                "zh": SPEAKERS[key]["speakerZh"],
                "roleEn": SPEAKERS[key]["roleEn"],
                "roleZh": SPEAKERS[key]["roleZh"],
                "avatar": SPEAKERS[key]["avatar"],
                "count": speaker_counts.get(key, 0),
            }
            for key in (
                "Luca", "Sarah", "Stella", "Chris", "Jeremy",
                "Freeman Thomas", "Shiro Nakamura", "Hideo Kodama",
                "Fabio Filippini", "Ed Welburn", "Tom Matano",
                "Hikonori Kagurazaka", "Kunihisa Ito", "Ikuo Maeda", "Kim Jones",
            )
        ],
        "stats": {
            "cars": len(cars),
            "withDialogue": sum(1 for c in cars if c["dialogues"]),
            "dialogues": len(items),
            "mappedDialogues": mapped_dialogues,
            "conversationLines": unique_conversation_lines,
            "unmatched": 0,
            "legacyReview": len(review_unmatched),
            "legacyReviewResolved": sum(
                bool(row.get("resolved")) for row in review_unmatched
            ),
        },
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(payload, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    rows = []
    for car in cars:
        rows.extend((car["nameEn"], d) for d in car["dialogues"])
    deduped_rows = {}
    for car_name, dialogue in rows:
        deduped_rows.setdefault(dialogue["id"], (car_name, dialogue))
    rows = list(deduped_rows.values())
    rows.sort(key=lambda row: row[1]["id"])
    for csv_path in (CSV_OUT, REPORT_CSV):
        with csv_path.open("w", encoding="utf-8-sig", newline="") as fh:
            writer = csv.DictWriter(
                fh,
                fieldnames=[
                    "id", "scene_id", "car", "speaker", "speaker_zh",
                    "role_zh", "en", "zh",
                ],
            )
            writer.writeheader()
            for car_name, d in rows:
                writer.writerow({
                    "id": d["id"],
                    "scene_id": d.get("sceneId", ""),
                    "car": car_name,
                    "speaker": d["speaker"],
                    "speaker_zh": d["speakerZh"],
                    "role_zh": d["roleZh"],
                    "en": d["en"],
                    "zh": d["zh"],
                })
    unmatched_csv_paths = (
        OUT.parent / "unmatched_dialogues.csv",
        Path(r"C:\gt7\gt7\_re\reports\unmatched_dialogues.csv"),
    )
    for csv_path in unmatched_csv_paths:
        with csv_path.open("w", encoding="utf-8-sig", newline="") as fh:
            writer = csv.DictWriter(
                fh,
                fieldnames=[
                    "id", "scene_id", "candidate_car", "speaker",
                    "speaker_zh", "resolved_car_en", "resolved_car_zh",
                    "resolution", "scene_keys", "en", "zh",
                ],
            )
            writer.writeheader()
            for d in sorted(unmatched, key=lambda item: item["id"]):
                writer.writerow({
                    "id": d["id"],
                    "scene_id": d.get("sceneId", ""),
                    "candidate_car": d.get("car") or "",
                    "speaker": d["speaker"],
                    "speaker_zh": d["speakerZh"],
                    "resolved_car_en": d.get("resolvedCarNameEn", ""),
                    "resolved_car_zh": d.get("resolvedCarNameZh", ""),
                    "resolution": d.get("resolution", ""),
                    "scene_keys": ";".join(d.get("sceneKeys", [])),
                    "en": d["en"],
                    "zh": d["zh"],
                })
    scene_rows = []
    for car in cars:
        scene_rows.extend((car["nameEn"], turn) for turn in car["conversation"])
    deduped_scene_rows = {}
    for car_name, turn in scene_rows:
        deduped_scene_rows.setdefault(
            (turn["id"], turn.get("sceneId") or ""),
            (car_name, turn),
        )
    scene_rows = list(deduped_scene_rows.values())
    scene_rows.sort(key=lambda row: row[1]["id"])
    for csv_path in (SCENE_CSV_OUT, SCENE_REPORT_CSV):
        with csv_path.open("w", encoding="utf-8-sig", newline="") as fh:
            writer = csv.DictWriter(
                fh,
                fieldnames=[
                    "id", "scene_id", "scene_order", "car", "speaker",
                    "speaker_zh", "role_zh", "is_car_detail", "en", "zh",
                ],
            )
            writer.writeheader()
            for car_name, turn in scene_rows:
                writer.writerow({
                    "id": turn["id"],
                    "scene_id": turn.get("sceneId", ""),
                    "scene_order": turn.get("sceneOrder", ""),
                    "car": car_name,
                    "speaker": turn["speaker"],
                    "speaker_zh": turn["speakerZh"],
                    "role_zh": turn["roleZh"],
                    "is_car_detail": turn.get("isCarDetail", True),
                    "en": turn["en"],
                    "zh": turn["zh"],
                })
    export_unmatched_review()
    print(json.dumps(payload["stats"], ensure_ascii=False))
    print("brands:", len(brands))
    print("brand list:", [b for b, _ in sorted(brands.items())][:60])


if __name__ == "__main__":
    main()
