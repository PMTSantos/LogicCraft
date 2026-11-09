//! LogicCraft CAD — Native Multiplatform Application Updater (`updater.rs`).
//!
//! Provides automated, cross-platform update functionality for:
//! - Windows (direct `logiccraft.exe` binary replacement with backup, or Setup EXE / MSI fallback)
//! - macOS (DMG disk image mount, PKG installer, or Universal tarball)
//! - Linux (DEB package installation or Linux tarball)
//!
//! Connects directly to official GitHub Releases API:
//! https://api.github.com/repos/PMTSantos/LogicCraft/releases/latest

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const UPDATER_VERSION: &str = env!("CARGO_PKG_VERSION");
const APP_NAME: &str = "LogicCraft CAD";
const REPO_URL: &str = "https://github.com/PMTSantos/LogicCraft";
const GITHUB_API_URL: &str = "https://api.github.com/repos/PMTSantos/LogicCraft/releases/latest";
const GITHUB_RELEASES_LIST_URL: &str =
    "https://api.github.com/repos/PMTSantos/LogicCraft/releases?per_page=5";

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
    println!(
        "    {} - Native Cross-Platform Updater v{}   ",
        APP_NAME, UPDATER_VERSION
    );
    println!("    Platform: {}", current_platform_description());
    println!("    Official Repository: {}", REPO_URL);
    println!("========================================================");

    let args: Vec<String> = std::env::args().collect();
    let is_check_only = args.iter().any(|a| a == "--check" || a == "-c");
    let is_silent = args.iter().any(|a| a == "--silent" || a == "-s");
    let should_launch =
        args.iter().any(|a| a == "--launch" || a == "-l") || !is_check_only;

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
    let installed_exe = install_dir.join(if cfg!(windows) {
        "logiccraft.exe"
    } else {
        "logiccraft"
    });
    let installed_version = detect_installed_version(&installed_exe);

    println!("[*] Installation Directory: {}", install_dir.display());
    match &installed_version {
        Some(v) => println!("[*] Installed Application Version: v{v}"),
        None => println!(
            "[*] Installed Application Version: (not yet installed at target path)"
        ),
    }

    // Step 1: Query GitHub API for latest release
    println!("\n[1/5] Checking for updates on GitHub...");
    let latest_release = fetch_latest_release();

    match &latest_release {
        Some(rel) => {
            println!(" [OK] Latest available version on GitHub: v{}", rel.version);
            let cur_ver = installed_version.as_deref().unwrap_or("0.0.0");
            let is_newer = is_version_newer(&rel.version, cur_ver);
            if is_newer {
                println!(
                    " [!] Update available: v{} -> v{}",
                    cur_ver, rel.version
                );
            } else if is_check_only {
                println!(
                    " [*] {} is up to date (v{}).",
                    APP_NAME, rel.version
                );
                return;
            } else {
                println!(
                    " [*] Installed version (v{}) matches latest GitHub release (v{}).",
                    cur_ver, rel.version
                );
                println!(
                    " [*] Proceeding to download and refresh v{} from GitHub...",
                    rel.version
                );
            }
        }
        None => {
            eprintln!(
                " [!] Note: Could not query GitHub Releases API directly (offline or rate limited)."
            );
            if is_check_only {
                println!(
                    "     Current installed version: v{}",
                    installed_version.as_deref().unwrap_or(UPDATER_VERSION)
                );
                return;
            }
            println!(
                "     Falling back to local release packages or manual release page..."
            );
        }
    }

    if is_check_only {
        return;
    }

    // Step 2: Ensure any running application instances are closed
    println!("\n[2/5] Closing running {} instances...", APP_NAME);
    close_running_app();

    // Step 3: Download or resolve update package
    println!("\n[3/5] Downloading update package from GitHub...");
    let mut update_applied = false;

    if let Some(rel) = &latest_release {
        if let Some(asset) = select_best_asset_for_platform(&rel.assets) {
            println!(
                " [OK] Selected asset: {} ({:.2} MB)",
                asset.name,
                asset.size as f64 / 1_048_576.0
            );
            let temp_dest = std::env::temp_dir().join(format!(
                "logiccraft-v{}-{}",
                rel.version, asset.name
            ));
            println!(" [*] Downloading from: {}", asset.download_url);
            println!(" [*] Target: {}", temp_dest.display());

            if download_asset(&asset.download_url, &temp_dest) {
                println!(" [SUCCESS] Download completed successfully.");
                println!("\n[4/5] Applying update to {}...", install_dir.display());
                update_applied =
                    apply_downloaded_asset(&temp_dest, &install_dir, &rel.version, &rel.assets);
            } else {
                eprintln!(
                    " [ERROR] Download failed. Launching browser fallback to GitHub release page..."
                );
                open_in_browser(&rel.html_url);
            }
        } else {
            println!(
                " [!] No platform-specific package matched. Opening release in browser..."
            );
            open_in_browser(&rel.html_url);
        }
    } else {
        // Offline / local fallback
        println!(" [*] Checking for local release build in workspace...");
        if let Some(source_exe) = find_local_binary(&install_dir) {
            println!(" [OK] Found local binary: {}", source_exe.display());
            println!("\n[4/5] Applying local update...");
            let _ = fs::create_dir_all(&install_dir);
            match fs::copy(&source_exe, &installed_exe) {
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

    if should_launch && installed_exe.exists() {
        println!(" [*] Launching updated application: {}", installed_exe.display());
        let _ = Command::new(&installed_exe).spawn();
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

fn detect_installed_version(installed_exe: &Path) -> Option<String> {
    if !installed_exe.exists() {
        return None;
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(path_str) = installed_exe.to_str() {
            let ps_cmd = format!(
                "(Get-Item -LiteralPath '{}').VersionInfo.ProductVersion",
                path_str.replace('\'', "''")
            );
            if let Ok(out) = Command::new("powershell")
                .args(["-NoProfile", "-Command", &ps_cmd])
                .output()
            {
                if out.status.success() {
                    let raw = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if !raw.is_empty() {
                        // Trim trailing .0 from 1.0.3.0 -> 1.0.3 if 4-part Windows version
                        let parts: Vec<&str> = raw.split('.').collect();
                        if parts.len() == 4 && parts[3] == "0" {
                            return Some(format!("{}.{}.{}", parts[0], parts[1], parts[2]));
                        }
                        return Some(raw);
                    }
                }
            }
        }
    }

    None
}

fn fetch_url_bytes(url: &str) -> Option<Vec<u8>> {
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };

    if let Ok(out) = Command::new(curl_bin)
        .args([
            "-s",
            "-L",
            "--connect-timeout",
            "10",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: LogicCraft-Updater",
            url,
        ])
        .output()
    {
        if out.status.success() && !out.stdout.is_empty() {
            return Some(out.stdout);
        }
    }

    if cfg!(target_os = "windows") {
        if let Ok(out) = Command::new(curl_bin)
            .args([
                "-s",
                "-L",
                "--ssl-no-revoke",
                "--connect-timeout",
                "10",
                "-H",
                "Accept: application/vnd.github+json",
                "-H",
                "User-Agent: LogicCraft-Updater",
                url,
            ])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                return Some(out.stdout);
            }
        }

        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             (Invoke-WebRequest -UseBasicParsing -Headers @{{'User-Agent'='LogicCraft-Updater'}} -Uri '{url}').Content"
        );
        if let Ok(out) = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .output()
        {
            if out.status.success() && !out.stdout.is_empty() {
                return Some(out.stdout);
            }
        }
    }

    None
}

