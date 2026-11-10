//! Canvas & Palette Vector Symbol Renderer (`canvas_renderer.rs`).
//!
//! Renders US IEEE Std 91 logic gates, IC packages, 7-segment displays, 4x4 hex keypads,
//! 5V/GND symbols, wireless From/To tags, and mini palette icons.

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Pos2, Rect, Sense, Stroke, Vec2,
};

use crate::library::{GateDef, GateShapeStyle};
use crate::mechanics::{LogicState, WIRE_CONN_RADIUS};
use crate::simulator::{CircuitPage, GateInstance};
use crate::LogicCraftApp;

const SEVEN_SEG_TABLE: [u8; 16] = [
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

pub fn dist_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 < 1e-4 {
        return p.distance(a);
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    let proj = a + ab * t;
    p.distance(proj)
}

/// Draws a toolbar button for Undo or Redo with a crisp custom-painted curved arrow icon.
pub fn draw_undo_redo_toolbar_button(
    ui: &mut egui::Ui,
    label: &str,
    is_undo: bool,
    enabled: bool,
    dark_mode: bool,
) -> egui::Response {
    let p = crate::theme::palette(dark_mode);
    let desired_size = Vec2::new(62.0, 22.0);
    let (rect, resp) = ui.allocate_exact_size(
        desired_size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    let (bg_fill, border_stroke, fg_col, icon_col) = if !enabled {
        (
            p.bg_secondary,
            Stroke::new(1.0, p.border_secondary),
            p.text_muted,
            p.text_muted,
        )
    } else if resp.is_pointer_button_down_on() {
        (
            p.bg_elevated,
            Stroke::new(1.5, p.cyan),
            p.text_primary,
            p.cyan,
        )
    } else if resp.hovered() {
        (
            p.bg_elevated,
            Stroke::new(1.0, p.border_primary),
            p.text_primary,
            p.cyan,
        )
    } else {
        (
            p.bg_secondary,
            Stroke::new(1.0, p.border_secondary),
            p.text_primary,
            p.cyan,
        )
    };

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, bg_fill);
    painter.rect_stroke(rect, 0.0, border_stroke);

    // Draw curved Undo (counter-clockwise) or Redo (clockwise) arrow icon on the left side
    let ic = Pos2::new(rect.left() + 11.5, rect.center().y + 1.0);
    let r = 4.6_f32;
    let stroke = Stroke::new(1.6_f32, icon_col);
    let mut arc_pts = Vec::with_capacity(13);
    for i in 0..=12 {
        let t = i as f32 / 12.0;
        let ang = if is_undo {
            (30.0_f32 - t * 195.0_f32).to_radians()
        } else {
            (150.0_f32 + t * 195.0_f32).to_radians()
        };
        arc_pts.push(Pos2::new(ic.x + r * ang.cos(), ic.y + r * ang.sin()));
    }
    painter.add(egui::Shape::line(arc_pts.clone(), stroke));
    if let Some(&tip) = arc_pts.last() {
        if is_undo {
            painter.line_segment([tip, Pos2::new(tip.x + 3.6, tip.y - 1.2)], stroke);
            painter.line_segment([tip, Pos2::new(tip.x + 1.4, tip.y - 3.6)], stroke);
        } else {
            painter.line_segment([tip, Pos2::new(tip.x - 3.6, tip.y - 1.2)], stroke);
            painter.line_segment([tip, Pos2::new(tip.x - 1.4, tip.y - 3.6)], stroke);
        }
    }

    painter.text(
        Pos2::new(rect.left() + 21.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::new(11.0, FontFamily::Monospace),
        fg_col,
    );

    resp
}

/// Computes which of the 16 (`0x0`..`0xF`) buttons on a `DD_KEYPAD_HEX` was clicked.
pub fn hit_test_hex_keypad_button(
    canvas_rect: Rect,
    page: &CircuitPage,
    g: &GateInstance,
    def: &GateDef,
    mp: Pos2,
) -> Option<u16> {
    let hw = def.half_size[0];
    let hh = def.half_size[1];
    let c0 = LogicCraftApp::world_to_screen(canvas_rect, page, g.pin_world_pos([-hw, hh]));
    let c1 = LogicCraftApp::world_to_screen(canvas_rect, page, g.pin_world_pos([hw, -hh]));
    let box_r = Rect::from_two_pos(c0, c1).shrink(3.0);
    if !box_r.contains(mp) {
        return None;
    }
    let col = (((mp.x - box_r.left()) / box_r.width()) * 4.0).floor() as i32;
    let row = (((mp.y - box_r.top()) / box_r.height()) * 4.0).floor() as i32;
    if (0..4).contains(&row) && (0..4).contains(&col) {
        Some((row * 4 + col) as u16)
    } else {
        None
    }
}

/// Keycap layout for `KEYBOARD_COMPONENT`: `(label, key_code, local_pos, width_mult)`.
pub const ARCADE_KEYBOARD_KEYS: &[(&str, u8, [f32; 2], f32)] = &[
    ("W", b'W', [-2.2, 0.75], 1.0),
    ("S", b'S', [-2.2, -0.95], 1.0),
    ("▲", 0x11, [0.25, 0.75], 1.0),
    ("▼", 0x12, [0.25, -0.95], 1.0),
    ("SPC", b' ', [-0.95, -2.55], 2.2),
];

/// Full QWERTY + Numbers + Function + Arrow keycap layout for `FULL_KEYBOARD_COMPONENT`:
/// `(label, key_code, local_pos, width_mult)`.
pub const FULL_KEYBOARD_KEYS: &[(&str, u8, [f32; 2], f32)] = &[
    // Row 1: Numbers + Backspace
    ("1", b'1', [-7.2, 2.2], 1.0),
    ("2", b'2', [-5.9, 2.2], 1.0),
    ("3", b'3', [-4.6, 2.2], 1.0),
    ("4", b'4', [-3.3, 2.2], 1.0),
    ("5", b'5', [-2.0, 2.2], 1.0),
    ("6", b'6', [-0.7, 2.2], 1.0),
    ("7", b'7', [0.6, 2.2], 1.0),
    ("8", b'8', [1.9, 2.2], 1.0),
    ("9", b'9', [3.2, 2.2], 1.0),
    ("0", b'0', [4.5, 2.2], 1.0),
    ("BK", 0x08, [6.0, 2.2], 1.35),
    // Row 2: QWERTYUIOP
    ("Q", b'Q', [-7.0, 0.65], 1.0),
    ("W", b'W', [-5.7, 0.65], 1.0),
    ("E", b'E', [-4.4, 0.65], 1.0),
    ("R", b'R', [-3.1, 0.65], 1.0),
    ("T", b'T', [-1.8, 0.65], 1.0),
    ("Y", b'Y', [-0.5, 0.65], 1.0),
    ("U", b'U', [0.8, 0.65], 1.0),
    ("I", b'I', [2.1, 0.65], 1.0),
    ("O", b'O', [3.4, 0.65], 1.0),
    ("P", b'P', [4.7, 0.65], 1.0),
    // Row 3: ASDFGHJKL + Enter
    ("A", b'A', [-6.8, -0.9], 1.0),
    ("S", b'S', [-5.5, -0.9], 1.0),
    ("D", b'D', [-4.2, -0.9], 1.0),
    ("F", b'F', [-2.9, -0.9], 1.0),
    ("G", b'G', [-1.6, -0.9], 1.0),
    ("H", b'H', [-0.3, -0.9], 1.0),
    ("J", b'J', [1.0, -0.9], 1.0),
    ("K", b'K', [2.3, -0.9], 1.0),
    ("L", b'L', [3.6, -0.9], 1.0),
    ("ENT", 0x0D, [5.35, -0.9], 1.75),
    // Row 4: ESC + ZXCVBNM + UP
    ("ESC", 0x1B, [-6.85, -2.45], 1.35),
    ("Z", b'Z', [-5.2, -2.45], 1.0),
    ("X", b'X', [-3.9, -2.45], 1.0),
    ("C", b'C', [-2.6, -2.45], 1.0),
    ("V", b'V', [-1.3, -2.45], 1.0),
    ("B", b'B', [0.0, -2.45], 1.0),
    ("N", b'N', [1.3, -2.45], 1.0),
    ("M", b'M', [2.6, -2.45], 1.0),
    ("▲", 0x11, [4.4, -2.45], 1.15),
    // Row 5: SPACE + LEFT / DOWN / RIGHT
    ("SPACE", b' ', [-2.3, -4.05], 4.8),
    ("◄", 0x13, [3.1, -4.05], 1.15),
    ("▼", 0x12, [4.4, -4.05], 1.15),
    ("►", 0x14, [5.7, -4.05], 1.15),
];

/// Hit-tests interactive keycaps on `KEYBOARD_COMPONENT` or `FULL_KEYBOARD_COMPONENT`.
pub fn hit_test_keyboard_key(
    canvas_rect: Rect,
    page: &CircuitPage,
    g: &GateInstance,
    mp: Pos2,
) -> Option<u8> {
    let px_per_world = 1.0 / page.zoom_world_per_px.max(0.01);
    if g.type_name == "KEYBOARD_COMPONENT" {
        let key_w = (1.55 * px_per_world).clamp(12.0, 28.0);
        let key_h = (1.25 * px_per_world).clamp(10.0, 22.0);
        for &(_lbl, code, lpos, w_mult) in ARCADE_KEYBOARD_KEYS {
            let kc = LogicCraftApp::world_to_screen(canvas_rect, page, g.pin_world_pos(lpos));
            let kr = Rect::from_center_size(kc, Vec2::new(key_w * w_mult, key_h));
            if kr.contains(mp) {
                return Some(code);
            }
        }
    } else if g.type_name == "FULL_KEYBOARD_COMPONENT" {
        let key_w = (1.12 * px_per_world).clamp(8.0, 24.0);
        let key_h = (1.22 * px_per_world).clamp(8.0, 22.0);
        for &(_lbl, code, lpos, w_mult) in FULL_KEYBOARD_KEYS {
            let kc = LogicCraftApp::world_to_screen(canvas_rect, page, g.pin_world_pos(lpos));
            let kr = Rect::from_center_size(kc, Vec2::new(key_w * w_mult, key_h));
            if kr.contains(mp) {
                return Some(code);
            }
        }
    }
    None
}

pub fn fit_font_size(text: &str, max_width: f32, preferred_size: f32) -> f32 {
    let char_count = text.chars().count().max(1) as f32;
    let max_by_width = (max_width / (char_count * 0.64)).clamp(5.5, preferred_size);
    max_by_width.min(preferred_size).max(5.5)
}

pub fn cubic_bezier(p0: [f32; 2], p1: [f32; 2], p2: [f32; 2], p3: [f32; 2], t: f32) -> [f32; 2] {
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

pub fn draw_mini_gate_icon(
    painter: &egui::Painter,
    rect: Rect,
    style: GateShapeStyle,
    out_inverted: bool,
    in_inverted: bool,
    dark_mode: bool,
) {
    let p = crate::theme::palette(dark_mode);
    let fg_col = match style {
        GateShapeStyle::RamRom { .. } | GateShapeStyle::CustomIc | GateShapeStyle::CpuProcessor => p.purple,
        GateShapeStyle::VramDisplay => p.green,
        GateShapeStyle::FlipFlopD
        | GateShapeStyle::FlipFlopJK
        | GateShapeStyle::Register
        | GateShapeStyle::KeyboardInput
        | GateShapeStyle::FullKeyboard => p.orange,
        _ => p.cyan,
    };
    let fill_col = p.component_fill;
    let stroke = Stroke::new(1.35_f32, fg_col);
    let c = rect.center();
    // Compact width for logic gates in palette preview
    let w = match style {
        GateShapeStyle::And
        | GateShapeStyle::Or
        | GateShapeStyle::Xor
        | GateShapeStyle::Buffer
        | GateShapeStyle::TriState => rect.width() * 0.25,
        _ => rect.width() * 0.34,
    };
    let h = rect.height() * 0.36;

    // Input/output bubbles offset
    let x_l = if in_inverted { c.x - w + 2.4 } else { c.x - w };
    let x_r = if out_inverted { c.x + w - 2.4 } else { c.x + w };

    match style {
        GateShapeStyle::And => {
            let x_arc = (x_l + x_r) * 0.45;
            let rx = (x_r - x_arc).max(2.0);
            let mut pts = Vec::with_capacity(20);
            pts.push(Pos2::new(x_l, c.y - h));
            pts.push(Pos2::new(x_arc, c.y - h));
            for i in 0..=14 {
                let ang = -std::f32::consts::FRAC_PI_2
                    + (i as f32 / 14.0) * std::f32::consts::PI;
                pts.push(Pos2::new(x_arc + ang.cos() * rx, c.y + ang.sin() * h));
            }
            pts.push(Pos2::new(x_l, c.y + h));
            painter.add(egui::Shape::convex_polygon(pts.clone(), fill_col, Stroke::NONE));
            painter.add(egui::Shape::closed_line(pts, stroke));
        }
        GateShapeStyle::Or | GateShapeStyle::Xor => {
            let is_xor = style == GateShapeStyle::Xor;
            let body_l = if is_xor { x_l + 2.2 } else { x_l };
            let steps = 14usize;
            let mut outline = Vec::with_capacity(steps * 3 + 3);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                let p = cubic_bezier(
                    [body_l, c.y - h],
                    [body_l + (x_r - body_l) * 0.45, c.y - h],
                    [body_l + (x_r - body_l) * 0.82, c.y - h * 0.52],
                    [x_r, c.y],
                    t,
                );
                outline.push(Pos2::new(p[0], p[1]));
            }
            for i in 1..=steps {
                let t = i as f32 / steps as f32;
                let p = cubic_bezier(
                    [x_r, c.y],
                    [body_l + (x_r - body_l) * 0.82, c.y + h * 0.52],
                    [body_l + (x_r - body_l) * 0.45, c.y + h],
                    [body_l, c.y + h],
                    t,
                );
                outline.push(Pos2::new(p[0], p[1]));
            }
            for i in 1..steps {
                let u = 1.0 - (i as f32 / steps as f32) * 2.0;
                let bx = body_l + 2.6 * (1.0 - u * u);
                outline.push(Pos2::new(bx, c.y + u * h));
            }
            painter.add(egui::Shape::closed_line(outline, stroke));
            if is_xor {
                let mut shield = Vec::with_capacity(steps + 1);
                for i in 0..=steps {
                    let u = -1.0 + (i as f32 / steps as f32) * 2.0;
                    let sx = x_l - 0.4 + 2.6 * (1.0 - u * u);
                    shield.push(Pos2::new(sx, c.y + u * h));
                }
                painter.add(egui::Shape::line(shield, stroke));
            }
        }
        GateShapeStyle::Buffer | GateShapeStyle::TriState => {
            let tri = vec![
                Pos2::new(x_l, c.y - h),
                Pos2::new(x_r, c.y),
                Pos2::new(x_l, c.y + h),
            ];
            painter.add(egui::Shape::convex_polygon(tri.clone(), fill_col, Stroke::NONE));
            painter.add(egui::Shape::closed_line(tri, stroke));
            if style == GateShapeStyle::TriState {
                let mid_x = (x_l + x_r) * 0.5;
                let bot_y = c.y + h * 0.5;
                painter.line_segment([Pos2::new(mid_x, bot_y), Pos2::new(mid_x, c.y + h + 2.0)], stroke);
            }
        }
        GateShapeStyle::BusEnd => {
            for dy in [-h * 0.65, 0.0, h * 0.65] {
                painter.line_segment([Pos2::new(c.x - w, c.y + dy), Pos2::new(c.x, c.y + dy * 0.3)], stroke);
            }
            painter.line_segment(
                [Pos2::new(c.x, c.y), Pos2::new(c.x + w, c.y)],
                Stroke::new(2.6_f32, fg_col),
            );
            painter.line_segment([Pos2::new(c.x + 2.0, c.y - 4.0), Pos2::new(c.x + 6.0, c.y + 4.0)], stroke);
        }
        GateShapeStyle::TGate => {
            let t1 = vec![Pos2::new(c.x - w, c.y - h * 0.8), Pos2::new(c.x + w, c.y), Pos2::new(c.x - w, c.y + h * 0.8)];
            let t2 = vec![Pos2::new(c.x + w, c.y - h * 0.8), Pos2::new(c.x - w, c.y), Pos2::new(c.x + w, c.y + h * 0.8)];
            painter.add(egui::Shape::closed_line(t1, stroke));
            painter.add(egui::Shape::closed_line(t2, stroke));
            painter.circle_stroke(Pos2::new(c.x, c.y - h * 0.65), 2.0, stroke);
        }
        GateShapeStyle::Transistor { pmos } => {
            painter.line_segment([Pos2::new(c.x - w, c.y), Pos2::new(c.x - 3.5, c.y)], stroke);
            if pmos {
                painter.circle_stroke(Pos2::new(c.x - 5.2, c.y), 1.8, stroke);
            }
            painter.line_segment([Pos2::new(c.x - 2.5, c.y - h * 0.7), Pos2::new(c.x - 2.5, c.y + h * 0.7)], stroke);
            painter.line_segment([Pos2::new(c.x + 0.5, c.y - h * 0.7), Pos2::new(c.x + 0.5, c.y + h * 0.7)], stroke);
            painter.line_segment([Pos2::new(c.x + 0.5, c.y - h * 0.55), Pos2::new(c.x + w * 0.6, c.y - h * 0.55)], stroke);
            painter.line_segment([Pos2::new(c.x + w * 0.6, c.y - h * 0.55), Pos2::new(c.x + w * 0.6, c.y - h)], stroke);
            painter.line_segment([Pos2::new(c.x + 0.5, c.y + h * 0.55), Pos2::new(c.x + w * 0.6, c.y + h * 0.55)], stroke);
            painter.line_segment([Pos2::new(c.x + w * 0.6, c.y + h * 0.55), Pos2::new(c.x + w * 0.6, c.y + h)], stroke);
        }
        GateShapeStyle::FromLink | GateShapeStyle::ToLink => {
            let is_from = style == GateShapeStyle::FromLink;
            let poly = if is_from {
                vec![
                    Pos2::new(c.x - w, c.y),
                    Pos2::new(c.x - w * 0.45, c.y - h * 0.75),
                    Pos2::new(c.x + w, c.y - h * 0.75),
                    Pos2::new(c.x + w, c.y + h * 0.75),
                    Pos2::new(c.x - w * 0.45, c.y + h * 0.75),
                ]
            } else {
                vec![
                    Pos2::new(c.x - w, c.y - h * 0.75),
                    Pos2::new(c.x + w * 0.45, c.y - h * 0.75),
                    Pos2::new(c.x + w, c.y),
                    Pos2::new(c.x + w * 0.45, c.y + h * 0.75),
                    Pos2::new(c.x - w, c.y + h * 0.75),
                ]
            };
            painter.add(egui::Shape::convex_polygon(poly.clone(), fill_col, Stroke::NONE));
            painter.add(egui::Shape::closed_line(poly, stroke));
            painter.text(
                c,
                Align2::CENTER_CENTER,
                if is_from { "FM" } else { "TO" },
                FontId::new(8.0, FontFamily::Monospace),
                fg_col,
            );
        }
        GateShapeStyle::JunctionNode => {
            painter.line_segment([Pos2::new(c.x - w, c.y), Pos2::new(c.x + w, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x, c.y - h), Pos2::new(c.x, c.y + h)], stroke);
            painter.circle_filled(c, 3.0, fg_col);
        }
        GateShapeStyle::ToggleSwitch => {
            let box_r = Rect::from_center_size(c, Vec2::new(w * 2.0, h * 1.65));
            painter.rect_filled(box_r, 0.0, p.green);
            painter.rect_stroke(box_r, 0.0, Stroke::new(1.2, p.border_primary));
            painter.text(
                c,
                Align2::CENTER_CENTER,
                "1",
                FontId::new(9.0, FontFamily::Monospace),
                Color32::from_rgb(10, 14, 18),
            );
        }
        GateShapeStyle::Clock => {
            let box_r = Rect::from_center_size(c, Vec2::new(w * 2.0, h * 1.65));
            painter.rect_filled(box_r, 0.0, fill_col);
            painter.rect_stroke(box_r, 0.0, stroke);
            let pts = vec![
                Pos2::new(c.x - w * 0.65, c.y + h * 0.35),
                Pos2::new(c.x - w * 0.65, c.y - h * 0.35),
                Pos2::new(c.x - w * 0.1, c.y - h * 0.35),
                Pos2::new(c.x - w * 0.1, c.y + h * 0.35),
                Pos2::new(c.x + w * 0.45, c.y + h * 0.35),
                Pos2::new(c.x + w * 0.45, c.y - h * 0.35),
                Pos2::new(c.x + w * 0.7, c.y - h * 0.35),
            ];
            painter.add(egui::Shape::line(pts, Stroke::new(1.4, p.green)));
        }
        GateShapeStyle::Pulse => {
            let box_r = Rect::from_center_size(c, Vec2::new(w * 2.0, h * 1.65));
            painter.rect_filled(box_r, 0.0, fill_col);
            painter.rect_stroke(box_r, 0.0, stroke);
            let pts = vec![
                Pos2::new(c.x - w * 0.65, c.y + h * 0.35),
                Pos2::new(c.x - w * 0.25, c.y + h * 0.35),
                Pos2::new(c.x - w * 0.25, c.y - h * 0.4),
                Pos2::new(c.x + w * 0.25, c.y - h * 0.4),
                Pos2::new(c.x + w * 0.25, c.y + h * 0.35),
                Pos2::new(c.x + w * 0.65, c.y + h * 0.35),
            ];
            painter.add(egui::Shape::line(pts, stroke));
        }
        GateShapeStyle::HexKeypad => {
            let box_r = Rect::from_center_size(c, Vec2::new(w * 1.9, h * 1.9));
            painter.rect_filled(box_r, 0.0, fill_col);
            painter.rect_stroke(box_r, 0.0, stroke);
            for i in 1..4 {
                let x = box_r.left() + box_r.width() * (i as f32 / 4.0);
                let y = box_r.top() + box_r.height() * (i as f32 / 4.0);
                painter.line_segment([Pos2::new(x, box_r.top()), Pos2::new(x, box_r.bottom())], Stroke::new(0.8_f32, fg_col));
                painter.line_segment([Pos2::new(box_r.left(), y), Pos2::new(box_r.right(), y)], Stroke::new(0.8_f32, fg_col));
            }
        }
        GateShapeStyle::Vdd => {
            let v_stroke = Stroke::new(1.6_f32, p.green);
            painter.line_segment([Pos2::new(c.x, c.y + h), Pos2::new(c.x, c.y - h * 0.7)], v_stroke);
            painter.line_segment([Pos2::new(c.x - w * 0.55, c.y - h * 0.1), Pos2::new(c.x, c.y - h * 0.7)], v_stroke);
            painter.line_segment([Pos2::new(c.x + w * 0.55, c.y - h * 0.1), Pos2::new(c.x, c.y - h * 0.7)], v_stroke);
        }
        GateShapeStyle::Ground => {
            painter.line_segment([Pos2::new(c.x, c.y - h), Pos2::new(c.x, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x - w * 0.75, c.y), Pos2::new(c.x + w * 0.75, c.y)], stroke);
            painter.line_segment([Pos2::new(c.x - w * 0.5, c.y + h * 0.45), Pos2::new(c.x + w * 0.5, c.y + h * 0.45)], stroke);
            painter.line_segment([Pos2::new(c.x - w * 0.25, c.y + h * 0.85), Pos2::new(c.x + w * 0.25, c.y + h * 0.85)], stroke);
        }
        GateShapeStyle::Led => {
            let r = h * 0.82;
            painter.line_segment([Pos2::new(c.x - w, c.y), Pos2::new(c.x - r, c.y)], stroke);
            painter.circle_filled(c, r, p.red);
            painter.circle_stroke(c, r, stroke);
        }
        GateShapeStyle::LedDisplay { .. } => {
            let box_r = Rect::from_center_size(c, Vec2::new(w * 1.7, h * 1.9));
            painter.rect_filled(box_r, 0.0, p.bg_grid);
            painter.rect_stroke(box_r, 0.0, stroke);
            draw_seven_seg_digit(painter, box_r.shrink(2.5), 8);
        }
        GateShapeStyle::Label => {
            painter.text(
                c,
                Align2::CENTER_CENTER,
                "TXT",
                FontId::new(9.5, FontFamily::Monospace),
                fg_col,
            );
        }
        GateShapeStyle::Mux => {
            let poly = vec![
                Pos2::new(c.x - w * 0.85, c.y - h),
                Pos2::new(c.x + w * 0.85, c.y - h * 0.6),
                Pos2::new(c.x + w * 0.85, c.y + h * 0.6),
                Pos2::new(c.x - w * 0.85, c.y + h),
            ];
            painter.add(egui::Shape::convex_polygon(poly.clone(), fill_col, Stroke::NONE));
            painter.add(egui::Shape::closed_line(poly, stroke));
            painter.text(c, Align2::CENTER_CENTER, "MUX", FontId::new(7.5, FontFamily::Monospace), fg_col);
        }
        GateShapeStyle::Decoder => {
            let poly = vec![
                Pos2::new(c.x - w * 0.85, c.y - h * 0.65),
                Pos2::new(c.x + w * 0.85, c.y - h),
                Pos2::new(c.x + w * 0.85, c.y + h),
                Pos2::new(c.x - w * 0.85, c.y + h * 0.65),
            ];
            painter.add(egui::Shape::convex_polygon(poly.clone(), fill_col, Stroke::NONE));
            painter.add(egui::Shape::closed_line(poly, stroke));
            painter.text(c, Align2::CENTER_CENTER, "DEC", FontId::new(7.5, FontFamily::Monospace), fg_col);
        }
        _ => {
            let box_r = Rect::from_center_size(c, Vec2::new(w * 1.95, h * 1.85));
            painter.rect_filled(box_r, 0.0, fill_col);
            painter.rect_stroke(box_r, 0.0, stroke);
            let txt = match style {
                GateShapeStyle::PriEncoder => "ENC",
                GateShapeStyle::Adder => "Σ+",
                GateShapeStyle::Comparator => "A=B",
                GateShapeStyle::FlipFlopD => "D-FF",
                GateShapeStyle::FlipFlopJK => "JK",
                GateShapeStyle::Register => "REG",
                GateShapeStyle::RamRom { is_rom: false } => "RAM",
                GateShapeStyle::RamRom { is_rom: true } => "ROM",
                GateShapeStyle::Pauseulator => "||",
                GateShapeStyle::ChipBox => "IC",
                GateShapeStyle::CustomIc => "CUST",
                GateShapeStyle::CpuProcessor => "CPU",
                GateShapeStyle::VramDisplay => "VRAM",
                GateShapeStyle::KeyboardInput => "KEYS",
                GateShapeStyle::FullKeyboard => "ASCII",
                _ => "IC",
            };
            painter.text(
                c,
                Align2::CENTER_CENTER,
                txt,
                FontId::new(8.0, FontFamily::Monospace),
                fg_col,
            );
        }
    }

    if in_inverted {
        for dy in [-h * 0.45, h * 0.45] {
            painter.circle_stroke(Pos2::new(c.x - w + 1.2, c.y + dy), 1.5, stroke);
        }
    }
    if out_inverted && !matches!(style, GateShapeStyle::ChipBox) {
        painter.circle_stroke(Pos2::new(c.x + w - 1.1, c.y), 1.7, stroke);
    }
}

pub fn short_pin_label(name: &str) -> String {
    match name {
        "clock" | "CLK" | "write_clock" => "CLK".into(),
        "clock_enable" => "CE".into(),
        "count_enable" => "EN".into(),
        "shift_enable" => "SE".into(),
        "shift_left" => "SHL".into(),
        "up_down" => "U/D".into(),
        "load" => "LD".into(),
        "clear" | "CLR" => "CLR".into(),
        "RST" => "RST".into(),
        "set" => "SET".into(),
        "carry_in" => "Cin".into(),
        "carry_out" => "Cout".into(),
        "write_enable" | "WE" => "WE".into(),
        "enable" | "ENABLE" | "ENABLE_0" | "EN" => "EN".into(),
        "VALID" => "V".into(),
        "KEY_VALID" => "KVAL".into(),
        "VSYNC" => "VSY".into(),
        "RW" => "R/W".into(),
        "HALT" => "HLT".into(),
        "P1_W" => "P1:W".into(),
        "P1_S" => "P1:S".into(),
        "P2_UP" => "P2:▲".into(),
        "P2_DN" => "P2:▼".into(),
        "START" => "STRT".into(),
        "ENTER" => "ENT".into(),
        "LEFT" => "◄L".into(),
        "RIGHT" => "►R".into(),
        "UP" => "▲U".into(),
        "DOWN" => "▼D".into(),
        "Q_bar" | "OUTINV_0" => "Q'".into(),
        "in_A_greater_B" => ">in".into(),
        "in_A_equal_B" => "=in".into(),
        "in_A_less_B" => "<in".into(),
        "A_greater_B" => "A>B".into(),
        "A_equal_B" => "A=B".into(),
        "A_less_B" => "A<B".into(),
        other => {
            if let Some(rest) = other.strip_prefix("ADDRESS_") {
                format!("A{rest}")
            } else if let Some(rest) = other.strip_prefix("ADDR_") {
                format!("A{rest}")
            } else if let Some(rest) = other.strip_prefix("DATA_IN_") {
                format!("D{rest}")
            } else if let Some(rest) = other.strip_prefix("DATA_OUT_") {
                format!("Q{rest}")
            } else if let Some(rest) = other.strip_prefix("DATA_") {
                format!("D{rest}")
            } else if let Some(rest) = other.strip_prefix("ASCII_") {
                format!("C{rest}")
            } else if let Some(rest) = other.strip_prefix("sum_") {
                format!("S{rest}")
            } else if let Some(rest) = other.strip_prefix("IN_") {
                format!("I{rest}")
            } else if let Some(rest) = other.strip_prefix("OUT_") {
                format!("Q{rest}")
            } else if let Some(rest) = other.strip_prefix("SEL_") {
                format!("S{rest}")
            } else {
                other.replace('_', "")
            }
        }
    }
}

pub fn draw_gate_on_canvas(
    painter: &egui::Painter,
    canvas_rect: Rect,
    page: &CircuitPage,
    g: &GateInstance,
    def: &GateDef,
    cpu: &crate::cpu::CpuComponent,
    dark_mode: bool,
    show_wire_conn: bool,
) {
    let p = crate::theme::palette(dark_mode);
    let px_per_world = 1.0 / page.zoom_world_per_px.max(0.01);
    let to_screen = |local: [f32; 2]| -> Pos2 {
        let wp = g.pin_world_pos(local);
        LogicCraftApp::world_to_screen(canvas_rect, page, wp)
    };

    let body_stroke_col = if g.selected {
        p.cyan
    } else {
        p.component_stroke
    };
    let body_fill = if g.selected {
        p.bg_elevated
    } else {
        p.component_fill
    };
    let title_text_col = if g.selected {
        p.cyan
    } else {
        match def.shape_style {
            GateShapeStyle::RamRom { .. } | GateShapeStyle::CustomIc => p.purple,
            _ => p.cyan,
        }
    };
    // Subtle, clean US-style stroke width
    let stroke = Stroke::new(if g.selected { 2.0_f32 } else { 1.45_f32 }, body_stroke_col);

    let center_s = LogicCraftApp::world_to_screen(canvas_rect, page, g.pos);
    let hw = def.half_size[0];
    let hh = def.half_size[1];

    // Draw distinct CAD selection halo & tactical corner brackets when component is selected
    if g.selected {
        let sel_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh])).expand(5.0);
        painter.rect_filled(sel_r, 0.0, p.selection_fill);
        painter.rect_stroke(
            sel_r,
            0.0,
            Stroke::new(
                1.0,
                Color32::from_rgba_unmultiplied(p.cyan.r(), p.cyan.g(), p.cyan.b(), 120),
            ),
        );
        crate::theme::draw_corner_brackets(painter, sel_r, 6.0, Stroke::new(1.8, p.cyan));
    }

    match def.shape_style {
        GateShapeStyle::And => {
            // Compact US IEEE/ANSI AND gate:
            // Flat vertical left edge at x_left, horizontal top/bottom to x_arc, smooth 32-segment rounded nose to x_right
            let h_body = hh * 0.84;
            let x_left = if def.in_inverted { -hw + 0.50 } else { -hw };
            let x_right = if def.out_inverted { hw - 0.54 } else { hw };
            let x_arc = (x_left + x_right) * 0.45;
            let rx = (x_right - x_arc).max(0.4);
            let mut poly = Vec::with_capacity(38);
            poly.push(to_screen([x_left, h_body]));
            poly.push(to_screen([x_arc, h_body]));
            for step in 1..32 {
                let ang =
                    std::f32::consts::FRAC_PI_2 - (step as f32 / 32.0) * std::f32::consts::PI;
                poly.push(to_screen([x_arc + ang.cos() * rx, ang.sin() * h_body]));
            }
            poly.push(to_screen([x_arc, -h_body]));
            poly.push(to_screen([x_left, -h_body]));

            painter.add(egui::Shape::convex_polygon(poly.clone(), body_fill, Stroke::NONE));
            painter.add(egui::Shape::closed_line(poly, stroke));

            if def.in_inverted {
                let r = (0.25 * px_per_world).clamp(2.5, 5.0);
                for pin in &def.inputs {
                    let bub_c = to_screen([-hw + 0.25, pin.pos[1]]);
                    painter.circle_filled(bub_c, r, body_fill);
                    painter.circle_stroke(bub_c, r, stroke);
                }
            }
            if def.out_inverted {
                let r = (0.27 * px_per_world).clamp(2.8, 5.5);
                let bub_c = to_screen([hw - 0.27, 0.0]);
                painter.circle_filled(bub_c, r, body_fill);
                painter.circle_stroke(bub_c, r, stroke);
            }
        }
        GateShapeStyle::Or | GateShapeStyle::Xor => {
            // Compact US IEEE/ANSI OR & XOR gate:
            // Smooth concave input back curve + cubic Bezier top and bottom arches meeting at pointed nose
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

            let x_back_at = |u: f32| -> f32 {
                body_corner_x + back_depth * (1.0 - u * u)
            };
            let x_shield_at = |u: f32| -> f32 {
                base_corner_x + back_depth * (1.0 - u * u)
            };
            let front_pt = |u: f32| -> [f32; 2] {
                let sign = if u >= 0.0 { 1.0 } else { -1.0 };
                let t = 1.0 - u.abs();
                cubic_bezier(
                    [body_corner_x, sign * h_body],
                    [body_corner_x + (x_tip - body_corner_x) * 0.46, sign * h_body],
                    [body_corner_x + (x_tip - body_corner_x) * 0.82, sign * h_body * 0.54],
                    [x_tip, 0.0],
                    t,
                )
            };

            // Fill concave OR body using horizontal convex quad strips so the concave back is exact
            let strips = 28usize;
            for i in 0..strips {
                let u0 = -1.0 + (i as f32 / strips as f32) * 2.0;
                let u1 = -1.0 + ((i + 1) as f32 / strips as f32) * 2.0;
                let l0 = to_screen([x_back_at(u0), u0 * h_body]);
                let l1 = to_screen([x_back_at(u1), u1 * h_body]);
                let r0 = to_screen(front_pt(u0));
                let r1 = to_screen(front_pt(u1));
                painter.add(egui::Shape::convex_polygon(
                    vec![l0, r0, r1, l1],
                    body_fill,
                    Stroke::NONE,
                ));
            }

            // Build high-resolution continuous perimeter for subtle US-style stroke
            let mut outline = Vec::with_capacity(strips * 3 + 4);
            for i in 0..=strips {
                let t = i as f32 / strips as f32;
                let p = cubic_bezier(
                    [body_corner_x, h_body],
                    [body_corner_x + (x_tip - body_corner_x) * 0.46, h_body],
                    [body_corner_x + (x_tip - body_corner_x) * 0.82, h_body * 0.54],
                    [x_tip, 0.0],
                    t,
                );
                outline.push(to_screen(p));
            }
            for i in 1..=strips {
                let t = i as f32 / strips as f32;
                let p = cubic_bezier(
                    [x_tip, 0.0],
                    [body_corner_x + (x_tip - body_corner_x) * 0.82, -h_body * 0.54],
                    [body_corner_x + (x_tip - body_corner_x) * 0.46, -h_body],
                    [body_corner_x, -h_body],
                    t,
                );
                outline.push(to_screen(p));
            }
            for i in 1..strips {
                let u = -1.0 + (i as f32 / strips as f32) * 2.0;
                outline.push(to_screen([x_back_at(u), u * h_body]));
            }
            painter.add(egui::Shape::closed_line(outline, stroke));

            // Secondary curved input arc for XOR / XNOR
            if is_xor {
                let mut shield_pts = Vec::with_capacity(strips + 1);
                for i in 0..=strips {
                    let u = -1.0 + (i as f32 / strips as f32) * 2.0;
                    shield_pts.push(to_screen([x_shield_at(u), u * h_body]));
                }
                painter.add(egui::Shape::line(shield_pts, stroke));
            }

            // Ensure every input pin at [-hw, y] connects seamlessly to the back curve
            for pin in &def.inputs {
                let u = (pin.pos[1] / h_body).clamp(-1.0, 1.0);
                let curve_x = x_back_at(u);
                let start_x = if def.in_inverted { -hw + 0.50 } else { pin.pos[0] };
                if (curve_x - start_x).abs() > 0.04 {
                    painter.line_segment(
                        [to_screen([start_x, pin.pos[1]]), to_screen([curve_x, pin.pos[1]])],
                        stroke,
                    );
                }
                if def.in_inverted {
                    let r = (0.25 * px_per_world).clamp(2.5, 5.0);
                    let bub_c = to_screen([-hw + 0.25, pin.pos[1]]);
                    painter.circle_filled(bub_c, r, body_fill);
                    painter.circle_stroke(bub_c, r, stroke);
                }
            }
            if def.out_inverted {
                let r = (0.27 * px_per_world).clamp(2.8, 5.5);
                let bub_c = to_screen([hw - 0.27, 0.0]);
                painter.circle_filled(bub_c, r, body_fill);
                painter.circle_stroke(bub_c, r, stroke);
            }
        }
        GateShapeStyle::Buffer | GateShapeStyle::TriState => {
            if def.inputs.len() <= 2 {
                // Single-bit US IEEE triangle buffer / inverter / tri-state with pins directly on the edges
                let x_tip = if def.out_inverted { hw - 0.54 } else { hw };
                let poly = vec![
                    to_screen([-hw, hh]),
                    to_screen([x_tip, 0.0]),
                    to_screen([-hw, -hh]),
                ];
                painter.add(egui::Shape::convex_polygon(poly.clone(), body_fill, Stroke::NONE));
                painter.add(egui::Shape::closed_line(poly, stroke));

                if def.out_inverted {
                    let r = (0.27 * px_per_world).clamp(2.8, 5.5);
                    let bub_c = to_screen([hw - 0.27, 0.0]);
                    painter.circle_filled(bub_c, r, body_fill);
                    painter.circle_stroke(bub_c, r, stroke);
                }
                if let Some(en_pin) = def.inputs.iter().find(|p| p.name.starts_with("ENABLE")) {
                    if en_pin.inverted {
                        let r = (0.25 * px_per_world).clamp(2.5, 5.0);
                        let bub_c = to_screen([0.0, -1.10]);
                        painter.circle_filled(bub_c, r, body_fill);
                        painter.circle_stroke(bub_c, r, stroke);
                    }
                }
            } else {
                // Multi-bit Inverter / Tri-State Bus IC box spanning exact [-hw, hh]..[hw, -hh]
                let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
                painter.rect_filled(box_r, 0.0, body_fill);
                painter.rect_stroke(box_r, 0.0, stroke);
                let lbl = if def.out_inverted { "INV" } else { "TRI" };
                painter.text(
                    center_s,
                    Align2::CENTER_CENTER,
                    lbl,
                    FontId::new((0.9 * px_per_world).clamp(8.0, 12.0), FontFamily::Monospace),
                    title_text_col,
                );
            }
        }
        GateShapeStyle::BusEnd => {
            for pin in &def.inputs {
                painter.line_segment(
                    [to_screen(pin.pos), to_screen([0.0, pin.pos[1] * 0.25])],
                    stroke,
                );
            }
            painter.line_segment(
                [to_screen([0.0, -hh * 0.5]), to_screen([0.0, hh * 0.5])],
                Stroke::new(2.4_f32, body_stroke_col),
            );
            painter.line_segment(
                [to_screen([0.0, 0.0]), to_screen([hw, 0.0])],
                Stroke::new(2.6_f32, body_stroke_col),
            );
        }
        GateShapeStyle::TGate => {
            let t1 = vec![
                to_screen([-hw, hh * 0.7]),
                to_screen([hw, 0.0]),
                to_screen([-hw, -hh * 0.7]),
            ];
            let t2 = vec![
                to_screen([hw, hh * 0.7]),
                to_screen([-hw, 0.0]),
                to_screen([hw, -hh * 0.7]),
            ];
            painter.add(egui::Shape::convex_polygon(t1.clone(), body_fill, Stroke::NONE));
            painter.add(egui::Shape::closed_line(t1, stroke));
            painter.add(egui::Shape::closed_line(t2, stroke));
            painter.line_segment([to_screen([0.0, hh * 0.35]), to_screen([0.0, hh])], stroke);
        }
        GateShapeStyle::Transistor { pmos } => {
            let g_end = if pmos { -1.1 } else { -0.7 };
            painter.line_segment([to_screen([-hw, 0.0]), to_screen([g_end, 0.0])], stroke);
            if pmos {
                let r = (0.22 * px_per_world).clamp(2.2, 4.5);
                let bub = to_screen([-0.9, 0.0]);
                painter.circle_filled(bub, r, body_fill);
                painter.circle_stroke(bub, r, stroke);
            }
            painter.line_segment([to_screen([-0.65, -1.2]), to_screen([-0.65, 1.2])], stroke);
            painter.line_segment([to_screen([-0.2, -1.3]), to_screen([-0.2, 1.3])], stroke);
            painter.line_segment([to_screen([-0.2, 0.9]), to_screen([0.0, 0.9])], stroke);
            painter.line_segment([to_screen([0.0, 0.9]), to_screen([0.0, hh])], stroke);
            painter.line_segment([to_screen([-0.2, -0.9]), to_screen([0.0, -0.9])], stroke);
            painter.line_segment([to_screen([0.0, -0.9]), to_screen([0.0, -hh])], stroke);
        }
        GateShapeStyle::JunctionNode => {
            for pin in def.inputs.iter().chain(def.outputs.iter()) {
                painter.line_segment([to_screen([0.0, 0.0]), to_screen(pin.pos)], stroke);
            }
            painter.circle_filled(center_s, (0.35 * px_per_world).clamp(3.0, 6.0), body_stroke_col);
        }
        GateShapeStyle::Vdd => {
            let vdd_col = if g.selected { p.cyan } else { p.green };
            let vdd_stroke = Stroke::new(if g.selected { 2.4_f32 } else { 2.0_f32 }, vdd_col);
            let p_stem_bot = to_screen([0.0, -hh]);
            let p_tip = to_screen([0.0, 0.85]);
            let p_left = to_screen([-0.85, -0.05]);
            let p_right = to_screen([0.85, -0.05]);
            painter.line_segment([p_stem_bot, p_tip], vdd_stroke);
            painter.line_segment([p_left, p_tip], vdd_stroke);
            painter.line_segment([p_right, p_tip], vdd_stroke);
            let lbl_pos = to_screen([0.0, 1.45]);
            painter.text(
                lbl_pos,
                Align2::CENTER_CENTER,
                "5V",
                FontId::new((0.95 * px_per_world).clamp(9.0, 13.0), FontFamily::Monospace),
                vdd_col,
            );
        }
        GateShapeStyle::Ground => {
            let gnd_stroke = Stroke::new(if g.selected { 2.4_f32 } else { 1.8_f32 }, body_stroke_col);
            let p_stem_top = to_screen([0.0, hh]);
            let p_stem_mid = to_screen([0.0, 0.0]);
            painter.line_segment([p_stem_top, p_stem_mid], gnd_stroke);
            for &(w_bar, y_off) in &[(1.15f32, 0.0f32), (0.75f32, -0.45f32), (0.35f32, -0.9f32)] {
                painter.line_segment(
                    [to_screen([-w_bar, y_off]), to_screen([w_bar, y_off])],
                    gnd_stroke,
                );
            }
            let lbl_pos = to_screen([0.0, -1.5]);
            painter.text(
                lbl_pos,
                Align2::CENTER_CENTER,
                "GND",
                FontId::new((0.9 * px_per_world).clamp(8.5, 12.0), FontFamily::Monospace),
                p.text_secondary,
            );
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
                    to_screen([-hw, 0.0]),
                    to_screen([-1.5, 0.92]),
                    to_screen([hw, 0.92]),
                    to_screen([hw, -0.92]),
                    to_screen([-1.5, -0.92]),
                ]
            } else {
                vec![
                    to_screen([-hw, 0.92]),
                    to_screen([1.5, 0.92]),
                    to_screen([hw, 0.0]),
                    to_screen([1.5, -0.92]),
                    to_screen([-hw, -0.92]),
                ]
            };
            let is_local = g.is_tag_local();
            let tag_accent = if is_local { p.purple } else { p.cyan };
            painter.add(egui::Shape::convex_polygon(tag_poly.clone(), p.bg_panel, Stroke::NONE));
            painter.add(egui::Shape::closed_line(
                tag_poly,
                Stroke::new(if g.selected { 2.0 } else { 1.4 }, if g.selected { p.cyan } else { tag_accent }),
            ));
            let scope_badge = if is_local { "L" } else { "S" };
            let label = if is_from {
                format!("FM[{scope_badge}]:{jid}")
            } else {
                format!("TO[{scope_badge}]:{jid}")
            };
            let box_r = if is_from {
                Rect::from_two_pos(to_screen([-1.4, 0.85]), to_screen([hw - 0.1, -0.85]))
            } else {
                Rect::from_two_pos(to_screen([-hw + 0.1, 0.85]), to_screen([1.4, -0.85]))
            };
            let fsize = fit_font_size(
                &label,
                box_r.width() - 3.0,
                (0.82 * px_per_world).clamp(7.0, 11.5),
            );
            painter.with_clip_rect(box_r).text(
                box_r.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::new(fsize, FontFamily::Monospace),
                tag_accent,
            );
        }
        GateShapeStyle::ToggleSwitch => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh - 0.1]), to_screen([hw, -hh + 0.1]));
            let is_on = g
                .lparams
                .get("OUTPUT_NUM")
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0)
                != 0;
            let sw_fill = if is_on { p.green } else { p.bg_elevated };
            let sw_text_col = if is_on {
                Color32::from_rgb(10, 14, 18)
            } else {
                p.text_primary
            };
            painter.rect_filled(box_r, 0.0, sw_fill);
            painter.rect_stroke(box_r, 0.0, stroke);
            let txt = if is_on { "ON [1]" } else { "OFF [0]" };
            let fsize = fit_font_size(txt, box_r.width() - 6.0, (1.05 * px_per_world).clamp(8.5, 14.0));
            painter.with_clip_rect(box_r.shrink(1.5)).text(
                box_r.center(),
                Align2::CENTER_CENTER,
                txt,
                FontId::new(fsize, FontFamily::Monospace),
                sw_text_col,
            );
        }
        GateShapeStyle::Clock => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh - 0.1]), to_screen([hw, -hh + 0.1]));
            let is_high = g.pin_states.get("CLK").copied() == Some(LogicState::One);
            painter.rect_filled(box_r, 0.0, body_fill);
            painter.rect_stroke(box_r, 0.0, stroke);
            let sq_col = if is_high { p.green } else { p.cyan };
            let pts = [
                to_screen([-1.6, -0.55]),
                to_screen([-1.6, 0.55]),
                to_screen([-0.55, 0.55]),
                to_screen([-0.55, -0.55]),
                to_screen([0.55, -0.55]),
                to_screen([0.55, 0.55]),
                to_screen([1.6, 0.55]),
            ];
            for seg in pts.windows(2) {
                painter.line_segment([seg[0], seg[1]], Stroke::new(1.8_f32, sq_col));
            }
        }
        GateShapeStyle::Pulse => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh - 0.1]), to_screen([hw, -hh + 0.1]));
            painter.rect_filled(box_r, 0.0, body_fill);
            painter.rect_stroke(box_r, 0.0, stroke);
            let pts = [
                to_screen([-1.6, -0.5]),
                to_screen([-0.6, -0.5]),
                to_screen([-0.6, 0.6]),
                to_screen([0.6, 0.6]),
                to_screen([0.6, -0.5]),
                to_screen([1.6, -0.5]),
            ];
            for seg in pts.windows(2) {
                painter.line_segment([seg[0], seg[1]], stroke);
            }
        }
        GateShapeStyle::HexKeypad => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
            painter.rect_filled(box_r, 0.0, body_fill);
            painter.rect_stroke(box_r, 0.0, stroke);

            let val = g
                .lparams
                .get("OUTPUT_NUM")
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0)
                & 0xF;

            let inner = box_r.shrink(3.5);
            let cell_w = inner.width() / 4.0;
            let cell_h = inner.height() / 4.0;
            let key_font = FontId::new((cell_h * 0.62).clamp(7.5, 13.5), FontFamily::Monospace);

            for r in 0..4 {
                for c in 0..4 {
                    let key_val = (r * 4 + c) as u16;
                    let cell_rect = Rect::from_min_size(
                        Pos2::new(
                            inner.left() + c as f32 * cell_w,
                            inner.top() + r as f32 * cell_h,
                        ),
                        Vec2::new(cell_w, cell_h),
                    )
                    .shrink(1.0);
                    let is_selected_key = key_val == val;
                    let key_bg = if is_selected_key {
                        p.cyan
                    } else {
                        p.bg_elevated
                    };
                    let key_fg = if is_selected_key {
                        Color32::from_rgb(10, 14, 18)
                    } else {
                        p.text_primary
                    };
                    painter.rect_filled(cell_rect, 0.0, key_bg);
                    painter.rect_stroke(cell_rect, 0.0, Stroke::new(1.0, p.border_secondary));
                    painter.text(
                        cell_rect.center(),
                        Align2::CENTER_CENTER,
                        format!("{key_val:X}"),
                        key_font.clone(),
                        key_fg,
                    );
                }
            }
        }
        GateShapeStyle::Led => {
            let st = g.pin_states.get("IN_0").copied().unwrap_or(LogicState::HiZ);
            let fill = match st {
                LogicState::One => p.red,
                LogicState::Zero => p.bg_elevated,
                _ => st.wire_color(dark_mode),
            };
            let led_c = to_screen([-0.5, 0.0]);
            let r = (1.5 * px_per_world).clamp(7.0, 26.0);
            painter.circle_filled(led_c, r, fill);
            painter.circle_stroke(led_c, r, stroke);
        }
        GateShapeStyle::LedDisplay { digits } => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
            painter.rect_filled(box_r, 0.0, p.bg_grid);
            painter.rect_stroke(box_r, 0.0, stroke);
            let val = g
                .lparams
                .get("CURRENT_VALUE")
                .and_then(|s| s.parse::<u16>().ok())
                .unwrap_or(0);
            if digits == 1 {
                draw_seven_seg_digit(painter, box_r.shrink(7.0), (val & 0xF) as usize);
            } else {
                let left_r = Rect::from_min_max(
                    box_r.left_top() + Vec2::new(6.0, 6.0),
                    Pos2::new(box_r.center().x - 2.0, box_r.bottom() - 6.0),
                );
                let right_r = Rect::from_min_max(
                    Pos2::new(box_r.center().x + 2.0, box_r.top() + 6.0),
                    box_r.right_bottom() - Vec2::new(6.0, 6.0),
                );
                draw_seven_seg_digit(painter, left_r, ((val >> 4) & 0xF) as usize);
                draw_seven_seg_digit(painter, right_r, (val & 0xF) as usize);
            }
        }
        GateShapeStyle::Label => {
            let txt = g
                .gparams
                .get("LABEL_TEXT")
                .map(|s| s.as_str())
                .unwrap_or("Label");
            painter.text(
                center_s,
                Align2::CENTER_CENTER,
                txt,
                FontId::new((1.25 * px_per_world).clamp(11.0, 20.0), FontFamily::Monospace),
                if g.selected { p.cyan } else { p.text_primary },
            );
        }
        GateShapeStyle::VramDisplay => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
            painter.rect_filled(box_r, 2.0, body_fill);
            painter.rect_stroke(box_r, 2.0, stroke);

            let disp = crate::cpu::DisplayComponent::from_lparams(&g.lparams);
            let (res_w, res_h) = disp.resolution.dimensions();
            let header_h = (1.35 * px_per_world).clamp(12.0, 22.0);
            let pad = (0.55 * px_per_world).clamp(4.0, 10.0);
            let side_pad = if px_per_world >= 7.0 {
                (1.45 * px_per_world).clamp(14.0, 26.0)
            } else {
                pad
            };

            // Header bar with VRAM Base Address, Resolution & Palette badge
            let hdr_r = Rect::from_min_max(
                box_r.left_top() + Vec2::new(pad, 2.0),
                Pos2::new(box_r.right() - pad, (box_r.top() + header_h).min(box_r.bottom())),
            );
            let hdr_txt = format!(
                "VRAM 0x{:04X} [{} // {}]",
                disp.vram_base,
                disp.resolution.label(),
                disp.palette.code_str()
            );
            let hdr_fsize = fit_font_size(&hdr_txt, hdr_r.width() - 4.0, (0.68 * px_per_world).clamp(7.0, 11.0));
            painter.with_clip_rect(hdr_r).text(
                hdr_r.center(),
                Align2::CENTER_CENTER,
                hdr_txt,
                FontId::new(hdr_fsize, FontFamily::Monospace),
                p.green,
            );

            // Draw monitor pin labels inside the left/right bezel (`CLK`, `WE`, `RST`, `EN` & `VSY`)
            if px_per_world >= 7.0 {
                let clipped = painter.with_clip_rect(box_r.shrink(1.5));
                let pin_font = FontId::new((0.46 * px_per_world).clamp(5.8, 8.5), FontFamily::Monospace);
                for pin in def.inputs.iter().chain(def.outputs.iter()) {
                    let lbl = short_pin_label(&pin.name);
                    if (pin.pos[0] + hw).abs() < 0.2 {
                        let tp = to_screen([-hw + 0.18, pin.pos[1]]);
                        clipped.text(tp, Align2::LEFT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    } else if (pin.pos[0] - hw).abs() < 0.2 {
                        let tp = to_screen([hw - 0.18, pin.pos[1]]);
                        clipped.text(tp, Align2::RIGHT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    }
                }
            }

            // Screen CRT/LCD Bezel
            let screen_r = Rect::from_min_max(
                Pos2::new(box_r.left() + side_pad, box_r.top() + header_h + 2.0),
                Pos2::new(box_r.right() - side_pad, box_r.bottom() - pad),
            );
            if screen_r.width() > 4.0 && screen_r.height() > 4.0 {
                let bg_col = disp.palette.pixel_color(0);
                painter.rect_filled(screen_r, 1.0, bg_col);

                let en_low = g.pin_states.get("EN").copied() == Some(LogicState::Zero);
                if en_low {
                    painter.with_clip_rect(screen_r).text(
                        screen_r.center(),
                        Align2::CENTER_CENTER,
                        "BLANKING (EN=0)",
                        FontId::new((0.65 * px_per_world).clamp(7.0, 11.0), FontFamily::Monospace),
                        p.text_muted,
                    );
                } else {
                    let pw = screen_r.width() / res_w as f32;
                    let ph = screen_r.height() / res_h as f32;
                    let base = disp.vram_base as u32;

                    // Horizontal run-length batched pixel rendering for maximum FPS
                    for py in 0..res_h {
                        let row_addr = base.wrapping_add((py * res_w) as u32);
                        let y0 = screen_r.top() + py as f32 * ph;
                        let y1 = y0 + ph.max(1.0);
                        let mut px = 0usize;
                        while px < res_w {
                            let val = cpu.bus.peek8(row_addr.wrapping_add(px as u32));
                            if val == 0 {
                                px += 1;
                                continue;
                            }
                            let col = disp.palette.pixel_color(val);
                            let start_x = px;
                            px += 1;
                            while px < res_w {
                                let next_val = cpu.bus.peek8(row_addr.wrapping_add(px as u32));
                                if next_val == 0 || disp.palette.pixel_color(next_val) != col {
                                    break;
                                }
                                px += 1;
                            }
                            let x0 = screen_r.left() + start_x as f32 * pw;
                            let x1 = screen_r.left() + px as f32 * pw;
                            painter.rect_filled(
                                Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)),
                                0.0,
                                col,
                            );
                        }
                    }
                }
                painter.rect_stroke(screen_r, 1.0, Stroke::new(1.1, p.border_primary));
            }
        }
        GateShapeStyle::CpuProcessor => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
            painter.rect_filled(box_r, 1.5, body_fill);
            painter.rect_stroke(box_r, 1.5, stroke);

            let state = cpu.core.get_state();
            let clipped = painter.with_clip_rect(box_r.shrink(2.0));
            let title = format!("CPU // {}", state.arch.code_str());
            let fsize = fit_font_size(&title, box_r.width() - 16.0, (0.85 * px_per_world).clamp(8.0, 12.5));
            clipped.text(
                Pos2::new(box_r.center().x, box_r.top() + fsize + 4.0),
                Align2::CENTER_CENTER,
                title,
                FontId::new(fsize, FontFamily::Monospace),
                p.purple,
            );

            let pc_txt = format!("PC:0x{:04X}", state.pc & 0xFFFF);
            let clk_txt = if cpu.use_internal_clock {
                if cpu.clock_hz >= 1_000_000 {
                    format!("{:.1}MHz", cpu.clock_hz as f32 / 1_000_000.0)
                } else {
                    format!("{}kHz", cpu.clock_hz / 1_000)
                }
            } else {
                "EXT-CLK".to_string()
            };
            let run_txt = if state.halted {
                "HALTED"
            } else if cpu.running {
                "● RUN"
            } else {
                "■ PAUSE"
            };
            let run_col = if state.halted {
                p.red
            } else if cpu.running {
                p.green
            } else {
                p.orange
            };
            let sub_fsize = (0.68 * px_per_world).clamp(6.8, 10.5);
            clipped.text(
                Pos2::new(box_r.center().x, box_r.center().y - sub_fsize * 0.7),
                Align2::CENTER_CENTER,
                pc_txt,
                FontId::new(sub_fsize, FontFamily::Monospace),
                p.cyan,
            );
            clipped.text(
                Pos2::new(box_r.center().x, box_r.center().y + sub_fsize * 0.65),
                Align2::CENTER_CENTER,
                format!("{run_txt} {clk_txt}"),
                FontId::new(sub_fsize * 0.92, FontFamily::Monospace),
                run_col,
            );

            if px_per_world >= 7.0 {
                let pin_font = FontId::new((0.50 * px_per_world).clamp(6.0, 8.8), FontFamily::Monospace);
                for pin in def.inputs.iter().chain(def.outputs.iter()) {
                    let lbl = short_pin_label(&pin.name);
                    if (pin.pos[0] + hw).abs() < 0.2 {
                        let tp = to_screen([-hw + 0.25, pin.pos[1]]);
                        clipped.text(tp, Align2::LEFT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    } else if (pin.pos[0] - hw).abs() < 0.2 {
                        let tp = to_screen([hw - 0.25, pin.pos[1]]);
                        clipped.text(tp, Align2::RIGHT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    }
                }
            }
        }
        GateShapeStyle::KeyboardInput => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
            painter.rect_filled(box_r, 1.5, body_fill);
            painter.rect_stroke(box_r, 1.5, stroke);

            let ks = cpu.keyboard.effective_state();
            let clipped = painter.with_clip_rect(box_r.shrink(2.0));
            let fsize = fit_font_size("MMIO KEYBOARD", box_r.width() - 8.0, (0.72 * px_per_world).clamp(7.0, 10.5));
            clipped.text(
                Pos2::new(box_r.center().x, box_r.top() + fsize * 0.8 + 3.0),
                Align2::CENTER_CENTER,
                "MMIO KEYBOARD",
                FontId::new(fsize, FontFamily::Monospace),
                p.orange,
            );

            // Render 5 interactive keycaps: W, S (P1), UP, DN (P2), and SPACE (START)
            let key_w = (1.55 * px_per_world).clamp(12.0, 28.0);
            let key_h = (1.25 * px_per_world).clamp(10.0, 22.0);
            for &(lbl, code, lpos, w_mult) in ARCADE_KEYBOARD_KEYS {
                let active = match code {
                    b'W' => ks.p1_up,
                    b'S' => ks.p1_down,
                    0x11 => ks.p2_up,
                    0x12 => ks.p2_down,
                    b' ' => ks.start,
                    _ => false,
                };
                let kc = to_screen(lpos);
                let kr = Rect::from_center_size(kc, Vec2::new(key_w * w_mult, key_h));
                let bg = if active { p.green } else { p.bg_elevated };
                let fg = if active {
                    Color32::from_rgb(10, 14, 18)
                } else {
                    p.text_primary
                };
                clipped.rect_filled(kr, 2.0, bg);
                clipped.rect_stroke(kr, 2.0, Stroke::new(1.0, p.border_primary));
                clipped.text(
                    kr.center(),
                    Align2::CENTER_CENTER,
                    lbl,
                    FontId::new((key_h * 0.56).clamp(6.5, 11.0), FontFamily::Monospace),
                    fg,
                );
            }

            if px_per_world >= 7.0 {
                let pin_font = FontId::new((0.46 * px_per_world).clamp(5.8, 8.4), FontFamily::Monospace);
                for pin in def.inputs.iter().chain(def.outputs.iter()) {
                    let lbl = short_pin_label(&pin.name);
                    if (pin.pos[0] + hw).abs() < 0.2 {
                        let tp = to_screen([-hw + 0.2, pin.pos[1]]);
                        clipped.text(tp, Align2::LEFT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    } else if (pin.pos[0] - hw).abs() < 0.2 {
                        let tp = to_screen([hw - 0.2, pin.pos[1]]);
                        clipped.text(tp, Align2::RIGHT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    }
                }
            }
        }
        GateShapeStyle::FullKeyboard => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
            painter.rect_filled(box_r, 2.0, body_fill);
            painter.rect_stroke(box_r, 2.0, stroke);

            let ks = cpu.keyboard.effective_state();
            let clipped = painter.with_clip_rect(box_r.shrink(2.0));

            let disp_code = if ks.ascii_code != 0 {
                ks.ascii_code
            } else {
                ks.last_key
            };
            let ch_disp = if (0x20..=0x7E).contains(&disp_code) {
                format!("'{}'", disp_code as char)
            } else {
                match disp_code {
                    0x00 => "IDLE".into(),
                    0x08 => "BKSP".into(),
                    0x0D => "ENTER".into(),
                    0x1B => "ESC".into(),
                    0x11 => "UP ▲".into(),
                    0x12 => "DN ▼".into(),
                    0x13 => "LT ◄".into(),
                    0x14 => "RT ►".into(),
                    _ => format!("0x{disp_code:02X}"),
                }
            };
            let hdr_txt = format!(
                "QWERTY / ASCII KEYBOARD [MMIO 0xF000 // CODE: 0x{:02X} ({})]",
                disp_code, ch_disp
            );
            let hdr_fsize = fit_font_size(
                &hdr_txt,
                box_r.width() - 18.0,
                (0.65 * px_per_world).clamp(6.8, 10.5),
            );
            clipped.text(
                Pos2::new(box_r.center().x - 0.6 * px_per_world, box_r.top() + hdr_fsize + 3.0),
                Align2::CENTER_CENTER,
                hdr_txt,
                FontId::new(hdr_fsize, FontFamily::Monospace),
                if ks.key_valid { p.green } else { p.orange },
            );

            let key_w = (1.12 * px_per_world).clamp(8.0, 24.0);
            let key_h = (1.22 * px_per_world).clamp(8.0, 22.0);
            for &(lbl, code, lpos, w_mult) in FULL_KEYBOARD_KEYS {
                let active = ks.ascii_code == code
                    || (code == b'W' && ks.p1_up)
                    || (code == b'S' && ks.p1_down)
                    || (code == b'A' && ks.p1_left)
                    || (code == b'D' && ks.p1_right)
                    || (code == 0x11 && ks.p2_up)
                    || (code == 0x12 && ks.p2_down)
                    || (code == 0x13 && ks.p2_left)
                    || (code == 0x14 && ks.p2_right)
                    || (code == b' ' && ks.space)
                    || (code == 0x0D && ks.enter)
                    || (code == 0x08 && ks.backspace)
                    || (code == 0x1B && ks.escape);
                let is_last = !active && ks.last_key == code && code != 0;
                let kc = to_screen(lpos);
                let kr = Rect::from_center_size(kc, Vec2::new(key_w * w_mult, key_h));
                let bg = if active {
                    p.green
                } else if is_last {
                    p.cyan.linear_multiply(0.32)
                } else {
                    p.bg_elevated
                };
                let fg = if active {
                    Color32::from_rgb(10, 14, 18)
                } else if is_last {
                    p.cyan
                } else {
                    p.text_primary
                };
                clipped.rect_filled(kr, 2.0, bg);
                clipped.rect_stroke(kr, 2.0, Stroke::new(0.9, p.border_primary));
                clipped.text(
                    kr.center(),
                    Align2::CENTER_CENTER,
                    lbl,
                    FontId::new((key_h * 0.50).clamp(5.8, 10.0), FontFamily::Monospace),
                    fg,
                );
            }

            if px_per_world >= 7.0 {
                let pin_font = FontId::new((0.44 * px_per_world).clamp(5.5, 8.2), FontFamily::Monospace);
                for pin in def.inputs.iter().chain(def.outputs.iter()) {
                    let lbl = short_pin_label(&pin.name);
                    if (pin.pos[0] + hw).abs() < 0.2 {
                        let tp = to_screen([-hw + 0.2, pin.pos[1]]);
                        clipped.text(tp, Align2::LEFT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    } else if (pin.pos[0] - hw).abs() < 0.2 {
                        let tp = to_screen([hw - 0.2, pin.pos[1]]);
                        clipped.text(tp, Align2::RIGHT_CENTER, lbl, pin_font.clone(), p.text_secondary);
                    }
                }
            }
        }
        _ => {
            let box_r = Rect::from_two_pos(to_screen([-hw, hh]), to_screen([hw, -hh]));
            painter.rect_filled(box_r, 0.0, body_fill);
            painter.rect_stroke(box_r, 0.0, stroke);

            let short_title = match def.shape_style {
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
                GateShapeStyle::CustomIc => g
                    .gparams
                    .get("CHIP_LABEL")
                    .or_else(|| def.default_gparams.get("CHIP_LABEL"))
                    .map(|s| s.as_str())
                    .unwrap_or("CUSTOM"),
                _ => &def.logic_type,
            };
            let clipped = painter.with_clip_rect(box_r.shrink(1.5));
            let fsize = fit_font_size(
                short_title,
                box_r.width() - 16.0,
                (0.88 * px_per_world).clamp(7.5, 12.0),
            );
            let title_y = if box_r.height() > 36.0 {
                box_r.top() + (fsize * 0.8 + 8.0).min(box_r.height() * 0.28)
            } else {
                box_r.center().y
            };
            clipped.text(
                Pos2::new(box_r.center().x, title_y),
                Align2::CENTER_CENTER,
                short_title,
                FontId::new(fsize, FontFamily::Monospace),
                title_text_col,
            );
            if let Some(cv) = g.lparams.get("CURRENT_VALUE") {
                if let Ok(v) = cv.parse::<u16>() {
                    if box_r.height() > 30.0 {
                        let val_txt = format!("0x{v:X}");
                        let vfsize = fit_font_size(
                            &val_txt,
                            box_r.width() - 16.0,
                            (1.05 * px_per_world).clamp(8.5, 13.5),
                        );
                        clipped.text(
                            box_r.center() + Vec2::new(0.0, 4.0),
                            Align2::CENTER_CENTER,
                            val_txt,
                            FontId::new(vfsize, FontFamily::Monospace),
                            p.orange,
                        );
                    }
                }
            }

            if px_per_world >= 8.5 {
                let pin_font = FontId::new((0.56 * px_per_world).clamp(6.5, 9.5), FontFamily::Monospace);
                let pin_lbl_col = p.text_secondary;
                for pin in def.inputs.iter().chain(def.outputs.iter()) {
                    let lbl = short_pin_label(&pin.name);
                    let on_left = (pin.pos[0] + hw).abs() < 0.15;
                    let on_right = (pin.pos[0] - hw).abs() < 0.15;
                    let on_top = (pin.pos[1] - hh).abs() < 0.15;
                    let on_bot = (pin.pos[1] + hh).abs() < 0.15;

                    if pin.name.contains("clock") && on_left {
                        let p_top = to_screen([-hw, pin.pos[1] + 0.35]);
                        let p_tip = to_screen([-hw + 0.45, pin.pos[1]]);
                        let p_bot = to_screen([-hw, pin.pos[1] - 0.35]);
                        painter.line_segment([p_top, p_tip], Stroke::new(1.2_f32, pin_lbl_col));
                        painter.line_segment([p_tip, p_bot], Stroke::new(1.2_f32, pin_lbl_col));
                    } else if on_left {
                        let tp = to_screen([-hw + 0.25, pin.pos[1]]);
                        clipped.text(tp, Align2::LEFT_CENTER, lbl, pin_font.clone(), pin_lbl_col);
                    } else if on_right {
                        let tp = to_screen([hw - 0.25, pin.pos[1]]);
                        clipped.text(tp, Align2::RIGHT_CENTER, lbl, pin_font.clone(), pin_lbl_col);
                    } else if on_top && (pin.pos[0]).abs() < 0.1 {
                        let tp = to_screen([pin.pos[0], hh - 0.42]);
                        clipped.text(tp, Align2::CENTER_CENTER, lbl, pin_font.clone(), pin_lbl_col);
                    } else if on_bot && (pin.pos[0]).abs() < 0.1 {
                        let tp = to_screen([pin.pos[0], -hh + 0.42]);
                        clipped.text(tp, Align2::CENTER_CENTER, lbl, pin_font.clone(), pin_lbl_col);
                    }

                    if pin.inverted {
                        let r = (0.22 * px_per_world).clamp(2.2, 4.5);
                        let p_s = to_screen(pin.pos);
                        painter.circle_filled(p_s, r, body_fill);
                        painter.circle_stroke(p_s, r, stroke);
                    }
                }
            }
        }
    }

    if show_wire_conn {
        let r = (WIRE_CONN_RADIUS * px_per_world).clamp(2.0, 3.8);
        for pin in def.inputs.iter().chain(def.outputs.iter()) {
            let pin_s = to_screen(pin.pos);
            let st = g
                .pin_states
                .get(&pin.name)
                .copied()
                .unwrap_or(LogicState::HiZ);
            painter.circle_filled(pin_s, r, st.wire_color(dark_mode));
        }
    }
}

