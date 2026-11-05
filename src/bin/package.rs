use std::process::Command;
use std::path::{Path, PathBuf};
use std::fs;

fn main() {
    println!("========================================================");
    println!("    LogicCraft CAD - Native Rust Packaging Utility       ");
    println!("========================================================");

    let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    println!("[*] Project Root: {}", project_root.display());

    let target_dir = project_root.join("target").join("installer");
    fs::create_dir_all(&target_dir).expect("Failed to create target/installer directory");

    let args: Vec<String> = std::env::args().collect();
    let target_msi = args.iter().any(|a| a == "--msi" || a == "-m");
    let target_pkg = args.iter().any(|a| a == "--pkg" || a == "-p");
    let target_all = !target_msi && !target_pkg;

    // Step 1: Ensure release build is up to date
    println!("\n[1/3] Building release binaries with cargo build --release --bins...");
    let status = Command::new("cargo")
        .args(["build", "--release", "--bins"])
        .current_dir(&project_root)
        .status();

    match status {
        Ok(s) if s.success() => println!(" [OK] Release binary compiled successfully."),
        Ok(s) => eprintln!(" [ERROR] cargo build failed with exit code: {:?}", s.code()),
        Err(e) => eprintln!(" [ERROR] Failed to run cargo build: {e}"),
    }

    // Step 2: Windows MSI packaging
    if target_all || target_msi {
        println!("\n[2/3] Checking Windows MSI packaging options...");
        build_windows_msi(&project_root, &target_dir);
    }

    // Step 3: macOS PKG packaging
    if target_all || target_pkg {
        println!("\n[3/3] Checking macOS PKG packaging options...");
        build_macos_pkg(&project_root, &target_dir);
    }

    // Summary & cargo-packager advice
    println!("\n========================================================");
    println!(" Packaging Summary & Direct Rust Tools:");
    println!("========================================================");
    println!(" 1. cargo-packager (Direct Rust Multiplatform Installer Tool):");
    println!("    Install: cargo install cargo-packager --locked");
    println!("    Build MSI: cargo packager --release --formats wix");
    println!("    Build PKG/DMG: cargo packager --release --formats dmg,pkg");
    println!("    Configured in Cargo.toml: [package.metadata.packager]");
    println!();
    println!(" 2. cargo-wix (Direct Rust Windows MSI Tool):");
    println!("    Install: cargo install cargo-wix --locked");
    println!("    Build MSI: cargo wix --nocapture");
    println!("    Configured in Cargo.toml: [package.metadata.wix]");
    println!();
    println!(" 3. Built Artifacts Directory: {}", target_dir.display());
    println!("========================================================");
}

fn build_windows_msi(root: &Path, out_dir: &Path) {
    if cfg!(target_os = "windows") {
        let candle_exists = Command::new("candle.exe").arg("-?").output().is_ok();
        let light_exists = Command::new("light.exe").arg("-?").output().is_ok();

        if candle_exists && light_exists {
            println!(" [*] WiX Toolset detected. Generating MSI installer...");
            let wxs_path = root.join("packaging").join("windows").join("installer.wxs");
            let wixobj = out_dir.join("installer.wixobj");
            let msi_out = out_dir.join("LogicCraft-2.5.0-x64.msi");

            let c_status = Command::new("candle.exe")
                .args(["-arch", "x64", "-out"])
                .arg(&wixobj)
                .arg(&wxs_path)
                .current_dir(root)
                .status();

            if let Ok(cs) = c_status {
                if cs.success() {
                    let l_status = Command::new("light.exe")
                        .args(["-ext", "WixUIExtension", "-out"])
                        .arg(&msi_out)
                        .arg(&wixobj)
                        .current_dir(root)
                        .status();

                    if let Ok(ls) = l_status {
                        if ls.success() {
                            println!(" [SUCCESS] Created MSI installer: {}", msi_out.display());
                            return;
                        }
                    }
                }
            }
        }

        // Check if cargo-packager is installed
        let packager_exists = Command::new("cargo-packager").arg("--version").output().is_ok();
        if packager_exists {
            println!(" [*] cargo-packager detected. Running cargo packager for MSI...");
            let _ = Command::new("cargo")
                .args(["packager", "--release", "--formats", "wix"])
                .current_dir(root)
                .status();
            return;
        }

        println!(" [i] WiX (candle/light) or cargo-packager not currently installed in PATH.");
        println!("     Run packaging\\windows\\build_installer.ps1 or install WiX / cargo-packager.");
    } else {
        println!(" [i] Windows MSI generation is designed for Windows hosts (or Wine).");
    }
}

fn build_macos_pkg(root: &Path, _out_dir: &Path) {
    if cfg!(target_os = "macos") {
        let pkg_script = root.join("packaging").join("macos").join("build_pkg.sh");
        if pkg_script.exists() {
            println!(" [*] macOS detected. Running packaging/macos/build_pkg.sh...");
            let status = Command::new("bash")
                .arg(&pkg_script)
                .current_dir(root)
                .status();
            if let Ok(s) = status {
                if s.success() {
                    println!(" [SUCCESS] Built macOS PKG installer.");
                    return;
                }
            }
        }
    } else {
        println!(" [i] macOS PKG generation requires macOS pkgbuild/productbuild tools.");
        println!("     Scripts created in packaging/macos/ (build_app.sh, build_pkg.sh, build_dmg.sh).");
    }
}
