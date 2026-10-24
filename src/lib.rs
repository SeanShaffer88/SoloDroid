//! SoloDroid core logic.
//!
//! SoloDroid is *not* a new emulator engine. It is a tiny, single-game
//! launcher around Google's standalone Android Emulator with exactly one
//! AOSP system image — no Android Studio, no Play Store image, no
//! multi-instance manager, no store UI.
//!
//! Everything in this module is pure / testable: paths, config, the
//! sdkmanager package list, the AVD `config.ini`, and the emulator / adb
//! argument vectors. `main.rs` is the thin shell that runs them.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The one game this build is for.
pub const GAME_NAME: &str = "Auto Drill: Galaxy";
/// Android package id of the game.
pub const GAME_PACKAGE: &str = "com.lavalabs.autodrill";
/// Google Play listing for the game (for reference / manual install help).
pub const GAME_PLAY_URL: &str =
    "https://play.google.com/store/apps/details?id=com.lavalabs.autodrill&hl=en-US";
/// AVD name (no spaces — the emulator is picky).
pub const AVD_NAME: &str = "SoloDroid";
/// Official Android command-line tools for Windows (pinned build).
/// Source: https://developer.android.com/studio#command-line-tools-only
pub const CMDLINE_TOOLS_URL: &str =
    "https://dl.google.com/android/repository/commandlinetools-win-11076708_latest.zip";
pub const CMDLINE_TOOLS_ZIP: &str = "commandlinetools-win.zip";

/// Which system-image flavour to install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageVariant {
    /// Pure AOSP (`default` tag): no Play Store, no Play Services.
    /// This is the ultra-minimal variant — the default.
    Aosp,
    /// Google APIs (`google_apis` tag): Play Services, still no Play
    /// Store app. ~300–500 MB larger. Escape hatch if a game hard-requires
    /// Play Services / licensing / billing.
    GoogleApis,
}

impl ImageVariant {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "aosp" | "default" => Some(Self::Aosp),
            "google_apis" | "googleapis" | "google-apis" => Some(Self::GoogleApis),
            _ => None,
        }
    }
    pub fn as_config_str(&self) -> &'static str {
        match self {
            Self::Aosp => "aosp",
            Self::GoogleApis => "google_apis",
        }
    }
    pub fn sdkmanager_tag(&self) -> &'static str {
        match self {
            Self::Aosp => "default",
            Self::GoogleApis => "google_apis",
        }
    }
}

/// User configuration, stored as JSON in `%LOCALAPPDATA%\SoloDroid\config.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Android API level for the system image. 35 is the default: new
    /// Play apps in 2026 target it, and it stays well above the minimum
    /// SDK of typical current Unity games.
    pub api_level: u32,
    pub variant: ImageVariant,
    /// Guest RAM in MiB. 1536 is the floor for a smooth Unity idle game;
    /// 2048 is safer on machines with RAM to spare.
    pub ram_mb: u32,
    /// Guest CPU cores.
    pub cores: u32,
    /// Emulated display.
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    /// Initial userdata partition size (sparse — it only occupies what
    /// the guest actually writes).
    pub data_partition_mb: u32,
    /// Game package to boot straight into.
    pub package: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            api_level: 35,
            variant: ImageVariant::Aosp,
            ram_mb: 1536,
            cores: 2,
            width: 720,
            height: 1280,
            dpi: 320,
            data_partition_mb: 2048,
            package: GAME_PACKAGE.to_string(),
        }
    }
}

