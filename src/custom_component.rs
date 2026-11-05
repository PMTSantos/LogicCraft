use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::library::{GateDef, GateShapeStyle, PinDef};
use crate::mechanics::LogicState;

/// Easy-to-write, human- and AI-friendly specification for a Custom Component.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CustomComponentSpec {
    /// Optional unique ID (e.g., "CUSTOM_FSUB"). If omitted, derived automatically from `name`.
    #[serde(default)]
    pub id: String,
    /// Display name in the Gate Library Palette (e.g., "Full Subtractor (1-Bit)").
    pub name: String,
    /// Short title displayed inside the IC body on the canvas (e.g., "FSUB").
    #[serde(default)]
    pub chip_label: String,
    /// Optional description of the component's behavior.
    #[serde(default)]
    pub description: String,
    /// Ordered list of input pin names (placed on the left edge, or bottom for reset).
    pub inputs: Vec<String>,
    /// Ordered list of output pin names (placed on the right edge).
    pub outputs: Vec<String>,
    /// Optional internal state bit names for sequential/clocked components (e.g., `["Q0", "Q1"]`).
    #[serde(default)]
    pub state_bits: Vec<String>,
    /// Optional clock input pin name (e.g., `"CLK"`). When specified, `state_equations` update on clock edge.
    #[serde(default)]
    pub clock_pin: Option<String>,
    /// `"rising"` (default) or `"falling"` edge trigger for `clock_pin`.
    #[serde(default)]
    pub clock_edge: Option<String>,
    /// Optional active-high asynchronous reset pin name (e.g., `"RST"` or `"CLR"`).
    #[serde(default)]
    pub reset_pin: Option<String>,
    /// Output boolean equations mapping each output pin name to a boolean expression.
    /// Supports `!`, `~`, `'` (NOT), `&`, `*`, `AND` (AND), `|`, `+`, `OR` (OR), `^`, `XOR` (XOR),
    /// `NAND`, `NOR`, `XNOR`, parentheses `(...)`, and constants `0` / `1`.
    #[serde(default)]
    pub equations: HashMap<String, String>,
    /// Next-state boolean equations mapping each state bit in `state_bits` to its next-state expression.
    #[serde(default)]
    pub state_equations: HashMap<String, String>,
    /// Optional truth table mapping input bit patterns (e.g., `"011"` or `"1X0"`) to output bit strings (e.g., `"10"`).
    /// Output bit order matches `outputs`. If an output is not defined in `equations`, `truth_table` is used.
    #[serde(default)]
    pub truth_table: HashMap<String, String>,
}

