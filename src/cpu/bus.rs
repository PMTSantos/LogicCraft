#![allow(dead_code)]
//! Memory Bus and Memory-Mapped I/O (MMIO) controller (`src/cpu/bus.rs`).
//!
//! ## Unified 64 KB Address Space (`0x0000` .. `0xFFFF`)
//! - `0x0000 ..= 0x3FFF` (16 KB): Program ROM / Code Segment
//! - `0x4000 ..= 0x7FFF` (16 KB): Data RAM & Stack (`SP` defaults to `0x7FFC`)
//! - `0x8000 ..= 0xEFFF` (28 KB): Video RAM (VRAM) for multiple memory-mapped displays
//!   - Default Display #1 Base: `0x8000` (e.g. 64x32 = 2048 bytes, 64x64 = 4096 bytes, 128x64 = 8192 bytes)
//!   - Additional Displays can map to `0x8000` (mirror) or distinct bases (`0x8800`, `0x9000`, `0xA000`, etc.)
//! - `0xF000 ..= 0xFFFF` (4 KB): Memory-Mapped I/O (MMIO) Peripherals
//!   - `0xF000` (`MMIO_KEY_STATE`): Bitmask (`bit0=W`, `bit1=S`, `bit2=Up`, `bit3=Down`, `bit4=Space/Enter`)
//!   - `0xF001` (`MMIO_KEY_P1_UP`): Player 1 Up (`W`) -> `0` or `1`
//!   - `0xF002` (`MMIO_KEY_P1_DOWN`): Player 1 Down (`S`) -> `0` or `1`
//!   - `0xF003` (`MMIO_KEY_P2_UP`): Player 2 Up (`ArrowUp`) -> `0` or `1`
//!   - `0xF004` (`MMIO_KEY_P2_DOWN`): Player 2 Down (`ArrowDown`) -> `0` or `1`
//!   - `0xF005` (`MMIO_KEY_START`): Start / Reset (`Space` or `Enter`) -> `0` or `1`
//!   - `0xF006` (`MMIO_KEY_LAST`): Last pressed key ASCII/code
//!   - `0xF010` (`MMIO_RNG`): Pseudo-random 8/16-bit value (advances LCG on read)
//!   - `0xF012` (`MMIO_TICK_LO`): 32-bit tick/frame counter (little-endian `0xF012..0xF015`)
//!   - `0xF016` (`MMIO_VSYNC`): Returns `1` when a new display frame tick is ready, `0` otherwise
//!   - `0xF020` (`MMIO_P1_SCORE`): Player 1 Score register
//!   - `0xF021` (`MMIO_P2_SCORE`): Player 2 Score register
//!   - `0xF022` (`MMIO_GPIO_OUT`): 8-bit digital output port driving `CpuComponent` output pins
//!   - `0xF023` (`MMIO_GPIO_IN`): 8-bit digital input port read from `CpuComponent` input pins
//!   - `0xF030` (`MMIO_VRAM_CLEAR`): Writing byte `v` fast-fills active VRAM (`0x8000..0x9FFF`) with `v`

pub const MEM_SIZE: usize = 65536;
pub const ROM_BASE: u16 = 0x0000;
pub const ROM_END: u16 = 0x3FFF;
pub const RAM_BASE: u16 = 0x4000;
pub const RAM_END: u16 = 0x7FFF;
pub const DEFAULT_SP: u32 = 0x7FFC;
pub const VRAM_BASE: u16 = 0x8000;
pub const VRAM_END: u16 = 0xEFFF;
pub const MMIO_BASE: u16 = 0xF000;

pub const MMIO_KEY_STATE: u16 = 0xF000;
pub const MMIO_KEY_P1_UP: u16 = 0xF001;
pub const MMIO_KEY_P1_DOWN: u16 = 0xF002;
pub const MMIO_KEY_P2_UP: u16 = 0xF003;
pub const MMIO_KEY_P2_DOWN: u16 = 0xF004;
pub const MMIO_KEY_START: u16 = 0xF005;
pub const MMIO_KEY_LAST: u16 = 0xF006;

pub const MMIO_RNG: u16 = 0xF010;
pub const MMIO_TICK_LO: u16 = 0xF012;
pub const MMIO_VSYNC: u16 = 0xF016;
pub const MMIO_P1_SCORE: u16 = 0xF020;
pub const MMIO_P2_SCORE: u16 = 0xF021;
pub const MMIO_GPIO_OUT: u16 = 0xF022;
pub const MMIO_GPIO_IN: u16 = 0xF023;
pub const MMIO_VRAM_CLEAR: u16 = 0xF030;

