# GT7 Research Browser

Vue 3 + React/Base UI + Tailwind + Vite static research browser for Gran
Turismo 7 PS5 data recovered from the local game installation. The public deployment is
<https://gt7.gakaki.com>.

## Features

- 591 canonical cars, 79 brands and 582 matched official thumbnails.
- 444 unique `CarDetail` records, all resolved to cars through the game's
  `GTCafeMessages`, `ConversationGTCafeMsg`, DMSB car keys and RT2 labels.
- 2,620 car-scene lines, preserving the recovered message-chain order.
- 634 brand/used/collection descriptions mapped to 581 cars, with EN/ZH text.
- 580 Car Collection cars paired to PS5 `DescriptionCatch` /
  `DescriptionLong` IDs, cross-checked against the official bilingual car-list
  modules for clean EN/ZH short and full descriptions.
- Current dealer rotation data for 50 used cars and 15 legend cars, including
  price, discount, rarity and hot/pick flags.
- 19 distinct real portraits decoded from the game's PS5 IDP0/TXS5 face
  atlases using the native Rust GNM mode 9 / BC7 pipeline.
- Five car tabs: cafe dialogue, Brand Central, Car Collection, Used Cars and Hagerty
  Collection.
- All 15 Cafe cast cards and the three vehicle-text merchants use real
  portraits decoded from the PS5 face atlases; no SVG or shared placeholder
  avatar is used.
- Search across car names, cafe dialogue and all three description sources.
- A 253-record legacy ambiguity review, retaining the old candidate and the
  exact RT2-resolved car for manual checking.
- Chinese, English and bilingual display modes.
- Keyboard navigation, quick tab switching, responsive mobile layout and
  content-visibility rendering.
- A viewport-fixed mobile previous/next car bar that follows the complete
  filtered car order across brands and keeps the shared URL in sync.
- A Café Menu Book archive with all 108 menus, 231 mapped car entries and
  their manufacturers, rewards, collector levels and bilingual game text.
- Base UI dialogs/tabs/switch with Tailwind and shadcn-style tokens for the
  Menu Book page, including autoplay and fixed mobile previous/next controls.

## URLs

- Home: <https://gt7.gakaki.com>
- Xiaomi SU7 Ultra, Brand Central:
  <https://gt7.gakaki.com/?car=0x0000D1BB&tab=brandCentral&lang=both>
- Xiaomi SU7 Ultra, Car Collection:
  <https://gt7.gakaki.com/?car=0x0000D1BB&tab=vehicleCollection&lang=both>
- Legacy 253 review:
  <https://gt7.gakaki.com/?review=unmatched&lang=both>
- Full Cafe cast portraits:
  <https://gt7.gakaki.com/?cast=all&lang=both>
- Cafe Menu Book archive:
  <https://gt7.gakaki.com/menus_cafe?lang=both>
- Le Mans Winners Extra Menu No. 57:
  <https://gt7.gakaki.com/menus_cafe?book=extra-057&lang=both>
- Data downloads: `public/data/`

## Local build

```powershell
python ..\scripts\build_site_data.py
python ..\scripts\verify_site_data.py
python ..\scripts\fetch_menu_covers.py
python ..\scripts\build_menu_books.py
python ..\scripts\sanitize_site_text.py
python ..\scripts\verify_menu_books.py
npm install
npm run build
npm run preview
```

The scripts require the recovered PS5/PS4 extraction tree under
`C:\gt7\gt7\_re\extracted`. Raw game containers and decryption keys are not
part of source control.

## Deploy

```powershell
cd C:\gt7\gt7\_re\site
npx wrangler deploy -c wrangler.production.toml
```

The production Worker is `gt7-cafe-edge` and serves the custom domain
`gt7.gakaki.com`.

## Data files

- `public/data/cars.json`: optimized site payload.
- `public/data/car_texts.csv`: 634 dealer description rows with category,
  message IDs and EN/ZH text.
- `public/data/vehicle_collection_texts.csv`: 580 unique-car Car Collection
  records with paired Catch/Long IDs and EN/ZH text.
- `public/data/car_text_orphans.csv`: 9 texts without a usable current DMSB
  car row.
- `public/data/cafe_dialogues.csv`: 444 unique CarDetail records.
- `public/data/cafe_scene_dialogues.csv`: 2,620 scene lines.
- `public/data/unmatched_dialogues.csv`: the legacy 253-record review.
- `public/data/unmatched_dialogues.md`: the same review grouped by resolved car.
- `public/data/menu_books.json`: 108 main/bonus/Extra menus with local source
  IDs, car/manufacturer mappings, EN/ZH text and cover-container paths.
- `public/data/menu_books.csv`: the same dataset in a spreadsheet-friendly
  format.

This is a non-commercial reverse-engineering research project. Game names,
texts and vehicle imagery remain the property of their respective owners.
