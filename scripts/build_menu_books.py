#!/usr/bin/env python3
"""Build the GT7 Cafe menu-book research dataset.

The menu-book identity, order, strings, collector levels and Extra Menu car
lists come from the local PS5 DMSB/RT2 data. A small set of Extra Menu stories
do not embed their CC number in the RT2 string, so those three-car lists are
matched from the published menu number and then resolved against the game's
own car-name table.

Main and Bonus Menu reward text is editorial metadata from the published
menu-book tables. It is kept separate from exact DMSB fields in ``sources``.
"""

from __future__ import annotations

import csv
import json
import re
import sys
import unicodedata
from pathlib import Path

ROOT = Path(r"C:\gt7\gt7")
REPO = ROOT / "_re"
SITE = REPO / "site" / "public"
CONTENTS = ROOT / "contents"
DATA_DIR = SITE / "data"
MENU_COVER_DIR = SITE / "menu_covers"
REPORT_DIR = REPO / "reports"

sys.path.insert(0, str(REPO / "scripts"))

from car_texts import (  # noqa: E402
    _clean_text,
    _find_dms_dir,
    _label_index,
    _parse_dms,
)
from textpick import load_best  # noqa: E402


TABLE = b"P6MH85C24L91ZTIYURQ7NKJ0OSBWDAVGX3FE"
FLAG_BOUNDARY = 0x81BF_0FFF


MAIN_COLLECTION_CARS = {
    1: [
        "Toyota Aqua S '11",
        "Honda Fit Hybrid '14",
        "Mazda Demio XD Touring '15",
    ],
    3: [
        "Fiat 500 F '68",
        "MINI Cooper S '65",
        "Volkswagen 1200 '66",
    ],
    5: [
        "Suzuki Swift Sport '17",
        "Honda Civic Type R (EK) '98",
        "Honda Integra Type R (DC2) '98",
    ],
    7: [
        "MINI Cooper S '05",
        "Abarth 500 '09",
        "Volkswagen Polo GTI '14",
    ],
    10: [
        "Nissan Silvia Q's (S13) '88",
        "Mazda Eunos Roadster (NA) '89",
        "Toyota Corolla Levin 1600GT APEX (AE86) '83",
    ],
    11: [
        "DS 3 Racing '11",
        "Peugeot 208 GTi by Peugeot Sport '14",
        "Renault Clio R.S. 220 Trophy '16",
    ],
    14: [
        "Ford Mustang Boss 429 '69",
        "Ford Mustang Mach 1 '71",
        "Ford Mustang GT Premium Fastback '15",
    ],
    15: [
        "Chevrolet Camaro SS '16",
        "Chevrolet Camaro Z28 '69",
        "Chevrolet Camaro ZL1 1LE Package '18",
    ],
    18: [
        "Nissan Silvia spec-R Aero (S15) '02",
        "Nissan Skyline GT-R V-spec II (R32) '94",
        "Nissan Fairlady Z Version S (Z33) '07",
    ],
    19: [
        "Subaru WRX STI Type S '14",
        "Mitsubishi Lancer Evolution Final Edition '15",
        'Toyota GR Yaris RZ "High Performance" \'20',
    ],
    22: [
        "BMW M3 Sport Evolution '89",
        "BMW M3 '03",
        "BMW M3 '07",
    ],
    23: [
        "Alfa Romeo MiTo 1.4 T Sport '09",
        "Alfa Romeo 4C '14",
        "Alfa Romeo 8C Competizione '08",
    ],
    26: [
        "Ford Focus RS '18",
        "Ford GT '06",
        "Ford F-150 SVT Raptor '11",
    ],
    27: [
        "Chevrolet Corvette Stingray (C7) '14",
        "Chevrolet Corvette Stingray Convertible (C3) '69",
        "Chevrolet Corvette ZR-1 (C4) '89",
    ],
    31: [
        "Porsche 911 Turbo (930) '81",
        "Porsche 911 Carrera RS (964) '92",
        "Porsche 911 Carrera RS (993) '95",
    ],
    32: [
        "Toyota Supra 3.0GT Turbo A '88",
        "Toyota Supra RZ '97",
        "Toyota GR Supra RZ '20",
    ],
    34: [
        "Mercedes-AMG C 63 S '15",
        "Mercedes-Benz SLS AMG '10",
        "Mercedes-AMG GT S '15",
    ],
    35: [
        "Nissan Skyline GT-R V-spec (R33) '97",
        "Nissan Skyline GT-R V-spec II Nur (R34) '02",
        "Nissan GT-R Premium Edition '17",
    ],
    37: [
        "Peugeot 205 Turbo 16 Evolution 2 '86",
        "Audi Sport quattro S1 Pikes Peak '87",
        "Ford Focus Gr.B Rally Car",
    ],
    38: [
        "Ferrari 308 GTB '75",
        "Ferrari 458 Italia '09",
        "Ferrari F12berlinetta '12",
    ],
}

