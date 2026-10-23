# Gran Turismo 7 PS5 Reverse-Engineering Research

This workspace contains game-specific extraction and research tooling for a
locally owned Gran Turismo 7 PS5 installation. Raw game containers, keys,
models, videos and bulk extracted assets are intentionally excluded from the
GitHub repository.

The public research browser is deployed at <https://gt7.gakaki.com>.
The requirement-by-requirement extraction status and remaining format
limitations are recorded in `report/FINAL_STATUS.md`.
All project-owned Python sources and the local Python virtual environment have
been removed; the remaining pipeline is Rust and uses Rayon for independent
store, text, image, index and site tasks.

## Current results

| Area | Result |
| --- | ---: |
| Canonical cars | 591 |
| Brands | 79 |
| Matched official thumbnails | 582 |
| Indexed MDL5 submodels | 59,498 |
| Unique CarDetail records | 444 |
| Car-scene dialogue lines | 2,620 |
| Dealer descriptions | 634 |
| Vehicle Collection cars | 580 |
| Cars with at least one description | 582 |
| Legacy 253 ambiguity review | 253 / 253 resolved |
| Strongly-typed subtitle speakers | 14 |
| Real decoded character portraits | 19 |
| Cafe Menu Books | 108 (39 main + 12 bonus + 57 Extra) |
| Menu-Book cars mapped to names/makes | 231 / 231 |
| Menu-Book covers located (screenshot or container) | 77 |
| Browser-ready Menu-Book covers | 74 |

The vehicle descriptions cover four independent game sources:

- Brand Central vehicle descriptions.
- PS5 `DescriptionCatch` / `DescriptionLong` Car Collection text.
- Used Car Dealer descriptions.
- Hagerty Collection descriptions.

Each car page exposes five tabs: GT Cafe, Brand Central, Car Collection, Used Cars and Hagerty
Collection. Chinese, English and bilingual modes are available.
The main car index also filters by Brand Central, Used Cars, Car Collection
and GT Cafe dialogue.

The Cafe Menu Book archive at <https://gt7.gakaki.com/menus_cafe> adds all
108 main, bonus and Extra menus. It includes collection names, cars,
manufacturers, rewards, collector levels, EN/ZH text, search, manufacturer
filters, keyboard navigation, autoplay and a viewport-fixed mobile
previous/next bar. The original PS5 collection-cover containers are resolved
from the local content store, while 74 public menu screenshots provide the
visible WebP covers. The remaining collection covers use the game car imagery
as a layout reconstruction because the retail JXL codestream is not accepted
by public decoders.

## Main directories

- `rust/gt7re/`: the complete extraction, DMSB/RT2, site-data, menu-book,
  verification, packaging and deployment pipeline.
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
cd rust\gt7re
cargo build --release
.\target\release\gt7re.exe update

# Refresh official descriptions / covers and deploy the production Worker:
.\target\release\gt7re.exe update --refresh-official --proxy http://127.0.0.1:7897 --deploy
```

`update` scans the content store, rebuilds the DMSB/RT2 research report, joins
the site data, rebuilds menu books, refreshes all parallel indexes and runs the
fail-fast Rust verifier. Independent extraction stages use Rayon.

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