impl Config {
    /// Minimal hand-rolled JSON reader for our own flat config file.
    /// (Zero dependencies is a feature: it keeps the binary tiny.)
    pub fn from_json(text: &str) -> Self {
        let mut cfg = Config::default();
        if let Some(v) = json_u32(text, "api_level") {
            if (24..=36).contains(&v) {
                cfg.api_level = v;
            }
        }
        if let Some(v) = json_string(text, "image_variant") {
            if let Some(variant) = ImageVariant::parse(&v) {
                cfg.variant = variant;
            }
        }
        if let Some(v) = json_u32(text, "ram_mb") {
            if (512..=8192).contains(&v) {
                cfg.ram_mb = v;
            }
        }
        if let Some(v) = json_u32(text, "cores") {
            if (1..=8).contains(&v) {
                cfg.cores = v;
            }
        }
        if let Some(v) = json_u32(text, "width") {
            if v >= 320 {
                cfg.width = v;
            }
        }
        if let Some(v) = json_u32(text, "height") {
            if v >= 320 {
                cfg.height = v;
            }
        }
        if let Some(v) = json_u32(text, "dpi") {
            if (120..=640).contains(&v) {
                cfg.dpi = v;
            }
        }
        if let Some(v) = json_u32(text, "data_partition_mb") {
            if (512..=16384).contains(&v) {
                cfg.data_partition_mb = v;
            }
        }
        if let Some(v) = json_string(text, "package") {
            if is_valid_package(&v) {
                cfg.package = v;
            }
        }
        cfg
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\n  \"api_level\": {},\n  \"image_variant\": \"{}\",\n  \"ram_mb\": {},\n  \"cores\": {},\n  \"width\": {},\n  \"height\": {},\n  \"dpi\": {},\n  \"data_partition_mb\": {},\n  \"package\": \"{}\"\n}}\n",
            self.api_level,
            self.variant.as_config_str(),
            self.ram_mb,
            self.cores,
            self.width,
            self.height,
            self.dpi,
            self.data_partition_mb,
            self.package
        )
    }

    /// The exact sdkmanager packages SoloDroid needs — and nothing else.
    /// No NDK, no build-tools, no platform SDK, no extras.
    pub fn sdk_packages(&self) -> Vec<String> {
        vec![
            "platform-tools".to_string(),
            "emulator".to_string(),
            format!(
                "system-images;android-{};{};x86_64",
                self.api_level,
                self.variant.sdkmanager_tag()
            ),
        ]
    }

    /// AVD hardware config (`config.ini`). Every value here is chosen to
    /// minimise footprint while staying playable for a 2D/3D idle game:
    /// no camera, no sensors beyond the essentials, no SD card, GPU on
    /// host, snapshot (Quick Boot) on.
    pub fn avd_config_ini(&self) -> String {
        let mut m: BTreeMap<&str, String> = BTreeMap::new();
        m.insert("avd.name", AVD_NAME.to_string());
        m.insert("abi.type", "x86_64".to_string());
        m.insert("target", format!("android-{}", self.api_level));
        m.insert(
            "image.sysdir.1",
            format!(
                "system-images\\android-{}\\{}\\x86_64\\",
                self.api_level,
                self.variant.sdkmanager_tag()
            ),
        );
        m.insert("hw.cpu.arch", "x86_64".to_string());
        m.insert("hw.cpu.ncore", self.cores.to_string());
        m.insert("hw.ramSize", self.ram_mb.to_string());
        m.insert("hw.lcd.width", self.width.to_string());
        m.insert("hw.lcd.height", self.height.to_string());
        m.insert("hw.lcd.density", self.dpi.to_string());
        m.insert("hw.lcd.depth", "32".to_string());
        m.insert("hw.gpu.enabled", "yes".to_string());
        m.insert("hw.gpu.mode", "auto".to_string());
        m.insert("hw.mainKeys", "no".to_string());
        m.insert("hw.keyboard", "yes".to_string());
        m.insert("hw.camera.back", "none".to_string());
        m.insert("hw.camera.front", "none".to_string());
        m.insert("hw.sdCard", "no".to_string());
        m.insert(
            "disk.dataPartition.size",
            format!("{}M", self.data_partition_mb),
        );
        m.insert("fastboot.forceFastBoot", "yes".to_string());
        m.insert("runtime.network.speed", "full".to_string());
        m.insert("runtime.network.latency", "none".to_string());
        m.insert("showDeviceFrame", "no".to_string());
        let mut out = String::new();
        for (k, v) in m {
            out.push_str(&format!("{k}={v}\n"));
        }
        out
    }

    /// Emulator command-line arguments for a play session.
    /// `-no-audio` is deliberately *not* used (the game has sound);
    /// `-no-boot-anim` and the snapshot flags are where the speed is.
    pub fn emulator_args(&self) -> Vec<String> {
        vec![
            "-avd".into(),
            AVD_NAME.into(),
            "-gpu".into(),
            "auto".into(),
            "-accel".into(),
            "auto".into(),
            "-no-boot-anim".into(),
            "-no-metrics".into(),
            "-memory".into(),
            self.ram_mb.to_string(),
            "-cores".into(),
            self.cores.to_string(),
        ]
    }
}

