//! SoloDroid CLI — the thin shell over the pure logic in `lib.rs`.
//!
//! Commands:
//!   solodroid            same as `play`
//!   solodroid setup      download the minimal SDK + create the AVD (once)
//!   solodroid play       boot (Quick Boot) and launch the game
//!   solodroid install <file.apk>   sideload an APK with adb
//!   solodroid status     install state + real on-disk size
//!   solodroid doctor     virtualization / disk / Java checks
//!   solodroid stop       stop the emulator
//!   solodroid reset      delete the AVD (game data is lost) — asks first

use solodroid::*;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map(String::as_str).unwrap_or("play");
    let code = match cmd {
        "play" => cmd_play(),
        "setup" => cmd_setup(),
        "install" => cmd_install(args.get(1)),
        "status" => cmd_status(),
        "doctor" => cmd_doctor(),
        "stop" => cmd_stop(),
        "reset" => cmd_reset(),
        "config" => cmd_config(),
        "-h" | "--help" | "help" => {
            print_help();
            0
        }
        "-V" | "--version" => {
            println!("solodroid {}", env!("CARGO_PKG_VERSION"));
            0
        }
        other => {
            eprintln!("Unknown command: {other}\n");
            print_help();
            2
        }
    };
    std::process::exit(code);
}

fn print_help() {
    println!(
        "SoloDroid {} — the smallest way to play one Android game on Windows\n\
         Built for: {GAME_NAME} ({GAME_PACKAGE})\n\n\
         USAGE:\n  solodroid [COMMAND]\n\n\
         COMMANDS:\n  play          Boot and launch the game (default)\n  \
         setup         One-time: download minimal SDK + create the AVD\n  \
         install <apk> Sideload an APK file into the emulator\n  \
         status        Show install state and real disk usage\n  \
         doctor        Check virtualization, disk space and Java\n  \
         stop          Stop the emulator\n  \
         reset         Delete the AVD and start clean (asks first)\n  \
         config        Print the active configuration\n  \
         help          This help\n\n\
         Everything lives in %LOCALAPPDATA%\\SoloDroid — delete that one\n\
         folder and SoloDroid is completely gone.",
        env!("CARGO_PKG_VERSION")
    );
}

fn load() -> (Paths, Config) {
    let paths = Paths::from_env();
    let cfg = match std::fs::read_to_string(paths.config_file()) {
        Ok(text) => Config::from_json(&text),
        Err(_) => Config::default(),
    };
    (paths, cfg)
}

fn banner() {
    println!("\x1b[31m■\x1b[0m SoloDroid — {GAME_NAME}");
}

fn cmd_config() -> i32 {
    let (paths, cfg) = load();
    banner();
    println!("Data folder: {}", paths.root.display());
    print!("{}", cfg.to_json());
    0
}

fn cmd_status() -> i32 {
    let (paths, cfg) = load();
    banner();
    println!("Data folder : {}", paths.root.display());
    println!(
        "SDK         : {}",
        if paths.emulator().exists() {
            "installed"
        } else {
            "not installed — run `solodroid setup`"
        }
    );
    println!(
        "System image: {} ({})",
        cfg.variant.as_config_str(),
        if paths.system_image_dir(&cfg).exists() {
            "installed"
        } else {
            "missing"
        }
    );
    println!(
        "AVD         : {}",
        if avd_exists(&paths) {
            "created"
        } else {
            "not created"
        }
    );
    if paths.root.exists() {
        println!(
            "Disk usage  : {} actual ({} estimated)",
            format_mb(dir_size_mb(&paths.root)),
            format_mb(estimated_footprint_mb(&cfg))
        );
    }
    0
}