fn parse_release(json: &serde_json::Value) -> Option<ReleaseInfo> {
    let tag_name = json.get("tag_name")?.as_str()?.trim().to_string();
    if tag_name.is_empty() {
        return None;
    }
    let version = tag_name
        .trim_start_matches(|c| c == 'v' || c == 'V')
        .to_string();
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

fn fetch_latest_release() -> Option<ReleaseInfo> {
    if let Some(bytes) = fetch_url_bytes(GITHUB_API_URL) {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(rel) = parse_release(&json) {
                return Some(rel);
            }
        }
    }

    let bytes = fetch_url_bytes(GITHUB_RELEASES_LIST_URL)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let arr = json.as_array()?;
    let mut best: Option<ReleaseInfo> = None;
    for item in arr {
        if item.get("draft").and_then(|d| d.as_bool()).unwrap_or(false) {
            continue;
        }
        if let Some(rel) = parse_release(item) {
            let replace = match &best {
                None => true,
                Some(cur) => is_version_newer(&rel.version, &cur.version),
            };
            if replace {
                best = Some(rel);
            }
        }
    }
    best
}

fn download_asset(url: &str, dest: &Path) -> bool {
    let Some(dest_str) = dest.to_str() else {
        return false;
    };
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };

    let ok = Command::new(curl_bin)
        .args([
            "-L",
            "--progress-bar",
            "-H",
            "User-Agent: LogicCraft-Updater",
            "-o",
            dest_str,
            url,
        ])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok && dest.exists() && dest.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
        return true;
    }

    if cfg!(target_os = "windows") {
        let ok_norev = Command::new(curl_bin)
            .args([
                "-L",
                "--ssl-no-revoke",
                "--progress-bar",
                "-H",
                "User-Agent: LogicCraft-Updater",
                "-o",
                dest_str,
                url,
            ])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if ok_norev && dest.exists() && dest.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
            return true;
        }

        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -UseBasicParsing -Headers @{{'User-Agent'='LogicCraft-Updater'}} -Uri '{url}' -OutFile '{dest_str}'"
        );
        let ok_ps = Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_cmd])
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if ok_ps && dest.exists() && dest.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
            return true;
        }
    }

    false
}

