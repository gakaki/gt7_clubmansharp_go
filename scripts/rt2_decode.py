#!/usr/bin/env python3
"""Decode Polyphony .rt2 localisation files using the recovered fixed keystream."""

import argparse
import csv
import json
import math
import sys
from collections import Counter
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "scripts"))
from rt2_probe import parse  # noqa: E402

ROOT = Path(__file__).resolve().parents[1]
NUL = 0x00


def build_model() -> np.ndarray:
    from rt2_xor import char_model

    model = np.array(char_model(), dtype=np.float32)
    # 512x512 byte->byte score matrix: M[k, b] = model[b ^ k]
    k = np.arange(256, dtype=np.uint16)[:, None]
    b = np.arange(256, dtype=np.uint16)[None, :]
    return model[(b ^ k).astype(np.int64)].astype(np.float32)


def build_first_model() -> np.ndarray:
    """Character model for the first byte of a string (letters, '<', digits)."""
    w = np.full(256, 0.02, dtype=np.float32)
    for b in range(ord("A"), ord("Z") + 1):
        w[b] = 3.0
    for b in range(ord("a"), ord("z") + 1):
        w[b] = 1.2
    for b in range(ord("0"), ord("9") + 1):
        w[b] = 0.5
    for ch, wt in [("<", 2.5), ("\"", 0.6), ("'", 0.4), ("(", 0.3), ("[", 0.2),
                   ("{", 0.1), ("/", 0.2), ("-", 0.2), (" ", 0.05)]:
        w[ord(ch)] = wt
    w[0] = 0.001
    k = np.arange(256, dtype=np.uint16)[:, None]
    b = np.arange(256, dtype=np.uint16)[None, :]
    return w[(b ^ k).astype(np.int64)].astype(np.float32)


FIRST_MAT = build_first_model()


def load_payloads(path: Path):
    data, count, sections = parse(path)
    out = []
    for sec in sections:
        name, entries = sec[0], sec[3]
        for sid, f1, size, b_off, p_off in entries:
            if size > 0:
                out.append((name, sid, data[p_off:p_off + size]))
    return out, data


def recover_keystream(payloads, maxlen: int, model_mat: np.ndarray, chunk: int = 4096):
    lens = np.array([len(p) for _, _, p in payloads], dtype=np.int64)
    L = int(min(maxlen, lens.max()))
    ks = np.zeros(L, dtype=np.uint8)
    conf = np.zeros(L, dtype=np.float32)

    order = np.argsort(-lens)  # longest first => most samples per deep column
    for start in range(0, L, chunk):
        end = min(start + chunk, L)
        rows = [i for i in order if lens[i] > start]
        if not rows:
            break
        width = end - start
        mat = np.zeros((len(rows), width), dtype=np.uint8)
        rowlen = np.zeros(len(rows), dtype=np.int64)
        for r, i in enumerate(rows):
            seg = payloads[i][2][start:end]
            mat[r, :len(seg)] = np.frombuffer(seg, dtype=np.uint8)
            rowlen[r] = len(seg)
        cols = np.broadcast_to(np.arange(width, dtype=np.int64)[None, :], mat.shape)
        # A payload of length n has a NUL terminator at index n-1, so only
        # bytes with (n-1) > index are text; the terminator is scored separately.
        valid = cols < (rowlen[:, None] - 1)
        flat = (cols[valid] * 256 + mat[valid].astype(np.int64))
        hist = np.bincount(flat, minlength=width * 256).reshape(width, 256)
        term_count = np.zeros(width, dtype=np.int64)
        for pos in range(start, end):
            term_count[pos - start] = int((lens == pos + 1).sum())
        # scores[c, k] = sum_b hist[c,b] * M[k,b]
        scores = hist.astype(np.float32) @ model_mat.T
        # add terminator evidence: plaintext 0 at those positions
        scores += term_count[:, None].astype(np.float32) * model_mat[:, 0][None, :]
        if start == 0:
            # first character: letters / '<' / digit, never a space
            scores[0] = hist[0].astype(np.float32) @ FIRST_MAT.T
        best = scores.argmax(axis=1)
        top2 = np.partition(scores, -2, axis=1)[:, -2:]
        ks[start:end] = best.astype(np.uint8)
        conf[start:end] = (top2[:, 1] - top2[:, 0]) / np.maximum(hist.sum(axis=1), 1)
    return ks, conf


