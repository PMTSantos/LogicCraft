# Changelog

All notable changes to **LogicCraft CAD** are documented in this file.

## [1.1.0] - 2026-11-10

### Added
- **Modular Multi-Architecture CPU Infrastructure (`src/cpu/`)**:
  - **Pluggable `CpuCore` Trait**: Unified hardware interface (`step()`, `reset()`, `load_program()`, `registers()`, `disassemble_current()`, `disassemble_at()`, `get_state()`, `set_pc()`) allowing runtime switching between processor architectures.
  - **Three Built-In Processor Cores**:
    1. **RISC-V RV32I (`Rv32iCore`)**: 32-bit general-purpose processor with 32 registers (`x0`–`x31`), arithmetic/logic operations, branches, jumps (`jal`/`jalr`), multiplication/division (`RV32M`), and byte/halfword/word memory access.
    2. **LC-16 Custom (`Lc16Core`)**: 16-bit custom RISC/CISC architecture with 16 primary registers (`r0`–`r15`), status flags (`Z`, `N`, `C`, `V`), stack instructions (`PUSH`/`POP`/`CALL`/`RET`), and full assembler compatibility.
    3. **MIN-8 Minimal (`Min8Core`)**: 8-bit educational processor core supporting compact accumulator/register instructions (`LDI`, `LDA`, `STA`, `ADD`, `SUB`, `INC`, `DEC`, `CMP`, `JMP`, `JZ`, `JNZ`).
  - **Unified 64 KB Memory Bus & Memory-Mapped I/O (`MemoryBus`)**:
    - `0x0000..=0x3FFF` (16 KB): Program ROM / Code segment.
    - `0x4000..=0x7FFF` (16 KB): Data RAM and Stack (`SP` initialized to `0x7FFC`).
    - `0x8000..=0xEFFF` (28 KB): Video RAM (VRAM) supporting multiple simultaneous graphical displays.
    - `0xF000..=0xFFFF` (4 KB): Memory-Mapped I/O (MMIO) registers for keyboard state (`0xF000`–`0xF006`), hardware RNG (`0xF010`), frame tick counter (`0xF012`), 60 Hz VSync (`0xF016`), Player 1 & Player 2 scores (`0xF020`–`0xF021`), GPIO pins (`0xF022`–`0xF023`), and fast VRAM clear (`0xF030`).
  - **Integrated Multi-Architecture Assembler (`src/cpu/assembler.rs`)**:
    - Supports labels, symbolic constants (`.equ`), memory directives (`.org`, `.byte`, `.word`, `.dword`, `.space`), pseudo-instructions (`li`, `la`, `mv`, `j`, `ret`, `neg`, `not`, `bgt`, `ble`, `beqz`, `bnez`), and line-accurate syntax error reporting with highlighted source lines.
- **New Canvas Hardware Components (`12 - CPU & Peripherals`)**:
  - **Programmable CPU (`CPU_COMPONENT`)**: Interactive processor package on the circuit canvas with bus/control pins (`CLK`, `RST`, `IN_0..3`, `ADDR_0..3`, `DATA_0..3`, `RW`, `HALT`), configurable between an internal high-speed clock (**1 kHz to 1 MHz**) and external circuit clock pin stepping.
  - **VRAM Graphical Monitor (`DISPLAY_COMPONENT`)**: Add multiple graphical monitors to the same circuit canvas, each independently configurable in the Inspector for **VRAM Base Address** (`0x8000`, `0x8800`, `0x9000`, `0xA000`), **Resolution** (`64x32`, `128x64`, `64x64`), and **Color Palette** (*Classic Monochrome*, *Amber CRT*, *Matrix Green*, and *Game Boy 4-Color*).
  - **Arcade Keyboard Input (`KEYBOARD_COMPONENT`)**: Captures real-time keyboard events (**Player 1**: `W`/`S`, **Player 2**: `Up`/`Down`, **Start/Reset**: `Space`/`Enter`) and maps them to both MMIO registers (`0xF000`–`0xF005`) and digital output pins on the canvas.
- **CPU & Assembly Studio / Real-Time Debugger (`Ctrl+M` / `🖥 CPU / ASM`)**:
  - Embedded Assembly code editor with line numbers, active `PC` line indicator (`▶`), syntax error banner with highlighted offending lines, CPU architecture selector dropdown, and **Assemble & Load**, **Load .asm**, **Save .asm**, and **Pong.asm** actions.
  - Real-time simulation controls: **Run**, **Pause**, **Single Step** (1 instruction), **Reset**, clock mode toggle, and real-time clock speed slider (**1 kHz – 1 MHz**).
  - Live hardware inspection panel featuring a **Register Viewer** (Hexadecimal and Decimal values with modified-register highlighting, `PC`, `SP`, cycle counter, and `Z/N/C/V` flags), **Live Disassembler**, and **Memory Hex Dump Viewer** with one-click jumps to `ROM 0x0000`, `RAM 0x4000`, `VRAM 0x8000`, and `MMIO 0xF000`.
- **2-Player Local Pong Arcade Game in Assembly (`examples/pong.asm`)**:
  - Complete playable 2-player local Pong demonstration pre-loaded on the **Pong Arcade (CPU + MMIO)** circuit tab (`🏓 PONG` toolbar button) featuring `W`/`S` and `Up`/`Down` paddle controls, 2D ball physics $(dx, dy)$, top/bottom wall and paddle collision reflection, scoring with center reset, and 60 FPS VSync-paced VRAM rendering.

---

## [1.0.6] - 2026-11-09

### Fixed
- **Prevent Duplicate App Installations on Update**: Fixed the updater creating a second installation in `%LOCALAPPDATA%\Programs\LogicCraft` when updating an existing `C:\Program Files\LogicCraft` (MSI) installation.
  - Updates now always replace the active installation in-place (requesting Administrator elevation if installed in `C:\Program Files\LogicCraft`).
  - Added automatic duplicate installation cleanup (`sync_windows_uninstall_and_cleanup_duplicates`) and Windows Uninstall registry version/name synchronization (`AppVerName=LogicCraft CAD`) so only one LogicCraft CAD app and shortcut exist on the system.

---

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
