use std::collections::{HashMap, VecDeque};

use crate::kmap::TruthTableKMap;
use crate::library::{GateDef, GateLibrary};
use crate::mechanics::{
    LogicState, DEFAULT_CLOCK_HALF_CYCLE, DEFAULT_GATE_DELAY, DEFAULT_VIEW_ZOOM, GRID_SPACING,
    OSCOPE_TRACE_LENGTH,
};

#[derive(Debug, Clone, PartialEq)]
pub struct GateInstance {
    pub id: u32,
    pub type_name: String,
    pub pos: [f32; 2], // World coordinates (+X right, +Y up)
    pub angle_deg: f32, // 0.0, 90.0, 180.0, 270.0
    pub gparams: HashMap<String, String>,
    pub lparams: HashMap<String, String>,
    pub pin_states: HashMap<String, LogicState>,
    pub prev_clock: LogicState,
    pub pulse_remaining: u32,
    pub clock_counter: u32,
    pub memory: HashMap<u16, u16>,
    pub selected: bool,
}

impl GateInstance {
    pub fn new(id: u32, def: &GateDef, pos: [f32; 2]) -> Self {
        let mut pin_states = HashMap::new();
        for p in &def.inputs {
            pin_states.insert(p.name.clone(), LogicState::HiZ);
        }
        for p in &def.outputs {
            pin_states.insert(p.name.clone(), LogicState::Zero);
        }
        let mut memory = HashMap::new();
        if def.logic_type == "RAM" {
            for addr in 0..16u16 {
                memory.insert(addr, (addr.wrapping_mul(3) ^ 0x5) & 0xFF);
            }
        }
        Self {
            id,
            type_name: def.type_name.clone(),
            pos,
            angle_deg: 0.0,
            gparams: def.default_gparams.clone(),
            lparams: def.default_lparams.clone(),
            pin_states,
            prev_clock: LogicState::Zero,
            pulse_remaining: 0,
            clock_counter: 0,
            memory,
            selected: false,
        }
    }

    pub fn pin_world_pos(&self, local_pos: [f32; 2]) -> [f32; 2] {
        let rad = self.angle_deg.to_radians();
        let (s, c) = rad.sin_cos();
        let rx = local_pos[0] * c - local_pos[1] * s;
        let ry = local_pos[0] * s + local_pos[1] * c;
        [self.pos[0] + rx, self.pos[1] + ry]
    }

    /// Returns `true` if this `From` / `To` connector tag is scoped to its own page only (`"LOCAL"`),
    /// or `false` if it is shared across all pages in the file (`"SHARED"`, default).
    pub fn is_tag_local(&self) -> bool {
        self.lparams
            .get("TAG_SCOPE")
            .map(|s| s.eq_ignore_ascii_case("LOCAL"))
            .unwrap_or(false)
    }

    pub fn set_tag_local(&mut self, local: bool) {
        self.lparams.insert(
            "TAG_SCOPE".into(),
            if local {
                "LOCAL".into()
            } else {
                "SHARED".into()
            },
        );
    }

