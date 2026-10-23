#!/usr/bin/env python3
"""Export the 253 legacy ambiguity records with exact resolutions."""

from __future__ import annotations

import json
from collections import Counter
from pathlib import Path


ROOT = Path(r"C:\gt7\gt7")
DATA = ROOT / r"_re\site\public\data\cars.json"
REPORT = ROOT / r"_re\reports\unmatched_dialogues.md"
PUBLIC_REPORT = ROOT / r"_re\site\public\data\unmatched_dialogues.md"
ID_MANIFEST = ROOT / r"_re\reports\legacy_review_ids.json"
CSV = ROOT / r"_re\reports\unmatched_dialogues.csv"


def _blockquote(text: str) -> str:
    lines = (text or "").replace("\r\n", "\n").replace("\r", "\n").split("\n")
    return "\n".join(f"> {line}" if line else ">" for line in lines)


def main() -> None:
    payload = json.loads(DATA.read_text(encoding="utf-8"))
    rows = sorted(
        payload.get("unmatched", []),
        key=lambda row: (
            row.get("resolvedCarNameEn") or "~unresolved",
            row.get("id") or "",
        ),
    )

    speaker_counts = Counter(row.get("speaker") or "Unknown" for row in rows)
    scene_keys = {
        scene
        for row in rows
        for scene in row.get("sceneKeys", [])
        if scene
    }
    resolution_counts = Counter(
        row.get("resolution") or "unknown" for row in rows
    )
    lines = [
        "# GT7 Cafe legacy CarDetail review",
        "",
        "These are the 253 records that could not be safely assigned by the "
        "previous heuristic scene reconstruction. They are now cross-checked "
        "against the exact GTCafe message, conversation, car-key and RT2 label "
        "tables. The exact result is shown here for human review.",
        "",
        f"- Legacy review records: **{len(rows)}**",
        f"- Exact-resolved records: **{sum(bool(row.get('resolved')) for row in rows)}**",
        f"- Distinct conversation scene keys: **{len(scene_keys)}**",
        f"- Source CSV: [{CSV.name}]({CSV.as_posix()})",
        "",
        "## Speaker summary",
        "",
        "| Speaker | Records |",
        "| --- | ---: |",
    ]
    lines.extend(
        f"| {speaker} | {count} |"
        for speaker, count in sorted(
            speaker_counts.items(), key=lambda item: (-item[1], item[0])
        )
    )
    lines.extend(["", "## Resolution summary", "", "| Resolution | Records |", "| --- | ---: |"])
    lines.extend(
        f"| {resolution} | {count} |"
        for resolution, count in sorted(resolution_counts.items())
    )
    lines.extend(["", "## Records grouped by exact car", ""])

    current_car = None
    for row in rows:
        car_en = row.get("resolvedCarNameEn") or "Unresolved"
        car_zh = row.get("resolvedCarNameZh") or ""
        if car_en != current_car:
            current_car = car_en
            title = f"### {car_en} · {car_zh}".rstrip(" ·")
            lines.extend([title, ""])

        candidate = row.get("car") or "unresolved"
        scenes = "; ".join(row.get("sceneKeys", [])) or "not linked to a recovered scene"
        lines.extend(
            [
                f"#### `{row.get('id') or 'unknown'}`",
                "",
                f"- Speaker: **{row.get('speaker') or 'Unknown'}** "
                f"({row.get('speakerZh') or row.get('speaker') or 'Unknown'})",
                f"- Legacy candidate: `{candidate}`",
                f"- Exact car key: `{row.get('carKey') or 'unknown'}`",
                f"- Exact resolution: `{row.get('resolution') or 'unknown'}`",
                f"- Conversation scene keys: `{scenes}`",
                "",
                "**English**",
                "",
                _blockquote(row.get("en") or ""),
                "",
                "**中文**",
                "",
                _blockquote(row.get("zh") or ""),
                "",
            ]
        )

    report_text = "\n".join(lines).rstrip() + "\n"
    for target in (REPORT, PUBLIC_REPORT):
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(report_text, encoding="utf-8")
    ID_MANIFEST.write_text(
        json.dumps([row.get("id") for row in rows if row.get("id")], indent=2),
        encoding="utf-8",
    )
    print(
        f"wrote {REPORT} and {PUBLIC_REPORT} "
        f"({len(rows)} records, {len(scene_keys)} scene keys)"
    )


if __name__ == "__main__":
    main()
