# SoloDroid — Project Brief

## Purpose
Play exactly one Android game — **Auto Drill: Galaxy**
(`com.lavalabs.autodrill`, LAVA LABS) — on Windows with the smallest
practical disk footprint, because Google Play Games on PC requires a
10 GB SSD minimum before any game is installed.

## Chosen approach (SEAN, 2026-10-24)
Ultra-minimal **AOSP** variant (~2.7 GB class), selected over the
Google APIs variant (~3.5 GB class).

SoloDroid is a launcher, not an emulator engine:
- Google's standalone Android Emulator binary (WHPX on Windows)
- Exactly one system image: `system-images;android-35;default;x86_64`
- Exactly three sdkmanager packages total
- One AVD, tuned minimal, booting straight into the one game

## Game facts (checked 2026-10-24)
- Package: `com.lavalabs.autodrill`
- Play listing: https://play.google.com/store/apps/details?id=com.lavalabs.autodrill&hl=en-US
- Size: ~77.6 MB; latest version seen: 24 (2026-10-20)
- Category: Strategy; free, contains ads
- Declared capabilities include: network access, foreground service,
  push notifications, in-app purchases, ad identifiers, and licence
  verification (AppBrain safety/technology summary)
- Minimum Android version / ABI split: **not published** on the Play
  listing or AppBrain — determined empirically in the field test
  (an APK inspection + install attempt answers both)

## Risks, labelled
1. **Licence / billing on AOSP — Unverified, real risk.**
   Pure AOSP has no Play Licensing or Play Billing. If the game
   enforces licence verification at launch, the AOSP build will not
   start the game and the `google_apis` escape hatch (already in
   `config.json`) becomes the shipping configuration.
2. **ARM-only APK — Unverified.** If the APK ships only arm64 native
   libraries, the x86_64 image may refuse or fail to run it.
3. **Ads on AOSP — Unverified.** Ad SDKs often bundle their own
   fallback when Play Services is absent, but rewarded-ad flows are
   the most likely feature to degrade.

## Non-goals
- No multi-game library, no store, no account system
- No Play Store image, ever (that defeats the purpose)
- No bundled game APK (licensing; the user supplies their own)
- No macOS / Linux host support in v0.1

## Defaults applied
Rust, MIT, Windows 10 + 11, zero third-party dependencies,
black/red accent in CLI output, Progress % in setup/play,
everything under `%LOCALAPPDATA%\SoloDroid`.
