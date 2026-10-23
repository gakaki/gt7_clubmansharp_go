#!/usr/bin/env python3
"""Diagnostic: decrypt gt.idx with the known PDIPFS index cipher and dump raw header bytes."""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "repos" / "gt7tool"))

from gt7tool.constants import INDEX_CIPHER_KEY, INDEX_CIPHER_IV, INDEX_HEADER_SIZE
from gt7tool.crypto import chacha20_decrypt, crc32_decrypt


def hexdump(data: bytes, width: int = 16) -> str:
    out = []
    for off in range(0, len(data), width):
        chunk = data[off:off + width]
        hexpart = " ".join(f"{b:02X}" for b in chunk).ljust(width * 3 - 1)
        asc = "".join(chr(b) if 0x20 <= b < 0x7F else "." for b in chunk)
        out.append(f"{off:08X}  {hexpart}  {asc}")
    return "\n".join(out)


def main() -> None:
    path = sys.argv[1] if len(sys.argv) > 1 else r"C:\gt7\gt7\contents\gt.idx"
    data = Path(path).read_bytes()
    print(f"file: {path}")
    print(f"size: {len(data):,} (0x{len(data):X})")

    dec = chacha20_decrypt(INDEX_CIPHER_KEY, INDEX_CIPHER_IV, data)
    print("\n=== chacha20(index key) first 0x80 ===")
    print(hexdump(dec[:0x80]))

    crc = crc32_decrypt(dec[:INDEX_HEADER_SIZE])
    print("\n=== crc32_decrypt(chacha20) first 0x100 ===")
    print(hexdump(crc[:0x100]))

    print("\n=== crc32_decrypt only (no chacha) first 0x100 ===")
    print(hexdump(crc32_decrypt(data[:INDEX_HEADER_SIZE])[:0x100]))

    print("\n=== first 0x20 bytes interpreted as u32le ===")
    for i in range(0, 0x20, 4):
        v = int.from_bytes(dec[i:i + 4], "little")
        print(f"  off 0x{i:02X} (chacha) = 0x{v:08X}")
    for i in range(0, 0x20, 4):
        v = int.from_bytes(crc[i:i + 4], "little")
        print(f"  off 0x{i:02X} (crc)    = 0x{v:08X}")


if __name__ == "__main__":
    main()
