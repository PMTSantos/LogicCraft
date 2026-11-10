//! RISC-V RV32I (with M-extension multiply/divide) 32-bit Processor Core (`src/cpu/rv32i.rs`).
//!
//! Standard 32-bit RISC-V integer architecture with 32 registers (`x0`..`x31`),
//! where `x0` (`zero`) is hardwired to `0` and `x2` (`sp`) initializes to `0x7FFC`.

use crate::cpu::bus::{MemoryBus, DEFAULT_SP};
use crate::cpu::{CpuArch, CpuCore, CpuFlags, CpuState, RegisterValue};

pub const RV_ABI_NAMES: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
    "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
    "t5", "t6",
];

#[derive(Debug, Clone)]
pub struct Rv32iCore {
    pub regs: [u32; 32],
    pub prev_regs: [u32; 32],
    pub pc: u32,
    pub cycles: u64,
    pub halted: bool,
    pub flags: CpuFlags,
}

impl Default for Rv32iCore {
    fn default() -> Self {
        Self::new()
    }
}

impl Rv32iCore {
    pub fn new() -> Self {
        let mut regs = [0u32; 32];
        regs[2] = DEFAULT_SP;
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
    pub fn read_reg(&self, idx: usize) -> u32 {
        if idx == 0 {
            0
        } else {
            self.regs[idx & 31]
        }
    }

    #[inline]
    pub fn write_reg(&mut self, idx: usize, val: u32) {
        if idx != 0 && idx < 32 {
            self.regs[idx] = val;
            self.flags.z = val == 0;
            self.flags.n = (val as i32) < 0;
        }
    }

    pub fn decode_and_disassemble(inst: u32, pc: u32) -> String {
        if inst == 0 || inst == 0x0000_0013 {
            return "nop".to_string();
        }
        let opcode = inst & 0x7F;
        let rd = ((inst >> 7) & 0x1F) as usize;
        let funct3 = (inst >> 12) & 0x07;
        let rs1 = ((inst >> 15) & 0x1F) as usize;
        let rs2 = ((inst >> 20) & 0x1F) as usize;
        let funct7 = (inst >> 25) & 0x7F;

        let rd_n = RV_ABI_NAMES[rd];
        let rs1_n = RV_ABI_NAMES[rs1];
        let rs2_n = RV_ABI_NAMES[rs2];

        match opcode {
            0x37 => {
                let imm = (inst & 0xFFFF_F000) >> 12;
                format!("lui {rd_n}, 0x{imm:X}")
            }
            0x17 => {
                let imm = (inst & 0xFFFF_F000) >> 12;
                format!("auipc {rd_n}, 0x{imm:X}")
            }
            0x6F => {
                let imm = decode_j_imm(inst);
                let target = pc.wrapping_add(imm as u32) & 0xFFFF;
                if rd == 0 {
                    format!("j 0x{target:04X}")
                } else {
                    format!("jal {rd_n}, 0x{target:04X}")
                }
            }
            0x67 => {
                let imm = decode_i_imm(inst);
                if rd == 0 && rs1 == 1 && imm == 0 {
                    "ret".to_string()
                } else {
                    format!("jalr {rd_n}, {imm}({rs1_n})")
                }
            }
            0x63 => {
                let imm = decode_b_imm(inst);
                let target = pc.wrapping_add(imm as u32) & 0xFFFF;
                let mnem = match funct3 {
                    0x0 => "beq",
                    0x1 => "bne",
                    0x4 => "blt",
                    0x5 => "bge",
                    0x6 => "bltu",
                    0x7 => "bgeu",
                    _ => "b??",
                };
                format!("{mnem} {rs1_n}, {rs2_n}, 0x{target:04X}")
            }
            0x03 => {
                let imm = decode_i_imm(inst);
                let mnem = match funct3 {
                    0x0 => "lb",
                    0x1 => "lh",
                    0x2 => "lw",
                    0x4 => "lbu",
                    0x5 => "lhu",
                    _ => "l??",
                };
                format!("{mnem} {rd_n}, {imm}({rs1_n})")
            }
            0x23 => {
                let imm = decode_s_imm(inst);
                let mnem = match funct3 {
                    0x0 => "sb",
                    0x1 => "sh",
                    0x2 => "sw",
                    _ => "s??",
                };
                format!("{mnem} {rs2_n}, {imm}({rs1_n})")
            }
            0x13 => {
                let imm = decode_i_imm(inst);
                let shamt = rs2;
                match funct3 {
                    0x0 => {
                        if rs1 == 0 {
                            format!("li {rd_n}, {imm}")
                        } else if imm == 0 {
                            format!("mv {rd_n}, {rs1_n}")
                        } else {
                            format!("addi {rd_n}, {rs1_n}, {imm}")
                        }
                    }
                    0x2 => format!("slti {rd_n}, {rs1_n}, {imm}"),
                    0x3 => format!("sltiu {rd_n}, {rs1_n}, {imm}"),
                    0x4 => format!("xori {rd_n}, {rs1_n}, {imm}"),
                    0x6 => format!("ori {rd_n}, {rs1_n}, {imm}"),
                    0x7 => format!("andi {rd_n}, {rs1_n}, {imm}"),
                    0x1 => format!("slli {rd_n}, {rs1_n}, {shamt}"),
                    0x5 => {
                        if (funct7 & 0x20) != 0 {
                            format!("srai {rd_n}, {rs1_n}, {shamt}")
                        } else {
                            format!("srli {rd_n}, {rs1_n}, {shamt}")
                        }
                    }
                    _ => format!("op_imm 0x{inst:08X}"),
                }
            }
            0x33 => {
                if funct7 == 0x01 {
                    let mnem = match funct3 {
                        0x0 => "mul",
                        0x4 => "div",
                        0x5 => "divu",
                        0x6 => "rem",
                        0x7 => "remu",
                        _ => "mul_op",
                    };
                    return format!("{mnem} {rd_n}, {rs1_n}, {rs2_n}");
                }
                let mnem = match (funct3, funct7) {
                    (0x0, 0x20) => "sub",
                    (0x0, _) => "add",
                    (0x1, _) => "sll",
                    (0x2, _) => "slt",
                    (0x3, _) => "sltu",
                    (0x4, _) => "xor",
                    (0x5, 0x20) => "sra",
                    (0x5, _) => "srl",
                    (0x6, _) => "or",
                    (0x7, _) => "and",
                    _ => "op",
                };
                format!("{mnem} {rd_n}, {rs1_n}, {rs2_n}")
            }
            0x73 => {
                if (inst >> 20) == 1 {
                    "ebreak (halt)".to_string()
                } else {
                    "ecall (halt)".to_string()
                }
            }
            _ => format!(".word 0x{inst:08X}"),
        }
    }
}

pub fn decode_i_imm(inst: u32) -> i32 {
    (inst as i32) >> 20
}

pub fn decode_s_imm(inst: u32) -> i32 {
    let imm11_5 = (inst >> 25) & 0x7F;
    let imm4_0 = (inst >> 7) & 0x1F;
    let raw = (imm11_5 << 5) | imm4_0;
    ((raw << 20) as i32) >> 20
}

pub fn decode_b_imm(inst: u32) -> i32 {
    let b12 = (inst >> 31) & 1;
    let b11 = (inst >> 7) & 1;
    let b10_5 = (inst >> 25) & 0x3F;
    let b4_1 = (inst >> 8) & 0x0F;
    let raw = (b12 << 12) | (b11 << 11) | (b10_5 << 5) | (b4_1 << 1);
    ((raw << 19) as i32) >> 19
}

pub fn decode_j_imm(inst: u32) -> i32 {
    let b20 = (inst >> 31) & 1;
    let b19_12 = (inst >> 12) & 0xFF;
    let b11 = (inst >> 20) & 1;
    let b10_1 = (inst >> 21) & 0x3FF;
    let raw = (b20 << 20) | (b19_12 << 12) | (b11 << 11) | (b10_1 << 1);
    ((raw << 11) as i32) >> 11
}

impl CpuCore for Rv32iCore {
    fn arch(&self) -> CpuArch {
        CpuArch::Rv32i
    }