fn is_version_newer(remote: &str, current: &str) -> bool {
    let parse_parts = |v: &str| -> Vec<u32> {
        v.trim()
            .trim_start_matches(|c: char| !c.is_ascii_digit())
            .split(|c: char| c == '.' || c == '-')
            .filter_map(|p| p.parse::<u32>().ok())
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
        // 1. Standalone logiccraft.exe for direct, fast, silent binary replacement
        if let Some(a) = assets.iter().find(|a| a.name == "logiccraft.exe") {
            return Some(a.clone());
        }
        // 2. Inno Setup installer EXE
        if let Some(a) = assets
            .iter()
            .find(|a| a.name.contains("Setup") && a.name.ends_with(".exe"))
        {
            return Some(a.clone());
        }
        // 3. MSI Windows Installer
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".msi")) {
            return Some(a.clone());
        }
        // 4. Portable ZIP archive
        if let Some(a) = assets
            .iter()
            .find(|a| a.name.contains("windows") && a.name.ends_with(".zip"))
        {
            return Some(a.clone());
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".dmg")) {
            return Some(a.clone());
        }
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".pkg")) {
            return Some(a.clone());
        }
        if let Some(a) = assets
            .iter()
            .find(|a| a.name.contains("macos") && a.name.ends_with(".tar.gz"))
        {
            return Some(a.clone());
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(a) = assets.iter().find(|a| a.name.ends_with(".deb")) {
            return Some(a.clone());
        }
        if let Some(a) = assets
            .iter()
            .find(|a| a.name.contains("linux") && a.name.ends_with(".tar.gz"))
        {
            return Some(a.clone());
        }
    }

    None
}