/// All on-disk locations, rooted at the data directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// `%LOCALAPPDATA%\SoloDroid`, falling back to `./SoloDroid-data`.
    pub fn from_env() -> Self {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        Self::new(base.join("SoloDroid"))
    }

    pub fn config_file(&self) -> PathBuf {
        self.root.join("config.json")
    }
    pub fn sdk_dir(&self) -> PathBuf {
        self.root.join("sdk")
    }
    pub fn avd_dir(&self) -> PathBuf {
        self.root.join("avd")
    }
    pub fn avd_home(&self) -> PathBuf {
        // ANDROID_AVD_HOME points at the folder *containing* <name>.avd
        self.avd_dir()
    }
    pub fn avd_folder(&self) -> PathBuf {
        self.avd_dir().join(format!("{AVD_NAME}.avd"))
    }
    pub fn cmdline_tools_dir(&self) -> PathBuf {
        self.sdk_dir().join("cmdline-tools").join("latest")
    }
    pub fn sdkmanager(&self) -> PathBuf {
        self.cmdline_tools_dir().join("bin").join("sdkmanager.bat")
    }
    pub fn avdmanager(&self) -> PathBuf {
        self.cmdline_tools_dir().join("bin").join("avdmanager.bat")
    }
    pub fn emulator(&self) -> PathBuf {
        self.sdk_dir().join("emulator").join("emulator.exe")
    }
    pub fn adb(&self) -> PathBuf {
        self.sdk_dir().join("platform-tools").join("adb.exe")
    }
    pub fn system_image_dir(&self, cfg: &Config) -> PathBuf {
        self.sdk_dir()
            .join("system-images")
            .join(format!("android-{}", cfg.api_level))
            .join(cfg.variant.sdkmanager_tag())
            .join("x86_64")
    }
}

/// adb argument vector to launch the game's main activity via monkey —
/// the reliable, manifest-agnostic way to start an app whose launcher
/// activity name we don't know.
pub fn adb_launch_args(package: &str) -> Vec<String> {
    vec![
        "shell".into(),
        "monkey".into(),
        "-p".into(),
        package.into(),
        "-c".into(),
        "android.intent.category.LAUNCHER".into(),
        "1".into(),
    ]
}

pub fn adb_wait_boot_args() -> Vec<String> {
    vec![
        "shell".into(),
        "getprop".into(),
        "sys.boot_completed".into(),
    ]
}

/// Is this path an XAPK (a zip bundle of a base APK + split APKs)?
/// Auto Drill: Galaxy ships this way — the base APK alone will not
/// install correctly without its `config.arm64_v8a` split.
pub fn is_xapk_path(path: &Path) -> bool {
    path.extension()
        .map(|e| e.eq_ignore_ascii_case("xapk"))
        .unwrap_or(false)
}

/// Order the APKs found inside an XAPK for `adb install-multiple`:
/// the base APK first (`<package>.apk` or `base.apk`), then the
/// `config.*` splits alphabetically, then anything else alphabetically.
pub fn order_split_apks(names: &[String], package: &str) -> Vec<String> {
    let base_names = [format!("{package}.apk"), "base.apk".to_string()];
    let mut base: Vec<String> = Vec::new();
    let mut configs: Vec<String> = Vec::new();
    let mut others: Vec<String> = Vec::new();
    for n in names {
        let file = Path::new(n)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| n.clone());
        if base_names.iter().any(|b| b == &file) {
            base.push(n.clone());
        } else if file.starts_with("config.") {
            configs.push(n.clone());
        } else {
            others.push(n.clone());
        }
    }
    configs.sort();
    others.sort();
    base.append(&mut configs);
    base.append(&mut others);
    base
}

pub fn adb_install_args(apk: &str) -> Vec<String> {
    vec!["install".into(), "-r".into(), apk.into()]
}

/// Title of the launch-time file picker dialog.
pub const PICKER_TITLE: &str = "Pick the Auto Drill: Galaxy game file";

/// The picker's file-type filter as a UTF-16, double-NUL-terminated
/// string in the exact layout `GetOpenFileNameW` expects:
/// (label, pattern) pairs, each NUL-terminated, plus a final NUL.
pub fn picker_filter_wide() -> Vec<u16> {
    let parts = [
        "Game packages (*.xapk;*.apk)",
        "*.xapk;*.apk",
        "XAPK bundles (*.xapk)",
        "*.xapk",
        "APK files (*.apk)",
        "*.apk",
        "All files (*.*)",
        "*.*",
    ];
    let mut v: Vec<u16> = Vec::new();
    for part in parts {
        v.extend(part.encode_utf16());
        v.push(0);
    }
    v.push(0);
    v
}