fn cmd_doctor() -> i32 {
    let (paths, cfg) = load();
    banner();
    let mut ok = true;

    // Java: sdkmanager needs a JDK/JRE 17+ on PATH or JAVA_HOME.
    let java = Command::new("java").arg("-version").output();
    match java {
        Ok(_) => println!("[ok]   Java found (needed only during setup)"),
        Err(_) => {
            println!(
                "[warn] Java not found — setup needs a JDK 17+ (e.g. Microsoft Build of OpenJDK)"
            );
            ok = false;
        }
    }

    // Virtualization: WHPX is the fast path on Windows 10/11.
    println!("[info] Hardware virtualization (VT-x/AMD-V) must be ON in BIOS/UEFI,");
    println!("       and Windows Hypervisor Platform enabled, for full-speed emulation.");
    println!("       Check: Task Manager > Performance > CPU > Virtualization: Enabled");

    // Disk space against the estimate, with headroom for the game + snapshot.
    let need = estimated_footprint_mb(&cfg) + 1024;
    println!(
        "[info] Free space needed: about {} (install + snapshot headroom)",
        format_mb(need)
    );
    println!("[info] Data folder: {}", paths.root.display());
    if ok {
        0
    } else {
        1
    }
}

fn run(cmd: &Path, args: &[String], envs: &[(&str, PathBuf)]) -> Result<i32, String> {
    let mut c = Command::new(cmd);
    c.args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (k, v) in envs {
        c.env(k, v);
    }
    let status = c
        .status()
        .map_err(|e| format!("failed to run {}: {e}", cmd.display()))?;
    Ok(status.code().unwrap_or(-1))
}

fn run_capture(cmd: &Path, args: &[String]) -> Result<String, String> {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| format!("failed to run {}: {e}", cmd.display()))?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Run a command answering every `License ... [y/N]` / prompt with `y`.
/// sdkmanager asks to accept licenses; piping `y` is its documented
/// unattended flow alongside `sdkmanager --licenses`.
fn run_yes(cmd: &Path, args: &[String], envs: &[(&str, PathBuf)]) -> Result<i32, String> {
    let mut c = Command::new(cmd);
    c.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    for (k, v) in envs {
        c.env(k, v);
    }
    let mut child = c
        .spawn()
        .map_err(|e| format!("failed to run {}: {e}", cmd.display()))?;
    if let Some(mut stdin) = child.stdin.take() {
        // Generous stream of "y\n" — sdkmanager stops reading when done.
        let _ = stdin.write_all(b"y\n".repeat(200).as_slice());
    }
    let status = child
        .wait()
        .map_err(|e| format!("failed waiting for {}: {e}", cmd.display()))?;
    Ok(status.code().unwrap_or(-1))
}

