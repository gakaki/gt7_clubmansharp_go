#!/usr/bin/env python3
"""Pool all localisations to recover a much better shared RT2 keystream."""

import sys
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
from rt2_decode import build_model, load_payloads  # noqa: E402

COMMON = Path(r"C:\gt7\gt7\_re\extracted\text\rtext\common")
MAX_ROWS = 150_000


def main() -> None:
    out_path = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(r"C:\gt7\gt7\_re\keys\rt2_pooled.ks.bin")
    langs = set(sys.argv[2].split(",")) if len(sys.argv) > 2 else {"us", "gb"}
    files = sorted(f for f in COMMON.glob("*.rt2") if f.stem in langs)
    payloads = []
    for f in files:
        payloads.extend(load_payloads(f)[0])
    payloads.sort(key=lambda p: -len(p[2]))
    print(f"files={len(files)} payloads={len(payloads):,} "
          f"maxlen={max(len(p[2]) for p in payloads)}")

    L = max(len(p[2]) for p in payloads)
    model = build_model()
    ks = bytearray(L)
    chunk = 512
    for start in range(0, L, chunk):
        end = min(start + chunk, L)
        rows = [p for p in payloads if len(p[2]) > start]
        if len(rows) > MAX_ROWS:
            step = len(rows) / MAX_ROWS
            rows = [rows[int(i * step)] for i in range(MAX_ROWS)]
        width = end - start
        mat = np.zeros((len(rows), width), dtype=np.uint8)
        rlen = np.zeros(len(rows), dtype=np.int64)
        for r, (_, _, body) in enumerate(rows):
            seg = body[start:end]
            mat[r, :len(seg)] = np.frombuffer(seg, dtype=np.uint8)
            rlen[r] = len(seg)
        cols = np.broadcast_to(np.arange(width, dtype=np.int64)[None, :], mat.shape)
        valid = cols < (rlen[:, None] - 1)
        flat = cols[valid] * 256 + mat[valid].astype(np.int64)
        hist = np.bincount(flat, minlength=width * 256).reshape(width, 256)
        term = np.zeros(width, dtype=np.int64)
        lens = np.array([len(p[2]) for p in rows], dtype=np.int64)
        for pos in range(start, end):
            term[pos - start] = int((lens == pos + 1).sum())
        scores = hist.astype(np.float32) @ model.T
        scores += term[:, None].astype(np.float32) * model[:, 0][None, :]
        best = scores.argmax(axis=1)
        ks[start:end] = best.astype(np.uint8).tobytes()
        if start % 2048 == 0:
            print(f"  {end}/{L}")

    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_bytes(bytes(ks))
    print(f"keystream -> {out_path} ({len(ks)} bytes)")


if __name__ == "__main__":
    main()
