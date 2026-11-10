//! MIN-8 Educational 8-Bit Minimal / CHIP-8 Inspired Processor Core (`src/cpu/min8.rs`).
//!
//! Designed for didactic clarity and rapid prototyping:
//! - 8 primary 8-bit registers `V0`..`V7` (aliased as `R0`..`R7` or `A, B, C, D, X, Y, H, L`)
//!   plus extended scratch/pointer registers so MMIO (`0xF000`) and VRAM (`0x8000`) programs run seamlessly.
//! - 16-bit Index / Address register `I`
//! - 16-bit Program Counter (`PC`) and Stack Pointer (`SP`)
//! - Supports 8-bit arithmetic, direct MMIO/VRAM loads & stores, `CLS`, `SETI`, and `PSET Vx, Vy, color`.

use crate::cpu::bus::{MemoryBus, DEFAULT_SP, VRAM_BASE};
use crate::cpu::lc16::*;
use crate::cpu::{CpuArch, CpuCore, CpuFlags, CpuState, RegisterValue};

pub const MIN8_OP_SETI: u8 = 0x30;
pub const MIN8_OP_ADDI_I: u8 = 0x31;
pub const MIN8_OP_PSET: u8 = 0x32;
pub const MIN8_OP_CLS: u8 = 0x33;

#[derive(Debug, Clone)]
pub struct Min8Core {
    pub regs: [u16; 16],
    pub prev_regs: [u16; 16],
    pub reg_i: u16,
    pub prev_i: u16,
    pub pc: u16,
    pub sp: u16,
    pub cycles: u64,
    pub halted: bool,
    pub flags: CpuFlags,
}

impl Default for Min8Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Min8Core {
    pub fn new() -> Self {
        Self {
            regs: [0u16; 16],
            prev_regs: [0u16; 16],
            reg_i: VRAM_BASE,
            prev_i: VRAM_BASE,
            pc: 0,
            sp: (DEFAULT_SP & 0xFFFF) as u16,
            cycles: 0,
            halted: false,
            flags: CpuFlags::default(),
        }
    }

    #[inline]
    fn update_zn(&mut self, val: u16) {
        self.flags.z = val == 0;
        self.flags.n = (val as i16) < 0;
    }

    pub fn decode_and_disassemble(word: u32) -> String {
        let op = (word & 0xFF) as u8;
        let rd = ((word >> 12) & 0x0F) as usize;
        let rs1 = ((word >> 8) & 0x0F) as usize;
        let imm = ((word >> 16) & 0xFFFF) as u16;
        let rs2 = (imm & 0x0F) as usize;
        let simm = imm as i16;

        match op {
            LC16_OP_NOP => "NOP".to_string(),
            LC16_OP_LDI => format!("LDI V{rd}, 0x{imm:04X} ({simm})"),
            LC16_OP_MOV => format!("MOV V{rd}, V{rs1}"),
            LC16_OP_ADD => format!("ADD V{rd}, V{rs1}, V{rs2}"),
            LC16_OP_ADDI => format!("ADDI V{rd}, V{rs1}, {simm}"),
            LC16_OP_SUB => format!("SUB V{rd}, V{rs1}, V{rs2}"),
            LC16_OP_SUBI => format!("SUBI V{rd}, V{rs1}, {simm}"),
            LC16_OP_AND => format!("AND V{rd}, V{rs1}, V{rs2}"),
            LC16_OP_ANDI => format!("ANDI V{rd}, V{rs1}, 0x{imm:02X}"),
            LC16_OP_OR => format!("OR V{rd}, V{rs1}, V{rs2}"),
            LC16_OP_ORI => format!("ORI V{rd}, V{rs1}, 0x{imm:02X}"),
            LC16_OP_XOR => format!("XOR V{rd}, V{rs1}, V{rs2}"),
            LC16_OP_XORI => format!("XORI V{rd}, V{rs1}, 0x{imm:02X}"),
            LC16_OP_SHL => format!("SHL V{rd}, V{rs1}, {}", imm & 15),
            LC16_OP_SHR => format!("SHR V{rd}, V{rs1}, {}", imm & 15),
            LC16_OP_CMP => format!("CMP V{rd}, V{rs1}"),
            LC16_OP_CMPI => format!("CMPI V{rd}, {simm}"),
            LC16_OP_LDB | LC16_OP_LDW => format!("LD V{rd}, [V{rs1} + 0x{imm:04X}]"),
            LC16_OP_STB | LC16_OP_STW => format!("ST V{rd}, [V{rs1} + 0x{imm:04X}]"),
            MIN8_OP_SETI => format!("SETI I, 0x{imm:04X}"),
            MIN8_OP_ADDI_I => format!("ADDI I, V{rd}"),
            MIN8_OP_PSET => format!("PSET V{rd}, V{rs1}, {}", imm & 0xFF),
            MIN8_OP_CLS => "CLS".to_string(),
            LC16_OP_JMP => format!("JMP 0x{imm:04X}"),
            LC16_OP_JEQ => format!("JZ 0x{imm:04X}"),
            LC16_OP_JNE => format!("JNZ 0x{imm:04X}"),
            LC16_OP_JLT => format!("JLT 0x{imm:04X}"),
            LC16_OP_JGE => format!("JGE 0x{imm:04X}"),
            LC16_OP_BEQ => format!("BEQ V{rd}, V{rs1}, 0x{imm:04X}"),
            LC16_OP_BNE => format!("BNE V{rd}, V{rs1}, 0x{imm:04X}"),
            LC16_OP_BLT => format!("BLT V{rd}, V{rs1}, 0x{imm:04X}"),
            LC16_OP_BGE => format!("BGE V{rd}, V{rs1}, 0x{imm:04X}"),
            LC16_OP_CALL => format!("CALL 0x{imm:04X}"),
            LC16_OP_RET => "RET".to_string(),
            LC16_OP_HALT => "HALT".to_string(),
            _ => format!(".BYTE 0x{op:02X}"),
        }
    }
}

