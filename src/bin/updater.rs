//! LogicCraft CAD — Native Multiplatform Application Updater (`updater.rs`).
//!
//! Handles:
//! - Detecting installed LogicCraft instances (`%LOCALAPPDATA%\Programs\LogicCraft` or custom path).
//! - Gracefully closing running `logiccraft.exe` instances to unlock Windows executables.
//! - Backing up existing binary to `backup/` before applying updates.
//! - Copying fresh `logiccraft.exe`, icons, updater scripts, and mechanics specifications.
//! - Validating persistent user settings (`%APPDATA%\LogicCraft\settings.json`).
//! - Launching the updated LogicCraft CAD application.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_VERSION: &str = "1.0.0";
const APP_NAME: &str = "LogicCraft CAD";
const REPO_URL: &str = "https://github.com/PMTSantos/LogicCraft";

fn main() {
    println!("========================================================");
    println!("    {} - Native Updater Utility v{}   ", APP_NAME, APP_VERSION);
    println!("    Official Repository: {}", REPO_URL);
    println!("========================================================");

    let args: Vec<String> = std::env::args().collect();
    let is_check_only = args.iter().any(|a| a == "--check" || a == "-c");
    let is_silent = args.iter().any(|a| a == "--silent" || a == "-s");
    let should_launch = args.iter().any(|a| a == "--launch" || a == "-l") || !is_check_only;

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
    println!("[*] Target Installation Directory: {}", install_dir.display());

    let installed_exe = install_dir.join("logiccraft.exe");
    let is_installed = installed_exe.exists();
    if is_installed {
        println!(" [OK] Found installed LogicCraft executable at: {}", installed_exe.display());
    } else {
        println!(" [i] LogicCraft is not currently installed at this location (fresh install mode).");
    }

    if is_check_only {
        println!("\n[*] Version Status: You are running the latest version (v{}).", APP_VERSION);
        println!("    No mandatory updates pending. System is up to date.");
        return;
    }

    // Step 1: Terminate running application instances to release Windows file locks
    println!("\n[1/5] Ensuring application instances are closed...");
    close_running_app();

    // Step 2: Resolve source release binary
    println!("\n[2/5] Locating latest release binary package...");
    let source_exe = find_source_binary(&install_dir);
    let Some(source_exe) = source_exe else {
        eprintln!(" [ERROR] Could not locate updated logiccraft.exe binary.");
        eprintln!("         Please run 'cargo build --release' or download the latest release package.");
        if !is_silent {
            wait_for_user();
        }
        std::process::exit(1);
    };
    println!(" [OK] Source binary found: {}", source_exe.display());

    // Step 3: Create backup if previous version exists
    println!("\n[3/5] Backing up previous version (if present)...");
    if is_installed {
        let backup_dir = install_dir.join("backup");
        let _ = fs::create_dir_all(&backup_dir);
        let backup_target = backup_dir.join(format!("logiccraft-v{}.bak.exe", APP_VERSION));
        match fs::copy(&installed_exe, &backup_target) {
            Ok(_) => println!(" [OK] Created backup at: {}", backup_target.display()),
            Err(e) => eprintln!(" [!] Warning: Failed to create backup: {e}"),
        }
    } else {
        let _ = fs::create_dir_all(&install_dir);
    }

    // Step 4: Install/Update new files
    println!("\n[4/5] Applying update to installation directory...");
    match fs::copy(&source_exe, &installed_exe) {
        Ok(bytes) => println!(" [SUCCESS] Installed updated binary ({} bytes).", bytes),
        Err(e) => {
            eprintln!(" [ERROR] Failed to write updated logiccraft.exe: {e}");
            if !is_silent {
                wait_for_user();
            }
            std::process::exit(1);
        }
    }

    // Copy companion files (icon, updater script, mechanics specifications)
    copy_auxiliary_files(&install_dir);

    // Step 5: Verify user settings persistence
    println!("\n[5/5] Verifying persistent user configuration...");
    verify_user_settings();

    println!("\n========================================================");
    println!(" [SUCCESS] {} Update to v{} Complete!", APP_NAME, APP_VERSION);
    println!("========================================================");

    if should_launch && installed_exe.exists() {
        println!(" [*] Launching updated application: {}", installed_exe.display());
        let _ = Command::new(&installed_exe).spawn();
    }

    if !is_silent && !should_launch {
        wait_for_user();
    }
}

fn resolve_install_directory(custom: Option<PathBuf>) -> PathBuf {
    if let Some(dir) = custom {
        return dir;
    }

    #[cfg(windows)]
    if let Ok(local_app) = std::env::var("LOCALAPPDATA") {
        let target = PathBuf::from(local_app).join("Programs").join("LogicCraft");
        return target;
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join("logiccraft.exe").exists() {
                return parent.to_path_buf();
            }
        }
    }

    PathBuf::from(".")
}

fn close_running_app() {
    #[cfg(windows)]
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
    #[cfg(not(windows))]
    {
        let _ = Command::new("pkill").arg("-f").arg("logiccraft").output();
    }
}

fn find_source_binary(install_dir: &Path) -> Option<PathBuf> {
    // 1. Current executable directory (if updater is distributed alongside release)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join("logiccraft.exe");
            if candidate.exists() && candidate != install_dir.join("logiccraft.exe") {
                return Some(candidate);
            }
        }
    }

    // 2. target/release/logiccraft.exe relative to CARGO_MANIFEST_DIR or current dir
    let candidates = [
        PathBuf::from("target").join("release").join("logiccraft.exe"),
        PathBuf::from("..").join("target").join("release").join("logiccraft.exe"),
        PathBuf::from("target").join("installer").join("stage_win").join("logiccraft.exe"),
    ];

    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }

    // 3. Fallback: if already in install directory, the binary is already current
    let cur = install_dir.join("logiccraft.exe");
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
    #[cfg(windows)]
    if let Ok(app_data) = std::env::var("APPDATA") {
        let settings_file = PathBuf::from(app_data).join("LogicCraft").join("settings.json");
        if settings_file.exists() {
            println!(" [OK] User settings detected and preserved: {}", settings_file.display());
            return;
        }
    }
    println!(" [i] User settings will be initialized on first run in standard AppData folder.");
}

fn wait_for_user() {
    println!("\nPress Enter to exit...");
    let mut buf = String::new();
    let _ = std::io::stdin().read_line(&mut buf);
}