# RT2 story strings for these menus do not include a CCxxx filename token.
# The first three car IDs are still explicit in the published Extra Menu
# ordering and are resolved through CAR_NAME_ALPHABET below.
EXTRA_CAR_NAME_FALLBACK = {
    6: [
        "Fiat Abarth 595 Esseesse '70",
        "Abarth 500 '09",
        "Autobianchi A112 Abarth '85",
    ],
    7: [
        "Nissan GT-R NISMO GT500 '16",
        "Lexus RC F GT500 '16",
        "Honda NSX CONCEPT-GT '16",
    ],
    11: [
        "Nissan SILVIA K's Dia Selection (S13) '90",
        "Nissan Silvia K's Type S (S14) '94",
        "Nissan SILVIA spec-R AERO (S15) '02",
    ],
    17: [
        "Porsche 911 Carrera RS (901) '73",
        "Porsche 911 Carrera RS (964) '92",
        "Porsche 911 Carrera RS Club Sport (993) '95",
    ],
    18: [
        "Honda RA272 '65",
        "Honda S800 '66",
        "McLaren MP4/4 '88",
    ],
    23: [
        "BMW M3 Sport Evolution '89",
        "Ford Sierra RS 500 Cosworth '87",
        "Nissan GT-R NISMO (R32) '90",
    ],
    22: [
        "Greening Auto Company Maverick",
        "Eckert's Rod & Custom Mach Forty",
        "GReddy Fugu Z",
    ],
    26: [
        "Maserati A6GCS/53 Spyder '54",
        "Maserati Merak SS '80",
        "Maserati MC20 '20",
    ],
    29: [
        "Porsche 959 '87",
        "Porsche Carrera GT '04",
        "Porsche 918 Spyder '13",
    ],
    35: [
        "Dodge Charger R/T 426 Hemi '68",
        "Dodge Viper GTS '02",
        "Dodge Challenger SRT Demon '18",
    ],
    36: [
        "Lamborghini Miura P400 Bertone Prototype '67",
        "Lamborghini Countach LP400 '74",
        "Lamborghini Aventador LP 700-4 '11",
    ],
    38: [
        "Ferrari 365 GTB4 '71",
        "Ferrari Testarossa '91",
        "Ferrari LaFerrari '13",
    ],
    39: [
        "Honda NSX GT500 '00",
        "Nissan GT-R GT500 '99",
        "Toyota Supra GT500 '97",
    ],
    40: [
        "McLaren F1 '94",
        "McLaren MP4-12C '10",
        "McLaren Ultimate Vision Gran Turismo",
    ],
    42: [
        "Ferrari F40 '92",
        "Ferrari F50 '95",
        "Enzo Ferrari '02",
    ],
    46: [
        "Mercedes-Benz Unimog Type 411 '62",
        "Mercedes-Benz 300 SL Coupe '54",
        "Mercedes-Benz W 196 R '55",
    ],
    50: [
        "Audi TT Coupe 3.2 quattro '03",
        "Audi TTS Coupé '09",
        "Audi TTS Coupe '14",
    ],
}

FORCE_EXTRA_CAR_FALLBACK = {11, 22, 46, 50}

MAIN_COLLECTION_NAMES_EN = {
    1: "Japanese Compact Cars",
    3: "European Classic Compacts",
    5: "Japanese FF Sports Cars",
    7: "European Hot Hatches",
    10: "Japanese FR Sports Cars",
    11: "French Hot Hatches",
    14: "Mustang",
    15: "Camaro",
    18: "Nissan Sports Cars",
    19: "Japanese Rally Base Cars",
    22: "BMW M3",
    23: "Alfa Romeo",
    26: "Ford",
    27: "Chevrolet",
    31: "Porsche 911",
    32: "Supra",
    34: "AMG",
    35: "Nissan GT-R",
    37: "Gr.B Rally Cars",
    38: "Ferrari",
}

BONUS_TITLES_EN = {
    40: "Lightweight K-Cup",
    41: "Vision Gran Turismo Trophy",
    42: "Gr.1 Prototype Series",
    43: "Enter the Neo-Classic Competition",
    44: "Enter the Hyper Car Parade",
    45: "The Silvia Sisters",
    46: "Historic Sports Car Masters",
    47: "Super Formula",
    48: "World Touring Car 900",
    49: "X2019 Nations Cup",
    50: "Jimny Cup",
    51: "Evolution Meeting",
}

EXTRA_NAMES_EN = [
    "Toyota 86",
    "Honda Type R",
    "Rotary Engine",
    "Nissan Fairlady Z",
    "Dodge Viper",
    "Abarth",
    "Gr.2 Racing Cars",
    "Corvette",
    "Mid-engine Porsche",
    "Honda NSX",
    "Nissan Silvia",
    "Renault",
    "Nissan Skyline GT-R",
    "Straight-Six BMWs",
    "Bugatti",
    "Honda Race Cars",
    "Porsche 911 Carrera RS",
    "Toyota Family Car",
    "Ford GT",
    "Shelby",
    "Alpine",
    "SEMA GT Awards",
    "Group A",
    "Subaru WRX",
    "Mitsubishi Lancer Evolution",
    "Maserati",
    "Aston Martin",
    "Kei Cars",
    "Porsche Supercars",
    "GR",
    "Road-Going Racers",
    "Lexus",
    "Red Bull X Series",
    "Nissan Motorsports",
    "Dodge",
    "Lamborghini",
    "Jaguar",
    "Ferrari 12-Cylinder Engines",
    "Racers of Japanese GTs",
    "McLaren",
    "Audi",
    "Ferrari Speciale",
    "Group C",
    "Peugeot",
    "Suzuki",
    "Mercedes-Benz",
    "Japanese Racing Pioneers",
    "Alfa Romeo Legends",
    "Legendary Road Racers",
    "Audi TT",
    "Mazda",
    "Hyundai N",
    "Muscle Cars",
    "Hybrid Prototypes",
]

TEXT_REPAIRS_EN = {
    "w�th": "with",
    "mig?ty": "mighty",
    "Am�rica": "America",
    "Amrica's": "America's",
    "t+e norm": "the norm",
    "performan e": "performance",
    "hi�tory": "history",
    "achieve�": "achieved",
    "i� seems": "it seems",
    "4ith": "with",
}

TEXT_REPAIRS_ZH = {
    "闻名�Ǎ于世": "闻名于世",
    "锋利���条": "锋利线条",
    "一�就": "一眼就",
    "连���四年": "连续四年",
    "中��布局": "中置布局",
    "型�之一": "车型之一",
    "�3�为": "成为",
    "让人动容": "令人动容",
    "现代的“N”品ތ": "现代的“N”品牌",
    "更新演变 画卷": "更新演变的画卷",
}

EXTRA_CAR_REWARD_OVERRIDES = {
    2: ("NSX Gr.B Rally Car", "NSX Gr.B Rally Car"),
    4: ("GReddy Fugu Z", "GReddy Fugu Z"),
    16: ("SF19 Super Formula / Honda '19", "SF19 Super Formula / Honda '19"),
    37: ("Jaguar D-Type '54", "Jaguar D-Type '54"),
}

