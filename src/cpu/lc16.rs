//! LogicCraft 16-Bit Custom CPU Core (`src/cpu/lc16.rs`).
//!
//! 16-bit general-purpose architecture (`R0`..`R15`) with a clean 32-bit fixed instruction word:
//! `[opcode: u8, (rd << 4) | rs1: u8, imm16_lo: u8, imm16_hi: u8]`
//! Every instruction has a full 16-bit immediate/address field, making MMIO (`0xF000`) and
//! VRAM (`0x8000`) addressing a single instruction.

use crate::cpu::bus::{MemoryBus, DEFAULT_SP};
use crate::cpu::{CpuArch, CpuCore, CpuFlags, CpuState, RegisterValue};

pub const LC16_OP_NOP: u8 = 0x00;
pub const LC16_OP_LDI: u8 = 0x01;
pub const LC16_OP_MOV: u8 = 0x02;
pub const LC16_OP_ADD: u8 = 0x03;
pub const LC16_OP_ADDI: u8 = 0x04;
pub const LC16_OP_SUB: u8 = 0x05;
pub const LC16_OP_SUBI: u8 = 0x06;
pub const LC16_OP_AND: u8 = 0x07;
pub const LC16_OP_ANDI: u8 = 0x08;
pub const LC16_OP_OR: u8 = 0x09;
pub const LC16_OP_ORI: u8 = 0x0A;
pub const LC16_OP_XOR: u8 = 0x0B;
pub const LC16_OP_XORI: u8 = 0x0C;
pub const LC16_OP_SHL: u8 = 0x0D;
pub const LC16_OP_SHR: u8 = 0x0E;
pub const LC16_OP_CMP: u8 = 0x0F;
pub const LC16_OP_CMPI: u8 = 0x10;
pub const LC16_OP_LDB: u8 = 0x11;
pub const LC16_OP_STB: u8 = 0x12;
pub const LC16_OP_LDW: u8 = 0x13;
pub const LC16_OP_STW: u8 = 0x14;
pub const LC16_OP_JMP: u8 = 0x15;
pub const LC16_OP_JEQ: u8 = 0x16;
pub const LC16_OP_JNE: u8 = 0x17;
pub const LC16_OP_JLT: u8 = 0x18;
pub const LC16_OP_JGE: u8 = 0x19;
pub const LC16_OP_JGT: u8 = 0x1A;
pub const LC16_OP_JLE: u8 = 0x1B;
pub const LC16_OP_CALL: u8 = 0x1C;
pub const LC16_OP_RET: u8 = 0x1D;
pub const LC16_OP_PUSH: u8 = 0x1E;
pub const LC16_OP_POP: u8 = 0x1F;
pub const LC16_OP_NEG: u8 = 0x20;
pub const LC16_OP_NOT: u8 = 0x21;
pub const LC16_OP_MUL: u8 = 0x22;
pub const LC16_OP_BEQ: u8 = 0x23;
pub const LC16_OP_BNE: u8 = 0x24;
pub const LC16_OP_BLT: u8 = 0x25;
pub const LC16_OP_BGE: u8 = 0x26;
pub const LC16_OP_SRA: u8 = 0x27;
pub const LC16_OP_HALT: u8 = 0xFF;

#[derive(Debug, Clone)]
pub struct Lc16Core {
    pub regs: [u16; 16],
    pub prev_regs: [u16; 16],
    pub pc: u16,
    pub cycles: u64,
    pub halted: bool,
    pub flags: CpuFlags,
}

impl Default for Lc16Core {
    fn default() -> Self {
        Self::new()
    }
}

