## LogicCraft CAD v1.1.1 — Top-Bar DEMO Menu, Context-Aware Toolbar, Full QWERTY/ASCII Keyboard & Detailed Pinouts

### Added
- **Top-Bar `DEMO` Menu Dropdown (`src/ui_panels.rs`, `src/i18n.rs`, `src/simulator.rs`)**:
  - Added a dedicated **`DEMO`** menu button directly next to the **`VIEW` (`VER`)** menu on the top navigation bar across all 5 locales (`EN`, `PT`, `ES`, `FR`, `DE`).
  - Moved the demo circuit launchers out of the `FILE` (`FICHEIRO`) dropdown into the new **`DEMO`** dropdown with 5 ready-to-run demonstration circuits:
    1. **4-Bit Counter + Adder + 7-Seg** (`populate_counter_adder_demo`)
    2. **RAM 16x4 + 7-Seg Decoder** (`populate_ram_decoder_demo`)
    3. **Pong Arcade (CPU + MMIO + Dual VRAM)** (`populate_pong_demo_standalone`)
    4. **QWERTY Keyboard + ASCII Bus + CPU Terminal** (`populate_keyboard_terminal_demo`)
    5. **Full Showcase Workspace (3 Pages)** (`populate_demo_circuit`)
- **Full QWERTY / ASCII Keyboard Peripheral (`FULL_KEYBOARD_COMPONENT`)**:
  - Added a complete QWERTY + ASCII + Arrows keyboard peripheral in library `12 - CPU & Peripherals` alongside the compact Arcade Keyboard.
  - Renders an interactive 5-row keyboard on the canvas (`0–9`, `Q–P`, `A–L`, `Z–M`, `Shift`, `Space`, `Bksp`, `Esc`, `Enter`, and `▲/▼/◄/►` directional keys) with a live ASCII character and hex code readout header.
  - Captures both physical host keyboard input and direct mouse clicks on any keycap on the canvas or in the Inspector panel.
  - Drives an 8-bit ASCII output bus (`ASCII_7`..`ASCII_0`), a `KEY_VALID` strobe pin, and dedicated directional/action output pins (`UP`, `DOWN`, `LEFT`, `RIGHT`, `ENTER`), plus `RST` and `CLR` inputs.
  - Expanded Memory-Mapped I/O (`MemoryBus`) with `MMIO_KEY_LAST` (`0xF006`), `MMIO_KEY_ASCII` (`0xF007`), `MMIO_KEY_DIR` (`0xF008`), and `MMIO_KEY_MODIFIERS` (`0xF009`), also registered as built-in Assembler symbols.
- **Detailed Pinout & Signal Descriptions for CPU, Displays, and Keyboards (`src/library.rs`, `src/canvas_view.rs`, `src/canvas_renderer.rs`, `src/ui_panels.rs`)**:
  - Added `describe_pin()` with localized (`EN` / `PT`) functional role titles and detailed hardware/MMIO signal explanations for every pin of `CPU_COMPONENT`, `DISPLAY_COMPONENT`, `KEYBOARD_COMPONENT`, and `FULL_KEYBOARD_COMPONENT`.
  - Hovering over any pin on the canvas now shows a multi-line tooltip with the pin's logic state, functional title, and detailed hardware description.
  - Added crisp on-package pin labels inside the component bezels on the canvas and a collapsible **`📌 PINOUT & SIGNAL DESCRIPTIONS`** reference section in the Inspector panel.

### Changed
- **Context-Aware Toolbar Buttons Beyond Oscilloscope (`src/ui_panels.rs`)**:
  - Toolbar buttons after **`📈 O-Scope`** now appear dynamically only when a component that uses them is present on the canvas:
    - **`🗐 Memory`** only appears when at least one memory device (`RAM`, `ROM`, or `CPU_COMPONENT`) exists on the canvas.
    - **`🖥 CPU / ASM`** only appears when a `CPU_COMPONENT` exists on the canvas.
- **Removed `🏓 PONG` Toolbar Button**:
  - Removed the standalone `🏓 PONG` button from the top action toolbar (Pong Arcade is now available directly in the top-bar **`DEMO`** menu).

**Full Changelog**: https://github.com/PMTSantos/LogicCraft/compare/v1.1.0...v1.1.1