MAIN_REWARDS = {
    1: ("1★ Roulette Ticket", "1★ 轮盘奖券"),
    2: ("Goodwood Motor Circuit, Brands Hatch", "古德伍德赛车场、布兰兹哈奇赛道"),
    3: ("1★ Roulette Ticket, High Speed Ring", "1★ 轮盘奖券、高速环状赛道"),
    4: ("Japanese Gift Car, Tsukuba, Suzuka, Brand Central", "日本车赠车、筑波赛道、铃鹿赛道、品牌购物"),
    5: ("1★ Roulette Ticket", "1★ 轮盘奖券"),
    6: ("Alsace, Sardegna Road Track", "阿尔萨斯、撒丁岛公路赛道"),
    7: ("Scapes, GT Auto", "风景、GT Auto"),
    8: ("Tokyo Expressway", "东京高速公路"),
    9: ("Japanese Gift Car, 1★ Roulette Ticket, Multiplayer, Sport", "日本车赠车、1★ 轮盘奖券、多人模式、运动模式"),
    10: ("Deep Forest Raceway", "深邃森林赛道"),
    11: ("2★ Roulette Ticket", "2★ 轮盘奖券"),
    12: ("General Motors Gift Car, Missions Pavilion", "通用汽车赠车、任务展馆"),
    13: ("Blue Moon Bay, Laguna Seca, Willow Springs", "蓝月湾、拉古纳塞卡、柳泉赛道"),
    14: ("Daytona International Speedway", "戴通纳国际赛道"),
    15: ("2★ Roulette Ticket", "2★ 轮盘奖券"),
    16: ("Trial Mountain", "山岳考验赛道"),
    17: ("Nissan Gift Car, Fuji, Autopolis, Legend Cars", "日产赠车、富士赛道、奥托波利斯、传奇车商"),
    18: ("Mount Panorama, Kyoto Driving Park", "全景山赛道、京都驾驶公园"),
    19: ("3★ Roulette Ticket", "3★ 轮盘奖券"),
    20: ("3★ Roulette Ticket", "3★ 轮盘奖券"),
    21: ("BMW Gift Car, Red Bull Ring, Dragon Trail, Nürburgring", "BMW赠车、红牛环、龙之径、纽博格林"),
    22: ("3★ Roulette Ticket", "3★ 轮盘奖券"),
    23: ("3★ Roulette Ticket", "3★ 轮盘奖券"),
    24: ("Autodromo Nazionale Monza", "蒙扎国家赛车场"),
    25: ("General Motors Gift Car, Colorado Springs", "通用汽车赠车、科罗拉多泉"),
    26: ("Special Stage Route X", "特别关卡 X 路线"),
    27: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
    28: ("Autodromo de Interlagos", "英特拉格斯赛道"),
    29: ("Porsche Gift Car, 4★ Roulette Ticket", "保时捷赠车、4★ 轮盘奖券"),
    30: ("Circuit de Spa-Francorchamps", "斯帕-弗朗科尔尚赛道"),
    31: ("Circuit de Sainte-Croix", "圣十字赛道"),
    32: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
    33: ("Audi Gift Car, Circuit de Barcelona-Catalunya", "奥迪赠车、巴塞罗那-加泰罗尼亚赛道"),
    34: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
    35: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
    36: ("Ferrari Gift Car, Fishermans Ranch, Sardegna Windmills", "法拉利赠车、渔夫牧场、撒丁岛风车"),
    37: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
    38: ("24 Heures du Mans Racing Circuit", "勒芒 24 小时赛道"),
    39: ("High-End Gift Car, 6★ Roulette Ticket, ending", "高端赠车、6★ 轮盘奖券、结局"),
}

BONUS_REWARDS = {
    40: ("Radical SR3 SL '13 Gift Car", "Radical SR3 SL '13 赠车"),
    41: ("6★ Roulette Ticket", "6★ 轮盘奖券"),
    42: ("6★ Roulette Ticket", "6★ 轮盘奖券"),
    43: ("5★ Roulette Ticket", "5★ 轮盘奖券"),
    44: ("6★ Engine Ticket", "6★ 引擎奖券"),
    45: ("6★ Parts Ticket", "6★ 零件奖券"),
    46: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
    47: ("6★ Roulette Ticket", "6★ 轮盘奖券"),
    48: ("5★ Roulette Ticket", "5★ 轮盘奖券"),
    49: ("6★ Roulette Ticket", "6★ 轮盘奖券"),
    50: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
    51: ("4★ Roulette Ticket", "4★ 轮盘奖券"),
}

EXTRA_WEB_SUPPLEMENTS = {
    55: {
        "internalCode": "CC113",
        "collectionNameEn": "High-Performance EVs",
        "collectionNameZh": "高性能电动车",
        "descriptionEn": (
            "Collect the three specified high-performance electric vehicles "
            "to complete this Extra Menu."
        ),
        "descriptionZh": "收集指定三款高性能电动车，完成这本追加菜单。",
        "rewardEn": "5★ Roulette Ticket",
        "rewardZh": "5★ 轮盘奖券",
        "collectorLevel": 70,
        "cars": [
            "Polestar 5 Performance '26",
            "Xiaomi SU7 Ultra '25",
            "Yangwang U9 '24",
        ],
        "sourceUrl": (
            "https://www.skymac.org/GT7-Cafe/"
            "Menu-113-Collection-Vehicules-electriques-hautes-performances.htm"
        ),
    },
    56: {
        "internalCode": "CC114",
        "collectionNameEn": "Tuned GT-Rs",
        "collectionNameZh": "改装 GT-R",
        "descriptionEn": (
            "Collect the three specified specially tuned Nissan GT-R models "
            "to complete this Extra Menu."
        ),
        "descriptionZh": "收集指定三款经过特别改装的Nissan GT-R，完成这本追加菜单。",
        "rewardEn": "6★ Roulette Ticket",
        "rewardZh": "6★ 轮盘奖券",
        "collectorLevel": 80,
        "cars": [
            "NISMO 400R '95",
            "NISMO R34 GT-R Z-tune '05",
            "Mine's BNR34 GT-R N1 base",
        ],
        "sourceUrl": (
            "https://www.skymac.org/GT7-Cafe/"
            "Menu-114-Collection-GT-R-preparees.htm"
        ),
    },
    57: {
        "internalCode": "CC115",
        "collectionNameEn": "Le Mans Winners",
        "collectionNameZh": "勒芒冠军赛车",
        "descriptionEn": (
            "Collect the three specified Le Mans-winning race cars to "
            "complete this Extra Menu."
        ),
        "descriptionZh": "收集指定三款勒芒冠军赛车，完成这本追加菜单。",
        "rewardEn": "6★ Roulette Ticket",
        "rewardZh": "6★ 轮盘奖券",
        "collectorLevel": 90,
        "cars": [
            "Ferrari 499P '23",
            "McLaren F1 GTR - BMW '95",
            "Ford Mark IV Race Car '67",
        ],
        "sourceUrl": (
            "https://www.skymac.org/GT7-Cafe/"
            "Menu-115-Collection-Championnes-du-Mans.htm"
        ),
    },
}


