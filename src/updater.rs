//! In-app and cross-platform GitHub release updater engine for LogicCraft CAD.
//!
//! Responsibilities:
//! - Non-blocking background check against GitHub Releases API on startup and on demand.
//! - Semantic version comparison (`is_version_newer`).
//! - One-click automatic download and in-place self-update (with backup and automatic restart)
//!   after user permission.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc::Sender;

pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPO_URL: &str = "https://github.com/PMTSantos/LogicCraft";
pub const GITHUB_LATEST_API_URL: &str =
    "https://api.github.com/repos/PMTSantos/LogicCraft/releases/latest";
pub const GITHUB_RELEASES_API_URL: &str =
    "https://api.github.com/repos/PMTSantos/LogicCraft/releases?per_page=5";

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[cfg(target_os = "windows")]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Clone)]
pub struct GitHubAsset {
    pub name: String,
    pub download_url: String,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub version: String,
    pub name: String,
    pub published_at: String,
    pub html_url: String,
    pub body: String,
    pub assets: Vec<GitHubAsset>,
}

#[derive(Debug, Clone)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate(GitHubRelease),
    UpdateAvailable(GitHubRelease),
    Downloading {
        release: GitHubRelease,
        progress: String,
    },
    ReadyToRestart {
        version: String,
        message: String,
    },
    Error(String),
}

#[derive(Debug)]
pub enum UpdateMessage {
    CheckResult {
        result: Result<GitHubRelease, String>,
        manual: bool,
    },
    DownloadProgress {
        release: GitHubRelease,
        progress: String,
    },
    InstallResult(Result<(String, bool), String>),
}

/// Remove any leftover `.old.exe` binaries created during a previous self-update.
pub fn cleanup_old_binaries() {
    if let Ok(exe) = std::env::current_exe() {
        let old_exe = exe.with_extension("old.exe");
        if old_exe.exists() {
            let _ = fs::remove_file(&old_exe);
        }
        if let Some(dir) = exe.parent() {
            let old_up = dir.join("logiccraft-updater.old.exe");
            if old_up.exists() {
                let _ = fs::remove_file(&old_up);
            }
        }
    }
}

pub fn is_version_newer(remote: &str, current: &str) -> bool {
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

fn run_hidden_command(program: &str, args: &[&str]) -> Option<std::process::Output> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    #[cfg(target_os = "windows")]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.output().ok()
}

fn fetch_url_bytes(url: &str) -> Result<Vec<u8>, String> {
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };

    // Attempt 1: standard curl
    if let Some(out) = run_hidden_command(
        curl_bin,
        &[
            "-s",
            "-L",
            "--connect-timeout",
            "10",
            "-H",
            "Accept: application/vnd.github+json",
            "-H",
            "User-Agent: LogicCraft-Updater",
            url,
        ],
    ) {
        if out.status.success() && !out.stdout.is_empty() {
            return Ok(out.stdout);
        }
    }

    // Attempt 2: curl with --ssl-no-revoke (handles Windows Schannel revocation check issues)
    if cfg!(target_os = "windows") {
        if let Some(out) = run_hidden_command(
            curl_bin,
            &[
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
            ],
        ) {
            if out.status.success() && !out.stdout.is_empty() {
                return Ok(out.stdout);
            }
        }

        // Attempt 3: PowerShell Invoke-WebRequest fallback
        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             (Invoke-WebRequest -UseBasicParsing -Headers @{{'User-Agent'='LogicCraft-Updater'}} -Uri '{url}').Content"
        );
        if let Some(out) =
            run_hidden_command("powershell", &["-NoProfile", "-Command", &ps_cmd])
        {
            if out.status.success() && !out.stdout.is_empty() {
                return Ok(out.stdout);
            }
        }
    }

    Err("Could not reach GitHub Releases API (check internet connection).".to_string())
}