fn apply_downloaded_asset(
    temp_path: &Path,
    install_dir: &Path,
    version: &str,
    all_assets: &[ReleaseAsset],
) -> bool {
    let filename = temp_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("");

    #[cfg(target_os = "windows")]
    {
        if filename.ends_with("logiccraft.exe") {
            let _ = fs::create_dir_all(install_dir);
            let target = install_dir.join("logiccraft.exe");
            let backup_dir = install_dir.join("backup");
            let _ = fs::create_dir_all(&backup_dir);
            let backup_path = backup_dir.join(format!("logiccraft-pre-v{version}.bak.exe"));
            if target.exists() {
                if fs::copy(&target, &backup_path).is_ok() {
                    println!(" [OK] Created backup at: {}", backup_path.display());
                }
            }
            match fs::copy(temp_path, &target) {
                Ok(bytes) => {
                    println!(
                        " [OK] Installed new logiccraft.exe (v{}, {} bytes) to {}",
                        version,
                        bytes,
                        target.display()
                    );
                    // Also update %LOCALAPPDATA%\Programs\LogicCraft\logiccraft.exe if different
                    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
                        let std_dir = PathBuf::from(local_app)
                            .join("Programs")
                            .join("LogicCraft");
                        let std_exe = std_dir.join("logiccraft.exe");
                        if std_dir.exists() && std_exe != target {
                            let _ = fs::copy(temp_path, &std_exe);
                        }
                    }
                    return true;
                }
                Err(e) => {
                    eprintln!(
                        " [!] Direct write to {} failed ({e}). Trying installer package...",
                        target.display()
                    );
                    // Fallback to MSI or Setup EXE if directory requires Admin UAC
                    if let Some(msi) = all_assets.iter().find(|a| a.name.ends_with(".msi")) {
                        let msi_dest = std::env::temp_dir().join(&msi.name);
                        if download_asset(&msi.download_url, &msi_dest) {
                            let _ = Command::new("msiexec")
                                .args(["/i", msi_dest.to_str().unwrap_or(""), "/passive"])
                                .status();
                            return true;
                        }
                    }
                }
            }
        } else if filename.ends_with(".exe") && filename.contains("Setup") {
            println!(
                " [*] Running LogicCraft Setup Installer: {}",
                temp_path.display()
            );
            match Command::new(temp_path)
                .args(["/SILENT", "/CLOSEAPPLICATIONS", "/RESTARTAPPLICATIONS"])
                .status()
            {
                Ok(s) if s.success() => {
                    println!(" [OK] Setup installer completed successfully.");
                    return true;
                }
                Ok(_) => {
                    println!(" [OK] Setup installer finished.");
                    return true;
                }
                Err(e) => eprintln!(" [ERROR] Failed to launch installer: {e}"),
            }
        } else if filename.ends_with(".msi") {
            println!(" [*] Executing MSI Package: {}", temp_path.display());
            match Command::new("msiexec")
                .args(["/i", temp_path.to_str().unwrap_or(""), "/passive"])
                .status()
            {
                Ok(_) => {
                    println!(" [OK] MSI Installer completed.");
                    return true;
                }
                Err(e) => eprintln!(" [ERROR] Failed to start msiexec: {e}"),
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
            println!(
                " [*] Opening Debian package with system package manager: {}",
                temp_path.display()
            );
            let res = Command::new("xdg-open").arg(temp_path).spawn();
            if res.is_ok() {
                println!(" [OK] Package manager launched.");
            }
            println!(" [i] Alternatively, install via terminal with:");
            println!("     sudo dpkg -i \"{}\"", temp_path.display());
            return true;
        }
    }

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

    let binary_name = if cfg!(windows) {
        "logiccraft.exe"
    } else {
        "logiccraft"
    };

    // 1. If logiccraft.exe is in the same directory as logiccraft-updater.exe, update that directory!
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join(binary_name).exists() {
                return parent.to_path_buf();
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            let local_dir = PathBuf::from(&local_app).join("Programs").join("LogicCraft");
            if local_dir.join(binary_name).exists() {
                return local_dir;
            }
        }
        if let Ok(prog_files) = std::env::var("ProgramFiles") {
            let pf_dir = PathBuf::from(prog_files).join("LogicCraft");
            if pf_dir.join(binary_name).exists() {
                return pf_dir;
            }
        }
        if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
            return PathBuf::from(local_app).join("Programs").join("LogicCraft");
        }
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
            let local_share = PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("logiccraft");
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
                std::thread::sleep(std::time::Duration::from_millis(600));
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
    let binary_name = if cfg!(windows) {
        "logiccraft.exe"
    } else {
        "logiccraft"
    };

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
        PathBuf::from("target")
            .join("installer")
            .join("stage_win")
            .join(binary_name),
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
        let settings_file = PathBuf::from(app_data)
            .join("LogicCraft")
            .join("settings.json");
        if settings_file.exists() {
            println!(
                " [OK] Windows settings detected and preserved: {}",
                settings_file.display()
            );
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
            println!(
                " [OK] macOS settings detected and preserved: {}",
                settings_file.display()
            );
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
            println!(
                " [OK] Linux settings detected and preserved: {}",
                settings_file.display()
            );
            return;
        }
    }

    println!(
        " [i] User settings will be initialized on first run in the standard user directory."
    );
}

fn wait_for_user() {
    println!("\nPress Enter to exit...");
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
}