def _norm(value: str) -> str:
    value = unicodedata.normalize("NFKD", value)
    value = value.encode("ascii", "ignore").decode().lower()
    value = value.replace("’", "'").replace("‘", "'")
    return re.sub(r"[^a-z0-9]+", " ", value).strip()


def _fnv1a(data: bytes) -> int:
    value = 0x811C9DC5
    for byte in data:
        value ^= byte
        value = (value * 16_777_619) & 0xFFFFFFFF
    return value


def _cache_key(value: int) -> str:
    def digit(index: int) -> str:
        current = value
        for _ in range(index):
            current //= 36
        return chr(TABLE[current % 36])

    flag = TABLE[1] if value >= FLAG_BOUNDARY else TABLE[0]
    return (
        f"{digit(0)}/{digit(1)}/"
        f"{chr(flag)}{digit(5)}{digit(4)}{digit(3)}{digit(2)}"
    )


def _asset_source(asset_path: str) -> tuple[str, str]:
    no_lead = asset_path.lstrip("/")
    variants = dict.fromkeys([
        asset_path,
        no_lead,
        asset_path.lower(),
        no_lead.lower(),
    ])
    for variant in variants:
        rel = _cache_key(_fnv1a(variant.encode("utf-8")))
        if (CONTENTS / rel).is_file():
            return rel, "present"
    rel = _cache_key(_fnv1a(asset_path.encode("utf-8")))
    return rel, "missing"


def _visible_text(text: str) -> str:
    """Remove control markup before comparing RT2 decoding candidates."""

    text = text.replace("\r\n", "\n").replace("\r", "\n")
    if "sseq" in text[:120]:
        end = text.find(">", text.find("sseq"))
        if end >= 0:
            text = text[end + 1 :]
    text = re.sub(r"<[^>]*>", "\n", text)
    text = re.sub(
        r"(?:[^\s]*?)\bs?cmd\s*=\s*[\"']?[^>\n]*>?",
        "\n",
        text,
        flags=re.IGNORECASE,
    )
    text = _clean_text(text)
    text = text.replace("\ufffd", "")
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip()


def _visible_quality(text: str) -> tuple[int, int, int]:
    readable = sum(ch.isalnum() for ch in text)
    replacement = text.count("\ufffd")
    suspicious = len(re.findall(r"[^\w\s.,!?;:'\"()\-–—…/&%+★\u3400-\u9fff]", text))
    return (readable - replacement * 1000 - suspicious * 8, -suspicious, len(text))


def load_visible_texts(language: str) -> dict[tuple[str, str], str]:
    """Pick the cleanest candidate by what users actually see."""

    directories = [
        REPO / "extracted" / "text_decoded",
        REPO / "extracted" / "text_pooled",
        REPO / "extracted" / "text_all",
        REPO / "extracted" / "text_patch_decoded",
    ]
    aliases = {"us": ("us", "gb"), "zh": ("cn", "zh")}.get(
        language, (language,)
    )
    candidates: dict[tuple[str, str], list[tuple[tuple[int, int, int], str]]] = {}
    for directory in directories:
        for alias in aliases:
            path = directory / f"{alias}.csv"
            if not path.is_file():
                continue
            with path.open(encoding="utf-8-sig", newline="") as handle:
                for row in csv.DictReader(handle):
                    key = (row["section"], row["id"])
                    visible = _visible_text(row["text"])
                    if visible:
                        raw_replacements = row["text"].count("\ufffd")
                        quality = _visible_quality(visible)
                        quality = (
                            quality[0] - raw_replacements * 1000,
                            quality[1],
                            quality[2],
                        )
                        candidates.setdefault(key, []).append(
                            (quality, visible)
                        )
    return {
        key: max(values, key=lambda item: item[0])[1]
        for key, values in candidates.items()
    }


