//! Central Circuit Canvas View & Interaction Controller (`canvas_view.rs`).
//!
//! Handles:
//! - Multi-page tab bar
//! - Viewport pan, zoom, and grid rendering
//! - Wire rendering, solder dots, and interactive segment dragging
//! - Gate rendering, pin hit-testing, toggle switches, and 4x4 hex keypad clicks
//! - Pinned K-Map and Boolean Function card dragging
//! - Marquee area selection box and multi-component lockstep group dragging

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};

use crate::canvas_renderer::{
    dist_to_segment, draw_gate_on_canvas, hit_test_hex_keypad_button,
};
use crate::kmap::{draw_kmap_function_on_canvas, draw_kmap_on_canvas};
use crate::mechanics::{
    LogicState, BUS_WIRE_WIDTH, SINGLE_WIRE_WIDTH, SOLDER_DOT_ANGLE_STEP_DEG, WIRE_CONN_RADIUS,
};
use crate::simulator::{CircuitPage, CircuitSimulator, GateInstance, PinRef};
use crate::LogicCraftApp;

impl LogicCraftApp {
    pub fn world_to_screen(rect: Rect, page: &CircuitPage, world: [f32; 2]) -> Pos2 {
        let center = rect.center();
        let px_per_world = 1.0 / page.zoom_world_per_px.max(0.01);
        Pos2::new(
            center.x + (world[0] - page.pan_world[0]) * px_per_world,
            center.y - (world[1] - page.pan_world[1]) * px_per_world,
        )
    }

    pub fn screen_to_world(rect: Rect, page: &CircuitPage, screen: Pos2) -> [f32; 2] {
        let center = rect.center();
        let world_per_px = page.zoom_world_per_px.max(0.01);
        [
            page.pan_world[0] + (screen.x - center.x) * world_per_px,
            page.pan_world[1] - (screen.y - center.y) * world_per_px,
        ]
    }

