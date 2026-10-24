# Changelog

All notable changes to SoloDroid are documented here.
Format follows Keep a Changelog; versions follow Semantic Versioning.

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