def sanitize_text(text: str) -> str:
    """Remove retail RT2 recovery debris from visible strings."""

    text = text.replace("\r\n", "\n").replace("\r", "\n")
    text = re.sub(r"锟斤拷|锟?斤拷", "", text)
    text = re.sub(r"[\x00-\x08\x0b-\x1f\x7f]", "", text)
    text = re.sub(r"<[^>]*>", "\n", text)
    text = re.sub(r"\?{2,}", "…", text)
    text = re.sub(r"(?<=\w)\?(?=\w)", "", text)
    text = text.replace("名?", "名号")
    text = re.sub(r"([.!?。！？])\s+\?\s*$", r"\1", text)
    for source, target in {**TEXT_REPAIRS_EN, **TEXT_REPAIRS_ZH}.items():
        text = text.replace(source, target)
    text = text.replace("\ufffd", "")
    # Recovery sometimes retains one or two codec bytes after markup or before
    # a trailing quote. Remove only isolated, low-information residue.
    text = re.sub(
        r"[\u0080-\u009f\u0370-\u05ff\u0600-\u08ff"
        r"\u2500-\u25ff]+",
        "",
        text,
    )
    text = re.sub(
        r"([.!?。！？…”\"'])\s*[A-Za-z0-9%/+<>=?]{1,4}\s*$",
        r"\1",
        text,
    )
    text = re.sub(r"[ \t]+\n", "\n", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return text.strip()


def clean_zh_label(text: str) -> str:
    """Remove the one-byte localisation debris seen on many PS5 labels."""

    text = sanitize_text(text)
    text = text.replace("名?", "名号")
    if text.endswith("组别") and "组别" in text:
        return text
    text = re.sub(r"(?<=[\u4e00-\u9fff])[A-Za-z78/\[\]]+$", "", text)
    text = re.sub(r"(?<=[A-Za-z])[XGD]$", "", text)
    text = re.sub(r"^(.*组别[1-4])[A-Za-z/]+$", r"\1", text)
    text = re.sub(r"[/\[]+$", "", text)
    return text.strip()


def _text_lookup(labels, us, zh, section: str, label: str) -> tuple[str, str, str]:
    text_id = labels.get((section, label))
    if not text_id:
        return "", "", ""
    return (
        sanitize_text(us.get((section, text_id), "")),
        sanitize_text(zh.get((section, text_id), "")),
        text_id,
    )


def _bare_car_name(name: str, brand: str) -> str:
    if brand and name.lower().startswith(brand.lower() + " "):
        return name[len(brand) + 1 :]
    aliases = {
        "mercedes-amg": "Mercedes-AMG",
        "mercedes-benz": "Mercedes-Benz",
        "ds automobiles": "DS",
    }
    normalized_brand = brand.lower()
    for alias, canonical in aliases.items():
        if normalized_brand == alias and name.lower().startswith(canonical.lower() + " "):
            return name[len(canonical) + 1 :]
    return name


def _car_score(query: str, car: dict) -> float:
    query_norm = _norm(query)
    name_norm = _norm(car["nameEn"])
    core_norm = _norm(_bare_car_name(car["nameEn"], car.get("brand", "")))
    if query_norm == name_norm:
        return 1000.0
    if query_norm == core_norm:
        return 900.0

    year_match = re.search(r"\b(?:19|20)\d{2}\b", query_norm)
    name_year = re.search(r"\b(?:19|20)\d{2}\b", name_norm)
    year_bonus = 0.0
    if year_match and name_year:
        if year_match.group(0) == name_year.group(0):
            year_bonus = 60.0
        elif abs(int(year_match.group(0)) - int(name_year.group(0))) <= 1:
            year_bonus = 18.0
        else:
            year_bonus = -50.0

    stop = {"the", "and", "edition", "package", "coupe", "car"}
    query_tokens = set(query_norm.split()) - stop
    core_tokens = set(core_norm.split()) - stop
    overlap = len(query_tokens & core_tokens)
    coverage = overlap / max(1, len(query_tokens))
    return year_bonus + coverage * 100 + overlap * 8 - abs(
        len(query_tokens) - len(core_tokens)
    )


def _resolve_car(query: str, cars: list[dict]) -> dict | None:
    ranked = sorted(
        ((_car_score(query, car), car) for car in cars),
        key=lambda item: item[0],
        reverse=True,
    )
    if not ranked or ranked[0][0] < 55:
        return None
    return ranked[0][1]


def _extra_story_index(us, zh) -> dict[int, dict]:
    """Map CC number to its first-three-car collection story."""

    found: dict[int, dict] = {}
    for (section, text_id), text in us.items():
        if section != "BuddyQuest" or "sseq" not in text or "Extra_Reward_" not in text:
            continue
        match = re.search(r"CC(\d{3})_", text)
        if not match:
            continue
        cc_number = int(match.group(1))
        ids = [
            int(value)
            for _, value in sorted(
                re.findall(r'car(\d+)="(\d+)"', text),
                key=lambda item: int(item[0]),
            )
        ]
        if not ids:
            continue
        candidate = {
            "textId": text_id,
            "carIds": ids[:3],
            "storyEn": sanitize_text(text),
            "storyZh": sanitize_text(zh.get((section, text_id), "")),
        }
        previous = found.get(cc_number)
        if previous is None or len(candidate["storyEn"]) > len(previous["storyEn"]):
            found[cc_number] = candidate
    return found


def _extra_reward(
    order: int,
    quest_rewards: dict,
    common_rewards: dict,
    gifts: dict,
) -> tuple[str, str, dict]:
    quest_reward = quest_rewards.get(5000 + order)
    if not quest_reward:
        return "Unknown", "未知", {}
    code = quest_reward["codes"][0] if quest_reward["codes"] else None
    common = common_rewards.get(code)
    if not common:
        return "Unknown", "未知", {"commonReward": code}
    reward_type = common["type"]
    if reward_type == 9:
        gift = gifts.get(common["value"])
        if gift and gift["key"].startswith("ROULETTE_"):
            stars = gift["star"]
            en = f"{stars}★ Roulette Ticket"
            zh = f"{stars}★ 轮盘奖券"
            return en, zh, {
                "commonReward": code,
                "giftItem": gift["id"],
                "giftKey": gift["key"],
                "star": stars,
            }
    if order in EXTRA_CAR_REWARD_OVERRIDES:
        en, zh = EXTRA_CAR_REWARD_OVERRIDES[order]
        return en, zh, {"commonReward": code, "type": reward_type}
    return "Roulette / reward item", "轮盘 / 奖励物品", {
        "commonReward": code,
        "type": reward_type,
    }


def build() -> dict:
    dms_dir = _find_dms_dir()
    us = load_best("us")
    zh = load_best("zh")
    visible_us = load_visible_texts("us")
    visible_zh = load_visible_texts("zh")
    labels = _label_index("us")
    cars_payload = json.loads((DATA_DIR / "cars.json").read_text(encoding="utf-8"))
    cars = cars_payload["cars"]
    cars_by_id = {int(car["id"], 16): car for car in cars}

    name_rows, name_string, name_u32 = _parse_dms(
        dms_dir / "CAR_NAME_ALPHABET.dmsb"
    )
    game_names = {
        name_u32(row, 0): name_string(name_u32(row, 0x20))
        for row in name_rows
    }

    quest_rows, quest_string, quest_u32 = _parse_dms(dms_dir / "Quests.dmsb")
    collection_rows, collection_string, collection_u32 = _parse_dms(
        dms_dir / "CarCollection.dmsb"
    )
    collection_by_code = {}
    for row in collection_rows:
        key = collection_string(collection_u32(row, 8))
        match = re.search(r"(MB|CC)(\d{3})", key, re.IGNORECASE)
        if match:
            collection_by_code[
                f"{match.group(1).upper()}{match.group(2)}"
            ] = row

    quest_reward_rows, _, quest_reward_u32 = _parse_dms(
        dms_dir / "QuestRewards.dmsb"
    )
    quest_rewards = {
        quest_reward_u32(row, 0): {
            "codes": [
                quest_reward_u32(row, offset)
                for offset in range(0x20, len(row), 4)
                if quest_reward_u32(row, offset) not in (0, 0xFFFFFFFF)
            ]
        }
        for row in quest_reward_rows
    }

    common_reward_rows, common_string, common_u32 = _parse_dms(
        dms_dir / "CommonReward.dmsb"
    )
    common_rewards = {
        common_u32(row, 0): {
            "id": common_u32(row, 0),
            "key": common_string(common_u32(row, 8)),
            "type": common_u32(row, 0x10),
            "value": common_u32(row, 0x38),
        }
        for row in common_reward_rows
    }
    gift_rows, gift_string, gift_u32 = _parse_dms(dms_dir / "GiftItem.dmsb")
    gifts = {
        gift_u32(row, 0): {
            "id": gift_u32(row, 0),
            "key": gift_string(gift_u32(row, 8)),
            "type": gift_u32(row, 0x10),
            "star": gift_u32(row, 0x28),
        }
        for row in gift_rows
    }

    extra_story = _extra_story_index(us, zh)
    for story in extra_story.values():
        key = ("BuddyQuest", story["textId"])
        story["storyEn"] = sanitize_text(
            visible_us.get(key, story["storyEn"])
        )
        story["storyZh"] = sanitize_text(
            visible_zh.get(key, story["storyZh"])
        )
    records = []
    unmatched_queries = []

    def add_car_names(names: list[str], source: str) -> list[dict]:
        resolved = []
        for query in names:
            car = _resolve_car(query, cars)
            if car is None:
                unmatched_queries.append({"query": query, "source": source})
                resolved.append({
                    "id": "",
                    "nameEn": query,
                    "nameZh": query,
                    "brand": "Unknown",
                    "image": "",
                    "matchMethod": "unresolved",
                })
            else:
                resolved.append({
                    "id": car["id"],
                    "nameEn": car["nameEn"],
                    "nameZh": car["nameZh"],
                    "brand": car["brand"],
                    "image": car["image"],
                    "brandLogo": car.get("brandLogo", ""),
                    "matchMethod": "game-name-table",
                })
        return resolved

    for row in quest_rows:
        quest_id = quest_u32(row, 0)
        if quest_id < 600:
            continue
        order = quest_u32(row, 0x10)
        key = quest_string(quest_u32(row, 8))
        code_match = re.search(r"MB(\d{3})", key, re.IGNORECASE)
        if not code_match or order < 1 or order > 51:
            continue
        code = f"MB{code_match.group(1)}"
        title_key = f"QuestDetail_{code}_Title_"
        description_key = f"QuestDetail_{code}_Description_"
        title_en, title_zh, title_id = _text_lookup(
            labels, us, zh, "GTCafeMenu", title_key
        )
        description_en, description_zh, description_id = _text_lookup(
            labels, us, zh, "GTCafeMenu", description_key
        )
        category = "main" if order <= 39 else "bonus"
        car_names = MAIN_COLLECTION_CARS.get(order, [])
        car_entries = add_car_names(car_names, f"{category}:{order}")

        cover_asset = ""
        cover_thumb_asset = ""
        cover_rel = ""
        thumb_rel = ""
        cover_status = "not-applicable"
        collection_row = collection_by_code.get(code)
        if collection_row:
            collection_key = collection_string(collection_u32(collection_row, 8))
            cover_asset = f"/piece/raw/collection_image/{collection_key}.img"
            cover_thumb_asset = (
                f"/piece/raw/collection_thumbnail/{collection_key}.img"
            )
            cover_rel, cover_status = _asset_source(cover_asset)
            thumb_rel, _ = _asset_source(cover_thumb_asset)

        reward_en, reward_zh = (
            MAIN_REWARDS.get(order) if category == "main" else BONUS_REWARDS.get(order)
        ) or ("Unknown", "未知")
        if category == "main":
            display_number = f"No.{order}"
            collection_en = MAIN_COLLECTION_NAMES_EN.get(
                order,
                re.sub(r"^Collection:\s*", "", title_en, flags=re.IGNORECASE),
            )
            if collection_en and order in MAIN_COLLECTION_NAMES_EN:
                title_en = f"Collection: {collection_en}"
            collection_zh = clean_zh_label(
                re.sub(r"^(?:收集|收藏)[：:]\s*", "", title_zh)
            )
            if order in MAIN_COLLECTION_NAMES_EN:
                title_zh = f"收集：{collection_zh}"
            else:
                title_zh = clean_zh_label(title_zh)
        else:
            display_number = f"No.{order}"
            title_en = BONUS_TITLES_EN.get(order, title_en)
            collection_en = title_en
            collection_zh = clean_zh_label(title_zh)
            title_zh = clean_zh_label(title_zh)

        records.append({
            "id": f"{category}-{order:03d}",
            "category": category,
            "categoryZh": "主线菜单书" if category == "main" else "奖励菜单书",
            "categoryEn": "Main Menu Book" if category == "main" else "Bonus Menu Book",
            "number": order,
            "displayNumber": display_number,
            "internalCode": code,
            "titleEn": title_en,
            "titleZh": title_zh,
            "collectionNameEn": collection_en,
            "collectionNameZh": collection_zh,
            "descriptionEn": description_en,
            "descriptionZh": description_zh,
            "requirementsEn": (
                "Collect: " + ", ".join(
                    car["nameEn"] for car in car_entries
                )
                if car_names
                else description_en
            ),
            "requirementsZh": (
                "收集：" + "、".join(
                    car["nameZh"] for car in car_entries
                )
                if car_names
                else description_zh
            ),
            "rewardEn": reward_en,
            "rewardZh": reward_zh,
            "collectorLevel": 0,
            "cars": car_entries,
            "manufacturers": sorted({car["brand"] for car in car_entries}),
            "coverAsset": cover_asset,
            "coverThumbAsset": cover_thumb_asset,
            "coverRel": cover_rel,
            "coverThumbRel": thumb_rel,
            "coverStatus": cover_status,
            "coverContainerStatus": cover_status,
            "cover": "",
            "thumbnail": "",
            "storyEn": description_en,
            "storyZh": description_zh,
            "sources": {
                "questId": quest_id,
                "questKey": key,
                "titleLabel": title_key,
                "titleTextId": title_id,
                "descriptionLabel": description_key,
                "descriptionTextId": description_id,
                "carList": "published menu table + local name resolver",
                "cover": "local PS5 collection_image",
            },
        })

    for row in collection_rows:
        collection_id = collection_u32(row, 0)
        if collection_id < 31:
            continue
        key = collection_string(collection_u32(row, 8))
        cc_match = re.search(r"CC(\d{3})", key, re.IGNORECASE)
        if not cc_match:
            continue
        cc_number = int(cc_match.group(1))
        order = collection_u32(row, 0x98)
        collector_level = collection_u32(row, 0x94)
        title_key = collection_string(collection_u32(row, 0x18))
        collection_key = collection_string(collection_u32(row, 0x20))
        description_key = collection_string(collection_u32(row, 0x28))
        title_en, title_zh, title_id = _text_lookup(
            labels, us, zh, "GTCafeMenu", title_key
        )
        name_en, name_zh, name_id = _text_lookup(
            labels, us, zh, "GTCafeMenu", collection_key
        )
        description_en, description_zh, description_id = _text_lookup(
            labels, us, zh, "GTCafeMenu", description_key
        )
        canonical_name_en = (
            EXTRA_NAMES_EN[order - 1]
            if 1 <= order <= len(EXTRA_NAMES_EN)
            else name_en
        )
        clean_name_zh = clean_zh_label(name_zh)
        if canonical_name_en:
            name_en = canonical_name_en
            title_en = f"Collection: {canonical_name_en}"
        name_zh = clean_name_zh
        prefix = "收藏：" if title_zh.startswith("收藏") else "收集："
        title_zh = f"{prefix}{name_zh}"

        story = extra_story.get(cc_number)
        if story and order not in FORCE_EXTRA_CAR_FALLBACK:
            names = [
                game_names.get(car_id, f"Car {car_id}")
                for car_id in story["carIds"]
            ]
            car_source = "BuddyQuest sseq + CAR_NAME_ALPHABET"
            car_ids = story["carIds"]
        else:
            names = EXTRA_CAR_NAME_FALLBACK.get(order, [])
            car_source = "published Extra Menu table"
            car_ids = []
        car_entries = add_car_names(names, f"extra:{order}")
        reward_en, reward_zh, reward_source = _extra_reward(
            order, quest_rewards, common_rewards, gifts
        )
        cover_asset = f"/piece/raw/collection_image/{key}.img"
        cover_thumb_asset = f"/piece/raw/collection_thumbnail/{key}.img"
        cover_rel, cover_status = _asset_source(cover_asset)
        thumb_rel, _ = _asset_source(cover_thumb_asset)
        records.append({
            "id": f"extra-{order:03d}",
            "category": "extra",
            "categoryZh": "追加菜单 / 收藏菜单",
            "categoryEn": "Extra Menu",
            "number": order,
            "displayNumber": f"EM {order}",
            "internalCode": f"CC{cc_match.group(1)}",
            "titleEn": title_en or f"Collection: {name_en}",
            "titleZh": title_zh or f"收藏：{name_zh}",
            "collectionNameEn": name_en,
            "collectionNameZh": name_zh,
            "descriptionEn": description_en,
            "descriptionZh": description_zh,
            "requirementsEn": "Collect: " + ", ".join(
                car["nameEn"] for car in car_entries
            ),
            "requirementsZh": "收集：" + "、".join(
                car["nameZh"] for car in car_entries
            ),
            "rewardEn": reward_en,
            "rewardZh": reward_zh,
            "collectorLevel": collector_level,
            "cars": car_entries,
            "manufacturers": sorted({car["brand"] for car in car_entries}),
            "coverAsset": cover_asset,
            "coverThumbAsset": cover_thumb_asset,
            "coverRel": cover_rel,
            "coverThumbRel": thumb_rel,
            "coverStatus": cover_status,
            "coverContainerStatus": cover_status,
            "cover": "",
            "thumbnail": "",
            "storyEn": story["storyEn"] if story else description_en,
            "storyZh": story["storyZh"] if story else description_zh,
            "sources": {
                "carCollectionId": collection_id,
                "collectionKey": key,
                "titleLabel": title_key,
                "titleTextId": title_id,
                "collectionNameLabel": collection_key,
                "collectionNameTextId": name_id,
                "descriptionLabel": description_key,
                "descriptionTextId": description_id,
                "storyTextId": story["textId"] if story else "",
                "carIds": car_ids,
                "carList": car_source,
                "reward": reward_source,
                "cover": "local PS5 collection_image",
            },
        })

    for order, supplement in EXTRA_WEB_SUPPLEMENTS.items():
        car_entries = add_car_names(
            supplement["cars"],
            f"extra-web-supplement:{order}",
        )
        title_en = f"Collection: {supplement['collectionNameEn']}"
        title_zh = f"收藏：{supplement['collectionNameZh']}"
        records.append({
            "id": f"extra-{order:03d}",
            "category": "extra",
            "runtime": "online-1.72",
            "categoryZh": "追加菜单 / 收藏菜单",
            "categoryEn": "Extra Menu",
            "number": order,
            "displayNumber": f"EM {order}",
            "internalCode": supplement["internalCode"],
            "titleEn": title_en,
            "titleZh": title_zh,
            "collectionNameEn": supplement["collectionNameEn"],
            "collectionNameZh": supplement["collectionNameZh"],
            "descriptionEn": supplement["descriptionEn"],
            "descriptionZh": supplement["descriptionZh"],
            "requirementsEn": "Collect: " + ", ".join(
                car["nameEn"] for car in car_entries
            ),
            "requirementsZh": "收集：" + "、".join(
                car["nameZh"] for car in car_entries
            ),
            "rewardEn": supplement["rewardEn"],
            "rewardZh": supplement["rewardZh"],
            "collectorLevel": supplement["collectorLevel"],
            "cars": car_entries,
            "manufacturers": sorted({car["brand"] for car in car_entries}),
            "coverAsset": "",
            "coverThumbAsset": "",
            "coverRel": "",
            "coverThumbRel": "",
            "coverStatus": "web-supplement",
            "coverContainerStatus": "missing",
            "cover": "",
            "thumbnail": "",
            "storyEn": supplement["descriptionEn"],
            "storyZh": supplement["descriptionZh"],
            "sources": {
                "webSupplement": supplement["sourceUrl"],
                "spec": "Spec IV / 1.72",
                "carList": "guide car list + local cars.json",
                "reward": "official 1.72 update / GTPlanet",
                "collectorLevel": "official 1.72 update",
                "cover": "public guide screenshot",
            },
        })

    cover_manifest_path = DATA_DIR / "menu_cover_sources.json"
    if cover_manifest_path.is_file():
        cover_manifest = json.loads(
            cover_manifest_path.read_text(encoding="utf-8")
        )
        screenshots = {
            entry["recordId"]: entry
            for entry in cover_manifest.get("entries", [])
        }
        for record in records:
            screenshot = screenshots.get(record["id"])
            if screenshot:
                record["cover"] = screenshot["cover"]
                record["coverStatus"] = "screenshot"
                record["coverSourcePage"] = screenshot["pageUrl"]
                record["coverScreenshot"] = screenshot["pictureUrl"]
                record["sources"]["coverPage"] = screenshot["pageUrl"]
                record["sources"]["coverScreenshot"] = screenshot["pictureUrl"]
            elif record["coverStatus"] == "present":
                record["coverStatus"] = "container-only"
            else:
                record["coverStatus"] = "missing"

    records.sort(
        key=lambda item: (
            {"main": 0, "bonus": 1, "extra": 2}[item["category"]],
            item["number"],
        )
    )
    payload = {
        "generatedFrom": "GT7 PS5 DMSB + RT2 + local content store",
        "gameVersion": "1.710.000 local + 1.72 web supplement",
        "counts": {
            "total": len(records),
            "main": sum(item["category"] == "main" for item in records),
            "bonus": sum(item["category"] == "bonus" for item in records),
            "extra": sum(item["category"] == "extra" for item in records),
            "withCars": sum(bool(item["cars"]) for item in records),
            "withCoverContainer": sum(
                item.get("coverContainerStatus") == "present"
                for item in records
            ),
            "withScreenshot": sum(
                item["coverStatus"] == "screenshot" for item in records
            ),
            "withAnyCover": sum(
                item["coverStatus"] in {"screenshot", "container-only"}
                or bool(item.get("cover"))
                for item in records
            ),
            "withDisplayCover": sum(bool(item.get("cover")) for item in records),
            "resolvedCars": sum(
                car.get("matchMethod") != "unresolved"
                for item in records
                for car in item["cars"]
            ),
        },
        "records": records,
        "unmatchedQueries": unmatched_queries,
    }

    DATA_DIR.mkdir(parents=True, exist_ok=True)
    REPORT_DIR.mkdir(parents=True, exist_ok=True)
    serialized = json.dumps(payload, ensure_ascii=False, separators=(",", ":"))
    (DATA_DIR / "menu_books.json").write_text(serialized, encoding="utf-8")
    (REPORT_DIR / "menu_books.json").write_text(serialized, encoding="utf-8")

    fields = [
        "category",
        "number",
        "displayNumber",
        "internalCode",
        "collectionNameEn",
        "collectionNameZh",
        "titleEn",
        "titleZh",
        "manufacturers",
        "carsEn",
        "carsZh",
        "collectorLevel",
        "rewardEn",
        "rewardZh",
        "coverAsset",
        "coverStatus",
        "descriptionEn",
        "descriptionZh",
    ]
    for target in (
        DATA_DIR / "menu_books.csv",
        REPORT_DIR / "menu_books.csv",
    ):
        with target.open("w", encoding="utf-8-sig", newline="") as handle:
            writer = csv.DictWriter(handle, fieldnames=fields)
            writer.writeheader()
            for record in records:
                writer.writerow({
                    "category": record["category"],
                    "number": record["number"],
                    "displayNumber": record["displayNumber"],
                    "internalCode": record["internalCode"],
                    "collectionNameEn": record["collectionNameEn"],
                    "collectionNameZh": record["collectionNameZh"],
                    "titleEn": record["titleEn"],
                    "titleZh": record["titleZh"],
                    "manufacturers": "; ".join(record["manufacturers"]),
                    "carsEn": "; ".join(car["nameEn"] for car in record["cars"]),
                    "carsZh": "; ".join(car["nameZh"] for car in record["cars"]),
                    "collectorLevel": record["collectorLevel"],
                    "rewardEn": record["rewardEn"],
                    "rewardZh": record["rewardZh"],
                    "coverAsset": record["coverAsset"],
                    "coverStatus": record["coverStatus"],
                    "descriptionEn": record["descriptionEn"],
                    "descriptionZh": record["descriptionZh"],
                })

    return payload


if __name__ == "__main__":
    result = build()
    print(json.dumps(result["counts"], ensure_ascii=False, indent=2))
    if result["unmatchedQueries"]:
        print("UNMATCHED")
        for item in result["unmatchedQueries"]:
            print(item)