pub fn adb_install_multiple_args(apks: &[String]) -> Vec<String> {
    let mut v: Vec<String> = vec!["install-multiple".into(), "-r".into()];
    v.extend(apks.iter().cloned());
    v
}

pub fn is_valid_package(s: &str) -> bool {
    !s.is_empty()
        && s.contains('.')
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
        && !s.starts_with('.')
        && !s.ends_with('.')
}

/// Rough on-disk budget, in MiB, for the AOSP variant — used for the
/// `status` / `doctor` disk-space checks and the README size claim.
/// These are planning estimates, not measurements.
pub fn estimated_footprint_mb(cfg: &Config) -> u32 {
    let image = match cfg.variant {
        ImageVariant::Aosp => 1550,
        ImageVariant::GoogleApis => 1950,
    };
    let emulator = 550;
    let platform_tools = 60;
    let tools = 180;
    // Game APK is ~78 MB installed, plus its data; userdata is sparse.
    let game_and_data = 350;
    image + emulator + platform_tools + tools + game_and_data
}

pub fn format_mb(mb: u32) -> String {
    if mb >= 1024 {
        format!("{:.1} GB", mb as f64 / 1024.0)
    } else {
        format!("{mb} MB")
    }
}

fn json_u32(text: &str, key: &str) -> Option<u32> {
    let needle = format!("\"{key}\"");
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start().strip_prefix(':')?.trim_start();
    let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    num.parse().ok()
}

fn json_string(text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    let start = text.find(&needle)? + needle.len();
    let rest = text[start..].trim_start().strip_prefix(':')?.trim_start();
    let quoted = rest.strip_prefix('"')?;
    let end = quoted.find('"')?;
    Some(quoted[..end].to_string())
}

pub fn avd_exists(paths: &Paths) -> bool {
    paths.avd_folder().join("config.ini").exists()
}

pub fn setup_complete(paths: &Paths, cfg: &Config) -> bool {
    paths.emulator().exists()
        && paths.adb().exists()
        && paths.system_image_dir(cfg).exists()
        && avd_exists(paths)
}

