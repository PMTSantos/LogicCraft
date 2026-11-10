use std::collections::HashMap;
use crate::custom_component::{builtin_custom_templates, CustomComponentSpec};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PinDef {
    pub name: String,
    pub pos: [f32; 2], // In gate local world units (CedarLogic coordinates: +X right, +Y up)
    pub is_output: bool,
    pub inverted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateShapeStyle {
    And,
    Or,
    Xor,
    Buffer,
    TriState,
    BusEnd,
    TGate,
    Transistor { pmos: bool },
    FromLink,
    ToLink,
    JunctionNode,
    ToggleSwitch,
    Clock,
    Pulse,
    HexKeypad,
    Vdd,
    Ground,
    Led,
    LedDisplay { digits: usize },
    Label,
    Mux,
    Decoder,
    PriEncoder,
    Adder,
    Comparator,
    FlipFlopD,
    FlipFlopJK,
    Register,
    RamRom { is_rom: bool },
    Pauseulator,
    ChipBox,
    CustomIc,
    CpuProcessor,
    VramDisplay,
    KeyboardInput,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GateDef {
    pub type_name: String,
    pub caption: String,
    pub category: String,
    pub logic_type: String,
    pub inputs: Vec<PinDef>,
    pub outputs: Vec<PinDef>,
    pub half_size: [f32; 2],
    pub shape_style: GateShapeStyle,
    pub out_inverted: bool,
    pub in_inverted: bool,
    pub default_gparams: HashMap<String, String>,
    pub default_lparams: HashMap<String, String>,
    pub custom_lines: Vec<[f32; 4]>,
}

#[derive(Debug, Clone)]
pub struct GateLibraryCategory {
    pub name: String,
    pub gates: Vec<GateDef>,
}

#[derive(Debug, Clone)]
pub struct GateLibrary {
    pub categories: Vec<GateLibraryCategory>,
    pub by_name: HashMap<String, GateDef>,
    pub custom_specs: HashMap<String, CustomComponentSpec>,
}

impl GateLibrary {
    pub fn new() -> Self {
        let mut categories = Vec::new();

        // 1 - Basic Gates
        let mut basic = Vec::new();
        for &(name, cap, logic, bits, out_inv, in_inv, style) in &[
            ("AA_AND2", "2-input AND", "AND", 2, false, false, GateShapeStyle::And),
            ("AA_AND3", "3-input AND", "AND", 3, false, false, GateShapeStyle::And),
            ("AA_AND4", "4-input AND", "AND", 4, false, false, GateShapeStyle::And),
            ("DA_AND8", "8-input AND", "AND", 8, false, false, GateShapeStyle::And),
            ("AE_OR2", "2-input OR", "OR", 2, false, false, GateShapeStyle::Or),
            ("AE_OR3", "3-input OR", "OR", 3, false, false, GateShapeStyle::Or),
            ("AE_OR4", "4-input OR", "OR", 4, false, false, GateShapeStyle::Or),
            ("DE_OR8", "8-input OR", "OR", 8, false, false, GateShapeStyle::Or),
            ("AI_XOR2", "2-input XOR", "XOR", 2, false, false, GateShapeStyle::Xor),
            ("AI_XOR3", "3-input XOR", "XOR", 3, false, false, GateShapeStyle::Xor),
            ("AI_XOR4", "4-input XOR", "XOR", 4, false, false, GateShapeStyle::Xor),
            ("AO_XNOR2", "2-input XNOR", "XOR", 2, true, false, GateShapeStyle::Xor),
            ("AO_XNOR3", "3-input XNOR", "XOR", 3, true, false, GateShapeStyle::Xor),
            ("AO_XNOR4", "4-input XNOR", "XOR", 4, true, false, GateShapeStyle::Xor),
            ("BA_NAND2", "2-input NAND", "AND", 2, true, false, GateShapeStyle::And),
            ("BA_NAND3", "3-input NAND", "AND", 3, true, false, GateShapeStyle::And),
            ("BA_NAND4", "4-input NAND", "AND", 4, true, false, GateShapeStyle::And),
            ("DI_NAND8", "8-input NAND", "AND", 8, true, false, GateShapeStyle::And),
            ("BE_NOR2", "2-input NOR", "OR", 2, true, false, GateShapeStyle::Or),
            ("BE_NOR3", "3-input NOR", "OR", 3, true, false, GateShapeStyle::Or),
            ("BE_NOR4", "4-input NOR", "OR", 4, true, false, GateShapeStyle::Or),
            ("DM_NOR8", "8-input NOR", "OR", 8, true, false, GateShapeStyle::Or),
            ("BI_NANDX2", "2-input NANDX", "AND", 2, false, true, GateShapeStyle::Or),
            ("BI_NANDX3", "3-input NANDX", "AND", 3, false, true, GateShapeStyle::Or),
            ("BI_NANDX4", "4-input NANDX", "AND", 4, false, true, GateShapeStyle::Or),
            ("DO_NANDX8", "8-input NANDX", "AND", 8, false, true, GateShapeStyle::Or),
            ("BM_NORX2", "2-input NORX", "OR", 2, false, true, GateShapeStyle::And),
            ("BM_NORX3", "3-input NORX", "OR", 3, false, true, GateShapeStyle::And),
            ("BM_NORX4", "4-input NORX", "OR", 4, false, true, GateShapeStyle::And),
            ("DU_NORX8", "8-input NORX", "OR", 8, false, true, GateShapeStyle::And),
        ] {
            basic.push(make_n_input_gate(
                name,
                cap,
                "1 - Basic Gates",
                logic,
                bits,
                out_inv,
                in_inv,
                style,
            ));
        }
        categories.push(GateLibraryCategory {
            name: "1 - Basic Gates".to_string(),
            gates: basic,
        });

        // 2 - Invert & Connect
        let mut inv_conn = Vec::new();
        inv_conn.push(GateDef {
            type_name: "AA_INVERTER".into(),
            caption: "Inverter (NOT)".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "BUFFER".into(),
            inputs: vec![pin("IN_0", [-1.8, 0.0], false, false)],
            outputs: vec![pin("OUT_0", [1.8, 0.0], true, true)],
            half_size: [1.8, 1.5],
            shape_style: GateShapeStyle::Buffer,
            out_inverted: true,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "1")]),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "AE_SMALL_INVERTER".into(),
            caption: "Small Inverter".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "BUFFER".into(),
            inputs: vec![pin("IN_0", [-1.4, 0.0], false, false)],
            outputs: vec![pin("OUT_0", [1.4, 0.0], true, true)],
            half_size: [1.4, 1.2],
            shape_style: GateShapeStyle::Buffer,
            out_inverted: true,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "1")]),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "AI_INVERTER_4BIT".into(),
            caption: "4-Bit Inverter".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "BUFFER".into(),
            inputs: (0..4).map(|i| pin(&format!("IN_{i}"), [-2.0, 1.5 - i as f32], false, false)).collect(),
            outputs: (0..4).map(|i| pin(&format!("OUT_{i}"), [2.0, 1.5 - i as f32], true, true)).collect(),
            half_size: [2.0, 2.5],
            shape_style: GateShapeStyle::Buffer,
            out_inverted: true,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "4")]),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "BA_TRI_STATE".into(),
            caption: "Tri-State Buffer (High Enable)".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "BUFFER".into(),
            inputs: vec![
                pin("IN_0", [-1.8, 0.0], false, false),
                pin("ENABLE_0", [0.0, -0.9], false, false),
            ],
            outputs: vec![pin("OUT_0", [1.8, 0.0], true, false)],
            half_size: [1.8, 1.8],
            shape_style: GateShapeStyle::TriState,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "1"), (" active_low_enable", "false")]),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "BE_TRI_STATE_LOW".into(),
            caption: "Tri-State Buffer (Low Enable)".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "BUFFER".into(),
            inputs: vec![
                pin("IN_0", [-1.8, 0.0], false, false),
                pin("ENABLE_0", [0.0, -1.35], false, true),
            ],
            outputs: vec![pin("OUT_0", [1.8, 0.0], true, false)],
            half_size: [1.8, 1.8],
            shape_style: GateShapeStyle::TriState,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "1"), ("ACTIVE_LOW_ENABLE", "true")]),
            custom_lines: Vec::new(),
        });
        for &(name, cap, bits) in &[
            ("BI_TRI_STATE_4BIT", "4-Bit Tri-State Bus", 4usize),
            ("BO_TRI_STATE_8BIT", "8-Bit Tri-State Bus", 8usize),
            ("BU_TRI_STATE_16BIT", "16-Bit Tri-State Bus", 16usize),
        ] {
            let h = (bits as f32 * 0.5).max(2.5);
            let mut inputs: Vec<PinDef> = (0..bits)
                .map(|i| {
                    let y = (bits as f32 - 1.0) * 0.5 - i as f32;
                    pin(&format!("IN_{i}"), [-3.0, y], false, false)
                })
                .collect();
            inputs.push(pin("ENABLE_0", [0.0, -h - 0.5], false, false));
            let outputs = (0..bits)
                .map(|i| {
                    let y = (bits as f32 - 1.0) * 0.5 - i as f32;
                    pin(&format!("OUT_{i}"), [3.0, y], true, false)
                })
                .collect();
            inv_conn.push(GateDef {
                type_name: name.into(),
                caption: cap.into(),
                category: "2 - Invert & Connect".into(),
                logic_type: "BUFFER".into(),
                inputs,
                outputs,
                half_size: [3.0, h + 0.5],
                shape_style: GateShapeStyle::TriState,
                out_inverted: false,
                in_inverted: false,
                default_gparams: HashMap::new(),
                default_lparams: map(&[("INPUT_BITS", &bits.to_string())]),
                custom_lines: Vec::new(),
            });
        }
        for &(name, cap, bits) in &[
            ("BUS_END_4", "4x1 Bus End", 4usize),
            ("BUS_END_8", "8x1 Bus End", 8usize),
            ("BUS_END_16", "16x1 Bus End", 16usize),
        ] {
            let h = (bits as f32 * 0.5).max(2.0);
            let inputs = (0..bits)
                .map(|i| {
                    let y = (bits as f32 - 1.0) * 0.5 - i as f32;
                    pin(&format!("BIT_{i}"), [-2.0, y], false, false)
                })
                .collect();
            let outputs = vec![pin("BUS", [2.0, 0.0], true, false)];
            inv_conn.push(GateDef {
                type_name: name.into(),
                caption: cap.into(),
                category: "2 - Invert & Connect".into(),
                logic_type: "BUS_END".into(),
                inputs,
                outputs,
                half_size: [2.0, h],
                shape_style: GateShapeStyle::BusEnd,
                out_inverted: false,
                in_inverted: false,
                default_gparams: HashMap::new(),
                default_lparams: map(&[("INPUT_BITS", &bits.to_string())]),
                custom_lines: Vec::new(),
            });
        }
        inv_conn.push(GateDef {
            type_name: "CA_SMALL_TGATE".into(),
            caption: "T-Gate (Transmission Gate)".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "TGATE".into(),
            inputs: vec![
                pin("IN_0", [-1.8, 0.0], false, false),
                pin("ENABLE_0", [0.0, 1.8], false, false),
            ],
            outputs: vec![pin("OUT_0", [1.8, 0.0], true, false)],
            half_size: [1.8, 1.8],
            shape_style: GateShapeStyle::TGate,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: HashMap::new(),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "DA_FROM".into(),
            caption: "\"From\" Connector (Input / 1 per Name)".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "FROM".into(),
            inputs: vec![pin("IN", [-2.5, 0.0], false, false)],
            outputs: Vec::new(),
            half_size: [2.5, 1.1],
            shape_style: GateShapeStyle::FromLink,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("JUNCTION_ID", "NET_A"), ("TAG_SCOPE", "SHARED")]),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "DE_TO".into(),
            caption: "\"To\" Connector (Output / Multi)".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "TO".into(),
            inputs: Vec::new(),
            outputs: vec![pin("OUT", [2.5, 0.0], true, false)],
            half_size: [2.5, 1.1],
            shape_style: GateShapeStyle::ToLink,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("JUNCTION_ID", "NET_A"), ("TAG_SCOPE", "SHARED")]),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "HA_JUNC_2".into(),
            caption: "2-way Junction".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "NODE".into(),
            inputs: vec![pin("IN_0", [-1.5, 0.0], false, false)],
            outputs: vec![
                pin("OUT_0", [1.5, 0.0], true, false),
                pin("OUT_1", [0.0, 1.5], true, false),
            ],
            half_size: [1.5, 1.5],
            shape_style: GateShapeStyle::JunctionNode,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: HashMap::new(),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "HE_JUNC_4".into(),
            caption: "4-way Junction".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "NODE".into(),
            inputs: vec![pin("IN_0", [-1.5, 0.0], false, false)],
            outputs: vec![
                pin("OUT_0", [1.5, 0.0], true, false),
                pin("OUT_1", [0.0, 1.5], true, false),
                pin("OUT_2", [0.0, -1.5], true, false),
            ],
            half_size: [1.5, 1.5],
            shape_style: GateShapeStyle::JunctionNode,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: HashMap::new(),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "VA_NMOS".into(),
            caption: "Simplified nMOS Transistor".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "TGATE".into(),
            inputs: vec![
                pin("IN_0", [0.0, -2.5], false, false),
                pin("ENABLE_0", [-2.5, 0.0], false, false),
            ],
            outputs: vec![pin("OUT_0", [0.0, 2.5], true, false)],
            half_size: [2.5, 2.5],
            shape_style: GateShapeStyle::Transistor { pmos: false },
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: HashMap::new(),
            custom_lines: Vec::new(),
        });
        inv_conn.push(GateDef {
            type_name: "VE_PMOS".into(),
            caption: "Simplified pMOS Transistor".into(),
            category: "2 - Invert & Connect".into(),
            logic_type: "TGATE".into(),
            inputs: vec![
                pin("IN_0", [0.0, 2.5], false, false),
                pin("ENABLE_0", [-2.5, 0.0], false, true),
            ],
            outputs: vec![pin("OUT_0", [0.0, -2.5], true, false)],
            half_size: [2.5, 2.5],
            shape_style: GateShapeStyle::Transistor { pmos: true },
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("ACTIVE_LOW_ENABLE", "true")]),
            custom_lines: Vec::new(),
        });
        categories.push(GateLibraryCategory {
            name: "2 - Invert & Connect".to_string(),
            gates: inv_conn,
        });

        // 3 - Input and Output
        let mut io_gates = Vec::new();
        io_gates.push(GateDef {
            type_name: "AA_TOGGLE".into(),
            caption: "On / Off Toggle Switch".into(),
            category: "3 - Input and Output".into(),
            logic_type: "DRIVER".into(),
            inputs: Vec::new(),
            outputs: vec![pin("OUT", [2.5, 0.0], true, false)],
            half_size: [2.5, 1.5],
            shape_style: GateShapeStyle::ToggleSwitch,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("OUTPUT_BITS", "1"), ("OUTPUT_NUM", "0")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "BB_CLOCK".into(),
            caption: "Variable-Rate Square Wave Clock".into(),
            category: "3 - Input and Output".into(),
            logic_type: "CLOCK".into(),
            inputs: Vec::new(),
            outputs: vec![pin("CLK", [2.5, 0.0], true, false)],
            half_size: [2.5, 1.5],
            shape_style: GateShapeStyle::Clock,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("HALF_CYCLE", "5")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "CC_PULSE".into(),
            caption: "Single-Pulse Generator".into(),
            category: "3 - Input and Output".into(),
            logic_type: "PULSE".into(),
            inputs: Vec::new(),
            outputs: vec![pin("OUT_0", [2.5, 0.0], true, false)],
            half_size: [2.5, 1.5],
            shape_style: GateShapeStyle::Pulse,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("PULSE_WIDTH", "4")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "DD_KEYPAD_HEX".into(),
            caption: "4-bit Hex Keypad".into(),
            category: "3 - Input and Output".into(),
            logic_type: "DRIVER".into(),
            inputs: Vec::new(),
            outputs: vec![
                pin("OUT_3", [3.5, 1.5], true, false),
                pin("OUT_2", [3.5, 0.5], true, false),
                pin("OUT_1", [3.5, -0.5], true, false),
                pin("OUT_0", [3.5, -1.5], true, false),
            ],
            half_size: [3.5, 3.0],
            shape_style: GateShapeStyle::HexKeypad,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("OUTPUT_BITS", "4"), ("OUTPUT_NUM", "0")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "EE_VDD".into(),
            caption: "5V Power (Vdd = 1)".into(),
            category: "3 - Input and Output".into(),
            logic_type: "DRIVER".into(),
            inputs: Vec::new(),
            outputs: vec![pin("OUT_0", [0.0, -2.0], true, false)],
            half_size: [1.5, 2.0],
            shape_style: GateShapeStyle::Vdd,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("OUTPUT_BITS", "1"), ("OUTPUT_NUM", "1")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "FF_GND".into(),
            caption: "Ground (GND = 0)".into(),
            category: "3 - Input and Output".into(),
            logic_type: "DRIVER".into(),
            inputs: Vec::new(),
            outputs: vec![pin("OUT_0", [0.0, 2.0], true, false)],
            half_size: [1.5, 2.0],
            shape_style: GateShapeStyle::Ground,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("OUTPUT_BITS", "1"), ("OUTPUT_NUM", "0")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "GA_LED".into(),
            caption: "LED Indicator".into(),
            category: "3 - Input and Output".into(),
            logic_type: "NODE".into(),
            inputs: vec![pin("IN_0", [-2.0, 0.0], false, false)],
            outputs: Vec::new(),
            half_size: [2.0, 1.5],
            shape_style: GateShapeStyle::Led,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: HashMap::new(),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "GE_LED_DISPLAY_4BIT".into(),
            caption: "4-Bit Hex LED Display".into(),
            category: "3 - Input and Output".into(),
            logic_type: "REGISTER".into(),
            inputs: vec![
                pin("IN_3", [-3.0, 1.5], false, false),
                pin("IN_2", [-3.0, 0.5], false, false),
                pin("IN_1", [-3.0, -0.5], false, false),
                pin("IN_0", [-3.0, -1.5], false, false),
            ],
            outputs: Vec::new(),
            half_size: [3.0, 3.0],
            shape_style: GateShapeStyle::LedDisplay { digits: 1 },
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "4"), ("CURRENT_VALUE", "0")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "GF_LED_DISPLAY_4BIT_OUT".into(),
            caption: "4-Bit LED Display (Pass-Through)".into(),
            category: "3 - Input and Output".into(),
            logic_type: "REGISTER".into(),
            inputs: vec![
                pin("IN_3", [-3.0, 1.5], false, false),
                pin("IN_2", [-3.0, 0.5], false, false),
                pin("IN_1", [-3.0, -0.5], false, false),
                pin("IN_0", [-3.0, -1.5], false, false),
            ],
            outputs: vec![
                pin("OUT_3", [3.0, 1.5], true, false),
                pin("OUT_2", [3.0, 0.5], true, false),
                pin("OUT_1", [3.0, -0.5], true, false),
                pin("OUT_0", [3.0, -1.5], true, false),
            ],
            half_size: [3.0, 3.0],
            shape_style: GateShapeStyle::LedDisplay { digits: 1 },
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "4"), ("CURRENT_VALUE", "0")]),
            custom_lines: Vec::new(),
        });
        io_gates.push(GateDef {
            type_name: "GI_LED_DISPLAY_8BIT".into(),
            caption: "8-Bit Hex LED Display".into(),
            category: "3 - Input and Output".into(),
            logic_type: "REGISTER".into(),
            inputs: (0..8)
                .map(|i| pin(&format!("IN_{}", 7 - i), [-4.0, 3.5 - i as f32], false, false))
                .collect(),
            outputs: Vec::new(),
            half_size: [4.0, 4.0],
            shape_style: GateShapeStyle::LedDisplay { digits: 2 },
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "8"), ("CURRENT_VALUE", "0")]),
            custom_lines: Vec::new(),
        });
        categories.push(GateLibraryCategory {
            name: "3 - Input and Output".to_string(),
            gates: io_gates,
        });

        // 4 - Text
        categories.push(GateLibraryCategory {
            name: "4 - Text".to_string(),
            gates: vec![GateDef {
                type_name: "AA_LABEL".into(),
                caption: "Text Label".into(),
                category: "4 - Text".into(),
                logic_type: "LABEL".into(),
                inputs: Vec::new(),
                outputs: Vec::new(),
                half_size: [3.5, 1.2],
                shape_style: GateShapeStyle::Label,
                out_inverted: false,
                in_inverted: false,
                default_gparams: map(&[("LABEL_TEXT", "Circuit Note"), ("TEXT_HEIGHT", "1.5")]),
                default_lparams: HashMap::new(),
                custom_lines: Vec::new(),
            }],
        });

        // 5 - MUX and Decoder
        let mut mux_dec = Vec::new();
        for &(name, cap, inputs_n, sel_n) in &[
            ("AA_MUX_2x1", "2x1 Multiplexer", 2usize, 1usize),
            ("AE_MUX_4x1", "4x1 Multiplexer", 4usize, 2usize),
            ("AI_MUX_8x1", "8x1 Multiplexer", 8usize, 3usize),
            ("AM_MUX_16x1", "16x1 Multiplexer", 16usize, 4usize),
        ] {
            let h = (inputs_n as f32 * 0.55).max(2.5);
            let mut inputs = Vec::new();
            for i in 0..inputs_n {
                let y = (inputs_n as f32 - 1.0) * 0.5 - i as f32;
                inputs.push(pin(&format!("IN_{i}"), [-3.0, y], false, false));
            }
            for s in 0..sel_n {
                let x = -1.0 + s as f32 * 1.0;
                inputs.push(pin(&format!("SEL_{s}"), [x, -h - 0.5], false, false));
            }
            mux_dec.push(GateDef {
                type_name: name.into(),
                caption: cap.into(),
                category: "5 - MUX and Decoder".into(),
                logic_type: "MUX".into(),
                inputs,
                outputs: vec![pin("OUT", [3.0, 0.0], true, false)],
                half_size: [3.0, h + 0.5],
                shape_style: GateShapeStyle::Mux,
                out_inverted: false,
                in_inverted: false,
                default_gparams: HashMap::new(),
                default_lparams: map(&[("INPUT_BITS", &inputs_n.to_string())]),
                custom_lines: Vec::new(),
            });
        }
        for &(name, cap, sel_n, outs_n) in &[
            ("BA_DECODER_2x4", "2x4 Decoder", 2usize, 4usize),
            ("BE_DECODER_3x8", "3x8 Decoder", 3usize, 8usize),
            ("BI_DECODER_4x16", "4x16 Decoder", 4usize, 16usize),
        ] {
            let h = (outs_n as f32 * 0.55).max(3.0);
            let mut inputs: Vec<PinDef> = (0..sel_n)
                .map(|i| {
                    let y = (sel_n as f32 - 1.0) * 0.5 - i as f32;
                    pin(&format!("IN_{i}"), [-3.0, y], false, false)
                })
                .collect();
            inputs.push(pin("ENABLE", [-3.0, -h + 0.5], false, false));
            let outputs = (0..outs_n)
                .map(|i| {
                    let y = (outs_n as f32 - 1.0) * 0.5 - i as f32;
                    pin(&format!("OUT_{i}"), [3.0, y], true, false)
                })
                .collect();
            mux_dec.push(GateDef {
                type_name: name.into(),
                caption: cap.into(),
                category: "5 - MUX and Decoder".into(),
                logic_type: "DECODER".into(),
                inputs,
                outputs,
                half_size: [3.0, h],
                shape_style: GateShapeStyle::Decoder,
                out_inverted: false,
                in_inverted: false,
                default_gparams: HashMap::new(),
                default_lparams: map(&[("INPUT_BITS", &sel_n.to_string())]),
                custom_lines: Vec::new(),
            });
        }
        for &(name, cap, ins_n, outs_n) in &[
            ("PRI_ENC_4x2", "4x2 Priority Encoder", 4usize, 2usize),
            ("PRI_ENC_8x3", "8x3 Priority Encoder", 8usize, 3usize),
            ("PRI_ENC_16x4", "16x4 Priority Encoder", 16usize, 4usize),
        ] {
            let h = (ins_n as f32 * 0.55).max(3.0);
            let inputs = (0..ins_n)
                .map(|i| {
                    let y = (ins_n as f32 - 1.0) * 0.5 - i as f32;
                    pin(&format!("IN_{i}"), [-3.0, y], false, false)
                })
                .collect();
            let mut outputs: Vec<PinDef> = (0..outs_n)
                .map(|i| {
                    let y = (outs_n as f32 - 1.0) * 0.5 - i as f32;
                    pin(&format!("OUT_{i}"), [3.0, y], true, false)
                })
                .collect();
            outputs.push(pin("VALID", [3.0, -h + 0.5], true, false));
            mux_dec.push(GateDef {
                type_name: name.into(),
                caption: cap.into(),
                category: "5 - MUX and Decoder".into(),
                logic_type: "PRI_ENCODER".into(),
                inputs,
                outputs,
                half_size: [3.0, h],
                shape_style: GateShapeStyle::PriEncoder,
                out_inverted: false,
                in_inverted: false,
                default_gparams: HashMap::new(),
                default_lparams: map(&[("INPUT_BITS", &ins_n.to_string())]),
                custom_lines: Vec::new(),
            });
        }
        categories.push(GateLibraryCategory {
            name: "5 - MUX and Decoder".to_string(),
            gates: mux_dec,
        });

        // 6 - Add & Compare
        let mut add_cmp = Vec::new();
        add_cmp.push(GateDef {
            type_name: "AA_FULLADDER_1BIT".into(),
            caption: "1-Bit Full Adder".into(),
            category: "6 - Add & Compare".into(),
            logic_type: "ADDER".into(),
            inputs: vec![
                pin("carry_in", [0.0, 3.0], false, false),
                pin("A_0", [-3.0, 1.0], false, false),
                pin("B_0", [-3.0, -1.0], false, false),
            ],
            outputs: vec![
                pin("sum_0", [3.0, 0.0], true, false),
                pin("carry_out", [0.0, -3.0], true, false),
            ],
            half_size: [3.0, 3.0],
            shape_style: GateShapeStyle::Adder,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "1")]),
            custom_lines: Vec::new(),
        });
        add_cmp.push(GateDef {
            type_name: "AE_FULLADDER_4BIT".into(),
            caption: "4-Bit Full Adder".into(),
            category: "6 - Add & Compare".into(),
            logic_type: "ADDER".into(),
            inputs: vec![
                pin("carry_in", [0.0, 5.0], false, false),
                pin("A_3", [-3.5, 3.5], false, false),
                pin("A_2", [-3.5, 2.5], false, false),
                pin("A_1", [-3.5, 1.5], false, false),
                pin("A_0", [-3.5, 0.5], false, false),
                pin("B_3", [-3.5, -0.5], false, false),
                pin("B_2", [-3.5, -1.5], false, false),
                pin("B_1", [-3.5, -2.5], false, false),
                pin("B_0", [-3.5, -3.5], false, false),
            ],
            outputs: vec![
                pin("sum_3", [3.5, 1.5], true, false),
                pin("sum_2", [3.5, 0.5], true, false),
                pin("sum_1", [3.5, -0.5], true, false),
                pin("sum_0", [3.5, -1.5], true, false),
                pin("carry_out", [0.0, -5.0], true, false),
            ],
            half_size: [3.5, 5.0],
            shape_style: GateShapeStyle::Adder,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "4")]),
            custom_lines: Vec::new(),
        });
        add_cmp.push(GateDef {
            type_name: "BE_COMPARATOR_4BIT".into(),
            caption: "4-Bit Magnitude Comparator".into(),
            category: "6 - Add & Compare".into(),
            logic_type: "COMPARE".into(),
            inputs: vec![
                pin("A_3", [-3.5, 4.0], false, false),
                pin("A_2", [-3.5, 3.0], false, false),
                pin("A_1", [-3.5, 2.0], false, false),
                pin("A_0", [-3.5, 1.0], false, false),
                pin("B_3", [-3.5, -1.0], false, false),
                pin("B_2", [-3.5, -2.0], false, false),
                pin("B_1", [-3.5, -3.0], false, false),
                pin("B_0", [-3.5, -4.0], false, false),
                pin("in_A_greater_B", [0.0, 5.0], false, false),
                pin("in_A_equal_B", [-1.5, 5.0], false, false),
                pin("in_A_less_B", [1.5, 5.0], false, false),
            ],
            outputs: vec![
                pin("A_greater_B", [3.5, 1.5], true, false),
                pin("A_equal_B", [3.5, 0.0], true, false),
                pin("A_less_B", [3.5, -1.5], true, false),
            ],
            half_size: [3.5, 5.0],
            shape_style: GateShapeStyle::Comparator,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "4")]),
            custom_lines: Vec::new(),
        });
        categories.push(GateLibraryCategory {
            name: "6 - Add & Compare".to_string(),
            gates: add_cmp,
        });

        // 7 - Flip Flops
        let mut ffs = Vec::new();
        ffs.push(GateDef {
            type_name: "AE_DFF_LOW".into(),
            caption: "D Flip-Flop (Low Active Set/Reset)".into(),
            category: "7 - Flip Flops".into(),
            logic_type: "REGISTER".into(),
            inputs: vec![
                pin("IN_0", [-3.0, 1.5], false, false),
                pin("clock", [-3.0, -1.5], false, false),
                pin("set", [0.0, 3.5], false, true),
                pin("clear", [0.0, -3.5], false, true),
            ],
            outputs: vec![
                pin("OUT_0", [3.0, 1.5], true, false),
                pin("OUTINV_0", [3.0, -1.5], true, true),
            ],
            half_size: [3.0, 3.5],
            shape_style: GateShapeStyle::FlipFlopD,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[
                ("INPUT_BITS", "1"),
                ("SYNC_SET", "false"),
                ("SYNC_CLEAR", "false"),
            ]),
            custom_lines: Vec::new(),
        });
        ffs.push(GateDef {
            type_name: "AE_DFF_LOW_NT".into(),
            caption: "D Flip-Flop (Negative Edge)".into(),
            category: "7 - Flip Flops".into(),
            logic_type: "REGISTER".into(),
            inputs: vec![
                pin("IN_0", [-3.0, 1.5], false, false),
                pin("clock", [-3.0, -1.5], false, true),
                pin("set", [0.0, 3.5], false, true),
                pin("clear", [0.0, -3.5], false, true),
            ],
            outputs: vec![
                pin("OUT_0", [3.0, 1.5], true, false),
                pin("OUTINV_0", [3.0, -1.5], true, true),
            ],
            half_size: [3.0, 3.5],
            shape_style: GateShapeStyle::FlipFlopD,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[
                ("INPUT_BITS", "1"),
                ("INVERTED_CLOCK", "true"),
            ]),
            custom_lines: Vec::new(),
        });
        ffs.push(GateDef {
            type_name: "AF_DFF_LOW".into(),
            caption: "D Flip-Flop (Clock Enabled)".into(),
            category: "7 - Flip Flops".into(),
            logic_type: "REGISTER".into(),
            inputs: vec![
                pin("IN_0", [-3.0, 2.0], false, false),
                pin("clock_enable", [-3.0, 0.0], false, false),
                pin("clock", [-3.0, -2.0], false, false),
                pin("set", [0.0, 3.5], false, true),
                pin("clear", [0.0, -3.5], false, true),
            ],
            outputs: vec![
                pin("OUT_0", [3.0, 1.5], true, false),
                pin("OUTINV_0", [3.0, -1.5], true, true),
            ],
            half_size: [3.0, 3.5],
            shape_style: GateShapeStyle::FlipFlopD,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "1")]),
            custom_lines: Vec::new(),
        });
        ffs.push(GateDef {
            type_name: "BE_JKFF_LOW".into(),
            caption: "JK Flip-Flop (Low Active Set/Reset)".into(),
            category: "7 - Flip Flops".into(),
            logic_type: "JKFF".into(),
            inputs: vec![
                pin("J", [-3.0, 2.0], false, false),
                pin("clock", [-3.0, 0.0], false, false),
                pin("K", [-3.0, -2.0], false, false),
                pin("set", [0.0, 3.5], false, true),
                pin("clear", [0.0, -3.5], false, true),
            ],
            outputs: vec![
                pin("Q", [3.0, 1.5], true, false),
                pin("Q_bar", [3.0, -1.5], true, true),
            ],
            half_size: [3.0, 3.5],
            shape_style: GateShapeStyle::FlipFlopJK,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("SYNC_SET", "false"), ("SYNC_CLEAR", "false")]),
            custom_lines: Vec::new(),
        });
        ffs.push(GateDef {
            type_name: "BE_JKFF_LOW_NT".into(),
            caption: "JK Flip-Flop (Negative Edge)".into(),
            category: "7 - Flip Flops".into(),
            logic_type: "JKFF".into(),
            inputs: vec![
                pin("J", [-3.0, 2.0], false, false),
                pin("clock", [-3.0, 0.0], false, true),
                pin("K", [-3.0, -2.0], false, false),
                pin("set", [0.0, 3.5], false, true),
                pin("clear", [0.0, -3.5], false, true),
            ],
            outputs: vec![
                pin("Q", [3.0, 1.5], true, false),
                pin("Q_bar", [3.0, -1.5], true, true),
            ],
            half_size: [3.0, 3.5],
            shape_style: GateShapeStyle::FlipFlopJK,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INVERTED_CLOCK", "true")]),
            custom_lines: Vec::new(),
        });
        categories.push(GateLibraryCategory {
            name: "7 - Flip Flops".to_string(),
            gates: ffs,
        });

        // 8 - Registers
        let mut regs = Vec::new();
        for &(name, cap, bits) in &[
            ("AA_REGISTER4", "4-Bit Counting Register", 4usize),
            ("AE_REGISTER8", "8-Bit Counting Register", 8usize),
            ("AI_REGISTER12", "12-Bit Counting Register", 12usize),
            ("AM_REGISTER16", "16-Bit Counting Register", 16usize),
        ] {
            let h = (bits as f32 * 0.5 + 3.0).max(5.0);
            let mut inputs: Vec<PinDef> = (0..bits)
                .map(|i| {
                    let y = h - 1.0 - i as f32;
                    pin(&format!("IN_{}", bits - 1 - i), [-3.5, y], false, false)
                })
                .collect();
            inputs.push(pin("load", [-3.5, -h + 4.0], false, false));
            inputs.push(pin("count_enable", [-3.5, -h + 3.0], false, false));
            inputs.push(pin("up_down", [-3.5, -h + 2.0], false, false));
            inputs.push(pin("clock", [-3.5, -h + 1.0], false, false));
            inputs.push(pin("clear", [0.0, -h], false, false));

            let mut outputs: Vec<PinDef> = (0..bits)
                .map(|i| {
                    let y = h - 1.5 - i as f32;
                    pin(&format!("OUT_{}", bits - 1 - i), [3.5, y], true, false)
                })
                .collect();
            outputs.push(pin("carry_out", [3.5, -h + 1.0], true, false));

            regs.push(GateDef {
                type_name: name.into(),
                caption: cap.into(),
                category: "8 - Registers".into(),
                logic_type: "REGISTER".into(),
                inputs,
                outputs,
                half_size: [3.5, h],
                shape_style: GateShapeStyle::Register,
                out_inverted: false,
                in_inverted: false,
                default_gparams: HashMap::new(),
                default_lparams: map(&[
                    ("INPUT_BITS", &bits.to_string()),
                    ("CURRENT_VALUE", "0"),
                ]),
                custom_lines: Vec::new(),
            });
        }
        regs.push(GateDef {
            type_name: "BA_SHIFT_REGISTER_4".into(),
            caption: "4-Bit Shift Register".into(),
            category: "8 - Registers".into(),
            logic_type: "REGISTER".into(),
            inputs: vec![
                pin("IN_3", [-3.5, 2.5], false, false),
                pin("IN_2", [-3.5, 1.5], false, false),
                pin("IN_1", [-3.5, 0.5], false, false),
                pin("IN_0", [-3.5, -0.5], false, false),
                pin("shift_left", [-3.5, -1.5], false, false),
                pin("shift_enable", [-3.5, -2.5], false, false),
                pin("load", [-3.5, -3.5], false, false),
                pin("clock", [-3.5, -4.5], false, false),
                pin("clear", [0.0, -5.0], false, false),
            ],
            outputs: vec![
                pin("OUT_3", [3.5, 2.5], true, false),
                pin("OUT_2", [3.5, 1.5], true, false),
                pin("OUT_1", [3.5, 0.5], true, false),
                pin("OUT_0", [3.5, -0.5], true, false),
            ],
            half_size: [3.5, 5.0],
            shape_style: GateShapeStyle::Register,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "4"), ("CURRENT_VALUE", "0")]),
            custom_lines: Vec::new(),
        });
        categories.push(GateLibraryCategory {
            name: "8 - Registers".to_string(),
            gates: regs,
        });

        // 9 - RAM and ROM
        let mut mems = Vec::new();
        for &(name, cap, addr_bits, data_bits, is_rom) in &[
            ("AA_RAM_4x4", "4x4 RAM", 4usize, 4usize, false),
            ("AE_RAM_8x8", "8x8 RAM", 8usize, 8usize, false),
            ("AI_RAM_12x16", "12x16 RAM", 12usize, 16usize, false),
            ("AM_RAM_16x16", "16x16 RAM", 16usize, 16usize, false),
            ("BA_ROM_4x4", "4x4 ROM", 4usize, 4usize, true),
            ("BE_ROM_8x8", "8x8 ROM", 8usize, 8usize, true),
            ("BI_ROM_12x16", "12x16 ROM", 12usize, 16usize, true),
            ("BM_ROM_16x16", "16x16 ROM", 16usize, 16usize, true),
        ] {
            let max_pins = (addr_bits + if is_rom { 1 } else { data_bits + 3 }).max(data_bits);
            let h = (max_pins as f32 * 0.45).max(4.0);
            let mut inputs = Vec::new();
            for i in 0..addr_bits {
                let y = h - 0.8 - i as f32 * 0.7;
                inputs.push(pin(&format!("ADDRESS_{}", addr_bits - 1 - i), [-4.0, y], false, false));
            }
            if !is_rom {
                for i in 0..data_bits.min(8) {
                    let y = -0.5 - i as f32 * 0.7;
                    inputs.push(pin(&format!("DATA_IN_{}", data_bits.min(8) - 1 - i), [-4.0, y], false, false));
                }
                inputs.push(pin("write_enable", [-2.0, -h], false, false));
                inputs.push(pin("write_clock", [0.0, -h], false, false));
            }
            inputs.push(pin("enable", [2.0, -h], false, false));

            let outputs = (0..data_bits.min(8))
                .map(|i| {
                    let y = (data_bits.min(8) as f32 - 1.0) * 0.4 - i as f32 * 0.8;
                    pin(&format!("DATA_OUT_{}", data_bits.min(8) - 1 - i), [4.0, y], true, false)
                })
                .collect();

            mems.push(GateDef {
                type_name: name.into(),
                caption: cap.into(),
                category: "9 - RAM and ROM".into(),
                logic_type: "RAM".into(),
                inputs,
                outputs,
                half_size: [4.0, h],
                shape_style: GateShapeStyle::RamRom { is_rom },
                out_inverted: false,
                in_inverted: false,
                default_gparams: HashMap::new(),
                default_lparams: map(&[
                    ("ADDRESS_BITS", &addr_bits.to_string()),
                    ("DATA_BITS", &data_bits.to_string()),
                ]),
                custom_lines: Vec::new(),
            });
        }
        categories.push(GateLibraryCategory {
            name: "9 - RAM and ROM".to_string(),
            gates: mems,
        });

        // 10 - Chips
        let mut chips = Vec::new();
        chips.push(GateDef {
            type_name: "PAUSEULATOR".into(),
            caption: "Pauseulator (Breakpoint Gate)".into(),
            category: "10 - Chips".into(),
            logic_type: "Pauseulator".into(),
            inputs: vec![pin("SIGNAL", [-3.0, 0.0], false, false)],
            outputs: Vec::new(),
            half_size: [3.0, 1.5],
            shape_style: GateShapeStyle::Pauseulator,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: HashMap::new(),
            custom_lines: Vec::new(),
        });
        chips.push(GateDef {
            type_name: "CHIP_74LS138".into(),
            caption: "74LS138 (3x8 Decoder DIP)".into(),
            category: "10 - Chips".into(),
            logic_type: "DECODER".into(),
            inputs: vec![
                pin("IN_0", [-3.5, 2.0], false, false),
                pin("IN_1", [-3.5, 1.0], false, false),
                pin("IN_2", [-3.5, 0.0], false, false),
                pin("ENABLE", [-3.5, -1.5], false, false),
            ],
            outputs: (0..8)
                .map(|i| pin(&format!("OUT_{i}"), [3.5, 3.5 - i as f32], true, true))
                .collect(),
            half_size: [3.5, 4.2],
            shape_style: GateShapeStyle::ChipBox,
            out_inverted: true,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("INPUT_BITS", "3")]),
            custom_lines: Vec::new(),
        });
        categories.push(GateLibraryCategory {
            name: "10 - Chips".to_string(),
            gates: chips,
        });

        // 11 - Custom Components (User & AI Imported / Created Components)
        let mut custom_specs = HashMap::new();
        let mut custom_gates = Vec::new();
        for spec in builtin_custom_templates() {
            if let Ok(norm) = spec.normalized() {
                custom_gates.push(norm.to_gate_def());
                custom_specs.insert(norm.id.clone(), norm);
            }
        }
        categories.push(GateLibraryCategory {
            name: "11 - Custom Components".to_string(),
            gates: custom_gates,
        });

        // 12 - CPU & Peripherals (Programmable CPU Core, Memory-Mapped Displays & Keyboard Input)
        let mut cpu_periphs = Vec::new();
        cpu_periphs.push(GateDef {
            type_name: "CPU_COMPONENT".into(),
            caption: "Programmable CPU Core (RV32I / LC16 / MIN8)".into(),
            category: "12 - CPU & Peripherals".into(),
            logic_type: "CPU".into(),
            inputs: vec![
                pin("CLK", [-5.5, 4.0], false, false),
                pin("RST", [-5.5, 2.5], false, false),
                pin("IN_3", [-5.5, 0.5], false, false),
                pin("IN_2", [-5.5, -0.5], false, false),
                pin("IN_1", [-5.5, -1.5], false, false),
                pin("IN_0", [-5.5, -2.5], false, false),
            ],
            outputs: vec![
                pin("ADDR_3", [5.5, 4.5], true, false),
                pin("ADDR_2", [5.5, 3.5], true, false),
                pin("ADDR_1", [5.5, 2.5], true, false),
                pin("ADDR_0", [5.5, 1.5], true, false),
                pin("DATA_3", [5.5, 0.0], true, false),
                pin("DATA_2", [5.5, -1.0], true, false),
                pin("DATA_1", [5.5, -2.0], true, false),
                pin("DATA_0", [5.5, -3.0], true, false),
                pin("RW", [5.5, -4.2], true, false),
                pin("HALT", [5.5, -5.2], true, false),
            ],
            half_size: [5.5, 6.0],
            shape_style: GateShapeStyle::CpuProcessor,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[
                ("CPU_ARCH", "RV32I"),
                ("CLOCK_MODE", "INTERNAL"),
                ("CLOCK_HZ", "150000"),
            ]),
            custom_lines: Vec::new(),
        });
        cpu_periphs.push(GateDef {
            type_name: "DISPLAY_COMPONENT".into(),
            caption: "VRAM Graphical Display (MMIO Monitor)".into(),
            category: "12 - CPU & Peripherals".into(),
            logic_type: "VRAM_DISPLAY".into(),
            inputs: vec![
                pin("CLK", [-9.0, 3.0], false, false),
                pin("WE", [-9.0, 1.0], false, false),
                pin("RST", [-9.0, -1.0], false, false),
                pin("EN", [-9.0, -3.0], false, false),
            ],
            outputs: vec![pin("VSYNC", [9.0, 0.0], true, false)],
            half_size: [9.0, 6.0],
            shape_style: GateShapeStyle::VramDisplay,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[
                ("VRAM_BASE", "0x8000"),
                ("RESOLUTION", "64x32"),
                ("PALETTE", "MATRIX_GREEN"),
            ]),
            custom_lines: Vec::new(),
        });
        cpu_periphs.push(GateDef {
            type_name: "KEYBOARD_COMPONENT".into(),
            caption: "Keyboard Input Controller (MMIO W/S/Up/Down)".into(),
            category: "12 - CPU & Peripherals".into(),
            logic_type: "KEYBOARD_INPUT".into(),
            inputs: vec![pin("RST", [-5.0, 0.0], false, false)],
            outputs: vec![
                pin("P1_W", [5.0, 2.4], true, false),
                pin("P1_S", [5.0, 1.2], true, false),
                pin("P2_UP", [5.0, 0.0], true, false),
                pin("P2_DN", [5.0, -1.2], true, false),
                pin("START", [5.0, -2.4], true, false),
            ],
            half_size: [5.0, 3.8],
            shape_style: GateShapeStyle::KeyboardInput,
            out_inverted: false,
            in_inverted: false,
            default_gparams: HashMap::new(),
            default_lparams: map(&[("MMIO_ADDR", "0xF000")]),
            custom_lines: Vec::new(),
        });
        categories.push(GateLibraryCategory {
            name: "12 - CPU & Peripherals".to_string(),
            gates: cpu_periphs,
        });

        let mut by_name = HashMap::new();
        for cat in &categories {
            for gate in &cat.gates {
                by_name.insert(gate.type_name.clone(), gate.clone());
            }
        }

        Self {
            categories,
            by_name,
            custom_specs,
        }
    }

    pub fn get(&self, type_name: &str) -> Option<&GateDef> {
        self.by_name.get(type_name)
    }

    /// Registers or updates a Custom Component in `"11 - Custom Components"` and `by_name`.
    pub fn register_custom_component(&mut self, spec: CustomComponentSpec) -> Result<String, String> {
        let norm = spec.normalized()?;
        let id = norm.id.clone();
        let gate_def = norm.to_gate_def();
        self.by_name.insert(id.clone(), gate_def.clone());
        self.custom_specs.insert(id.clone(), norm);

        if let Some(cat) = self
            .categories
            .iter_mut()
            .find(|c| c.name.starts_with("11 - Custom"))
        {
            if let Some(existing) = cat.gates.iter_mut().find(|g| g.type_name == id) {
                *existing = gate_def;
            } else {
                cat.gates.push(gate_def);
            }
        } else {
            self.categories.push(GateLibraryCategory {
                name: "11 - Custom Components".to_string(),
                gates: vec![gate_def],
            });
        }
        Ok(id)
    }
}

