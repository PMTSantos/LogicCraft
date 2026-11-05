# CedarLogic Mechanics & Architecture Reference

This document records the constants, simulation rules, rendering parameters, and file format mechanics extracted from `CedarLogic.exe` via Ghidra decompilation (`craft-anything-cli` Deep mode).

---

## 1. Application Settings & Defaults (`function 0x44f9a0`, `function 0x458f00`)

In `FUN_0044f9a0` (`0x44f9a0`) and `FUN_00458f00` (`0x458f00`), CedarLogic loads and saves configuration keys in `res/settings.ini` and initializes the global settings singleton returned by `FUN_00450db0` (`0x450db0`, stored at `0x6d5194`):

| Name | Group | Value | Unit | Source | Description |
|---|---|---|---|---|---|
| `default_frame_width` | `window` | `600` | `px` | `function 0x44f9a0` | Fallback/default window width (`*(uint32*)(s + 0x1a8) = 600`) |
| `default_frame_height` | `window` | `600` | `px` | `function 0x44f9a0` | Fallback/default window height (`*(uint32*)(s + 0x1ac) = 600`) |
| `initial_frame_width` | `window` | `1800` | `px` | `function 0x4512b0` | Initial `wxFrame` constructor width (`0x708 = 1800`) |
| `initial_frame_height` | `window` | `900` | `px` | `function 0x4512b0` | Initial `wxFrame` constructor height (`0x384 = 900`) |
| `default_frame_left` | `window` | `20` | `px` | `function 0x44f9a0` | Default window X position (`0x14 = 20`) |
| `default_frame_top` | `window` | `20` | `px` | `function 0x44f9a0` | Default window Y position (`0x14 = 20`) |
| `default_time_step_ms` | `simulation` | `25` | `ms` | `function 0x44f9a0` | Default simulation time step (`0x19 = 25` ms) |
| `min_time_step_ms` | `simulation` | `1` | `ms` | `function 0x4512b0` | Minimum toolbar `wxSlider` time step (`1` ms) |
| `max_time_step_ms` | `simulation` | `500` | `ms` | `function 0x4512b0` | Maximum toolbar `wxSlider` time step (`500` ms) |
| `default_refresh_rate_ms` | `rendering` | `60` | `ms` | `function 0x44f9a0` | Default canvas refresh interval (`0x3c = 60` ms) |
| `wire_conn_radius` | `rendering` | `0.18` | `world` | `function 0x44f9a0` | Radius of wire solder/connection dots (`0x3e3851ec = 0.18f`) |
| `wire_conn_visible` | `rendering` | `true` | `bool` | `function 0x44f9a0` | Default visibility of wire connection points (`*(uint8*)(s + 0x1c4) = 1`) |
| `gridline_visible` | `rendering` | `true` | `bool` | `function 0x44f9a0` | Default visibility of canvas gridlines (`*(uint8*)(s + 0x1c5) = 1`) |

---

## 2. Canvas, Grid, Minimap & Oscilloscope (`function 0x420b50`, `function 0x4471f0`, `function 0x4512b0`, `function 0x45c180`, `function 0x45a150`)

- **Grid Snap & Spacing (`function 0x420b50`)**: `GUICanvas` initializes horizontal and vertical grid spacing to `0.5` world units (`0x3f000000 = 0.5f` passed to `FUN_00448100` and `FUN_004482b0`).
- **Default View Scale (`function 0x4471f0`)**: `klsGLCanvas` sets `viewZoom` to `0.08` world units per pixel (`0x3da3d70a = 0.08f`, equivalent to `12.5` pixels per world unit).
- **Minimap Dimensions (`function 0x4512b0`)**: `klsMiniMap` (`FUN_00449250`) is created with size `130 x 100` pixels (`0x82 = 130`, `100`).
- **Oscilloscope Frame (`function 0x45c180`, `function 0x45a150`)**: `OscopeFrame` (`FUN_0045c180`) opens at `800 x 250` pixels (`local_24 = 800`, `local_20 = 0xfa = 250`) and renders vertical time divisions in `OscopeCanvas::OnRender` (`FUN_0045a150`) at x-coordinates `0, 20, 40, 60, 80, 100, 120, 140, 160, 180`.

| Name | Group | Value | Unit | Source |
|---|---|---|---|---|
| `grid_spacing` | `canvas` | `0.5` | `world` | `function 0x420b50` |
| `default_view_zoom` | `canvas` | `0.08` | `world/px` | `function 0x4471f0` |
| `minimap_width` | `window` | `130` | `px` | `function 0x4512b0` |
| `minimap_height` | `window` | `100` | `px` | `function 0x4512b0` |
| `oscope_width` | `oscope` | `800` | `px` | `function 0x45c180` |
| `oscope_height` | `oscope` | `250` | `px` | `function 0x45c180` |
| `oscope_grid_step_x` | `oscope` | `20` | `steps` | `function 0x45a150` |
| `oscope_trace_length` | `oscope` | `180` | `steps` | `function 0x45a150` |
| `solder_dot_angle_step_deg` | `rendering` | `30` | `deg` | `function 0x43ad70` |
| `default_clock_half_cycle` | `simulation` | `5` | `steps` | `function 0x4813e0` |

---

## 3. Logic States & Wire Rendering (`function 0x43ad70`, `function 0x486eb0`)

In `FUN_0043ad70` (`guiWire::draw` at `0x43ad70`) and `FUN_00486eb0` (`Gate_REGISTER::gateProcess` at `0x486eb0`), CedarLogic simulates 5-valued digital logic on every wire and bus bit:

