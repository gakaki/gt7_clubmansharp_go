#!/usr/bin/env python3
"""Fetch and normalize the public menu-book cover screenshots.

The retail PS5 collection covers are stored in a game-specific JPEG XL
container. The JXL box layout and EXIF data are readable, but the codestream
is rejected by libjxl, so the covers cannot currently be decoded locally.

The skymac GT7 Cafe guide publishes the corresponding in-game cover screens as
numbered JPEGs. This script maps the guide heading (not the URL page number) to
the actual menu number, downloads each cover, and converts it to web-sized
WebP for the research UI. Seasonal menus are deliberately skipped because they
are time-limited and have a separate data model.
"""

from __future__ import annotations

import html
import json
import re
import time
import urllib.error
import urllib.request
from urllib.parse import urljoin
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path

from PIL import Image


ROOT = Path(r"C:\gt7\gt7\_re")
OUT_DIR = ROOT / "site" / "public" / "menu_covers"
MANIFEST = ROOT / "site" / "public" / "data" / "menu_cover_sources.json"
INDEX_URL = "https://www.skymac.org/GT7-Cafe.htm"
BASE_URL = "https://www.skymac.org/"
USER_AGENT = "Mozilla/5.0 GT7 research preservation"


def fetch(url: str, attempts: int = 4) -> bytes:
    error = None
    for attempt in range(attempts):
        request = urllib.request.Request(
            url,
            headers={"User-Agent": USER_AGENT, "Accept": "*/*"},
        )
        try:
            with urllib.request.urlopen(request, timeout=45) as response:
                return response.read()
        except (urllib.error.URLError, TimeoutError, ConnectionError) as exc:
            error = exc
            time.sleep(1.5 * (attempt + 1))
    raise RuntimeError(f"failed to fetch {url}: {error}")


def classify_heading(heading: str) -> tuple[str, int] | None:
    heading = html.unescape(re.sub(r"\s+", " ", heading)).strip()
    match = re.search(
        r"Livre des menus\s*(?:n[°º]\s*)?(\d+)",
        heading,
        re.IGNORECASE,
    )
    if match:
        number = int(match.group(1))
        return ("main" if number <= 39 else "bonus", number)
    match = re.search(
        r"Menu\s+suppl[eé]mentaire\s*(?:n[°º]\s*)?(\d+)",
        heading,
        re.IGNORECASE,
    )
    if match:
        return "extra", int(match.group(1))
    return None


def scrape_menu_page(page_id: int, page_url: str) -> dict:
    page = fetch(page_url).decode("utf-8", "replace")
    h1_match = re.search(r"<h1[^>]*>(.*?)</h1>", page, re.IGNORECASE | re.DOTALL)
    heading = (
        html.unescape(re.sub(r"<[^>]+>", " ", h1_match.group(1)))
        if h1_match
        else ""
    )
    heading = re.sub(r"\s+", " ", heading).strip()
    if "saisonnier" in page_url.lower():
        return {
            "pageId": page_id,
            "pageUrl": page_url,
            "heading": heading,
            "error": "seasonal menu skipped",
        }
    classification = classify_heading(heading)
    image_match = re.search(
        r'(?:src|href)=["\']([^"\']*GTCafePictures/picture(\d+)\.jpg)',
        page,
        re.IGNORECASE,
    )
    if classification is None:
        return {
            "pageId": page_id,
            "pageUrl": page_url,
            "heading": heading,
            "error": "unrecognized heading",
        }
    if image_match is None:
        category, number = classification
        return {
            "pageId": page_id,
            "pageUrl": page_url,
            "heading": heading,
            "category": category,
            "number": number,
            "recordId": f"{category}-{number:03d}",
            "error": "no cover screenshot on guide page",
        }
    category, number = classification
    record_id = f"{category}-{number:03d}"
    picture_path = image_match.group(1)
    picture_number = int(image_match.group(2))
    image_url = urljoin(BASE_URL, picture_path)
    image_bytes = fetch(image_url)
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    output = OUT_DIR / f"{record_id}.webp"
    temp = OUT_DIR / f".{record_id}.download"
    temp.write_bytes(image_bytes)
    try:
        with Image.open(temp) as image:
            image = image.convert("RGB")
            image.thumbnail((1280, 720), Image.Resampling.LANCZOS)
            image.save(output, "WEBP", quality=84, method=6)
    finally:
        temp.unlink(missing_ok=True)
    return {
        "pageId": page_id,
        "pageUrl": page_url,
        "heading": heading,
        "category": category,
        "number": number,
        "recordId": record_id,
        "pictureNumber": picture_number,
        "pictureUrl": image_url,
        "cover": f"menu_covers/{record_id}.webp",
        "source": "skymac.org GT7 Cafe guide screenshots (research use)",
    }


def main() -> None:
    index = fetch(INDEX_URL).decode("utf-8", "replace")
    links = {}
    for match in re.finditer(
        r'href=["\'](GT7-Cafe/Menu-(\d+)-[^"\']+\.htm)["\']',
        index,
        re.IGNORECASE,
    ):
        links[int(match.group(2))] = urljoin(BASE_URL, match.group(1))

    entries_by_id = {}
    errors = []
    with ThreadPoolExecutor(max_workers=8) as pool:
        futures = {
            pool.submit(scrape_menu_page, page_id, url): page_id
            for page_id, url in sorted(links.items())
        }
        for future in as_completed(futures):
            page_id = futures[future]
            try:
                entry = future.result()
                if entry.get("error"):
                    errors.append(entry)
                else:
                    previous = entries_by_id.get(entry["recordId"])
                    if previous is None:
                        entries_by_id[entry["recordId"]] = entry
                    else:
                        errors.append({
                            "pageId": page_id,
                            "recordId": entry["recordId"],
                            "error": "duplicate record; kept first page",
                        })
                    print(
                        f"OK {page_id:03d} {entry['recordId']} "
                        f"picture={entry['pictureNumber']} {entry['heading']}"
                    )
            except Exception as exc:
                errors.append({"pageId": page_id, "error": str(exc)})

    entries = sorted(
        entries_by_id.values(),
        key=lambda item: (item["category"], item["number"]),
    )
    payload = {
        "generatedFrom": "skymac.org GT7 Cafe guide menu screenshots",
        "indexUrl": INDEX_URL,
        "count": len(entries),
        "errors": errors,
        "entries": entries,
    }
    MANIFEST.parent.mkdir(parents=True, exist_ok=True)
    MANIFEST.write_text(
        json.dumps(payload, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    valid_names = {
        Path(entry["cover"]).name
        for entry in entries
    }
    for output in OUT_DIR.glob("*.webp"):
        if output.name not in valid_names:
            output.unlink()
    print(json.dumps({"count": len(entries), "errors": len(errors)}, indent=2))


if __name__ == "__main__":
    main()
