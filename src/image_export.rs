use std::path::Path;

use crate::library::{GateLibrary, GateShapeStyle};
use crate::mechanics::LogicState;
use crate::simulator::CircuitSimulator;

const SEVEN_SEG_BITS: [u8; 16] = [
    0b0111111, // 0
    0b0000110, // 1
    0b1011011, // 2
    0b1001111, // 3
    0b1100110, // 4
    0b1101101, // 5
    0b1111101, // 6
    0b0000111, // 7
    0b1111111, // 8
    0b1101111, // 9
    0b1110111, // A
    0b1111100, // b
    0b0111001, // C
    0b1011110, // d
    0b1111001, // E
    0b1110001, // F
];

/// Renders the current circuit canvas to an `RgbaImage` with all wires, solder dots, gate symbols,
/// 5V/GND symbols, 4x4 hex keypads, 7-segment displays, From/To connectors, and component names.
pub fn render_canvas_image(
    sim: &CircuitSimulator,
    lib: &GateLibrary,
    dark_mode: bool,
    with_grid: bool,
    with_color: bool,
) -> image::RgbaImage {
    let width = 1440u32;
    let height = 860u32;
    let bg = if !with_color || !dark_mode {
        [248, 250, 253, 255]
    } else {
        [15, 19, 27, 255]
    };
    let mut img = image::RgbaImage::from_pixel(width, height, image::Rgba(bg));

    let page = &sim.pages[sim.current_page];
    let canvas_cx = width as f32 * 0.5;
    let canvas_cy = height as f32 * 0.5;
    let px_per_w = (1.0 / page.zoom_world_per_px.max(0.01)).clamp(8.0, 28.0);

    let to_px = |w: [f32; 2]| -> (i32, i32) {
        (
            (canvas_cx + (w[0] - page.pan_world[0]) * px_per_w).round() as i32,
            (canvas_cy - (w[1] - page.pan_world[1]) * px_per_w).round() as i32,
        )
    };

    // 1. Optional Grid
    if with_grid {
        let minor_col = if !with_color || !dark_mode {
            [226, 232, 240, 255]
        } else {
            [24, 30, 42, 255]
        };
        let major_col = if !with_color || !dark_mode {
            [203, 213, 225, 255]
        } else {
            [36, 45, 62, 255]
        };
        for gx in -60..=60 {
            let (sx, _) = to_px([gx as f32, 0.0]);
            if sx >= 0 && (sx as u32) < width {
                let col = if gx % 5 == 0 { major_col } else { minor_col };
                draw_line_px(&mut img, sx, 0, sx, height as i32 - 1, col, 1);
            }
        }
        for gy in -40..=40 {
            let (_, sy) = to_px([0.0, gy as f32]);
            if sy >= 0 && (sy as u32) < height {
                let col = if gy % 5 == 0 { major_col } else { minor_col };
                draw_line_px(&mut img, 0, sy, width as i32 - 1, sy, col, 1);
            }
        }
    }

    // Header banner with app & page title
    let title_col = if !with_color || !dark_mode {
        [30, 41, 59, 255]
    } else {
        [185, 210, 245, 255]
    };
    draw_text_px(
        &mut img,
        16,
        14,
        &format!("LogicCraft - {}", page.name),
        title_col,
        2,
    );

    // 2. Draw Wires & Solder Dots
    for w in &page.wires {
        let rgba = if !with_color {
            [20, 20, 20, 255]
        } else {
            let c = w.state.wire_color(dark_mode);
            [c.r(), c.g(), c.b(), 255]
        };
        let thickness = if w.bus_bits > 1 { 3 } else { 2 };
        for seg in w.points.windows(2) {
            let (x0, y0) = to_px(seg[0]);
            let (x1, y1) = to_px(seg[1]);
            draw_line_px(&mut img, x0, y0, x1, y1, rgba, thickness);
        }
        for &dot in &w.solder_dots {
            let (dx, dy) = to_px(dot);
            draw_circle_filled_px(&mut img, dx, dy, 3, rgba);
        }
    }

    // 3. Draw Gates, Symbols & Component Names (matching the live canvas 1:1)
    let stroke_col = if !with_color || !dark_mode {
        [38, 48, 68, 255]
    } else {
        [190, 208, 232, 255]
    };
    let body_fill = if !with_color || !dark_mode {
        [255, 255, 255, 255]
    } else {
        [28, 37, 54, 255]
    };
    let label_col = if !with_color || !dark_mode {
        [20, 65, 145, 255]
    } else {
        [140, 205, 255, 255]
    };
    let pin_lbl_col = if !with_color || !dark_mode {
        [65, 80, 105, 255]
    } else {
        [150, 172, 202, 255]
    };

    for g in &page.gates {
        let Some(def) = lib.get(&g.type_name) else {
            continue;
        };
        let to_gate_px = |local: [f32; 2]| -> (i32, i32) { to_px(g.pin_world_pos(local)) };
        let (cx, cy) = to_px(g.pos);
        let hw = def.half_size[0];
        let hh = def.half_size[1];

        match def.shape_style {
            GateShapeStyle::Label => {
                let txt = g
                    .gparams
                    .get("LABEL_TEXT")
                    .map(|s| s.as_str())
                    .unwrap_or("Label");
                draw_text_centered_px(&mut img, cx, cy, txt, label_col, 2);
            }
            GateShapeStyle::And => {
                let h_body = hh * 0.84;
                let x_left = if def.in_inverted { -hw + 0.50 } else { -hw };
                let x_right = if def.out_inverted { hw - 0.54 } else { hw };
                let x_arc = (x_left + x_right) * 0.45;
                let rx = (x_right - x_arc).max(0.4);
                let mut poly = Vec::with_capacity(38);
                poly.push(to_gate_px([x_left, h_body]));
                poly.push(to_gate_px([x_arc, h_body]));
                for step in 1..32 {
                    let ang = std::f32::consts::FRAC_PI_2
                        - (step as f32 / 32.0) * std::f32::consts::PI;
                    poly.push(to_gate_px([x_arc + ang.cos() * rx, ang.sin() * h_body]));
                }
                poly.push(to_gate_px([x_arc, -h_body]));
                poly.push(to_gate_px([x_left, -h_body]));
                fill_polygon_px(&mut img, &poly, body_fill);
                stroke_polygon_px(&mut img, &poly, stroke_col, 2);

                if def.in_inverted {
                    let r = (0.25 * px_per_w).round().max(3.0) as i32;
                    for pin in &def.inputs {
                        let (bx, by) = to_gate_px([-hw + 0.25, pin.pos[1]]);
                        draw_circle_filled_px(&mut img, bx, by, r, body_fill);
                        draw_circle_outline_px(&mut img, bx, by, r, stroke_col);
                    }
                }
                if def.out_inverted {
                    let r = (0.27 * px_per_w).round().max(3.0) as i32;
                    let (bx, by) = to_gate_px([hw - 0.27, 0.0]);
                    draw_circle_filled_px(&mut img, bx, by, r, body_fill);
                    draw_circle_outline_px(&mut img, bx, by, r, stroke_col);
                }
            }
            GateShapeStyle::Or | GateShapeStyle::Xor => {
                let is_xor = def.shape_style == GateShapeStyle::Xor;
                let h_body = hh * 0.84;
                let x_tip = if def.out_inverted { hw - 0.54 } else { hw };
                let back_depth = 0.40_f32;
                let base_corner_x = if def.in_inverted {
                    -hw + 0.50 - 0.14
                } else {
                    -hw - 0.25
                };
                let body_corner_x = if is_xor {
                    base_corner_x + 0.28
                } else {
                    base_corner_x
                };
                let x_back_at = |u: f32| -> f32 { body_corner_x + back_depth * (1.0 - u * u) };
                let x_shield_at = |u: f32| -> f32 { base_corner_x + back_depth * (1.0 - u * u) };
                let front_pt = |u: f32| -> [f32; 2] {
                    let sign = if u >= 0.0 { 1.0 } else { -1.0 };
                    let t = 1.0 - u.abs();
                    cubic_bezier_pt(
                        [body_corner_x, sign * h_body],
                        [body_corner_x + (x_tip - body_corner_x) * 0.46, sign * h_body],
                        [body_corner_x + (x_tip - body_corner_x) * 0.82, sign * h_body * 0.54],
                        [x_tip, 0.0],
                        t,
                    )
                };

                let strips = 28usize;
                for i in 0..strips {
                    let u0 = -1.0 + (i as f32 / strips as f32) * 2.0;
                    let u1 = -1.0 + ((i + 1) as f32 / strips as f32) * 2.0;
                    let quad = [
                        to_gate_px([x_back_at(u0), u0 * h_body]),
                        to_gate_px(front_pt(u0)),
                        to_gate_px(front_pt(u1)),
                        to_gate_px([x_back_at(u1), u1 * h_body]),
                    ];
                    fill_polygon_px(&mut img, &quad, body_fill);
                }

                let mut outline = Vec::with_capacity(strips * 3 + 4);
                for i in 0..=strips {
                    let t = i as f32 / strips as f32;
                    let p = cubic_bezier_pt(
                        [body_corner_x, h_body],
                        [body_corner_x + (x_tip - body_corner_x) * 0.46, h_body],
                        [body_corner_x + (x_tip - body_corner_x) * 0.82, h_body * 0.54],
                        [x_tip, 0.0],
                        t,
                    );
                    outline.push(to_gate_px(p));
                }
                for i in 1..=strips {
                    let t = i as f32 / strips as f32;
                    let p = cubic_bezier_pt(
                        [x_tip, 0.0],
                        [body_corner_x + (x_tip - body_corner_x) * 0.82, -h_body * 0.54],
                        [body_corner_x + (x_tip - body_corner_x) * 0.46, -h_body],
                        [body_corner_x, -h_body],
                        t,
                    );
                    outline.push(to_gate_px(p));
                }
                for i in 1..strips {
                    let u = -1.0 + (i as f32 / strips as f32) * 2.0;
                    outline.push(to_gate_px([x_back_at(u), u * h_body]));
                }
                stroke_polygon_px(&mut img, &outline, stroke_col, 2);

                if is_xor {
                    let mut prev: Option<(i32, i32)> = None;
                    for i in 0..=strips {
                        let u = -1.0 + (i as f32 / strips as f32) * 2.0;
                        let cur = to_gate_px([x_shield_at(u), u * h_body]);
                        if let Some((px0, py0)) = prev {
                            draw_line_px(&mut img, px0, py0, cur.0, cur.1, stroke_col, 2);
                        }
                        prev = Some(cur);
                    }
                }

                for pin in &def.inputs {
                    let u = (pin.pos[1] / h_body).clamp(-1.0, 1.0);
                    let curve_x = x_back_at(u);
                    let start_x = if def.in_inverted { -hw + 0.50 } else { pin.pos[0] };
                    if (curve_x - start_x).abs() > 0.04 {
                        let (x0, y0) = to_gate_px([start_x, pin.pos[1]]);
                        let (x1, y1) = to_gate_px([curve_x, pin.pos[1]]);
                        draw_line_px(&mut img, x0, y0, x1, y1, stroke_col, 2);
                    }
                    if def.in_inverted {
                        let r = (0.25 * px_per_w).round().max(3.0) as i32;
                        let (bx, by) = to_gate_px([-hw + 0.25, pin.pos[1]]);
                        draw_circle_filled_px(&mut img, bx, by, r, body_fill);
                        draw_circle_outline_px(&mut img, bx, by, r, stroke_col);
                    }
                }
                if def.out_inverted {
                    let r = (0.27 * px_per_w).round().max(3.0) as i32;
                    let (bx, by) = to_gate_px([hw - 0.27, 0.0]);
                    draw_circle_filled_px(&mut img, bx, by, r, body_fill);
                    draw_circle_outline_px(&mut img, bx, by, r, stroke_col);
                }
            }
            GateShapeStyle::Buffer | GateShapeStyle::TriState => {
                if def.inputs.len() <= 2 {
                    let x_tip = if def.out_inverted { hw - 0.54 } else { hw };
                    let tri = [
                        to_gate_px([-hw, hh]),
                        to_gate_px([x_tip, 0.0]),
                        to_gate_px([-hw, -hh]),
                    ];
                    fill_polygon_px(&mut img, &tri, body_fill);
                    stroke_polygon_px(&mut img, &tri, stroke_col, 2);
                    if def.out_inverted {
                        let r = (0.27 * px_per_w).round().max(3.0) as i32;
                        let (bx, by) = to_gate_px([hw - 0.27, 0.0]);
                        draw_circle_filled_px(&mut img, bx, by, r, body_fill);
                        draw_circle_outline_px(&mut img, bx, by, r, stroke_col);
                    }
                    if let Some(en_pin) = def.inputs.iter().find(|p| p.name.starts_with("ENABLE")) {
                        if en_pin.inverted {
                            let r = (0.25 * px_per_w).round().max(3.0) as i32;
                            let (bx, by) = to_gate_px([0.0, -1.10]);
                            draw_circle_filled_px(&mut img, bx, by, r, body_fill);
                            draw_circle_outline_px(&mut img, bx, by, r, stroke_col);
                        }
                    }
                } else {
                    let (c0x, c0y) = to_gate_px([-hw, hh]);
                    let (c1x, c1y) = to_gate_px([hw, -hh]);
                    let x0 = c0x.min(c1x);
                    let x1 = c0x.max(c1x);
                    let y0 = c0y.min(c1y);
                    let y1 = c0y.max(c1y);
                    fill_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, body_fill);
                    stroke_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, stroke_col);
                    draw_text_centered_px(
                        &mut img,
                        cx,
                        cy,
                        if def.out_inverted { "INV" } else { "TRI" },
                        label_col,
                        1,
                    );
                }
            }
            GateShapeStyle::Vdd => {
                // 5V Arrow Up design attached directly to [0.0, -hh]
                let (p_bot_x, p_bot_y) = to_gate_px([0.0, -hh]);
                let (p_tip_x, p_tip_y) = to_gate_px([0.0, 0.85]);
                let (p_left_x, p_left_y) = to_gate_px([-0.85, -0.05]);
                let (p_right_x, p_right_y) = to_gate_px([0.85, -0.05]);
                let vdd_col = if with_color {
                    [255, 85, 85, 255]
                } else {
                    stroke_col
                };
                draw_line_px(&mut img, p_bot_x, p_bot_y, p_tip_x, p_tip_y, vdd_col, 2);
                draw_line_px(&mut img, p_left_x, p_left_y, p_tip_x, p_tip_y, vdd_col, 2);
                draw_line_px(&mut img, p_right_x, p_right_y, p_tip_x, p_tip_y, vdd_col, 2);
                let (lx, ly) = to_gate_px([0.0, 1.45]);
                draw_text_centered_px(&mut img, lx, ly, "5V", vdd_col, 1);
            }
            GateShapeStyle::Ground => {
                // GND 3-horizontal-bar symbol attached directly to [0.0, hh]
                let (p_top_x, p_top_y) = to_gate_px([0.0, hh]);
                let (p_mid_x, p_mid_y) = to_gate_px([0.0, 0.0]);
                draw_line_px(&mut img, p_top_x, p_top_y, p_mid_x, p_mid_y, stroke_col, 2);
                for &(w_bar, y_off) in &[(1.15f32, 0.0f32), (0.75f32, -0.45f32), (0.35f32, -0.9f32)] {
                    let (x0, y0) = to_gate_px([-w_bar, y_off]);
                    let (x1, y1) = to_gate_px([w_bar, y_off]);
                    draw_line_px(&mut img, x0, y0, x1, y1, stroke_col, 2);
                }
                let (lx, ly) = to_gate_px([0.0, -1.5]);
                draw_text_centered_px(&mut img, lx, ly, "GND", label_col, 1);
            }
            GateShapeStyle::FromLink | GateShapeStyle::ToLink => {
                let is_from = def.shape_style == GateShapeStyle::FromLink;
                let jid = g
                    .lparams
                    .get("JUNCTION_ID")
                    .map(|s| s.as_str())
                    .unwrap_or("NET_A");
                let tag_poly = if is_from {
                    vec![
                        to_gate_px([-hw, 0.0]),
                        to_gate_px([-1.5, 0.92]),
                        to_gate_px([hw, 0.92]),
                        to_gate_px([hw, -0.92]),
                        to_gate_px([-1.5, -0.92]),
                    ]
                } else {
                    vec![
                        to_gate_px([-hw, 0.92]),
                        to_gate_px([1.5, 0.92]),
                        to_gate_px([hw, 0.0]),
                        to_gate_px([1.5, -0.92]),
                        to_gate_px([-hw, -0.92]),
                    ]
                };
                let tag_fill = if !with_color {
                    body_fill
                } else if is_from {
                    if dark_mode {
                        [32, 58, 78, 255]
                    } else {
                        [225, 242, 255, 255]
                    }
                } else if dark_mode {
                    [48, 38, 76, 255]
                } else {
                    [238, 228, 255, 255]
                };
                fill_polygon_px(&mut img, &tag_poly, tag_fill);
                stroke_polygon_px(&mut img, &tag_poly, stroke_col, 2);
                let scope_tag = if g.is_tag_local() { "L" } else { "S" };
                let label = if is_from {
                    format!("FM[{scope_tag}]:{jid}")
                } else {
                    format!("TO[{scope_tag}]:{jid}")
                };
                let (tx, ty) = if is_from {
                    to_gate_px([0.4, 0.0])
                } else {
                    to_gate_px([-0.4, 0.0])
                };
                draw_text_centered_px(&mut img, tx, ty, &label, label_col, 1);
            }
            GateShapeStyle::ToggleSwitch => {
                let (c0x, c0y) = to_gate_px([-hw, hh - 0.1]);
                let (c1x, c1y) = to_gate_px([hw, -hh + 0.1]);
                let x0 = c0x.min(c1x);
                let x1 = c0x.max(c1x);
                let y0 = c0y.min(c1y);
                let y1 = c0y.max(c1y);
                let is_on = g
                    .lparams
                    .get("OUTPUT_NUM")
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(0)
                    != 0;
                let sw_fill = if !with_color {
                    body_fill
                } else if is_on {
                    [195, 42, 42, 255]
                } else if dark_mode {
                    [38, 46, 62, 255]
                } else {
                    [218, 225, 236, 255]
                };
                let sw_fg = if (is_on && with_color) || (dark_mode && with_color) {
                    [255, 255, 255, 255]
                } else {
                    [25, 32, 48, 255]
                };
                fill_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, sw_fill);
                stroke_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, stroke_col);
                draw_text_centered_px(
                    &mut img,
                    (x0 + x1) / 2,
                    (y0 + y1) / 2,
                    if is_on { "ON (1)" } else { "OFF (0)" },
                    sw_fg,
                    1,
                );
            }
            GateShapeStyle::Clock => {
                let (c0x, c0y) = to_gate_px([-hw, hh - 0.1]);
                let (c1x, c1y) = to_gate_px([hw, -hh + 0.1]);
                let x0 = c0x.min(c1x);
                let x1 = c0x.max(c1x);
                let y0 = c0y.min(c1y);
                let y1 = c0y.max(c1y);
                fill_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, body_fill);
                stroke_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, stroke_col);
                let is_high = g.pin_states.get("CLK").copied() == Some(LogicState::One);
                let sq_col = if !with_color {
                    stroke_col
                } else if is_high {
                    [235, 65, 65, 255]
                } else if dark_mode {
                    [100, 190, 255, 255]
                } else {
                    [30, 105, 200, 255]
                };
                let pts = [
                    to_gate_px([-1.6, -0.55]),
                    to_gate_px([-1.6, 0.55]),
                    to_gate_px([-0.55, 0.55]),
                    to_gate_px([-0.55, -0.55]),
                    to_gate_px([0.55, -0.55]),
                    to_gate_px([0.55, 0.55]),
                    to_gate_px([1.6, 0.55]),
                ];
                for seg in pts.windows(2) {
                    draw_line_px(&mut img, seg[0].0, seg[0].1, seg[1].0, seg[1].1, sq_col, 2);
                }
            }
            GateShapeStyle::Pulse => {
                let (c0x, c0y) = to_gate_px([-hw, hh - 0.1]);
                let (c1x, c1y) = to_gate_px([hw, -hh + 0.1]);
                let x0 = c0x.min(c1x);
                let x1 = c0x.max(c1x);
                let y0 = c0y.min(c1y);
                let y1 = c0y.max(c1y);
                fill_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, body_fill);
                stroke_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, stroke_col);
                let pts = [
                    to_gate_px([-1.6, -0.5]),
                    to_gate_px([-0.6, -0.5]),
                    to_gate_px([-0.6, 0.6]),
                    to_gate_px([0.6, 0.6]),
                    to_gate_px([0.6, -0.5]),
                    to_gate_px([1.6, -0.5]),
                ];
                for seg in pts.windows(2) {
                    draw_line_px(&mut img, seg[0].0, seg[0].1, seg[1].0, seg[1].1, stroke_col, 2);
                }
            }
            GateShapeStyle::Led => {
                let st = g.pin_states.get("IN_0").copied().unwrap_or(LogicState::HiZ);
                let fill = match st {
                    LogicState::One if with_color => [255, 55, 55, 255],
                    LogicState::Zero => {
                        if dark_mode && with_color {
                            [45, 52, 65, 255]
                        } else {
                            [215, 222, 232, 255]
                        }
                    }
                    _ => [130, 150, 180, 255],
                };
                let (lcx, lcy) = to_gate_px([-0.5, 0.0]);
                let r = (1.5 * px_per_w).round() as i32;
                draw_circle_filled_px(&mut img, lcx, lcy, r, fill);
                draw_circle_outline_px(&mut img, lcx, lcy, r, stroke_col);
            }
            GateShapeStyle::HexKeypad => {
                let (c0x, c0y) = to_gate_px([-hw, hh]);
                let (c1x, c1y) = to_gate_px([hw, -hh]);
                let x0 = c0x.min(c1x);
                let x1 = c0x.max(c1x);
                let y0 = c0y.min(c1y);
                let y1 = c0y.max(c1y);
                fill_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, body_fill);
                stroke_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, stroke_col);
                let active_val = g
                    .lparams
                    .get("OUTPUT_NUM")
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(0)
                    & 0xF;
                let cell_w = (x1 - x0 - 8).max(16) / 4;
                let cell_h = (y1 - y0 - 8).max(16) / 4;
                for r in 0..4i32 {
                    for c in 0..4i32 {
                        let k = (r * 4 + c) as u16;
                        let kx = x0 + 4 + c * cell_w;
                        let ky = y0 + 4 + r * cell_h;
                        if k == active_val {
                            fill_rect_i32(
                                &mut img,
                                kx + 1,
                                ky + 1,
                                cell_w - 2,
                                cell_h - 2,
                                if with_color {
                                    [225, 70, 55, 255]
                                } else {
                                    [180, 180, 180, 255]
                                },
                            );
                        }
                        let ch = format!("{k:X}");
                        draw_text_centered_px(
                            &mut img,
                            kx + cell_w / 2,
                            ky + cell_h / 2,
                            &ch,
                            stroke_col,
                            1,
                        );
                    }
                }
            }
            GateShapeStyle::LedDisplay { digits } => {
                let (c0x, c0y) = to_gate_px([-hw, hh]);
                let (c1x, c1y) = to_gate_px([hw, -hh]);
                let x0 = c0x.min(c1x);
                let x1 = c0x.max(c1x);
                let y0 = c0y.min(c1y);
                let y1 = c0y.max(c1y);
                fill_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, [18, 22, 30, 255]);
                stroke_rect_i32(&mut img, x0, y0, x1 - x0, y1 - y0, stroke_col);
                let val = g
                    .lparams
                    .get("CURRENT_VALUE")
                    .and_then(|s| s.parse::<u16>().ok())
                    .unwrap_or(0);
                if digits == 1 {
                    draw_seven_seg_px(&mut img, x0 + 7, y0 + 7, x1 - 7, y1 - 7, (val & 0xF) as usize, with_color);
                } else {
                    let mid = (x0 + x1) / 2;
                    draw_seven_seg_px(&mut img, x0 + 6, y0 + 6, mid - 2, y1 - 6, ((val >> 4) & 0xF) as usize, with_color);
                    draw_seven_seg_px(&mut img, mid + 2, y0 + 6, x1 - 6, y1 - 6, (val & 0xF) as usize, with_color);
                }
            }
            _ => {
                let (c0x, c0y) = to_gate_px([-hw, hh]);
                let (c1x, c1y) = to_gate_px([hw, -hh]);
                let x0 = c0x.min(c1x);
                let x1 = c0x.max(c1x);
                let y0 = c0y.min(c1y);
                let y1 = c0y.max(c1y);
                fill_rect_i32(&mut img, x0, y0, (x1 - x0).max(16), (y1 - y0).max(14), body_fill);
                stroke_rect_i32(&mut img, x0, y0, (x1 - x0).max(16), (y1 - y0).max(14), stroke_col);

                let comp_name: &str = match def.shape_style {
                    GateShapeStyle::Adder => "ADDER",
                    GateShapeStyle::Comparator => "COMP",
                    GateShapeStyle::FlipFlopD => "D-FF",
                    GateShapeStyle::FlipFlopJK => "JK-FF",
                    GateShapeStyle::Register => "REG/CNT",
                    GateShapeStyle::Mux => "MUX",
                    GateShapeStyle::Decoder => "DEC",
                    GateShapeStyle::PriEncoder => "PRI-ENC",
                    GateShapeStyle::RamRom { is_rom: false } => "RAM",
                    GateShapeStyle::RamRom { is_rom: true } => "ROM",
                    GateShapeStyle::Pauseulator => "PAUSE",
                    GateShapeStyle::ChipBox => "74LS138",
                    GateShapeStyle::CpuProcessor => "CPU",
                    GateShapeStyle::VramDisplay => "VRAM DISPLAY",
                    GateShapeStyle::KeyboardInput => "KEYBOARD",
                    GateShapeStyle::FullKeyboard => "ASCII KEYBOARD",
                    GateShapeStyle::CustomIc => g
                        .gparams
                        .get("CHIP_LABEL")
                        .or_else(|| def.default_gparams.get("CHIP_LABEL"))
                        .map(|s| s.as_str())
                        .unwrap_or("CUSTOM"),
                    _ => &def.logic_type,
                };
                let title_y = if (y1 - y0) > 36 { y0 + 13 } else { cy };
                draw_text_centered_px(&mut img, cx, title_y, comp_name, label_col, 1);

                if let Some(cv) = g.lparams.get("CURRENT_VALUE") {
                    if let Ok(v) = cv.parse::<u16>() {
                        if (y1 - y0) > 30 {
                            let val_col = if !with_color {
                                stroke_col
                            } else if dark_mode {
                                [255, 200, 85, 255]
                            } else {
                                [195, 95, 10, 255]
                            };
                            draw_text_centered_px(
                                &mut img,
                                cx,
                                cy + 4,
                                &format!("0X{v:X}"),
                                val_col,
                                1,
                            );
                        }
                    }
                }

                for pin in def.inputs.iter().chain(def.outputs.iter()) {
                    let on_left = (pin.pos[0] + hw).abs() < 0.15;
                    if pin.name.contains("clock") && on_left {
                        let (tx0, ty0) = to_gate_px([-hw, pin.pos[1] + 0.35]);
                        let (tx1, ty1) = to_gate_px([-hw + 0.45, pin.pos[1]]);
                        let (tx2, ty2) = to_gate_px([-hw, pin.pos[1] - 0.35]);
                        draw_line_px(&mut img, tx0, ty0, tx1, ty1, pin_lbl_col, 1);
                        draw_line_px(&mut img, tx1, ty1, tx2, ty2, pin_lbl_col, 1);
                    }
                    if pin.inverted {
                        let (bx, by) = to_gate_px(pin.pos);
                        draw_circle_filled_px(&mut img, bx, by, 3, body_fill);
                        draw_circle_outline_px(&mut img, bx, by, 3, stroke_col);
                    }
                }
            }
        }

        // Draw pin connection dots directly on the edge
        for pin in def.inputs.iter().chain(def.outputs.iter()) {
            let (px, py) = to_gate_px(pin.pos);
            let st = g
                .pin_states
                .get(&pin.name)
                .copied()
                .unwrap_or(LogicState::HiZ);
            let c = st.wire_color(dark_mode);
            let p_rgba = if with_color {
                [c.r(), c.g(), c.b(), 255]
            } else {
                stroke_col
            };
            draw_circle_filled_px(&mut img, px, py, 2, p_rgba);
        }
    }

    // 4. Draw any pinned K-Map and Function cards on the canvas
    for kmap in &sim.kmaps {
        if kmap.pinned_on_canvas {
            let (kx, ky) = to_px(kmap.canvas_pos);
            let n = kmap.num_vars.clamp(2, 4);
            let row_bits = n / 2;
            let col_bits = n - row_bits;
            let rows = 1i32 << row_bits;
            let cols = 1i32 << col_bits;
            let cw = 26i32;
            let ch = 20i32;
            let w = (cols * cw + 60).max(175);
            let h = 36 + rows * ch + 18;
            fill_rect_i32(&mut img, kx, ky, w, h, body_fill);
            stroke_rect_i32(&mut img, kx, ky, w, h, label_col);
            draw_text_px(&mut img, kx + 8, ky + 8, &kmap.name, label_col, 1);
            let r_gray = crate::kmap::gray_code(row_bits);
            let c_gray = crate::kmap::gray_code(col_bits);
            let gx0 = kx + 38;
            let gy0 = ky + 34;
            for (ri, &(r_val, rlbl)) in r_gray.iter().enumerate() {
                let cy = gy0 + (ri as i32) * ch;
                draw_text_px(&mut img, gx0 - 18, cy + 6, rlbl, stroke_col, 1);
                for (ci, &(c_val, clbl)) in c_gray.iter().enumerate() {
                    let cx = gx0 + (ci as i32) * cw;
                    if ri == 0 {
                        draw_text_px(&mut img, cx + 6, gy0 - 10, clbl, stroke_col, 1);
                    }
                    stroke_rect_i32(&mut img, cx, cy, cw, ch, label_col);
                    let m = (r_val << col_bits) | c_val;
                    let v = kmap.values.get(m).copied().unwrap_or(0);
                    let vstr = match v {
                        1 => "1",
                        2 => "X",
                        _ => "0",
                    };
                    draw_text_centered_px(&mut img, cx + cw / 2, cy + ch / 2, vstr, stroke_col, 1);
                }
            }
        }
        if kmap.pinned_on_canvas || kmap.func_pinned_on_canvas {
            let (fx, fy) = to_px(kmap.func_canvas_pos);
            let (_, sop) = kmap.solve_minimal_sop();
            let pos = kmap.solve_minimal_pos();
            let w = ((sop.len().max(pos.len()) as i32) * 6 + 40).max(175);
            let h = 42;
            fill_rect_i32(&mut img, fx, fy, w, h, body_fill);
            stroke_rect_i32(&mut img, fx, fy, w, h, label_col);
            draw_text_px(&mut img, fx + 8, fy + 8, &format!("SOP: {sop}"), label_col, 1);
            draw_text_px(&mut img, fx + 8, fy + 24, &format!("POS: {pos}"), stroke_col, 1);
        }
    }

    img
}