fn download_file_to_path(url: &str, dest: &Path) -> Result<(), String> {
    let dest_str = dest.to_str().ok_or("Invalid destination path")?;
    let curl_bin = if cfg!(target_os = "windows") {
        "curl.exe"
    } else {
        "curl"
    };

    if let Some(out) = run_hidden_command(
        curl_bin,
        &[
            "-s",
            "-L",
            "--connect-timeout",
            "15",
            "-H",
            "User-Agent: LogicCraft-Updater",
            "-o",
            dest_str,
            url,
        ],
    ) {
        if out.status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
            return Ok(());
        }
    }

    if cfg!(target_os = "windows") {
        if let Some(out) = run_hidden_command(
            curl_bin,
            &[
                "-s",
                "-L",
                "--ssl-no-revoke",
                "--connect-timeout",
                "15",
                "-H",
                "User-Agent: LogicCraft-Updater",
                "-o",
                dest_str,
                url,
            ],
        ) {
            if out.status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
                return Ok(());
            }
        }

        let ps_cmd = format!(
            "[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12; \
             Invoke-WebRequest -UseBasicParsing -Headers @{{'User-Agent'='LogicCraft-Updater'}} -Uri '{url}' -OutFile '{dest_str}'"
        );
        if let Some(out) =
            run_hidden_command("powershell", &["-NoProfile", "-Command", &ps_cmd])
        {
            if out.status.success() && dest.exists() && dest.metadata().map(|m| m.len() > 1024).unwrap_or(false) {
                return Ok(());
            }
        }
    }

    Err(format!("Failed to download update package from {url}"))
}

fn parse_release_json(json: &serde_json::Value) -> Option<GitHubRelease> {
    let tag_name = json.get("tag_name")?.as_str()?.trim().to_string();
    if tag_name.is_empty() {
        return None;
    }
    let version = tag_name
        .trim_start_matches(|c: char| c == 'v' || c == 'V')
        .to_string();
    let name = json
        .get("name")
        .and_then(|n| n.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or(&tag_name)
        .to_string();
    let published_at = json
        .get("published_at")
        .and_then(|p| p.as_str())
        .unwrap_or("")
        .chars()
        .take(10)
        .collect::<String>();
    let html_url = json
        .get("html_url")
        .and_then(|u| u.as_str())
        .unwrap_or(REPO_URL)
        .to_string();

    let raw_body = json
        .get("body")
        .and_then(|b| b.as_str())
        .unwrap_or("")
        .trim();
    let mut body_lines: Vec<&str> = Vec::new();
    for line in raw_body.lines() {
        let trimmed = line.trim();
        if body_lines.last().copied() == Some(trimmed) && trimmed.starts_with("**Full Changelog**")
        {
            continue;
        }
        body_lines.push(trimmed);
    }
    let body = body_lines.join("\n").trim().to_string();

    let mut assets = Vec::new();
    if let Some(arr) = json.get("assets").and_then(|a| a.as_array()) {
        for item in arr {
            if let (Some(aname), Some(aurl)) = (
                item.get("name").and_then(|n| n.as_str()),
                item.get("browser_download_url").and_then(|u| u.as_str()),
            ) {
                let size = item.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
                assets.push(GitHubAsset {
                    name: aname.to_string(),
                    download_url: aurl.to_string(),
                    size,
                });
            }
        }
    }

    Some(GitHubRelease {
        tag_name,
        version,
        name,
        published_at,
        html_url,
        body,
        assets,
    })
}

pub fn fetch_latest_github_release() -> Result<GitHubRelease, String> {
    // 1. Check /releases/latest
    if let Ok(bytes) = fetch_url_bytes(GITHUB_LATEST_API_URL) {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&bytes) {
            if let Some(rel) = parse_release_json(&json) {
                return Ok(rel);
            }
        }
    }

    // 2. Fallback to /releases list and pick highest version
    let bytes = fetch_url_bytes(GITHUB_RELEASES_API_URL)?;
    let json: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| format!("Invalid GitHub API JSON response: {e}"))?;

    if let Some(arr) = json.as_array() {
        let mut best: Option<GitHubRelease> = None;
        for item in arr {
            if item.get("draft").and_then(|d| d.as_bool()).unwrap_or(false) {
                continue;
            }
            if let Some(rel) = parse_release_json(item) {
                let replace = match &best {
                    None => true,
                    Some(cur) => is_version_newer(&rel.version, &cur.version),
                };
                if replace {
                    best = Some(rel);
                }
            }
        }
        if let Some(rel) = best {
            return Ok(rel);
        }
    }

    Err("No published releases found on GitHub.".to_string())
}

pub fn spawn_update_check(tx: Sender<UpdateMessage>, ctx: egui::Context, manual: bool) {
    std::thread::spawn(move || {
        let result = fetch_latest_github_release();
        let _ = tx.send(UpdateMessage::CheckResult { result, manual });
        ctx.request_repaint();
    });
}

pub fn spawn_download_and_install(
    tx: Sender<UpdateMessage>,
    ctx: egui::Context,
    release: GitHubRelease,
) {
    std::thread::spawn(move || {
        let res = perform_download_and_install(&tx, &ctx, &release);
        let _ = tx.send(UpdateMessage::InstallResult(res));
        ctx.request_repaint();
    });
}

