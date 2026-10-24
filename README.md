## TL;DR

**SoloDroid is the smallest practical way to play one Android game — Auto Drill: Galaxy — on Windows.** It's a tiny Rust launcher around Google's standalone Android Emulator with a single AOSP system image: no Android Studio, no Play Store, no multi-game library. Estimated total footprint **~2.6 GB** vs the 10 GB minimum for Google Play Games on PC. Run `solodroid setup` once, sideload the game APK you legally obtained, then `solodroid play` boots straight into the game.

---

# SoloDroid

A single-game, ultra-minimal Android launcher for Windows 10 / 11 (x64).

Built for exactly one game:

- **Auto Drill: Galaxy** — `com.lavalabs.autodrill` — [Google Play listing](https://play.google.com/store/apps/details?id=com.lavalabs.autodrill&hl=en-US)

SoloDroid is not a new emulator engine and doesn't pretend to be. All of the size saving comes from *not installing everything else*:

| What a normal setup installs | SoloDroid |
| --- | --- |
| Android Studio IDE (10–15 GB with caches) | ❌ Never |
| Play Store system image + Store updaters | ❌ AOSP `default` image |
| Multiple AVDs / device profiles | ❌ One AVD, one game |
| NDK, build-tools, SDK platforms, extras | ❌ None |
| Emulator + platform-tools + 1 system image | ✅ Only these 3 packages |

## Size budget (estimates, not measurements)

| Component | Estimate |
| --- | --- |
| AOSP system image (Android 35, x86_64) | ~1.55 GB |
| Android Emulator | ~550 MB |
| Command-line tools | ~180 MB |
| Platform-tools (adb) | ~60 MB |
| Game (~78 MB APK) + data + snapshot headroom | ~350 MB |
| **Total** | **~2.6–2.7 GB** |

`solodroid status` shows the *actual* on-disk size on your machine.

## Quick start

Prerequisites:

- Windows 10 (2004+) or Windows 11, x64, admin account
- Hardware virtualization (VT-x / AMD-V) enabled in BIOS/UEFI, and Windows Hypervisor Platform enabled
- A JDK 17+ on `PATH` (only needed during `setup`, because `sdkmanager` is a Java tool) — e.g. Microsoft Build of OpenJDK
- About 4 GB free disk space (install + Quick Boot snapshot headroom)

```powershell
solodroid setup                 # one-time, downloads the minimal SDK + creates the AVD
solodroid install C:\path\to\autodrill.apk
solodroid play                  # or just: solodroid
```

Get the APK legally — for example, back it up from your own phone where the game is installed. SoloDroid does not download games from third-party APK sites.

## Commands

| Command | What it does |
| --- | --- |
| `solodroid` / `solodroid play` | Boot (Quick Boot) and launch the game |
| `solodroid setup` | One-time minimal SDK install + AVD creation, with Progress % |
| `solodroid install <file.apk>` | Sideload an APK via adb |
| `solodroid status` | Install state + real disk usage |
| `solodroid doctor` | Java / virtualization / disk checks |
| `solodroid stop` | Stop the emulator |
| `solodroid reset` | Delete the AVD and start clean (asks first; game data is lost) |
| `solodroid config` | Print the active configuration |

Everything lives in **one folder**: `%LOCALAPPDATA%\SoloDroid`. Delete that folder and SoloDroid is completely uninstalled — nothing else is written anywhere.

## Configuration

`%LOCALAPPDATA%\SoloDroid\config.json` (see `examples/config.example.json`):

```json
{
  "api_level": 35,
  "image_variant": "aosp",
  "ram_mb": 1536,
  "cores": 2,
  "width": 720,
  "height": 1280,
  "dpi": 320,
  "data_partition_mb": 2048,
  "package": "com.lavalabs.autodrill"
}
```

The AVD profile is tuned for minimum footprint: host GPU, no cameras, no SD card, fast-boot snapshots on, sparse userdata that only occupies what the game actually writes.

## The honest catch: pure AOSP

The AOSP image has **no Google Play Store, no Play Services, no Play Licensing, and no Play Billing**. Auto Drill: Galaxy declares in-app purchases and licence verification, so:

- The core idle game *may* run fine sideloaded — **Unverified** until the field test in [docs/FIELD_TEST.md](docs/FIELD_TEST.md) is run on a real Windows machine.
- Purchases, licence checks, cloud saves, and push features may fail on AOSP even if the game itself runs.
- An ARM-only APK may not run on the x86_64 image (also covered by the field test).

**Escape hatch (already built in):** if the game hard-requires Play Services, edit `config.json`, set `"image_variant": "google_apis"`, delete the AVD (`solodroid reset`), and re-run `solodroid setup`. That swaps in the Google APIs image — Play Services included, still no Play Store app — for roughly +400 MB.

## Screenshots

Screenshots will be added from a real run on the target Windows machine once the field test is done. No placeholder or stock images are used in this project.

Capture list: setup progress, `status` output, the game running in the emulator window.

## Build from source

```powershell
cargo build --release
cargo test
```

Zero third-party dependencies — Rust standard library only.

## License

MIT — see [LICENSE](LICENSE).

Android, Google Play, and Auto Drill: Galaxy are trademarks of their respective owners. SoloDroid is an independent launcher for Google's Android Emulator and is not affiliated with Google or LAVA LABS.
