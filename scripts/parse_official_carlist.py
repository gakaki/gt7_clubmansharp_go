#!/usr/bin/env python3
"""Parse the official GT7 car-list DOM into a JSON catalogue with thumbnails."""

import html
import json
import re
from pathlib import Path

SRC = Path(r"C:\gt7\gt7\_re\site\official_carlist2.html")
OUT = Path(r"C:\gt7\gt7\_re\reports\official_cars.json")
BASE = "https://www.gran-turismo.com"


def main() -> None:
    s = SRC.read_text(encoding="utf-8", errors="replace")
    brand_logo = {}
    for m in re.finditer(r'<h3 class="css-16jmhlm">(.*?)</h3>', s, re.S):
        brand = html.unescape(re.sub(r"<[^>]+>", "", m.group(1))).strip()
        before = s[max(0, m.start() - 4000):m.start()]
        logos = re.findall(r'/common/dist/gt7/carlist/tuner_logos/[^"\']+', before)
        if logos:
            brand_logo.setdefault(brand, logos[-1])
    parts = re.split(r'<h3 class="css-16jmhlm">', s)
    cars = []
    for part in parts[1:]:
        brand = html.unescape(re.match(r"([^<]*)", part).group(1)).strip()
        logo = brand_logo.get(brand)
        blocks = re.findall(
            r'data-car-id="(car\d+)"(.{0,2000}?)(?=data-car-id=|<h3 class="css-16jmhlm">|\Z)',
            part,
            re.S,
        )
        for cid, body in blocks:
            name = re.search(r'<span class="css-af5abj">(.*?)</span>', body, re.S)
            thumb = re.search(r'src="(/common/dist/gt7/carlist/car_thumbnails/[^"]+)"', body)
            if not name:
                continue
            cars.append({
                "brand": brand,
                "id": cid,
                "name": html.unescape(re.sub(r"<[^>]+>", "", name.group(1))).strip(),
                "thumb": (BASE + thumb.group(1)) if thumb
                else f"{BASE}/common/dist/gt7/carlist/car_thumbnails/{cid}.png",
                "logo": (BASE + logo) if logo else None,
            })

    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(cars, ensure_ascii=False, indent=1), encoding="utf-8")
    with_thumb = sum(1 for c in cars if c["thumb"])
    print(f"cars={len(cars)} with_thumb={with_thumb} brands={len({c['brand'] for c in cars})}")
    for c in cars[:5]:
        print(" ", c["brand"], "|", c["name"], "|", c["thumb"])


if __name__ == "__main__":
    main()
