#!/usr/bin/env python3
"""Extract and decode the real GT7 Cafe character face atlases.

The PS5 face images are IDP0 containers around a TXS5 texture set.  Their
descriptors use GNM swizzle mode 9 (64 KiB Standard) with 16-byte BC7
elements.  This script implements that exact within-block AddrLib equation,
decodes BC7, and writes a PNG atlas for every Cafe character found in the
game's DMSB character table.
"""

from __future__ import annotations

import argparse
import json
import struct
import sys
import zlib
from dataclasses import dataclass
from pathlib import Path

import texture2ddecoder
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
GAME_CONTENTS = Path(r"C:\gt7\gt7\contents")


@dataclass(frozen=True)
class Character:
    key: str
    display: str
    character_id: str
    asset_path: str
    source_path: str | None = None


# Character IDs come from MsgCharacters.dmsb.  The source paths were resolved
# through the GT7 FNV-1a content-store cache key and can be checked with
# `gt7re resolve`.
CHARACTERS = [
    Character("luca", "Luca", "CafeMaster", "piece/4k/face/message_cafemaster_01.img"),
    Character("sarah", "Sarah Eaton", "Sarah", "piece/4k/face/message_sarah_01.img"),
    Character("stella", "Stella", "CarLover", "piece/4k/face/message_carlover_01.img"),
    Character("chris", "Chris", "ExpertOldMan1", "piece/4k/face/message_expertoldman1_01.img"),
    Character("jeremy", "Jeremy", "RaceEnthusiast", "piece/4k/face/message_raceenthusiast_01.img"),
    Character(
        "hideo-kodama",
        "Hideo Kodama",
        "CarDesignerProf1Kodama",
        "piece/4k/face/message_hideokodama_01.img",
    ),
    Character(
        "shiro-nakamura",
        "Shiro Nakamura",
        "CarDesignerProf2Nakamura",
        "piece/4k/face/message_shironakamura_01.img",
    ),
    Character(
        "kunihisa-ito",
        "Kunihisa Ito",
        "CarDesignerProf3Ito",
        "piece/4k/face/message_kunihisaito_01.img",
    ),
    Character(
        "hikonori-kagurazaka",
        "Hikonori Kagurazaka",
        "CarDesignerProf4Harada",
        "piece/4k/face/message_cardesignerprof4harada_01.img",
    ),
    Character(
        "tom-matano",
        "Tom Matano",
        "CarDesignerProf5Tom",
        "piece/4k/face/message_tommatano_01.img",
    ),
    Character(
        "fabio-filippini",
        "Fabio Filippini",
        "CarDesignerProf6Fabio",
        "piece/4k/face/message_fabiofilippini_01.img",
    ),
    Character(
        "ed-welburn",
        "Ed Welburn",
        "CarDesignerProf7Ed",
        "piece/4k/face/message_edwelburn_01.img",
    ),
    Character(
        "freeman-thomas",
        "Freeman Thomas",
        "CarDesignerProf8Freeman",
        "piece/4k/face/message_freemanthomas_01.img",
    ),
    Character(
        "ikuo-maeda",
        "Ikuo Maeda",
        "CarDesignerProf9Maeda",
        "piece/4k/face/message_ikuomaeda_01.img",
    ),
    Character(
        "kim-jones",
        "Kim Jones",
        "DiorKimJones",
        "piece/4k/face/message_diorkimjones_01.img",
    ),
    Character(
        "hagerty",
        "McKeel Hagerty",
        "Hagerty",
        "piece/4k/face/message_hagerty_01.img",
    ),
    Character(
        "used-car-owner",
        "Used Car Dealer",
        "UsedCarOwner",
        "piece/4k/face/message_usedcar_01.img",
    ),
    Character(
        "brand-central-advisor",
        "Brand Central Advisor",
        "BrandCentralOwner",
        "piece/4k/face/message_brandcentraladvisor_01.img",
    ),
    Character(
        "legend-car-owner",
        "Legend Car Owner",
        "LegendCarOwner",
        "piece/4k/face/message_legendcarowner_01.img",
    ),
]


def _add_script_path() -> None:
    scripts = str(ROOT / "scripts")
    if scripts not in sys.path:
        sys.path.insert(0, scripts)


def content_file(asset_path: str, contents: Path) -> Path:
    _add_script_path()
    import gt7store

    key = gt7store.compute_cache_key(gt7store.fnv1a(asset_path))
    return contents / key


