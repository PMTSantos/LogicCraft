//! Centralized visual theme — **Dark Technical Neo-Brutalism** (`theme.rs`).
//!
//! Every color, stroke, rounding and spacing used by the UI is defined here so the look of the
//! whole application can be changed from a single place.
//!
//! ## Color semantics (keep consistent across the app)
//! | Accent   | Meaning                                                    |
//! |----------|------------------------------------------------------------|
//! | `CYAN`   | Information, connections, selection & active elements      |
//! | `GREEN`  | Execution / running, positive states, logic HIGH           |
//! | `ORANGE` | Warnings, operations, intermediate states, register values |
//! | `RED`    | Errors, conflicts, alert / active-indicator states (LEDs)  |
//! | `PURPLE` | Special elements: memory (RAM/ROM), custom ICs, K-Map      |

use eframe::egui::{
    self, style::Selection, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin,
    Pos2, Rect, Response, RichText, Rounding, Sense, Stroke, TextStyle, Ui, Vec2,
};
use eframe::epaint::Shadow;

// ───────────────────────────── Base palette (dark) ─────────────────────────────
pub const BG_MAIN: Color32 = Color32::from_rgb(0x0D, 0x0F, 0x12);
pub const BG_SECONDARY: Color32 = Color32::from_rgb(0x12, 0x15, 0x1A);
pub const BG_PANEL: Color32 = Color32::from_rgb(0x17, 0x1B, 0x21);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(0x1C, 0x22, 0x2A);
pub const BG_GRID: Color32 = Color32::from_rgb(0x0B, 0x0E, 0x12);
pub const BORDER_PRIMARY: Color32 = Color32::from_rgb(0x30, 0x39, 0x45);
pub const BORDER_SECONDARY: Color32 = Color32::from_rgb(0x24, 0x2B, 0x34);

pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0xE2, 0xE8, 0xF0);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x92, 0x9D, 0xAA);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x65, 0x71, 0x80);

// ───────────────────────────── Accents ─────────────────────────────
pub const CYAN: Color32 = Color32::from_rgb(0x38, 0xBD, 0xF8);
pub const GREEN: Color32 = Color32::from_rgb(0x22, 0xC5, 0x5E);
pub const ORANGE: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
pub const RED: Color32 = Color32::from_rgb(0xEF, 0x44, 0x44);
pub const PURPLE: Color32 = Color32::from_rgb(0xA7, 0x8B, 0xFA);

/// Global geometry: straight corners (0..2 px max).
pub const RADIUS: f32 = 0.0;
pub const RADIUS_SMALL: f32 = 1.0;

/// Runtime palette (dark = neo-brutalist lab console, light = engineering blueprint paper).
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg_main: Color32,
    pub bg_secondary: Color32,
    pub bg_panel: Color32,
    pub bg_elevated: Color32,
    pub bg_grid: Color32,
    pub grid_minor: Color32,
    pub grid_major: Color32,
    pub border: Color32,
    pub border_weak: Color32,
    pub border_primary: Color32,
    pub border_secondary: Color32,
    pub text: Color32,
    pub text_dim: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_muted: Color32,
    pub selection_fill: Color32,
    pub cyan: Color32,
    pub green: Color32,
    pub orange: Color32,
    pub red: Color32,
    pub purple: Color32,
    /// Fill used inside gate bodies / component packages on the canvas
    pub component_fill: Color32,
    /// Default outline of components on the canvas
    pub component_stroke: Color32,
    /// Logic LOW wire color
    pub wire_low: Color32,
}

pub const DARK: Palette = Palette {
    bg_main: BG_MAIN,
    bg_secondary: BG_SECONDARY,
    bg_panel: BG_PANEL,
    bg_elevated: BG_ELEVATED,
    bg_grid: BG_GRID,
    grid_minor: Color32::from_rgb(0x15, 0x1A, 0x21),
    grid_major: Color32::from_rgb(0x20, 0x27, 0x31),
    border: BORDER_PRIMARY,
    border_weak: BORDER_SECONDARY,
    border_primary: BORDER_PRIMARY,
    border_secondary: BORDER_SECONDARY,
    text: TEXT_PRIMARY,
    text_dim: TEXT_SECONDARY,
    text_primary: TEXT_PRIMARY,
    text_secondary: TEXT_SECONDARY,
    text_muted: TEXT_MUTED,
    selection_fill: Color32::from_rgba_premultiplied(0x0E, 0x2F, 0x3E, 0x26),
    cyan: CYAN,
    green: GREEN,
    orange: ORANGE,
    red: RED,
    purple: PURPLE,
    component_fill: Color32::from_rgb(0x12, 0x16, 0x1C),
    component_stroke: Color32::from_rgb(0xB4, 0xBF, 0xCC),
    wire_low: Color32::from_rgb(0x4E, 0x5C, 0x6C),
};

