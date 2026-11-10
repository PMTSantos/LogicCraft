## LogicCraft CAD v1.1.0 — Programmable Multi-Architecture CPU, MMIO Graphical Displays, Arcade Keyboard & Pong

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
  - **VRAM Graphical Monitor (`DISPLAY_COMPONENT`)**: Add multiple graphical monitors to the same circuit canvas, each independently configurable in the Inspector for **VRAM Base Address** (e.g. `0x8000`, `0x8800`, `0x9000`, `0xA000` to mirror or split displays), **Resolution** (`64x32`, `128x64`, `64x64`), and **Color Palette** (*Classic Monochrome*, *Amber CRT*, *Matrix Green*, and *Game Boy 4-Color*).
  - **Arcade Keyboard Input (`KEYBOARD_COMPONENT`)**: Captures real-time keyboard events (**Player 1**: `W`/`S`, **Player 2**: `Up`/`Down`, **Start/Reset**: `Space`/`Enter`) and maps them to both MMIO registers (`0xF000`–`0xF005`) and digital output pins on the canvas.
- **CPU & Assembly Studio / Real-Time Debugger (`Ctrl+M` / `🖥 CPU / ASM`)**:
  - Embedded Assembly code editor with line numbers, active `PC` line indicator (`▶`), syntax error banner with highlighted offending lines, CPU architecture selector dropdown, and **Assemble & Load**, **Load .asm**, **Save .asm**, and **Pong.asm** actions.
  - Real-time simulation controls: **Run**, **Pause**, **Single Step** (1 instruction), **Reset**, clock mode toggle, and real-time clock speed slider (**1 kHz – 1 MHz**).
  - Live hardware inspection panel featuring a **Register Viewer** (Hexadecimal and Decimal values with modified-register highlighting, `PC`, `SP`, cycle counter, and `Z/N/C/V` flags), **Live Disassembler** showing the current instruction under `PC` and upcoming instructions, and **Memory Hex Dump Viewer** with one-click jumps to `ROM 0x0000`, `RAM 0x4000`, `VRAM 0x8000`, and `MMIO 0xF000`.
- **2-Player Local Pong Arcade Game in Assembly (`examples/pong.asm`)**:
  - Complete playable 2-player local Pong demonstration pre-loaded on the **Pong Arcade (CPU + MMIO)** circuit tab (`🏓 PONG` toolbar button) featuring `W`/`S` and `Up`/`Down` paddle controls, 2D ball velocity $(dx, dy)$, top/bottom wall reflection ($dy = -dy$), paddle collision reflection ($dx = -dx$), goal detection with score tracking and center ball reset, and 60 FPS VSync-paced VRAM rendering.

**Full Changelog**: https://github.com/PMTSantos/LogicCraft/compare/v1.0.6...v1.1.0