def decrypt(payload: bytes, ks: np.ndarray) -> bytes:
    n = min(len(payload), len(ks))
    out = bytearray(payload)
    for i in range(n):
        out[i] ^= ks[i]
    return bytes(out)


def cmd_stats(args) -> None:
    payloads, _ = load_payloads(Path(args.file))
    lens = sorted((len(p) for _, _, p in payloads), reverse=True)
    print(f"file: {args.file}")
    print(f"payloads: {len(lens):,}  max={lens[0]:,} min={lens[-1]}")
    print(f"top 10 lengths: {lens[:10]}")
    for t in (256, 512, 1024, 2048, 4096, 8192, 16384):
        n = sum(1 for l in lens if l >= t)
        print(f"  payloads >= {t:6}: {n:,}")


def cmd_recover(args) -> None:
    model_mat = build_model()
    all_payloads = []
    per_file = {}
    for f in args.files:
        p = Path(f)
        payloads, _ = load_payloads(p)
        per_file[p.name] = payloads
        all_payloads.extend(payloads)
        print(f"{p.name}: {len(payloads):,} payloads")

    maxlen = min(args.maxlen, max(len(pl[2]) for pl in all_payloads))
    print(f"recovering keystream, maxlen={maxlen:,}")
    ks, conf = recover_keystream(all_payloads, maxlen, model_mat)
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(ks.tobytes())
    print(f"keystream -> {out} ({len(ks):,} bytes)")
    print(f"mean confidence: {conf.mean():.4f}")
    low = np.where(conf < 0.05)[0]
    print(f"low-confidence offsets: {len(low)} (first: {low[:20]})")

    if len(args.files) > 1:
        print("\nper-file keystream agreement (vs pooled):")
        for fname, payloads in per_file.items():
            k2, c2 = recover_keystream(payloads, maxlen, model_mat)
            n = min(len(k2), len(ks))
            m = c2[:n] > 0.05
            agree = (k2[:n][m] == ks[:n][m]).mean() if m.any() else 0.0
            print(f"  {fname:14} confident={m.sum():6} agreement={100*agree:.1f}%")


def cmd_decode(args) -> None:
    ks = np.frombuffer(Path(args.keystream).read_bytes(), dtype=np.uint8)
    payloads, _ = load_payloads(Path(args.file))
    sec_out: dict[str, dict[str, str]] = {}
    bad = 0
    rows = []
    for name, sid, payload in payloads:
        dec = decrypt(payload, ks)
        dec = dec.rstrip(b"\x00")
        try:
            text = dec.decode("utf-8")
        except UnicodeDecodeError:
            text = dec.decode("utf-8", "replace")
            bad += 1
        sec_out.setdefault(name, {})[f"0x{sid:08X}"] = text
        rows.append({"section": name, "id": f"0x{sid:08X}", "text": text})

    outdir = Path(args.out)
    outdir.mkdir(parents=True, exist_ok=True)
    stem = Path(args.file).stem
    (outdir / f"{stem}.json").write_text(
        json.dumps(sec_out, ensure_ascii=False, indent=1), encoding="utf-8")
    with open(outdir / f"{stem}.csv", "w", newline="", encoding="utf-8-sig") as fh:
        w = csv.DictWriter(fh, fieldnames=["section", "id", "text"])
        w.writeheader()
        w.writerows(rows)
    print(f"{stem}: {len(rows):,} strings, {len(sec_out)} sections, "
          f"undecodable {bad} -> {outdir}")
    for name in list(sec_out)[:3]:
        sample = list(sec_out[name].items())[:1]
        for sid, text in sample:
            print(f"  {name} {sid}: {text[:90]!r}")