def _pattern_axis(value: int, pattern: list[tuple[int, int]], use_x: bool) -> int:
    result = 0
    for bit, (x_mask, y_mask) in enumerate(pattern):
        mask = x_mask if use_x else y_mask
        result |= ((value & mask).bit_count() & 1) << bit
    return result


def _unpack_ps2zip(data: bytes) -> bytes:
    if len(data) < 8 or struct.unpack_from("<I", data, 0)[0] != 0xFFF7EEC5:
        return data
    size = struct.unpack_from("<i", data, 4)[0]
    if size >= 0:
        raise ValueError(f"invalid PS2ZIP size {size}")
    payload = data[8:]
    try:
        return zlib.decompress(payload)
    except zlib.error:
        return zlib.decompress(payload, -15)


def unwrap_idp0(data: bytes) -> bytes:
    data = _unpack_ps2zip(data)
    if data[:4] != b"IDP0":
        raise ValueError(f"expected IDP0, got {data[:16].hex()}")
    payload_offset, payload_size = struct.unpack_from("<QQ", data, 0x30)
    payload = data[payload_offset : payload_offset + payload_size]
    if payload[:4] != b"TXS5":
        raise ValueError(f"expected TXS5, got {payload[:16].hex()}")
    return payload


def gnm_mode9_bc7(raw: bytes, width: int, height: int) -> bytes:
    """Detile BC7 blocks from GNM swizzle mode 9 (64 KiB Standard).

    The descriptor stores dimensions in pixels; the hardware equation works on
    the 4x4 BC blocks.  A 64 KiB swizzle block contains 64x64 block elements.
    """

    zero = (0, 0)

    def x(bit: int) -> tuple[int, int]:
        return 1 << bit, 0

    def y(bit: int) -> tuple[int, int]:
        return 0, 1 << bit

    # GFX10_SW_64K_S pattern at 16 bytes per element.
    pattern = [
        zero,
        zero,
        zero,
        zero,
        y(0),
        y(1),
        x(0),
        x(1),
        y(2),
        x(2),
        y(3),
        x(3),
        y(4),
        x(4),
        y(5),
        x(5),
    ]
    block_w = (width + 3) // 4
    block_h = (height + 3) // 4
    macro_w = macro_h = 64
    macro_bytes = 65536
    blocks_per_row = (block_w + macro_w - 1) // macro_w
    x_terms = [_pattern_axis(value, pattern, True) for value in range(macro_w)]
    y_terms = [_pattern_axis(value, pattern, False) for value in range(macro_h)]

    # The game can omit unused tail bytes.  Missing blocks decode as transparent
    # black, while all stored blocks retain their exact byte offsets.
    linear = bytearray(block_w * block_h * 16)
    for by in range(block_h):
        for bx in range(block_w):
            block_index = (by // macro_h) * blocks_per_row + (bx // macro_w)
            source = block_index * macro_bytes + (
                x_terms[bx % macro_w] ^ y_terms[by % macro_h]
            )
            if source + 16 <= len(raw):
                target = (by * block_w + bx) * 16
                linear[target : target + 16] = raw[source : source + 16]
    return bytes(linear)


def decode_bc7(raw: bytes, width: int, height: int) -> Image.Image:
    padded_w = ((width + 3) // 4) * 4
    padded_h = ((height + 3) // 4) * 4
    rgba = bytearray(texture2ddecoder.decode_bc7(raw, padded_w, padded_h))
    for index in range(0, len(rgba), 4):
        rgba[index], rgba[index + 2] = rgba[index + 2], rgba[index]
    return Image.frombytes("RGBA", (padded_w, padded_h), rgba).crop(
        (0, 0, width, height)
    )


def decode_face_asset(asset_path: str, contents: Path) -> tuple[Image.Image, dict]:
    source = content_file(asset_path, contents)
    payload = unwrap_idp0(source.read_bytes())
    table = struct.unpack_from("<I", payload, 0x28)[0]
    data_offset = struct.unpack_from("<Q", payload, table)[0]
    data_size = struct.unpack_from("<I", payload, table + 8)[0]
    format_and_flags = struct.unpack_from("<I", payload, table + 0x0C)[0]
    width = struct.unpack_from("<H", payload, table + 0x10)[0]
    height = struct.unpack_from("<H", payload, table + 0x12)[0]
    image_format = format_and_flags & 0xFFFF
    flags = format_and_flags >> 16
    swizzle_mode = flags & 0x1F
    if swizzle_mode != 9:
        raise ValueError(f"{asset_path}: unsupported GNM swizzle mode {swizzle_mode}")
    if image_format != 0x0700:
        raise ValueError(f"{asset_path}: unsupported TXS5 format 0x{image_format:04X}")
    raw = payload[data_offset : data_offset + data_size]
    blocks = gnm_mode9_bc7(raw, width, height)
    image = decode_bc7(blocks, width, height)
    meta = {
        "assetPath": asset_path,
        "sourcePath": str(source),
        "sourceSize": source.stat().st_size,
        "txs5Size": len(payload),
        "dataOffset": data_offset,
        "dataSize": data_size,
        "format": f"0x{image_format:04X}",
        "flags": f"0x{flags:04X}",
        "swizzleMode": swizzle_mode,
        "width": width,
        "height": height,
        "pixelFormat": "BC7_UNORM",
    }
    return image, meta


def make_contact_sheet(items: list[tuple[Character, Image.Image]], out: Path) -> None:
    cell_w, cell_h = 240, 300
    cols = 4
    rows = (len(items) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * cell_w, rows * cell_h), (20, 20, 24))
    draw = ImageDraw.Draw(sheet)
    font = ImageFont.load_default()
    for index, (character, image) in enumerate(items):
        preview = image.copy()
        preview.thumbnail((cell_w - 20, cell_h - 48), Image.Resampling.LANCZOS)
        x = (index % cols) * cell_w + (cell_w - preview.width) // 2
        y = (index // cols) * cell_h + 32
        sheet.paste(preview, (x, y), preview)
        draw.text(
            ((index % cols) * cell_w + 10, (index // cols) * cell_h + 10),
            f"{character.display} [{character.character_id}]",
            fill=(240, 240, 240),
            font=font,
        )
    sheet.save(out)


def make_avatar(image: Image.Image) -> Image.Image:
    """Crop the full-face sprite shared by the GT7 character atlases.

    The texture is a real sprite atlas rather than a contact sheet.  Every
    character atlas uses the same layout for its primary head sprite.  Keep a
    small margin around the face and fit it to a square so it remains readable
    in the compact speaker filters.
    """

    face = image.crop((84, 0, 148, 64)).convert("RGB")
    return face.resize((128, 128), Image.Resampling.LANCZOS)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--contents", type=Path, default=GAME_CONTENTS, help="GT7 contents directory"
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=ROOT / "reports" / "face_atlases",
        help="output directory for decoded face atlases",
    )
    parser.add_argument(
        "--avatars-out",
        type=Path,
        default=ROOT / "site" / "public" / "avatars",
        help="output directory for primary character avatar crops",
    )
    args = parser.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    args.avatars_out.mkdir(parents=True, exist_ok=True)

    decoded: list[tuple[Character, Image.Image]] = []
    manifest = []
    for character in CHARACTERS:
        try:
            image, meta = decode_face_asset(character.asset_path, args.contents)
        except FileNotFoundError:
            print(f"MISSING {character.display}: {character.asset_path}")
            continue
        output = args.out / f"{character.key}.png"
        image.save(output)
        avatar = make_avatar(image)
        avatar_output = args.avatars_out / f"{character.key}.png"
        avatar.save(avatar_output, optimize=True)
        decoded.append((character, image))
        meta.update(
            {
                "key": character.key,
                "display": character.display,
                "characterId": character.character_id,
                "output": str(output),
                "avatarOutput": str(avatar_output),
                "avatarCrop": [84, 0, 148, 64],
            }
        )
        manifest.append(meta)
        print(
            f"OK {character.display}: {meta['width']}x{meta['height']} "
            f"mode={meta['swizzleMode']} -> {output}"
        )

    manifest_path = args.out / "manifest.json"
    manifest_path.write_text(
        json.dumps(
            {
                "generatedFrom": "GT7 PS5 IDP0/TXS5 + GNM mode 9 + BC7",
                "count": len(manifest),
                "entries": manifest,
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )
    make_contact_sheet(decoded, args.out / "contact_sheet.png")
    print(f"WROTE {manifest_path}")


if __name__ == "__main__":
    main()
