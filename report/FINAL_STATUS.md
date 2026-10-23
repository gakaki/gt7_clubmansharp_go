# GT7 PS5 extraction and research status

This report records the current, evidence-backed state of the local research
tree. Raw game data remains under `C:\gt7\gt7\contents`; generated artifacts
remain under `C:\gt7\gt7\_re`.

## Verified outputs

| Area | Current result | Evidence |
| --- | ---: | --- |
| Content-store files | 95,339 / 282.03 GiB | `reports/store_census_rust.csv` |
| Named asset mappings | 12,279 | `reports/named.txt` |
| Localisation | 19 languages x 42,638 strings | `extracted/text_decoded/*.csv` |
| Menu Adhoc disassembly | 1,906 files / 1,325,950 lines | `reports/gt7_menu_scripts_diss.zip` |
| Menu disassembly hashes | 1,906 SHA-256 rows | `reports/menu_disassembly_manifest.csv` |
| UI project strings | 189,277 | `reports/mprj_strings.json` |
| UI textures from GPB/IDP0 | 6,058 | `extracted/ui_txs/` |
| Real decoded character portraits | 19 | `site/public/avatars/extracted/` |
| Packed DDS textures | 9,097 / 42.40 GiB | `extracted/textures/` |
| JPEG XL containers | 4,074 / 1.05 GiB | `extracted/jxl/` |
| JPEG XL box index | 4,074, all with EXIF boxes | `reports/jxl_index.json` |
| IDP0 payloads | 14,983 / 4.99 GiB | `extracted/idp0/` |
| Video files | 15 / 5.25 GiB | `extracted/video/` |
| SNDZ sound containers | 860 / 0.62 GiB, section tags indexed | `extracted/audio/`, `reports/sndz_index.json` |
| MDL5 metadata records | 8,149 | `reports/mdl5_index.json` |
| MDL5 submodel records | 59,498 (7,279 files with valid model tables) | `reports/mdl5_index.json` |
| TXS5 descriptors | 19,279 | `reports/txs5_index*.json` |
| GPB4 DMSB bundle | 311 / 311, byte-identical | `reports/dms_rust/` |
| Real Cafe face textures | 19 decoded, BC7 | `reports/face_atlases_rust/` |
| Track center lines | 85 | `extracted/tracks/map4/` |
| Track preview images | 85 | `extracted/tracks/preview/` |
| RADA race-line chunks | 392 | `extracted/tracks/rada/` |
| Canonical cars | 591 | `site/public/data/cars.json` |
| Dealer descriptions | 634 / 581 cars | `site/public/data/car_texts.csv` |
| Car Collection descriptions | 580 / 580 cars | `site/public/data/vehicle_collection_texts.csv` |
| GT Cafe dialogue languages | 19 | `site/public/data/dialogue_translations/` |
| Cars with any vehicle description | 582 | `site/public/data/cars.json` |
| Current dealer offers | 65 (50 used, 15 legend) | `site/public/data/dealer_offers.csv` |
| Unique CarDetail entries | 444 | `site/public/data/cafe_dialogues.csv` |
| Cafe scene lines | 2,620 | `site/public/data/cafe_scene_dialogues.csv` |
| Legacy ambiguity review | 253 / 253 resolved | `site/public/data/unmatched_dialogues.csv` |

## Extracted code and UI research

The three named Adhoc bytecode files are `main.adc`, `boot_entry.adc`, and
`scriptsmenu.adc`. The latter is a 16.4 MiB script package containing 1,906
menu scripts. GTAdhocToolchain 1.4.0 disassembled the package into readable
`.diss` files covering Brand Central, GT Cafe, Garage, Legend Cars, races,
championships, settings and developer modules.

The code archive excludes the original bytecode and includes only the
disassembly plus a complete per-file SHA-256 manifest:

- `reports/gt7_menu_scripts_diss.zip`
- `reports/menu_disassembly_manifest.csv`

The core executable `eboot.bin` is an encrypted PS5 SELF and cannot be
decompiled without a console-readable/decrypted ELF. This is the only major
code-logic blocker; the menu/Adhoc layer is available.

## Explicit format limitations

| Format | Status | Reason |
| --- | --- | --- |
| PS5 TXS5 pixel data | Cafe face atlases decoded with GNM mode 9 + BC7 | Other TXS5 pixel formats still need per-format validation |
| MDL5 geometry | Header, model tables, texture/shader offsets and identifier strings indexed | Remaining vertex/index/container layout is undocumented |
| SNDZ | Containers extracted | Files are synthesis programs rather than directly playable PCM/WAV |
| JPEG XL previews | 4,074 containers extracted; pixel decode blocked | `pillow-jxl-plugin 1.3.8` and `imagecodecs 2026.10.10` reject the game's JXL codestream |
| GPB | IDP0/TXS assets extracted; full node graph incomplete | Remaining GPB property schema unknown |
| Core game logic | Blocked | `eboot.bin` is encrypted PS5 SELF |

These rows are limitations, not claims of full extraction. They remain listed
so the research tree does not misrepresent partial formats as complete.

## Verification

```text
gt7re verify:
PASS native Rust verifier:
cars=591 withDialogue=332 dialogues=444 conversationLines=2620
vehicleCollectionCars=580 dealerOffers=65 menuBooks=108 avatars=19

verify_vehicle_collection_ui.mjs (production):
PASS five tabs, split Brand Central / Car Collection, EN/ZH/bilingual,
Peugeot 205 + Xiaomi SU7 entries, empty-state labeling, 390px mobile layout,
fixed previous/next bar and 1-5 keyboard tabs

cargo test --release:
9 passed, 0 failed
```

The production Worker `gt7-cafe-edge` deployment
`f0c4cee8-e47d-4634-a34e-d7df6e94a088` serves
<https://gt7.gakaki.com>.

The complete project-owned Python pipeline (63 scripts and the local virtual
environment) has been removed. Rust now owns store discovery, RT2, GPB4,
research joins, site generation, menu books, official caches, avatars,
indexing, diagnostics, packaging, verification and deployment orchestration.
`gt7re update` performs the update path with Rayon-based parallel stages.

The native Rust `pipeline` command performs the baseline research path:
GPB4 extraction from the original PS5 `PRPYS`, RT2/DMSB joins, all three
vehicle-description categories, exact Cafe scene reconstruction, the 253-row
historical review, and 19 real Cafe/dealer face atlases. The site builder then
adds 580 unique-car Car Collection entries with PS5
`DescriptionCatch` / `DescriptionLong` ownership evidence and bilingual
cross-checks against the official car-list description modules.
