# SoloDroid — Field Test (run on SEAN's Windows machine)

All runtime behaviour is **Unverified** until this checklist is run on
a real Windows machine. The VM that built SoloDroid cannot run the
Android Emulator at speed and cannot render the result.

## Setup
- [ ] JDK 17+ installed and `java -version` works in PowerShell
- [ ] Virtualization shows **Enabled** in Task Manager → Performance → CPU
- [ ] `solodroid doctor` — note any warnings
- [ ] `solodroid setup` completes; record: wall time, any sdkmanager errors
- [ ] `solodroid status` — record the **actual** disk usage line
      (this replaces the ~2.7 GB estimate with a measured figure)

## The game
- [x] Game file obtained: `ADG.xapk` (base + `config.arm64_v8a` split),
      already inspected by Muse 2026-10-24 — arm64-only native code,
      minSdk 24, Play Billing + CHECK_LICENSE + Play Integrity declared
- [ ] `solodroid install <path-to-ADG.xapk>` — record the exact result:
      - [ ] Installed cleanly (base + split via install-multiple)
      - [ ] Failed: paste the adb error verbatim (an ABI/translation
            rejection here is decisive evidence)
- [ ] `solodroid play` — record:
      - [ ] Game reaches its main screen (screenshot)
      - [ ] Game blocks on a licence / Play Services error (screenshot — this triggers the `google_apis` escape hatch)
      - [ ] Game crashes / black screen (screenshot + note timing)
- [ ] Play 5 minutes: mining loop runs, joystick input works, sound plays
- [ ] Watch / attempt an ad flow if offered — note what happens
- [ ] Attempt opening the in-app purchase screen — note what happens
      (do **not** complete a purchase for the test)
- [ ] Close and `solodroid play` again — Quick Boot resume time: ___ s

## Screenshots owed for the README
- [ ] `setup` progress in the terminal
- [ ] `status` output
- [ ] The game running in the emulator window

## Verdict to report back
One line each: setup OK?, actual GB, game launches?, ads?, IAP screen?,
second-boot time. That decides whether v0.1 ships on AOSP or switches
to the `google_apis` image as the default.
