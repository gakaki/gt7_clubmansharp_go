#!/usr/bin/env python3
"""Search the PS5/GNM BC7 block layout for GT7 cafe portrait atlases."""

from __future__ import annotations

import argparse
import struct
from pathlib import Path

import numpy as np
import texture2ddecoder
from PIL import Image, ImageDraw


def morton(value: int, width: int, height: int) -> tuple[int, int]:
    x = y = 0
    step = 0
    while width > 1 or height > 1:
        if width > 1:
            x |= (value & 1) << step
            value >>= 1
            width >>= 1
        if height > 1:
            y |= (value & 1) << step
            value >>= 1
            height >>= 1
        step += 1
    return x, y


def stage_microtiles(
    source: np.ndarray,
    width: int,
    height: int,
    micro_w: int,
    micro_h: int,
    group_w: int,
    group_h: int,
    group_order: str,
) -> np.ndarray:
    """Map BC blocks from GFX10 microtiles to a linear block grid."""
    blocks = source.reshape(-1, 16)
    out = np.zeros((height, width, 16), dtype=np.uint8)
    miw, mih = micro_w, micro_h
    gx = (width + group_w - 1) // group_w
    gy = (height + group_h - 1) // group_h
    mw = (group_w + miw - 1) // miw
    mh = (group_h + mih - 1) // mih
    source_index = 0

    for gyi in range(gy):
        for gxi in range(gx):
            for mi in range(mw * mh):
                if group_order == "morton":
                    mx, my = morton(mi, mw, mh)
                elif group_order == "raster":
                    mx, my = mi % mw, mi // mw
                else:
                    raise ValueError(group_order)

                for micro_index in range(miw * mih):
                    # GFX10 128bpp Thin micro-tile bits: y0,y1,y2,y3,x0,x1.
                    bx = micro_index // mih
                    by = micro_index % mih
                    x = gxi * group_w + mx * miw + bx
                    y = gyi * group_h + my * mih + by
                    if source_index >= len(blocks):
                        break
                    if x < width and y < height:
                        out[y, x] = blocks[source_index]
                    source_index += 1
    return out


def decode_bc7(blocks: np.ndarray) -> Image.Image:
    height, width, _ = blocks.shape
    raw = np.ascontiguousarray(blocks).tobytes()
    rgba = texture2ddecoder.decode_bc7(raw, width * 4, height * 4)
    return Image.fromarray(
        np.frombuffer(rgba, dtype=np.uint8).reshape(height * 4, width * 4, 4),
        "RGBA",
    )


def within_block_offset(x: int, y: int, width: int, height: int, kind: str) -> int:
    if kind == "raster":
        return y * width + x
    if kind in {"standard", "morton"}:
        offset = 0
        output_bit = 0
        width_bits = width.bit_length() - 1
        height_bits = height.bit_length() - 1
        for bit in range(max(width_bits, height_bits)):
            axes = ((x, width_bits), (y, height_bits))
            if kind == "morton":
                axes = (axes[1], axes[0])
            for coordinate, count in axes:
                if bit < count:
                    offset |= ((coordinate >> bit) & 1) << output_bit
                    output_bit += 1
        return offset
    raise ValueError(kind)


def detile_4k(
    source: np.ndarray,
    width: int,
    height: int,
    block_width: int,
    block_height: int,
    kind: str,
) -> np.ndarray:
    blocks = source.reshape(-1, 16)
    out = np.zeros((height, width, 16), dtype=np.uint8)
    blocks_per_row = (width + block_width - 1) // block_width
    for y in range(height):
        for x in range(width):
            block_index = (y // block_height) * blocks_per_row + x // block_width
            inner = within_block_offset(
                x % block_width, y % block_height, block_width, block_height, kind
            )
            source_index = block_index * 256 + inner
            if source_index < len(blocks):
                out[y, x] = blocks[source_index]
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("input", type=Path)
    parser.add_argument(
        "--out",
        type=Path,
        default=Path(r"C:\gt7\gt7\_re\reports\avatar_gfx10_search.png"),
    )
    args = parser.parse_args()

    data = args.input.read_bytes()
    txs = data.find(b"TXS5")
    if txs < 0:
        raise SystemExit("no TXS5")
    payload = data[txs:]
    info = struct.unpack_from("<Q", payload, 0x28)[0]
    data_offset = struct.unpack_from("<I", payload, info)[0]
    data_size = struct.unpack_from("<I", payload, info + 8)[0]
    logical_w = struct.unpack_from("<H", payload, info + 0x10)[0]
    logical_h = struct.unpack_from("<H", payload, info + 0x12)[0]
    physical_w, physical_h = 480, 512
    raw = payload[data_offset:data_offset + data_size]
    blocks = np.frombuffer(raw, dtype=np.uint8)
    lw = logical_w // 4
    lh = (logical_h + 3) // 4
    pw = physical_w // 4
    ph = physical_h // 4
    variants: list[tuple[str, Image.Image]] = []
    # GT7's TXS5 stores mip 0 first, unlike the conventional GFX10
    # smallest-mip-first chain used by console memory descriptors.
    mip0_size = ((lw + 15) // 16) * ((lh + 15) // 16) * 4096
    base_mip = np.frombuffer(
        payload[data_offset:data_offset + mip0_size],
        dtype=np.uint8,
    )
    for block_w, block_h in (
        (1, 256), (2, 128), (4, 64), (8, 32), (16, 16),
        (32, 8), (64, 4), (128, 2), (256, 1),
    ):
        for kind in ("standard", "morton", "raster"):
            name = f"4KiB {block_w}x{block_h} {kind}"
            try:
                out = detile_4k(base_mip, lw, lh, block_w, block_h, kind)
                img = decode_bc7(out)
                variants.append((name, img))
            except Exception as exc:
                variants.append((f"{name}: {exc}", Image.new("RGBA", (lw * 4, lh * 4))))

    thumb_w, thumb_h = 176, 220
    label_h = 26
    cols = 4
    rows = (len(variants) + cols - 1) // cols
    sheet = Image.new("RGB", (cols * thumb_w, rows * (thumb_h + label_h)), "white")
    draw = ImageDraw.Draw(sheet)
    for index, (name, image) in enumerate(variants):
        image.thumbnail((thumb_w, thumb_h), Image.Resampling.LANCZOS)
        x = (index % cols) * thumb_w
        y = (index // cols) * (thumb_h + label_h)
        draw.text((x + 4, y + 4), name, fill="black")
        sheet.paste(image, (x, y + label_h), image if image.mode == "RGBA" else None)
    sheet.save(args.out)
    print(f"saved {args.out} variants={len(variants)}")
    print(f"logical={logical_w}x{logical_h} physical={physical_w}x{physical_h}")


if __name__ == "__main__":
    main()