pub const KEY_BIT_P1_UP: u8 = 1 << 0;
pub const KEY_BIT_P1_DOWN: u8 = 1 << 1;
pub const KEY_BIT_P2_UP: u8 = 1 << 2;
pub const KEY_BIT_P2_DOWN: u8 = 1 << 3;
pub const KEY_BIT_START: u8 = 1 << 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyboardState {
    pub p1_up: bool,
    pub p1_down: bool,
    pub p2_up: bool,
    pub p2_down: bool,
    pub start: bool,
    pub last_key: u8,
}

impl KeyboardState {
    pub fn to_bitmask(self) -> u8 {
        let mut mask = 0u8;
        if self.p1_up {
            mask |= KEY_BIT_P1_UP;
        }
        if self.p1_down {
            mask |= KEY_BIT_P1_DOWN;
        }
        if self.p2_up {
            mask |= KEY_BIT_P2_UP;
        }
        if self.p2_down {
            mask |= KEY_BIT_P2_DOWN;
        }
        if self.start {
            mask |= KEY_BIT_START;
        }
        mask
    }
}

#[derive(Debug, Clone)]
pub struct MemoryBus {
    pub data: Vec<u8>,
    pub keys: KeyboardState,
    pub rng_state: u32,
    pub tick_counter: u32,
    pub vsync_ready: bool,
    pub vsync_poll_count: u32,
    pub last_bus_addr: u16,
    pub last_bus_data: u8,
    pub last_bus_write: bool,
}

impl Default for MemoryBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryBus {
    pub fn new() -> Self {
        Self {
            data: vec![0u8; MEM_SIZE],
            keys: KeyboardState::default(),
            rng_state: 0xACE1_1337,
            tick_counter: 0,
            vsync_ready: true,
            vsync_poll_count: 0,
            last_bus_addr: 0,
            last_bus_data: 0,
            last_bus_write: false,
        }
    }

    pub fn reset(&mut self) {
        // Preserve loaded ROM (0x0000..=0x3FFF), clear RAM, VRAM, and MMIO
        for b in &mut self.data[RAM_BASE as usize..MEM_SIZE] {
            *b = 0;
        }
        self.rng_state = 0xACE1_1337;
        self.tick_counter = 0;
        self.vsync_ready = true;
        self.vsync_poll_count = 0;
        self.last_bus_addr = 0;
        self.last_bus_data = 0;
        self.last_bus_write = false;
        self.sync_keys_to_mmio();
    }

    pub fn clear_all(&mut self) {
        self.data.fill(0);
        self.reset();
    }

    pub fn load_program(&mut self, bytes: &[u8]) {
        self.data[0..=ROM_END as usize].fill(0);
        let len = bytes.len().min(MEM_SIZE);
        self.data[..len].copy_from_slice(&bytes[..len]);
    }

    pub fn update_keys(&mut self, keys: KeyboardState) {
        let mut merged = keys;
        if merged.last_key == 0 {
            merged.last_key = self.keys.last_key;
        }
        self.keys = merged;
        self.sync_keys_to_mmio();
    }

    pub fn sync_keys_to_mmio(&mut self) {
        let mask = self.keys.to_bitmask();
        self.data[MMIO_KEY_STATE as usize] = mask;
        self.data[MMIO_KEY_P1_UP as usize] = u8::from(self.keys.p1_up);
        self.data[MMIO_KEY_P1_DOWN as usize] = u8::from(self.keys.p1_down);
        self.data[MMIO_KEY_P2_UP as usize] = u8::from(self.keys.p2_up);
        self.data[MMIO_KEY_P2_DOWN as usize] = u8::from(self.keys.p2_down);
        self.data[MMIO_KEY_START as usize] = u8::from(self.keys.start);
        if self.keys.last_key != 0 {
            self.data[MMIO_KEY_LAST as usize] = self.keys.last_key;
        }
    }

    pub fn notify_frame_tick(&mut self) {
        self.tick_counter = self.tick_counter.wrapping_add(1);
        let bytes = self.tick_counter.to_le_bytes();
        let base = MMIO_TICK_LO as usize;
        self.data[base..base + 4].copy_from_slice(&bytes);
        self.vsync_ready = true;
        self.vsync_poll_count = 0;
    }

    fn next_rng_byte(&mut self) -> u8 {
        self.rng_state = self
            .rng_state
            .wrapping_mul(1664525)
            .wrapping_add(1013904223);
        ((self.rng_state >> 16) & 0xFF) as u8
    }

