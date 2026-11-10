//! Integrated Multi-Architecture Assembler (`src/cpu/assembler.rs`).
//!
//! Supports:
//! - Architectures: `RV32I` (32-bit RISC-V), `LC16` (16-bit Custom), and `MIN8` (8-bit Minimal).
//! - Directives: `.org`, `.byte` (`.db`), `.word` (`.dw` / `.half`), `.dword`, `.space`, `.equ` / `.const`.
//! - Labels (`loop:`, `start:`) and symbolic constants (`VRAM_BASE = 0x8000` or `.equ VRAM_BASE, 0x8000`).
//! - Detailed syntax & semantic error reporting with 1-based line numbers and source snippets.

use std::collections::HashMap;

use crate::cpu::lc16::*;
use crate::cpu::min8::*;
use crate::cpu::CpuArch;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssemblyError {
    pub line: usize, // 1-based line number
    pub message: String,
    pub line_text: String,
}

#[derive(Debug, Clone)]
pub struct AssembledProgram {
    pub bytes: Vec<u8>,
    pub entry_pc: u32,
    pub instruction_count: usize,
    pub labels: HashMap<String, u32>,
    pub addr_to_line: HashMap<u32, usize>,
    pub line_to_addr: HashMap<usize, u32>,
}

#[derive(Debug, Clone)]
struct ParsedLine {
    line_num: usize,
    raw_text: String,
    mnemonic: String,
    operands: Vec<String>,
}