fn cmd_setup() -> i32 {
    let (paths, cfg) = load();
    banner();
    let steps = 4;
    println!("Progress: 0% — setting up the minimal AOSP emulator for {GAME_NAME}");

    // Persist the config we are setting up for.
    if let Err(e) = std::fs::create_dir_all(&paths.root) {
        eprintln!("Cannot create {}: {e}", paths.root.display());
        return 1;
    }
    let _ = std::fs::write(paths.config_file(), cfg.to_json());

    // 1. Command-line tools (only if sdkmanager is not already here).
    if !paths.sdkmanager().exists() {
        println!("Progress: 10% — downloading Android command-line tools…");
        let zip = paths.root.join(CMDLINE_TOOLS_ZIP);
        let dl = Command::new("curl.exe")
            .args([
                "-L",
                "-o",
                zip.to_str().unwrap_or_default(),
                CMDLINE_TOOLS_URL,
            ])
            .status();
        match dl {
            Ok(s) if s.success() => {}
            _ => {
                eprintln!("Download failed. Get the tools manually from");
                eprintln!("https://developer.android.com/studio#command-line-tools-only");
                eprintln!("and unzip so that this file exists:");
                eprintln!("  {}", paths.sdkmanager().display());
                return 1;
            }
        }
        let expand = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Force -Path '{}' -DestinationPath '{}'",
                    zip.display(),
                    paths.sdk_dir().join("cmdline-tools-tmp").display()
                ),
            ])
            .status();
        if !matches!(expand, Ok(s) if s.success()) {
            eprintln!("Could not unzip the command-line tools.");
            return 1;
        }
        // Zip layout is cmdline-tools/… — normalise to cmdline-tools/latest/…
        let tmp = paths
            .sdk_dir()
            .join("cmdline-tools-tmp")
            .join("cmdline-tools");
        let latest = paths.cmdline_tools_dir();
        if let Some(parent) = latest.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::rename(&tmp, &latest).is_err() {
            eprintln!(
                "Could not place tools at {}. Move them there manually.",
                latest.display()
            );
            return 1;
        }
        let _ = std::fs::remove_file(&zip);
    }
    println!("Progress: 25% — command-line tools ready (1/{steps})");

    let sdk_env: Vec<(&str, PathBuf)> = vec![
        ("ANDROID_HOME", paths.sdk_dir()),
        ("ANDROID_SDK_ROOT", paths.sdk_dir()),
        ("ANDROID_AVD_HOME", paths.avd_home()),
    ];

    // 2. Licences.
    let _ = run_yes(&paths.sdkmanager(), &["--licenses".to_string()], &sdk_env);
    println!("Progress: 40% — licences accepted (2/{steps})");

    // 3. The three packages — and nothing else.
    let mut pkg_args: Vec<String> = cfg.sdk_packages();
    println!(
        "Progress: 50% — installing: {} (the big download, ~{})",
        pkg_args.join(", "),
        format_mb(estimated_footprint_mb(&cfg))
    );
    let mut full: Vec<String> = vec![];
    full.append(&mut pkg_args);
    match run_yes(&paths.sdkmanager(), &full, &sdk_env) {
        Ok(0) => {}
        Ok(code) => {
            eprintln!("sdkmanager exited with {code}. Re-run `solodroid setup` to retry.");
            return 1;
        }
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    }
    println!("Progress: 80% — SDK installed (3/{steps})");

    // 4. Create the AVD from our config.ini.
    if !avd_exists(&paths) {
        let avd_args = vec![
            "create".to_string(),
            "avd".to_string(),
            "-n".to_string(),
            AVD_NAME.to_string(),
            "-k".to_string(),
            format!(
                "system-images;android-{};{};x86_64",
                cfg.api_level,
                cfg.variant.sdkmanager_tag()
            ),
            "--force".to_string(),
        ];
        // avdmanager asks "create a custom hardware profile? [no]" — stdin
        // inherits the terminal here; the generated profile is replaced by
        // our tuned config.ini immediately after, either way.
        match run(&paths.avdmanager(), &avd_args, &sdk_env) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("{e}");
                return 1;
            }
        }
        let ini_path = paths.avd_folder().join("config.ini");
        if let Err(e) = std::fs::write(&ini_path, cfg.avd_config_ini()) {
            eprintln!("Could not write {}: {e}", ini_path.display());
            return 1;
        }
    }
    println!("Progress: 100% — setup complete (4/{steps})");
    println!();
    println!("Next:");
    println!("  1. Get the game APK legally (e.g. back it up from your own phone).");
    println!("  2. solodroid install <path-to-apk>");
    println!("  3. solodroid play");
    println!();
    println!("Note: this is a pure AOSP image — no Play Store, no Play Services.");
    println!("If the game refuses to run (licence / billing check), set");
    println!("\"image_variant\": \"google_apis\" in config.json and re-run setup.");
    0
}

fn ensure_setup(paths: &Paths, cfg: &Config) -> bool {
    if setup_complete(paths, cfg) {
        return true;
    }
    println!("First run — running setup…");
    cmd_setup() == 0 && setup_complete(paths, cfg)
}

