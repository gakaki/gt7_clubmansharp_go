#!/usr/bin/env python3
"""Match GT Cafe car-detail conversations to car names and pair EN/ZH."""

import csv
import json
import re
from pathlib import Path

TEXT_DIRS = [
    Path(r"C:\gt7\gt7\_re\extracted\text_pooled"),
    Path(r"C:\gt7\gt7\_re\extracted\text_decoded"),
]
OUT = Path(r"C:\gt7\gt7\_re\reports\cafe_dialogues.json")


def load(lang):
    from textpick import load_best
    return load_best(lang)


def clean(s: str) -> str:
    s = re.sub(r"<[^>]*>", "", s)
    s = re.sub(r"^[^\w]{0,3}(?:sseq|seq)[^>]*>", "", s)
    s = re.sub(r"^[^\w]{0,3}p[^\n]{0,80}>", "", s)
    s = s.strip()
    if s.endswith("Y") and len(s) > 1 and s[-2].isalnum():
        s = s[:-1]
    return s.strip()


def main() -> None:
    us, zh = load("us"), load("zh")
    names = []
    for (sec, _id), t in us.items():
        if sec != "CarNameLong":
            continue
        n = clean(t)
        m = re.search(r"'(\d\d)", n)
        if m:
            n = n[:m.end()]
        else:
            n = n.rstrip(" >vY")
        names.append(n)
    norm = {}
    for n in names:
        key = re.sub(r"\s*'\d\d$", "", n).strip().lower()
        toks = key.split()
        norm.setdefault(key, n)
        if len(toks) >= 3:
            norm.setdefault(" ".join(toks[:3]), n)
        if len(toks) >= 2:
            norm.setdefault(" ".join(toks[:2]), n)

    items = []
    for (sec, sid), text in us.items():
        if sec != "BuddyGTCafe" or "CarDetail" not in text:
            continue
        body = clean(text)
        low = body.lower()
        matched = None
        best_len = 0
        for key, name in norm.items():
            if key and key in low and len(key) > best_len:
                matched, best_len = name, len(key)
        items.append({
            "id": sid,
            "en": body,
            "zh": clean(zh.get((sec, sid), "")),
            "car": matched,
        })

    OUT.write_text(json.dumps(items, ensure_ascii=False, indent=1), encoding="utf-8")
    matched = [i for i in items if i["car"]]
    print(f"CarDetail entries: {len(items)}, matched to a car name: {len(matched)}")
    for i in matched[:10]:
        print(f"  {i['car']:28} {i['id']}  {i['en'][:80]!r}")
    unmatched = [i for i in items if not i["car"]]
    print(f"\nunmatched samples ({len(unmatched)}):")
    for i in unmatched[:6]:
        print(f"  {i['id']}  {i['en'][:100]!r}")


if __name__ == "__main__":
    main()