/// Copies the given `RgbaImage` directly to the OS system clipboard as an image.
pub fn copy_image_to_system_clipboard(img: &image::RgbaImage) -> Result<(), String> {
    let temp_path = std::env::temp_dir().join("logiccraft_clipboard_canvas.png");
    img.save(&temp_path)
        .map_err(|e| format!("Failed to save temporary clipboard image: {e}"))?;

    #[cfg(target_os = "windows")]
    {
        let path_str = temp_path.to_string_lossy().replace('\'', "''");
        let ps_script = format!(
            "Add-Type -AssemblyName System.Windows.Forms; Add-Type -AssemblyName System.Drawing; $img = [System.Drawing.Image]::FromFile('{path_str}'); [System.Windows.Forms.Clipboard]::SetImage($img); $img.Dispose();"
        );
        let status = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command", &ps_script])
            .status()
            .map_err(|e| format!("Clipboard command error: {e}"))?;
        if !status.success() {
            return Err("PowerShell SetImage returned non-zero status".to_string());
        }
    }

    Ok(())
}

pub fn save_canvas_image_to_path(img: &image::RgbaImage, path: &Path) -> Result<(), String> {
    img.save(path).map_err(|e| format!("Image save error: {e}"))
}

fn fill_rect_i32(img: &mut image::RgbaImage, x0: i32, y0: i32, w: i32, h: i32, rgba: [u8; 4]) {
    let start_x = x0.max(0) as u32;
    let start_y = y0.max(0) as u32;
    let end_x = (x0 + w.max(0)).max(0).min(img.width() as i32) as u32;
    let end_y = (y0 + h.max(0)).max(0).min(img.height() as i32) as u32;
    for y in start_y..end_y {
        for x in start_x..end_x {
            img.put_pixel(x, y, image::Rgba(rgba));
        }
    }
}

