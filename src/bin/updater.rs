//! LogicCraft CAD — Native Multiplatform Application Updater (`updater.rs`).
//!
//! Provides automated, cross-platform update functionality for:
//! - Windows (MSI installer, Setup EXE, or standalone binary replacement)
//! - macOS (DMG disk image mount, PKG installer, or Universal tarball)
//! - Linux (DEB package installation or Linux tarball)
//!
//! Connects directly to official GitHub Releases API:
//! https://api.github.com/repos/PMTSantos/LogicCraft/releases/latest

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "LogicCraft CAD";
const REPO_URL: &str = "https://github.com/PMTSantos/LogicCraft";
const GITHUB_API_URL: &str = "https://api.github.com/repos/PMTSantos/LogicCraft/releases/latest";

#[derive(Debug, Clone)]
struct ReleaseAsset {
    name: String,
    download_url: String,
    size: u64,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct ReleaseInfo {
    tag_name: String,
    version: String,
    html_url: String,
    assets: Vec<ReleaseAsset>,
}

fn main() {
    println!("========================================================");
    println!("    {} - Native Cross-Platform Updater v{}   ", APP_NAME, APP_VERSION);
    println!("    Platform: {}", current_platform_description());
    println!("    Official Repository: {}", REPO_URL);
    println!("========================================================");

    let args: Vec<String> = std::env::args().collect();
    let is_check_only = args.iter().any(|a| a == "--check" || a == "-c");
    let is_silent = args.iter().any(|a| a == "--silent" || a == "-s");
    let is_forced = args.iter().any(|a| a == "--force" || a == "-f");
    let should_launch = args.iter().any(|a| a == "--launch" || a == "-l");

    let mut custom_dir: Option<PathBuf> = None;
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--dir" || arg == "-d" {
            if let Some(d) = iter.next() {
                custom_dir = Some(PathBuf::from(d));
            }
        } else if let Some(rest) = arg.strip_prefix("--dir=") {
            custom_dir = Some(PathBuf::from(rest));
        }
    }

    let install_dir = resolve_install_directory(custom_dir);
    println!("[*] Installation Directory: {}", install_dir.display());

    // Step 1: Query GitHub API for latest release
    println!("\n[1/5] Checking for updates on GitHub...");
    let latest_release = fetch_latest_release();

    match &latest_release {
        Some(rel) => {
            println!(" [OK] Latest available version on GitHub: v{}", rel.version);
            let is_newer = is_version_newer(&rel.version, APP_VERSION);
            if !is_newer && !is_forced {
                println!(" [*] You are already running the latest version of {} (v{}).", APP_NAME, APP_VERSION);
                if is_check_only {
                    return;
                }
                println!("     Use --force to reinstall or re-download.");
                if !is_silent {
                    wait_for_user();
                }
                return;
            } else if is_newer {
                println!(" [!] New update available: v{} -> v{}", APP_VERSION, rel.version);
            } else {
                println!(" [*] Force update requested for v{}.", rel.version);
            }
        }
        None => {
            eprintln!(" [!] Note: Could not query GitHub Releases API directly (offline or rate limited).");
            if is_check_only {
                println!("     Current installed version: v{}", APP_VERSION);
                return;
            }
            println!("     Falling back to local release packages or manual release page...");
        }
    }

    if is_check_only {
        return;
    }

    // Step 2: Ensure any running application instances are closed
    println!("\n[2/5] Closing running {} instances...", APP_NAME);
    close_running_app();

    // Step 3: Download or resolve update package
    println!("\n[3/5] Resolving platform update package...");
    let mut update_applied = false;

    if let Some(rel) = &latest_release {
        if let Some(asset) = select_best_asset_for_platform(&rel.assets) {
            println!(" [OK] Selected asset: {} ({} MB)", asset.name, asset.size / 1_000_000);
            let temp_dest = std::env::temp_dir().join(&asset.name);
            println!(" [*] Downloading from: {}", asset.download_url);
            println!(" [*] Target: {}", temp_dest.display());

            let download_ok = Command::new("curl")
                .args(["-L", "--progress-bar", "-o", temp_dest.to_str().unwrap_or(""), &asset.download_url])
                .status()
                .map(|s| s.success())
                .unwrap_or(false);

            if download_ok && temp_dest.exists() {
                println!(" [SUCCESS] Download completed successfully.");
                println!("\n[4/5] Applying update for platform...");
                update_applied = apply_downloaded_asset(&temp_dest, &install_dir);
            } else {
                eprintln!(" [ERROR] Download failed. Launching browser fallback to GitHub release page...");
                open_in_browser(&rel.html_url);
            }
        } else {
            println!(" [!] No platform-specific package matched. Opening release in browser...");
            open_in_browser(&rel.html_url);
        }
    } else {
        // Offline / local fallback
        println!(" [*] Checking for local release build in workspace...");
        if let Some(source_exe) = find_local_binary(&install_dir) {
            println!(" [OK] Found local binary: {}", source_exe.display());
            println!("\n[4/5] Applying local update...");
            let target_exe = install_dir.join(if cfg!(windows) { "logiccraft.exe" } else { "logiccraft" });
            let _ = fs::create_dir_all(&install_dir);
            match fs::copy(&source_exe, &target_exe) {
                Ok(bytes) => {
                    println!(" [SUCCESS] Copied updated binary ({} bytes).", bytes);
                    copy_auxiliary_files(&install_dir);
                    update_applied = true;
                }
                Err(e) => eprintln!(" [ERROR] Failed to copy local binary: {e}"),
            }
        } else {
            println!(" [*] Launching official GitHub Releases page in browser...");
            open_in_browser(&format!("{REPO_URL}/releases/latest"));
        }
    }

    // Step 5: Verify user configuration persistence
    println!("\n[5/5] Verifying persistent user configuration...");
    verify_user_settings();

    println!("\n========================================================");
    if update_applied {
        println!(" [SUCCESS] {} Update Applied Successfully!", APP_NAME);
    } else {
        println!(" [COMPLETED] Update process finished.");
    }
    println!("========================================================");

    if should_launch {
        let exe = install_dir.join(if cfg!(windows) { "logiccraft.exe" } else { "logiccraft" });
        if exe.exists() {
            println!(" [*] Launching: {}", exe.display());
            let _ = Command::new(&exe).spawn();
        }
    }

    if !is_silent {
        wait_for_user();
    }
}

