#!/usr/bin/env python3
"""Download official GT7 car thumbnails and brand logos."""

import json
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from urllib.request import Request, urlopen

SRC = Path(r"C:\gt7\gt7\_re\reports\official_cars.json")
CARS_OUT = Path(r"C:\gt7\gt7\_re\site\public\cars")
LOGOS_OUT = Path(r"C:\gt7\gt7\_re\site\public\logos")


def fetch(url: str, dest: Path, force: bool = False) -> bool:
    if not force and dest.exists() and dest.stat().st_size > 1000:
        return True
    for attempt in range(3):
        try:
            req = Request(url, headers={"User-Agent": "Mozilla/5.0"})
            with urlopen(req, timeout=30) as r:
                data = r.read()
            if len(data) < 500:
                return False
            dest.parent.mkdir(parents=True, exist_ok=True)
            dest.write_bytes(data)
            return True
        except Exception:
            time.sleep(1 + attempt)
    return False


def main() -> None:
    cars = json.loads(SRC.read_text(encoding="utf-8"))
    CARS_OUT.mkdir(parents=True, exist_ok=True)
    LOGOS_OUT.mkdir(parents=True, exist_ok=True)
    jobs = [(c["thumb"], CARS_OUT / f"{c['id']}.png", False) for c in cars if c["thumb"]]
    logos = {}
    for c in cars:
        if c.get("logo"):
            logos.setdefault(c["brand"], c["logo"])
    jobs += [(url, LOGOS_OUT / f"{brand.replace(' ', '_').replace('/', '_')}.png", True)
             for brand, url in logos.items()]
    print(f"downloading {len(jobs)} images ...")
    with ThreadPoolExecutor(max_workers=12) as ex:
        results = list(ex.map(lambda j: fetch(*j), jobs))
    print(f"ok={sum(results)} failed={len(results)-sum(results)}")
    missing = [c["id"] for c, ok in zip(cars, results[:len(cars)]) if not ok]
    print("missing sample:", missing[:20])


if __name__ == "__main__":
    main()
