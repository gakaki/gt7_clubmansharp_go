#!/usr/bin/env python3
"""Load the cleanest decoding of every RT2 string across all recovered keys."""

import csv
import difflib
import functools
import re
from pathlib import Path

DIRS = [
    Path(r"C:\gt7\gt7\_re\extracted\text_decoded"),
    Path(r"C:\gt7\gt7\_re\extracted\text_pooled"),
    Path(r"C:\gt7\gt7\_re\extracted\text_all"),
    Path(r"C:\gt7\gt7\_re\extracted\text_patch_decoded"),
]


def _prose(text: str) -> str:
    """Approximate the visible text so damaged markup does not outrank prose."""

    text = text.replace("\r\n", "\n").replace("\r", "\n")
    text = re.sub(r"<[^>]*>", " ", text)
    text = re.sub(r"[^\w\s]\s*(?:sseq|seq|p|next|yesno)\b[^>]*>", " ", text,
                  flags=re.IGNORECASE)
    return re.sub(r"\s+", " ", text).strip()


def _quality(text: str) -> tuple[int, int, int, int]:
    """Rank candidates by full, readable prose rather than raw string length.

    The older loader preferred the longest decoding whenever replacement-char
    counts tied.  Some recovery passes retain a complete body but corrupt only
    a trailing markup tag, while others have fewer tag errors but a truncated
    sentence.  Visible prose is therefore the primary signal.
    """

    body = _prose(text)
    readable = sum(ch.isalnum() for ch in body)
    replacement = body.count("\ufffd")
    well_formed_tags = len(re.findall(r"<(?:sseq|p|next|yesno)\b[^>]*>", text,
                                     flags=re.IGNORECASE))
    valid_ids = len(re.findall(r"\b(?:id|msgid|nmsgid)=\d{6,8}\b", text))
    return (readable - replacement * 80, well_formed_tags, valid_ids, len(body))


def _repair_replacements(best: str, alternatives: list[str]) -> str:
    """Fill replacement characters from independently decoded candidates.

    The recovery passes fail on different bytes.  The structurally best
    candidate generally keeps the full control markup, while another pass may
    retain a character that the best pass lost.  Only substitute when the
    surrounding characters align, which prevents splicing unrelated strings.
    """

    if "\ufffd" not in best:
        return best

    repaired = best
    for candidate in alternatives:
        if candidate == repaired or "\ufffd" not in repaired:
            continue
        matcher = difflib.SequenceMatcher(None, repaired, candidate, autojunk=False)
        pieces = []
        changed = False
        cursor = 0
        for tag, left_a, right_a, left_b, right_b in matcher.get_opcodes():
            pieces.append(repaired[cursor:left_a])
            old = repaired[left_a:right_a]
            new = candidate[left_b:right_b]
            if (
                tag == "replace"
                and "\ufffd" in old
                and new
                and "\ufffd" not in new
                and len(old) <= 16
                and len(new) <= 16
            ):
                pieces.append(new)
                changed = True
            else:
                pieces.append(old)
            cursor = right_a
        pieces.append(repaired[cursor:])
        if changed:
            repaired = "".join(pieces)

    chars = list(repaired)
    others = [value for value in alternatives if value != repaired]
    for index, char in enumerate(chars):
        if char != "\ufffd":
            continue
        for candidate in others:
            if index >= len(candidate) or candidate[index] == "\ufffd":
                continue
            left_start = max(0, index - 8)
            left = repaired[left_start:index]
            right = repaired[index + 1:index + 9]
            same_left = sum(
                value == candidate[left_start + offset]
                for offset, value in enumerate(left)
                if value != "\ufffd"
            )
            same_right = sum(
                value == candidate[index + 1 + offset]
                for offset, value in enumerate(right)
                if value != "\ufffd"
                and index + 1 + offset < len(candidate)
            )
            context = sum(value != "\ufffd" for value in left + right)
            needed = min(6, max(1, context))
            if same_left + same_right >= needed:
                chars[index] = candidate[index]
                break
    return "".join(chars)


@functools.lru_cache(maxsize=None)
def load_best(lang: str) -> dict:
    best = {}
    candidates: dict[tuple[str, str], list[tuple[int, str]]] = {}
    aliases = {
        "us": ("us", "gb"),
        "zh": ("cn", "zh"),
    }.get(lang, (lang,))
    for d in DIRS:
        for alias in aliases:
            p = d / f"{alias}.csv"
            if not p.exists():
                continue
            with open(p, encoding="utf-8-sig", newline="") as fh:
                for r in csv.DictReader(fh):
                    key = (r["section"], r["id"])
                    text = r["text"]
                    normalized = text.replace("\r\n", "\n").replace("\r", "\n")
                    candidates.setdefault(key, []).append(
                        (_quality(normalized), normalized)
                    )
    for key, choices in candidates.items():
        # Stable sort keeps the first, generally most structurally complete
        # recovery pass, when prose quality is equal.
        best_text = max(choices, key=lambda item: item[0])[1]
        best[key] = _repair_replacements(
            best_text,
            [text for _, text in sorted(choices, key=lambda item: item[0], reverse=True)],
        )
    return best