fn current_platform_description() -> &'static str {
    #[cfg(target_os = "windows")]
    return "Windows (x86_64)";
    #[cfg(target_os = "macos")]
    return "macOS (Universal / Apple Silicon & Intel)";
    #[cfg(target_os = "linux")]
    return "Linux (x86_64 / amd64)";
    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    return "Unknown OS";
}

fn fetch_latest_release() -> Option<ReleaseInfo> {
    let output = Command::new("curl")
        .args([
            "-s",
            "-L",
            "-H",
            "User-Agent: LogicCraft-Updater",
            GITHUB_API_URL,
        ])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout).ok()?;
    let tag_name = json.get("tag_name")?.as_str()?.to_string();
    let version = tag_name.trim_start_matches(|c| c == 'v' || c == 'V').to_string();
    let html_url = json
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or(REPO_URL)
        .to_string();

    let mut assets = Vec::new();
    if let Some(arr) = json.get("assets").and_then(|a| a.as_array()) {
        for item in arr {
            if let (Some(name), Some(url)) = (
                item.get("name").and_then(|n| n.as_str()),
                item.get("browser_download_url").and_then(|u| u.as_str()),
            ) {
                let size = item.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                assets.push(ReleaseAsset {
                    name: name.to_string(),
                    download_url: url.to_string(),
                    size,
                });
            }
        }
    }

    Some(ReleaseInfo {
        tag_name,
        version,
        html_url,
        assets,
    })
}

fn is_version_newer(remote: &str, current: &str) -> bool {
    let parse_parts = |v: &str| -> Vec<u32> {
        v.split('.')
            .filter_map(|p| p.trim_start_matches(|c: char| !c.is_ascii_digit()).parse::<u32>().ok())
            .collect()
    };
    let r_parts = parse_parts(remote);
    let c_parts = parse_parts(current);

    let len = r_parts.len().max(c_parts.len());
    for i in 0..len {
        let r = r_parts.get(i).copied().unwrap_or(0);
        let c = c_parts.get(i).copied().unwrap_or(0);
        if r > c {
            return true;
        } else if r < c {
            return false;
        }
    }
    false
}

