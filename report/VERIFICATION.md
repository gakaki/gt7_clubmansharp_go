# gt7re smoke test report

Time: 2026-10-17 10:54:20

| case | result |
|---|---|
| resolve | PASS |
| stats | PASS |
| rt2-info | PASS |
| rt2-decode | PASS |
| rt2-recover | PASS |
| packed | PASS |
| udp-layout | PASS |
| map4 | PASS |
| rada | PASS |
| txs5-index | PASS |
| idp0-extract | PASS |
| strings | PASS |
| catalog | PASS |
| extract-kind | PASS |
| mdl5-index | PASS |
| link-all | PASS |

Result: 16 / 16 PASS

Artifacts: rust/out/smoke

## Vehicle / cafe data validation (2026-10-23)

`python scripts\verify_site_data.py`:

```text
PASS cars=591 with_dialogue=329 dialogues=444 scene_lines=2620 mapped=444
images=582 speakers=14 car_texts=634 text_cars=581 review=253
```

`cargo test --release`: 9 passed, 0 failed.

`gt7re pipeline`: 311 GPB4 entries, 311 DMSB files, 3,529 cafe messages,
1,698 scenes, 444 exact CarDetail rows, 634 text entries, 2,620 scene lines,
65 dealer offers, 0 unresolved cafe car keys and 19 real character portraits.

`python scripts\verify_avatars.py`:

```text
PASS avatars=19 cafe=15 unique_hashes=19 min_pair_distance=5
```

Two consecutive `gt7re pipeline` runs produced identical SHA-256 hashes for
the research JSON, dealer/vehicle/cafe CSV files and avatar manifest.