pub fn assemble(source: &str, arch: CpuArch) -> Result<AssembledProgram, Vec<AssemblyError>> {
    let mut errors = Vec::new();
    let mut constants: HashMap<String, i64> = HashMap::new();
    let mut labels: HashMap<String, u32> = HashMap::new();
    let mut parsed_lines: Vec<ParsedLine> = Vec::new();

    // Built-in helpful MMIO/VRAM constants always available
    constants.insert("VRAM_BASE".to_string(), 0x8000);
    constants.insert("MMIO_BASE".to_string(), 0xF000);
    constants.insert("MMIO_KEY_STATE".to_string(), 0xF000);
    constants.insert("MMIO_KEY_P1_UP".to_string(), 0xF001);
    constants.insert("MMIO_KEY_P1_DOWN".to_string(), 0xF002);
    constants.insert("MMIO_KEY_P2_UP".to_string(), 0xF003);
    constants.insert("MMIO_KEY_P2_DOWN".to_string(), 0xF004);
    constants.insert("MMIO_KEY_START".to_string(), 0xF005);
    constants.insert("MMIO_RNG".to_string(), 0xF010);
    constants.insert("MMIO_TICK".to_string(), 0xF012);
    constants.insert("MMIO_VSYNC".to_string(), 0xF016);
    constants.insert("MMIO_P1_SCORE".to_string(), 0xF020);
    constants.insert("MMIO_P2_SCORE".to_string(), 0xF021);
    constants.insert("MMIO_GPIO_OUT".to_string(), 0xF022);
    constants.insert("MMIO_GPIO_IN".to_string(), 0xF023);
    constants.insert("MMIO_VRAM_CLEAR".to_string(), 0xF030);

    // Pre-pass: strip comments, extract `.equ` / `=` constants, labels, and instructions
    for (idx, raw_line) in source.lines().enumerate() {
        let line_num = idx + 1;
        let clean = strip_comment(raw_line).trim().to_string();
        if clean.is_empty() {
            continue;
        }

        // Check `NAME = VALUE` assignment
        if let Some((lhs, rhs)) = clean.split_once('=') {
            let name = lhs.trim();
            if is_valid_ident(name) {
                match eval_expr(rhs.trim(), &constants, &labels) {
                    Ok(v) => {
                        constants.insert(name.to_uppercase(), v);
                        constants.insert(name.to_string(), v);
                    }
                    Err(msg) => errors.push(AssemblyError {
                        line: line_num,
                        message: msg,
                        line_text: raw_line.to_string(),
                    }),
                }
                continue;
            }
        }

        // Extract one or more `label:` prefixes on the line
        let mut rest = clean.as_str();
        while let Some(colon_pos) = rest.find(':') {
            let candidate = rest[..colon_pos].trim();
            if candidate.is_empty() || candidate.contains(char::is_whitespace) {
                break;
            }
            if !is_valid_ident(candidate) {
                errors.push(AssemblyError {
                    line: line_num,
                    message: format!("Label inválido: '{candidate}'"),
                    line_text: raw_line.to_string(),
                });
                break;
            }
            parsed_lines.push(ParsedLine {
                line_num,
                raw_text: raw_line.to_string(),
                mnemonic: "@LABEL".to_string(),
                operands: vec![candidate.to_string()],
            });
            rest = rest[colon_pos + 1..].trim();
        }

        if rest.is_empty() {
            continue;
        }

        let (mnem, ops_str) = split_mnemonic_operands(rest);
        let mnem_lower = mnem.to_ascii_lowercase();

        if mnem_lower == ".equ" || mnem_lower == ".const" || mnem_lower == ".set" {
            let ops = split_operands(ops_str);
            if ops.len() != 2 {
                errors.push(AssemblyError {
                    line: line_num,
                    message: format!("Diretiva '{mnem}' requer 2 operandos: NOME, VALOR"),
                    line_text: raw_line.to_string(),
                });
            } else {
                let name = ops[0].trim();
                match eval_expr(&ops[1], &constants, &labels) {
                    Ok(v) => {
                        constants.insert(name.to_uppercase(), v);
                        constants.insert(name.to_string(), v);
                    }
                    Err(msg) => errors.push(AssemblyError {
                        line: line_num,
                        message: msg,
                        line_text: raw_line.to_string(),
                    }),
                }
            }
            continue;
        }

        parsed_lines.push(ParsedLine {
            line_num,
            raw_text: raw_line.to_string(),
            mnemonic: mnem.to_string(),
            operands: split_operands(ops_str),
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Pass 1: Compute addresses and record label positions
    let mut pc: u32 = 0;
    let mut entry_pc: Option<u32> = None;

    for pl in &parsed_lines {
        if pl.mnemonic == "@LABEL" {
            let lbl = &pl.operands[0];
            if labels.contains_key(lbl) {
                errors.push(AssemblyError {
                    line: pl.line_num,
                    message: format!("Label duplicado: '{lbl}'"),
                    line_text: pl.raw_text.clone(),
                });
            } else {
                labels.insert(lbl.clone(), pc);
                labels.insert(lbl.to_uppercase(), pc);
            }
            continue;
        }

        let mnem_lower = pl.mnemonic.to_ascii_lowercase();
        match mnem_lower.as_str() {
            ".org" => {
                if pl.operands.len() != 1 {
                    errors.push(AssemblyError {
                        line: pl.line_num,
                        message: "Diretiva .org requer exatamente 1 endereço".to_string(),
                        line_text: pl.raw_text.clone(),
                    });
                } else {
                    match eval_expr(&pl.operands[0], &constants, &labels) {
                        Ok(addr) if (0..=0xFFFF).contains(&addr) => {
                            pc = addr as u32;
                        }
                        Ok(addr) => errors.push(AssemblyError {
                            line: pl.line_num,
                            message: format!("Endereço .org fora dos limites (0x0000..0xFFFF): {addr}"),
                            line_text: pl.raw_text.clone(),
                        }),
                        Err(msg) => errors.push(AssemblyError {
                            line: pl.line_num,
                            message: msg,
                            line_text: pl.raw_text.clone(),
                        }),
                    }
                }
            }
            ".byte" | ".db" => {
                if pl.operands.is_empty() {
                    errors.push(AssemblyError {
                        line: pl.line_num,
                        message: "Diretiva .byte requer pelo menos 1 valor".to_string(),
                        line_text: pl.raw_text.clone(),
                    });
                }
                pc = pc.wrapping_add(pl.operands.len() as u32);
            }
            ".word" | ".dw" | ".half" => {
                if pl.operands.is_empty() {
                    errors.push(AssemblyError {
                        line: pl.line_num,
                        message: "Diretiva .word requer pelo menos 1 valor".to_string(),
                        line_text: pl.raw_text.clone(),
                    });
                }
                pc = pc.wrapping_add((pl.operands.len() * 2) as u32);
            }
            ".dword" | ".dd" => {
                if pl.operands.is_empty() {
                    errors.push(AssemblyError {
                        line: pl.line_num,
                        message: "Diretiva .dword requer pelo menos 1 valor".to_string(),
                        line_text: pl.raw_text.clone(),
                    });
                }
                pc = pc.wrapping_add((pl.operands.len() * 4) as u32);
            }
            ".space" => {
                if pl.operands.len() != 1 {
                    errors.push(AssemblyError {
                        line: pl.line_num,
                        message: "Diretiva .space requer 1 tamanho em bytes".to_string(),
                        line_text: pl.raw_text.clone(),
                    });
                } else if let Ok(n) = eval_expr(&pl.operands[0], &constants, &labels) {
                    pc = pc.wrapping_add(n.max(0) as u32);
                }
            }
            _ => {
                if entry_pc.is_none() {
                    entry_pc = Some(pc);
                }
                let sz = instruction_size(&mnem_lower, &pl.operands, arch, &constants, &labels);
                pc = pc.wrapping_add(sz);
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    // Pass 2: Emit machine code bytes
    let mut mem = Vec::new();
    let mut addr_to_line = HashMap::new();
    let mut line_to_addr = HashMap::new();
    let mut instruction_count = 0usize;
    pc = 0;

    for pl in &parsed_lines {
        if pl.mnemonic == "@LABEL" {
            continue;
        }
        let mnem_lower = pl.mnemonic.to_ascii_lowercase();
        match mnem_lower.as_str() {
            ".org" => {
                if let Ok(addr) = eval_expr(&pl.operands[0], &constants, &labels) {
                    pc = addr as u32;
                }
            }
            ".byte" | ".db" => {
                line_to_addr.entry(pl.line_num).or_insert(pc);
                for op in &pl.operands {
                    match eval_expr(op, &constants, &labels) {
                        Ok(v) => {
                            write_byte_at(&mut mem, pc as usize, (v & 0xFF) as u8);
                            pc += 1;
                        }
                        Err(msg) => errors.push(AssemblyError {
                            line: pl.line_num,
                            message: msg,
                            line_text: pl.raw_text.clone(),
                        }),
                    }
                }
            }
            ".word" | ".dw" | ".half" => {
                line_to_addr.entry(pl.line_num).or_insert(pc);
                for op in &pl.operands {
                    match eval_expr(op, &constants, &labels) {
                        Ok(v) => {
                            let bytes = (v as u16).to_le_bytes();
                            write_byte_at(&mut mem, pc as usize, bytes[0]);
                            write_byte_at(&mut mem, (pc + 1) as usize, bytes[1]);
                            pc += 2;
                        }
                        Err(msg) => errors.push(AssemblyError {
                            line: pl.line_num,
                            message: msg,
                            line_text: pl.raw_text.clone(),
                        }),
                    }
                }
            }
            ".dword" | ".dd" => {
                line_to_addr.entry(pl.line_num).or_insert(pc);
                for op in &pl.operands {
                    match eval_expr(op, &constants, &labels) {
                        Ok(v) => {
                            let bytes = (v as u32).to_le_bytes();
                            for (i, b) in bytes.into_iter().enumerate() {
                                write_byte_at(&mut mem, pc as usize + i, b);
                            }
                            pc += 4;
                        }
                        Err(msg) => errors.push(AssemblyError {
                            line: pl.line_num,
                            message: msg,
                            line_text: pl.raw_text.clone(),
                        }),
                    }
                }
            }
            ".space" => {
                line_to_addr.entry(pl.line_num).or_insert(pc);
                if let Ok(n) = eval_expr(&pl.operands[0], &constants, &labels) {
                    let count = n.max(0) as usize;
                    for i in 0..count {
                        write_byte_at(&mut mem, pc as usize + i, 0);
                    }
                    pc += count as u32;
                }
            }
            _ => {
                addr_to_line.insert(pc, pl.line_num);
                line_to_addr.entry(pl.line_num).or_insert(pc);
                let res = match arch {
                    CpuArch::Rv32i => {
                        encode_rv32i(&mnem_lower, &pl.operands, pc, &constants, &labels)
                    }
                    CpuArch::Lc16 => {
                        encode_lc16(&mnem_lower, &pl.operands, pc, &constants, &labels)
                    }
                    CpuArch::Min8 => {
                        encode_min8(&mnem_lower, &pl.operands, pc, &constants, &labels)
                    }
                };
                match res {
                    Ok(words) => {
                        for w in words {
                            let bytes = w.to_le_bytes();
                            for (i, b) in bytes.into_iter().enumerate() {
                                write_byte_at(&mut mem, pc as usize + i, b);
                            }
                            addr_to_line.insert(pc, pl.line_num);
                            pc += 4;
                            instruction_count += 1;
                        }
                    }
                    Err(msg) => errors.push(AssemblyError {
                        line: pl.line_num,
                        message: msg,
                        line_text: pl.raw_text.clone(),
                    }),
                }
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(AssembledProgram {
        bytes: mem,
        entry_pc: entry_pc.unwrap_or(0),
        instruction_count,
        labels,
        addr_to_line,
        line_to_addr,
    })
}

fn write_byte_at(buf: &mut Vec<u8>, addr: usize, val: u8) {
    if addr >= 65536 {
        return;
    }
    if buf.len() <= addr {
        buf.resize(addr + 1, 0);
    }
    buf[addr] = val;
}

fn instruction_size(
    mnem: &str,
    operands: &[String],
    arch: CpuArch,
    constants: &HashMap<String, i64>,
    labels: &HashMap<String, u32>,
) -> u32 {
    match arch {
        CpuArch::Rv32i => match mnem {
            "li" | "ldi" => {
                if operands.len() == 2 {
                    if let Ok(val) = eval_expr(&operands[1], constants, labels) {
                        if (-2048..=2047).contains(&val) {
                            return 4;
                        }
                    }
                }
                8 // lui + addi
            }
            "la" => 8,
            // Classic `cmp rs1, imm` or `cmpi` + `jeq`/`jne` on RV32I
            "cmpi" => 4,
            _ => 4,
        },
        CpuArch::Lc16 | CpuArch::Min8 => 4,
    }
}

fn strip_comment(line: &str) -> &str {
    let mut end = line.len();
    for pat in [";", "#", "//"] {
        if let Some(pos) = line.find(pat) {
            end = end.min(pos);
        }
    }
    &line[..end]
}

fn is_valid_ident(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_' || first == '.') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn split_mnemonic_operands(s: &str) -> (&str, &str) {
    let s = s.trim();
    if let Some(idx) = s.find(char::is_whitespace) {
        (&s[..idx], s[idx..].trim())
    } else {
        (s, "")
    }
}

fn split_operands(s: &str) -> Vec<String> {
    if s.trim().is_empty() {
        return Vec::new();
    }
    s.split(',')
        .map(|part| part.trim().to_string())
        .filter(|part| !part.is_empty())
        .collect()
}

pub fn eval_expr(
    expr: &str,
    constants: &HashMap<String, i64>,
    labels: &HashMap<String, u32>,
) -> Result<i64, String> {
    let s = expr.trim();
    if s.is_empty() {
        return Err("Expressão vazia".to_string());
    }
    // Support simple binary `+` or `-` in immediate expressions (e.g. `VRAM_BASE + 64`)
    if let Some(pos) = s[1..].rfind('+') {
        let idx = pos + 1;
        let l = eval_expr(&s[..idx], constants, labels)?;
        let r = eval_expr(&s[idx + 1..], constants, labels)?;
        return Ok(l + r);
    }
    if let Some(pos) = s[1..].rfind('-') {
        let idx = pos + 1;
        let lhs_part = s[..idx].trim();
        if !lhs_part.is_empty() {
            let l = eval_expr(lhs_part, constants, labels)?;
            let r = eval_expr(&s[idx + 1..], constants, labels)?;
            return Ok(l - r);
        }
    }

    if let Some(rest) = s.strip_prefix('-') {
        return Ok(-eval_expr(rest, constants, labels)?);
    }
    if let Some(rest) = s.strip_prefix('+') {
        return eval_expr(rest, constants, labels);
    }
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        let clean = hex.replace('_', "");
        return i64::from_str_radix(&clean, 16)
            .map_err(|_| format!("Valor hexadecimal inválido: '{s}'"));
    }
    if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        let clean = bin.replace('_', "");
        return i64::from_str_radix(&clean, 2)
            .map_err(|_| format!("Valor binário inválido: '{s}'"));
    }
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 3 {
        let ch = s.chars().nth(1).unwrap_or('\0');
        return Ok(ch as i64);
    }
    if let Ok(num) = s.replace('_', "").parse::<i64>() {
        return Ok(num);
    }
    if let Some(&v) = constants
        .get(s)
        .or_else(|| constants.get(&s.to_uppercase()))
    {
        return Ok(v);
    }
    if let Some(&addr) = labels.get(s).or_else(|| labels.get(&s.to_uppercase())) {
        return Ok(addr as i64);
    }
    Err(format!("Símbolo ou imediato desconhecido: '{s}'"))
}

pub fn parse_rv_reg(s: &str) -> Result<u32, String> {
    let lower = s.trim().to_ascii_lowercase();
    let idx = match lower.as_str() {
        "zero" | "x0" | "r0" => 0,
        "ra" | "x1" | "lr" => 1,
        "sp" | "x2" => 2,
        "gp" | "x3" => 3,
        "tp" | "x4" => 4,
        "t0" | "x5" | "r1" | "v0" | "a" => 5,
        "t1" | "x6" | "r2" | "v1" | "b" => 6,
        "t2" | "x7" | "r3" | "v2" | "c" => 7,
        "s0" | "fp" | "x8" | "r4" | "v3" | "d" => 8,
        "s1" | "x9" | "r5" | "v4" | "x" => 9,
        "a0" | "x10" | "r6" | "v5" | "y" => 10,
        "a1" | "x11" | "r7" | "v6" => 11,
        "a2" | "x12" | "r8" | "v7" => 12,
        "a3" | "x13" | "r9" => 13,
        "a4" | "x14" | "r10" => 14,
        "a5" | "x15" | "r11" => 15,
        "a6" | "x16" | "r12" => 16,
        "a7" | "x17" | "r13" => 17,
        "s2" | "x18" | "r14" => 18,
        "s3" | "x19" | "r15" => 19,
        "s4" | "x20" => 20,
        "s5" | "x21" => 21,
        "s6" | "x22" => 22,
        "s7" | "x23" => 23,
        "s8" | "x24" => 24,
        "s9" | "x25" => 25,
        "s10" | "x26" => 26,
        "s11" | "x27" => 27,
        "t3" | "x28" => 28,
        "t4" | "x29" => 29,
        "t5" | "x30" => 30,
        "t6" | "x31" => 31,
        _ => {
            if let Some(num_s) = lower.strip_prefix('x') {
                if let Ok(n) = num_s.parse::<u32>() {
                    if n < 32 {
                        return Ok(n);
                    }
                }
            }
            return Err(format!("Registo RISC-V inválido: '{s}'"));
        }
    };
    Ok(idx)
}

pub fn parse_lc16_reg(s: &str) -> Result<u8, String> {
    let lower = s.trim().to_ascii_lowercase();
    let idx = match lower.as_str() {
        "r0" | "x0" | "zero" | "v0" | "a" => 0,
        "r1" | "x1" | "t0" | "v1" | "b" => 1,
        "r2" | "x2" | "t1" | "v2" | "c" => 2,
        "r3" | "x3" | "t2" | "v3" | "d" => 3,
        "r4" | "x4" | "t3" | "v4" | "x" => 4,
        "r5" | "x5" | "t4" | "v5" | "y" => 5,
        "r6" | "x6" | "t5" | "v6" | "h" => 6,
        "r7" | "x7" | "t6" | "v7" | "l" => 7,
        "r8" | "x8" | "s0" | "fp" => 8,
        "r9" | "x9" | "s1" => 9,
        "r10" | "x10" | "s2" | "a0" => 10,
        "r11" | "x11" | "s3" | "a1" => 11,
        "r12" | "x12" | "s4" | "a2" => 12,
        "r13" | "x13" | "s5" | "a3" => 13,
        "r14" | "x14" | "sp" | "s6" => 14,
        "r15" | "x15" | "ra" | "lr" | "s7" => 15,
        "a4" | "s8" => 12,
        "a5" | "s9" => 13,
        _ => return Err(format!("Registo inválido: '{s}' (use R0..R15 ou t0..t6/s0..s7/a0..a3)")),
    };
    Ok(idx)
}

/// Parses memory operand forms:
/// - `offset(rs1)` -> `(rs1, offset)`
/// - `[rs1 + offset]` -> `(rs1, offset)`
/// - `[rs1]` -> `(rs1, 0)`
/// - `[0x8000]` or `0x8000` -> `(0, 0x8000)`
fn parse_mem_operand(
    op: &str,
    arch: CpuArch,
    constants: &HashMap<String, i64>,
    labels: &HashMap<String, u32>,
) -> Result<(u8, i64), String> {
    let s = op.trim();
    // Form 1: `imm(reg)`
    if let (Some(lp), Some(rp)) = (s.find('('), s.rfind(')')) {
        if lp < rp {
            let imm_str = s[..lp].trim();
            let reg_str = s[lp + 1..rp].trim();
            let offset = if imm_str.is_empty() {
                0
            } else {
                eval_expr(imm_str, constants, labels)?
            };
            let reg = match arch {
                CpuArch::Rv32i => parse_rv_reg(reg_str)? as u8,
                _ => parse_lc16_reg(reg_str)?,
            };
            return Ok((reg, offset));
        }
    }
    // Form 2: `[reg + imm]` or `[imm]` or `[reg]`
    let inner = s
        .strip_prefix('[')
        .and_then(|r| r.strip_suffix(']'))
        .map(|r| r.trim())
        .unwrap_or(s);

    if let Some((lhs, rhs)) = inner.split_once('+') {
        let reg = match arch {
            CpuArch::Rv32i => parse_rv_reg(lhs.trim())? as u8,
            _ => parse_lc16_reg(lhs.trim())?,
        };
        let off = eval_expr(rhs.trim(), constants, labels)?;
        return Ok((reg, off));
    }
    if let Some((lhs, rhs)) = inner.split_once('-') {
        if let Ok(reg) = match arch {
            CpuArch::Rv32i => parse_rv_reg(lhs.trim()).map(|r| r as u8),
            _ => parse_lc16_reg(lhs.trim()),
        } {
            let off = eval_expr(rhs.trim(), constants, labels)?;
            return Ok((reg, -off));
        }
    }

    // Try single register
    if let Ok(reg) = match arch {
        CpuArch::Rv32i => parse_rv_reg(inner).map(|r| r as u8),
        _ => parse_lc16_reg(inner),
    } {
        return Ok((reg, 0));
    }

    // Otherwise direct address
    let addr = eval_expr(inner, constants, labels)?;
    Ok((0, addr))
}

fn encode_r_type(opcode: u32, rd: u32, funct3: u32, rs1: u32, rs2: u32, funct7: u32) -> u32 {
    (opcode & 0x7F)
        | ((rd & 0x1F) << 7)
        | ((funct3 & 0x07) << 12)
        | ((rs1 & 0x1F) << 15)
        | ((rs2 & 0x1F) << 20)
        | ((funct7 & 0x7F) << 25)
}

fn encode_i_type(opcode: u32, rd: u32, funct3: u32, rs1: u32, imm: i32) -> u32 {
    let imm12 = (imm as u32) & 0xFFF;
    (opcode & 0x7F)
        | ((rd & 0x1F) << 7)
        | ((funct3 & 0x07) << 12)
        | ((rs1 & 0x1F) << 15)
        | (imm12 << 20)
}

fn encode_s_type(opcode: u32, funct3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let uimm = (imm as u32) & 0xFFF;
    let imm4_0 = uimm & 0x1F;
    let imm11_5 = (uimm >> 5) & 0x7F;
    (opcode & 0x7F)
        | (imm4_0 << 7)
        | ((funct3 & 0x07) << 12)
        | ((rs1 & 0x1F) << 15)
        | ((rs2 & 0x1F) << 20)
        | (imm11_5 << 25)
}

fn encode_b_type(opcode: u32, funct3: u32, rs1: u32, rs2: u32, offset: i32) -> u32 {
    let uoff = (offset as u32) & 0x1FFE;
    let b12 = (uoff >> 12) & 1;
    let b11 = (uoff >> 11) & 1;
    let b10_5 = (uoff >> 5) & 0x3F;
    let b4_1 = (uoff >> 1) & 0x0F;
    (opcode & 0x7F)
        | (b11 << 7)
        | (b4_1 << 8)
        | ((funct3 & 0x07) << 12)
        | ((rs1 & 0x1F) << 15)
        | ((rs2 & 0x1F) << 20)
        | (b10_5 << 25)
        | (b12 << 31)
}

fn encode_u_type(opcode: u32, rd: u32, upper20: u32) -> u32 {
    (opcode & 0x7F) | ((rd & 0x1F) << 7) | ((upper20 & 0xFFFFF) << 12)
}

fn encode_j_type(opcode: u32, rd: u32, offset: i32) -> u32 {
    let uoff = (offset as u32) & 0x1F_FFFE;
    let b20 = (uoff >> 20) & 1;
    let b19_12 = (uoff >> 12) & 0xFF;
    let b11 = (uoff >> 11) & 1;
    let b10_1 = (uoff >> 1) & 0x3FF;
    (opcode & 0x7F)
        | ((rd & 0x1F) << 7)
        | (b19_12 << 12)
        | (b11 << 20)
        | (b10_1 << 21)
        | (b20 << 31)
}

fn encode_rv32i(
    mnem: &str,
    ops: &[String],
    pc: u32,
    constants: &HashMap<String, i64>,
    labels: &HashMap<String, u32>,
) -> Result<Vec<u32>, String> {
    let need_ops = |n: usize| -> Result<(), String> {
        if ops.len() != n {
            Err(format!(
                "Instrução '{mnem}' espera {n} operando(s), mas recebeu {}",
                ops.len()
            ))
        } else {
            Ok(())
        }
    };

    match mnem {
        "nop" => Ok(vec![encode_i_type(0x13, 0, 0x0, 0, 0)]),
        "ebreak" | "halt" => Ok(vec![0x0010_0073]),
        "ecall" => Ok(vec![0x0000_0073]),
        "ret" => Ok(vec![encode_i_type(0x67, 0, 0x0, 1, 0)]),
        "li" | "ldi" => {
            need_ops(2)?;
            let rd = parse_rv_reg(&ops[0])?;
            let val = eval_expr(&ops[1], constants, labels)?;
            if (-2048..=2047).contains(&val) {
                Ok(vec![encode_i_type(0x13, rd, 0x0, 0, val as i32)])
            } else {
                let uval = val as i32 as u32;
                let upper = (uval.wrapping_add(0x800)) >> 12;
                let lower = (uval.wrapping_sub(upper << 12)) as i32;
                Ok(vec![
                    encode_u_type(0x37, rd, upper),
                    encode_i_type(0x13, rd, 0x0, rd, lower),
                ])
            }
        }
        "la" => {
            need_ops(2)?;
            let rd = parse_rv_reg(&ops[0])?;
            let val = eval_expr(&ops[1], constants, labels)?;
            let uval = val as i32 as u32;
            let upper = (uval.wrapping_add(0x800)) >> 12;
            let lower = (uval.wrapping_sub(upper << 12)) as i32;
            Ok(vec![
                encode_u_type(0x37, rd, upper),
                encode_i_type(0x13, rd, 0x0, rd, lower),
            ])
        }
        "mv" | "mov" => {
            need_ops(2)?;
            let rd = parse_rv_reg(&ops[0])?;
            let rs = parse_rv_reg(&ops[1])?;
            Ok(vec![encode_i_type(0x13, rd, 0x0, rs, 0)])
        }
        "not" => {
            need_ops(2)?;
            let rd = parse_rv_reg(&ops[0])?;
            let rs = parse_rv_reg(&ops[1])?;
            Ok(vec![encode_i_type(0x13, rd, 0x4, rs, -1)])
        }
        "neg" => {
            need_ops(2)?;
            let rd = parse_rv_reg(&ops[0])?;
            let rs = parse_rv_reg(&ops[1])?;
            Ok(vec![encode_r_type(0x33, rd, 0x0, 0, rs, 0x20)])
        }
        "lui" => {
            need_ops(2)?;
            let rd = parse_rv_reg(&ops[0])?;
            let imm = eval_expr(&ops[1], constants, labels)? as u32;
            Ok(vec![encode_u_type(0x37, rd, imm)])
        }
        "auipc" => {
            need_ops(2)?;
            let rd = parse_rv_reg(&ops[0])?;
            let imm = eval_expr(&ops[1], constants, labels)? as u32;
            Ok(vec![encode_u_type(0x17, rd, imm)])
        }
        "j" | "jmp" => {
            need_ops(1)?;
            let target = eval_expr(&ops[0], constants, labels)? as i32;
            let offset = target - (pc as i32);
            Ok(vec![encode_j_type(0x6F, 0, offset)])
        }
        "jal" | "call" => {
            if ops.len() == 1 {
                let target = eval_expr(&ops[0], constants, labels)? as i32;
                let offset = target - (pc as i32);
                Ok(vec![encode_j_type(0x6F, 1, offset)])
            } else {
                need_ops(2)?;
                let rd = parse_rv_reg(&ops[0])?;
                let target = eval_expr(&ops[1], constants, labels)? as i32;
                let offset = target - (pc as i32);
                Ok(vec![encode_j_type(0x6F, rd, offset)])
            }
        }
        "jr" => {
            need_ops(1)?;
            let rs1 = parse_rv_reg(&ops[0])?;
            Ok(vec![encode_i_type(0x67, 0, 0x0, rs1, 0)])
        }
        "jalr" => {
            if ops.len() == 1 {
                let rs1 = parse_rv_reg(&ops[0])?;
                Ok(vec![encode_i_type(0x67, 1, 0x0, rs1, 0)])
            } else if ops.len() == 2 {
                let rd = parse_rv_reg(&ops[0])?;
                let (rs1, off) = parse_mem_operand(&ops[1], CpuArch::Rv32i, constants, labels)?;
                Ok(vec![encode_i_type(0x67, rd, 0x0, rs1 as u32, off as i32)])
            } else {
                need_ops(3)?;
                let rd = parse_rv_reg(&ops[0])?;
                let rs1 = parse_rv_reg(&ops[1])?;
                let off = eval_expr(&ops[2], constants, labels)? as i32;
                Ok(vec![encode_i_type(0x67, rd, 0x0, rs1, off)])
            }
        }
        "beq" | "bne" | "blt" | "bge" | "bltu" | "bgeu" | "ble" | "bgt" | "bleu" | "bgtu" => {
            need_ops(3)?;
            let mut rs1 = parse_rv_reg(&ops[0])?;
            let mut rs2 = parse_rv_reg(&ops[1])?;
            let target = eval_expr(&ops[2], constants, labels)? as i32;
            let offset = target - (pc as i32);
            let f3 = match mnem {
                "beq" => 0x0,
                "bne" => 0x1,
                "blt" => 0x4,
                "bge" => 0x5,
                "bltu" => 0x6,
                "bgeu" => 0x7,
                "ble" => {
                    std::mem::swap(&mut rs1, &mut rs2);
                    0x5 // bge rs2, rs1
                }
                "bgt" => {
                    std::mem::swap(&mut rs1, &mut rs2);
                    0x4 // blt rs2, rs1
                }
                "bleu" => {
                    std::mem::swap(&mut rs1, &mut rs2);
                    0x7 // bgeu rs2, rs1
                }
                "bgtu" => {
                    std::mem::swap(&mut rs1, &mut rs2);
                    0x6 // bltu rs2, rs1
                }
                _ => 0x0,
            };
            Ok(vec![encode_b_type(0x63, f3, rs1, rs2, offset)])
        }
        "beqz" | "jz" | "jeq" => {
            if ops.len() == 2 {
                let rs1 = parse_rv_reg(&ops[0])?;
                let target = eval_expr(&ops[1], constants, labels)? as i32;
                let offset = target - (pc as i32);
                Ok(vec![encode_b_type(0x63, 0x0, rs1, 0, offset)])
            } else {
                Err(format!("'{mnem}' em RV32I requer (rs1, label) ou use 'beq rs1, rs2, label'"))
            }
        }
        "bnez" | "jnz" | "jne" => {
            if ops.len() == 2 {
                let rs1 = parse_rv_reg(&ops[0])?;
                let target = eval_expr(&ops[1], constants, labels)? as i32;
                let offset = target - (pc as i32);
                Ok(vec![encode_b_type(0x63, 0x1, rs1, 0, offset)])
            } else {
                Err(format!("'{mnem}' em RV32I requer (rs1, label) ou use 'bne rs1, rs2, label'"))
            }
        }
        "lb" | "lh" | "lw" | "lbu" | "lhu" | "ldb" | "ldw" | "ld" => {
            let f3 = match mnem {
                "lb" => 0x0,
                "lh" | "ldw" => 0x1,
                "lw" => 0x2,
                "lbu" | "ldb" | "ld" => 0x4,
                "lhu" => 0x5,
                _ => 0x2,
            };
            if ops.len() == 2 {
                let rd = parse_rv_reg(&ops[0])?;
                let (rs1, off) = parse_mem_operand(&ops[1], CpuArch::Rv32i, constants, labels)?;
                Ok(vec![encode_i_type(0x03, rd, f3, rs1 as u32, off as i32)])
            } else if ops.len() == 3 {
                let rd = parse_rv_reg(&ops[0])?;
                let rs1 = parse_rv_reg(&ops[1])?;
                let off = eval_expr(&ops[2], constants, labels)? as i32;
                Ok(vec![encode_i_type(0x03, rd, f3, rs1, off)])
            } else {
                Err(format!("Instrução de leitura '{mnem}' requer 2 operandos: rd, offset(rs1)"))
            }
        }
        "sb" | "sh" | "sw" | "stb" | "stw" | "st" => {
            let f3 = match mnem {
                "sb" | "stb" | "st" => 0x0,
                "sh" | "stw" => 0x1,
                "sw" => 0x2,
                _ => 0x2,
            };
            if ops.len() == 2 {
                let rs2 = parse_rv_reg(&ops[0])?;
                let (rs1, off) = parse_mem_operand(&ops[1], CpuArch::Rv32i, constants, labels)?;
                Ok(vec![encode_s_type(0x23, f3, rs1 as u32, rs2, off as i32)])
            } else if ops.len() == 3 {
                let rs2 = parse_rv_reg(&ops[0])?;
                let rs1 = parse_rv_reg(&ops[1])?;
                let off = eval_expr(&ops[2], constants, labels)? as i32;
                Ok(vec![encode_s_type(0x23, f3, rs1, rs2, off)])
            } else {
                Err(format!("Instrução de escrita '{mnem}' requer 2 operandos: rs2, offset(rs1)"))
            }
        }
        "addi" | "subi" | "slti" | "sltiu" | "xori" | "ori" | "andi" | "slli" | "srli"
        | "srai" => {
            need_ops(3)?;
            let rd = parse_rv_reg(&ops[0])?;
            let rs1 = parse_rv_reg(&ops[1])?;
            let mut imm = eval_expr(&ops[2], constants, labels)? as i32;
            if mnem == "subi" {
                imm = -imm;
            }
            let inst = match mnem {
                "addi" | "subi" => encode_i_type(0x13, rd, 0x0, rs1, imm),
                "slti" => encode_i_type(0x13, rd, 0x2, rs1, imm),
                "sltiu" => encode_i_type(0x13, rd, 0x3, rs1, imm),
                "xori" => encode_i_type(0x13, rd, 0x4, rs1, imm),
                "ori" => encode_i_type(0x13, rd, 0x6, rs1, imm),
                "andi" => encode_i_type(0x13, rd, 0x7, rs1, imm),
                "slli" => encode_r_type(0x13, rd, 0x1, rs1, (imm & 31) as u32, 0x00),
                "srli" => encode_r_type(0x13, rd, 0x5, rs1, (imm & 31) as u32, 0x00),
                "srai" => encode_r_type(0x13, rd, 0x5, rs1, (imm & 31) as u32, 0x20),
                _ => unreachable!(),
            };
            Ok(vec![inst])
        }
        "add" | "sub" | "sll" | "slt" | "sltu" | "xor" | "srl" | "sra" | "or" | "and" | "mul"
        | "div" | "divu" | "rem" | "remu" | "shl" | "shr" => {
            need_ops(3)?;
            let rd = parse_rv_reg(&ops[0])?;
            let rs1 = parse_rv_reg(&ops[1])?;
            // Allow 3rd operand to be either a register or an immediate for convenience!
            if let Ok(rs2) = parse_rv_reg(&ops[2]) {
                let (f3, f7) = match mnem {
                    "add" => (0x0, 0x00),
                    "sub" => (0x0, 0x20),
                    "sll" | "shl" => (0x1, 0x00),
                    "slt" => (0x2, 0x00),
                    "sltu" => (0x3, 0x00),
                    "xor" => (0x4, 0x00),
                    "srl" | "shr" => (0x5, 0x00),
                    "sra" => (0x5, 0x20),
                    "or" => (0x6, 0x00),
                    "and" => (0x7, 0x00),
                    "mul" => (0x0, 0x01),
                    "div" => (0x4, 0x01),
                    "divu" => (0x5, 0x01),
                    "rem" => (0x6, 0x01),
                    "remu" => (0x7, 0x01),
                    _ => (0x0, 0x00),
                };
                Ok(vec![encode_r_type(0x33, rd, f3, rs1, rs2, f7)])
            } else {
                let mut imm = eval_expr(&ops[2], constants, labels)? as i32;
                if mnem == "sub" {
                    imm = -imm;
                }
                let inst = match mnem {
                    "add" | "sub" => encode_i_type(0x13, rd, 0x0, rs1, imm),
                    "xor" => encode_i_type(0x13, rd, 0x4, rs1, imm),
                    "or" => encode_i_type(0x13, rd, 0x6, rs1, imm),
                    "and" => encode_i_type(0x13, rd, 0x7, rs1, imm),
                    "sll" | "shl" => encode_r_type(0x13, rd, 0x1, rs1, (imm & 31) as u32, 0x00),
                    "srl" | "shr" => encode_r_type(0x13, rd, 0x5, rs1, (imm & 31) as u32, 0x00),
                    "sra" => encode_r_type(0x13, rd, 0x5, rs1, (imm & 31) as u32, 0x20),
                    _ => return Err(format!("Operando inválido para '{mnem}': '{}'", ops[2])),
                };
                Ok(vec![inst])
            }
        }
        _ => Err(format!("Instrução desconhecida para arquitetura RV32I: '{mnem}'")),
    }
}

fn pack_lc16(op: u8, rd: u8, rs1: u8, imm: u16) -> u32 {
    (op as u32)
        | (((rd & 0x0F) as u32) << 12)
        | (((rs1 & 0x0F) as u32) << 8)
        | ((imm as u32) << 16)
}

fn encode_lc16(
    mnem: &str,
    ops: &[String],
    _pc: u32,
    constants: &HashMap<String, i64>,
    labels: &HashMap<String, u32>,
) -> Result<Vec<u32>, String> {
    let need_ops = |n: usize| -> Result<(), String> {
        if ops.len() != n {
            Err(format!(
                "Instrução '{mnem}' espera {n} operando(s), mas recebeu {}",
                ops.len()
            ))
        } else {
            Ok(())
        }
    };

    match mnem {
        "nop" => Ok(vec![pack_lc16(LC16_OP_NOP, 0, 0, 0)]),
        "halt" | "ebreak" | "ecall" => Ok(vec![pack_lc16(LC16_OP_HALT, 0, 0, 0)]),
        "ret" => Ok(vec![pack_lc16(LC16_OP_RET, 0, 0, 0)]),
        "ldi" | "li" | "la" => {
            need_ops(2)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let imm = eval_expr(&ops[1], constants, labels)? as u16;
            Ok(vec![pack_lc16(LC16_OP_LDI, rd, 0, imm)])
        }
        "mov" | "mv" => {
            need_ops(2)?;
            let rd = parse_lc16_reg(&ops[0])?;
            if let Ok(rs1) = parse_lc16_reg(&ops[1]) {
                Ok(vec![pack_lc16(LC16_OP_MOV, rd, rs1, 0)])
            } else {
                let imm = eval_expr(&ops[1], constants, labels)? as u16;
                Ok(vec![pack_lc16(LC16_OP_LDI, rd, 0, imm)])
            }
        }
        "neg" => {
            need_ops(2)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let rs1 = parse_lc16_reg(&ops[1])?;
            Ok(vec![pack_lc16(LC16_OP_NEG, rd, rs1, 0)])
        }
        "not" => {
            need_ops(2)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let rs1 = parse_lc16_reg(&ops[1])?;
            Ok(vec![pack_lc16(LC16_OP_NOT, rd, rs1, 0)])
        }
        "push" => {
            need_ops(1)?;
            let rd = parse_lc16_reg(&ops[0])?;
            Ok(vec![pack_lc16(LC16_OP_PUSH, rd, 0, 0)])
        }
        "pop" => {
            need_ops(1)?;
            let rd = parse_lc16_reg(&ops[0])?;
            Ok(vec![pack_lc16(LC16_OP_POP, rd, 0, 0)])
        }
        "cmp" | "cmpi" => {
            need_ops(2)?;
            let rd = parse_lc16_reg(&ops[0])?;
            if mnem == "cmp" {
                if let Ok(rs1) = parse_lc16_reg(&ops[1]) {
                    return Ok(vec![pack_lc16(LC16_OP_CMP, rd, rs1, 0)]);
                }
            }
            let imm = eval_expr(&ops[1], constants, labels)? as u16;
            Ok(vec![pack_lc16(LC16_OP_CMPI, rd, 0, imm)])
        }
        "add" | "sub" | "and" | "or" | "xor" | "mul" => {
            if ops.len() == 2 {
                let rd = parse_lc16_reg(&ops[0])?;
                if let Ok(rs2) = parse_lc16_reg(&ops[1]) {
                    let op = match mnem {
                        "add" => LC16_OP_ADD,
                        "sub" => LC16_OP_SUB,
                        "and" => LC16_OP_AND,
                        "or" => LC16_OP_OR,
                        "xor" => LC16_OP_XOR,
                        "mul" => LC16_OP_MUL,
                        _ => LC16_OP_ADD,
                    };
                    return Ok(vec![pack_lc16(op, rd, rd, rs2 as u16)]);
                } else {
                    let imm = eval_expr(&ops[1], constants, labels)? as u16;
                    let op = match mnem {
                        "add" => LC16_OP_ADDI,
                        "sub" => LC16_OP_SUBI,
                        "and" => LC16_OP_ANDI,
                        "or" => LC16_OP_ORI,
                        "xor" => LC16_OP_XORI,
                        _ => LC16_OP_ADDI,
                    };
                    return Ok(vec![pack_lc16(op, rd, rd, imm)]);
                }
            }
            need_ops(3)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let rs1 = parse_lc16_reg(&ops[1])?;
            if let Ok(rs2) = parse_lc16_reg(&ops[2]) {
                let op = match mnem {
                    "add" => LC16_OP_ADD,
                    "sub" => LC16_OP_SUB,
                    "and" => LC16_OP_AND,
                    "or" => LC16_OP_OR,
                    "xor" => LC16_OP_XOR,
                    "mul" => LC16_OP_MUL,
                    _ => LC16_OP_ADD,
                };
                Ok(vec![pack_lc16(op, rd, rs1, rs2 as u16)])
            } else {
                let imm = eval_expr(&ops[2], constants, labels)? as u16;
                let op = match mnem {
                    "add" => LC16_OP_ADDI,
                    "sub" => LC16_OP_SUBI,
                    "and" => LC16_OP_ANDI,
                    "or" => LC16_OP_ORI,
                    "xor" => LC16_OP_XORI,
                    _ => LC16_OP_ADDI,
                };
                Ok(vec![pack_lc16(op, rd, rs1, imm)])
            }
        }
        "addi" | "subi" | "andi" | "ori" | "xori" => {
            need_ops(3)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let rs1 = parse_lc16_reg(&ops[1])?;
            let imm = eval_expr(&ops[2], constants, labels)? as u16;
            let op = match mnem {
                "addi" => LC16_OP_ADDI,
                "subi" => LC16_OP_SUBI,
                "andi" => LC16_OP_ANDI,
                "ori" => LC16_OP_ORI,
                "xori" => LC16_OP_XORI,
                _ => LC16_OP_ADDI,
            };
            Ok(vec![pack_lc16(op, rd, rs1, imm)])
        }
        "shl" | "sll" | "slli" | "shr" | "srl" | "srli" | "sra" | "srai" => {
            if ops.len() == 2 {
                let rd = parse_lc16_reg(&ops[0])?;
                let sh = eval_expr(&ops[1], constants, labels)? as u16;
                let op = if mnem.starts_with("shl") || mnem.starts_with("sll") {
                    LC16_OP_SHL
                } else if mnem.starts_with("sra") {
                    LC16_OP_SRA
                } else {
                    LC16_OP_SHR
                };
                return Ok(vec![pack_lc16(op, rd, rd, sh)]);
            }
            need_ops(3)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let rs1 = parse_lc16_reg(&ops[1])?;
            let sh = eval_expr(&ops[2], constants, labels)? as u16;
            let op = if mnem.starts_with("shl") || mnem.starts_with("sll") {
                LC16_OP_SHL
            } else if mnem.starts_with("sra") {
                LC16_OP_SRA
            } else {
                LC16_OP_SHR
            };
            Ok(vec![pack_lc16(op, rd, rs1, sh)])
        }
        "ldb" | "lb" | "lbu" | "ldw" | "lh" | "lhu" | "lw" | "ld" => {
            need_ops(2)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let (rs1, off) = parse_mem_operand(&ops[1], CpuArch::Lc16, constants, labels)?;
            let op = match mnem {
                "ldw" | "lh" | "lhu" | "lw" => LC16_OP_LDW,
                _ => LC16_OP_LDB,
            };
            Ok(vec![pack_lc16(op, rd, rs1, off as u16)])
        }
        "stb" | "sb" | "stw" | "sh" | "sw" | "st" => {
            need_ops(2)?;
            let rd = parse_lc16_reg(&ops[0])?;
            let (rs1, off) = parse_mem_operand(&ops[1], CpuArch::Lc16, constants, labels)?;
            let op = match mnem {
                "stw" | "sh" | "sw" => LC16_OP_STW,
                _ => LC16_OP_STB,
            };
            Ok(vec![pack_lc16(op, rd, rs1, off as u16)])
        }
        "jmp" | "j" => {
            need_ops(1)?;
            let target = eval_expr(&ops[0], constants, labels)? as u16;
            Ok(vec![pack_lc16(LC16_OP_JMP, 0, 0, target)])
        }
        "jeq" | "jz" | "jne" | "jnz" | "jlt" | "jge" | "jgt" | "jle" => {
            need_ops(1)?;
            let target = eval_expr(&ops[0], constants, labels)? as u16;
            let op = match mnem {
                "jeq" | "jz" => LC16_OP_JEQ,
                "jne" | "jnz" => LC16_OP_JNE,
                "jlt" => LC16_OP_JLT,
                "jge" => LC16_OP_JGE,
                "jgt" => LC16_OP_JGT,
                "jle" => LC16_OP_JLE,
                _ => LC16_OP_JMP,
            };
            Ok(vec![pack_lc16(op, 0, 0, target)])
        }
        "beq" | "bne" | "blt" | "bge" | "ble" | "bgt" => {
            need_ops(3)?;
            let mut r1 = parse_lc16_reg(&ops[0])?;
            let mut r2 = parse_lc16_reg(&ops[1])?;
            let target = eval_expr(&ops[2], constants, labels)? as u16;
            let op = match mnem {
                "beq" => LC16_OP_BEQ,
                "bne" => LC16_OP_BNE,
                "blt" => LC16_OP_BLT,
                "bge" => LC16_OP_BGE,
                "ble" => {
                    std::mem::swap(&mut r1, &mut r2);
                    LC16_OP_BGE
                }
                "bgt" => {
                    std::mem::swap(&mut r1, &mut r2);
                    LC16_OP_BLT
                }
                _ => LC16_OP_BEQ,
            };
            Ok(vec![pack_lc16(op, r1, r2, target)])
        }
        "beqz" => {
            need_ops(2)?;
            let r1 = parse_lc16_reg(&ops[0])?;
            let target = eval_expr(&ops[1], constants, labels)? as u16;
            Ok(vec![pack_lc16(LC16_OP_BEQ, r1, 0, target)])
        }
        "bnez" => {
            need_ops(2)?;
            let r1 = parse_lc16_reg(&ops[0])?;
            let target = eval_expr(&ops[1], constants, labels)? as u16;
            Ok(vec![pack_lc16(LC16_OP_BNE, r1, 0, target)])
        }
        "call" | "jal" => {
            let target_str = if ops.len() == 2 { &ops[1] } else { &ops[0] };
            let target = eval_expr(target_str, constants, labels)? as u16;
            Ok(vec![pack_lc16(LC16_OP_CALL, 0, 0, target)])
        }
        _ => Err(format!("Instrução desconhecida para arquitetura LC16: '{mnem}'")),
    }
}

fn encode_min8(
    mnem: &str,
    ops: &[String],
    pc: u32,
    constants: &HashMap<String, i64>,
    labels: &HashMap<String, u32>,
) -> Result<Vec<u32>, String> {
    match mnem {
        "cls" => Ok(vec![pack_lc16(MIN8_OP_CLS, 0, 0, 0)]),
        "seti" => {
            if ops.is_empty() {
                return Err("SETI requer um endereço de 16 bits".to_string());
            }
            let target_str = if ops.len() == 2 { &ops[1] } else { &ops[0] };
            let imm = eval_expr(target_str, constants, labels)? as u16;
            Ok(vec![pack_lc16(MIN8_OP_SETI, 0, 0, imm)])
        }
        "pset" | "drw" => {
            if ops.len() < 2 {
                return Err("PSET requer pelo menos (Vx, Vy [, cor])".to_string());
            }
            let rx = parse_lc16_reg(&ops[0])? & 7;
            let ry = parse_lc16_reg(&ops[1])? & 7;
            let col = if ops.len() >= 3 {
                eval_expr(&ops[2], constants, labels)? as u16
            } else {
                3
            };
            Ok(vec![pack_lc16(MIN8_OP_PSET, rx, ry, col)])
        }
        _ => encode_lc16(mnem, ops, pc, constants, labels),
    }
}