fn stroke_rect_i32(img: &mut image::RgbaImage, x0: i32, y0: i32, w: i32, h: i32, rgba: [u8; 4]) {
    let x1 = x0 + w.max(1);
    let y1 = y0 + h.max(1);
    draw_line_px(img, x0, y0, x1, y0, rgba, 2);
    draw_line_px(img, x1, y0, x1, y1, rgba, 2);
    draw_line_px(img, x1, y1, x0, y1, rgba, 2);
    draw_line_px(img, x0, y1, x0, y0, rgba, 2);
}

fn draw_line_px(
    img: &mut image::RgbaImage,
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
    rgba: [u8; 4],
    thickness: i32,
) {
    let steps = (x1 - x0).abs().max((y1 - y0).abs()).max(1);
    let half = (thickness - 1).max(0) / 2;
    for s in 0..=steps {
        let t = s as f32 / steps as f32;
        let cx = (x0 as f32 + (x1 - x0) as f32 * t).round() as i32;
        let cy = (y0 as f32 + (y1 - y0) as f32 * t).round() as i32;
        for dy in -half..=half {
            for dx in -half..=half {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, image::Rgba(rgba));
                }
            }
        }
    }
}

fn draw_circle_filled_px(img: &mut image::RgbaImage, cx: i32, cy: i32, r: i32, rgba: [u8; 4]) {
    let r2 = r * r;
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r2 {
                let px = cx + dx;
                let py = cy + dy;
                if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
                    img.put_pixel(px as u32, py as u32, image::Rgba(rgba));
                }
            }
        }
    }
}