pub const LIGHT: Palette = Palette {
    bg_main: Color32::from_rgb(0xE6, 0xE9, 0xED),
    bg_secondary: Color32::from_rgb(0xDD, 0xE1, 0xE6),
    bg_panel: Color32::from_rgb(0xEE, 0xF0, 0xF3),
    bg_elevated: Color32::from_rgb(0xF6, 0xF7, 0xF9),
    bg_grid: Color32::from_rgb(0xF4, 0xF6, 0xF8),
    grid_minor: Color32::from_rgb(0xE3, 0xE7, 0xEC),
    grid_major: Color32::from_rgb(0xCB, 0xD2, 0xDB),
    border: Color32::from_rgb(0x9A, 0xA4, 0xB1),
    border_weak: Color32::from_rgb(0xC3, 0xCA, 0xD3),
    border_primary: Color32::from_rgb(0x9A, 0xA4, 0xB1),
    border_secondary: Color32::from_rgb(0xC3, 0xCA, 0xD3),
    text: Color32::from_rgb(0x11, 0x16, 0x1D),
    text_dim: Color32::from_rgb(0x3E, 0x48, 0x55),
    text_primary: Color32::from_rgb(0x11, 0x16, 0x1D),
    text_secondary: Color32::from_rgb(0x3E, 0x48, 0x55),
    text_muted: Color32::from_rgb(0x6B, 0x75, 0x82),
    selection_fill: Color32::from_rgba_premultiplied(0x02, 0x20, 0x30, 0x26),
    cyan: Color32::from_rgb(0x02, 0x84, 0xC7),
    green: Color32::from_rgb(0x15, 0x80, 0x3D),
    orange: Color32::from_rgb(0xB4, 0x53, 0x09),
    red: Color32::from_rgb(0xDC, 0x26, 0x26),
    purple: Color32::from_rgb(0x6D, 0x28, 0xD9),
    component_fill: Color32::from_rgb(0xFF, 0xFF, 0xFF),
    component_stroke: Color32::from_rgb(0x1A, 0x20, 0x29),
    wire_low: Color32::from_rgb(0x33, 0x3D, 0x4A),
};

#[inline]
pub fn palette(dark: bool) -> Palette {
    if dark {
        DARK
    } else {
        LIGHT
    }
}

/// Returns `c` with a new alpha (premultiplication handled by egui).
#[inline]
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

// ───────────────────────────── Style application ─────────────────────────────