pub fn draw_seven_seg_digit(painter: &egui::Painter, rect: Rect, digit: usize) {
    let mask = SEVEN_SEG_TABLE[digit & 0xF];
    let l = rect.left() + 3.0;
    let r = rect.right() - 3.0;
    let t = rect.top() + 3.0;
    let b = rect.bottom() - 3.0;
    let m = (t + b) * 0.5;

    let segs: [([Pos2; 2], u8); 7] = [
        ([Pos2::new(l, t), Pos2::new(r, t)], 0), // a (top)
        ([Pos2::new(r, t), Pos2::new(r, m)], 1), // b (top-right)
        ([Pos2::new(r, m), Pos2::new(r, b)], 2), // c (bot-right)
        ([Pos2::new(l, b), Pos2::new(r, b)], 3), // d (bottom)
        ([Pos2::new(l, m), Pos2::new(l, b)], 4), // e (bot-left)
        ([Pos2::new(l, t), Pos2::new(l, m)], 5), // f (top-left)
        ([Pos2::new(l, m), Pos2::new(r, m)], 6), // g (middle)
    ];

    for (pts, bit) in segs {
        let on = ((mask >> bit) & 1) != 0;
        let col = if on {
            crate::theme::RED
        } else {
            Color32::from_rgb(30, 22, 26)
        };
        painter.line_segment(pts, Stroke::new(if on { 3.2_f32 } else { 1.4_f32 }, col));
    }
}