impl CpuCore for Min8Core {
    fn arch(&self) -> CpuArch {
        CpuArch::Min8
    }

    fn reset(&mut self) {
        self.regs = [0u16; 16];
        self.prev_regs = [0u16; 16];
        self.reg_i = VRAM_BASE;
        self.prev_i = VRAM_BASE;
        self.pc = 0;
        self.sp = (DEFAULT_SP & 0xFFFF) as u16;
        self.cycles = 0;
        self.halted = false;
        self.flags = CpuFlags::default();
    }

    fn load_program(&mut self, program: &[u8], bus: &mut MemoryBus) {
        bus.load_program(program);
        self.reset();
    }

    fn step(&mut self, bus: &mut MemoryBus) -> bool {
        if self.halted {
            return false;
        }
        self.prev_regs = self.regs;
        self.prev_i = self.reg_i;
        let word = bus.read32(self.pc as u32);
        self.cycles = self.cycles.wrapping_add(1);

        let op = (word & 0xFF) as u8;
        let rd = ((word >> 12) & 0x0F) as usize;
        let rs1 = ((word >> 8) & 0x0F) as usize;
        let imm = ((word >> 16) & 0xFFFF) as u16;
        let rs2 = (imm & 0x0F) as usize;
        let mut next_pc = self.pc.wrapping_add(4);

        match op {
            LC16_OP_NOP => {}
            LC16_OP_LDI => {
                self.regs[rd] = imm;
                if imm >= 0x4000 {
                    self.reg_i = imm;
                }
                self.update_zn(imm);
            }
            LC16_OP_MOV => {
                let v = self.regs[rs1];
                self.regs[rd] = v;
                self.update_zn(v);
            }
            LC16_OP_ADD => {
                let (res, carry) = self.regs[rs1].overflowing_add(self.regs[rs2]);
                self.regs[rd] = res;
                self.flags.c = carry;
                self.update_zn(res);
            }
            LC16_OP_ADDI => {
                let (res, carry) = self.regs[rs1].overflowing_add(imm);
                self.regs[rd] = res;
                self.flags.c = carry;
                self.update_zn(res);
            }
            LC16_OP_SUB => {
                let (res, borrow) = self.regs[rs1].overflowing_sub(self.regs[rs2]);
                self.regs[rd] = res;
                self.flags.c = !borrow;
                self.update_zn(res);
            }
            LC16_OP_SUBI => {
                let (res, borrow) = self.regs[rs1].overflowing_sub(imm);
                self.regs[rd] = res;
                self.flags.c = !borrow;
                self.update_zn(res);
            }
            LC16_OP_AND => {
                let res = self.regs[rs1] & self.regs[rs2];
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_ANDI => {
                let res = self.regs[rs1] & imm;
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_OR => {
                let res = self.regs[rs1] | self.regs[rs2];
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_ORI => {
                let res = self.regs[rs1] | imm;
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_XOR => {
                let res = self.regs[rs1] ^ self.regs[rs2];
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_XORI => {
                let res = self.regs[rs1] ^ imm;
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_SHL => {
                let res = self.regs[rs1].wrapping_shl((imm & 15) as u32);
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_SHR => {
                let res = self.regs[rs1].wrapping_shr((imm & 15) as u32);
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_CMP => {
                let a = self.regs[rd];
                let b = self.regs[rs1];
                self.flags.z = a == b;
                self.flags.n = (a as i16) < (b as i16);
                self.flags.c = a >= b;
            }
            LC16_OP_CMPI => {
                let a = self.regs[rd];
                let b = imm;
                self.flags.z = a == b;
                self.flags.n = (a as i16) < (b as i16);
                self.flags.c = a >= b;
            }
            LC16_OP_LDB | LC16_OP_LDW => {
                let base = if rs1 == 0 && imm >= 0x4000 {
                    0
                } else {
                    self.regs[rs1]
                };
                let addr = base.wrapping_add(imm) as u32;
                let val = bus.read8(addr) as u16;
                self.regs[rd] = val;
                self.update_zn(val);
            }
            LC16_OP_STB | LC16_OP_STW => {
                let base = if rs1 == 0 && imm >= 0x4000 {
                    0
                } else {
                    self.regs[rs1]
                };
                let addr = base.wrapping_add(imm) as u32;
                bus.write8(addr, (self.regs[rd] & 0xFF) as u8);
            }
            MIN8_OP_SETI => {
                self.reg_i = imm;
            }
            MIN8_OP_ADDI_I => {
                self.reg_i = self.reg_i.wrapping_add(self.regs[rd]);
            }
            MIN8_OP_PSET => {
                let x = self.regs[rd] & 63;
                let y = self.regs[rs1] & 31;
                let col = if (imm & 0xFF) == 0 {
                    3
                } else {
                    (imm & 0xFF) as u8
                };
                let addr = VRAM_BASE.wrapping_add(y * 64 + x) as u32;
                bus.write8(addr, col);
            }
            MIN8_OP_CLS => {
                bus.write8(0xF030, 0);
            }
            LC16_OP_JMP => {
                next_pc = imm;
            }
            LC16_OP_JEQ => {
                if self.flags.z {
                    next_pc = imm;
                }
            }
            LC16_OP_JNE => {
                if !self.flags.z {
                    next_pc = imm;
                }
            }
            LC16_OP_JLT => {
                if self.flags.n {
                    next_pc = imm;
                }
            }
            LC16_OP_JGE => {
                if !self.flags.n {
                    next_pc = imm;
                }
            }
            LC16_OP_BEQ => {
                if self.regs[rd] == self.regs[rs1] {
                    next_pc = imm;
                }
            }
            LC16_OP_BNE => {
                if self.regs[rd] != self.regs[rs1] {
                    next_pc = imm;
                }
            }
            LC16_OP_BLT => {
                if (self.regs[rd] as i16) < (self.regs[rs1] as i16) {
                    next_pc = imm;
                }
            }
            LC16_OP_BGE => {
                if (self.regs[rd] as i16) >= (self.regs[rs1] as i16) {
                    next_pc = imm;
                }
            }
            LC16_OP_CALL => {
                self.sp = self.sp.wrapping_sub(2);
                bus.write16(self.sp as u32, next_pc);
                next_pc = imm;
            }
            LC16_OP_RET => {
                next_pc = bus.read16(self.sp as u32);
                self.sp = self.sp.wrapping_add(2);
            }
            LC16_OP_HALT => {
                self.halted = true;
                return false;
            }
            _ => {}
        }

        self.pc = next_pc;
        true
    }

    fn registers(&self) -> Vec<RegisterValue> {
        let names = [
            "V0 (A)", "V1 (B)", "V2 (C)", "V3 (D)", "V4 (X)", "V5 (Y)", "V6 (H)", "V7 (L)",
        ];
        let short = ["V0", "V1", "V2", "V3", "V4", "V5", "V6", "V7"];
        let mut out: Vec<RegisterValue> = (0..8)
            .map(|i| RegisterValue {
                name: names[i].to_string(),
                short_name: short[i].to_string(),
                value: (self.regs[i] & 0xFF) as u32,
                changed: self.regs[i] != self.prev_regs[i],
                bits: 8,
            })
            .collect();
        out.push(RegisterValue {
            name: "I  (IDX)".to_string(),
            short_name: "I".to_string(),
            value: self.reg_i as u32,
            changed: self.reg_i != self.prev_i,
            bits: 16,
        });
        out
    }

    fn disassemble_current(&self, bus: &MemoryBus) -> String {
        let word = bus.peek32(self.pc as u32);
        Self::decode_and_disassemble(word)
    }

    fn disassemble_at(&self, addr: u32, bus: &MemoryBus) -> (String, u32) {
        let word = bus.peek32(addr);
        (Self::decode_and_disassemble(word), 4)
    }

    fn get_state(&self) -> CpuState {
        CpuState {
            arch: CpuArch::Min8,
            pc: self.pc as u32,
            sp: self.sp as u32,
            cycles: self.cycles,
            halted: self.halted,
            flags: self.flags,
        }
    }

    fn set_pc(&mut self, pc: u32) {
        self.pc = (pc & 0xFFFF) as u16;
    }
}
