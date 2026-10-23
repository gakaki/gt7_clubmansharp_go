#!/usr/bin/env python3
"""Try to decrypt Polyphony .rt2 string blobs with known/derived keys."""

import sys
from collections import Counter
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "repos" / "gt7tool"))

from gt7tool.constants import (  # noqa: E402
    INDEX_CIPHER_KEY, VOLUME_CIPHER_KEY, SPECDB_CIPHER_KEY, DATABASE_CIPHER_KEY,
)
from Crypto.Cipher import ChaCha20, AES  # noqa: E402
from Crypto.Hash import SHA256  # noqa: E402


def parse(path: Path):
    data = path.read_bytes()
    count = int.from_bytes(data[4:8], "little")
    sections = []
    for i in range(count):
        off = 0x20 + i * 0x18
        name_off = int.from_bytes(data[off:off + 8], "little")
        n_entries = int.from_bytes(data[off + 8:off + 16], "little")
        entries_off = int.from_bytes(data[off + 16:off + 24], "little")
        name = data[name_off:data.find(b"\x00", name_off)].decode("ascii", "replace")
        entries = []
        for j in range(n_entries):
            e = entries_off + j * 0x18
            entries.append((
                int.from_bytes(data[e:e + 4], "little"),
                int.from_bytes(data[e + 4:e + 6], "little"),
                int.from_bytes(data[e + 6:e + 8], "little"),
                int.from_bytes(data[e + 8:e + 16], "little"),
                int.from_bytes(data[e + 16:e + 24], "little"),
            ))
        sections.append((name, n_entries, entries))
    return data, count, sections


def printable_ratio(b: bytes) -> float:
    if not b:
        return 0.0
    ok = sum(1 for x in b if 0x20 <= x < 0x7F or x in (9, 10, 13))
    return ok / len(b)


def utf16_printable_ratio(b: bytes) -> float:
    if len(b) < 2:
        return 0.0
    n = len(b) // 2
    ok = 0
    for i in range(n):
        ch = b[2 * i] | (b[2 * i + 1] << 8)
        if 0x20 <= ch < 0x7F or (0x3000 <= ch <= 0x9FFF) or (0xFF00 <= ch <= 0xFFEF):
            ok += 1
    return ok / n


def main() -> None:
    path = Path(sys.argv[1])
    data, count, sections = parse(path)

    # collect samples
    samples = []
    for name, n, entries in sections:
        for sid, f1, size, b_off, p_off in entries[:6]:
            samples.append((name, data[b_off:b_off + 16], data[p_off:p_off + size]))

    keys = {
        "INDEX": INDEX_CIPHER_KEY,
        "VOLUME": VOLUME_CIPHER_KEY,
        "SPECDB": SPECDB_CIPHER_KEY,
        "DATABASE": DATABASE_CIPHER_KEY,
        "sha(INDEX)": SHA256.new(INDEX_CIPHER_KEY).digest(),
        "sha(VOLUME)": SHA256.new(VOLUME_CIPHER_KEY).digest(),
    }

    print(f"samples: {len(samples)}")
    name, hdr, payload = samples[0]
    print(f"first: {name} hdr={hdr.hex()} size={len(payload)}")

    best = []
    for kname, key in keys.items():
        # ChaCha20 IETF, nonce = first 12 bytes of blob
        try:
            out = ChaCha20.new(key=key, nonce=hdr[0:12]).decrypt(payload)
            best.append((printable_ratio(out), f"chacha20-ietf/{kname} nonce=blob[0:12]", out))
        except Exception as exc:
            pass
        # ChaCha20 legacy 8-byte nonce
        try:
            out = ChaCha20.new(key=key, nonce=hdr[0:8]).decrypt(payload)
            best.append((printable_ratio(out), f"chacha20/{kname} nonce=blob[0:8]", out))
        except Exception:
            pass
        # AES ECB / CBC
        try:
            out = AES.new(key, AES.MODE_ECB).decrypt(payload[:len(payload) // 16 * 16])
            best.append((printable_ratio(out), f"aes-ecb/{kname}", out))
        except Exception:
            pass
        try:
            out = AES.new(key, AES.MODE_CBC, iv=hdr[0:16]).decrypt(payload[:len(payload) // 16 * 16])
            best.append((printable_ratio(out), f"aes-cbc/{kname} iv=blob[0:16]", out))
        except Exception:
            pass

    best.sort(key=lambda t: -t[0])
    print("\ntop attempts by printable ratio:")
    for ratio, label, out in best[:12]:
        print(f"  {ratio:5.2f}  {label:44} {out[:48]!r}")

    # distribution test: is the keystream identical across blobs?
    print("\nbyte-value distribution at payload offset 0..3 across all entries:")
    for pos in range(4):
        c = Counter()
        for nm, n, entries in sections:
            for sid, f1, size, b_off, p_off in entries:
                if pos < size:
                    c[data[p_off + pos]] += 1
        print(f"  pos {pos}: distinct={len(c)} top={c.most_common(5)}")

    print("\nutf16 ratio of raw payloads (sanity):",
          f"{utf16_printable_ratio(payload):.3f}")


if __name__ == "__main__":
    main()
