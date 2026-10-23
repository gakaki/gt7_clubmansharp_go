#!/usr/bin/env python3
"""Trim whitespace around brand logos and normalise their size."""

from pathlib import Path

from PIL import Image, ImageChops

ROOT = Path(r"C:\gt7\gt7\_re\site\public\logos")


def main() -> None:
    n = 0
    for p in ROOT.glob("*.png"):
        try:
            im = Image.open(p).convert("RGBA")
            bg = Image.new("RGBA", im.size, (255, 255, 255, 255))
            diff = ImageChops.difference(im, bg).convert("L")
            mask = diff.point(lambda v: 255 if v > 12 else 0)
            bbox = mask.getbbox()
            if bbox:
                im = im.crop(bbox)
            im.thumbnail((320, 160), Image.LANCZOS)
            im.save(p)
            n += 1
        except Exception as exc:
            print("skip", p.name, exc)
    print("trimmed", n)


if __name__ == "__main__":
    main()