impl Lc16Core {
    pub fn new() -> Self {
        let mut regs = [0u16; 16];
        regs[14] = (DEFAULT_SP & 0xFFFF) as u16; // R14 = SP
        Self {
            regs,
            prev_regs: regs,
            pc: 0,
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
            LC16_OP_LDI => format!("LDI R{rd}, 0x{imm:04X} ({simm})"),
            LC16_OP_MOV => format!("MOV R{rd}, R{rs1}"),
            LC16_OP_ADD => format!("ADD R{rd}, R{rs1}, R{rs2}"),
            LC16_OP_ADDI => format!("ADDI R{rd}, R{rs1}, {simm}"),
            LC16_OP_SUB => format!("SUB R{rd}, R{rs1}, R{rs2}"),
            LC16_OP_SUBI => format!("SUBI R{rd}, R{rs1}, {simm}"),
            LC16_OP_AND => format!("AND R{rd}, R{rs1}, R{rs2}"),
            LC16_OP_ANDI => format!("ANDI R{rd}, R{rs1}, 0x{imm:04X}"),
            LC16_OP_OR => format!("OR R{rd}, R{rs1}, R{rs2}"),
            LC16_OP_ORI => format!("ORI R{rd}, R{rs1}, 0x{imm:04X}"),
            LC16_OP_XOR => format!("XOR R{rd}, R{rs1}, R{rs2}"),
            LC16_OP_XORI => format!("XORI R{rd}, R{rs1}, 0x{imm:04X}"),
            LC16_OP_SHL => format!("SHL R{rd}, R{rs1}, {}", imm & 15),
            LC16_OP_SHR => format!("SHR R{rd}, R{rs1}, {}", imm & 15),
            LC16_OP_SRA => format!("SRA R{rd}, R{rs1}, {}", imm & 15),
            LC16_OP_CMP => format!("CMP R{rd}, R{rs1}"),
            LC16_OP_CMPI => format!("CMPI R{rd}, {simm}"),
            LC16_OP_LDB => format!("LDB R{rd}, [R{rs1} + 0x{imm:04X}]"),
            LC16_OP_STB => format!("STB R{rd}, [R{rs1} + 0x{imm:04X}]"),
            LC16_OP_LDW => format!("LDW R{rd}, [R{rs1} + 0x{imm:04X}]"),
            LC16_OP_STW => format!("STW R{rd}, [R{rs1} + 0x{imm:04X}]"),
            LC16_OP_JMP => format!("JMP 0x{imm:04X}"),
            LC16_OP_JEQ => format!("JEQ 0x{imm:04X}"),
            LC16_OP_JNE => format!("JNE 0x{imm:04X}"),
            LC16_OP_JLT => format!("JLT 0x{imm:04X}"),
            LC16_OP_JGE => format!("JGE 0x{imm:04X}"),
            LC16_OP_JGT => format!("JGT 0x{imm:04X}"),
            LC16_OP_JLE => format!("JLE 0x{imm:04X}"),
            LC16_OP_CALL => format!("CALL 0x{imm:04X}"),
            LC16_OP_RET => "RET".to_string(),
            LC16_OP_PUSH => format!("PUSH R{rd}"),
            LC16_OP_POP => format!("POP R{rd}"),
            LC16_OP_NEG => format!("NEG R{rd}, R{rs1}"),
            LC16_OP_NOT => format!("NOT R{rd}, R{rs1}"),
            LC16_OP_MUL => format!("MUL R{rd}, R{rs1}, R{rs2}"),
            LC16_OP_BEQ => format!("BEQ R{rd}, R{rs1}, 0x{imm:04X}"),
            LC16_OP_BNE => format!("BNE R{rd}, R{rs1}, 0x{imm:04X}"),
            LC16_OP_BLT => format!("BLT R{rd}, R{rs1}, 0x{imm:04X}"),
            LC16_OP_BGE => format!("BGE R{rd}, R{rs1}, 0x{imm:04X}"),
            LC16_OP_HALT => "HALT".to_string(),
            _ => format!(".WORD 0x{word:08X}"),
        }
    }
}

impl CpuCore for Lc16Core {
    fn arch(&self) -> CpuArch {
        CpuArch::Lc16
    }

