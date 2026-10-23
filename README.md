# Gran Turismo 7 PS5 Reverse-Engineering Research

This workspace contains game-specific extraction and research tooling for a
locally owned Gran Turismo 7 PS5 installation. Raw game containers, keys,
models, videos and bulk extracted assets are intentionally excluded from the
GitHub repository.

The public research browser is deployed at <https://gt7.gakaki.com>.
The requirement-by-requirement extraction status and remaining format
limitations are recorded in `report/FINAL_STATUS.md`.

## Current results

| Area | Result |
| --- | ---: |
| Canonical cars | 591 |
| Brands | 79 |
| Matched official thumbnails | 582 |
| Indexed MDL5 submodels | 59,498 |
| Unique CarDetail records | 444 |
| Car-scene dialogue lines | 2,620 |
| Dealer/garage descriptions | 634 |
| Cars with at least one description | 581 |
| Legacy 253 ambiguity review | 253 / 253 resolved |
| Strongly-typed subtitle speakers | 14 |
| Real decoded character portraits | 19 |
| Cafe Menu Books | 105 (39 main + 12 bonus + 54 Extra) |
| Menu-Book cars mapped to names/makes | 222 / 222 |
| Menu-Book cover containers resolved | 74 |

The vehicle descriptions cover three independent game sources:

- Brand Central vehicle descriptions.
- Used Car Dealer descriptions.
- Hagerty Collection descriptions.

Each car page exposes four tabs: GT Cafe, Brand Central, Used Cars and Hagerty
Collection. Chinese, English and bilingual modes are available.

The Cafe Menu Book archive at <https://gt7.gakaki.com/menus_cafe> adds all
105 main, bonus and Extra menus. It includes collection names, cars,
manufacturers, rewards, collector levels, EN/ZH text, search, manufacturer
filters, keyboard navigation, autoplay and a viewport-fixed mobile
previous/next bar. The 74 collection-cover JXL containers are resolved from
the PS5 content store; the public page reconstructs their layouts from game
car imagery because the retail codestream is not accepted by public JXL
decoders.

## Main directories

- `scripts/`: Python extraction, DMSB parsing, RT2 text recovery, scene
  reconstruction, site-data generation and verification.
- `site/src/`: Vue 3 frontend.
- `site/public/data/`: optimized JSON/CSV/Markdown research data.
- `reports/`: extraction and mapping reports.
- `rust/gt7re/src/`: Rust content-store, RT2, GPB4, TXD/TXS, track and UDP
  parsing research.
- `report/`: format notes and reverse-engineering documentation.

The recovered menu bytecode is documented in
`reports/gt7_menu_scripts_diss.zip`: 1,906 readable disassembly files,
1,325,950 lines, with per-file SHA-256 hashes in
`reports/menu_disassembly_manifest.csv`.

## Rebuild

```powershell
cd C:\gt7\gt7\_re
& .\rust\gt7re\target\release\gt7re.exe pipeline
python scripts\build_site_data.py
python scripts\verify_site_data.py
python scripts\verify_avatars.py
python scripts\verify_rust_parity.py
python scripts\build_menu_books.py
cd site
npm install
npm run build
npx wrangler deploy -c wrangler.production.toml
```

## Data sources

- PS5 game data under `C:\gt7\gt7\contents`.
- Recovered DMSB database under `_re/extracted/dms_prpys`.
- Recovered RT2 localisation payloads under
  `_re/extracted/text/rtext/common`.
- Official GT7 car list for thumbnails and brand logos.

The 253-row review is retained in `reports/unmatched_dialogues.*` and at
<https://gt7.gakaki.com/?review=unmatched&lang=both>. Every row now has an
exact RT2-derived car assignment, while the historical candidate remains
visible for manual comparison.

This is a non-commercial preservation and reverse-engineering project. Game
assets and text remain the property of their respective rights holders.
