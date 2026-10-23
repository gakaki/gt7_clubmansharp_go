#!/usr/bin/env python3
"""Build a bilingual (EN/ZH) text dossier for a car query."""

import csv
import re
import sys
from pathlib import Path

TEXT_DIRS = [
    Path(r"C:\gt7\gt7\_re\extracted\text_decoded"),
    Path(r"C:\gt7\gt7\_re\extracted\text_patch_decoded"),
]
ALT_DIR = Path(r"C:\gt7\gt7\_re\extracted\text_pooled")

SECTION_DESC = {
    "BuddyContentCarBrandCentralOwner": "品牌中心 / 车商讲解（车库内同一段车辆介绍）",
    "BuddyContentCarUsedCarOwner": "二手车商讲解",
    "BuddyContentCarHagerty": "Hagerty 传奇车商讲解",
    "BuddyRace": "咖啡屋 / 赛事开场对白",
    "BuddyGTCafe": "GT 咖啡屋多人对白",
    "BuddyQuest": "咖啡屋任务 / 引导对白",
    "BuddyMenuRecommendation": "咖啡屋菜单推荐（版本更新介绍）",
    "DeliveryItem": "车辆交付 / 奖励文案",
    "WeeklyChallenge": "周挑战赛事名称",
    "CarNameLong": "车辆全名",
    "CarNameShort": "车辆短名",
    "CarNameAR": "车辆名（AR 显示）",
    "TunerName": "品牌 / 改装厂名",
    "XIAOMI_TITLE": "小米品牌年表标题",
    "XIAOMI_DESCRIPTION": "小米品牌年表正文",
    "SeasonInfo": "赛季活动信息",
}


def load(lang):
    from textpick import load_best
    return load_best(lang)


def load_alt(lang):
    p = ALT_DIR / f"{lang}.csv"
    out = {}
    if p.exists():
        with open(p, encoding="utf-8-sig", newline="") as fh:
            for r in csv.DictReader(fh):
                out[(r["section"], r["id"])] = r["text"]
    return out


_alt_lookup = {}


def clean(s: str) -> str:
    s = re.sub(r"<p[^>]*/>", "\n", s)
    s = re.sub(r"<p[^>]*>", "\n", s)
    s = re.sub(r"<next[^>]*>", "", s)
    s = re.sub(r"<yesno[^>]*>", "", s)
    s = re.sub(r"<sseq[^>]*>", "", s)
    s = re.sub(r"<br[^>]*>", "\n", s)
    s = re.sub(r"<([^<>/]+)>", r"\1", s)
    s = s.replace("\r\n", "\n").replace("\r", "\n")
    s = re.sub(r"\n{3,}", "\n\n", s)
    return s.strip()


def main() -> None:
    global _alt_lookup
    _alt_lookup = {(lang, k): v for lang in ("us", "zh") for k, v in load_alt(lang).items()}
    query = sys.argv[1] if len(sys.argv) > 1 else "SU7"
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else Path(
        r"C:\gt7\gt7\_re\reports\car_xiaomi_su7.md")
    us, zh = load("us"), load("zh")
    keys = set(us) | set(zh)
    hits = []
    q = query.lower()
    for k in keys:
        en = us.get(k, "")
        cn = zh.get(k, "")
        if q in en.lower() or query in cn or "xiaomi" in en.lower() or "小米" in cn:
            hits.append((k, clean(en), clean(cn)))

    order = {k: i for i, k in enumerate(SECTION_DESC)}
    hits.sort(key=lambda x: (order.get(x[0][0], 99), x[0][0], x[0][1]))

    lines = [f"# {query} — 游戏内中英文文本档案", ""]
    by_sec = {}
    for (sec, sid), en, cn in hits:
        by_sec.setdefault(sec, []).append((sid, en, cn))
    for sec, items in by_sec.items():
        lines.append(f"## {SECTION_DESC.get(sec, sec)}（{sec}，{len(items)} 条）")
        lines.append("")
        for sid, en, cn in items:
            lines.append(f"### `{sid}`")
            lines.append("")
            lines.append("**EN**")
            lines.append("")
            lines.append(en)
            lines.append("")
            lines.append("**中文**")
            lines.append("")
            lines.append(cn)
            lines.append("")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"{len(hits)} entries -> {out}")
    for sec, items in by_sec.items():
        print(f"  {sec:34} {len(items)}")


if __name__ == "__main__":
    main()