fn draw_circle_outline_px(img: &mut image::RgbaImage, cx: i32, cy: i32, r: i32, rgba: [u8; 4]) {
    for deg in (0..360).step_by(6) {
        let rad = (deg as f32).to_radians();
        let px = cx + (r as f32 * rad.cos()).round() as i32;
        let py = cy + (r as f32 * rad.sin()).round() as i32;
        if px >= 0 && py >= 0 && (px as u32) < img.width() && (py as u32) < img.height() {
            img.put_pixel(px as u32, py as u32, image::Rgba(rgba));
        }
    }
}

fn draw_seven_seg_px(
    img: &mut image::RgbaImage,
    l: i32,
    t: i32,
    r: i32,
    b: i32,
    digit: usize,
    with_color: bool,
) {
    let mask = SEVEN_SEG_BITS[digit & 0xF];
    let m = (t + b) / 2;
    let segs: [((i32, i32), (i32, i32), u8); 7] = [
        ((l, t), (r, t), 0),
        ((r, t), (r, m), 1),
        ((r, m), (r, b), 2),
        ((l, b), (r, b), 3),
        ((l, m), (l, b), 4),
        ((l, t), (l, m), 5),
        ((l, m), (r, m), 6),
    ];
    for ((x0, y0), (x1, y1), bit) in segs {
        let on = ((mask >> bit) & 1) != 0;
        let col = if on {
            if with_color {
                [255, 55, 55, 255]
            } else {
                [240, 240, 240, 255]
            }
        } else {
            [45, 28, 32, 255]
        };
        draw_line_px(img, x0, y0, x1, y1, col, if on { 3 } else { 1 });
    }
}