impl CustomComponentSpec {
    /// Normalizes the spec and ensures `id` and `chip_label` are populated, validating pins.
    pub fn normalized(mut self) -> Result<Self, String> {
        self.name = self.name.trim().to_string();
        if self.name.is_empty() {
            return Err("Component 'name' cannot be empty.".to_string());
        }
        self.inputs = self
            .inputs
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        self.outputs = self
            .outputs
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if self.inputs.is_empty() && self.outputs.is_empty() {
            return Err("Component must have at least one input or output pin.".to_string());
        }

        if self.id.trim().is_empty() {
            let slug: String = self
                .name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_uppercase()
                    } else {
                        '_'
                    }
                })
                .collect();
            let clean = slug.trim_matches('_').to_string();
            self.id = format!("CUSTOM_{}", if clean.is_empty() { "IC" } else { &clean });
        } else {
            self.id = self.id.trim().to_uppercase();
            if !self.id.starts_with("CUSTOM_") {
                self.id = format!("CUSTOM_{}", self.id);
            }
        }

        if self.chip_label.trim().is_empty() {
            let short: String = self
                .name
                .split_whitespace()
                .next()
                .unwrap_or("CUSTOM")
                .chars()
                .take(7)
                .collect();
            self.chip_label = short.to_uppercase();
        } else {
            self.chip_label = self.chip_label.trim().to_string();
        }

        Ok(self)
    }

    /// Converts this `CustomComponentSpec` into a `GateDef` for the palette and canvas renderer.
    pub fn to_gate_def(&self) -> GateDef {
        let max_side_pins = self.inputs.len().max(self.outputs.len()).max(2);
        let half_h = ((max_side_pins as f32) * 0.55 + 1.0).max(2.5);
        let half_w = 3.5_f32;

        let mut in_pins = Vec::with_capacity(self.inputs.len());
        let n_in = self.inputs.len();
        for (i, pin_name) in self.inputs.iter().enumerate() {
            let y = if n_in == 1 {
                0.0
            } else {
                let span = (half_h - 0.8).max(0.5) * 2.0;
                (half_h - 0.8) - (i as f32 / (n_in - 1) as f32) * span
            };
            let y_snapped = (y * 2.0).round() * 0.5;
            in_pins.push(PinDef {
                name: pin_name.clone(),
                pos: [-half_w, y_snapped.clamp(-half_h + 0.5, half_h - 0.5)],
                is_output: false,
                inverted: false,
            });
        }

        let mut out_pins = Vec::with_capacity(self.outputs.len());
        let n_out = self.outputs.len();
        for (i, pin_name) in self.outputs.iter().enumerate() {
            let y = if n_out == 1 {
                0.0
            } else {
                let span = (half_h - 0.8).max(0.5) * 2.0;
                (half_h - 0.8) - (i as f32 / (n_out - 1) as f32) * span
            };
            let y_snapped = (y * 2.0).round() * 0.5;
            out_pins.push(PinDef {
                name: pin_name.clone(),
                pos: [half_w, y_snapped.clamp(-half_h + 0.5, half_h - 0.5)],
                is_output: true,
                inverted: false,
            });
        }

        let mut default_gparams = HashMap::new();
        default_gparams.insert("CHIP_LABEL".into(), self.chip_label.clone());

        GateDef {
            type_name: self.id.clone(),
            caption: self.name.clone(),
            category: "11 - Custom Components".to_string(),
            logic_type: "CUSTOM_IC".to_string(),
            inputs: in_pins,
            outputs: out_pins,
            half_size: [half_w, half_h],
            shape_style: GateShapeStyle::CustomIc,
            out_inverted: false,
            in_inverted: false,
            default_gparams,
            default_lparams: HashMap::new(),
            custom_lines: Vec::new(),
        }
    }

    /// Evaluates this custom component for a given gate instance.
    pub fn evaluate(
        &self,
        pin_states: &mut HashMap<String, LogicState>,
        lparams: &mut HashMap<String, String>,
        prev_clock: &mut LogicState,
    ) {
        // 1. Read input pin values as booleans (HiZ defaults to false/0)
        let mut env: HashMap<String, bool> = HashMap::new();
        for inp in &self.inputs {
            let val = matches!(
                pin_states.get(inp).copied().unwrap_or(LogicState::Zero),
                LogicState::One
            );
            env.insert(inp.clone(), val);
            env.insert(inp.to_uppercase(), val);
        }

        // 2. Load current sequential state bits from `lparams`
        for sb in &self.state_bits {
            let key = format!("STATE_{sb}");
            let cur = lparams.get(&key).map(|v| v == "1").unwrap_or(false);
            env.insert(sb.clone(), cur);
            env.insert(sb.to_uppercase(), cur);
        }

        // 3. Handle asynchronous reset or clock edge for sequential state bits
        if !self.state_bits.is_empty() {
            let reset_active = self
                .reset_pin
                .as_ref()
                .and_then(|rp| env.get(rp).or_else(|| env.get(&rp.to_uppercase())).copied())
                .unwrap_or(false);

            if reset_active {
                for sb in &self.state_bits {
                    lparams.insert(format!("STATE_{sb}"), "0".to_string());
                    env.insert(sb.clone(), false);
                    env.insert(sb.to_uppercase(), false);
                }
            } else if let Some(ref clk_name) = self.clock_pin {
                let clk_state = pin_states
                    .get(clk_name)
                    .copied()
                    .unwrap_or(LogicState::Zero);
                let falling = self
                    .clock_edge
                    .as_deref()
                    .map(|e| e.eq_ignore_ascii_case("falling"))
                    .unwrap_or(false);
                let triggered = if falling {
                    *prev_clock == LogicState::One && clk_state == LogicState::Zero
                } else {
                    *prev_clock == LogicState::Zero && clk_state == LogicState::One
                };
                *prev_clock = clk_state;

                if triggered {
                    let mut next_vals = Vec::new();
                    for sb in &self.state_bits {
                        if let Some(expr) = self
                            .state_equations
                            .get(sb)
                            .or_else(|| self.state_equations.get(&sb.to_uppercase()))
                        {
                            let v = eval_boolean_expr(expr, &env);
                            next_vals.push((sb.clone(), v));
                        }
                    }
                    for (sb, v) in next_vals {
                        lparams.insert(
                            format!("STATE_{sb}"),
                            if v { "1".to_string() } else { "0".to_string() },
                        );
                        env.insert(sb.clone(), v);
                        env.insert(sb.to_uppercase(), v);
                    }
                }
            }
        }

        // 4. Evaluate truth table if non-empty
        let mut tt_outputs: HashMap<String, bool> = HashMap::new();
        if !self.truth_table.is_empty() {
            let input_pattern: String = self
                .inputs
                .iter()
                .map(|inp| if *env.get(inp).unwrap_or(&false) { '1' } else { '0' })
                .collect();

            // Check exact match first, then wildcard match ('X', 'x', '-', '?')
            let matched_out = self
                .truth_table
                .get(&input_pattern)
                .cloned()
                .or_else(|| {
                    self.truth_table
                        .iter()
                        .find(|(pat, _)| pattern_matches(pat, &input_pattern))
                        .map(|(_, out)| out.clone())
                });

            if let Some(out_bits) = matched_out {
                let clean_bits: Vec<char> = out_bits
                    .chars()
                    .filter(|c| *c == '0' || *c == '1' || *c == 'X' || *c == 'x')
                    .collect();
                for ( idx, out_name) in self.outputs.iter().enumerate() {
                    let b = clean_bits.get(idx).copied().unwrap_or('0') == '1';
                    tt_outputs.insert(out_name.clone(), b);
                }
            }
        }

        // 5. Evaluate output equations (can also reference previously computed outputs)
        for out_name in &self.outputs {
            let val = if let Some(expr) = self
                .equations
                .get(out_name)
                .or_else(|| self.equations.get(&out_name.to_uppercase()))
            {
                if !expr.trim().is_empty() {
                    eval_boolean_expr(expr, &env)
                } else {
                    tt_outputs.get(out_name).copied().unwrap_or(false)
                }
            } else {
                tt_outputs.get(out_name).copied().unwrap_or(false)
            };
            env.insert(out_name.clone(), val);
            env.insert(out_name.to_uppercase(), val);
            pin_states.insert(
                out_name.clone(),
                if val {
                    LogicState::One
                } else {
                    LogicState::Zero
                },
            );
        }
    }
}