/// Applies the neo-brutalist `Visuals` + `Style` to the egui context.
/// Call once at startup and whenever the dark/light mode changes.
pub fn apply(ctx: &egui::Context, dark: bool) {
    let p = palette(dark);
    let mut visuals = if dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    let r0 = Rounding::same(RADIUS);
    let r1 = Rounding::same(RADIUS_SMALL);

    visuals.override_text_color = Some(p.text);
    visuals.panel_fill = p.bg_panel;
    visuals.window_fill = p.bg_secondary;
    visuals.extreme_bg_color = p.bg_main;
    visuals.faint_bg_color = p.bg_elevated;
    visuals.code_bg_color = p.bg_main;
    visuals.hyperlink_color = p.cyan;
    visuals.warn_fg_color = p.orange;
    visuals.error_fg_color = p.red;
    visuals.window_rounding = r0;
    visuals.menu_rounding = r0;
    visuals.window_shadow = Shadow::NONE;
    visuals.popup_shadow = Shadow::NONE;
    visuals.window_stroke = Stroke::new(1.0, p.border);
    visuals.resize_corner_size = 8.0;
    visuals.clip_rect_margin = 2.0;
    visuals.striped = true;
    visuals.slider_trailing_fill = true;
    visuals.selection = Selection {
        bg_fill: alpha(p.cyan, if dark { 70 } else { 60 }),
        stroke: Stroke::new(1.0, p.cyan),
    };

    let w = &mut visuals.widgets;
    // Labels, separators, frames
    w.noninteractive.bg_fill = p.bg_panel;
    w.noninteractive.weak_bg_fill = p.bg_panel;
    w.noninteractive.bg_stroke = Stroke::new(1.0, p.border_weak);
    w.noninteractive.fg_stroke = Stroke::new(1.0, p.text_dim);
    w.noninteractive.rounding = r0;
    w.noninteractive.expansion = 0.0;
    // Idle buttons
    w.inactive.bg_fill = p.bg_elevated;
    w.inactive.weak_bg_fill = p.bg_elevated;
    w.inactive.bg_stroke = Stroke::new(1.0, p.border);
    w.inactive.fg_stroke = Stroke::new(1.0, p.text);
    w.inactive.rounding = r1;
    w.inactive.expansion = 0.0;
    // Hover: contrast bump, cyan outline
    let hover_bg = if dark {
        Color32::from_rgb(0x24, 0x2C, 0x37)
    } else {
        Color32::from_rgb(0xFF, 0xFF, 0xFF)
    };
    w.hovered.bg_fill = hover_bg;
    w.hovered.weak_bg_fill = hover_bg;
    w.hovered.bg_stroke = Stroke::new(1.0, alpha(p.cyan, 200));
    w.hovered.fg_stroke = Stroke::new(1.5, p.text);
    w.hovered.rounding = r1;
    w.hovered.expansion = 0.0;
    // Pressed
    w.active.bg_fill = alpha(p.cyan, 60);
    w.active.weak_bg_fill = alpha(p.cyan, 60);
    w.active.bg_stroke = Stroke::new(2.0, p.cyan);
    w.active.fg_stroke = Stroke::new(2.0, p.text);
    w.active.rounding = r1;
    w.active.expansion = 0.0;
    // Open combo/menus
    w.open.bg_fill = p.bg_main;
    w.open.weak_bg_fill = p.bg_main;
    w.open.bg_stroke = Stroke::new(1.0, p.cyan);
    w.open.fg_stroke = Stroke::new(1.0, p.text);
    w.open.rounding = r0;
    w.open.expansion = 0.0;

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = Vec2::new(4.0, 3.0);
    style.spacing.button_padding = Vec2::new(5.0, 2.5);
    style.spacing.window_margin = Margin::same(8.0);
    style.spacing.menu_margin = Margin::same(4.0);
    style.spacing.interact_size = Vec2::new(32.0, 22.0);
    style.spacing.icon_width = 12.0;
    style.spacing.icon_width_inner = 6.0;
    style.spacing.slider_width = 90.0;
    style.text_styles = [
        (TextStyle::Heading, FontId::new(14.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(12.0, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(10.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
    ]
    .into();
    ctx.set_style(style);
}

/// Installs technical fonts if they are present on the system — no bundled assets or new
/// dependencies. Preference: Inter / Roboto (UI) and JetBrains Mono / IBM Plex Mono (data).
/// egui's built-in fonts are kept as fallbacks so icons/emoji keep rendering.
pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        dirs.push(std::path::Path::new(&windir).join("Fonts"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(std::path::Path::new(&local).join("Microsoft").join("Windows").join("Fonts"));
    }
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(std::path::Path::new(&home).join(".fonts"));
        dirs.push(std::path::Path::new(&home).join(".local/share/fonts"));
        dirs.push(std::path::Path::new(&home).join("Library/Fonts"));
    }
    dirs.push("/usr/share/fonts/truetype".into());
    dirs.push("/Library/Fonts".into());

    let find = |names: &[&str]| -> Option<Vec<u8>> {
        for d in &dirs {
            for n in names {
                let p = d.join(n);
                if let Ok(bytes) = std::fs::read(&p) {
                    return Some(bytes);
                }
            }
        }
        None
    };

    let ui_font = find(&[
        "Inter-Regular.ttf",
        "Inter-Regular.otf",
        "Inter.ttf",
        "Roboto-Regular.ttf",
        "Roboto.ttf",
        "segoeui.ttf",
    ]);
    let mono_font = find(&[
        "JetBrainsMono-Regular.ttf",
        "JetBrainsMono[wght].ttf",
        "IBMPlexMono-Regular.ttf",
        "IBMPlexMono-Regular.otf",
        "CascadiaMono.ttf",
        "consola.ttf",
    ]);

    if let Some(bytes) = ui_font {
        fonts.font_data.insert("tech_ui".into(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "tech_ui".into());
    }
    if let Some(bytes) = mono_font {
        fonts.font_data.insert("tech_mono".into(), FontData::from_owned(bytes));
        fonts
            .families
            .entry(FontFamily::Monospace)
            .or_default()
            .insert(0, "tech_mono".into());
    }
    ctx.set_fonts(fonts);
}

// ───────────────────────────── Reusable widgets ─────────────────────────────

/// Compact panel section header: solid accent bar + UPPERCASE monospace title + hairline rule.
pub fn section_header(ui: &mut Ui, title: &str, accent: Color32) {
    let p = palette(ui.visuals().dark_mode);
    let h = 18.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), h), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, RADIUS, p.bg_secondary);
    painter.rect_filled(
        Rect::from_min_size(rect.min, Vec2::new(3.0, h)),
        RADIUS,
        accent,
    );
    painter.text(
        Pos2::new(rect.left() + 9.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        title.to_uppercase(),
        FontId::new(10.5, FontFamily::Monospace),
        p.text,
    );
    painter.line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0, p.border),
    );
}