    /// Returns `true` only if this component is an actual RAM or ROM memory IC (`_RAM_` / `_ROM_`).
    pub fn is_memory_device(&self) -> bool {
        self.type_name.contains("_RAM_") || self.type_name.contains("_ROM_")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PinRef {
    pub gate_id: u32,
    pub pin_name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WireInstance {
    pub id: u32,
    pub endpoints: Vec<PinRef>,
    pub points: Vec<[f32; 2]>, // Polyline vertices in world space
    pub solder_dots: Vec<[f32; 2]>,
    pub state: LogicState,
    pub bus_bits: usize,
    pub bus_states: Vec<LogicState>,
    pub label: String,
    pub selected: bool,
    pub custom_mid_x: Option<f32>,
    pub custom_mid_y: Option<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CircuitPage {
    pub name: String,
    pub gates: Vec<GateInstance>,
    pub wires: Vec<WireInstance>,
    pub pan_world: [f32; 2],
    pub zoom_world_per_px: f32,
}

impl CircuitPage {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            gates: Vec::new(),
            wires: Vec::new(),
            pan_world: [0.0, 0.0],
            zoom_world_per_px: DEFAULT_VIEW_ZOOM,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OscopeChannel {
    pub name: String,
    pub target_wire_id: Option<u32>,
    pub target_pin: Option<PinRef>,
    pub history: VecDeque<LogicState>,
}

pub struct CircuitSimulator {
    pub pages: Vec<CircuitPage>,
    pub current_page: usize,
    pub next_gate_id: u32,
    pub next_wire_id: u32,
    pub step_count: u64,
    pub running: bool,
    pub locked: bool,
    pub time_step_ms: u32,
    pub oscope_channels: Vec<OscopeChannel>,
    pub oscope_paused: bool,
    pub max_history: usize,
    pub pause_requested_by_gate: bool,
    pub kmaps: Vec<TruthTableKMap>,
    pub active_kmap: usize,
}

impl CircuitSimulator {
    pub fn new(lib: &GateLibrary) -> Self {
        #[cfg(debug_assertions)]
        let load_demo = std::env::var("LOGICCRAFT_NO_DEMO").is_err();
        #[cfg(not(debug_assertions))]
        let load_demo = std::env::var("LOGICCRAFT_DEMO")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let pages = if load_demo {
            vec![
                CircuitPage::new("Page 1 - Counter & ALU"),
                CircuitPage::new("Page 2 - Memory & Mux"),
            ]
        } else {
            vec![CircuitPage::new("Page 1")]
        };

        let kmaps = if load_demo {
            vec![TruthTableKMap::new_default("Majority / Carry Function")]
        } else {
            Vec::new()
        };

        let mut sim = Self {
            pages,
            current_page: 0,
            next_gate_id: 1,
            next_wire_id: 1,
            step_count: 0,
            running: load_demo,
            locked: false,
            time_step_ms: crate::mechanics::DEFAULT_TIME_STEP_MS,
            oscope_channels: Vec::new(),
            oscope_paused: false,
            max_history: OSCOPE_TRACE_LENGTH,
            pause_requested_by_gate: false,
            kmaps,
            active_kmap: 0,
        };

        if load_demo {
            sim.populate_demo_circuit(lib);
            for _ in 0..24 {
                sim.step(lib);
            }
        }

        sim
    }

    pub fn snap_to_grid(val: f32) -> f32 {
        (val / GRID_SPACING).round() * GRID_SPACING
    }

    /// Returns all existing "From" connector (`DA_FROM`) names across the project
    pub fn from_connector_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        for p in &self.pages {
            for g in &p.gates {
                if g.type_name == "DA_FROM" {
                    if let Some(jid) = g.lparams.get("JUNCTION_ID") {
                        if !names.contains(jid) {
                            names.push(jid.clone());
                        }
                    }
                }
            }
        }
        names
    }

    /// Returns available "From" connector (`DA_FROM`) names matching the requested scope (`local_only` on `page_idx` or shared across all pages).
    pub fn from_connector_names_for_scope(&self, page_idx: usize, local_only: bool) -> Vec<String> {
        let mut names = Vec::new();
        if local_only {
            if let Some(p) = self.pages.get(page_idx) {
                for g in &p.gates {
                    if g.type_name == "DA_FROM" && g.is_tag_local() {
                        if let Some(jid) = g.lparams.get("JUNCTION_ID") {
                            if !names.contains(jid) {
                                names.push(jid.clone());
                            }
                        }
                    }
                }
            }
        } else {
            for p in &self.pages {
                for g in &p.gates {
                    if g.type_name == "DA_FROM" && !g.is_tag_local() {
                        if let Some(jid) = g.lparams.get("JUNCTION_ID") {
                            if !names.contains(jid) {
                                names.push(jid.clone());
                            }
                        }
                    }
                }
            }
        }
        names
    }

    /// Checks whether another "From" connector (`DA_FROM`) already uses `name`
    pub fn is_from_name_taken(&self, name: &str, ignore_gate_id: Option<u32>) -> bool {
        let target = name.trim();
        for p in &self.pages {
            for g in &p.gates {
                if g.type_name == "DA_FROM" && Some(g.id) != ignore_gate_id {
                    if let Some(jid) = g.lparams.get("JUNCTION_ID") {
                        if jid.trim().eq_ignore_ascii_case(target) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Checks whether another "From" connector (`DA_FROM`) in the same scope (`local_only` on `page_idx` or shared globally) uses `name`.
    pub fn is_from_name_taken_in_scope(
        &self,
        name: &str,
        page_idx: usize,
        local_only: bool,
        ignore_gate_id: Option<u32>,
    ) -> bool {
        let target = name.trim();
        if local_only {
            if let Some(p) = self.pages.get(page_idx) {
                for g in &p.gates {
                    if g.type_name == "DA_FROM" && g.is_tag_local() && Some(g.id) != ignore_gate_id {
                        if let Some(jid) = g.lparams.get("JUNCTION_ID") {
                            if jid.trim().eq_ignore_ascii_case(target) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        } else {
            for p in &self.pages {
                for g in &p.gates {
                    if g.type_name == "DA_FROM" && !g.is_tag_local() && Some(g.id) != ignore_gate_id {
                        if let Some(jid) = g.lparams.get("JUNCTION_ID") {
                            if jid.trim().eq_ignore_ascii_case(target) {
                                return true;
                            }
                        }
                    }
                }
            }
            false
        }
    }

    /// Generates a unique `JUNCTION_ID` name for a new "From" connector (`NET_A`, `NET_B`, ...)
    pub fn next_unique_from_name(&self) -> String {
        for ch in b'A'..=b'Z' {
            let candidate = format!("NET_{}", ch as char);
            if !self.is_from_name_taken(&candidate, None) {
                return candidate;
            }
        }
        let mut idx = 1u32;
        loop {
            let candidate = format!("NET_{idx}");
            if !self.is_from_name_taken(&candidate, None) {
                return candidate;
            }
            idx += 1;
        }
    }

    pub fn populate_demo_circuit(&mut self, lib: &GateLibrary) {
        while self.pages.len() < 2 {
            let n = self.pages.len() + 1;
            self.pages.push(CircuitPage::new(format!("Page {n}")));
        }
        self.pages[0].gates.clear();
        self.pages[0].wires.clear();
        self.pages[1].gates.clear();
        self.pages[1].wires.clear();

        // Build a complete, visually clear, interactive digital circuit on Page 1:
        // 1) Variable-rate Clock + Count Enable Toggle Switch + Clear Switch driving a 4-Bit Counting Register
        // 2) 4-Bit Hex LED Display showing live counter value (0..F)
        // 3) 4-Bit Hex Keypad + 4-Bit Full Adder adding Keypad + Counter -> 4-Bit Sum LED Display + Carry LED
        // 4) Basic Logic section: 2-input AND, OR, XOR, NAND, Inverter with Toggle Switches and LEDs
        let add_gate = |sim: &mut Self, page_idx: usize, type_name: &str, pos: [f32; 2]| -> u32 {
            let def = lib.get(type_name).expect(type_name);
            let id = sim.next_gate_id;
            sim.next_gate_id += 1;
            let inst = GateInstance::new(id, def, pos);
            sim.pages[page_idx].gates.push(inst);
            id
        };

        // Title Label
        let lbl_id = add_gate(self, 0, "AA_LABEL", [-16.0, 17.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == lbl_id) {
            g.gparams
                .insert("LABEL_TEXT".into(), "4-Bit Synchronous Counter & Full Adder".into());
        }

        // Clock & Controls for 4-Bit Counting Register
        let clk_id = add_gate(self, 0, "BB_CLOCK", [-28.0, 5.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == clk_id) {
            g.lparams.insert("HALF_CYCLE".into(), "3".into());
        }
        let en_sw = add_gate(self, 0, "AA_TOGGLE", [-28.0, 9.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == en_sw) {
            g.lparams.insert("OUTPUT_NUM".into(), "1".into());
        }
        let up_sw = add_gate(self, 0, "AA_TOGGLE", [-28.0, 13.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == up_sw) {
            g.lparams.insert("OUTPUT_NUM".into(), "1".into());
        }

        let reg_id = add_gate(self, 0, "AA_REGISTER4", [-16.0, 9.5]);
        let disp_cnt = add_gate(self, 0, "GF_LED_DISPLAY_4BIT_OUT", [-5.5, 11.5]);
        let carry_led = add_gate(self, 0, "GA_LED", [-5.5, 5.5]);

        // Connect Clock, Count Enable, Up/Down to Register, and Register outputs to 4-Bit Pass-Through Hex Display
        self.connect_pins(lib, 0, clk_id, "CLK", reg_id, "clock", "CLK");
        self.connect_pins(lib, 0, en_sw, "OUT", reg_id, "count_enable", "CNT_EN");
        self.connect_pins(lib, 0, up_sw, "OUT", reg_id, "up_down", "UP_MODE");
        self.connect_pins(lib, 0, reg_id, "OUT_3", disp_cnt, "IN_3", "CNT_B3");
        self.connect_pins(lib, 0, reg_id, "OUT_2", disp_cnt, "IN_2", "CNT_B2");
        self.connect_pins(lib, 0, reg_id, "OUT_1", disp_cnt, "IN_1", "CNT_B1");
        self.connect_pins(lib, 0, reg_id, "OUT_0", disp_cnt, "IN_0", "CNT_B0");
        self.connect_pins(lib, 0, reg_id, "carry_out", carry_led, "IN_0", "CARRY");

        // 4-Bit ALU / Full Adder section (Right side)
        let keypad_id = add_gate(self, 0, "DD_KEYPAD_HEX", [6.0, 13.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == keypad_id) {
            g.lparams.insert("OUTPUT_NUM".into(), "5".into());
        }
        let adder_id = add_gate(self, 0, "AE_FULLADDER_4BIT", [18.0, 9.5]);
        let sum_disp = add_gate(self, 0, "GE_LED_DISPLAY_4BIT", [28.5, 10.5]);
        let cout_led = add_gate(self, 0, "GA_LED", [28.5, 4.5]);

        self.connect_pins(lib, 0, keypad_id, "OUT_3", adder_id, "A_3", "KEY_3");
        self.connect_pins(lib, 0, keypad_id, "OUT_2", adder_id, "A_2", "KEY_2");
        self.connect_pins(lib, 0, keypad_id, "OUT_1", adder_id, "A_1", "KEY_1");
        self.connect_pins(lib, 0, keypad_id, "OUT_0", adder_id, "A_0", "KEY_0");

        let w_b3 = self.connect_pins(lib, 0, disp_cnt, "OUT_3", adder_id, "B_3", "CNT_B3");
        let w_b2 = self.connect_pins(lib, 0, disp_cnt, "OUT_2", adder_id, "B_2", "CNT_B2");
        let w_b1 = self.connect_pins(lib, 0, disp_cnt, "OUT_1", adder_id, "B_1", "CNT_B1");
        let w_b0 = self.connect_pins(lib, 0, disp_cnt, "OUT_0", adder_id, "B_0", "CNT_B0");
        for (wid, mx) in [(w_b3, 1.0f32), (w_b2, 0.4), (w_b1, -0.2), (w_b0, -0.8)] {
            if let Some(w) = self.pages[0].wires.iter_mut().find(|w| w.id == wid) {
                w.custom_mid_x = Some(mx);
            }
        }

        self.connect_pins(lib, 0, adder_id, "sum_3", sum_disp, "IN_3", "SUM_3");
        self.connect_pins(lib, 0, adder_id, "sum_2", sum_disp, "IN_2", "SUM_2");
        self.connect_pins(lib, 0, adder_id, "sum_1", sum_disp, "IN_1", "SUM_1");
        self.connect_pins(lib, 0, adder_id, "sum_0", sum_disp, "IN_0", "SUM_0");
        self.connect_pins(lib, 0, adder_id, "carry_out", cout_led, "IN_0", "ALU_COUT");

        // Combinational Logic & Tri-State / Flip-Flop Showcase (Lower half)
        let sw_a = add_gate(self, 0, "AA_TOGGLE", [-28.0, -3.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == sw_a) {
            g.lparams.insert("OUTPUT_NUM".into(), "1".into());
        }
        let sw_b = add_gate(self, 0, "AA_TOGGLE", [-28.0, -8.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == sw_b) {
            g.lparams.insert("OUTPUT_NUM".into(), "0".into());
        }
        let and_id = add_gate(self, 0, "AA_AND2", [-16.0, -3.5]);
        let xor_id = add_gate(self, 0, "AI_XOR2", [-16.0, -9.5]);
        let or_id = add_gate(self, 0, "AE_OR2", [-5.0, -6.5]);
        let inv_id = add_gate(self, 0, "AA_INVERTER", [5.0, -3.5]);
        let led_or = add_gate(self, 0, "GA_LED", [6.0, -8.5]);
        let led_inv = add_gate(self, 0, "GA_LED", [13.5, -3.5]);

        self.connect_pins(lib, 0, sw_a, "OUT", and_id, "IN_0", "SW_A");
        self.connect_pins(lib, 0, sw_b, "OUT", and_id, "IN_1", "SW_B");
        self.connect_pins(lib, 0, sw_a, "OUT", xor_id, "IN_0", "SW_A");
        self.connect_pins(lib, 0, clk_id, "CLK", xor_id, "IN_1", "CLK_XOR");
        self.connect_pins(lib, 0, and_id, "OUT", or_id, "IN_0", "AND_OUT");
        self.connect_pins(lib, 0, xor_id, "OUT", or_id, "IN_1", "XOR_OUT");
        self.connect_pins(lib, 0, and_id, "OUT", inv_id, "IN_0", "AND_OUT");
        self.connect_pins(lib, 0, inv_id, "OUT_0", led_inv, "IN_0", "NAND_OUT");
        self.connect_pins(lib, 0, or_id, "OUT", led_or, "IN_0", "OR_OUT");

        // JK Flip-Flop driven by Clock on lower right, plus 5V (Vdd), GND, and From/To Connectors
        let vdd_id = add_gate(self, 0, "EE_VDD", [18.5, -2.0]);
        let gnd_id = add_gate(self, 0, "FF_GND", [-16.0, 2.0]);
        let from_conn = add_gate(self, 0, "DA_FROM", [-20.5, 1.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == from_conn) {
            g.lparams.insert("JUNCTION_ID".into(), "CLK_BUS".into());
        }
        let to_conn1 = add_gate(self, 0, "DE_TO", [15.5, -8.0]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == to_conn1) {
            g.lparams.insert("JUNCTION_ID".into(), "CLK_BUS".into());
        }
        let to_conn2 = add_gate(self, 0, "DE_TO", [15.5, -14.5]);
        if let Some(g) = self.pages[0].gates.iter_mut().find(|g| g.id == to_conn2) {
            g.lparams.insert("JUNCTION_ID".into(), "CLK_BUS".into());
        }
        let conn_led = add_gate(self, 0, "GA_LED", [23.5, -14.5]);

        let jk_id = add_gate(self, 0, "BE_JKFF_LOW", [24.0, -7.5]);
        let jk_led = add_gate(self, 0, "GA_LED", [31.0, -6.0]);
        self.connect_pins(lib, 0, gnd_id, "OUT_0", reg_id, "clear", "GND_CLR");
        self.connect_pins(lib, 0, clk_id, "CLK", from_conn, "IN", "CLK_SRC");
        self.connect_pins(lib, 0, vdd_id, "OUT_0", jk_id, "J", "VDD");
        self.connect_pins(lib, 0, vdd_id, "OUT_0", jk_id, "K", "VDD");
        self.connect_pins(lib, 0, to_conn1, "OUT", jk_id, "clock", "CLK_TO_JK");
        self.connect_pins(lib, 0, to_conn2, "OUT", conn_led, "IN_0", "CLK_TO_LED");
        self.connect_pins(lib, 0, jk_id, "Q", jk_led, "IN_0", "JK_Q");
        self.rebuild_wire_geometry(lib, 0);

        // Page 2: RAM & Decoder circuit
        let p2_key = add_gate(self, 1, "DD_KEYPAD_HEX", [-18.0, 4.0]);
        let p2_ram = add_gate(self, 1, "AA_RAM_4x4", [-4.0, 2.0]);
        let p2_disp = add_gate(self, 1, "GE_LED_DISPLAY_4BIT", [10.0, 3.0]);
        let p2_vdd = add_gate(self, 1, "EE_VDD", [-6.0, 9.0]);
        self.connect_pins(lib, 1, p2_key, "OUT_3", p2_ram, "ADDRESS_3", "ADDR_3");
        self.connect_pins(lib, 1, p2_key, "OUT_2", p2_ram, "ADDRESS_2", "ADDR_2");
        self.connect_pins(lib, 1, p2_key, "OUT_1", p2_ram, "ADDRESS_1", "ADDR_1");
        self.connect_pins(lib, 1, p2_key, "OUT_0", p2_ram, "ADDRESS_0", "ADDR_0");
        self.connect_pins(lib, 1, p2_vdd, "OUT_0", p2_ram, "enable", "RAM_EN");
        self.connect_pins(lib, 1, p2_ram, "DATA_OUT_3", p2_disp, "IN_3", "DOUT_3");
        self.connect_pins(lib, 1, p2_ram, "DATA_OUT_2", p2_disp, "IN_2", "DOUT_2");
        self.connect_pins(lib, 1, p2_ram, "DATA_OUT_1", p2_disp, "IN_1", "DOUT_1");
        self.connect_pins(lib, 1, p2_ram, "DATA_OUT_0", p2_disp, "IN_0", "DOUT_0");
        self.rebuild_wire_geometry(lib, 1);

        // Configure Oscilloscope Channels
        self.oscope_channels = vec![
            OscopeChannel {
                name: "CLK (Clock)".into(),
                target_wire_id: Some(1),
                target_pin: Some(PinRef {
                    gate_id: clk_id,
                    pin_name: "CLK".into(),
                }),
                history: VecDeque::new(),
            },
            OscopeChannel {
                name: "CNT_B0 (LSB)".into(),
                target_wire_id: Some(7),
                target_pin: Some(PinRef {
                    gate_id: reg_id,
                    pin_name: "OUT_0".into(),
                }),
                history: VecDeque::new(),
            },
            OscopeChannel {
                name: "CNT_B1".into(),
                target_wire_id: Some(6),
                target_pin: Some(PinRef {
                    gate_id: reg_id,
                    pin_name: "OUT_1".into(),
                }),
                history: VecDeque::new(),
            },
            OscopeChannel {
                name: "CNT_B2".into(),
                target_wire_id: Some(5),
                target_pin: Some(PinRef {
                    gate_id: reg_id,
                    pin_name: "OUT_2".into(),
                }),
                history: VecDeque::new(),
            },
            OscopeChannel {
                name: "XOR_OUT".into(),
                target_wire_id: None,
                target_pin: Some(PinRef {
                    gate_id: xor_id,
                    pin_name: "OUT".into(),
                }),
                history: VecDeque::new(),
            },
        ];
    }

    pub fn connect_pins(
        &mut self,
        lib: &GateLibrary,
        page_idx: usize,
        g1: u32,
        p1: &str,
        g2: u32,
        p2: &str,
        label: &str,
    ) -> u32 {
        let pos1 = self.pin_world_position(lib, page_idx, g1, p1).unwrap_or([0.0, 0.0]);
        let pos2 = self.pin_world_position(lib, page_idx, g2, p2).unwrap_or([0.0, 0.0]);
        let mid_x = Self::snap_to_grid((pos1[0] + pos2[0]) * 0.5);
        let points = if (pos1[1] - pos2[1]).abs() < 0.01 {
            vec![pos1, pos2]
        } else {
            vec![pos1, [mid_x, pos1[1]], [mid_x, pos2[1]], pos2]
        };
        let id = self.next_wire_id;
        self.next_wire_id += 1;
        self.pages[page_idx].wires.push(WireInstance {
            id,
            endpoints: vec![
                PinRef {
                    gate_id: g1,
                    pin_name: p1.to_string(),
                },
                PinRef {
                    gate_id: g2,
                    pin_name: p2.to_string(),
                },
            ],
            points,
            solder_dots: Vec::new(),
            state: LogicState::Zero,
            bus_bits: 1,
            bus_states: vec![LogicState::Zero],
            label: label.to_string(),
            selected: false,
            custom_mid_x: None,
            custom_mid_y: None,
        });
        self.rebuild_wire_geometry(lib, page_idx);
        id
    }

    pub fn add_gate_on_page(
        &mut self,
        lib: &GateLibrary,
        page_idx: usize,
        type_name: &str,
        pos: [f32; 2],
    ) -> u32 {
        let def = lib.get(type_name).expect(type_name);
        let id = self.next_gate_id;
        self.next_gate_id += 1;
        let inst = GateInstance::new(id, def, pos);
        self.pages[page_idx].gates.push(inst);
        id
    }

    /// Synthesizes the active K-Map's minimal Sum-of-Products circuit onto `page_idx`
    pub fn synthesize_kmap_to_page(&mut self, lib: &GateLibrary, page_idx: usize, kmap_idx: usize) {
        let Some(kmap) = self.kmaps.get(kmap_idx).cloned() else {
            return;
        };
        let (imps, _expr) = kmap.solve_minimal_sop();
        let n = kmap.num_vars.clamp(2, 4);
        let base_x = -12.0f32;
        let base_y = -15.0f32;

        // Place input toggle switches and inverters for each variable
        let mut var_sw_ids = Vec::new();
        let mut var_inv_ids = Vec::new();
        for i in 0..n {
            let y = base_y - (i as f32) * 4.5;
            let sw_id = self.add_gate_on_page(lib, page_idx, "AA_TOGGLE", [base_x, y]);
            let inv_id = self.add_gate_on_page(lib, page_idx, "AE_SMALL_INVERTER", [base_x + 6.5, y - 1.5]);
            self.connect_pins(
                lib,
                page_idx,
                sw_id,
                "OUT",
                inv_id,
                "IN_0",
                kmap.effective_var_name(i),
            );
            var_sw_ids.push(sw_id);
            var_inv_ids.push(inv_id);
        }

        let mut term_out_pins: Vec<(u32, String)> = Vec::new();
        for (ti, imp) in imps.iter().enumerate() {
            let mut lits: Vec<(u32, &'static str)> = Vec::new();
            for i in 0..n {
                let bit_pos = n - 1 - i;
                let bit_mask = 1u8 << bit_pos;
                if (imp.mask & bit_mask) == 0 {
                    if (imp.bits & bit_mask) != 0 {
                        lits.push((var_sw_ids[i], "OUT"));
                    } else {
                        lits.push((var_inv_ids[i], "OUT_0"));
                    }
                }
            }
            let ty = base_y - (ti as f32) * 4.5;
            if lits.is_empty() {
                let vdd = self.add_gate_on_page(lib, page_idx, "EE_VDD", [base_x + 14.0, ty]);
                term_out_pins.push((vdd, "OUT_0".into()));
            } else if lits.len() == 1 {
                term_out_pins.push((lits[0].0, lits[0].1.into()));
            } else {
                let and_type = match lits.len() {
                    2 => "AA_AND2",
                    3 => "AA_AND3",
                    _ => "AA_AND4",
                };
                let and_id = self.add_gate_on_page(lib, page_idx, and_type, [base_x + 15.0, ty]);
                for (li, &(src_g, src_p)) in lits.iter().take(4).enumerate() {
                    self.connect_pins(
                        lib,
                        page_idx,
                        src_g,
                        src_p,
                        and_id,
                        &format!("IN_{li}"),
                        "TERM",
                    );
                }
                term_out_pins.push((and_id, "OUT".into()));
            }
        }

        let out_led = self.add_gate_on_page(lib, page_idx, "GA_LED", [base_x + 29.0, base_y - 3.0]);
        if term_out_pins.len() == 1 {
            self.connect_pins(
                lib,
                page_idx,
                term_out_pins[0].0,
                &term_out_pins[0].1,
                out_led,
                "IN_0",
                kmap.effective_output_name(),
            );
        } else if term_out_pins.len() >= 2 {
            let or_type = match term_out_pins.len() {
                2 => "AE_OR2",
                3 => "AE_OR3",
                4 => "AE_OR4",
                _ => "DE_OR8",
            };
            let or_id = self.add_gate_on_page(lib, page_idx, or_type, [base_x + 22.5, base_y - 3.0]);
            for (oi, (src_g, src_p)) in term_out_pins.iter().take(8).enumerate() {
                self.connect_pins(
                    lib,
                    page_idx,
                    *src_g,
                    src_p,
                    or_id,
                    &format!("IN_{oi}"),
                    "SOP",
                );
            }
            self.connect_pins(
                lib,
                page_idx,
                or_id,
                "OUT",
                out_led,
                "IN_0",
                kmap.effective_output_name(),
            );
        }
    }

    pub fn pin_world_position(
        &self,
        lib: &GateLibrary,
        page_idx: usize,
        gate_id: u32,
        pin_name: &str,
    ) -> Option<[f32; 2]> {
        let page = self.pages.get(page_idx)?;
        let gate = page.gates.iter().find(|g| g.id == gate_id)?;
        let def = lib.get(&gate.type_name)?;
        let pin = def
            .inputs
            .iter()
            .chain(def.outputs.iter())
            .find(|p| p.name == pin_name)
            .or_else(|| def.inputs.first().or_else(|| def.outputs.first()))?;
        Some(gate.pin_world_pos(pin.pos))
    }

    pub fn rebuild_wire_geometry(&mut self, lib: &GateLibrary, page_idx: usize) {
        let Some(page) = self.pages.get(page_idx) else {
            return;
        };

        // Track occupied vertical routing lanes: (mid_x, y_min, y_max, src_pin)
        // Wires sharing the same `src_pin` belong to the same net and MAY share a vertical trunk;
        // wires from different `src_pin`s are offset into parallel vertical lanes so they never overlap!
        let mut occupied_lanes: Vec<(f32, f32, f32, Option<PinRef>)> = Vec::new();
        let mut updated_points: Vec<(Vec<[f32; 2]>, Vec<[f32; 2]>)> =
            Vec::with_capacity(page.wires.len());

        for w in &page.wires {
            if w.endpoints.len() >= 2 {
                let p1 = self
                    .pin_world_position(lib, page_idx, w.endpoints[0].gate_id, &w.endpoints[0].pin_name)
                    .unwrap_or_else(|| *w.points.first().unwrap_or(&[0.0, 0.0]));
                let p2 = self
                    .pin_world_position(lib, page_idx, w.endpoints[1].gate_id, &w.endpoints[1].pin_name)
                    .unwrap_or_else(|| *w.points.last().unwrap_or(&[0.0, 0.0]));
                let src_pin = w.endpoints.first().cloned();

                if let Some(mid_x) = w.custom_mid_x {
                    let y_min = p1[1].min(p2[1]);
                    let y_max = p1[1].max(p2[1]);
                    occupied_lanes.push((mid_x, y_min, y_max, src_pin));
                    updated_points.push((
                        vec![p1, [mid_x, p1[1]], [mid_x, p2[1]], p2],
                        Vec::new(),
                    ));
                } else if let Some(mid_y) = w.custom_mid_y {
                    updated_points.push((
                        vec![p1, [p1[0], mid_y], [p2[0], mid_y], p2],
                        Vec::new(),
                    ));
                } else if (p1[1] - p2[1]).abs() < 0.01 {
                    updated_points.push((vec![p1, p2], Vec::new()));
                } else if (p1[0] - p2[0]).abs() < 0.01 {
                    updated_points.push((vec![p1, p2], Vec::new()));
                } else {
                    let base_mid_x = Self::snap_to_grid((p1[0] + p2[0]) * 0.5);
                    let y_min = p1[1].min(p2[1]);
                    let y_max = p1[1].max(p2[1]);

                    let mut chosen_x = base_mid_x;
                    // If another wire from the SAME source pin already has a lane near base_mid_x, share it
                    if let Some(&(existing_x, _, _, _)) = occupied_lanes.iter().find(|&&(lx, _, _, ref sp)| {
                        *sp == src_pin && (lx - base_mid_x).abs() <= 2.5
                    }) {
                        chosen_x = existing_x;
                    } else {
                        // Otherwise find a lane near base_mid_x that doesn't collide with a different net
                        let x_min_bound = p1[0].min(p2[0]) + 0.5;
                        let x_max_bound = p1[0].max(p2[0]) - 0.5;
                        for step in 0..16i32 {
                            let delta = if step == 0 {
                                0.0
                            } else if step % 2 == 1 {
                                ((step + 1) / 2) as f32 * 0.6
                            } else {
                                -(step / 2) as f32 * 0.6
                            };
                            let cand = base_mid_x + delta;
                            if x_min_bound < x_max_bound && (cand < x_min_bound || cand > x_max_bound) {
                                continue;
                            }
                            let collides = occupied_lanes.iter().any(|&(lx, ly0, ly1, ref sp)| {
                                *sp != src_pin
                                    && (lx - cand).abs() < 0.45
                                    && !(y_max < ly0 - 0.1 || y_min > ly1 + 0.1)
                            });
                            if !collides {
                                chosen_x = cand;
                                break;
                            }
                        }
                    }

                    occupied_lanes.push((chosen_x, y_min, y_max, src_pin));
                    updated_points.push((
                        vec![p1, [chosen_x, p1[1]], [chosen_x, p2[1]], p2],
                        Vec::new(),
                    ));
                }
            } else {
                updated_points.push((w.points.clone(), Vec::new()));
            }
        }

        // Compute true electrical T-junction solder dots ONLY where 2+ wires sharing a net branch
        let num_wires = updated_points.len();
        for i in 0..num_wires {
            let src_i = page.wires[i].endpoints.first();
            for j in (i + 1)..num_wires {
                let src_j = page.wires[j].endpoints.first();
                if src_i.is_some() && src_i == src_j {
                    // Check if they share a trunk vertical x and branch at one of the corners
                    let pts_i = &updated_points[i].0;
                    let pts_j = &updated_points[j].0;
                    if pts_i.len() == 4 && pts_j.len() == 4 && (pts_i[1][0] - pts_j[1][0]).abs() < 0.05 {
                        let branch_pt = pts_i[1];
                        if !updated_points[i].1.contains(&branch_pt) {
                            updated_points[i].1.push(branch_pt);
                        }
                    }
                }
            }
        }

        if let Some(page) = self.pages.get_mut(page_idx) {
            for (w, (pts, dots)) in page.wires.iter_mut().zip(updated_points) {
                w.points = pts;
                w.solder_dots = dots;
            }
        }
    }

    /// Advances the digital logic simulation by `DEFAULT_GATE_DELAY` step (FUN_00456940, FUN_0047e0b0)
    pub fn step(&mut self, lib: &GateLibrary) {
        for _ in 0..DEFAULT_GATE_DELAY {
            self.step_count += 1;

            // Collect wireless JUNCTION_ID ("From" -> "To") states:
            // - Shared ("SHARED", default): `DA_FROM` broadcasts to `"SHARED"` `DE_TO` connectors across ALL pages.
            // - Local ("LOCAL"): `DA_FROM` broadcasts ONLY to `"LOCAL"` `DE_TO` connectors on the SAME page.
            let mut shared_junction_states: HashMap<String, LogicState> = HashMap::new();
            for page in &self.pages {
                for g in &page.gates {
                    if g.type_name == "DA_FROM" && !g.is_tag_local() {
                        let jid = g
                            .lparams
                            .get("JUNCTION_ID")
                            .cloned()
                            .unwrap_or_else(|| "NET_A".into());
                        let in_state = g
                            .pin_states
                            .get("IN")
                            .copied()
                            .unwrap_or(LogicState::HiZ);
                        if in_state != LogicState::HiZ {
                            shared_junction_states
                                .entry(jid)
                                .and_modify(|prev| {
                                    if *prev != in_state {
                                        *prev = LogicState::Conflict;
                                    }
                                })
                                .or_insert(in_state);
                        }
                    }
                }
            }

            // Propagate wires and evaluate gates on each page
            for page in &mut self.pages {
                // Collect page-local ("LOCAL") `DA_FROM` states for this page
                let mut local_junction_states: HashMap<String, LogicState> = HashMap::new();
                for g in &page.gates {
                    if g.type_name == "DA_FROM" && g.is_tag_local() {
                        let jid = g
                            .lparams
                            .get("JUNCTION_ID")
                            .cloned()
                            .unwrap_or_else(|| "NET_A".into());
                        let in_state = g
                            .pin_states
                            .get("IN")
                            .copied()
                            .unwrap_or(LogicState::HiZ);
                        if in_state != LogicState::HiZ {
                            local_junction_states
                                .entry(jid)
                                .and_modify(|prev| {
                                    if *prev != in_state {
                                        *prev = LogicState::Conflict;
                                    }
                                })
                                .or_insert(in_state);
                        }
                    }
                }

                // First, resolve wire states from all output pins connected to each wire
                let mut gate_map: HashMap<u32, usize> = HashMap::new();
                for (idx, g) in page.gates.iter().enumerate() {
                    gate_map.insert(g.id, idx);
                }

                // Reset input pins to HiZ before wire resolution
                for g in &mut page.gates {
                    if let Some(def) = lib.get(&g.type_name) {
                        for p in &def.inputs {
                            g.pin_states.insert(p.name.clone(), LogicState::HiZ);
                        }
                    }
                }

                // Group connected nets (wires sharing an endpoint pin form a single electrical net)
                let mut pin_net_state: HashMap<PinRef, LogicState> = HashMap::new();
                for w in &page.wires {
                    let mut drivers = Vec::new();
                    for ep in &w.endpoints {
                        if let Some(&g_idx) = gate_map.get(&ep.gate_id) {
                            let g = &page.gates[g_idx];
                            if let Some(def) = lib.get(&g.type_name) {
                                if def.outputs.iter().any(|p| p.name == ep.pin_name) {
                                    let st = g
                                        .pin_states
                                        .get(&ep.pin_name)
                                        .copied()
                                        .unwrap_or(LogicState::HiZ);
                                    if st != LogicState::HiZ {
                                        drivers.push(st);
                                    }
                                }
                            }
                        }
                    }
                    let resolved = resolve_drivers(&drivers);
                    for ep in &w.endpoints {
                        pin_net_state
                            .entry(ep.clone())
                            .and_modify(|prev| {
                                *prev = resolve_drivers(&[*prev, resolved]);
                            })
                            .or_insert(resolved);
                    }
                }

                // Second pass so multi-wire nets share resolved state
                for w in &mut page.wires {
                    let mut net_states = Vec::new();
                    for ep in &w.endpoints {
                        if let Some(&st) = pin_net_state.get(ep) {
                            if st != LogicState::HiZ {
                                net_states.push(st);
                            }
                        }
                    }
                    let final_state = resolve_drivers(&net_states);
                    w.state = final_state;
                    w.bus_states = vec![final_state; w.bus_bits.max(1)];
                    for ep in &w.endpoints {
                        if let Some(&g_idx) = gate_map.get(&ep.gate_id) {
                            let g = &mut page.gates[g_idx];
                            if let Some(def) = lib.get(&g.type_name) {
                                if def.inputs.iter().any(|p| p.name == ep.pin_name) {
                                    g.pin_states.insert(ep.pin_name.clone(), final_state);
                                }
                            }
                        }
                    }
                }

                // Evaluate every gate
                for g in &mut page.gates {
                    let Some(def) = lib.get(&g.type_name) else {
                        continue;
                    };
                    evaluate_gate(
                        g,
                        def,
                        lib,
                        &shared_junction_states,
                        &local_junction_states,
                        &mut self.pause_requested_by_gate,
                    );
                }
            }

            if self.pause_requested_by_gate {
                self.running = false;
                self.pause_requested_by_gate = false;
            }

            // Record oscilloscope history from Page 0
            if !self.oscope_paused {
                if let Some(page0) = self.pages.first() {
                    for ch in &mut self.oscope_channels {
                        let st = if let Some(pin_ref) = &ch.target_pin {
                            page0
                                .gates
                                .iter()
                                .find(|g| g.id == pin_ref.gate_id)
                                .and_then(|g| g.pin_states.get(&pin_ref.pin_name).copied())
                                .unwrap_or(LogicState::HiZ)
                        } else if let Some(wid) = ch.target_wire_id {
                            page0
                                .wires
                                .iter()
                                .find(|w| w.id == wid)
                                .map(|w| w.state)
                                .unwrap_or(LogicState::HiZ)
                        } else {
                            LogicState::HiZ
                        };
                        ch.history.push_back(st);
                        while ch.history.len() > self.max_history {
                            ch.history.pop_front();
                        }
                    }
                }
            }
        }
    }
}

fn resolve_drivers(drivers: &[LogicState]) -> LogicState {
    let mut active: Vec<LogicState> = drivers
        .iter()
        .copied()
        .filter(|&s| s != LogicState::HiZ)
        .collect();
    if active.is_empty() {
        return LogicState::HiZ;
    }
    if active.contains(&LogicState::Conflict) {
        return LogicState::Conflict;
    }
    if active.contains(&LogicState::Unknown) {
        return LogicState::Unknown;
    }
    let first = active.pop().unwrap();
    if active.iter().all(|&s| s == first) {
        first
    } else {
        LogicState::Conflict
    }
}

fn in_bit(g: &GateInstance, name: &str, default_high: bool) -> LogicState {
    match g.pin_states.get(name).copied().unwrap_or(LogicState::HiZ) {
        LogicState::HiZ => {
            if default_high {
                LogicState::One
            } else {
                LogicState::Zero
            }
        }
        other => other,
    }
}

fn evaluate_gate(
    g: &mut GateInstance,
    def: &GateDef,
    lib: &GateLibrary,
    shared_junction_states: &HashMap<String, LogicState>,
    local_junction_states: &HashMap<String, LogicState>,
    pause_requested: &mut bool,
) {
    match def.logic_type.as_str() {
        "AND" => {
            let mut any_zero = false;
            let mut any_unknown = false;
            for p in &def.inputs {
                let mut st = in_bit(g, &p.name, true);
                if def.in_inverted || p.inverted {
                    st = st.invert();
                }
                match st {
                    LogicState::Zero => any_zero = true,
                    LogicState::One => {}
                    _ => any_unknown = true,
                }
            }
            let mut out = if any_zero {
                LogicState::Zero
            } else if any_unknown {
                LogicState::Unknown
            } else {
                LogicState::One
            };
            if def.out_inverted {
                out = out.invert();
            }
            g.pin_states.insert("OUT".into(), out);
        }
        "OR" => {
            let mut any_one = false;
            let mut any_unknown = false;
            for p in &def.inputs {
                let mut st = in_bit(g, &p.name, false);
                if def.in_inverted || p.inverted {
                    st = st.invert();
                }
                match st {
                    LogicState::One => any_one = true,
                    LogicState::Zero => {}
                    _ => any_unknown = true,
                }
            }
            let mut out = if any_one {
                LogicState::One
            } else if any_unknown {
                LogicState::Unknown
            } else {
                LogicState::Zero
            };
            if def.out_inverted {
                out = out.invert();
            }
            g.pin_states.insert("OUT".into(), out);
        }
        "XOR" => {
            let mut ones = 0usize;
            let mut any_unknown = false;
            for p in &def.inputs {
                let mut st = in_bit(g, &p.name, false);
                if def.in_inverted || p.inverted {
                    st = st.invert();
                }
                match st {
                    LogicState::One => ones += 1,
                    LogicState::Zero => {}
                    _ => any_unknown = true,
                }
            }
            let mut out = if any_unknown {
                LogicState::Unknown
            } else if ones % 2 == 1 {
                LogicState::One
            } else {
                LogicState::Zero
            };
            if def.out_inverted {
                out = out.invert();
            }
            g.pin_states.insert("OUT".into(), out);
        }
        "BUFFER" => {
            let bits = g
                .lparams
                .get("INPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);
            let has_enable = def.inputs.iter().any(|p| p.name == "ENABLE_0");
            let active_low = g
                .lparams
                .get("ACTIVE_LOW_ENABLE")
                .map(|v| v == "true")
                .unwrap_or(false);
            let enabled = if has_enable {
                let en = in_bit(g, "ENABLE_0", !active_low);
                if active_low {
                    en == LogicState::Zero
                } else {
                    en == LogicState::One
                }
            } else {
                true
            };
            for i in 0..bits {
                let in_name = format!("IN_{i}");
                let out_name = format!("OUT_{i}");
                let out = if !enabled {
                    LogicState::HiZ
                } else {
                    let st = in_bit(g, &in_name, false);
                    if def.out_inverted {
                        st.invert()
                    } else {
                        st
                    }
                };
                g.pin_states.insert(out_name, out);
            }
        }
        "TGATE" => {
            let active_low = g
                .lparams
                .get("ACTIVE_LOW_ENABLE")
                .map(|v| v == "true")
                .unwrap_or(false);
            let en = in_bit(g, "ENABLE_0", false);
            let enabled = if active_low {
                en == LogicState::Zero
            } else {
                en == LogicState::One
            };
            let out = if enabled {
                g.pin_states.get("IN_0").copied().unwrap_or(LogicState::HiZ)
            } else {
                LogicState::HiZ
            };
            g.pin_states.insert("OUT_0".into(), out);
        }
        "FROM" => {}
        "TO" => {
            let jid = g
                .lparams
                .get("JUNCTION_ID")
                .cloned()
                .unwrap_or_else(|| "NET_A".into());
            let st = if g.is_tag_local() {
                local_junction_states
                    .get(&jid)
                    .copied()
                    .unwrap_or(LogicState::HiZ)
            } else {
                shared_junction_states
                    .get(&jid)
                    .copied()
                    .unwrap_or(LogicState::HiZ)
            };
            g.pin_states.insert("OUT".into(), st);
        }
        "NODE" => {
            let st = g.pin_states.get("IN_0").copied().unwrap_or(LogicState::HiZ);
            for p in &def.outputs {
                g.pin_states.insert(p.name.clone(), st);
            }
        }
        "BUS_END" => {
            let st = in_bit(g, "BIT_0", false);
            g.pin_states.insert("BUS".into(), st);
        }
        "DRIVER" => {
            let bits = g
                .lparams
                .get("OUTPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);
            let num = g
                .lparams
                .get("OUTPUT_NUM")
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0);
            if bits == 1 {
                let st = if (num & 1) != 0 {
                    LogicState::One
                } else {
                    LogicState::Zero
                };
                if def.outputs.iter().any(|p| p.name == "OUT") {
                    g.pin_states.insert("OUT".into(), st);
                } else {
                    g.pin_states.insert("OUT_0".into(), st);
                }
            } else {
                for i in 0..bits {
                    let st = if ((num >> i) & 1) != 0 {
                        LogicState::One
                    } else {
                        LogicState::Zero
                    };
                    g.pin_states.insert(format!("OUT_{i}"), st);
                }
            }
        }
        "CLOCK" => {
            let half_cycle = g
                .lparams
                .get("HALF_CYCLE")
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(DEFAULT_CLOCK_HALF_CYCLE)
                .max(1);
            g.clock_counter += 1;
            if g.clock_counter >= half_cycle {
                g.clock_counter = 0;
                let cur = g
                    .pin_states
                    .get("CLK")
                    .copied()
                    .unwrap_or(LogicState::Zero);
                g.pin_states.insert("CLK".into(), cur.invert());
            }
        }
        "PULSE" => {
            let out = if g.pulse_remaining > 0 {
                g.pulse_remaining -= 1;
                LogicState::One
            } else {
                LogicState::Zero
            };
            g.pin_states.insert("OUT_0".into(), out);
        }
        "MUX" => {
            let inputs_n = g
                .lparams
                .get("INPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(2);
            let sel_bits = (inputs_n as f32).log2().round() as usize;
            let mut sel_idx = 0usize;
            for s in 0..sel_bits {
                if in_bit(g, &format!("SEL_{s}"), false) == LogicState::One {
                    sel_idx |= 1 << s;
                }
            }
            let out = in_bit(g, &format!("IN_{}", sel_idx.min(inputs_n - 1)), false);
            g.pin_states.insert("OUT".into(), out);
        }
        "DECODER" => {
            let sel_n = g
                .lparams
                .get("INPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(2);
            let outs_n = 1usize << sel_n;
            let en = in_bit(g, "ENABLE", true) == LogicState::One;
            let mut sel_idx = 0usize;
            for s in 0..sel_n {
                if in_bit(g, &format!("IN_{s}"), false) == LogicState::One {
                    sel_idx |= 1 << s;
                }
            }
            for o in 0..outs_n {
                let mut st = if en && o == sel_idx {
                    LogicState::One
                } else {
                    LogicState::Zero
                };
                if def.out_inverted {
                    st = st.invert();
                }
                g.pin_states.insert(format!("OUT_{o}"), st);
            }
        }
        "PRI_ENCODER" => {
            let ins_n = g
                .lparams
                .get("INPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(4);
            let outs_n = (ins_n as f32).log2().round() as usize;
            let mut highest: Option<usize> = None;
            for i in (0..ins_n).rev() {
                if in_bit(g, &format!("IN_{i}"), false) == LogicState::One {
                    highest = Some(i);
                    break;
                }
            }
            if let Some(idx) = highest {
                g.pin_states.insert("VALID".into(), LogicState::One);
                for o in 0..outs_n {
                    let bit = if ((idx >> o) & 1) != 0 {
                        LogicState::One
                    } else {
                        LogicState::Zero
                    };
                    g.pin_states.insert(format!("OUT_{o}"), bit);
                }
            } else {
                g.pin_states.insert("VALID".into(), LogicState::Zero);
                for o in 0..outs_n {
                    g.pin_states.insert(format!("OUT_{o}"), LogicState::Zero);
                }
            }
        }
        "ADDER" => {
            let bits = g
                .lparams
                .get("INPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(1);
            let mut carry = u16::from(in_bit(g, "carry_in", false) == LogicState::One);
            for i in 0..bits {
                let a = u16::from(in_bit(g, &format!("A_{i}"), false) == LogicState::One);
                let b = u16::from(in_bit(g, &format!("B_{i}"), false) == LogicState::One);
                let total = a + b + carry;
                let sum_bit = if (total & 1) != 0 {
                    LogicState::One
                } else {
                    LogicState::Zero
                };
                carry = (total >> 1) & 1;
                g.pin_states.insert(format!("sum_{i}"), sum_bit);
            }
            g.pin_states.insert(
                "carry_out".into(),
                if carry != 0 {
                    LogicState::One
                } else {
                    LogicState::Zero
                },
            );
        }
        "COMPARE" => {
            let bits = g
                .lparams
                .get("INPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(4);
            let mut a = 0u16;
            let mut b = 0u16;
            for i in 0..bits {
                if in_bit(g, &format!("A_{i}"), false) == LogicState::One {
                    a |= 1 << i;
                }
                if in_bit(g, &format!("B_{i}"), false) == LogicState::One {
                    b |= 1 << i;
                }
            }
            let (gt, eq, lt) = if a > b {
                (LogicState::One, LogicState::Zero, LogicState::Zero)
            } else if a < b {
                (LogicState::Zero, LogicState::Zero, LogicState::One)
            } else {
                let in_gt = in_bit(g, "in_A_greater_B", false);
                let in_lt = in_bit(g, "in_A_less_B", false);
                let in_eq = in_bit(g, "in_A_equal_B", true);
                (in_gt, in_eq, in_lt)
            };
            g.pin_states.insert("A_greater_B".into(), gt);
            g.pin_states.insert("A_equal_B".into(), eq);
            g.pin_states.insert("A_less_B".into(), lt);
        }
        "JKFF" => {
            let inv_clk = g
                .lparams
                .get("INVERTED_CLOCK")
                .map(|v| v == "true")
                .unwrap_or(false);
            let clk = in_bit(g, "clock", false);
            let set_low = in_bit(g, "set", true) == LogicState::Zero;
            let clr_low = in_bit(g, "clear", true) == LogicState::Zero;
            let mut q = g.pin_states.get("Q").copied().unwrap_or(LogicState::Zero);
            if set_low && clr_low {
                q = LogicState::One;
            } else if set_low {
                q = LogicState::One;
            } else if clr_low {
                q = LogicState::Zero;
            } else {
                let triggered = if inv_clk {
                    g.prev_clock == LogicState::One && clk == LogicState::Zero
                } else {
                    g.prev_clock == LogicState::Zero && clk == LogicState::One
                };
                if triggered {
                    let j = in_bit(g, "J", false) == LogicState::One;
                    let k = in_bit(g, "K", false) == LogicState::One;
                    q = match (j, k) {
                        (false, false) => q,
                        (false, true) => LogicState::Zero,
                        (true, false) => LogicState::One,
                        (true, true) => q.invert(),
                    };
                }
            }
            g.prev_clock = clk;
            g.pin_states.insert("Q".into(), q);
            g.pin_states.insert("Q_bar".into(), q.invert());
        }
        "REGISTER" => {
            let bits = g
                .lparams
                .get("INPUT_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(4);
            let mask = ((1u32 << bits) - 1) as u16;
            let mut cur_val = g
                .lparams
                .get("CURRENT_VALUE")
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0)
                & mask;

            // LED displays are combinational pass-throughs of IN_0..IN_N
            if g.type_name.starts_with("GE_LED_DISPLAY")
                || g.type_name.starts_with("GF_LED_DISPLAY")
                || g.type_name.starts_with("GI_LED_DISPLAY")
            {
                let mut v = 0u16;
                for i in 0..bits {
                    let st = in_bit(g, &format!("IN_{i}"), false);
                    if st == LogicState::One {
                        v |= 1 << i;
                    }
                    if g.type_name == "GF_LED_DISPLAY_4BIT_OUT" {
                        g.pin_states.insert(format!("OUT_{i}"), st);
                    }
                }
                g.lparams.insert("CURRENT_VALUE".into(), v.to_string());
                return;
            }

            // D Flip-Flop (1-bit register)
            if bits == 1 {
                let inv_clk = g
                    .lparams
                    .get("INVERTED_CLOCK")
                    .map(|v| v == "true")
                    .unwrap_or(false);
                let clk = in_bit(g, "clock", false);
                let set_low = in_bit(g, "set", true) == LogicState::Zero;
                let clr_low = in_bit(g, "clear", true) == LogicState::Zero;
                let clk_en = in_bit(g, "clock_enable", true) == LogicState::One;
                if set_low {
                    cur_val = 1;
                } else if clr_low {
                    cur_val = 0;
                } else {
                    let triggered = if inv_clk {
                        g.prev_clock == LogicState::One && clk == LogicState::Zero
                    } else {
                        g.prev_clock == LogicState::Zero && clk == LogicState::One
                    };
                    if triggered && clk_en {
                        cur_val = u16::from(in_bit(g, "IN_0", false) == LogicState::One);
                    }
                }
                g.prev_clock = clk;
                let q = if cur_val != 0 {
                    LogicState::One
                } else {
                    LogicState::Zero
                };
                g.lparams.insert("CURRENT_VALUE".into(), cur_val.to_string());
                g.pin_states.insert("OUT_0".into(), q);
                g.pin_states.insert("OUTINV_0".into(), q.invert());
                return;
            }

            // Counting / Shift Register (FUN_00486eb0)
            let clk = in_bit(g, "clock", false);
            let clr = in_bit(g, "clear", false) == LogicState::One;
            if clr {
                cur_val = 0;
            } else if g.prev_clock == LogicState::Zero && clk == LogicState::One {
                let load = in_bit(g, "load", false) == LogicState::One;
                if load {
                    let mut in_v = 0u16;
                    for i in 0..bits {
                        if in_bit(g, &format!("IN_{i}"), false) == LogicState::One {
                            in_v |= 1 << i;
                        }
                    }
                    cur_val = in_v & mask;
                } else if g.type_name.contains("SHIFT") {
                    if in_bit(g, "shift_enable", false) == LogicState::One {
                        if in_bit(g, "shift_left", false) == LogicState::One {
                            cur_val = (cur_val << 1) & mask;
                        } else {
                            cur_val = (cur_val >> 1) & mask;
                        }
                    }
                } else if in_bit(g, "count_enable", false) == LogicState::One {
                    let up = in_bit(g, "up_down", true) == LogicState::One;
                    if up {
                        cur_val = cur_val.wrapping_add(1) & mask;
                    } else {
                        cur_val = cur_val.wrapping_sub(1) & mask;
                    }
                }
            }
            g.prev_clock = clk;
            g.lparams.insert("CURRENT_VALUE".into(), cur_val.to_string());
            for i in 0..bits {
                let bit = if ((cur_val >> i) & 1) != 0 {
                    LogicState::One
                } else {
                    LogicState::Zero
                };
                g.pin_states.insert(format!("OUT_{i}"), bit);
            }
            let carry_out = if cur_val == mask {
                LogicState::One
            } else {
                LogicState::Zero
            };
            g.pin_states.insert("carry_out".into(), carry_out);
        }
        "RAM" => {
            let addr_bits = g
                .lparams
                .get("ADDRESS_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(4)
                .min(12);
            let data_bits = g
                .lparams
                .get("DATA_BITS")
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(4)
                .min(8);
            let mut addr = 0u16;
            for i in 0..addr_bits {
                if in_bit(g, &format!("ADDRESS_{i}"), false) == LogicState::One {
                    addr |= 1 << i;
                }
            }
            let wclk = in_bit(g, "write_clock", false);
            let wen = in_bit(g, "write_enable", false) == LogicState::One;
            if g.prev_clock == LogicState::Zero && wclk == LogicState::One && wen {
                let mut din = 0u16;
                for i in 0..data_bits {
                    if in_bit(g, &format!("DATA_IN_{i}"), false) == LogicState::One {
                        din |= 1 << i;
                    }
                }
                g.memory.insert(addr, din);
            }
            g.prev_clock = wclk;
            let en = in_bit(g, "enable", true) == LogicState::One;
            let word = g.memory.get(&addr).copied().unwrap_or(0);
            for i in 0..data_bits {
                let out = if !en {
                    LogicState::HiZ
                } else if ((word >> i) & 1) != 0 {
                    LogicState::One
                } else {
                    LogicState::Zero
                };
                g.pin_states.insert(format!("DATA_OUT_{i}"), out);
            }
        }
        "Pauseulator" => {
            let sig = in_bit(g, "SIGNAL", false);
            if g.prev_clock == LogicState::Zero && sig == LogicState::One {
                *pause_requested = true;
            }
            g.prev_clock = sig;
        }
        "CUSTOM_IC" => {
            if let Some(spec) = lib.custom_specs.get(&g.type_name) {
                spec.evaluate(&mut g.pin_states, &mut g.lparams, &mut g.prev_clock);
            }
        }
        _ => {}
    }
}