fn draw_text_centered_px(
    img: &mut image::RgbaImage,
    cx: i32,
    cy: i32,
    text: &str,
    rgba: [u8; 4],
    scale: i32,
) {
    let s = scale.max(1);
    let total_w = (text.chars().count() as i32) * 6 * s - s;
    let total_h = 7 * s;
    draw_text_px(img, cx - total_w / 2, cy - total_h / 2, text, rgba, s);
}

fn draw_text_px(
    img: &mut image::RgbaImage,
    x: i32,
    y: i32,
    text: &str,
    rgba: [u8; 4],
    scale: i32,
) {
    let s = scale.max(1);
    for (idx, ch) in text.chars().enumerate() {
        let glyph = glyph_5x7(ch);
        let gx = x + (idx as i32) * 6 * s;
        for (row, &bits) in glyph.iter().enumerate() {
            for col in 0..5 {
                if ((bits >> (4 - col)) & 1) != 0 {
                    for sy in 0..s {
                        for sx in 0..s {
                            let px = gx + col * s + sx;
                            let py = y + (row as i32) * s + sy;
                            if px >= 0
                                && py >= 0
                                && (px as u32) < img.width()
                                && (py as u32) < img.height()
                            {
                                img.put_pixel(px as u32, py as u32, image::Rgba(rgba));
                            }
                        }
                    }
                }
            }
        }
    }
}