    pub fn draw_central_canvas(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);

        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(p.bg_main)
                    .inner_margin(egui::Margin::symmetric(0.0, 0.0)),
            )
            .show(ctx, |ui| {
                // Industrial Page Tab Bar
                let mut close_idx: Option<usize> = None;
                let mut rename_to: Option<(usize, String)> = None;
                let mut cancel_editing = false;
                let mut start_editing: Option<usize> = None;
                let mut select_page: Option<usize> = None;

                egui::Frame::none()
                    .fill(p.bg_secondary)
                    .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                    .stroke(Stroke::new(1.0, p.border_secondary))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 4.0;
                            ui.spacing_mut().interact_size.y = 22.0;

                                let pages_count = self.sim.pages.len();
                                for idx in 0..pages_count {
                                    let is_cur = idx == self.sim.current_page;
                                    let name = self.sim.pages[idx].name.clone();
                                    let is_editing = self.editing_page_name == Some(idx);
                                    let can_close = pages_count > 1;
                                    let close_w = if can_close { 20.0 } else { 0.0 };
                                    let tab_h = 22.0;

                                    let font = FontId::new(10.5, FontFamily::Monospace);
                                    let title_text = format!("PG.{:02} // {}", idx + 1, name.to_uppercase());
                                    let title_w = if is_editing {
                                        150.0
                                    } else {
                                        let galley = ui.painter().layout_no_wrap(
                                            title_text.clone(),
                                            font.clone(),
                                            p.text,
                                        );
                                        galley.size().x + 16.0
                                    };
                                    let tab_w = close_w + title_w;

                                    let (tab_rect, tab_resp) = ui.allocate_exact_size(
                                        Vec2::new(tab_w, tab_h),
                                        Sense::click(),
                                    );

                                    let close_rect = if can_close {
                                        Rect::from_min_size(tab_rect.left_top(), Vec2::new(close_w, tab_h))
                                    } else {
                                        Rect::NOTHING
                                    };
                                    let title_rect = Rect::from_min_size(
                                        Pos2::new(tab_rect.left() + close_w, tab_rect.top()),
                                        Vec2::new(title_w, tab_h),
                                    );

                                    let hover_pos = ui.input(|i| i.pointer.hover_pos());
                                    let tab_hovered = tab_resp.hovered();
                                    let close_hovered = can_close
                                        && hover_pos.map_or(false, |pos| close_rect.contains(pos));
                                    let title_hovered =
                                        hover_pos.map_or(false, |pos| title_rect.contains(pos));

                                    if close_hovered {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                        if ui.input(|i| i.pointer.primary_clicked()) {
                                            close_idx = Some(idx);
                                        }
                                    } else if title_hovered && !is_editing {
                                        if tab_resp.double_clicked() {
                                            start_editing = Some(idx);
                                        } else if tab_resp.clicked() {
                                            select_page = Some(idx);
                                        }
                                    }

                                    let painter = ui.painter();

                                    // Tab container background fill
                                    let bg_col = if is_cur {
                                        p.bg_elevated
                                    } else if tab_hovered {
                                        p.bg_secondary
                                    } else {
                                        p.bg_panel
                                    };
                                    painter.rect_filled(tab_rect, 0.0, bg_col);

                                    // Tab container outer border
                                    let border_col = if is_cur {
                                        p.cyan
                                    } else if tab_hovered {
                                        p.border_primary
                                    } else {
                                        p.border_secondary
                                    };
                                    painter.rect_stroke(tab_rect, 0.0, Stroke::new(1.0, border_col));

                                    // Bottom accent line for active tab
                                    if is_cur {
                                        painter.line_segment(
                                            [
                                                Pos2::new(tab_rect.left(), tab_rect.bottom() - 1.0),
                                                Pos2::new(tab_rect.right(), tab_rect.bottom() - 1.0),
                                            ],
                                            Stroke::new(2.0, p.cyan),
                                        );
                                    }

                                    // Close button on the left, inside same box, only right border, red when hovered
                                    if can_close {
                                        if close_hovered {
                                            painter.rect_filled(
                                                close_rect,
                                                0.0,
                                                Color32::from_rgba_unmultiplied(239, 68, 68, 48),
                                            );
                                            painter.text(
                                                close_rect.center(),
                                                Align2::CENTER_CENTER,
                                                "×",
                                                FontId::new(12.5, FontFamily::Monospace),
                                                p.red,
                                            );
                                        } else {
                                            painter.text(
                                                close_rect.center(),
                                                Align2::CENTER_CENTER,
                                                "×",
                                                FontId::new(12.5, FontFamily::Monospace),
                                                if is_cur { p.text_secondary } else { p.text_dim },
                                            );
                                        }

                                        // Right border separating the close button from the title
                                        painter.line_segment(
                                            [
                                                Pos2::new(close_rect.right(), close_rect.top()),
                                                Pos2::new(close_rect.right(), close_rect.bottom()),
                                            ],
                                            Stroke::new(1.0, border_col),
                                        );
                                    }

                                    // Page Name / Inline Rename
                                    if is_editing {
                                        ui.allocate_ui_at_rect(title_rect, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.spacing_mut().item_spacing.x = 2.0;
                                                ui.label(
                                                    RichText::new(format!("PG.{:02} //", idx + 1))
                                                        .font(FontId::new(10.0, FontFamily::Monospace))
                                                        .color(p.cyan),
                                                );
                                                let te = egui::TextEdit::singleline(
                                                    &mut self.page_name_draft,
                                                )
                                                .desired_width(70.0)
                                                .font(FontId::new(10.5, FontFamily::Monospace));
                                                let edit_resp = ui.add(te);
                                                if !edit_resp.has_focus() && !edit_resp.lost_focus() {
                                                    edit_resp.request_focus();
                                                }
                                                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                                let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                                if enter || edit_resp.lost_focus() {
                                                    let trimmed = self.page_name_draft.trim().to_string();
                                                    if !trimmed.is_empty() {
                                                        rename_to = Some((idx, trimmed));
                                                    } else {
                                                        cancel_editing = true;
                                                    }
                                                } else if escape {
                                                    cancel_editing = true;
                                                }
                                            });
                                        });
                                    } else {
                                        let text_col = if is_cur { p.cyan } else { p.text };
                                        painter.text(
                                            Pos2::new(title_rect.left() + 8.0, title_rect.center().y),
                                            Align2::LEFT_CENTER,
                                            title_text,
                                            font,
                                            text_col,
                                        );
                                    }
                                }

                                crate::theme::group_separator(ui);
                                if crate::theme::tool_button(ui, tr.canvas_add_page, false, p.cyan)
                                    .clicked()
                                {
                                    self.add_new_tab();
                                }
                            });
                    });
                if let Some((idx, new_name)) = rename_to {
                    self.save_undo_snapshot();
                    self.sim.pages[idx].name = new_name;
                    self.editing_page_name = None;
                }
                if cancel_editing {
                    self.editing_page_name = None;
                }
                if let Some(idx) = start_editing {
                    self.editing_page_name = Some(idx);
                    self.page_name_draft = self.sim.pages[idx].name.clone();
                }
                if let Some(idx) = select_page {
                    self.sim.current_page = idx;
                }
                if let Some(idx) = close_idx {
                    self.close_tab(idx);
                }

                // Main Circuit Canvas
                let avail = ui.available_size();
                let (canvas_rect, resp) = ui.allocate_exact_size(avail, Sense::click_and_drag());
                self.canvas_size = canvas_rect.size();
                let painter = ui.painter_at(canvas_rect);

                painter.rect_filled(canvas_rect, 0.0, p.bg_grid);
                painter.rect_stroke(canvas_rect, 0.0, Stroke::new(1.0, p.border_secondary));

                let page_idx = self.sim.current_page;

                // Mouse wheel zoom
                if resp.hovered() {
                    let scroll = ui.input(|i| i.raw_scroll_delta.y);
                    if scroll.abs() > 0.1 {
                        let factor = if scroll > 0.0 { 0.9 } else { 1.11 };
                        let page = &mut self.sim.pages[page_idx];
                        page.zoom_world_per_px = (page.zoom_world_per_px * factor).clamp(0.02, 0.35);
                    }
                }

                // Middle-mouse or right-mouse drag pan
                if resp.dragged_by(egui::PointerButton::Middle)
                    || resp.dragged_by(egui::PointerButton::Secondary)
                {
                    let delta = resp.drag_delta();
                    let page = &mut self.sim.pages[page_idx];
                    let scale = page.zoom_world_per_px;
                    page.pan_world[0] -= delta.x * scale;
                    page.pan_world[1] += delta.y * scale;
                }

                // Draw Technical Engineering Gridlines
                if self.show_gridlines {
                    let page = &self.sim.pages[page_idx];
                    let top_left_w = Self::screen_to_world(canvas_rect, page, canvas_rect.left_top());
                    let bot_right_w =
                        Self::screen_to_world(canvas_rect, page, canvas_rect.right_bottom());
                    let step_w = if page.zoom_world_per_px > 0.12 {
                        2.0
                    } else {
                        1.0
                    };
                    let start_x = (top_left_w[0] / step_w).floor() as i32;
                    let end_x = (bot_right_w[0] / step_w).ceil() as i32;
                    let start_y = (bot_right_w[1] / step_w).floor() as i32;
                    let end_y = (top_left_w[1] / step_w).ceil() as i32;

                    for gx in start_x..=end_x {
                        let wx = gx as f32 * step_w;
                        let sx = Self::world_to_screen(canvas_rect, page, [wx, 0.0]).x;
                        let is_major = gx % 5 == 0;
                        painter.line_segment(
                            [
                                Pos2::new(sx, canvas_rect.top()),
                                Pos2::new(sx, canvas_rect.bottom()),
                            ],
                            Stroke::new(
                                if is_major { 1.1_f32 } else { 0.8_f32 },
                                if is_major { p.grid_major } else { p.grid_minor },
                            ),
                        );
                    }
                    for gy in start_y..=end_y {
                        let wy = gy as f32 * step_w;
                        let sy = Self::world_to_screen(canvas_rect, page, [0.0, wy]).y;
                        let is_major = gy % 5 == 0;
                        painter.line_segment(
                            [
                                Pos2::new(canvas_rect.left(), sy),
                                Pos2::new(canvas_rect.right(), sy),
                            ],
                            Stroke::new(
                                if is_major { 1.1_f32 } else { 0.8_f32 },
                                if is_major { p.grid_major } else { p.grid_minor },
                            ),
                        );
                    }
                }

                let pointer_pos = resp
                    .interact_pointer_pos()
                    .or_else(|| ui.input(|i| i.pointer.hover_pos()));
                let press_pos = ui.input(|i| i.pointer.press_origin()).or(pointer_pos);

                // Draw Wires & detect hovered/draggable wire segments
                let mut hovered_wire_seg: Option<(u32, bool, Pos2)> = None;
                let mut drag_start_wire_seg: Option<(u32, bool, Pos2)> = None;
                {
                    let page = &self.sim.pages[page_idx];
                    let px_per_world = 1.0 / page.zoom_world_per_px.max(0.01);
                    let mut best_hover_score = 8.5_f32;
                    let mut best_press_score = 9.5_f32;
                    for w in &page.wires {
                        let col = if w.selected {
                            p.cyan
                        } else {
                            w.state.wire_color(self.dark_mode)
                        };
                        let width = if w.bus_bits > 1 {
                            BUS_WIRE_WIDTH
                        } else if w.selected {
                            SINGLE_WIRE_WIDTH + 1.8
                        } else {
                            SINGLE_WIRE_WIDTH + 1.0
                        };
                        let num_segs = w.points.len().saturating_sub(1);
                        for (seg_idx, seg) in w.points.windows(2).enumerate() {
                            let p0 = Self::world_to_screen(canvas_rect, page, seg[0]);
                            let p1 = Self::world_to_screen(canvas_rect, page, seg[1]);
                            painter.line_segment([p0, p1], Stroke::new(width, col));

                            let seg_len = p0.distance(p1);
                            if seg_len < 2.0 {
                                continue;
                            }
                            let is_vert = (p0.x - p1.x).abs() <= (p0.y - p1.y).abs();
                            let mid = Pos2::new((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
                            // Prefer middle trunk segment or vertical segment near corners
                            let priority_bonus = if (num_segs == 3 && seg_idx == 1) || is_vert {
                                1.5_f32
                            } else {
                                0.0_f32
                            };

                            if let Some(mp) = pointer_pos {
                                let dist = dist_to_segment(mp, p0, p1);
                                if dist <= 8.5 && (dist - priority_bonus) < best_hover_score {
                                    best_hover_score = dist - priority_bonus;
                                    hovered_wire_seg = Some((w.id, is_vert, mid));
                                }
                            }
                            if let Some(pp) = press_pos {
                                let dist = dist_to_segment(pp, p0, p1);
                                if dist <= 9.5 && (dist - priority_bonus) < best_press_score {
                                    best_press_score = dist - priority_bonus;
                                    drag_start_wire_seg = Some((w.id, is_vert, mid));
                                }
                            }
                        }
                        if self.show_wire_conn {
                            let dot_r = (WIRE_CONN_RADIUS * px_per_world).clamp(2.5, 6.0);
                            for &dot in &w.solder_dots {
                                let dp = Self::world_to_screen(canvas_rect, page, dot);
                                let mut poly = Vec::new();
                                let mut deg = 0u32;
                                while deg < 360 {
                                    let rad = (deg as f32).to_radians();
                                    poly.push(Pos2::new(
                                        dp.x + dot_r * rad.cos(),
                                        dp.y + dot_r * rad.sin(),
                                    ));
                                    deg += SOLDER_DOT_ANGLE_STEP_DEG;
                                }
                                painter.add(egui::Shape::convex_polygon(poly, col, Stroke::NONE));
                            }
                        }
                    }
                }

            // Draw Gates & Interactive Pins
            let mut clicked_pin: Option<PinRef> = None;
            let mut hovered_pin_info: Option<(PinRef, Pos2, LogicState)> = None;
            let mut clicked_gate_id: Option<u32> = None;
            let mut toggled_gate_action: Option<(u32, u16)> = None;
            let mut group_sel_rect: Option<Rect> = None;
            let mut selected_gate_ids: Vec<u32> = Vec::new();

            {
                let page = &self.sim.pages[page_idx];
                let px_per_world = 1.0 / page.zoom_world_per_px.max(0.01);
                for g in &page.gates {
                    let Some(def) = self.lib.get(&g.type_name) else {
                        continue;
                    };
                    let center_s = Self::world_to_screen(canvas_rect, page, g.pos);
                    let rot_90 = ((g.angle_deg / 90.0).round() as i32).rem_euclid(2) == 1;
                    let (hw, hh) = if rot_90 {
                        (def.half_size[1], def.half_size[0])
                    } else {
                        (def.half_size[0], def.half_size[1])
                    };
                    let half_px = Vec2::new(hw * px_per_world, hh * px_per_world);
                    let gate_rect = Rect::from_center_size(center_s, half_px * 2.0);

                    if g.selected {
                        selected_gate_ids.push(g.id);
                        group_sel_rect = Some(match group_sel_rect {
                            Some(r) => r.union(gate_rect),
                            None => gate_rect,
                        });
                    }

                    draw_gate_on_canvas(
                        &painter,
                        canvas_rect,
                        page,
                        g,
                        def,
                        self.dark_mode,
                        self.show_wire_conn,
                    );

                    // Check pin hover / click (only on click, never hijack dragging!)
                    for pin in def.inputs.iter().chain(def.outputs.iter()) {
                        let pw = g.pin_world_pos(pin.pos);
                        let ps = Self::world_to_screen(canvas_rect, page, pw);
                        let st = g
                            .pin_states
                            .get(&pin.name)
                            .copied()
                            .unwrap_or(LogicState::HiZ);
                        if let Some(mp) = pointer_pos {
                            if mp.distance(ps) <= 9.0 {
                                hovered_pin_info = Some((
                                    PinRef {
                                        gate_id: g.id,
                                        pin_name: pin.name.clone(),
                                    },
                                    ps,
                                    st,
                                ));
                                if resp.clicked() {
                                    clicked_pin = Some(PinRef {
                                        gate_id: g.id,
                                        pin_name: pin.name.clone(),
                                    });
                                }
                            }
                        }
                    }

                    // Check gate body click / interactive toggle / 4x4 hex keypad button click
                    if resp.clicked() {
                        if let Some(mp) = pointer_pos {
                            if gate_rect.contains(mp) && hovered_pin_info.is_none() {
                                clicked_gate_id = Some(g.id);
                                if g.type_name == "AA_TOGGLE" {
                                    let cur = g
                                        .lparams
                                        .get("OUTPUT_NUM")
                                        .and_then(|s| s.parse::<u16>().ok())
                                        .unwrap_or(0);
                                    toggled_gate_action = Some((g.id, cur ^ 1));
                                } else if g.type_name == "CC_PULSE" {
                                    toggled_gate_action = Some((g.id, 0xFFFF));
                                } else if g.type_name == "DD_KEYPAD_HEX" {
                                    if let Some(hex_key) =
                                        hit_test_hex_keypad_button(canvas_rect, page, g, def, mp)
                                    {
                                        toggled_gate_action = Some((g.id, hex_key));
                                    }
                                }
                            }
                        }
                    } else if resp.drag_started_by(egui::PointerButton::Primary) {
                        let hit_drag_gate = press_pos
                            .map_or(false, |pp| gate_rect.expand(2.0).contains(pp))
                            || pointer_pos.map_or(false, |mp| gate_rect.contains(mp));
                        if hit_drag_gate {
                            clicked_gate_id = Some(g.id);
                        }
                    }
                }
            }

            // When multiple components are selected, draw a group bounding box and allow dragging anywhere inside the group
            if selected_gate_ids.len() > 1 {
                if let Some(g_rect) = group_sel_rect {
                    let group_box = g_rect.expand(8.0);
                    painter.rect_stroke(
                        group_box,
                        0.0,
                        Stroke::new(
                            1.0,
                            Color32::from_rgba_unmultiplied(p.cyan.r(), p.cyan.g(), p.cyan.b(), 140),
                        ),
                    );
                    crate::theme::draw_corner_brackets(
                        &painter,
                        group_box,
                        10.0,
                        Stroke::new(2.0, p.cyan),
                    );
                    if resp.drag_started_by(egui::PointerButton::Primary)
                        && clicked_gate_id.is_none()
                    {
                        let inside_group = press_pos
                            .map_or(false, |pp| group_box.contains(pp))
                            || pointer_pos.map_or(false, |mp| group_box.contains(mp));
                        if inside_group {
                            clicked_gate_id = selected_gate_ids.first().copied();
                        }
                    }
                }
            }

            // Draw any pinned Truth Table / K-Map & Boolean Function cards on the canvas
            let mut hovered_kmap_idx: Option<usize> = None;
            let mut hovered_kmap_func_idx: Option<usize> = None;
            {
                let page = &self.sim.pages[page_idx];
                let px_per_world = 1.0 / page.zoom_world_per_px.max(0.01);
                for (ki, kmap) in self.sim.kmaps.iter().enumerate() {
                    if kmap.pinned_on_canvas {
                        let sp = Self::world_to_screen(canvas_rect, page, kmap.canvas_pos);
                        let k_rect = draw_kmap_on_canvas(
                            &painter,
                            sp,
                            px_per_world,
                            kmap,
                            self.dark_mode,
                            self.dragging_kmap == Some(ki),
                        );
                        if let Some(mp) = pointer_pos {
                            if k_rect.contains(mp) {
                                hovered_kmap_idx = Some(ki);
                            }
                        }
                    }
                    if kmap.pinned_on_canvas || kmap.func_pinned_on_canvas {
                        let fsp = Self::world_to_screen(canvas_rect, page, kmap.func_canvas_pos);
                        let f_rect = draw_kmap_function_on_canvas(
                            &painter,
                            fsp,
                            px_per_world,
                            kmap,
                            self.dark_mode,
                            self.dragging_kmap_func == Some(ki),
                        );
                        if let Some(mp) = pointer_pos {
                            if f_rect.contains(mp) {
                                hovered_kmap_func_idx = Some(ki);
                            }
                        }
                    }
                }
            }
            if hovered_kmap_idx.is_some()
                || hovered_kmap_func_idx.is_some()
                || self.dragging_kmap.is_some()
                || self.dragging_kmap_func.is_some()
            {
                ctx.set_cursor_icon(if self.dragging_kmap.is_some() || self.dragging_kmap_func.is_some() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                });
            }

            // Show wire drag handle when hovering a wire segment
            if hovered_pin_info.is_none()
                && clicked_gate_id.is_none()
                && hovered_kmap_idx.is_none()
                && hovered_kmap_func_idx.is_none()
            {
                if let Some((_, is_vert, mid)) = hovered_wire_seg {
                    let handle_r = Rect::from_center_size(mid, Vec2::splat(8.0));
                    painter.rect_filled(handle_r, 0.0, p.orange);
                    painter.rect_stroke(handle_r, 0.0, Stroke::new(1.0, p.bg_main));
                    ctx.set_cursor_icon(if is_vert {
                        egui::CursorIcon::ResizeHorizontal
                    } else {
                        egui::CursorIcon::ResizeVertical
                    });
                }
            }

            // Highlight hovered pin & active wiring preview line
            if let Some((ref pref, ps, st)) = hovered_pin_info {
                let pin_box = Rect::from_center_size(ps, Vec2::splat(12.0));
                painter.rect_stroke(pin_box, 0.0, Stroke::new(1.8_f32, p.cyan));
                let badge_txt = format!(" {} : {} ", pref.pin_name, st.label());
                let badge_rect = Rect::from_center_size(
                    Pos2::new(ps.x, ps.y - 18.0),
                    Vec2::new((badge_txt.len() as f32 * 6.8).max(76.0), 18.0),
                );
                painter.rect_filled(badge_rect, 0.0, p.bg_elevated);
                painter.rect_stroke(badge_rect, 0.0, Stroke::new(1.0, p.cyan));
                painter.text(
                    badge_rect.center(),
                    Align2::CENTER_CENTER,
                    badge_txt,
                    FontId::new(10.5, FontFamily::Monospace),
                    p.text_primary,
                );
            }

            if let Some(ref start_pin) = self.wiring_from {
                let page = &self.sim.pages[page_idx];
                if let Some(p0_w) = self.sim.pin_world_position(
                    &self.lib,
                    page_idx,
                    start_pin.gate_id,
                    &start_pin.pin_name,
                ) {
                    let p0_s = Self::world_to_screen(canvas_rect, page, p0_w);
                    if let Some(mp) = pointer_pos {
                        let mid_x = (p0_s.x + mp.x) * 0.5;
                        painter.line_segment(
                            [p0_s, Pos2::new(mid_x, p0_s.y)],
                            Stroke::new(2.0_f32, p.cyan),
                        );
                        painter.line_segment(
                            [Pos2::new(mid_x, p0_s.y), Pos2::new(mid_x, mp.y)],
                            Stroke::new(2.0_f32, p.cyan),
                        );
                        painter.line_segment(
                            [Pos2::new(mid_x, mp.y), mp],
                            Stroke::new(2.0_f32, p.cyan),
                        );
                    }
                }
            }

            if resp.clicked() || resp.drag_started() {
                ctx.memory_mut(|m| {
                    if let Some(focused_id) = m.focused() {
                        m.surrender_focus(focused_id);
                    }
                });
            }

            // Handle placing a new gate on canvas click (hold Ctrl to place multiple)
            let active_wire_seg = if resp.drag_started_by(egui::PointerButton::Primary) {
                drag_start_wire_seg.or(hovered_wire_seg)
            } else {
                hovered_wire_seg
            };

            if let Some(place_type) = self.placing_gate_type.clone() {
                let ctrl_held = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                if self.placing_multi_active && !ctrl_held {
                    self.placing_gate_type = None;
                    self.placing_multi_active = false;
                } else if ui.input(|i| i.pointer.button_clicked(egui::PointerButton::Secondary)) {
                    self.placing_gate_type = None;
                    self.placing_multi_active = false;
                } else if let Some(mp) = pointer_pos {
                    let page = &self.sim.pages[page_idx];
                    let wpos = Self::screen_to_world(canvas_rect, page, mp);
                    let snapped = [
                        CircuitSimulator::snap_to_grid(wpos[0]),
                        CircuitSimulator::snap_to_grid(wpos[1]),
                    ];
                    let sp = Self::world_to_screen(canvas_rect, page, snapped);
                    let target_r = Rect::from_center_size(sp, Vec2::splat(32.0));
                    painter.rect_filled(target_r, 0.0, p.selection_fill);
                    crate::theme::draw_corner_brackets(
                        &painter,
                        target_r,
                        8.0,
                        Stroke::new(1.8_f32, p.cyan),
                    );
                    if (resp.clicked() || (resp.drag_stopped() && resp.hovered()))
                        && !self.sim.locked
                    {
                        if let Some(def) = self.lib.get(&place_type).cloned() {
                            self.save_undo_snapshot();
                            let id = self.sim.next_gate_id;
                            self.sim.next_gate_id += 1;
                            let mut inst = GateInstance::new(id, &def, snapped);
                            if inst.type_name == "DA_FROM" {
                                let unique = self.sim.next_unique_from_name();
                                inst.lparams.insert("JUNCTION_ID".into(), unique);
                            } else if inst.type_name == "DE_TO" {
                                if let Some(last_from) =
                                    self.sim.from_connector_names().last().cloned()
                                {
                                    inst.lparams.insert("JUNCTION_ID".into(), last_from);
                                }
                            }
                            for g in &mut self.sim.pages[page_idx].gates {
                                g.selected = false;
                            }
                            for w in &mut self.sim.pages[page_idx].wires {
                                w.selected = false;
                            }
                            inst.selected = true;
                            self.sim.pages[page_idx].gates.push(inst);
                            if ctrl_held {
                                self.placing_gate_type = Some(place_type);
                                self.placing_multi_active = true;
                            } else {
                                self.placing_gate_type = None;
                                self.placing_multi_active = false;
                            }
                            self.status_message = format!(
                                "Placed {} (ID #{id}).",
                                self.locale.gate_caption(&def.caption)
                            );
                        }
                    }
                }
            } else if let Some(pin_ref) = clicked_pin {
                if !self.sim.locked {
                    if let Some(from_ref) = self.wiring_from.take() {
                        if from_ref != pin_ref {
                            self.save_undo_snapshot();
                            let wid = self.sim.connect_pins(
                                &self.lib,
                                page_idx,
                                from_ref.gate_id,
                                &from_ref.pin_name,
                                pin_ref.gate_id,
                                &pin_ref.pin_name,
                                &format!("{}_{}", from_ref.pin_name, pin_ref.pin_name),
                            );
                            self.status_message = format!("Connected Wire #{wid}.");
                        }
                    } else {
                        self.wiring_from = Some(pin_ref.clone());
                        self.status_message = format!(
                            "Wiring from Gate #{} pin '{}'...",
                            pin_ref.gate_id, pin_ref.pin_name
                        );
                    }
                }
            } else if let Some(ki) = hovered_kmap_func_idx {
                if resp.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(mp) = pointer_pos {
                        let page = &self.sim.pages[page_idx];
                        let wpos = Self::screen_to_world(canvas_rect, page, mp);
                        if let Some(km) = self.sim.kmaps.get(ki) {
                            self.dragging_kmap_func = Some(ki);
                            self.drag_offset_world = [
                                km.func_canvas_pos[0] - wpos[0],
                                km.func_canvas_pos[1] - wpos[1],
                            ];
                        }
                    }
                }
            } else if let Some(ki) = hovered_kmap_idx {
                if resp.drag_started_by(egui::PointerButton::Primary) {
                    if let Some(mp) = pointer_pos {
                        let page = &self.sim.pages[page_idx];
                        let wpos = Self::screen_to_world(canvas_rect, page, mp);
                        if let Some(km) = self.sim.kmaps.get(ki) {
                            self.dragging_kmap = Some(ki);
                            self.drag_offset_world = [
                                km.canvas_pos[0] - wpos[0],
                                km.canvas_pos[1] - wpos[1],
                            ];
                        }
                    }
                }
            } else if let Some((gid, val)) = toggled_gate_action {
                let page = &mut self.sim.pages[page_idx];
                for g in &mut page.gates {
                    g.selected = g.id == gid;
                    if g.id == gid {
                        if val == 0xFFFF {
                            let width = g
                                .lparams
                                .get("PULSE_WIDTH")
                                .and_then(|s| s.parse::<u32>().ok())
                                .unwrap_or(4);
                            g.pulse_remaining = width;
                        } else {
                            g.lparams.insert("OUTPUT_NUM".into(), val.to_string());
                            self.status_message =
                                format!("Set Component #{gid} output to 0x{val:X} ({val}).");
                        }
                    }
                }
                for w in &mut page.wires {
                    w.selected = false;
                }
                self.sim.step(&self.lib);
            } else if let Some(gid) = clicked_gate_id {
                let shift_held = ui.input(|i| i.modifiers.shift);
                let page = &mut self.sim.pages[page_idx];
                let already_selected = page
                    .gates
                    .iter()
                    .find(|g| g.id == gid)
                    .map(|g| g.selected)
                    .unwrap_or(false);

                if resp.drag_started_by(egui::PointerButton::Primary) && !self.sim.locked {
                    if !already_selected {
                        for g in &mut page.gates {
                            if shift_held {
                                if g.id == gid {
                                    g.selected = true;
                                }
                            } else {
                                g.selected = g.id == gid;
                            }
                        }
                        if !shift_held {
                            for w in &mut page.wires {
                                w.selected = false;
                            }
                        }
                    }
                    if let Some(mp) = press_pos.or(pointer_pos) {
                        let wpos = Self::screen_to_world(canvas_rect, page, mp);
                        self.dragging_gate_id = Some(gid);
                        self.drag_anchor_world = wpos;
                        self.drag_initial_positions = page
                            .gates
                            .iter()
                            .filter(|g| g.selected)
                            .map(|g| (g.id, g.pos))
                            .collect();
                        let sel_ids: Vec<u32> =
                            self.drag_initial_positions.iter().map(|&(id, _)| id).collect();
                        for w in &mut page.wires {
                            if !w.endpoints.is_empty()
                                && w.endpoints.iter().all(|ep| sel_ids.contains(&ep.gate_id))
                            {
                                w.selected = true;
                            }
                        }
                        self.drag_initial_wire_mids = page
                            .wires
                            .iter()
                            .filter(|w| {
                                !w.endpoints.is_empty()
                                    && w.endpoints.iter().all(|ep| sel_ids.contains(&ep.gate_id))
                            })
                            .map(|w| (w.id, w.custom_mid_x, w.custom_mid_y))
                            .collect();
                        self.drag_snapshot_saved = false;
                    }
                } else if resp.clicked() {
                    for g in &mut page.gates {
                        if shift_held {
                            if g.id == gid {
                                g.selected = !g.selected;
                            }
                        } else {
                            g.selected = g.id == gid;
                        }
                    }
                    if !shift_held {
                        for w in &mut page.wires {
                            w.selected = false;
                        }
                    }
                }
            } else if let Some((wid, is_vert, _)) = active_wire_seg {
                if resp.clicked() || resp.drag_started_by(egui::PointerButton::Primary) {
                    let page = &mut self.sim.pages[page_idx];
                    for g in &mut page.gates {
                        g.selected = false;
                    }
                    for w in &mut page.wires {
                        w.selected = w.id == wid;
                    }
                    if resp.drag_started_by(egui::PointerButton::Primary) && !self.sim.locked {
                        self.dragging_wire = Some((wid, is_vert));
                        let mut group = vec![wid];
                        if let Some(target_w) = page.wires.iter().find(|w| w.id == wid) {
                            let target_eps = target_w.endpoints.clone();
                            let target_coord = target_w.points.windows(2).find_map(|seg| {
                                let a = seg[0];
                                let b = seg[1];
                                let dx = (b[0] - a[0]).abs();
                                let dy = (b[1] - a[1]).abs();
                                if dx < 0.01 && dy < 0.01 {
                                    return None;
                                }
                                let sv = dx < dy;
                                if sv == is_vert {
                                    Some(if is_vert { a[0] } else { a[1] })
                                } else {
                                    None
                                }
                            });
                            if let Some(tc) = target_coord {
                                for w in &page.wires {
                                    if w.id != wid
                                        && w.endpoints.iter().any(|ep| target_eps.contains(ep))
                                    {
                                        let shares_trunk = w.points.windows(2).any(|seg| {
                                            let a = seg[0];
                                            let b = seg[1];
                                            let dx = (b[0] - a[0]).abs();
                                            let dy = (b[1] - a[1]).abs();
                                            if dx < 0.01 && dy < 0.01 {
                                                return false;
                                            }
                                            let sv = dx < dy;
                                            let c = if is_vert { a[0] } else { a[1] };
                                            sv == is_vert && (c - tc).abs() < 0.05
                                        });
                                        if shares_trunk {
                                            group.push(w.id);
                                        }
                                    }
                                }
                            }
                        }
                        for w in &mut page.wires {
                            if group.contains(&w.id) {
                                w.selected = true;
                            }
                        }
                        self.dragging_wire_group = group;
                        self.drag_snapshot_saved = false;
                    }
                }
            } else if resp.drag_started_by(egui::PointerButton::Primary)
                && self.placing_gate_type.is_none()
                && self.wiring_from.is_none()
            {
                let space_held = ui.input(|i| i.key_down(Key::Space));
                if !space_held {
                    self.selection_box_start = press_pos.or(pointer_pos);
                }
            } else if resp.clicked() {
                self.wiring_from = None;
                let page = &mut self.sim.pages[page_idx];
                for g in &mut page.gates {
                    g.selected = false;
                }
                for w in &mut page.wires {
                    w.selected = false;
                }
            }

            // Handle dragging K-Map, Function, multi-gate group, wire segment, or area selection box
            if let Some(ki) = self.dragging_kmap_func {
                if resp.dragged_by(egui::PointerButton::Primary) {
                    if let Some(mp) = pointer_pos {
                        let page = &self.sim.pages[page_idx];
                        let wpos = Self::screen_to_world(canvas_rect, page, mp);
                        if let Some(km) = self.sim.kmaps.get_mut(ki) {
                            km.func_canvas_pos = [
                                wpos[0] + self.drag_offset_world[0],
                                wpos[1] + self.drag_offset_world[1],
                            ];
                        }
                    }
                } else if resp.drag_stopped() {
                    self.dragging_kmap_func = None;
                }
            } else if let Some(ki) = self.dragging_kmap {
                if resp.dragged_by(egui::PointerButton::Primary) {
                    if let Some(mp) = pointer_pos {
                        let page = &self.sim.pages[page_idx];
                        let wpos = Self::screen_to_world(canvas_rect, page, mp);
                        if let Some(km) = self.sim.kmaps.get_mut(ki) {
                            km.canvas_pos = [
                                wpos[0] + self.drag_offset_world[0],
                                wpos[1] + self.drag_offset_world[1],
                            ];
                        }
                    }
                } else if resp.drag_stopped() {
                    self.dragging_kmap = None;
                }
            } else if self.dragging_gate_id.is_some() {
                if resp.dragged_by(egui::PointerButton::Primary) && !self.sim.locked {
                    if let Some(mp) = pointer_pos {
                        if !self.drag_snapshot_saved {
                            self.save_undo_snapshot();
                            self.drag_snapshot_saved = true;
                        }
                        let page = &mut self.sim.pages[page_idx];
                        let wpos = Self::screen_to_world(canvas_rect, page, mp);
                        let dx = CircuitSimulator::snap_to_grid(wpos[0] - self.drag_anchor_world[0]);
                        let dy = CircuitSimulator::snap_to_grid(wpos[1] - self.drag_anchor_world[1]);
                        for &(sel_id, init_pos) in &self.drag_initial_positions {
                            if let Some(g) = page.gates.iter_mut().find(|g| g.id == sel_id) {
                                g.pos = [init_pos[0] + dx, init_pos[1] + dy];
                            }
                        }
                        for &(wid, init_mx, init_my) in &self.drag_initial_wire_mids {
                            if let Some(w) = page.wires.iter_mut().find(|w| w.id == wid) {
                                w.custom_mid_x = init_mx.map(|mx| mx + dx);
                                w.custom_mid_y = init_my.map(|my| my + dy);
                            }
                        }
                        self.sim.rebuild_wire_geometry(&self.lib, page_idx);
                    }
                } else if resp.drag_stopped() {
                    self.dragging_gate_id = None;
                    self.drag_initial_positions.clear();
                    self.drag_initial_wire_mids.clear();
                }
            } else if let Some((wid, is_vert)) = self.dragging_wire {
                if resp.dragged_by(egui::PointerButton::Primary) && !self.sim.locked {
                    if let Some(mp) = pointer_pos {
                        if !self.drag_snapshot_saved {
                            self.save_undo_snapshot();
                            self.drag_snapshot_saved = true;
                        }
                        let page = &mut self.sim.pages[page_idx];
                        let wpos = Self::screen_to_world(canvas_rect, page, mp);
                        let target_ids = if self.dragging_wire_group.is_empty() {
                            vec![wid]
                        } else {
                            self.dragging_wire_group.clone()
                        };
                        let snapped_x = CircuitSimulator::snap_to_grid(wpos[0]);
                        let snapped_y = CircuitSimulator::snap_to_grid(wpos[1]);
                        for w in &mut page.wires {
                            if target_ids.contains(&w.id) {
                                if is_vert {
                                    w.custom_mid_x = Some(snapped_x);
                                    w.custom_mid_y = None;
                                } else {
                                    w.custom_mid_y = Some(snapped_y);
                                    w.custom_mid_x = None;
                                }
                            }
                        }
                        self.sim.rebuild_wire_geometry(&self.lib, page_idx);
                    }
                } else if resp.drag_stopped() {
                    self.dragging_wire = None;
                    self.dragging_wire_group.clear();
                }
            } else if let Some(start_s) = self.selection_box_start {
                if resp.dragged_by(egui::PointerButton::Primary) {
                    if let Some(mp) = pointer_pos {
                        let sel_rect = Rect::from_two_pos(start_s, mp);
                        let shift_held = ui.input(|i| i.modifiers.shift);
                        let page = &mut self.sim.pages[page_idx];
                        let zoom = page.zoom_world_per_px.max(0.01);
                        let px_per_world = 1.0 / zoom;
                        let pan = page.pan_world;

                        let mut sel_gate_ids = Vec::new();
                        for g in &mut page.gates {
                            let (hw, hh) = self
                                .lib
                                .get(&g.type_name)
                                .map(|d| (d.half_size[0], d.half_size[1]))
                                .unwrap_or((2.0, 2.0));
                            let rot_90 = ((g.angle_deg / 90.0).round() as i32).rem_euclid(2) == 1;
                            let (gw, gh) = if rot_90 { (hh, hw) } else { (hw, hh) };
                            let center_s = Pos2::new(
                                canvas_rect.center().x + (g.pos[0] - pan[0]) / zoom,
                                canvas_rect.center().y - (g.pos[1] - pan[1]) / zoom,
                            );
                            let g_rect = Rect::from_center_size(
                                center_s,
                                Vec2::new(gw * 2.0 * px_per_world, gh * 2.0 * px_per_world),
                            );
                            if sel_rect.intersects(g_rect) {
                                g.selected = true;
                            } else if !shift_held {
                                g.selected = false;
                            }
                            if g.selected {
                                sel_gate_ids.push(g.id);
                            }
                        }
                        for w in &mut page.wires {
                            let both_in = !w.endpoints.is_empty()
                                && w.endpoints.iter().all(|ep| sel_gate_ids.contains(&ep.gate_id));
                            if both_in {
                                w.selected = true;
                            } else if !shift_held {
                                w.selected = false;
                            }
                        }

                        // Draw the tactical marquee area selection box
                        painter.rect_filled(sel_rect, 0.0, p.selection_fill);
                        painter.rect_stroke(sel_rect, 0.0, Stroke::new(1.2_f32, p.cyan));
                        crate::theme::draw_corner_brackets(
                            &painter,
                            sel_rect,
                            8.0,
                            Stroke::new(2.0_f32, p.cyan),
                        );
                        if sel_rect.width() > 24.0 || sel_rect.height() > 24.0 {
                            let tag_txt = format!(" {} [{}] ", tr.canvas_area_select.to_uppercase(), sel_gate_ids.len());
                            let tag_rect = Rect::from_min_size(
                                Pos2::new(sel_rect.left(), (sel_rect.top() - 18.0).max(canvas_rect.top() + 2.0)),
                                Vec2::new((tag_txt.len() as f32 * 6.8).max(95.0), 17.0),
                            );
                            painter.rect_filled(tag_rect, 0.0, p.bg_elevated);
                            painter.rect_stroke(tag_rect, 0.0, Stroke::new(1.0, p.cyan));
                            painter.text(
                                tag_rect.center(),
                                Align2::CENTER_CENTER,
                                tag_txt,
                                FontId::new(10.5, FontFamily::Monospace),
                                p.cyan,
                            );
                        }
                    }
                } else if resp.drag_stopped() {
                    let count = self.sim.pages[page_idx]
                        .gates
                        .iter()
                        .filter(|g| g.selected)
                        .count();
                    if count > 0 {
                        self.status_message =
                            format!("{count} {}", tr.inspector_group_selected);
                    }
                    self.selection_box_start = None;
                }
            } else if resp.dragged_by(egui::PointerButton::Primary)
                && self.placing_gate_type.is_none()
                && self.wiring_from.is_none()
            {
                let delta = resp.drag_delta();
                let page = &mut self.sim.pages[page_idx];
                let scale = page.zoom_world_per_px;
                page.pan_world[0] -= delta.x * scale;
                page.pan_world[1] += delta.y * scale;
            }
        });
    }
}
