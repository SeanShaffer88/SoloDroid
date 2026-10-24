# Changelog

All notable changes to SoloDroid are documented here.
Format follows Keep a Changelog; versions follow Semantic Versioning.

## [0.1.2] - 2026-10-24

### Added
- **Launch-time file picker.** Running `solodroid` / `play` when
  the game is not yet installed now opens the native Windows file
  picker ("Pick the Auto Drill: Galaxy game file", filtered to
  `.xapk`/`.apk`) instead of stopping with a "run install"
  message — pick `ADG.xapk` and SoloDroid installs it (base +
  splits) and launches the game in the same run. `solodroid
  install` with no file argument opens the same picker. Zero new
  dependencies: the dialog is direct `GetOpenFileNameW` FFI with a
  compile-time-pinned struct layout.

## [0.1.1] - 2026-10-24

### Added
- **XAPK / split-APK install support.** The actual game file
  (inspected 2026-10-24 from the user's own backup, `ADG.xapk`)
  is a base APK plus a required `config.arm64_v8a` split — the
  base alone cannot install. `solodroid install <file.xapk>` now
  extracts the bundle and installs base + splits together with
  `adb install-multiple`.

### Confirmed game facts (from the supplied XAPK's manifest)
- Package `com.lavalabs.autodrill`, version 25, **minSdk 24**
  (Android 7.0), targetSdk 36.
- **Native code is arm64-v8a only** (25 libraries in the config
  split; the base APK has none). There is no x86_64 build, so the
  x86_64 emulator image must rely on its ARM translation layer.
- The manifest declares Play Billing (`com.android.vending.BILLING`),
  Play licence verification (`com.android.vending.CHECK_LICENSE`),
  the ad-id permission, and the APK bundles Firebase + Play
  Services client libraries and Play Integrity — so the pure-AOSP
  risk flagged in 0.1.0 is real and confirmed at the manifest
  level. Whether the game *enforces* any of it at runtime is still
  Unverified until the field test.

## [0.1.0] - 2026-10-24

### Added
- Ultra-minimal, single-game Android launcher for Windows, built for
  **Auto Drill: Galaxy** (`com.lavalabs.autodrill`).
- `setup` command: downloads only the Android command-line tools, then
  installs exactly three SDK packages — `platform-tools`, `emulator`,
  and one AOSP `system-images;android-35;default;x86_64` image — and
  creates one tuned AVD. No Android Studio, no Play Store image.
- `play` command: Quick Boot launch, waits for boot, launches the game
  straight away. Also the default when run with no arguments.
- `install <apk>`, `status`, `doctor`, `stop`, `reset`, and `config`
  commands.
- Zero-dependency Rust implementation; all state lives in one folder,
  `%LOCALAPPDATA%\SoloDroid` — delete it and SoloDroid is gone.
- Tuned AVD profile: 1536 MB RAM, 2 cores, 720×1280 @ 320 dpi, no
  camera, no SD card, host GPU, fast-boot snapshots on, sparse 2 GB
  userdata partition.
- Escape hatch: set `"image_variant": "google_apis"` in `config.json`
  to swap the AOSP image for the Google APIs image (Play Services, still
  no Play Store) if a game hard-requires it.

### Known limitations (honest labels)
- **Runtime is Unverified** until the field test in
  `docs/FIELD_TEST.md` is run on a real Windows machine.
- Pure AOSP has no Play Services, Play Licensing, or Play Billing.
  Auto Drill: Galaxy declares in-app purchases and licence
  verification, so those features may not work on the AOSP image even
  if the core game does.
- An ARM-only APK may not run on the x86_64 image; that is checked in
  the field test.
