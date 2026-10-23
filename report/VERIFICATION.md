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

`gt7re verify`:

```text
PASS cars=591 withDialogue=332 dialogues=444 conversationLines=2620
vehicleCollectionCars=580 dealerOffers=65 menuBooks=108 avatars=19
```

`cargo test --release`: 9 passed, 0 failed.

`npm run build`: passed.

Production browser verification:

- Vehicle details and source filters: 15 / 15 checks passed.
- Multilingual dialogue switching and persistence: 9 / 9 checks passed,
  including Japanese + German, French + Italian and mobile layout.
- Cafe menu books: 17 / 17 checks passed.
- Searchable Python source files in `_re`: 0.

`cargo test --release`: 9 passed, 0 failed.

`gt7re pipeline`: 95,339 census files, 311 GPB4 entries, 311 DMSB files,
3,529 cafe messages, 1,698 scenes, 444 exact CarDetail rows, 634 text
entries, 2,620 scene lines, 65 dealer offers, 8,149 MDL5 files with 59,498
submodels, 4,074 JXL containers, 860 SNDZ containers, 0 unresolved cafe car
keys and 19 real character portraits.

`gt7re jxl-index`: 4,074 containers, 1,123,293,789 codestream bytes and
4,074 EXIF boxes.

`gt7re sndz-index`: 860 containers, all with known section tags; 18,053
`WAVS` and 857 `WAVD` sections indexed.

`gt7re mdl5-index`: 8,149 files, 59,498 submodel records, 7,279 valid model
tables, 6,215,273 identifiers, texture and shader offsets for every file.

Two consecutive full `gt7re pipeline` runs produced identical SHA-256 hashes
for the census, research JSON, dealer/vehicle/cafe CSV files, MDL5/JXL/SNDZ
indexes and avatar manifest.