def cmd_auto(args) -> None:
    """Recover the keystream from each file individually and decode it."""
    model_mat = build_model()
    outdir = Path(args.out)
    outdir.mkdir(parents=True, exist_ok=True)
    keydir = Path(args.keydir)
    keydir.mkdir(parents=True, exist_ok=True)

    for f in args.files:
        p = Path(f)
        payloads, _ = load_payloads(p)
        maxlen = max(len(x[2]) for x in payloads)
        ks, conf = recover_keystream(payloads, maxlen, model_mat)
        (keydir / f"{p.stem}.rt2.ks.bin").write_bytes(ks.tobytes())

        sec_out: dict[str, dict[str, str]] = {}
        rows = []
        valid = 0
        for name, sid, payload in payloads:
            dec = bytearray(payload)
            n = min(len(payload), len(ks))
            for i in range(n):
                dec[i] ^= ks[i]
            # Payload layout: [1 tag byte][text bytes][optional NUL pad].
            # The tag byte varies and is not recoverable by frequency analysis,
            # so it is dropped here.
            dec = bytes(dec).rstrip(b"\x00")
            try:
                text = dec.decode("utf-8")
                valid += 1
            except UnicodeDecodeError:
                text = dec.decode("utf-8", "replace")
            sec_out.setdefault(name, {})[f"0x{sid:08X}"] = text
            rows.append({"section": name, "id": f"0x{sid:08X}", "text": text})

        (outdir / f"{p.stem}.json").write_text(
            json.dumps(sec_out, ensure_ascii=False, indent=1), encoding="utf-8")
        with open(outdir / f"{p.stem}.csv", "w", newline="", encoding="utf-8-sig") as fh:
            w = csv.DictWriter(fh, fieldnames=["section", "id", "text"])
            w.writeheader()
            w.writerows(rows)
        print(f"{p.stem}: {len(rows):,} strings, valid utf-8 {valid:,} "
              f"({100.0*valid/max(len(rows),1):.1f}%), keystream {len(ks):,} bytes")
        longest = sorted(payloads, key=lambda x: -len(x[2]))[:2]
        for name, sid, payload in longest:
            dec = bytes(b ^ ks[i] for i, b in enumerate(payload[:len(ks)])).rstrip(b"\x00")
            print(f"   [{name} 0x{sid:08X} len={len(payload)}] "
                  f"{dec.decode('utf-8','replace')[:160]!r}")


def cmd_dump(args) -> None:
    """Show decrypted samples for specific sections."""
    if args.keystream == "auto":
        model_mat = build_model()
        payloads, _ = load_payloads(Path(args.file))
        ks, _c = recover_keystream(payloads, max(len(p[2]) for p in payloads), model_mat)
    else:
        ks = np.frombuffer(Path(args.keystream).read_bytes(), dtype=np.uint8)
        payloads, _ = load_payloads(Path(args.file))

    wanted = set(args.section)
    shown = 0
    for name, sid, p in payloads:
        if wanted and name not in wanted:
            continue
        d = bytes(b ^ ks[i] for i, b in enumerate(p[:len(ks)])).rstrip(b"\x00")
        print(f"[{name} 0x{sid:08X} len={len(p)}]")
        print(f"   raw0={p[:8].hex()}  dec={d[:120]!r}")
        shown += 1
        if shown >= args.limit:
            break


def main() -> None:
    ap = argparse.ArgumentParser()
    sub = ap.add_subparsers(dest="cmd", required=True)

    p = sub.add_parser("stats")
    p.add_argument("file")
    p.set_defaults(func=cmd_stats)

    p = sub.add_parser("recover")
    p.add_argument("files", nargs="+")
    p.add_argument("--maxlen", type=int, default=1 << 20)
    p.add_argument("--out", default=str(ROOT / "keys" / "rt2_keystream.bin"))
    p.set_defaults(func=cmd_recover)

    p = sub.add_parser("decode")
    p.add_argument("file")
    p.add_argument("--keystream", default=str(ROOT / "keys" / "rt2_keystream.bin"))
    p.add_argument("-o", "--out", required=True)
    p.set_defaults(func=cmd_decode)

    p = sub.add_parser("auto")
    p.add_argument("files", nargs="+")
    p.add_argument("-o", "--out", required=True)
    p.add_argument("--keydir", default=str(ROOT / "keys"))
    p.set_defaults(func=cmd_auto)

    p = sub.add_parser("dump")
    p.add_argument("file")
    p.add_argument("--keystream", default="auto")
    p.add_argument("--section", action="append", default=[])
    p.add_argument("--limit", type=int, default=8)
    p.set_defaults(func=cmd_dump)

    args = ap.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