fn pattern_matches(pattern: &str, actual: &str) -> bool {
    let p_chars: Vec<char> = pattern
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_')
        .collect();
    let a_chars: Vec<char> = actual.chars().collect();
    if p_chars.len() != a_chars.len() {
        return false;
    }
    p_chars.into_iter().zip(a_chars).all(|(p, a)| {
        p == a || p == 'X' || p == 'x' || p == '-' || p == '?'
    })
}

#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Ident(String),
    Const(bool),
    Not,
    PostfixNot,
    And,
    Nand,
    Or,
    Nor,
    Xor,
    Xnor,
    LParen,
    RParen,
}

/// Evaluates a boolean expression string against `env`.
/// Supports:
/// - Variables (case-insensitive fallback)
/// - Constants: `0`, `1`, `false`, `true`, `LOW`, `HIGH`
/// - Prefix NOT: `!A`, `~A`, `NOT A`
/// - Postfix NOT: `A'`
/// - AND: `&`, `&&`, `*`, `AND`
/// - NAND: `NAND`
/// - OR: `|`, `||`, `+`, `OR`
/// - NOR: `NOR`
/// - XOR: `^`, `XOR`, `!=`
/// - XNOR: `XNOR`, `==`, `=`
/// - Parentheses: `(...)`
pub fn eval_boolean_expr(expr: &str, env: &HashMap<String, bool>) -> bool {
    let tokens = tokenize_expr(expr);
    if tokens.is_empty() {
        return false;
    }
    let mut pos = 0usize;
    parse_or(&tokens, &mut pos, env)
}