1. **`state_zero` (`0`)**: Logic `0` (Low) — rendered in black `RGB(0.0, 0.0, 0.0)` (`glColor4f(0, 0, 0, 1.0)`).
2. **`state_one` (`1`)**: Logic `1` (High) — rendered in red `RGB(1.0, 0.0, 0.0)` (`glColor4f(1.0, 0, 0, 1.0)`). For multi-bit buses, the red channel is scaled proportionally to the unsigned bus value `value / (2^bits - 1)`.
3. **`state_hi_z` (`2`)**: High-Impedance (`HI_Z` / floating) — rendered in green `RGB(0.0, 0.78, 0.0)` (`0x3f47ae14 = 0.78f`).
4. **`state_conflict` (`3`)**: Bus Conflict (multiple active drivers with opposing states) — rendered in cyan `RGB(0.0, 1.0, 1.0)` (`glColor4f(0, 1.0, 1.0, 1.0)`).
5. **`state_unknown` (`4`)**: Unknown / Uninitialized (`UNKNOWN`) — rendered in blue-violet `RGB(0.3, 0.3, 1.0)` (`0x3e99999a = 0.3f`).

- **Wire Line Widths (`function 0x43ad70`)**:
  - `single_wire_width = 1.0` px
  - `bus_wire_width = 4.0` px (with `glPointSize(4.0)` at segment endpoints)
  - Selected wires use dashed stipple `0x9999`.
  - Solder dot polygon uses `30` degree steps (`0x1e`, 12 segments around `360` degrees).

---

## 4. Gate Simulation & Library Architecture (`function 0x47e0b0`, `function 0x481040`, `function 0x485de0`, `function 0x486eb0`)

- **Default Gate Delay (`function 0x481040`)**: `Gate` base class constructor sets `defaultDelay = 1` simulation step (`param_1[2] = 1`).
- **21 Simulation Gate Classes (`function 0x47e0b0`)**:
  - `AND` (`Gate_AND`, `0x481310`): N-input AND/NAND/NANDX (`INPUT_BITS`, `OUTINV` / inverted inputs).
  - `OR` (`Gate_OR`, `0x4820c0`): N-input OR/NOR/NORX.
  - `XOR` (`Gate_XOR`, `0x482970`): N-input XOR/XNOR.
  - `BUFFER` (`Gate_PASS`, `0x482150`): Inverter, 4-bit Inverter, Tri-state Buffer (high/low active enable), 4/8/16-bit Tri-State Bus.
  - `MUX` (`Gate_MUX`, `0x481cf0`): 2x1, 4x1, 8x1, 16x1 Multiplexers.
  - `DECODER` (`Gate_DECODER`, `0x481670`): 2x4, 3x8, 4x16 Decoders.
  - `PRI_ENCODER` (`Gate_PRI_ENCODER`, `0x482200`): 4x2, 8x3, 16x4 Priority Encoders.
  - `BUS_END` (`Gate_BUS_END`, `0x4813a0`): 4x1, 8x1, 16x1 Bus Splitters/Combiners.
  - `CLOCK` (`Gate_CLOCK`, `0x4813e0`): Square-wave clock generator (`HALF_CYCLE` parameter).
  - `PULSE` (`Gate_PULSE`, `0x482350`): Single-pulse generator (`PULSE_WIDTH` parameter).
  - `DRIVER` (`Gate_DRIVER`, `0x481800`): Toggle Switch (`AA_TOGGLE`), 4-bit Hex Keypad (`DD_KEYPAD_HEX`), Vdd (`EE_VDD`), Ground (`FF_GND`).
  - `ADDER` (`Gate_ADDER`, `0x4811c0`): 1-Bit and 4-Bit Full Adders (`carry_in`, `carry_out`, `sum`).
  - `COMPARE` (`Gate_COMPARE`, `0x481480`): 4-Bit Magnitude Comparator (`A_equal_B`, `A_greater_B`, `A_less_B`).
  - `JKFF` (`Gate_JKFF`, `0x481950`, `0x485de0`): Positive and negative edge-triggered JK Flip-Flops with low-active Set/Clear (`SYNC_SET`, `SYNC_CLEAR`).
  - `RAM` (`Gate_RAM`, `0x4823f0`, `0x486960`): 4x4, 8x8, 12x16, 16x16 RAM and ROM (`ADDRESS_BITS`, `DATA_BITS`, `write_clock`, `.cdm` and Intel `.hex` memory files).
  - `REGISTER` (`Gate_REGISTER`, `0x4825a0`, `0x486eb0`): D Flip-Flops (positive/negative triggered, clock-enabled), 4/8-Bit 7-Segment/Hex LED Displays, 4/8/12/16-Bit Counting Registers, 4-Bit Shift Register.
  - `JUNCTION` (`Gate_JUNCTION`, `0x481bc0`): Named wireless links (`FROM` / `TO` tags sharing `JUNCTION_ID`).
  - `TGATE` (`Gate_T`, `0x482860`): CMOS Transmission Gate, nMOS and pMOS transistors.
  - `NODE` (`Gate_NODE`, `0x481de0`): 2-way / 4-way Junctions and single LED indicator (`GA_LED`).
  - `EQUIVALENCE` (`Gate_EQUIVALENCE`, `0x4818c0`): Bidirectional equivalence node.
  - `Pauseulator` (`Gate_pauseulator`, `0x482a00`): Breakpoint gate that pauses simulation when its input transitions high.
- **10 Component Libraries (`res/cl_gatedefs.xml`, loaded by `function 0x44f9a0`)**:
  1. `1 - Basic Gates`
  2. `2 - Invert & Connect`
  3. `3 - Input and Output`
  4. `4 - Text`
  5. `5 - MUX and Decoder`
  6. `6 - Add & Compare`
  7. `7 - Flip Flops`
  8. `8 - Registers`
  9. `9 - RAM and ROM`
  10. `10 - Chips`
