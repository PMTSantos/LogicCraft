# LogicCraft CAD — Digital Logic Circuit Studio

[![Release](https://img.shields.io/github/v/release/PMTSantos/LogicCraft?include_prereleases&style=flat-square&color=00d26a)](https://github.com/PMTSantos/LogicCraft/releases)
[![Build Status](https://img.shields.io/github/actions/workflow/status/PMTSantos/LogicCraft/release.yml?branch=main&style=flat-square)](https://github.com/PMTSantos/LogicCraft/actions)
[![License](https://img.shields.io/badge/License-MIT-blue.svg?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%20%7C%20macOS%20%7C%20Linux-orange.svg?style=flat-square)](https://github.com/PMTSantos/LogicCraft)
[![Rust](https://img.shields.io/badge/Rust-1.75%2B-blue.svg?style=flat-square&logo=rust)](https://www.rust-lang.org)

Modern cross-platform digital logic circuit builder, 5-state logic simulator, Quine-McCluskey Karnaugh Map solver, and custom hardware specification studio written in **Rust + egui (`eframe`)**.

- **Repository**: [https://github.com/PMTSantos/LogicCraft](https://github.com/PMTSantos/LogicCraft)
- **Latest Releases**: [https://github.com/PMTSantos/LogicCraft/releases](https://github.com/PMTSantos/LogicCraft/releases)

---

## Table of Contents
1. [Key Features](#key-features)
2. [Prerequisites & System Requirements](#prerequisites--system-requirements)
3. [Building the Application](#building-the-application)
4. [Running the Application](#running-the-application)
5. [Testing & Quality Assurance](#testing--quality-assurance)
6. [Creating Windows Installers & Packages](#creating-windows-installers--packages)
7. [Native Application Updater](#native-application-updater)
8. [GitHub Releases & Versioning Workflow](#github-releases--versioning-workflow)
9. [Persistent Settings](#persistent-settings)
10. [Changelog](#changelog)
11. [Repository Structure](#repository-structure)

---

## Key Features

- **5-State Logic Engine**: Accurate evaluation of digital states (`0`, `1`, `Hi-Z`, `Conflict`, `Unknown`).
- **11 Gate Libraries**: Comprehensive IEEE US standard gate packages, switches, hex keypads, multiplexers, decoders, arithmetic units, flip-flops, registers, RAM/ROM, and custom ICs.
- **Dynamic Minimap**: Aspect-ratio-preserving overview of the full circuit with real-time viewport tracking and instant click-to-pan navigation.
- **Cross-Session Persistence**: Automatically remembers your language (`EN`, `PT`, `ES`, `FR`, `DE`), theme (Dark/Light), grid visibility, wire solder dots, and oscilloscope panel.
- **Clean Windows Desktop Experience**: Configured with `#![windows_subsystem = "windows"]` — no terminal or CMD prompt window opens when launching the application.
- **Integrated Native Updater**: Seamless in-app checking and standalone background updater (`logiccraft-updater.exe` / `update.ps1`) that safely handles executable locks and upgrades installations.

---

## Prerequisites & System Requirements

| Tool / Requirement | Minimum Version | Notes |
|---|---|---|
| **Operating System** | Windows 10/11, macOS 10.15+, or Linux | Primary targets: Windows x86_64, macOS Apple Silicon / Intel, Linux x64 |
| **Rust Toolchain** | `1.75.0` or newer | Installed via [rustup.rs](https://rustup.rs) (`cargo`, `rustc`) |
| **Graphics Backend** | OpenGL 3.3+ or DirectX 12 / Vulkan | Glow (default) or WGPU selectable via `--glow` or `--wgpu` |
| **WiX Toolset** *(Optional)* | 3.11+ (`candle.exe`, `light.exe`) | For generating Windows `.msi` installers |
| **Inno Setup** *(Optional)* | 6.0+ (`iscc.exe`) | For generating Windows Setup `.exe` installers |
| **cargo-packager** *(Optional)* | Latest | `cargo install cargo-packager --locked` for cross-platform installers |

---

## Building the Application

### 1. Debug Build (Rapid Development)
```powershell
cargo build
```

### 2. Optimized Release Build
Compiles with link-time optimization and the Windows GUI subsystem (no console window):
```powershell
cargo build --release
```
The output executable is created at:
```
target/release/logiccraft.exe
```

### 3. Build All Binaries
Compiles `logiccraft.exe`, the native updater `logiccraft-updater.exe`, and the packaging utility `package.exe`:
```powershell
cargo build --release --bins
```

---

## Running the Application

### Launching with Cargo
```powershell
cargo run --release
```

### Command-Line Options
```powershell
# Set initial locale on launch (en, pt, es, fr, de)
cargo run --release -- --lang=pt
cargo run --release -- --lang=es

# Select graphics rendering backend
cargo run --release -- --glow      # OpenGL backend (default, broad compatibility)
cargo run --release -- --wgpu      # WGPU DirectX 12 / Vulkan backend

# Headless automated screenshot capture
cargo run --release -- --screenshot=screenshot.png
```

---

## Testing & Quality Assurance

Run the comprehensive test suite:
```powershell
cargo test
```
Includes:
- Settings serialization / deserialization round-trip tests
- Configuration fallback validation for partial configs
- Multi-page circuit tab management and tracking tests

---

## Creating Windows Installers & Packages

### 1. Automated Windows Installer Build Suite (Recommended)
This script builds release binaries and packages all Windows distribution formats into `target/installer/`:
```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\build_installer.ps1
```
Output files in `target\installer\`:
- `LogicCraft-v1.0.4-windows-x64.zip` — Standalone portable archive with updater and installer scripts
- `LogicCraft-1.0.4-x64.msi` — Windows Installer package (when WiX or `cargo-packager` is installed)
- `LogicCraft-Setup-1.0.4-x64.exe` — Inno Setup executable (when `iscc.exe` is installed)

### 2. Direct User Installation Script
Installs LogicCraft CAD directly to `%LOCALAPPDATA%\Programs\LogicCraft`, registers the `.lcc` file association, and creates Start Menu and Desktop shortcuts:
```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1
```

### 3. Uninstallation Script
Cleans shortcuts, registry file associations, and application binaries:
```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\uninstall.ps1
```

---

## Creating macOS Installers & Packages

### 1. macOS DMG Disk Image (`.dmg`)
Builds the standalone macOS `.app` bundle with icon conversion and generates a drag-and-drop DMG:
```bash
bash packaging/macos/build_dmg.sh
```
Output: `target/installer/LogicCraft-1.0.4.dmg`

### 2. macOS Component Package (`.pkg`)
Generates an Apple installer package configured for `/Applications`:
```bash
bash packaging/macos/build_pkg.sh
```
Output: `target/installer/LogicCraft-1.0.4.pkg` (or fallback `LogicCraft-v1.0.4-macos.tar.gz`)

---

## Creating Linux Installers & Packages

### 1. Debian / Ubuntu Package (`.deb`)
Packages the binaries, `.desktop` launcher, and high-resolution icons into a native Debian/Ubuntu package:
```bash
bash packaging/linux/build_deb.sh
```
Output: `target/installer/logiccraft_1.0.4_amd64.deb`

To install:
```bash
sudo dpkg -i target/installer/logiccraft_1.0.4_amd64.deb
```

### 2. Standalone Linux Portable Tarball (`.tar.gz`)
Generates a portable archive with standalone executable, desktop launcher, and user `install.sh`:
```bash
bash packaging/linux/build_tarball.sh
```
Output: `target/installer/LogicCraft-v1.0.4-linux-x64.tar.gz`

---

## Native Application Updater

LogicCraft CAD provides three complementary updater workflows:

### 1. Standalone Native Binary (`logiccraft-updater.exe`)
Built with `cargo build --release --bin logiccraft-updater`.

```powershell
# Interactive mode (checks status, backs up old binary, applies update, launches app)
target\release\logiccraft-updater.exe

# Check update status without modifying any files
target\release\logiccraft-updater.exe --check

# Silent automated update (for automation or scripts)
target\release\logiccraft-updater.exe --silent

# Target a custom installation directory
target\release\logiccraft-updater.exe --dir "C:\Custom\LogicCraft"

# Launch application after update
target\release\logiccraft-updater.exe --launch
```

### 2. Windows PowerShell Updater Script (`update.ps1`)
Safely terminates running `logiccraft.exe` processes to release file locks, creates a backup in `backup/`, copies updated binaries and mechanics specifications, verifies user settings, and restarts the app:
```powershell
powershell -ExecutionPolicy Bypass -File packaging\windows\update.ps1
```

Options:
```powershell
# Update without relaunching the app
powershell -ExecutionPolicy Bypass -File packaging\windows\update.ps1 -NoLaunch

# Update a custom directory
powershell -ExecutionPolicy Bypass -File packaging\windows\update.ps1 -InstallDir "C:\Custom\LogicCraft"
```

### 3. In-App Updater Modal
- Open the top menu: **Help** → **Check for Updates...** (or in Portuguese: **Ajuda** → **Verificar Atualizações...**).
- Or open **Help** → **About LogicCraft** and click **[ 🔄 Check for Updates / Atualizações ]**.
- From this window, users can verify version status, launch the native updater, open GitHub Releases, or view the installation folder.

---

## GitHub Releases & Versioning Workflow

The repository is configured with automated GitHub Actions workflows:

### Creating a New Version & GitHub Release
1. Update the version in `Cargo.toml` (e.g. `1.0.4`, `1.1.0`).
2. Commit and tag the release:
   ```powershell
   git add .
   git commit -m "fix(v1.0.4): automatic startup update check, in-app self-updater, and standalone updater fixes"
   git tag -a v1.0.4 -m "LogicCraft CAD v1.0.4 Release"
   git push origin main
   git push origin v1.0.4
   ```
3. The GitHub Actions release workflow (`.github/workflows/release.yml`) automatically triggers on the `v*` tag:
   - Builds optimized multi-platform release binaries on Windows, macOS, and Linux
   - Generates the portable packages and installers (`.exe`, `.msi`, `.dmg`, `.pkg`, `.deb`, `.tar.gz`, `.zip`)
   - Publishes the GitHub Release with downloadable release assets for all three operating systems

---

## Persistent Settings

User preferences are automatically preserved across application restarts in:
- **Windows**: `%APPDATA%\LogicCraft\settings.json`
- **macOS**: `~/Library/Application Support/LogicCraft/settings.json`
- **Linux**: `~/.config/logiccraft/settings.json`

### Persisted Options:
- **Language / Locale**: English (`En`), Português (`Pt`), Español (`Es`), Français (`Fr`), Deutsch (`De`)
- **Theme**: Dark mode or Light mode
- **Gridlines**: Visible or Hidden
- **Solder Dots**: Wire connection terminals visible or hidden
- **Oscilloscope**: Technical logic analyzer visible or hidden
- **Image Exporter**: Canvas grid and color preferences

---

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for the full release history.

### v1.1.0
- **Added**: Modular Multi-Architecture Programmable CPU (`RV32I` 32-bit, `LC-16` 16-bit, `MIN-8` 8-bit) with a unified 64 KB Memory Bus, Memory-Mapped I/O (MMIO), and an integrated Assembler supporting labels, `.org`, `.byte`, `.word`, `.equ`, and line-highlighted syntax errors.
- **Added**: New hardware components in library `12 - CPU & Peripherals`: `CPU_COMPONENT` (1 kHz – 1 MHz internal clock or circuit pin clock), multiple configurable `DISPLAY_COMPONENT` VRAM monitors (custom VRAM base address, `64x32`/`128x64`/`64x64` resolutions, and Monochrome/Amber/Matrix Green/Game Boy palettes), and `KEYBOARD_COMPONENT` (capturing `W`/`S`, `Up`/`Down`, and `Space`/`Enter`).
- **Added**: **CPU & Assembly Studio / Debugger** (`Ctrl+M` / `🖥 CPU / ASM`) with an embedded Assembly editor, `Run`/`Pause`/`Single Step`/`Reset` controls, live Hex & Decimal register inspector, live disassembler around `PC`, and interactive Memory Hex Dump with quick jump to VRAM (`0x8000`).
- **Added**: Complete 2-player local **Pong** game in Assembly (`examples/pong.asm`) pre-configured on the **Pong Arcade (CPU + MMIO)** circuit tab (`🏓 PONG`).

### v1.0.6
- **Fixed**: Prevented the updater from creating a second duplicate LogicCraft installation when updating across `C:\Program Files\LogicCraft` and `%LOCALAPPDATA%\Programs\LogicCraft`, and added automatic duplicate cleanup and Windows Installed Apps version synchronization.

### v1.0.5
- **Added**: Double-click on any wire segment to create a movable breakpoint for flexible orthogonal wire routing (drag the breakpoint in 2D or drag split segments independently; double-click or right-click a breakpoint to remove it).

### v1.0.4
- **Added**: Automatic background GitHub update check on application startup with a one-click permission prompt to download, install, and restart automatically.
- **Fixed**: Live GitHub version and release notes updates in the Update Dialog when clicking "Check for Updates Now".
- **Fixed**: In-app self-updater and standalone `logiccraft-updater.exe` execution (no longer exits early after Step 1 or fails on paths with spaces).

### v1.0.3
- **Added**: `Ctrl+C` and `Ctrl+V` clipboard copy and paste for selected components and internal wire connections (with cascaded grid offset on repeated pastes).
- **Added**: Multi-component group dragging — drag any selected component or anywhere inside the group bounding box to move the entire selection and its internal wires together.
- **Added**: Continuous multi-component placement while holding `Ctrl` and clicking on the canvas.
- **Added**: Localized cycling shortcut and workflow tips in the footer status bar right after the `RUN` / `HALT` indicator.
- **Fixed**: Reliable vertical and horizontal wire segment dragging (using press origin hit-testing, co-located net trunk synchronization, and `custom_mid_x` routing priority).
- **Fixed**: Removed the RAM/ROM memory editor button and memory device classification from `FROM` (`DA_FROM`) and `TO` (`DE_TO`) junction connectors.
- **Removed**: Top-right canvas telemetry HUD overlay (`X`, `Y`, `ZOOM`, `GRID`).

---

## Repository Structure

```
app/
├── .github/
│   └── workflows/
│       ├── ci.yml                # Automated CI test and check workflow
│       └── release.yml           # Automated release and artifact publishing
├── .gitignore                    # Git ignore specifications
├── Cargo.toml                    # Package manifest & metadata
├── build.rs                      # Windows resource compiler (embeds icon and metadata)
├── README.md                     # Documentation and build instructions
├── src/
│   ├── main.rs                   # Entrypoint (GUI subsystem, settings loader, app loop)
│   ├── settings.rs               # AppSettings model (load/save/persistence in %APPDATA%)
│   ├── i18n.rs                   # Localization engine (EN, PT, ES, FR, DE strings)
│   ├── ui_dialogs.rs             # Dialog windows (About, Updater, K-Map, RAM/ROM)
│   ├── ui_panels.rs              # Menus, toolbars, and dynamic minimap renderer
│   ├── canvas_view.rs            # Circuit canvas and interactive workspace
│   ├── canvas_renderer.rs        # Vector gate symbols, pins, and IC packages
│   ├── library.rs                # 11 component libraries & pin definitions
│   ├── simulator.rs              # 5-state digital logic simulation engine
│   ├── custom_component.rs       # Custom IC JSON specification engine
│   ├── kmap.rs                   # Quine-McCluskey K-Map solver & truth tables
│   ├── image_export.rs           # PNG / BMP circuit export engine
│   ├── cdl_io.rs                 # Circuit (.cdl), memory (.cdm), and trace (.cdo) I/O
│   ├── mechanics.rs              # 5-state types and visual layout constants
│   ├── theme.rs                  # Industrial color palette and styling
│   └── bin/
│       ├── updater.rs            # Standalone native updater binary (`logiccraft-updater`)
│       └── package.rs            # Multiplatform packaging automation tool
├── packaging/
│   ├── assets/                   # Application icons (icon.ico, icon.png)
│   ├── windows/
│   │   ├── install.ps1           # User-level installation script
│   │   ├── update.ps1            # Standalone update script
│   │   ├── uninstall.ps1         # Complete cleanup script
│   │   ├── build_installer.ps1   # Installer compilation suite
│   │   ├── installer.iss         # Inno Setup compilation script
│   │   └── installer.wxs         # WiX Toolset MSI definition
│   └── macos/                    # macOS bundle and packaging scripts
└── shots/                        # Verification screenshots
```