fn send_progress(tx: &Sender<UpdateMessage>, ctx: &egui::Context, release: &GitHubRelease, msg: &str) {
    let _ = tx.send(UpdateMessage::DownloadProgress {
        release: release.clone(),
        progress: msg.to_string(),
    });
    ctx.request_repaint();
}

fn perform_download_and_install(
    tx: &Sender<UpdateMessage>,
    ctx: &egui::Context,
    release: &GitHubRelease,
) -> Result<(String, bool), String> {
    let temp_dir = std::env::temp_dir().join(format!("logiccraft-update-v{}", release.version));
    let _ = fs::create_dir_all(&temp_dir);

    #[cfg(target_os = "windows")]
    {
        // Step 1: Prefer downloading logiccraft.exe directly for instant, seamless self-replacement
        if let Some(exe_asset) = release.assets.iter().find(|a| a.name == "logiccraft.exe") {
            send_progress(
                tx,
                ctx,
                release,
                &format!(
                    "Downloading logiccraft.exe (v{}, {:.1} MB)...",
                    release.version,
                    exe_asset.size as f64 / 1_048_576.0
                ),
            );
            let downloaded_exe = temp_dir.join("logiccraft.exe");
            download_file_to_path(&exe_asset.download_url, &downloaded_exe)?;

            // Also download logiccraft-updater.exe if available in the release
            if let Some(up_asset) = release
                .assets
                .iter()
                .find(|a| a.name == "logiccraft-updater.exe")
            {
                send_progress(tx, ctx, release, "Downloading companion updater binary...");
                let downloaded_up = temp_dir.join("logiccraft-updater.exe");
                let _ = download_file_to_path(&up_asset.download_url, &downloaded_up);
            }

            send_progress(tx, ctx, release, "Applying update to installation...");

            // Try atomic in-place self-replacement of the currently running executable
            if let Ok(current_exe) = std::env::current_exe() {
                let current_dir = current_exe
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| PathBuf::from("."));

                // Backup current binary if writable
                let backup_dir = current_dir.join("backup");
                let _ = fs::create_dir_all(&backup_dir);
                let _ = fs::copy(
                    &current_exe,
                    backup_dir.join(format!("logiccraft-v{APP_VERSION}.bak.exe")),
                );

                // Update companion updater binary if we downloaded it
                let downloaded_up = temp_dir.join("logiccraft-updater.exe");
                if downloaded_up.exists() {
                    let target_up = current_dir.join("logiccraft-updater.exe");
                    let old_up = current_dir.join("logiccraft-updater.old.exe");
                    let _ = fs::remove_file(&old_up);
                    if target_up.exists() {
                        let _ = fs::rename(&target_up, &old_up);
                    }
                    let _ = fs::copy(&downloaded_up, &target_up);
                }

                // Rename running binary to logiccraft.old.exe and copy new binary into place
                let old_exe = current_exe.with_extension("old.exe");
                let _ = fs::remove_file(&old_exe);
                if fs::rename(&current_exe, &old_exe).is_ok() {
                    match fs::copy(&downloaded_exe, &current_exe) {
                        Ok(_) => {
                            // Also keep %LOCALAPPDATA%\Programs\LogicCraft in sync if it exists
                            if let Ok(local_appdata) = std::env::var("LOCALAPPDATA") {
                                let std_dir = PathBuf::from(local_appdata)
                                    .join("Programs")
                                    .join("LogicCraft");
                                let std_exe = std_dir.join("logiccraft.exe");
                                if std_dir.exists() && std_exe != current_exe {
                                    let _ = fs::copy(&downloaded_exe, &std_exe);
                                    if downloaded_up.exists() {
                                        let _ = fs::copy(
                                            &downloaded_up,
                                            std_dir.join("logiccraft-updater.exe"),
                                        );
                                    }
                                }
                            }
                            return Ok((release.version.clone(), true));
                        }
                        Err(e) => {
                            // Rollback rename if copy failed
                            let _ = fs::rename(&old_exe, &current_exe);
                            return Err(format!("Failed to write updated binary: {e}"));
                        }
                    }
                }
            }
        }

        // Step 2: Fallback to Setup EXE or MSI installer if in-place replacement wasn't possible
        // (e.g., installed in C:\Program Files requiring elevation)
        if let Some(setup_asset) = release
            .assets
            .iter()
            .find(|a| a.name.contains("Setup") && a.name.ends_with(".exe"))
        {
            send_progress(
                tx,
                ctx,
                release,
                &format!("Downloading installer {}...", setup_asset.name),
            );
            let setup_path = temp_dir.join(&setup_asset.name);
            download_file_to_path(&setup_asset.download_url, &setup_path)?;
            send_progress(tx, ctx, release, "Launching Windows Setup installer...");
            Command::new(&setup_path)
                .args([
                    "/SILENT",
                    "/CLOSEAPPLICATIONS",
                    "/RESTARTAPPLICATIONS",
                ])
                .spawn()
                .map_err(|e| format!("Failed to start installer: {e}"))?;
            std::thread::sleep(std::time::Duration::from_millis(600));
            std::process::exit(0);
        }

        if let Some(msi_asset) = release.assets.iter().find(|a| a.name.ends_with(".msi")) {
            send_progress(
                tx,
                ctx,
                release,
                &format!("Downloading MSI package {}...", msi_asset.name),
            );
            let msi_path = temp_dir.join(&msi_asset.name);
            download_file_to_path(&msi_asset.download_url, &msi_path)?;
            send_progress(tx, ctx, release, "Launching Windows MSI installer...");
            Command::new("msiexec")
                .args(["/i", msi_path.to_str().unwrap_or(""), "/passive"])
                .spawn()
                .map_err(|e| format!("Failed to launch msiexec: {e}"))?;
            std::thread::sleep(std::time::Duration::from_millis(600));
            std::process::exit(0);
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(asset) = release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".pkg") || a.name.ends_with(".dmg"))
        {
            send_progress(
                tx,
                ctx,
                release,
                &format!("Downloading {}...", asset.name),
            );
            let pkg_path = temp_dir.join(&asset.name);
            download_file_to_path(&asset.download_url, &pkg_path)?;
            let _ = Command::new("open").arg(&pkg_path).spawn();
            return Ok((release.version.clone(), false));
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(asset) = release
            .assets
            .iter()
            .find(|a| a.name.ends_with(".deb") || a.name.ends_with(".tar.gz"))
        {
            send_progress(
                tx,
                ctx,
                release,
                &format!("Downloading {}...", asset.name),
            );
            let pkg_path = temp_dir.join(&asset.name);
            download_file_to_path(&asset.download_url, &pkg_path)?;
            let _ = Command::new("xdg-open").arg(&pkg_path).spawn();
            return Ok((release.version.clone(), false));
        }
    }

    open_url(&release.html_url);
    Ok((release.version.clone(), false))
}

