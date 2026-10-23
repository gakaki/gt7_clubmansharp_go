#!/usr/bin/env python3
"""Recover the fixed keystream used for Polyphony .rt2 string payloads.

Observation: for any fixed payload byte offset, the ciphertext byte values are
limited to ~80-100 distinct values with a strongly skewed distribution.  That is
the signature of plaintext XOR a *fixed* keystream (reused across all blobs),
not of a per-blob stream or a block cipher.  So per offset we can recover the
keystream byte by single-byte frequency analysis.
"""

import argparse
import math
import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rt2_probe import parse  # noqa: E402


def char_model() -> list[float]:
    """log2 probability per byte, English-ish plus UTF-8 continuation bytes."""
    freq = {}
    for ch, w in [
        (" ", 18.0), ("e", 10.2), ("t", 7.5), ("a", 6.6), ("o", 6.1), ("i", 5.7),
        ("n", 5.5), ("s", 5.2), ("r", 5.0), ("h", 5.0), ("l", 3.3), ("d", 3.3),
        ("c", 2.3), ("u", 2.3), ("m", 2.0), ("f", 1.8), ("p", 1.6), ("g", 1.6),
        ("w", 1.6), ("y", 1.5), ("b", 1.2), ("v", 0.8), ("k", 0.6), ("x", 0.15),
        ("j", 0.12), ("z", 0.07), ("q", 0.08),
        ("E", 1.0), ("T", 0.9), ("A", 0.9), ("O", 0.7), ("I", 0.6), ("N", 0.6),
        ("S", 0.6), ("R", 0.6), ("H", 0.5), ("L", 0.4), ("D", 0.4), ("C", 0.4),
        ("U", 0.3), ("M", 0.3), ("P", 0.3), ("B", 0.25), ("G", 0.25), ("F", 0.2),
        ("W", 0.2), ("Y", 0.2), ("V", 0.1), ("K", 0.1), ("J", 0.08), ("X", 0.03),
        ("Z", 0.03), ("Q", 0.03),
        (".", 1.2), (",", 1.2), ("'", 0.8), ("-", 0.5), ("\"", 0.4), ("(", 0.2),
        (")", 0.2), (":", 0.2), (";", 0.15), ("!", 0.15), ("?", 0.15), ("/", 0.15),
        ("&", 0.1), ("%", 0.1), ("+", 0.08), ("=", 0.08), ("*", 0.05), ("@", 0.03),
        ("#", 0.03), ("$", 0.03), ("_", 0.05), ("|", 0.02), ("~", 0.02), ("<", 0.02),
        (">", 0.02), ("[", 0.02), ("]", 0.02), ("{", 0.01), ("}", 0.01), ("^", 0.01),
        ("`", 0.01), ("\\", 0.01), ("0", 0.3), ("1", 0.3), ("2", 0.25), ("3", 0.2),
        ("4", 0.15), ("5", 0.15), ("6", 0.15), ("7", 0.13), ("8", 0.13), ("9", 0.13),
        ("\n", 0.5), ("\r", 0.3), ("\t", 0.1),
    ]:
        freq[ord(ch)] = freq.get(ord(ch), 0.0) + w

    # UTF-8 multibyte lead/continuation bytes: plausible for accents / CJK
    for b in list(range(0xC2, 0xF5)):
        freq.setdefault(b, 2.0)
    for b in range(0x80, 0xC0):
        freq.setdefault(b, 3.0)

    floor = 0.002
    return [math.log2(freq.get(b, floor)) for b in range(256)]


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("file")
    ap.add_argument("--limit", type=int, default=4096, help="max payload offsets to recover")
    ap.add_argument("--show", type=int, default=6, help="payloads to print decoded")
    ap.add_argument("--maxlen", type=int, default=400, help="chars of decoded samples to print")
    args = ap.parse_args()

    data, count, sections = parse(Path(args.file))
    model = char_model()

    payloads = []
    for sec in sections:
        name, entries = sec[0], sec[3]
        for sid, f1, size, b_off, p_off in entries:
            if size > 0:
                payloads.append((name, sid, data[p_off:p_off + size]))
    print(f"payloads: {len(payloads):,}")

    maxlen = min(args.limit, max(len(p) for _, _, p in payloads))
    print(f"recovering keystream for first {maxlen:,} offsets")

    keystream = bytearray()
    confidence = []
    for i in range(maxlen):
        c = Counter()
        for _, _, p in payloads:
            if len(p) > i:
                c[p[i]] += 1
        best_k, best_s = 0, -1e30
        second = -1e30
        for k in range(256):
            s = 0.0
            for b, n in c.items():
                s += n * model[b ^ k]
            if s > best_s:
                best_k, best_s, second = k, s, best_s
            elif s > second:
                second = s
        keystream.append(best_k)
        confidence.append(best_s - second)

    out = Path(args.file + ".keystream.bin")
    out.write_bytes(bytes(keystream))
    print(f"keystream -> {out}")
    print(f"keystream[:64] = {bytes(keystream[:64]).hex()}")

    # periodicity check: does a shorter period explain the keystream?
    for period in (8, 16, 24, 32, 48, 64, 128, 256, 512, 1024):
        if period * 3 > maxlen:
            break
        agree = sum(1 for i in range(period, maxlen) if keystream[i] == keystream[i % period])
        print(f"  period {period:5}: agreement {agree}/{maxlen - period} "
              f"({100.0*agree/(maxlen-period):.1f}%)")

    print("\ndecoded samples:")
    for name, sid, p in payloads[:args.show]:
        dec = bytes(b ^ keystream[i] for i, b in enumerate(p[:maxlen]))
        head = min(len(dec), args.maxlen)
        print(f"\n--- {name} id=0x{sid:08X} len={len(p)}")
        try:
            print(dec[:head].decode("utf-8", "replace")[:args.maxlen])
        except Exception as exc:
            print(f"decode error {exc}")


if __name__ == "__main__":
    main()