    fn reset(&mut self) {
        self.regs = [0u16; 16];
        self.regs[14] = (DEFAULT_SP & 0xFFFF) as u16;
        self.prev_regs = self.regs;
        self.pc = 0;
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
                let sh = (imm & 15) as u32;
                let res = self.regs[rs1].wrapping_shl(sh);
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_SHR => {
                let sh = (imm & 15) as u32;
                let res = self.regs[rs1].wrapping_shr(sh);
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_SRA => {
                let sh = (imm & 15) as u32;
                let res = ((self.regs[rs1] as i16).wrapping_shr(sh)) as u16;
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_CMP => {
                let a = self.regs[rd];
                let b = self.regs[rs1];
                let (diff, borrow) = a.overflowing_sub(b);
                self.flags.z = a == b;
                self.flags.n = (a as i16) < (b as i16);
                self.flags.c = !borrow;
                let _ = diff;
            }
            LC16_OP_CMPI => {
                let a = self.regs[rd];
                let b = imm;
                let (_, borrow) = a.overflowing_sub(b);
                self.flags.z = a == b;
                self.flags.n = (a as i16) < (b as i16);
                self.flags.c = !borrow;
            }
            LC16_OP_LDB => {
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
            LC16_OP_STB => {
                let base = if rs1 == 0 && imm >= 0x4000 {
                    0
                } else {
                    self.regs[rs1]
                };
                let addr = base.wrapping_add(imm) as u32;
                bus.write8(addr, (self.regs[rd] & 0xFF) as u8);
            }
            LC16_OP_LDW => {
                let base = if rs1 == 0 && imm >= 0x4000 {
                    0
                } else {
                    self.regs[rs1]
                };
                let addr = base.wrapping_add(imm) as u32;
                let val = bus.read16(addr);
                self.regs[rd] = val;
                self.update_zn(val);
            }
            LC16_OP_STW => {
                let base = if rs1 == 0 && imm >= 0x4000 {
                    0
                } else {
                    self.regs[rs1]
                };
                let addr = base.wrapping_add(imm) as u32;
                bus.write16(addr, self.regs[rd]);
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
            LC16_OP_JGT => {
                if !self.flags.z && !self.flags.n {
                    next_pc = imm;
                }
            }
            LC16_OP_JLE => {
                if self.flags.z || self.flags.n {
                    next_pc = imm;
                }
            }
            LC16_OP_CALL => {
                let ret = next_pc;
                self.regs[15] = ret;
                self.regs[14] = self.regs[14].wrapping_sub(2);
                bus.write16(self.regs[14] as u32, ret);
                next_pc = imm;
            }
            LC16_OP_RET => {
                let ret = bus.read16(self.regs[14] as u32);
                self.regs[14] = self.regs[14].wrapping_add(2);
                next_pc = if ret != 0 { ret } else { self.regs[15] };
            }
            LC16_OP_PUSH => {
                self.regs[14] = self.regs[14].wrapping_sub(2);
                bus.write16(self.regs[14] as u32, self.regs[rd]);
            }
            LC16_OP_POP => {
                let val = bus.read16(self.regs[14] as u32);
                self.regs[14] = self.regs[14].wrapping_add(2);
                self.regs[rd] = val;
                self.update_zn(val);
            }
            LC16_OP_NEG => {
                let res = (-(self.regs[rs1] as i16)) as u16;
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_NOT => {
                let res = !self.regs[rs1];
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_MUL => {
                let res = self.regs[rs1].wrapping_mul(self.regs[rs2]);
                self.regs[rd] = res;
                self.update_zn(res);
            }
            LC16_OP_BEQ => {
                let eq = self.regs[rd] == self.regs[rs1];
                self.flags.z = eq;
                if eq {
                    next_pc = imm;
                }
            }
            LC16_OP_BNE => {
                let ne = self.regs[rd] != self.regs[rs1];
                self.flags.z = !ne;
                if ne {
                    next_pc = imm;
                }
            }
            LC16_OP_BLT => {
                let lt = (self.regs[rd] as i16) < (self.regs[rs1] as i16);
                self.flags.n = lt;
                if lt {
                    next_pc = imm;
                }
            }
            LC16_OP_BGE => {
                let ge = (self.regs[rd] as i16) >= (self.regs[rs1] as i16);
                self.flags.n = !ge;
                if ge {
                    next_pc = imm;
                }
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
        let aliases = [
            "R0", "R1", "R2", "R3", "R4", "R5", "R6", "R7", "R8", "R9", "R10", "R11", "R12",
            "R13", "SP", "LR",
        ];
        (0..16)
            .map(|i| RegisterValue {
                name: format!("R{i:<2} ({})", aliases[i]),
                short_name: aliases[i].to_string(),
                value: self.regs[i] as u32,
                changed: self.regs[i] != self.prev_regs[i],
                bits: 16,
            })
            .collect()
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
            arch: CpuArch::Lc16,
            pc: self.pc as u32,
            sp: self.regs[14] as u32,
            cycles: self.cycles,
            halted: self.halted,
            flags: self.flags,
        }
    }

    fn set_pc(&mut self, pc: u32) {
        self.pc = (pc & 0xFFFF) as u16;
    }
}
