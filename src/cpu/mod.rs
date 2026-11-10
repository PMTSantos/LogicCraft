#![allow(dead_code)]
//! Modular Multi-Architecture CPU, Assembler, MMIO & Peripherals (`src/cpu/mod.rs`).
//!
//! Includes:
//! - [`CpuCore`]: Trait abstraction for pluggable CPU architectures (`step()`, `reset()`,
//!   `load_program()`, `registers()`, `disassemble_current()`, `get_state()`).
//! - [`Rv32iCore`]: 32-bit RISC-V (`RV32I` + `RV32M`) general-purpose processor core.
//! - [`Lc16Core`]: 16-bit LogicCraft Custom RISC/CISC processor core.
//! - [`Min8Core`]: 8-bit Minimal / CHIP-8 inspired educational processor core.
//! - [`CpuComponent`], [`DisplayComponent`], and [`KeyboardInputComponent`]: Canvas hardware components.

pub mod assembler;
pub mod bus;
pub mod lc16;
pub mod min8;
pub mod rv32i;

use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

use crate::cpu::assembler::{assemble, AssembledProgram, AssemblyError};
use crate::cpu::bus::{KeyboardState, MemoryBus, VRAM_BASE};
use crate::cpu::lc16::Lc16Core;
use crate::cpu::min8::Min8Core;
use crate::cpu::rv32i::Rv32iCore;

pub const BUILTIN_PONG_ASM: &str = include_str!("../../examples/pong.asm");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CpuArch {
    Rv32i,
    Lc16,
    Min8,
}

impl Default for CpuArch {
    fn default() -> Self {
        Self::Rv32i
    }
}

impl CpuArch {
    pub const ALL: [CpuArch; 3] = [CpuArch::Rv32i, CpuArch::Lc16, CpuArch::Min8];