    pub fn read8(&mut self, addr: u32) -> u8 {
        let a = (addr & 0xFFFF) as u16;
        self.last_bus_addr = a;
        self.last_bus_write = false;
        let val = match a {
            MMIO_KEY_STATE => self.keys.to_bitmask(),
            MMIO_KEY_P1_UP => u8::from(self.keys.p1_up),
            MMIO_KEY_P1_DOWN => u8::from(self.keys.p1_down),
            MMIO_KEY_P2_UP => u8::from(self.keys.p2_up),
            MMIO_KEY_P2_DOWN => u8::from(self.keys.p2_down),
            MMIO_KEY_START => u8::from(self.keys.start),
            MMIO_KEY_LAST => self.keys.last_key,
            MMIO_RNG => {
                let r = self.next_rng_byte();
                self.data[a as usize] = r;
                r
            }
            0xF011 => {
                let r = self.next_rng_byte();
                self.data[a as usize] = r;
                r
            }
            MMIO_VSYNC => {
                self.vsync_poll_count = self.vsync_poll_count.wrapping_add(1);
                // If polled in a headless test loop without UI frames, auto-release after 64 polls
                if self.vsync_ready || self.vsync_poll_count >= 64 {
                    self.vsync_ready = false;
                    self.vsync_poll_count = 0;
                    1
                } else {
                    0
                }
            }
            _ => self.data[a as usize],
        };
        self.last_bus_data = val;
        val
    }

    /// Side-effect-free peek for UI hex dump and disassembler.
    pub fn peek8(&self, addr: u32) -> u8 {
        let a = (addr & 0xFFFF) as u16;
        match a {
            MMIO_KEY_STATE => self.keys.to_bitmask(),
            MMIO_KEY_P1_UP => u8::from(self.keys.p1_up),
            MMIO_KEY_P1_DOWN => u8::from(self.keys.p1_down),
            MMIO_KEY_P2_UP => u8::from(self.keys.p2_up),
            MMIO_KEY_P2_DOWN => u8::from(self.keys.p2_down),
            MMIO_KEY_START => u8::from(self.keys.start),
            MMIO_KEY_LAST => self.keys.last_key,
            _ => self.data[a as usize],
        }
    }

    pub fn peek16(&self, addr: u32) -> u16 {
        let b0 = self.peek8(addr) as u16;
        let b1 = self.peek8(addr.wrapping_add(1)) as u16;
        b0 | (b1 << 8)
    }

    pub fn peek32(&self, addr: u32) -> u32 {
        let b0 = self.peek8(addr) as u32;
        let b1 = self.peek8(addr.wrapping_add(1)) as u32;
        let b2 = self.peek8(addr.wrapping_add(2)) as u32;
        let b3 = self.peek8(addr.wrapping_add(3)) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    pub fn write8(&mut self, addr: u32, val: u8) {
        let a = (addr & 0xFFFF) as u16;
        self.last_bus_addr = a;
        self.last_bus_data = val;
        self.last_bus_write = true;

        match a {
            MMIO_KEY_STATE..=MMIO_KEY_START => {
                // Read-only hardware key state registers (though allow writing in tests if needed)
                self.data[a as usize] = val;
            }
            MMIO_VRAM_CLEAR => {
                self.data[a as usize] = val;
                let start = VRAM_BASE as usize;
                let end = (VRAM_BASE as usize + 0x2000).min(MEM_SIZE);
                self.data[start..end].fill(val);
            }
            _ => {
                self.data[a as usize] = val;
            }
        }
    }

    pub fn read16(&mut self, addr: u32) -> u16 {
        let b0 = self.read8(addr) as u16;
        let b1 = self.read8(addr.wrapping_add(1)) as u16;
        b0 | (b1 << 8)
    }

    pub fn write16(&mut self, addr: u32, val: u16) {
        self.write8(addr, (val & 0xFF) as u8);
        self.write8(addr.wrapping_add(1), (val >> 8) as u8);
    }

    pub fn read32(&mut self, addr: u32) -> u32 {
        let b0 = self.read8(addr) as u32;
        let b1 = self.read8(addr.wrapping_add(1)) as u32;
        let b2 = self.read8(addr.wrapping_add(2)) as u32;
        let b3 = self.read8(addr.wrapping_add(3)) as u32;
        b0 | (b1 << 8) | (b2 << 16) | (b3 << 24)
    }

    pub fn write32(&mut self, addr: u32, val: u32) {
        self.write8(addr, (val & 0xFF) as u8);
        self.write8(addr.wrapping_add(1), ((val >> 8) & 0xFF) as u8);
        self.write8(addr.wrapping_add(2), ((val >> 16) & 0xFF) as u8);
        self.write8(addr.wrapping_add(3), ((val >> 24) & 0xFF) as u8);
    }
}
