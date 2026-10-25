# GT7 Offline Archive — Compose Multiplatform

This client runs the GT7 vehicle/cafe archive without a server or network connection.
The same Jetpack Compose UI is shared by Android and desktop JVM targets.

## Offline guarantee

- The Android manifest does not request `android.permission.INTERNET`.
- All JSON, thumbnails, avatars, menu covers, 4K AVIF images, and official JXL files are read from the package.
- There are no `http://` or `https://` runtime fetches in the Compose client.
- Android reads from APK `assets/`; desktop reads from the packaged `app/resources/` directory.
- `site/public` is the single canonical source. Gradle embeds it directly and does not duplicate it in the repository.

## Targets

- Android: Jetpack Compose, min SDK 26, target SDK 35.
- Windows: Compose Desktop, portable app image and ZIP.
- macOS: Compose Desktop, native app and DMG configuration.

## Build prerequisites

- JDK 21
- Android SDK with platform 35 and build-tools 35.0.0
- The existing `site/public` resource tree
- Windows MSI/EXE packaging additionally requires WiX 3; set `WIX_PATH` when it is installed outside `PATH`

## Android

```powershell
$env:JAVA_HOME = '<jdk-21>'
$env:ANDROID_HOME = '<android-sdk>'
.\gradlew.bat :composeApp:assembleDebug
```

The APK is written to:

```text
composeApp/build/outputs/apk/debug/composeApp-debug.apk
```

The complete archive currently produces a compressed APK of about 1.57 GiB.
It is intended for sideloading/local research, not Google Play distribution.

## Windows

```powershell
.\gradlew.bat :composeApp:createDistributable
```

The runnable application is written to:

```text
composeApp/build/compose/binaries/main/app/GT7OfflineArchive/
```

Create a portable ZIP:

```powershell
.\gradlew.bat :composeApp:packageWindowsPortableZip
```

Windows Installer has a practical package-size limit. With the full 2.1 GiB resource
tree, WiX can fail while linking the MSI even though the packaged application itself
is valid. Use the portable application/ZIP for the complete offline build.

## macOS

Run the following on macOS, with the same `site/public` tree present:

```bash
chmod +x gradlew scripts/build-macos.sh
./gradlew :composeApp:createDistributable
./gradlew :composeApp:packageDmg
```

The desktop runtime is built on the host platform. A macOS DMG therefore cannot be
produced on Windows, but the shared Compose code and `Dmg` target are already configured.

## Verification

```powershell
.\gradlew.bat :composeApp:verifyOfflineSources
```

The source check validates a representative set of all resource classes. A built APK
can be audited with:

```powershell
$env:ANDROID_HOME = '<android-sdk>'
& "$env:ANDROID_HOME\build-tools\35.0.0\aapt2.exe" dump permissions `
  composeApp\build\outputs\apk\debug\composeApp-debug.apk
```

Expected: no `android.permission.INTERNET` entry.

## Content ownership

Game data, images, names, and dialogue belong to Sony Interactive Entertainment,
Polyphony Digital, and their respective rights holders. This repository is an
offline research archive and is not affiliated with Sony or Polyphony Digital.
