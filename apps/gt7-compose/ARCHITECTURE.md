# Architecture

## Shared modules

`composeApp/src/commonMain` contains:

- Kotlin serialization models for vehicles, locale catalogs, dialogues, garage data,
  collection text, and menu books.
- `OfflineRepository`, which reads resources through an `expect/actual` abstraction.
- The responsive Compose UI, search/grouping/filtering, car detail tabs, language
  switching, cafe scenes, 4K gallery, and menu-book archive.

`androidMain` provides `AssetManager` access and Android bitmap decoding. `desktopMain`
provides filesystem access to the packaged app resources and Skia image decoding.

## Resource layout

Gradle embeds `site/public` directly:

- Android: `composeApp:mergeDebugAssets`
- Desktop: `prepareDesktopAppResources` → `appResourcesRootDir/common` shared subtree
- Installed desktop app: `app/resources/{data,cars,avatars,logos,menu_covers,common,official}`

No runtime resource cache is downloaded. The small in-memory image cache is bounded and
only avoids decoding the same visible image repeatedly.

## Responsive UI

- Width ≥ 960 dp: permanent manufacturer rail.
- Width < 960 dp: horizontally scrollable brand/filter strips.
- Car hero switches between side-by-side and stacked layouts.
- Gallery is a full-window black viewer with slideshow speed, pause, and translation data.

## Platform packaging

- Android APK: universal debug package containing all assets.
- Windows: `jpackage` application image plus reproducible ZIP.
- macOS: `jpackage` `.app` and DMG target; build on a macOS host.