pub fn dir_size_mb(path: &Path) -> u32 {
    fn walk(p: &Path, total: &mut u64) {
        if let Ok(meta) = std::fs::metadata(p) {
            if meta.is_file() {
                *total += meta.len();
                return;
            }
        }
        if let Ok(entries) = std::fs::read_dir(p) {
            for e in entries.flatten() {
                walk(&e.path(), total);
            }
        }
    }
    let mut total: u64 = 0;
    walk(path, &mut total);
    (total / (1024 * 1024)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_ultra_minimal_aosp_profile() {
        let c = Config::default();
        assert_eq!(c.variant, ImageVariant::Aosp);
        assert_eq!(c.api_level, 35);
        assert_eq!(c.ram_mb, 1536);
        assert_eq!(c.package, GAME_PACKAGE);
    }

    #[test]
    fn sdk_packages_are_exactly_three_and_aosp_by_default() {
        let pkgs = Config::default().sdk_packages();
        assert_eq!(pkgs.len(), 3);
        assert!(pkgs.contains(&"platform-tools".to_string()));
        assert!(pkgs.contains(&"emulator".to_string()));
        assert!(pkgs.contains(&"system-images;android-35;default;x86_64".to_string()));
    }

    #[test]
    fn google_apis_variant_swaps_only_the_image() {
        let c = Config {
            variant: ImageVariant::GoogleApis,
            ..Config::default()
        };
        let pkgs = c.sdk_packages();
        assert!(pkgs.contains(&"system-images;android-35;google_apis;x86_64".to_string()));
        assert!(estimated_footprint_mb(&c) > estimated_footprint_mb(&Config::default()));
    }

    #[test]
    fn footprint_estimate_is_in_the_27gb_class() {
        let mb = estimated_footprint_mb(&Config::default());
        assert!((2400..=3100).contains(&mb), "estimate was {mb} MB");
    }

    #[test]
    fn avd_ini_has_no_sdcard_no_camera_and_fastboot() {
        let ini = Config::default().avd_config_ini();
        assert!(ini.contains("hw.sdCard=no"));
        assert!(ini.contains("hw.camera.back=none"));
        assert!(ini.contains("fastboot.forceFastBoot=yes"));
        assert!(ini.contains("hw.ramSize=1536"));
        assert!(ini.contains("abi.type=x86_64"));
    }

    #[test]
    fn emulator_args_boot_the_single_avd_without_boot_anim() {
        let args = Config::default().emulator_args();
        assert_eq!(args[0], "-avd");
        assert_eq!(args[1], AVD_NAME);
        assert!(args.contains(&"-no-boot-anim".to_string()));
        assert!(args.contains(&"-no-metrics".to_string()));
    }

    #[test]
    fn config_json_round_trips() {
        let c = Config {
            ram_mb: 2048,
            api_level: 34,
            ..Config::default()
        };
        let parsed = Config::from_json(&c.to_json());
        assert_eq!(parsed, c);
    }

    #[test]
    fn config_rejects_out_of_range_values() {
        let c = Config::from_json(r#"{"ram_mb": 99999, "api_level": 12, "package": "bad"}"#);
        assert_eq!(c.ram_mb, 1536);
        assert_eq!(c.api_level, 35);
        assert_eq!(c.package, GAME_PACKAGE);
    }

    #[test]
    fn variant_parse_accepts_aliases() {
        assert_eq!(ImageVariant::parse("aosp"), Some(ImageVariant::Aosp));
        assert_eq!(
            ImageVariant::parse("Google_APIs"),
            Some(ImageVariant::GoogleApis)
        );
        assert_eq!(ImageVariant::parse("playstore"), None);
    }

    #[test]
    fn package_validation() {
        assert!(is_valid_package("com.lavalabs.autodrill"));
        assert!(!is_valid_package("nodots"));
        assert!(!is_valid_package(".leading.dot"));
        assert!(!is_valid_package("has space.com"));
    }

    #[test]
    fn launch_args_target_the_game_package() {
        let args = adb_launch_args(GAME_PACKAGE);
        assert!(args.contains(&GAME_PACKAGE.to_string()));
        assert!(args.contains(&"monkey".to_string()));
    }

    #[test]
    fn xapk_detection_is_by_extension() {
        assert!(is_xapk_path(Path::new("ADG.xapk")));
        assert!(is_xapk_path(Path::new("game.XAPK")));
        assert!(!is_xapk_path(Path::new("game.apk")));
        assert!(!is_xapk_path(Path::new("xapk")));
    }

    #[test]
    fn split_ordering_puts_base_first_then_configs() {
        let names = vec![
            "config.arm64_v8a.apk".to_string(),
            "com.lavalabs.autodrill.apk".to_string(),
        ];
        let ordered = order_split_apks(&names, GAME_PACKAGE);
        assert_eq!(ordered[0], "com.lavalabs.autodrill.apk");
        assert_eq!(ordered[1], "config.arm64_v8a.apk");
    }

    #[test]
    fn install_multiple_args_include_every_split() {
        let apks = vec!["base.apk".to_string(), "config.arm64_v8a.apk".to_string()];
        let args = adb_install_multiple_args(&apks);
        assert_eq!(args[0], "install-multiple");
        assert!(args.contains(&"base.apk".to_string()));
        assert!(args.contains(&"config.arm64_v8a.apk".to_string()));
    }

    #[test]
    fn picker_filter_is_double_nul_terminated_and_offers_xapk() {
        let f = picker_filter_wide();
        assert_eq!(&f[f.len() - 2..], &[0, 0]);
        let text = String::from_utf16_lossy(&f);
        assert!(text.contains("*.xapk;*.apk"));
        assert!(text.contains("Game packages"));
        // No interior double-NUL before the end (that would truncate it).
        let interior = &f[..f.len() - 2];
        assert!(!interior.windows(2).any(|w| w == [0, 0]));
    }

    #[test]
    fn paths_stay_under_one_root() {
        let p = Paths::new(PathBuf::from(r"C:\Users\test\AppData\Local\SoloDroid"));
        assert!(p.emulator().starts_with(&p.root));
        assert!(p.adb().starts_with(&p.root));
        assert!(p.avd_folder().starts_with(&p.root));
        assert_eq!(p.avd_folder().file_name().unwrap(), "SoloDroid.avd");
    }

    #[test]
    fn format_mb_switches_to_gb() {
        assert_eq!(format_mb(2690), "2.6 GB");
        assert_eq!(format_mb(512), "512 MB");
    }
}
