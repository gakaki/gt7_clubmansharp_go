# GT7 Research Browser

Vue 3 + Vite static research browser for Gran Turismo 7 PS5 data recovered
from the local game installation. The public deployment is
<https://gt7.gakaki.com>.

## Features

- 591 canonical cars, 79 brands and 582 matched official thumbnails.
- 444 unique `CarDetail` records, all resolved to cars through the game's
  `GTCafeMessages`, `ConversationGTCafeMsg`, DMSB car keys and RT2 labels.
- 2,620 car-scene lines, preserving the recovered message-chain order.
- 634 brand/used/collection descriptions mapped to 581 cars, with EN/ZH text.
- Four car tabs: cafe dialogue, Brand Central, Used Cars and Hagerty
  Collection.
- Search across car names, cafe dialogue and all three description sources.
- A 253-record legacy ambiguity review, retaining the old candidate and the
  exact RT2-resolved car for manual checking.
- Chinese, English and bilingual display modes.
- Keyboard navigation, quick tab switching, responsive mobile layout and
  content-visibility rendering.

## URLs

- Home: <https://gt7.gakaki.com>
- Xiaomi SU7 Ultra, Brand Central:
  <https://gt7.gakaki.com/?car=0x0000D1BB&tab=brandCentral&lang=both>
- Legacy 253 review:
  <https://gt7.gakaki.com/?review=unmatched&lang=both>
- Data downloads: `public/data/`

## Local build

```powershell
python ..\scripts\build_site_data.py
python ..\scripts\verify_site_data.py
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
- `public/data/car_text_orphans.csv`: 9 texts without a usable current DMSB
  car row.
- `public/data/cafe_dialogues.csv`: 444 unique CarDetail records.
- `public/data/cafe_scene_dialogues.csv`: 2,620 scene lines.
- `public/data/unmatched_dialogues.csv`: the legacy 253-record review.
- `public/data/unmatched_dialogues.md`: the same review grouped by resolved car.

This is a non-commercial reverse-engineering research project. Game names,
texts and vehicle imagery remain the property of their respective owners.
