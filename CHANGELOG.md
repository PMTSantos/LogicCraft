# Changelog

All notable changes to **LogicCraft CAD** are documented in this file.

## [1.0.5] - 2026-11-09

### Added
- **Wire Breakpoints on Double-Click**: Double-click on any vertical or horizontal wire segment to create an interactive breakpoint (`WireInstance::add_breakpoint`).
  - Drag the breakpoint handle in 2D (`[x, y]`) or drag the split wire segments on either side orthogonally (`WireInstance::move_breakpoint` / `WireInstance::move_segment_orthogonal`) to custom-route wires around components.
  - Double-click or right-click an existing breakpoint to remove it and merge the segment (`WireInstance::remove_breakpoint`).
  - Full persistence of wire waypoints and breakpoints in `.lcc` / `.cdl` circuit files, undo/redo snapshots, group dragging, and `Ctrl+C` / `Ctrl+V` copy-paste.

---

## [1.0.4] - 2026-11-09

### Added
- **Automatic Startup Update Check & Permission Prompt**: On launch, LogicCraft CAD automatically checks the official GitHub Releases API in the background and opens the Update Dialog with release notes and a one-click **"Download & Install Now"** button whenever a newer version is available.
- **In-App Self-Updater Engine (`src/updater.rs`)**: Downloads the new release directly from GitHub inside the app with live progress telemetry, backs up the previous executable, performs atomic in-place binary replacement (`logiccraft.old.exe`), and automatically restarts into the new version.

### Fixed
- **Verify Updates Now Button**: Fixed the Update Dialog so clicking **"Check for Updates Now"** live-updates the modal banner, GitHub release version, publication date, and release notes instead of displaying a static label.
- **Execute LogicCraft Updater Button**: Fixed launching `logiccraft-updater.exe` on Windows paths containing spaces (such as `C:\Users\Pedro Santos\...`) and added automatic in-app download/installation fallback when `logiccraft-updater.exe` is not present.
- **Standalone `logiccraft-updater.exe` Early Exit**: Fixed `logiccraft-updater.exe` exiting after Step 1 (`Checking for updates on GitHub...`) by detecting the actual installed `logiccraft.exe` file version, proceeding through all 5 download/install steps when run interactively, and preferring direct `logiccraft.exe` binary replacement.
- **Windows Installers (`.msi` / `.exe`)**: Bundled `logiccraft-updater.exe` into the WiX `.msi` installer and set `PrivilegesRequired=lowest` in the Inno Setup `.exe` installer for seamless user-space self-updates.

---

## [1.0.3] - 2026-11-09

### Added
- **Copy & Paste (`Ctrl+C` / `Ctrl+V`)**: Copy any selected component or group of components (along with internal wire connections and custom routing midpoints) using `Ctrl+C` and paste them onto the canvas with `Ctrl+V`. Consecutive pastes automatically cascade by `+2.5, -2.5` grid units.
- **Group Dragging**: Selecting multiple components renders a tactical group bounding box and allows dragging the entire group (and its interconnecting wires) together from any component or inside the group selection area.
- **Multi-Component Placement (`Ctrl` + Click)**: Hold `Ctrl` while placing a component on the canvas to place multiple instances consecutively with each mouse click. Releasing `Ctrl` or right-clicking exits placement mode.
- **Footer Cycling Tips**: Added localized, automatically cycling keyboard shortcut and workflow tips in the bottom status bar right after the `RUN` / `HALT` state indicator (click the tip to cycle immediately).

### Fixed
- **Vertical & Horizontal Wire Dragging**: Fixed vertical wire segments failing to move when dragged by evaluating wire hit-testing at the pointer press origin (`press_origin`), prioritizing `custom_mid_x` over `custom_mid_y` in orthogonal geometry reconstruction, and moving co-located net trunk segments together.
- **`TO` and `FROM` Connectors**: Fixed `DA_FROM` ("From") and `DE_TO` ("To") wireless junction connectors being misidentified as ROM memory devices (`"DA_FROM"` matched `"ROM"` substring). They no longer display the "Edit RAM/ROM Contents..." button or appear in the Memory Editor modal.
- **Pin Click vs. Component Drag Priority**: Fixed component pins hijacking drag gestures when starting a drag near a pin on a selected component or group.

### Removed
- **Canvas Telemetry HUD**: Removed the top-right coordinate, zoom, and grid overlay box inside the circuit canvas.

---

## [1.0.2] - 2026-04-19

### Added
- Complete circuit state persistence in `.lcc` / `.cdl` files including Karnaugh Maps, custom IC definitions, and multi-page tabs.
- Cross-platform native updater support for Windows, macOS, and Linux.

---

## [1.0.1] - 2026-04-19

### Added
- Native `.lcc` (LogicCraft Circuit) file format support, inline page tab renaming and closing, and pinned Karnaugh Map canvas overlays.

---

## [1.0.0] - 2026-04-19

### Added
- Initial public release of LogicCraft CAD — 5-state digital logic simulator, 11 component libraries, Quine-McCluskey Karnaugh Map Studio, Custom IC Hardware Studio, and multi-channel oscilloscope.