    fn reset(&mut self) {
        self.regs = [0u32; 32];
        self.regs[2] = DEFAULT_SP;
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
        let inst = bus.read32(self.pc);
        self.cycles = self.cycles.wrapping_add(1);

        if inst == 0 {
            self.pc = self.pc.wrapping_add(4) & 0xFFFF;
            return true;
        }

        let opcode = inst & 0x7F;
        let rd = ((inst >> 7) & 0x1F) as usize;
        let funct3 = (inst >> 12) & 0x07;
        let rs1 = ((inst >> 15) & 0x1F) as usize;
        let rs2 = ((inst >> 20) & 0x1F) as usize;
        let funct7 = (inst >> 25) & 0x7F;

        let v1 = self.read_reg(rs1);
        let v2 = self.read_reg(rs2);
        let mut next_pc = self.pc.wrapping_add(4);

        match opcode {
            0x37 => {
                // LUI
                self.write_reg(rd, inst & 0xFFFF_F000);
            }
            0x17 => {
                // AUIPC
                let imm = inst & 0xFFFF_F000;
                self.write_reg(rd, self.pc.wrapping_add(imm));
            }
            0x6F => {
                // JAL
                let imm = decode_j_imm(inst);
                let ret_addr = self.pc.wrapping_add(4);
                next_pc = self.pc.wrapping_add(imm as u32);
                self.write_reg(rd, ret_addr);
            }
            0x67 => {
                // JALR
                let imm = decode_i_imm(inst);
                let ret_addr = self.pc.wrapping_add(4);
                next_pc = v1.wrapping_add(imm as u32) & !1;
                self.write_reg(rd, ret_addr);
            }
            0x63 => {
                // BRANCH
                let imm = decode_b_imm(inst);
                let take = match funct3 {
                    0x0 => v1 == v2,                   // BEQ
                    0x1 => v1 != v2,                   // BNE
                    0x4 => (v1 as i32) < (v2 as i32),  // BLT
                    0x5 => (v1 as i32) >= (v2 as i32), // BGE
                    0x6 => v1 < v2,                    // BLTU
                    0x7 => v1 >= v2,                   // BGEU
                    _ => false,
                };
                self.flags.z = v1 == v2;
                self.flags.n = (v1 as i32) < (v2 as i32);
                if take {
                    next_pc = self.pc.wrapping_add(imm as u32);
                }
            }
            0x03 => {
                // LOAD
                let imm = decode_i_imm(inst);
                let addr = v1.wrapping_add(imm as u32);
                let loaded = match funct3 {
                    0x0 => (bus.read8(addr) as i8) as i32 as u32,  // LB
                    0x1 => (bus.read16(addr) as i16) as i32 as u32, // LH
                    0x2 => bus.read32(addr),                        // LW
                    0x4 => bus.read8(addr) as u32,                  // LBU
                    0x5 => bus.read16(addr) as u32,                 // LHU
                    _ => 0,
                };
                self.write_reg(rd, loaded);
            }
            0x23 => {
                // STORE
                let imm = decode_s_imm(inst);
                let addr = v1.wrapping_add(imm as u32);
                match funct3 {
                    0x0 => bus.write8(addr, (v2 & 0xFF) as u8),
                    0x1 => bus.write16(addr, (v2 & 0xFFFF) as u16),
                    0x2 => bus.write32(addr, v2),
                    _ => {}
                }
            }
            0x13 => {
                // OP-IMM
                let imm = decode_i_imm(inst);
                let uimm = imm as u32;
                let shamt = (inst >> 20) & 0x1F;
                let res = match funct3 {
                    0x0 => v1.wrapping_add(uimm),                         // ADDI
                    0x2 => u32::from((v1 as i32) < imm),                  // SLTI
                    0x3 => u32::from(v1 < uimm),                          // SLTIU
                    0x4 => v1 ^ uimm,                                     // XORI
                    0x6 => v1 | uimm,                                     // ORI
                    0x7 => v1 & uimm,                                     // ANDI
                    0x1 => v1 << shamt,                                   // SLLI
                    0x5 => {
                        if (funct7 & 0x20) != 0 {
                            ((v1 as i32) >> shamt) as u32                 // SRAI
                        } else {
                            v1 >> shamt                                   // SRLI
                        }
                    }
                    _ => 0,
                };
                self.write_reg(rd, res);
            }
            0x33 => {
                // OP & RV32M
                let res = if funct7 == 0x01 {
                    match funct3 {
                        0x0 => v1.wrapping_mul(v2),
                        0x4 => {
                            if v2 == 0 {
                                u32::MAX
                            } else {
                                ((v1 as i32).wrapping_div(v2 as i32)) as u32
                            }
                        }
                        0x5 => {
                            if v2 == 0 {
                                u32::MAX
                            } else {
                                v1 / v2
                            }
                        }
                        0x6 => {
                            if v2 == 0 {
                                v1
                            } else {
                                ((v1 as i32).wrapping_rem(v2 as i32)) as u32
                            }
                        }
                        0x7 => {
                            if v2 == 0 {
                                v1
                            } else {
                                v1 % v2
                            }
                        }
                        _ => 0,
                    }
                } else {
                    match (funct3, funct7) {
                        (0x0, 0x20) => v1.wrapping_sub(v2),               // SUB
                        (0x0, _) => v1.wrapping_add(v2),                  // ADD
                        (0x1, _) => v1 << (v2 & 0x1F),                    // SLL
                        (0x2, _) => u32::from((v1 as i32) < (v2 as i32)), // SLT
                        (0x3, _) => u32::from(v1 < v2),                   // SLTU
                        (0x4, _) => v1 ^ v2,                              // XOR
                        (0x5, 0x20) => ((v1 as i32) >> (v2 & 0x1F)) as u32, // SRA
                        (0x5, _) => v1 >> (v2 & 0x1F),                    // SRL
                        (0x6, _) => v1 | v2,                              // OR
                        (0x7, _) => v1 & v2,                              // AND
                        _ => 0,
                    }
                };
                self.write_reg(rd, res);
            }
            0x73 => {
                // ECALL / EBREAK -> Halt
                self.halted = true;
                return false;
            }
            _ => {}
        }

        self.pc = next_pc & 0xFFFF;
        true
    }

    fn registers(&self) -> Vec<RegisterValue> {
        // Show the 16 primary registers (zero, ra, sp, t0..t6, s0..s3, a0..a5) + all 32
        (0..32)
            .map(|i| RegisterValue {
                name: format!("x{i:<2} ({})", RV_ABI_NAMES[i]),
                short_name: RV_ABI_NAMES[i].to_string(),
                value: self.regs[i],
                changed: self.regs[i] != self.prev_regs[i],
                bits: 32,
            })
            .collect()
    }

    fn disassemble_current(&self, bus: &MemoryBus) -> String {
        let inst = bus.peek32(self.pc);
        Self::decode_and_disassemble(inst, self.pc)
    }

    fn disassemble_at(&self, addr: u32, bus: &MemoryBus) -> (String, u32) {
        let inst = bus.peek32(addr);
        (Self::decode_and_disassemble(inst, addr), 4)
    }

    fn get_state(&self) -> CpuState {
        CpuState {
            arch: CpuArch::Rv32i,
            pc: self.pc,
            sp: self.regs[2],
            cycles: self.cycles,
            halted: self.halted,
            flags: self.flags,
        }
    }

    fn set_pc(&mut self, pc: u32) {
        self.pc = pc & 0xFFFF;
    }
}