/// Industrial toolbar toggle/button. `active` paints a solid accent underline + tinted fill.
pub fn tool_button(ui: &mut Ui, label: &str, active: bool, accent: Color32) -> Response {
    let p = palette(ui.visuals().dark_mode);
    let text = RichText::new(label.to_uppercase())
        .font(FontId::new(10.5, FontFamily::Monospace))
        .size(11.0)
        .color(if active { accent } else { p.text });
    let mut btn = egui::Button::new(text)
        .min_size(Vec2::new(0.0, 22.0))
        .rounding(Rounding::same(RADIUS_SMALL));
    if active {
        btn = btn
            .fill(alpha(accent, 34))
            .stroke(Stroke::new(1.0, accent));
    }
    let resp = ui.add(btn);
    if active {
        let r = resp.rect;
        ui.painter().line_segment(
            [
                Pos2::new(r.left() + 1.0, r.bottom() - 1.0),
                Pos2::new(r.right() - 1.0, r.bottom() - 1.0),
            ],
            Stroke::new(2.0, accent),
        );
    }
    resp
}

/// Primary action button: solid accent fill, dark text.
pub fn primary_button(ui: &mut Ui, label: &str, accent: Color32) -> Response {
    let text = RichText::new(label.to_uppercase())
        .font(FontId::new(10.5, FontFamily::Monospace))
        .size(11.0)
        .strong()
        .color(BG_MAIN);
    ui.add(
        egui::Button::new(text)
            .min_size(Vec2::new(0.0, 22.0))
            .fill(accent)
            .stroke(Stroke::new(1.0, accent))
            .rounding(Rounding::same(RADIUS_SMALL)),
    )
}

/// Thin vertical group separator for toolbars.
pub fn group_separator(ui: &mut Ui) {
    let p = palette(ui.visuals().dark_mode);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 22.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(rect.center().x, rect.top() + 2.0),
            Pos2::new(rect.center().x, rect.bottom() - 2.0),
        ],
        Stroke::new(1.0, p.border),
    );
}

/// Small monospace technical label ("KEY  VALUE" readouts, status fields).
#[allow(dead_code)]
pub fn mono_label(text: impl Into<String>, color: Color32) -> RichText {
    RichText::new(text.into()).monospace().size(10.5).color(color)
}

/// Solid status pill (e.g. `● RUN` / `■ PAUSE`) for the status bar.
pub fn status_pill(ui: &mut Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(
        text.to_string(),
        FontId::new(10.0, FontFamily::Monospace),
        BG_MAIN,
    );
    let size = galley.size() + Vec2::new(10.0, 4.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, RADIUS, color);
    ui.painter().galley(
        Pos2::new(rect.left() + 5.0, rect.top() + 2.0),
        galley,
        BG_MAIN,
    );
}

/// Standard frame for side/bottom panels: solid fill + 1px top/side borders, no shadows.
pub fn panel_frame(dark: bool) -> egui::Frame {
    let p = palette(dark);
    egui::Frame::none()
        .fill(p.bg_panel)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(6.0))
}

/// Frame for the top toolbar strip.
pub fn toolbar_frame(dark: bool) -> egui::Frame {
    let p = palette(dark);
    egui::Frame::none()
        .fill(p.bg_secondary)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::symmetric(6.0, 3.0))
}

/// Frame for floating windows (compact header, hard border, no shadow).
pub fn window_frame(dark: bool) -> egui::Frame {
    let p = palette(dark);
    egui::Frame::none()
        .fill(p.bg_secondary)
        .stroke(Stroke::new(1.0, p.border))
        .inner_margin(Margin::same(8.0))
}

/// HUD-style corner brackets around a rectangle (used for canvas selection).
pub fn draw_corner_brackets(painter: &egui::Painter, r: Rect, len: f32, stroke: Stroke) {
    let l = len.min(r.width() * 0.4).min(r.height() * 0.4);
    for (c, dx, dy) in [
        (r.left_top(), 1.0, 1.0),
        (r.right_top(), -1.0, 1.0),
        (r.left_bottom(), 1.0, -1.0),
        (r.right_bottom(), -1.0, -1.0),
    ] {
        painter.line_segment([c, Pos2::new(c.x + dx * l, c.y)], stroke);
        painter.line_segment([c, Pos2::new(c.x, c.y + dy * l)], stroke);
    }
}