    pub fn code_str(self) -> &'static str {
        match self {
            CpuArch::Rv32i => "RV32I",
            CpuArch::Lc16 => "LC16",
            CpuArch::Min8 => "MIN8",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            CpuArch::Rv32i => "RISC-V RV32I (32-bit Main Core)",
            CpuArch::Lc16 => "LC-16 Custom (16-bit Main Core)",
            CpuArch::Min8 => "MIN-8 Minimal (8-bit Educational Core)",
        }
    }

    pub fn from_str_code(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "LC16" | "CUSTOM16" | "16BIT" => CpuArch::Lc16,
            "MIN8" | "CHIP8" | "8BIT" => CpuArch::Min8,
            _ => CpuArch::Rv32i,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CpuFlags {
    pub z: bool, // Zero
    pub n: bool, // Negative
    pub c: bool, // Carry
    pub v: bool, // Overflow
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuState {
    pub arch: CpuArch,
    pub pc: u32,
    pub sp: u32,
    pub cycles: u64,
    pub halted: bool,
    pub flags: CpuFlags,
}

#[derive(Debug, Clone)]
pub struct RegisterValue {
    pub name: String,
    pub short_name: String,
    pub value: u32,
    pub changed: bool,
    pub bits: u8,
}

/// Pluggable CPU Core trait for multi-architecture simulation.
pub trait CpuCore: Send + Sync {
    fn arch(&self) -> CpuArch;
    fn step(&mut self, bus: &mut MemoryBus) -> bool;
    fn reset(&mut self);
    fn load_program(&mut self, program: &[u8], bus: &mut MemoryBus);
    fn registers(&self) -> Vec<RegisterValue>;
    fn disassemble_current(&self, bus: &MemoryBus) -> String;
    fn disassemble_at(&self, addr: u32, bus: &MemoryBus) -> (String, u32);
    fn get_state(&self) -> CpuState;
    fn set_pc(&mut self, pc: u32);
}

// ============================================================================
// DisplayComponent — Configurable Memory-Mapped Graphical Monitor
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayResolution {
    Res64x32,
    Res128x64,
    Res64x64,
}

impl DisplayResolution {
    pub const ALL: [DisplayResolution; 3] = [
        DisplayResolution::Res64x32,
        DisplayResolution::Res128x64,
        DisplayResolution::Res64x64,
    ];

    pub fn dimensions(self) -> (usize, usize) {
        match self {
            DisplayResolution::Res64x32 => (64, 32),
            DisplayResolution::Res128x64 => (128, 64),
            DisplayResolution::Res64x64 => (64, 64),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            DisplayResolution::Res64x32 => "64x32",
            DisplayResolution::Res128x64 => "128x64",
            DisplayResolution::Res64x64 => "64x64",
        }
    }

    pub fn from_str_label(s: &str) -> Self {
        match s.trim() {
            "128x64" => DisplayResolution::Res128x64,
            "64x64" => DisplayResolution::Res64x64,
            _ => DisplayResolution::Res64x32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayPalette {
    Monochrome,
    Amber,
    MatrixGreen,
    GameBoy,
}

impl DisplayPalette {
    pub const ALL: [DisplayPalette; 4] = [
        DisplayPalette::Monochrome,
        DisplayPalette::Amber,
        DisplayPalette::MatrixGreen,
        DisplayPalette::GameBoy,
    ];

    pub fn code_str(self) -> &'static str {
        match self {
            DisplayPalette::Monochrome => "MONOCHROME",
            DisplayPalette::Amber => "AMBER",
            DisplayPalette::MatrixGreen => "MATRIX_GREEN",
            DisplayPalette::GameBoy => "GAMEBOY",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            DisplayPalette::Monochrome => "Classic Monochrome",
            DisplayPalette::Amber => "Amber CRT",
            DisplayPalette::MatrixGreen => "Matrix Green",
            DisplayPalette::GameBoy => "Game Boy (4-Color)",
        }
    }

    pub fn from_str_code(s: &str) -> Self {
        match s.trim().to_ascii_uppercase().as_str() {
            "AMBER" => DisplayPalette::Amber,
            "MATRIX_GREEN" | "MATRIX" | "GREEN" => DisplayPalette::MatrixGreen,
            "GAMEBOY" | "GB" | "4COLOR" => DisplayPalette::GameBoy,
            _ => DisplayPalette::Monochrome,
        }
    }

    /// Maps a VRAM pixel byte (`0` = background, `1..=255` = foreground shade) to `Color32`.
    pub fn pixel_color(self, val: u8) -> Color32 {
        match self {
            DisplayPalette::Monochrome => match val {
                0 => Color32::from_rgb(12, 15, 20),
                1 => Color32::from_rgb(110, 125, 145),
                2 => Color32::from_rgb(185, 200, 220),
                _ => Color32::from_rgb(245, 250, 255),
            },
            DisplayPalette::Amber => match val {
                0 => Color32::from_rgb(22, 14, 6),
                1 => Color32::from_rgb(130, 72, 12),
                2 => Color32::from_rgb(215, 130, 24),
                _ => Color32::from_rgb(255, 184, 48),
            },
            DisplayPalette::MatrixGreen => match val {
                0 => Color32::from_rgb(8, 18, 12),
                1 => Color32::from_rgb(24, 115, 55),
                2 => Color32::from_rgb(45, 195, 95),
                _ => Color32::from_rgb(110, 255, 145),
            },
            DisplayPalette::GameBoy => match val {
                0 => Color32::from_rgb(15, 56, 15),    // Darkest Olive
                1 => Color32::from_rgb(48, 98, 48),    // Dark Olive Green
                2 => Color32::from_rgb(139, 172, 15),  // Light Olive Green
                _ => Color32::from_rgb(155, 188, 15),  // Brightest DMG Green
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayComponent {
    pub vram_base: u16,
    pub resolution: DisplayResolution,
    pub palette: DisplayPalette,
}

impl Default for DisplayComponent {
    fn default() -> Self {
        Self {
            vram_base: VRAM_BASE,
            resolution: DisplayResolution::Res64x32,
            palette: DisplayPalette::MatrixGreen,
        }
    }
}

impl DisplayComponent {
    pub fn from_lparams(lparams: &std::collections::HashMap<String, String>) -> Self {
        let vram_base = lparams
            .get("VRAM_BASE")
            .and_then(|s| {
                let t = s.trim();
                if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
                    u16::from_str_radix(hex, 16).ok()
                } else {
                    t.parse::<u16>().ok()
                }
            })
            .unwrap_or(VRAM_BASE);
        let resolution = lparams
            .get("RESOLUTION")
            .map(|s| DisplayResolution::from_str_label(s))
            .unwrap_or(DisplayResolution::Res64x32);
        let palette = lparams
            .get("PALETTE")
            .map(|s| DisplayPalette::from_str_code(s))
            .unwrap_or(DisplayPalette::MatrixGreen);
        Self {
            vram_base,
            resolution,
            palette,
        }
    }
}

// ============================================================================
// KeyboardInputComponent — Hardware & MMIO Keyboard Controller
// ============================================================================

#[derive(Debug, Clone, Default)]
pub struct KeyboardInputComponent {
    pub state: KeyboardState,
    /// Optional manual mouse-click overrides on the canvas component buttons
    pub canvas_override: KeyboardState,
}

impl KeyboardInputComponent {
    pub fn effective_state(&self) -> KeyboardState {
        KeyboardState {
            p1_up: self.state.p1_up || self.canvas_override.p1_up,
            p1_down: self.state.p1_down || self.canvas_override.p1_down,
            p2_up: self.state.p2_up || self.canvas_override.p2_up,
            p2_down: self.state.p2_down || self.canvas_override.p2_down,
            start: self.state.start || self.canvas_override.start,
            last_key: if self.state.last_key != 0 {
                self.state.last_key
            } else {
                self.canvas_override.last_key
            },
        }
    }
}

// ============================================================================
// CpuComponent — Pluggable Processor Subsystem & Debugger State
// ============================================================================

pub struct CpuComponent {
    pub core: Box<dyn CpuCore>,
    pub bus: MemoryBus,
    pub keyboard: KeyboardInputComponent,
    pub running: bool,
    pub use_internal_clock: bool,
    pub clock_hz: u32, // 1_000 Hz (1 kHz) to 1_000_000 Hz (1 MHz)
    pub source_code: String,
    pub last_assembled: Option<AssembledProgram>,
    pub assembly_errors: Vec<AssemblyError>,
    pub status_log: String,
    pub hex_dump_base: u16,
    cycle_accumulator: f64,
}

impl Default for CpuComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for CpuComponent {
    fn clone(&self) -> Self {
        let arch = self.core.arch();
        let mut new_core: Box<dyn CpuCore> = match arch {
            CpuArch::Rv32i => Box::new(Rv32iCore::new()),
            CpuArch::Lc16 => Box::new(Lc16Core::new()),
            CpuArch::Min8 => Box::new(Min8Core::new()),
        };
        new_core.set_pc(self.core.get_state().pc);
        Self {
            core: new_core,
            bus: self.bus.clone(),
            keyboard: self.keyboard.clone(),
            running: self.running,
            use_internal_clock: self.use_internal_clock,
            clock_hz: self.clock_hz,
            source_code: self.source_code.clone(),
            last_assembled: self.last_assembled.clone(),
            assembly_errors: self.assembly_errors.clone(),
            status_log: self.status_log.clone(),
            hex_dump_base: self.hex_dump_base,
            cycle_accumulator: 0.0,
        }
    }
}

impl CpuComponent {
    pub fn new() -> Self {
        let mut comp = Self {
            core: Box::new(Rv32iCore::new()),
            bus: MemoryBus::new(),
            keyboard: KeyboardInputComponent::default(),
            running: true,
            use_internal_clock: true,
            clock_hz: 150_000, // 150 kHz default for smooth 60 FPS Pong
            source_code: BUILTIN_PONG_ASM.to_string(),
            last_assembled: None,
            assembly_errors: Vec::new(),
            status_log: String::new(),
            hex_dump_base: VRAM_BASE,
            cycle_accumulator: 0.0,
        };
        let _ = comp.assemble_and_load();
        // Run initial boot steps so the initial Pong court/paddles/ball are visible on the screen immediately
        comp.bus.notify_frame_tick();
        for _ in 0..2500 {
            if !comp.core.step(&mut comp.bus) {
                break;
            }
            if comp.bus.last_bus_addr == bus::MMIO_VSYNC {
                break;
            }
        }
        comp
    }

    pub fn arch(&self) -> CpuArch {
        self.core.arch()
    }

    pub fn load_pong_demo(&mut self) {
        self.core = Box::new(Rv32iCore::new());
        self.source_code = BUILTIN_PONG_ASM.to_string();
        let _ = self.assemble_and_load();
        self.bus.notify_frame_tick();
        for _ in 0..2500 {
            if !self.core.step(&mut self.bus) {
                break;
            }
            if self.bus.last_bus_addr == bus::MMIO_VSYNC {
                break;
            }
        }
    }

    pub fn set_arch(&mut self, arch: CpuArch) {
        if self.core.arch() == arch {
            return;
        }
        self.core = match arch {
            CpuArch::Rv32i => Box::new(Rv32iCore::new()),
            CpuArch::Lc16 => Box::new(Lc16Core::new()),
            CpuArch::Min8 => Box::new(Min8Core::new()),
        };
        let _ = self.assemble_and_load();
    }

    pub fn assemble_and_load(&mut self) -> Result<usize, Vec<AssemblyError>> {
        let arch = self.core.arch();
        match assemble(&self.source_code, arch) {
            Ok(prog) => {
                let count = prog.instruction_count;
                let byte_len = prog.bytes.len();
                self.bus.clear_all();
                self.core.load_program(&prog.bytes, &mut self.bus);
                self.core.set_pc(prog.entry_pc);
                self.assembly_errors.clear();
                self.last_assembled = Some(prog);
                self.status_log = format!(
                    "[OK] {} assembled: {} instructions ({} bytes)",
                    arch.code_str(),
                    count,
                    byte_len
                );
                Ok(count)
            }
            Err(errs) => {
                self.assembly_errors = errs.clone();
                if let Some(first) = errs.first() {
                    self.status_log = format!("Syntax error at line {}: {}", first.line, first.message);
                }
                Err(errs)
            }
        }
    }

    pub fn reset(&mut self) {
        self.bus.reset();
        self.core.reset();
        if let Some(ref prog) = self.last_assembled {
            self.core.set_pc(prog.entry_pc);
        }
        self.cycle_accumulator = 0.0;
        self.status_log = "CPU Reset complete.".to_string();
    }

    pub fn single_step(&mut self) -> bool {
        self.bus.update_keys(self.keyboard.effective_state());
        self.bus.vsync_ready = true;
        self.core.step(&mut self.bus)
    }

    /// Advances the CPU by a time slice `dt_secs` without blocking the UI thread FPS.
    pub fn tick_frame(&mut self, dt_secs: f64) {
        self.bus.update_keys(self.keyboard.effective_state());
        if !self.running || !self.use_internal_clock {
            return;
        }
        let clamped_dt = dt_secs.clamp(0.001, 0.05);
        self.bus.notify_frame_tick();

        let hz = (self.clock_hz.clamp(1_000, 1_000_000)) as f64;
        self.cycle_accumulator += hz * clamped_dt;
        let steps_to_run = (self.cycle_accumulator as usize).min(25_000);
        self.cycle_accumulator -= steps_to_run as f64;

        let mut vsync_waits = 0u32;
        for _ in 0..steps_to_run {
            if !self.core.step(&mut self.bus) {
                self.running = false;
                break;
            }
            // If CPU is spinning on MMIO_VSYNC after completing its frame render, yield early
            // so high clock speeds (e.g. 1 MHz) never waste CPU time or cause UI lag!
            if !self.bus.vsync_ready && self.bus.last_bus_addr == bus::MMIO_VSYNC {
                vsync_waits += 1;
                if vsync_waits > 8 {
                    break;
                }
            }
        }
    }

    /// Returns a list of `(address, hex_bytes, disassembly, is_current_pc, source_line)` around `pc`.
    pub fn disassemble_window(&self, count: usize) -> Vec<(u32, String, String, bool, Option<usize>)> {
        let pc = self.core.get_state().pc;
        let start_addr = pc.saturating_sub(8) & !3;
        let mut addr = start_addr;
        let mut out = Vec::with_capacity(count);
        for _ in 0..count {
            let (text, step) = self.core.disassemble_at(addr, &self.bus);
            let w = self.bus.peek32(addr);
            let hex = format!("{w:08X}");
            let src_line = self
                .last_assembled
                .as_ref()
                .and_then(|p| p.addr_to_line.get(&addr).copied());
            out.push((addr, hex, text, addr == pc, src_line));
            addr = addr.wrapping_add(step.max(4)) & 0xFFFF;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::bus::*;

    #[test]
    fn test_assembler_directives_labels_and_constants() {
        let asm = r#"
            .equ SECRET_VAL, 0x42
            .org 0x0000
        start:
            li t0, VRAM_BASE
            li t1, SECRET_VAL
            sb t1, 0(t0)
            j done
            nop
        done:
            halt
            .org 0x0100
            .byte 0x11, 0x22, 0x33
            .word 0xABCD
        "#;

        for arch in [CpuArch::Rv32i, CpuArch::Lc16, CpuArch::Min8] {
            let prog = assemble(asm, arch).expect("Should assemble cleanly");
            assert!(prog.labels.contains_key("start"));
            assert!(prog.labels.contains_key("done"));
            assert_eq!(prog.bytes[0x0100], 0x11);
            assert_eq!(prog.bytes[0x0101], 0x22);
            assert_eq!(prog.bytes[0x0102], 0x33);
            assert_eq!(prog.bytes[0x0103], 0xCD);
            assert_eq!(prog.bytes[0x0104], 0xAB);
        }
    }

    #[test]
    fn test_assembler_syntax_error_reporting_with_line_numbers() {
        let bad_asm = "li t0, 10\ninvalid_opcode_xyz t0, t1\naddi t0, t0, 1\n";
        let err = assemble(bad_asm, CpuArch::Rv32i).unwrap_err();
        assert_eq!(err.len(), 1);
        assert_eq!(err[0].line, 2);
        assert!(err[0].line_text.contains("invalid_opcode_xyz"));
    }

    #[test]
    fn test_rv32i_execution_and_mmio_vram_write() {
        let asm = r#"
            li t0, 0x8000
            li t1, 3
            sb t1, 0(t0)
            sb t1, 65(t0)
            lbu t2, 0(t0)
            addi t3, t2, 7
            halt
        "#;
        let prog = assemble(asm, CpuArch::Rv32i).unwrap();
        let mut bus = MemoryBus::new();
        let mut core = Rv32iCore::new();
        core.load_program(&prog.bytes, &mut bus);

        while core.step(&mut bus) {}

        assert!(core.get_state().halted);
        assert_eq!(bus.peek8(0x8000), 3);
        assert_eq!(bus.peek8(0x8000 + 65), 3);
        assert_eq!(core.read_reg(7), 3);  // t2 = 3
        assert_eq!(core.read_reg(28), 10); // t3 = 10
    }

    #[test]
    fn test_lc16_and_min8_execution_and_mmio_keys() {
        let asm = r#"
            li t6, 0xF000
            lbu t0, 0(t6)
            li t5, 0x8000
            sb t0, 0(t5)
            halt
        "#;
        for arch in [CpuArch::Lc16, CpuArch::Min8] {
            let prog = assemble(asm, arch).unwrap();
            let mut bus = MemoryBus::new();
            bus.update_keys(KeyboardState {
                p1_up: true,
                p1_down: false,
                p2_up: false,
                p2_down: true,
                start: false,
                last_key: b'W',
            });
            let mut core: Box<dyn CpuCore> = match arch {
                CpuArch::Lc16 => Box::new(Lc16Core::new()),
                CpuArch::Min8 => Box::new(Min8Core::new()),
                _ => unreachable!(),
            };
            core.load_program(&prog.bytes, &mut bus);
            while core.step(&mut bus) {}
            assert_eq!(bus.peek8(0x8000), KEY_BIT_P1_UP | KEY_BIT_P2_DOWN);
        }
    }

    #[test]
    fn test_pong_asm_assembles_and_renders_to_vram_on_all_cores() {
        for arch in [CpuArch::Rv32i, CpuArch::Lc16, CpuArch::Min8] {
            let prog = assemble(BUILTIN_PONG_ASM, arch)
                .unwrap_or_else(|e| panic!("pong.asm failed on {:?}: {:?}", arch, e));
            let mut bus = MemoryBus::new();
            let mut core: Box<dyn CpuCore> = match arch {
                CpuArch::Rv32i => Box::new(Rv32iCore::new()),
                CpuArch::Lc16 => Box::new(Lc16Core::new()),
                CpuArch::Min8 => Box::new(Min8Core::new()),
            };
            core.load_program(&prog.bytes, &mut bus);

            // Simulate Player 1 pressing W and Player 2 pressing Down across 3 frames
            bus.update_keys(KeyboardState {
                p1_up: true,
                p1_down: false,
                p2_up: false,
                p2_down: true,
                start: false,
                last_key: b'W',
            });
            for _frame in 0..3 {
                bus.notify_frame_tick();
                for _ in 0..2500 {
                    core.step(&mut bus);
                    if !bus.vsync_ready && bus.last_bus_addr == MMIO_VSYNC {
                        break;
                    }
                }
            }

            // Verify VRAM (0x8000..0x87FF) has active rendered pixels (court borders, net, paddles, ball)
            let lit_pixels = (0x8000..0x8800u32)
                .filter(|&addr| bus.peek8(addr) != 0)
                .count();
            assert!(
                lit_pixels > 100,
                "Expected >100 lit pixels in VRAM for {:?}, got {}",
                arch,
                lit_pixels
            );
        }
    }
}