fn tokenize_expr(expr: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let chars: Vec<char> = expr.chars().collect();
    let mut i = 0usize;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        match c {
            '(' => {
                out.push(Tok::LParen);
                i += 1;
            }
            ')' => {
                out.push(Tok::RParen);
                i += 1;
            }
            '~' => {
                out.push(Tok::Not);
                i += 1;
            }
            '\'' => {
                out.push(Tok::PostfixNot);
                i += 1;
            }
            '!' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    out.push(Tok::Xor);
                    i += 2;
                } else {
                    out.push(Tok::Not);
                    i += 1;
                }
            }
            '&' => {
                if i + 1 < chars.len() && chars[i + 1] == '&' {
                    i += 2;
                } else {
                    i += 1;
                }
                out.push(Tok::And);
            }
            '*' | '·' => {
                out.push(Tok::And);
                i += 1;
            }
            '|' => {
                if i + 1 < chars.len() && chars[i + 1] == '|' {
                    i += 2;
                } else {
                    i += 1;
                }
                out.push(Tok::Or);
            }
            '+' => {
                out.push(Tok::Or);
                i += 1;
            }
            '^' | '⊕' => {
                out.push(Tok::Xor);
                i += 1;
            }
            '=' => {
                if i + 1 < chars.len() && chars[i + 1] == '=' {
                    i += 2;
                } else {
                    i += 1;
                }
                out.push(Tok::Xnor);
            }
            '0' => {
                out.push(Tok::Const(false));
                i += 1;
            }
            '1' => {
                out.push(Tok::Const(true));
                i += 1;
            }
            _ if c.is_ascii_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();
                match word.to_ascii_uppercase().as_str() {
                    "NOT" => out.push(Tok::Not),
                    "AND" => out.push(Tok::And),
                    "NAND" => out.push(Tok::Nand),
                    "OR" => out.push(Tok::Or),
                    "NOR" => out.push(Tok::Nor),
                    "XOR" => out.push(Tok::Xor),
                    "XNOR" | "NXOR" => out.push(Tok::Xnor),
                    "TRUE" | "HIGH" => out.push(Tok::Const(true)),
                    "FALSE" | "LOW" => out.push(Tok::Const(false)),
                    _ => out.push(Tok::Ident(word)),
                }
            }
            _ => {
                i += 1;
            }
        }
    }
    out
}

fn parse_or(tokens: &[Tok], pos: &mut usize, env: &HashMap<String, bool>) -> bool {
    let mut lhs = parse_xor(tokens, pos, env);
    while *pos < tokens.len() {
        match tokens[*pos] {
            Tok::Or => {
                *pos += 1;
                let rhs = parse_xor(tokens, pos, env);
                lhs = lhs || rhs;
            }
            Tok::Nor => {
                *pos += 1;
                let rhs = parse_xor(tokens, pos, env);
                lhs = !(lhs || rhs);
            }
            _ => break,
        }
    }
    lhs
}

fn parse_xor(tokens: &[Tok], pos: &mut usize, env: &HashMap<String, bool>) -> bool {
    let mut lhs = parse_and(tokens, pos, env);
    while *pos < tokens.len() {
        match tokens[*pos] {
            Tok::Xor => {
                *pos += 1;
                let rhs = parse_and(tokens, pos, env);
                lhs ^= rhs;
            }
            Tok::Xnor => {
                *pos += 1;
                let rhs = parse_and(tokens, pos, env);
                lhs = !(lhs ^ rhs);
            }
            _ => break,
        }
    }
    lhs
}