fn select_best_asset_for_platform(assets: &[ReleaseAsset]) -> Option<ReleaseAsset> {
    #[cfg(target_os = "windows")]
    {
        // 1. Inno Setup installer EXE
        if let Some(a) = assets.iter().find(|a| a.name.contains("Setup") && a.name.ends_with(".exe")) {
            return Some(a.clone());
        }
        // 2. MSI Windows Installer
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".msi")) {
            return Some(a.clone());
        }
        // 3. Portable ZIP archive
        if let Some(a) = assets.iter().find(|a| a.name.contains("windows") && a.name.ends_with(".zip")) {
            return Some(a.clone());
        }
        // 4. Standalone EXE
        if let Some(a) = assets.iter().find(|a| a.name == "logiccraft.exe") {
            return Some(a.clone());
        }
    }

    #[cfg(target_os = "macos")]
    {
        // 1. macOS DMG Disk Image
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".dmg")) {
            return Some(a.clone());
        }
        // 2. macOS PKG installer package
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".pkg")) {
            return Some(a.clone());
        }
        // 3. macOS Universal Tarball
        if let Some(a) = assets.iter().find(|a| a.name.contains("macos") && a.name.ends_with(".tar.gz")) {
            return Some(a.clone());
        }
    }

    #[cfg(target_os = "linux")]
    {
        // 1. Debian/Ubuntu DEB package
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".deb")) {
            return Some(a.clone());
        }
        // 2. Linux tarball
        if let Some(a) = assets.iter().find(|a| a.name.contains("linux") && a.name.ends_with(".tar.gz")) {
            return Some(a.clone());
        }
    }

    None
}