fn pin(name: &str, pos: [f32; 2], is_output: bool, inverted: bool) -> PinDef {
    PinDef {
        name: name.to_string(),
        pos,
        is_output,
        inverted,
    }
}

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn make_n_input_gate(
    type_name: &str,
    caption: &str,
    category: &str,
    logic_type: &str,
    bits: usize,
    out_inverted: bool,
    in_inverted: bool,
    shape_style: GateShapeStyle,
) -> GateDef {
    let half_h = match bits {
        2 => 2.0,
        3 => 3.0,
        4 => 4.0,
        _ => 5.0,
    };
    let half_w = 2.0_f32;
    let inputs = (0..bits)
        .map(|i| {
            let y = match bits {
                2 => 1.0 - (i as f32) * 2.0,
                3 => 2.0 - (i as f32) * 2.0,
                4 => 3.0 - (i as f32) * 2.0,
                _ => 3.5 - (i as f32) * 1.0,
            };
            pin(&format!("IN_{i}"), [-half_w, y], false, in_inverted)
        })
        .collect();
    let outputs = vec![pin("OUT", [half_w, 0.0], true, out_inverted)];
    GateDef {
        type_name: type_name.to_string(),
        caption: caption.to_string(),
        category: category.to_string(),
        logic_type: logic_type.to_string(),
        inputs,
        outputs,
        half_size: [half_w, half_h],
        shape_style,
        out_inverted,
        in_inverted,
        default_gparams: HashMap::new(),
        default_lparams: map(&[("INPUT_BITS", &bits.to_string())]),
        custom_lines: Vec::new(),
    }
}