fn parse_and(tokens: &[Tok], pos: &mut usize, env: &HashMap<String, bool>) -> bool {
    let mut lhs = parse_unary(tokens, pos, env);
    while *pos < tokens.len() {
        match tokens[*pos] {
            Tok::And => {
                *pos += 1;
                let rhs = parse_unary(tokens, pos, env);
                lhs = lhs && rhs;
            }
            Tok::Nand => {
                *pos += 1;
                let rhs = parse_unary(tokens, pos, env);
                lhs = !(lhs && rhs);
            }
            // Support implicit AND juxtaposition like `A B` or `(A+B)(C+D)`
            Tok::Ident(_) | Tok::LParen | Tok::Not => {
                let rhs = parse_unary(tokens, pos, env);
                lhs = lhs && rhs;
            }
            _ => break,
        }
    }
    lhs
}

fn parse_unary(tokens: &[Tok], pos: &mut usize, env: &HashMap<String, bool>) -> bool {
    if *pos >= tokens.len() {
        return false;
    }
    if tokens[*pos] == Tok::Not {
        *pos += 1;
        return !parse_unary(tokens, pos, env);
    }
    let mut val = parse_primary(tokens, pos, env);
    while *pos < tokens.len() && tokens[*pos] == Tok::PostfixNot {
        *pos += 1;
        val = !val;
    }
    val
}

fn parse_primary(tokens: &[Tok], pos: &mut usize, env: &HashMap<String, bool>) -> bool {
    if *pos >= tokens.len() {
        return false;
    }
    match &tokens[*pos] {
        Tok::Const(b) => {
            let v = *b;
            *pos += 1;
            v
        }
        Tok::Ident(name) => {
            let v = env
                .get(name)
                .or_else(|| env.get(&name.to_uppercase()))
                .copied()
                .unwrap_or(false);
            *pos += 1;
            v
        }
        Tok::LParen => {
            *pos += 1;
            let v = parse_or(tokens, pos, env);
            if *pos < tokens.len() && tokens[*pos] == Tok::RParen {
                *pos += 1;
            }
            v
        }
        _ => {
            *pos += 1;
            false
        }
    }
}

/// Returns a ready-to-copy prompt that a user can paste into any AI assistant (ChatGPT, Claude, Gemini)
/// to generate a valid Custom Component JSON for LogicCraft.
pub fn ai_prompt_template() -> &'static str {
    r#"Create a custom digital logic component in JSON format for LogicCraft.
Return ONLY valid JSON matching this schema (no extra commentary needed):

{
  "id": "CUSTOM_MY_CHIP",
  "name": "Human Readable Component Name",
  "chip_label": "SHORT",
  "description": "Brief explanation of what this component does",
  "inputs": ["A", "B", "CIN"],
  "outputs": ["Y", "COUT"],
  "state_bits": [],
  "clock_pin": null,
  "clock_edge": "rising",
  "reset_pin": null,
  "equations": {
    "Y": "A ^ B ^ CIN",
    "COUT": "(A & B) | (CIN & (A ^ B))"
  },
  "state_equations": {},
  "truth_table": {}
}

Rules:
1. For combinational logic, you can either fill `"equations"` (using `!`, `&`, `|`, `^`, `~`, `'`, parentheses, `0`, `1`) OR fill `"truth_table"` mapping input bit strings (in the exact order of `"inputs"`, e.g. `"011"`) to output bit strings (in the exact order of `"outputs"`, e.g. `"01"`).
2. For sequential/clocked logic (flip-flops, counters, FSMs), list internal register bits in `"state_bits"`, set `"clock_pin"` (and optionally `"reset_pin"`), put next-state expressions in `"state_equations"`, and put output expressions in `"equations"`.

Component I want you to create: [DESCRIBE YOUR COMPONENT HERE]"#
}