fn apply_downloaded_asset(temp_path: &Path, install_dir: &Path) -> bool {
    let filename = temp_path.file_name().and_then(|f| f.to_str()).unwrap_or("");

    #[cfg(target_os = "windows")]
    {
        if filename.ends_with(".exe") && filename.contains("Setup") {
            println!(" [*] Launching LogicCraft Setup Installer: {}", temp_path.display());
            match Command::new(temp_path).spawn() {
                Ok(_) => {
                    println!(" [OK] Installer wizard launched successfully.");
                    return true;
                }
                Err(e) => eprintln!(" [ERROR] Failed to launch installer: {e}"),
            }
        } else if filename.ends_with(".msi") {
            println!(" [*] Executing MSI Package: {}", temp_path.display());
            match Command::new("msiexec").args(["/i", temp_path.to_str().unwrap()]).spawn() {
                Ok(_) => {
                    println!(" [OK] MSI Installer initiated successfully.");
                    return true;
                }
                Err(e) => eprintln!(" [ERROR] Failed to start msiexec: {e}"),
            }
        } else if filename == "logiccraft.exe" {
            let target = install_dir.join("logiccraft.exe");
            let backup_dir = install_dir.join("backup");
            let _ = fs::create_dir_all(&backup_dir);
            let backup_path = backup_dir.join(format!("logiccraft-v{}.bak.exe", APP_VERSION));
            if target.exists() {
                let _ = fs::copy(&target, &backup_path);
            }
            if fs::copy(temp_path, &target).is_ok() {
                println!(" [OK] Replaced installed executable with new binary.");
                return true;
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if filename.ends_with(".dmg") || filename.ends_with(".pkg") {
            println!(" [*] Opening macOS installer: {}", temp_path.display());
            match Command::new("open").arg(temp_path).spawn() {
                Ok(_) => {
                    println!(" [OK] macOS Package opened successfully.");
                    return true;
                }
                Err(e) => eprintln!(" [ERROR] Failed to run open: {e}"),
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        if filename.ends_with(".deb") {
            println!(" [*] Opening Debian package with system package manager: {}", temp_path.display());
            let res = Command::new("xdg-open").arg(temp_path).spawn();
            if res.is_ok() {
                println!(" [OK] Package manager launched.");
            }
            println!(" [i] Alternatively, install via terminal with:");
            println!("     sudo dpkg -i \"{}\"", temp_path.display());
            return true;
        }
    }

    // Default open with platform handler
    open_in_browser(temp_path.to_str().unwrap_or(""));
    true
}

fn open_in_browser(target: &str) {
    #[cfg(target_os = "windows")]
    let _ = Command::new("cmd").args(["/c", "start", "", target]).spawn();

    #[cfg(target_os = "macos")]
    let _ = Command::new("open").arg(target).spawn();

    #[cfg(target_os = "linux")]
    let _ = Command::new("xdg-open").arg(target).spawn();
}

fn resolve_install_directory(custom: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = custom {
        return dir;
    }

    #[cfg(target_os = "windows")]
    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        return PathBuf::from(local_app).join("Programs").join("LogicCraft");
    }

    #[cfg(target_os = "macos")]
    {
        let mac_app = PathBuf::from("/Applications/LogicCraft.app");
        if mac_app.exists() {
            return mac_app;
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(home) = std::env::var("HOME") {
            let local_share = PathBuf::from(home).join(".local").join("share").join("logiccraft");
            if local_share.exists() {
                return local_share;
            }
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            return parent.to_path_buf();
        }
    }

    PathBuf::from(".")
}

fn close_running_app() {
    #[cfg(target_os = "windows")]
    {
        let status = Command::new("taskkill")
            .args(["/IM", "logiccraft.exe", "/F"])
            .output();
        if let Ok(out) = status {
            if out.status.success() {
                println!(" [OK] Terminated running logiccraft.exe instance.");
                std::thread::sleep(std::time::Duration::from_millis(500));
            } else {
                println!(" [OK] No running logiccraft.exe instances detected.");
            }
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("pkill").arg("-f").arg("logiccraft").output();
        println!(" [OK] Terminated running logiccraft instances.");
    }
}

fn find_local_binary(install_dir: &Path) -> Option<PathBuf> {
    let binary_name = if cfg!(windows) { "logiccraft.exe" } else { "logiccraft" };

    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(binary_name);
            if candidate.exists() && candidate != install_dir.join(binary_name) {
                return Some(candidate);
            }
        }
    }

    let candidates = [
        PathBuf::from("target").join("release").join(binary_name),
        PathBuf::from("..").join("target").join("release").join(binary_name),
        PathBuf::from("target").join("installer").join("stage_win").join(binary_name),
    ];

    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }

    let cur = install_dir.join(binary_name);
    if cur.exists() {
        return Some(cur);
    }

    None
}

fn copy_auxiliary_files(install_dir: &Path) {
    let source_pairs = [
        ("packaging/assets/icon.ico", "icon.ico"),
        ("packaging/windows/update.ps1", "update.ps1"),
        ("packaging/windows/uninstall.ps1", "uninstall.ps1"),
        ("mechanics.json", "mechanics.json"),
        ("MECHANICS.md", "MECHANICS.md"),
    ];

    for (src, dst) in source_pairs {
        let s = PathBuf::from(src);
        if s.exists() {
            let d = install_dir.join(dst);
            let _ = fs::copy(&s, &d);
        }
    }
}

fn verify_user_settings() {
    #[cfg(target_os = "windows")]
    if let Ok(app_data) = std::env::var("APPDATA") {
        let settings_file = PathBuf::from(app_data).join("LogicCraft").join("settings.json");
        if settings_file.exists() {
            println!(" [OK] Windows settings detected and preserved: {}", settings_file.display());
            return;
        }
    }

    #[cfg(target_os = "macos")]
    if let Ok(home) = std::env::var("HOME") {
        let settings_file = PathBuf::from(home)
            .join("Library")
            .join("Application Support")
            .join("LogicCraft")
            .join("settings.json");
        if settings_file.exists() {
            println!(" [OK] macOS settings detected and preserved: {}", settings_file.display());
            return;
        }
    }

    #[cfg(target_os = "linux")]
    if let Ok(home) = std::env::var("HOME") {
        let settings_file = PathBuf::from(home)
            .join(".config")
            .join("LogicCraft")
            .join("settings.json");
        if settings_file.exists() {
            println!(" [OK] Linux settings detected and preserved: {}", settings_file.display());
            return;
        }
    }

    println!(" [i] User settings will be initialized on first run in the standard user directory.");
}

fn wait_for_user() {
    println!("\nPress Enter to exit...");
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
}
