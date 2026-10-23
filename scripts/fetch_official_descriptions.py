#!/usr/bin/env python3
"""Cache the official GT7 car-list descriptions used by the research UI.

The public car-list SPA ships one static description module per locale. Those
modules contain the same ``DescriptionCatch`` (``hero``) and
``DescriptionLong`` (``desc``) fields found in the PS5 description RT2 files,
but without the statistical-decoding damage seen in the Chinese payloads.

The cache is deterministic apart from the discovered hashed asset names. It is
intended as a clean cross-check for the local RT2 extraction, not as a
replacement for the car-key/text-id ownership evidence.
"""

from __future__ import annotations

import argparse
import json
import re
import urllib.request
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "reports" / "official_car_descriptions.json"
BASE = "https://www.gran-turismo.com"


def _fetch(url: str, proxy: str | None = None) -> str:
    handlers = []
    if proxy:
        handlers.append(
            urllib.request.ProxyHandler({
                "http": proxy,
                "https": proxy,
            })
        )
    opener = urllib.request.build_opener(*handlers)
    request = urllib.request.Request(
        url,
        headers={"User-Agent": "gt7-research/1.0"},
    )
    with opener.open(request, timeout=60) as response:
        return response.read().decode("utf-8")


def _asset_name(app_js: str, pattern: str) -> str:
    match = re.search(pattern, app_js)
    if not match:
        raise RuntimeError(f"asset not found in car-list app: {pattern}")
    return match.group(1)


def _parse_descriptions(source: str) -> dict[str, dict[str, str]]:
    rows = re.findall(
        r"car(\d+):\{desc:`(.*?)`,hero:`(.*?)`,id:`car\1`\}",
        source,
        flags=re.DOTALL,
    )
    return {
        f"car{number}": {"desc": desc, "hero": hero}
        for number, desc, hero in rows
    }


def _parse_cars(source: str) -> dict[str, dict]:
    blocks = re.findall(
        r"car(\d+):\{(.*?)\}(?=,car\d+:|};export\{e as Cars\})",
        source,
        flags=re.DOTALL,
    )
    result = {}
    for number, body in blocks:
        car_id = f"car{number}"
        name = re.search(r"nameLong:`(.*?)`", body)
        manufacturer = re.search(r"manufacturerId:`(.*?)`", body)
        result[car_id] = {
            "id": car_id,
            "name": name.group(1) if name else "",
            "manufacturerId": manufacturer.group(1) if manufacturer else "",
        }
    return result


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--proxy", default="")
    parser.add_argument("--out", type=Path, default=OUT)
    args = parser.parse_args()
    proxy = args.proxy or None

    page = _fetch(f"{BASE}/cn/gt7/carlist", proxy)
    app_path = _asset_name(
        page,
        r'src="(/common/dist/gt7/carlist/assets/index-[^"]+\.js)"',
    )
    app_js = _fetch(BASE + app_path, proxy)
    description_assets = {
        locale: _asset_name(
            app_js,
            rf"\./descriptions\.{locale}-([^`]+)\.js",
        )
        for locale in ("cn", "us")
    }
    car_assets = {
        locale: _asset_name(
            app_js,
            rf"\./cars\.{locale}-([^`]+)\.js",
        )
        for locale in ("cn", "us")
    }
    asset_root = app_path.rsplit("/", 1)[0]
    descriptions = {
        locale: _parse_descriptions(
            _fetch(
                f"{BASE}{asset_root}/descriptions.{locale}-"
                f"{description_assets[locale]}.js",
                proxy,
            )
        )
        for locale in ("cn", "us")
    }
    cars = {
        locale: _parse_cars(
            _fetch(
                f"{BASE}{asset_root}/cars.{locale}-{car_assets[locale]}.js",
                proxy,
            )
        )
        for locale in ("cn", "us")
    }

    ids = set(descriptions["cn"]) & set(descriptions["us"])
    if len(ids) != 580:
        raise RuntimeError(f"expected 580 bilingual descriptions, got {len(ids)}")

    records = []
    for car_id in sorted(ids, key=lambda value: int(value[3:])):
        cn = descriptions["cn"][car_id]
        us = descriptions["us"][car_id]
        records.append({
            "id": car_id,
            "nameEn": cars["us"].get(car_id, {}).get("name", ""),
            "nameZh": cars["cn"].get(car_id, {}).get("name", ""),
            "manufacturerId": cars["us"].get(car_id, {}).get(
                "manufacturerId", ""
            ),
            "heroEn": us["hero"],
            "descEn": us["desc"],
            "heroZh": cn["hero"],
            "descZh": cn["desc"],
            "sourceUrlEn": f"{BASE}/us/gt7/carlist/id/{car_id}",
            "sourceUrlZh": f"{BASE}/cn/gt7/carlist/id/{car_id}",
        })

    payload = {
        "generatedFrom": "official Gran Turismo 7 car-list description modules",
        "sources": {
            "cn": f"{BASE}{asset_root}/descriptions.cn-"
                  f"{description_assets['cn']}.js",
            "us": f"{BASE}{asset_root}/descriptions.us-"
                  f"{description_assets['us']}.js",
        },
        "count": len(records),
        "cars": records,
    }
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(
        json.dumps(payload, ensure_ascii=False, indent=1),
        encoding="utf-8",
    )
    print(f"wrote {len(records)} bilingual descriptions -> {args.out}")


if __name__ == "__main__":
    main()
