#!/usr/bin/env python3
"""Attempt to decode GT7 RADA (course_line) point streams."""

import math
import struct
import sys
from pathlib import Path


def u32(d, o):
    return struct.unpack_from("<I", d, o)[0]


def f32(d, o):
    return struct.unpack_from("<f", d, o)[0]


def i16(d, o):
    return struct.unpack_from("<h", d, o)[0]


def main() -> None:
    path = Path(sys.argv[1])
    d = path.read_bytes()
    print(f"{path.name}: {len(d)} bytes")
    print(f"magic={d[:4]!r} declared={u32(d,4)}")
    for o in range(0x10, 0xD0, 4):
        v = u32(d, o)
        if v:
            print(f"  0x{o:03X}: {v} (0x{v:X})")
    print(f"count@0xD0 = {u32(d,0xD0)}")
    print(f"floats@0xDC = {[round(f32(d,o),3) for o in range(0xDC,0x100,4)]}")

    # candidate section [0x180, next table offset 0x1B10)
    a, b = 0x180, 0x1B10
    sec = d[a:b]
    print(f"section 0x{a:X}..0x{b:X}: {len(sec)} bytes")

    # periodicity probe
    best = []
    for period in range(4, 33):
        same = 0
        total = 0
        for i in range(0, min(len(sec) - period, 4096)):
            total += 1
            if sec[i] == sec[i + period]:
                same += 1
        best.append((same / max(total, 1), period))
    best.sort(reverse=True)
    print("periodicity:", [(p, round(s, 3)) for s, p in best[:6]])

    count = u32(d, 0xD0)
    start = (f32(d, 0xE4), f32(d, 0xE8), f32(d, 0xEC))
    print(f"candidate start {start}")

    for stride in (6, 8, 10, 12, 16):
        need = count * stride
        if need > len(sec):
            print(f"  stride {stride}: needs {need} > {len(sec)}")
            continue
        for scale in (1.0, 1 / 64, 1 / 256, 1 / 1024, 1 / 4096, 1 / 16384):
            x, y, z = start
            pts = []
            ok = True
            for i in range(count):
                o = i * stride
                dx = i16(sec, o)
                dy = i16(sec, o + 2)
                dz = i16(sec, o + 4)
                x += dx * scale
                y += dy * scale
                z += dz * scale
                pts.append((x, y, z))
            dx = pts[-1][0] - pts[0][0]
            dy = pts[-1][1] - pts[0][1]
            dz = pts[-1][2] - pts[0][2]
            closure = math.sqrt(dx * dx + dy * dy + dz * dz)
            xs = [p[0] for p in pts]
            zs = [p[2] for p in pts]
            span = max(max(xs) - min(xs), max(zs) - min(zs))
            length = sum(
                math.dist(pts[i], pts[i + 1]) for i in range(len(pts) - 1)
            )
            if span < 1e6 and length < 100_000:
                print(f"  stride {stride:2} scale {scale:<9.6f} span={span:8.1f} "
                      f"len={length:8.1f} closure={closure:7.2f}")


if __name__ == "__main__":
    main()
