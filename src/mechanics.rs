// Constants and parameters extracted from CedarLogic.exe (see app/MECHANICS.md and app/mechanics.json)

// Window & Layout Constants (function 0x44f9a0, function 0x4512b0, function 0x45c180)
pub const DEFAULT_FRAME_WIDTH: u32 = 600;
pub const DEFAULT_FRAME_HEIGHT: u32 = 600;
pub const INITIAL_FRAME_WIDTH: f32 = 1800.0;
pub const INITIAL_FRAME_HEIGHT: f32 = 900.0;
pub const DEFAULT_FRAME_LEFT: f32 = 20.0;
pub const DEFAULT_FRAME_TOP: f32 = 20.0;
pub const MINIMAP_WIDTH: f32 = 130.0;
pub const MINIMAP_HEIGHT: f32 = 100.0;
#[allow(dead_code)]
pub const OSCOPE_WIDTH: f32 = 800.0;
#[allow(dead_code)]
pub const OSCOPE_HEIGHT: f32 = 250.0;
pub const OSCOPE_GRID_STEP_X: usize = 20;
pub const OSCOPE_TRACE_LENGTH: usize = 180;

// Simulation & Timing Constants (function 0x44f9a0, function 0x4512b0, function 0x481040, function 0x4813e0)
pub const DEFAULT_TIME_STEP_MS: u32 = 25;
pub const MIN_TIME_STEP_MS: u32 = 1;
pub const MAX_TIME_STEP_MS: u32 = 500;
pub const DEFAULT_REFRESH_RATE_MS: u32 = 60;
pub const DEFAULT_GATE_DELAY: u32 = 1;
pub const DEFAULT_CLOCK_HALF_CYCLE: u32 = 5;

// Canvas & Rendering Constants (function 0x44f9a0, function 0x420b50, function 0x4471f0, function 0x43ad70)
pub const WIRE_CONN_RADIUS: f32 = 0.18;
pub const WIRE_CONN_VISIBLE: bool = true;
pub const GRIDLINE_VISIBLE: bool = true;
pub const GRID_SPACING: f32 = 0.5;
pub const DEFAULT_VIEW_ZOOM: f32 = 0.08;
pub const SINGLE_WIRE_WIDTH: f32 = 1.0;
pub const BUS_WIRE_WIDTH: f32 = 4.0;
#[allow(dead_code)]
pub const WIRE_HI_Z_GREEN: f32 = 0.78;
#[allow(dead_code)]
pub const WIRE_UNKNOWN_RG: f32 = 0.3;
pub const SOLDER_DOT_ANGLE_STEP_DEG: u32 = 30;

// 5-State Logic Values (function 0x43ad70, function 0x486eb0)
#[allow(dead_code)]
pub const STATE_ZERO: u8 = 0;
#[allow(dead_code)]
pub const STATE_ONE: u8 = 1;
#[allow(dead_code)]
pub const STATE_HI_Z: u8 = 2;
#[allow(dead_code)]
pub const STATE_CONFLICT: u8 = 3;
#[allow(dead_code)]
pub const STATE_UNKNOWN: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogicState {
    Zero = 0,
    One = 1,
    HiZ = 2,
    Conflict = 3,
    Unknown = 4,
}

impl LogicState {
    #[allow(dead_code)]
    pub fn from_u8(v: u8) -> Self {
        match v {
            STATE_ZERO => Self::Zero,
            STATE_ONE => Self::One,
            STATE_HI_Z => Self::HiZ,
            STATE_CONFLICT => Self::Conflict,
            _ => Self::Unknown,
        }
    }

    #[allow(dead_code)]
    pub fn to_u8(self) -> u8 {
        match self {
            Self::Zero => STATE_ZERO,
            Self::One => STATE_ONE,
            Self::HiZ => STATE_HI_Z,
            Self::Conflict => STATE_CONFLICT,
            Self::Unknown => STATE_UNKNOWN,
        }
    }

    pub fn invert(self) -> Self {
        match self {
            Self::Zero => Self::One,
            Self::One => Self::Zero,
            Self::HiZ | Self::Conflict | Self::Unknown => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Zero => "0 (LOW)",
            Self::One => "1 (HIGH)",
            Self::HiZ => "Z (HI-Z)",
            Self::Conflict => "X (CONFLICT)",
            Self::Unknown => "? (UNKNOWN)",
        }
    }

    /// Returns the technical Neo-Brutalist signal color for wires and pins:
    /// - `One` (HIGH): `#22C55E` (Green — positive/active signal)
    /// - `Zero` (LOW): `#4E5C6C` (Muted Slate — inactive low signal)
    /// - `HiZ` (High-Z): `#F59E0B` (Orange — intermediate/floating state)
    /// - `Conflict` (Short/Error): `#EF4444` (Red — alert/conflict state)
    /// - `Unknown` (Uninitialized): `#A78BFA` (Purple — special/unknown state)
    pub fn wire_color(self, dark_mode: bool) -> egui::Color32 {
        let p = crate::theme::palette(dark_mode);
        match self {
            Self::Zero => p.wire_low,
            Self::One => p.green,
            Self::HiZ => p.orange,
            Self::Conflict => p.red,
            Self::Unknown => p.purple,
        }
    }
}
