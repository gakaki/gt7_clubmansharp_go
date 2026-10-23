#!/usr/bin/env python3
"""Test the 'unit direction per point' hypothesis for RADA sections."""

import math
import struct
import sys
from pathlib import Path


def main() -> None:
    d = Path(sys.argv[1]).read_bytes()
    starts = [0xD0, 0x1B10, 0x3510, 0x50F0, 0x6C00, 0x8610, 0x88B0, 0xA440]
    for s in starts:
        count = struct.unpack_from("<I", d, s)[0]
        f1 = struct.unpack_from("<3f", d, s + 0x0C)
        f2 = struct.unpack_from("<3f", d, s + 0x18)
        off1 = struct.unpack_from("<Q", d, s + 0x28)[0]
        off2 = struct.unpack_from("<Q", d, s + 0x30)[0]
        n = min(count, (off2 - off1) // 6)
        sx = sy = sz = 0.0
        mags = []
        for i in range(n):
            x, y, z = struct.unpack_from("<3h", d, off1 + i * 6)
            sx += x / 32767.0
            sy += y / 32767.0
            sz += z / 32767.0
            mags.append((x * x + y * y + z * z) ** 0.5 / 32767.0)
        mag = math.sqrt(sx * sx + sy * sy + sz * sz)
        print(f"0x{s:05X} count={count:5} n={n:5} off=0x{off1:X}..0x{off2:X} "
              f"sumdir=({sx:8.2f},{sy:8.2f},{sz:8.2f}) |sum|={mag:8.2f} "
              f"mean_mag={sum(mags)/len(mags):.3f} "
              f"f1={tuple(round(v,2) for v in f1)} f2={tuple(round(v,2) for v in f2)}")


if __name__ == "__main__":
    main()