fn glyph_5x7(c: char) -> [u8; 7] {
    match c.to_ascii_uppercase() {
        'A' => [0b01110, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'B' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10001, 0b10001, 0b11110],
        'C' => [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
        'D' => [0b11100, 0b10010, 0b10001, 0b10001, 0b10001, 0b10010, 0b11100],
        'E' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
        'F' => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b10000],
        'G' => [0b01110, 0b10001, 0b10000, 0b10111, 0b10001, 0b10001, 0b01110],
        'H' => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        'I' => [0b01110, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        'J' => [0b00111, 0b00010, 0b00010, 0b00010, 0b10010, 0b10010, 0b01100],
        'K' => [0b10001, 0b10010, 0b10100, 0b11000, 0b10100, 0b10010, 0b10001],
        'L' => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        'M' => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        'N' => [0b10001, 0b11001, 0b10101, 0b10011, 0b10001, 0b10001, 0b10001],
        'O' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'P' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
        'Q' => [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
        'R' => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        'S' => [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
        'T' => [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
        'U' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        'V' => [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01010, 0b00100],
        'W' => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        'X' => [0b10001, 0b10001, 0b01010, 0b00100, 0b01010, 0b10001, 0b10001],
        'Y' => [0b10001, 0b10001, 0b01010, 0b00100, 0b00100, 0b00100, 0b00100],
        'Z' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b10000, 0b11111],
        '0' => [0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110],
        '1' => [0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110],
        '2' => [0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111],
        '3' => [0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110],
        '4' => [0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010],
        '5' => [0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110],
        '6' => [0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110],
        '7' => [0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000],
        '8' => [0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110],
        '9' => [0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100],
        '-' => [0b00000, 0b00000, 0b00000, 0b11111, 0b00000, 0b00000, 0b00000],
        '_' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b11111],
        ':' => [0b00000, 0b00100, 0b00100, 0b00000, 0b00100, 0b00100, 0b00000],
        '/' => [0b00001, 0b00010, 0b00010, 0b00100, 0b01000, 0b01000, 0b10000],
        '(' => [0b00010, 0b00100, 0b01000, 0b01000, 0b01000, 0b00100, 0b00010],
        ')' => [0b01000, 0b00100, 0b00010, 0b00010, 0b00010, 0b00100, 0b01000],
        '[' => [0b01110, 0b01000, 0b01000, 0b01000, 0b01000, 0b01000, 0b01110],
        ']' => [0b01110, 0b00010, 0b00010, 0b00010, 0b00010, 0b00010, 0b01110],
        '&' => [0b01100, 0b10010, 0b10100, 0b01000, 0b10101, 0b10010, 0b01101],
        '+' => [0b00000, 0b00100, 0b00100, 0b11111, 0b00100, 0b00100, 0b00000],
        '=' => [0b00000, 0b00000, 0b11111, 0b00000, 0b11111, 0b00000, 0b00000],
        '\'' => [0b00100, 0b00100, 0b01000, 0b00000, 0b00000, 0b00000, 0b00000],
        '.' => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00100, 0b00100],
        _ => [0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000, 0b00000],
    }
}

fn cubic_bezier_pt(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
    let u = 1.0 - t;
    let b0 = u * u * u;
    let b1 = 3.0 * u * u * t;
    let b2 = 3.0 * u * t * t;
    let b3 = t * t * t;
    [
        b0 * p0[0] + b1 * p1[0] + b2 * p2[0] + b3 * p3[0],
        b0 * p0[1] + b1 * p1[1] + b2 * p2[1] + b3 * p3[1],
    ]
}

fn fill_polygon_px(img: &mut image::RgbaImage, poly: &[(i32, i32)], rgba: [u8; 4]) {
    if poly.len() < 3 {
        return;
    }
    let min_y = poly.iter().map(|p| p.1).min().unwrap_or(0).max(0);
    let max_y = poly
        .iter()
        .map(|p| p.1)
        .max()
        .unwrap_or(0)
        .min(img.height() as i32 - 1);
    let n = poly.len();
    for y in min_y..=max_y {
        let yf = y as f32 + 0.5;
        let mut nodes = Vec::new();
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = (poly[i].0 as f32, poly[i].1 as f32);
            let (xj, yj) = (poly[j].0 as f32, poly[j].1 as f32);
            if (yi < yf && yj >= yf) || (yj < yf && yi >= yf) {
                let x = xi + (yf - yi) / (yj - yi) * (xj - xi);
                nodes.push(x.round() as i32);
            }
            j = i;
        }
        nodes.sort_unstable();
        for pair in nodes.chunks_exact(2) {
            let x0 = pair[0].max(0);
            let x1 = pair[1].min(img.width() as i32 - 1);
            for x in x0..=x1 {
                img.put_pixel(x as u32, y as u32, image::Rgba(rgba));
            }
        }
    }
}

fn stroke_polygon_px(
    img: &mut image::RgbaImage,
    poly: &[(i32, i32)],
    rgba: [u8; 4],
    thickness: i32,
) {
    if poly.len() < 2 {
        return;
    }
    for i in 0..poly.len() {
        let p0 = poly[i];
        let p1 = poly[(i + 1) % poly.len()];
        draw_line_px(img, p0.0, p0.1, p1.0, p1.1, rgba, thickness);
    }
}