/// Built-in starter templates so users can inspect, test, or customize examples immediately.
pub fn builtin_custom_templates() -> Vec<CustomComponentSpec> {
    vec![
        CustomComponentSpec {
            id: "CUSTOM_FULL_SUBTRACTOR".into(),
            name: "1-Bit Full Subtractor".into(),
            chip_label: "FSUB".into(),
            description: "Computes DIFF = A - B - BIN and borrow output BOUT.".into(),
            inputs: vec!["A".into(), "B".into(), "BIN".into()],
            outputs: vec!["DIFF".into(), "BOUT".into()],
            state_bits: Vec::new(),
            clock_pin: None,
            clock_edge: None,
            reset_pin: None,
            equations: [
                ("DIFF".into(), "A ^ B ^ BIN".into()),
                ("BOUT".into(), "(!A & B) | (!(A ^ B) & BIN)".into()),
            ]
            .into_iter()
            .collect(),
            state_equations: HashMap::new(),
            truth_table: HashMap::new(),
        },
        CustomComponentSpec {
            id: "CUSTOM_MAJORITY_3".into(),
            name: "3-Input Majority Voter".into(),
            chip_label: "MAJ3".into(),
            description: "Outputs 1 when two or more inputs (A, B, C) are 1.".into(),
            inputs: vec!["A".into(), "B".into(), "C".into()],
            outputs: vec!["MAJ".into(), "PARITY".into()],
            state_bits: Vec::new(),
            clock_pin: None,
            clock_edge: None,
            reset_pin: None,
            equations: [
                ("MAJ".into(), "(A & B) | (B & C) | (A & C)".into()),
                ("PARITY".into(), "A ^ B ^ C".into()),
            ]
            .into_iter()
            .collect(),
            state_equations: HashMap::new(),
            truth_table: HashMap::new(),
        },
        CustomComponentSpec {
            id: "CUSTOM_TFF".into(),
            name: "T Flip-Flop (Toggle with Enable & Reset)".into(),
            chip_label: "T-FF".into(),
            description: "Toggles Q on rising CLK edge when T=1 and EN=1; async active-high RST.".into(),
            inputs: vec!["T".into(), "EN".into(), "CLK".into(), "RST".into()],
            outputs: vec!["Q".into(), "Q_BAR".into()],
            state_bits: vec!["Q_REG".into()],
            clock_pin: Some("CLK".into()),
            clock_edge: Some("rising".into()),
            reset_pin: Some("RST".into()),
            equations: [
                ("Q".into(), "Q_REG".into()),
                ("Q_BAR".into(), "!Q_REG".into()),
            ]
            .into_iter()
            .collect(),
            state_equations: [(
                "Q_REG".into(),
                "(EN & (T ^ Q_REG)) | (!EN & Q_REG)".into(),
            )]
            .into_iter()
            .collect(),
            truth_table: HashMap::new(),
        },
        CustomComponentSpec {
            id: "CUSTOM_2BIT_ALU".into(),
            name: "Simple 1-Bit ALU (ADD/AND/OR/XOR)".into(),
            chip_label: "ALU1".into(),
            description: "OP1..OP0 selects operation: 00=AND, 01=OR, 10=XOR, 11=ADD(with CIN).".into(),
            inputs: vec![
                "A".into(),
                "B".into(),
                "CIN".into(),
                "OP1".into(),
                "OP0".into(),
            ],
            outputs: vec!["RES".into(), "COUT".into()],
            state_bits: Vec::new(),
            clock_pin: None,
            clock_edge: None,
            reset_pin: None,
            equations: [
                (
                    "RES".into(),
                    "(!OP1 & !OP0 & (A & B)) | (!OP1 & OP0 & (A | B)) | (OP1 & !OP0 & (A ^ B)) | (OP1 & OP0 & (A ^ B ^ CIN))".into(),
                ),
                (
                    "COUT".into(),
                    "OP1 & OP0 & ((A & B) | (CIN & (A ^ B)))".into(),
                ),
            ]
            .into_iter()
            .collect(),
            state_equations: HashMap::new(),
            truth_table: HashMap::new(),
        },
    ]
}