fn cmd_play() -> i32 {
    let (paths, cfg) = load();
    banner();
    if !ensure_setup(&paths, &cfg) {
        eprintln!("Setup is not complete. Run `solodroid setup` and see what it reports.");
        return 1;
    }

    // Already running? Just relaunch the game.
    if let Ok(out) = run_capture(&paths.adb(), &["devices".to_string()]) {
        if out.contains("emulator-") {
            println!("Emulator already running — launching {GAME_NAME}…");
            let _ = run(&paths.adb(), &adb_launch_args(&cfg.package), &[]);
            return 0;
        }
    }

    println!("Progress: 10% — booting (Quick Boot snapshot after the first boot)…");
    let args = cfg.emulator_args();
    let mut child = match Command::new(paths.emulator())
        .args(&args)
        .env("ANDROID_HOME", paths.sdk_dir())
        .env("ANDROID_AVD_HOME", paths.avd_home())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Could not start the emulator: {e}");
            eprintln!("Run `solodroid doctor` for virtualization checks.");
            return 1;
        }
    };

    // Wait for adb + boot completion (up to ~3 minutes on a cold boot).
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut booted = false;
    while Instant::now() < deadline {
        if let Ok(out) = run_capture(&paths.adb(), &adb_wait_boot_args()) {
            if out.trim() == "1" {
                booted = true;
                break;
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
    if !booted {
        eprintln!("The emulator did not finish booting in 3 minutes.");
        eprintln!("First boots are the slowest; try `solodroid play` once more.");
        let _ = child.kill();
        return 1;
    }
    println!("Progress: 80% — booted. Launching {GAME_NAME}…");

    // Is the game installed?
    let pkgs = run_capture(
        &paths.adb(),
        &[
            "shell".into(),
            "pm".into(),
            "list".into(),
            "packages".into(),
            cfg.package.clone(),
        ],
    )
    .unwrap_or_default();
    if !pkgs.contains(&cfg.package) {
        println!("{GAME_NAME} is not installed yet.");
        println!("Install it with:  solodroid install <path-to-apk>");
        println!("Play listing:     {GAME_PLAY_URL}");
        return 1;
    }

    match run(&paths.adb(), &adb_launch_args(&cfg.package), &[]) {
        Ok(_) => {
            println!("Progress: 100% — game launched. Have fun, SEAN.");
            0
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn cmd_install(apk: Option<&String>) -> i32 {
    let (paths, cfg) = load();
    let Some(apk) = apk else {
        eprintln!("Usage: solodroid install <path-to-apk>");
        return 2;
    };
    let apk_path = PathBuf::from(apk);
    if !apk_path.exists() {
        eprintln!("No such file: {apk}");
        return 1;
    }
    if !setup_complete(&paths, &cfg) {
        eprintln!("Run `solodroid setup` first, and boot once with `solodroid play`.");
        return 1;
    }
    println!(
        "Installing {apk} … (the emulator must be running — `solodroid play` in another window)"
    );
    match run(
        &paths.adb(),
        &[
            "install".to_string(),
            "-r".to_string(),
            apk_path.display().to_string(),
        ],
        &[],
    ) {
        Ok(0) => {
            println!("Installed. Run `solodroid play` to launch {GAME_NAME}.");
            0
        }
        Ok(code) => {
            eprintln!("adb install exited with {code}.");
            eprintln!(
                "Common causes: emulator not running, or an ARM-only APK on the x86_64 image."
            );
            1
        }
        Err(e) => {
            eprintln!("{e}");
            1
        }
    }
}

fn cmd_stop() -> i32 {
    let (paths, _cfg) = load();
    if !paths.adb().exists() {
        println!("Nothing to stop — SDK not installed.");
        return 0;
    }
    let _ = run(&paths.adb(), &["emu".to_string(), "kill".to_string()], &[]);
    println!("Emulator stopped.");
    0
}

fn cmd_reset() -> i32 {
    let (paths, _cfg) = load();
    print!(
        "Delete the AVD at {} ? Game data will be lost. [y/N] ",
        paths.avd_folder().display()
    );
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    let stdin = std::io::stdin();
    if stdin.lock().read_line(&mut line).is_err() {
        return 1;
    }
    if line.trim().eq_ignore_ascii_case("y") {
        let _ = cmd_stop();
        match std::fs::remove_dir_all(paths.avd_folder()) {
            Ok(()) => {
                println!("AVD deleted. `solodroid setup` will recreate it.");
                0
            }
            Err(e) => {
                eprintln!("Could not delete: {e}");
                1
            }
        }
    } else {
        println!("Cancelled.");
        0
    }
}