/// Restart the application by spawning `current_exe()` and exiting the current process.
pub fn restart_application() {
    if let Ok(exe) = std::env::current_exe() {
        let _ = Command::new(&exe).spawn();
        std::process::exit(0);
    }
}

/// Launch the standalone `logiccraft-updater` binary in its own console window,
/// properly handling paths with spaces on Windows.
pub fn launch_standalone_updater() -> bool {
    let updater_name = if cfg!(windows) {
        "logiccraft-updater.exe"
    } else {
        "logiccraft-updater"
    };

    let current_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()));

    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(ref d) = current_dir {
        candidates.push(d.join(updater_name));
    }
    #[cfg(windows)]
    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(local_app)
                .join("Programs")
                .join("LogicCraft")
                .join(updater_name),
        );
    }
    candidates.push(PathBuf::from("target").join("release").join(updater_name));
    candidates.push(PathBuf::from("target").join("debug").join(updater_name));

    let found = candidates.into_iter().find(|p| p.exists());
    let Some(exe) = found else {
        return false;
    };

    #[cfg(windows)]
    {
        const CREATE_NEW_CONSOLE: u32 = 0x00000010;
        let mut cmd = Command::new(&exe);
        if let Some(ref d) = current_dir {
            cmd.arg("--dir").arg(d);
        }
        cmd.arg("--force").arg("--launch");
        cmd.creation_flags(CREATE_NEW_CONSOLE);
        cmd.spawn().is_ok()
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "Terminal"])
            .arg(&exe)
            .spawn()
            .is_ok()
    }

    #[cfg(target_os = "linux")]
    {
        Command::new("x-terminal-emulator")
            .arg("-e")
            .arg(&exe)
            .spawn()
            .or_else(|_| Command::new(&exe).spawn())
            .is_ok()
    }

    #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
    {
        Command::new(&exe).spawn().is_ok()
    }
}

pub fn open_url(url: &str) {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", "start", "", url]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        let _ = cmd.spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = Command::new("xdg-open").arg(url).spawn();
    }
}
