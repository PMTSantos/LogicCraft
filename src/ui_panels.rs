//! Top Menu Bar, Toolbar, Left Sidebar, Inspector, Oscilloscope & Status Bar (`ui_panels.rs`).

use std::collections::VecDeque;

use eframe::egui::{
    self, Align2, Color32, FontFamily, FontId, Pos2, Rect, RichText, Sense, Stroke, Vec2,
};

use crate::canvas_renderer::{draw_mini_gate_icon, draw_undo_redo_toolbar_button};
use crate::cdl_io::{export_cdo, import_cdo};
use crate::i18n::Locale;
use crate::mechanics::{
    LogicState, DEFAULT_VIEW_ZOOM, MAX_TIME_STEP_MS, MINIMAP_HEIGHT, MINIMAP_WIDTH,
    MIN_TIME_STEP_MS, OSCOPE_GRID_STEP_X, OSCOPE_HEIGHT, OSCOPE_TRACE_LENGTH, OSCOPE_WIDTH,
};
use crate::simulator::{CircuitPage, OscopeChannel};
use crate::LogicCraftApp;

impl LogicCraftApp {
    pub fn draw_top_bars(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);

        // Top Menu Bar
        egui::TopBottomPanel::top("top_menu_bar")
            .exact_height(26.0)
            .frame(
                egui::Frame::none()
                    .fill(p.bg_main)
                    .stroke(Stroke::new(1.0, p.border))
                    .inner_margin(egui::Margin {
                        left: 6.0,
                        right: 0.0,
                        top: 0.0,
                        bottom: 0.0,
                    }),
            )
            .show(ctx, |ui| {
                // Top Menu Bar
                egui::menu::bar(ui, |ui| {
                    ui.add_space(2.0);
                    let title_resp = ui.add(
                        egui::Label::new(
                            RichText::new("◆ LOGICCRAFT // CAD")
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .strong()
                                .color(p.cyan),
                        )
                        .sense(Sense::click_and_drag()),
                    );
                    if title_resp.double_clicked() {
                        let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                        ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                    } else if title_resp.drag_started_by(egui::PointerButton::Primary) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }
                    crate::theme::group_separator(ui);

                    ui.menu_button(tr.menu_file.to_uppercase(), |ui| {
                        if ui.button(tr.file_new).clicked() {
                            self.save_undo_snapshot();
                            self.sim.pages = vec![CircuitPage::new("Page 1")];
                            self.sim.current_page = 0;
                            self.status_message = "Created new blank circuit.".to_string();
                            ui.close_menu();
                        }
                        if ui.button(tr.file_load_demo).clicked() {
                            self.save_undo_snapshot();
                            self.sim.populate_demo_circuit(&self.lib);
                            self.status_message =
                                "Loaded Counter, Full Adder & Connector Demo Circuit.".to_string();
                            ui.close_menu();
                        }
                        if ui.button("🏓 Load Pong Arcade Demo (CPU + VRAM + Keyboard)").clicked() {
                            self.save_undo_snapshot();
                            self.sim.populate_demo_circuit(&self.lib);
                            if let Some(pong_idx) = self
                                .sim
                                .pages
                                .iter()
                                .position(|pg| pg.name.contains("Pong"))
                            {
                                self.sim.current_page = pong_idx;
                            }
                            self.sim.cpu.load_pong_demo();
                            self.sim.cpu.running = true;
                            self.status_message =
                                "Loaded Pong Arcade (2 Players: W/S & Up/Down, Space to Start)!".to_string();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(tr.file_open).clicked() {
                            ui.close_menu();
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("LogicCraft Circuit (*.lcc, *.cdl)", &["lcc", "cdl", "xml"])
                                .pick_file()
                            {
                                self.load_circuit_from_path(&path);
                            }
                        }
                        if ui.button(tr.file_save).clicked() {
                            ui.close_menu();
                            self.save_circuit_dialog(false);
                        }
                        if ui.button(tr.file_save_as).clicked() {
                            ui.close_menu();
                            self.save_circuit_dialog(true);
                        }
                        ui.separator();
                        if ui.checkbox(&mut self.export_with_grid, tr.file_export_grid).changed() {
                            self.save_settings();
                        }
                        if ui.checkbox(&mut self.export_with_color, tr.file_export_color).changed() {
                            self.save_settings();
                        }
                        if ui.button(tr.file_export_image).clicked() {
                            ui.close_menu();
                            self.export_canvas_image_dialog();
                        }
                        if ui.button(tr.file_copy_image).clicked() {
                            ui.close_menu();
                            self.copy_canvas_image_to_clipboard();
                        }
                        ui.separator();
                        if ui.button(tr.file_exit).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    });

                    ui.menu_button(tr.menu_edit.to_uppercase(), |ui| {
                        if ui
                            .add_enabled(
                                !self.undo_stack.is_empty(),
                                egui::Button::new(tr.edit_undo),
                            )
                            .clicked()
                        {
                            self.undo();
                            ui.close_menu();
                        }
                        if ui
                            .add_enabled(
                                !self.redo_stack.is_empty(),
                                egui::Button::new(tr.edit_redo),
                            )
                            .clicked()
                        {
                            self.redo();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(tr.edit_new_tab).clicked() {
                            self.add_new_tab();
                            ui.close_menu();
                        }
                        if ui.button(tr.edit_select_all).clicked() {
                            let page_idx = self.sim.current_page;
                            for g in &mut self.sim.pages[page_idx].gates {
                                g.selected = true;
                            }
                            for w in &mut self.sim.pages[page_idx].wires {
                                w.selected = true;
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(tr.edit_copy).clicked() {
                            self.copy_selected(ctx);
                            ui.close_menu();
                        }
                        if ui.button(tr.edit_paste).clicked() {
                            self.paste_clipboard();
                            ui.close_menu();
                        }
                        if ui.button(tr.file_copy_image).clicked() {
                            self.copy_canvas_image_to_clipboard();
                            ui.close_menu();
                        }
                        if ui.button(tr.edit_rotate).clicked() {
                            self.rotate_selected(90.0);
                            ui.close_menu();
                        }
                        if ui.button(tr.edit_delete).clicked() {
                            self.delete_selected();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(tr.edit_custom_studio).clicked() {
                            self.show_custom_ic_studio = true;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button(tr.menu_view.to_uppercase(), |ui| {
                        if ui.checkbox(&mut self.show_oscope, tr.view_oscope).changed() {
                            self.save_settings();
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(&mut self.show_kmap_studio, tr.view_kmap)
                            .changed()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(&mut self.show_memory_editor, tr.view_memory)
                            .changed()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(&mut self.show_custom_ic_studio, tr.view_custom_ic)
                            .changed()
                        {
                            ui.close_menu();
                        }
                        if ui
                            .checkbox(
                                &mut self.show_cpu_studio,
                                "🖥 CPU & Assembly Studio / Debugger\tCtrl+M",
                            )
                            .changed()
                        {
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button(tr.view_settings, |ui| {
                            if ui.checkbox(&mut self.show_gridlines, tr.view_gridlines).changed() {
                                self.save_settings();
                            }
                            if ui.checkbox(&mut self.show_wire_conn, tr.view_wire_dots).changed() {
                                self.save_settings();
                            }
                            if ui.checkbox(&mut self.dark_mode, tr.view_dark_theme).changed() {
                                self.apply_theme(ctx);
                                self.save_settings();
                            }
                        });
                    });

                    // Dedicated Language / Locale Menu
                    ui.menu_button(format!("{}", tr.menu_language.to_uppercase()), |ui| {
                        for loc in Locale::ALL {
                            if ui
                                .selectable_label(self.locale == loc, loc.display_name())
                                .clicked()
                            {
                                self.locale = loc;
                                self.status_message = loc.tr().status_welcome.to_string();
                                self.save_settings();
                                ui.close_menu();
                            }
                        }
                    });

                    ui.menu_button(tr.menu_help.to_uppercase(), |ui| {
                        if ui.button("Contents...\tF1").clicked() {
                            self.show_about_dialog = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(tr.update_menu_item).clicked() {
                            self.show_update_dialog = true;
                            self.trigger_update_check(ctx, true);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button(tr.about_window_title).clicked() {
                            self.show_about_dialog = true;
                            ui.close_menu();
                        }
                    });

                    // Embed default window buttons (minimize, maximize, close) at the other end of the bar
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        Self::draw_window_caption_buttons(ui, ctx, p, self.locale);

                        // Empty space between menus and caption buttons acts as a draggable titlebar strip
                        let remaining_rect = ui.available_rect_before_wrap();
                        if remaining_rect.width() > 10.0 {
                            let drag_resp = ui.interact(
                                remaining_rect,
                                ui.id().with("title_bar_drag_spacer"),
                                Sense::click_and_drag(),
                            );
                            if drag_resp.double_clicked() {
                                let is_max = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max));
                            } else if drag_resp.drag_started_by(egui::PointerButton::Primary) {
                                ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                            }
                        }
                    });
                });
            });

        // Technical Action Toolbar Panel with symmetric top and bottom margins
        egui::TopBottomPanel::top("action_toolbar")
            .frame(
                egui::Frame::none()
                    .fill(p.bg_secondary)
                    .stroke(Stroke::new(1.0, p.border))
                    .inner_margin(egui::Margin::symmetric(6.0, 4.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    ui.spacing_mut().interact_size.y = 22.0;
                    ui.spacing_mut().slider_width = 64.0;

                    if crate::theme::tool_button(ui, tr.tb_new, false, p.cyan)
                        .on_hover_text(tr.file_new)
                        .clicked()
                    {
                            self.save_undo_snapshot();
                            self.sim.pages = vec![CircuitPage::new("Page 1")];
                            self.sim.current_page = 0;
                        }
                        if crate::theme::tool_button(ui, tr.tb_open, false, p.cyan)
                            .on_hover_text(tr.file_open)
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("LogicCraft Circuit (*.lcc, *.cdl)", &["lcc", "cdl", "xml"])
                                .pick_file()
                            {
                                self.load_circuit_from_path(&path);
                            }
                        }
                        if crate::theme::tool_button(ui, tr.tb_save, false, p.cyan)
                            .on_hover_text(tr.file_save)
                            .clicked()
                        {
                            self.save_circuit_dialog(false);
                        }
                        if crate::theme::tool_button(ui, tr.tb_copy_img, false, p.cyan)
                            .on_hover_text(tr.file_copy_image)
                            .clicked()
                        {
                            self.copy_canvas_image_to_clipboard();
                        }

                        crate::theme::group_separator(ui);

                        if draw_undo_redo_toolbar_button(
                            ui,
                            tr.tb_undo,
                            true,
                            !self.undo_stack.is_empty(),
                            self.dark_mode,
                        )
                        .on_hover_text(tr.edit_undo)
                        .clicked()
                        {
                            self.undo();
                        }
                        if draw_undo_redo_toolbar_button(
                            ui,
                            tr.tb_redo,
                            false,
                            !self.redo_stack.is_empty(),
                            self.dark_mode,
                        )
                        .on_hover_text(tr.edit_redo)
                        .clicked()
                        {
                            self.redo();
                        }
                        if crate::theme::tool_button(ui, tr.tb_rotate, false, p.cyan)
                            .on_hover_text(tr.edit_rotate)
                            .clicked()
                        {
                            self.rotate_selected(90.0);
                        }

                        crate::theme::group_separator(ui);

                        let placing_from = self.placing_gate_type.as_deref() == Some("DA_FROM");
                        if crate::theme::tool_button(ui, tr.tb_from, placing_from, p.cyan)
                            .on_hover_text(tr.inspector_from_title)
                            .clicked()
                        {
                            self.placing_gate_type = Some("DA_FROM".to_string());
                            self.placing_multi_active = false;
                        }
                        let placing_to = self.placing_gate_type.as_deref() == Some("DE_TO");
                        if crate::theme::tool_button(ui, tr.tb_to, placing_to, p.purple)
                            .on_hover_text(tr.inspector_to_title)
                            .clicked()
                        {
                            self.placing_gate_type = Some("DE_TO".to_string());
                            self.placing_multi_active = false;
                        }

                        crate::theme::group_separator(ui);

                        if crate::theme::tool_button(ui, "ZOOM +", false, p.cyan)
                            .on_hover_text("Zoom In")
                            .clicked()
                        {
                            let page = &mut self.sim.pages[self.sim.current_page];
                            page.zoom_world_per_px = (page.zoom_world_per_px * 0.8).max(0.02);
                        }
                        if crate::theme::tool_button(ui, "ZOOM -", false, p.cyan)
                            .on_hover_text("Zoom Out")
                            .clicked()
                        {
                            let page = &mut self.sim.pages[self.sim.current_page];
                            page.zoom_world_per_px = (page.zoom_world_per_px * 1.25).min(0.35);
                        }
                        if crate::theme::tool_button(ui, tr.tb_fit, false, p.cyan)
                            .on_hover_text("Reset viewport")
                            .clicked()
                        {
                            let page = &mut self.sim.pages[self.sim.current_page];
                            page.pan_world = [0.0, 2.0];
                            page.zoom_world_per_px = DEFAULT_VIEW_ZOOM;
                        }

                        crate::theme::group_separator(ui);

                        let (play_txt, play_col) = if self.sim.running {
                            (tr.tb_pause, p.green)
                        } else {
                            (tr.tb_play, p.orange)
                        };
                        if crate::theme::tool_button(ui, play_txt, true, play_col)
                            .on_hover_text("Pause / Resume simulation (Space)")
                            .clicked()
                        {
                            self.sim.running = !self.sim.running;
                        }
                        if crate::theme::tool_button(ui, tr.tb_step, false, p.orange)
                            .on_hover_text("Advance simulation by 1 step")
                            .clicked()
                        {
                            self.sim.running = false;
                            self.sim.step(&self.lib);
                        }

                        let speed_galley = ui.painter().layout_no_wrap(
                            tr.tb_speed.to_uppercase(),
                            FontId::new(10.0, FontFamily::Monospace),
                            p.text_secondary,
                        );
                        let (speed_rect, _) = ui.allocate_exact_size(
                            Vec2::new(speed_galley.size().x + 4.0, 22.0),
                            Sense::hover(),
                        );
                        ui.painter().galley(
                            Pos2::new(
                                speed_rect.left() + 2.0,
                                speed_rect.center().y - (speed_galley.size().y / 2.0) + 1.5,
                            ),
                            speed_galley,
                            p.text_secondary,
                        );
                        ui.add(
                            egui::Slider::new(
                                &mut self.sim.time_step_ms,
                                MIN_TIME_STEP_MS..=MAX_TIME_STEP_MS,
                            )
                            .suffix("ms")
                            .logarithmic(true),
                        )
                        .on_hover_text("Simulation TimeStep (1..500 ms)");

                        crate::theme::group_separator(ui);

                        if crate::theme::tool_button(ui, tr.tb_kmap, self.show_kmap_studio, p.purple)
                            .on_hover_text(tr.view_kmap)
                            .clicked()
                        {
                            self.show_kmap_studio = !self.show_kmap_studio;
                        }
                        if crate::theme::tool_button(ui, tr.tb_oscope, self.show_oscope, p.cyan)
                            .on_hover_text(tr.view_oscope)
                            .clicked()
                        {
                            self.show_oscope = !self.show_oscope;
                            self.save_settings();
                        }
                        if crate::theme::tool_button(ui, tr.tb_memory, self.show_memory_editor, p.purple)
                            .on_hover_text(tr.view_memory)
                            .clicked()
                        {
                            self.show_memory_editor = !self.show_memory_editor;
                        }
                        if crate::theme::tool_button(
                            ui,
                            "🖥 CPU / ASM",
                            self.show_cpu_studio,
                            p.green,
                        )
                        .on_hover_text("Open Programmable CPU & Assembly Debugger Studio (Ctrl+M)")
                        .clicked()
                        {
                            self.show_cpu_studio = !self.show_cpu_studio;
                        }
                        if crate::theme::tool_button(ui, "🏓 PONG", false, p.cyan)
                            .on_hover_text("Jump to 2-Player Pong Arcade Page & Run CPU")
                            .clicked()
                        {
                            if let Some(pong_idx) = self
                                .sim
                                .pages
                                .iter()
                                .position(|pg| pg.name.contains("Pong"))
                            {
                                self.sim.current_page = pong_idx;
                            } else {
                                self.save_undo_snapshot();
                                let new_idx = self.sim.pages.len();
                                self.sim
                                    .pages
                                    .push(CircuitPage::new("Pong Arcade (CPU + MMIO)"));
                                self.sim.populate_pong_page(&self.lib, new_idx);
                                self.sim.current_page = new_idx;
                            }
                            self.sim.cpu.running = true;
                            self.status_message =
                                "Pong Arcade Ready — Player 1: W / S | Player 2: Up / Down | Space: Start"
                                    .to_string();
                        }
                    });
            });
    }

    /// Renders the embedded window management buttons: Minimize, Maximize/Restore, and Close.
    pub fn draw_window_caption_buttons(
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        p: crate::theme::Palette,
        locale: crate::i18n::Locale,
    ) {
        let (tip_min, tip_max, tip_res, tip_cls) = locale.win_tooltips();
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let hover_bg = if p.bg_main == crate::theme::BG_MAIN {
            Color32::from_rgb(0x22, 0x2A, 0x35)
        } else {
            Color32::from_rgb(0xDC, 0xE2, 0xEA)
        };

        // Note: Because this is called in a right_to_left layout,
        // we add Close first (far right), then Maximize, then Minimize (leftmost in group).

        // 1. CLOSE BUTTON (Far Right)
        let (close_rect, close_resp) = ui.allocate_exact_size(Vec2::new(40.0, 26.0), Sense::click());
        let close_hovered = close_resp.hovered();
        let close_active = close_resp.is_pointer_button_down_on();
        let close_bg = if close_active {
            Color32::from_rgb(0xC4, 0x1B, 0x22)
        } else if close_hovered {
            Color32::from_rgb(0xE8, 0x11, 0x23)
        } else {
            Color32::TRANSPARENT
        };
        let close_fg = if close_hovered || close_active {
            Color32::WHITE
        } else {
            p.text_dim
        };
        ui.painter().rect_filled(close_rect, 0.0, close_bg);
        let center = close_rect.center();
        let s = 4.0;
        ui.painter().line_segment(
            [center - Vec2::new(s, s), center + Vec2::new(s, s)],
            Stroke::new(1.3, close_fg),
        );
        ui.painter().line_segment(
            [center + Vec2::new(-s, s), center - Vec2::new(-s, s)],
            Stroke::new(1.3, close_fg),
        );
        let close_resp = close_resp.on_hover_text(tip_cls);
        if close_resp.clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        // 2. MAXIMIZE / RESTORE BUTTON (Middle)
        let (max_rect, mut max_resp) = ui.allocate_exact_size(Vec2::new(36.0, 26.0), Sense::click());
        let max_hovered = max_resp.hovered();
        let max_active = max_resp.is_pointer_button_down_on();
        let max_bg = if max_active {
            crate::theme::alpha(p.cyan, 60)
        } else if max_hovered {
            hover_bg
        } else {
            Color32::TRANSPARENT
        };
        let max_fg = if max_hovered || max_active { p.text } else { p.text_dim };
        ui.painter().rect_filled(max_rect, 0.0, max_bg);
        let center = max_rect.center();
        if !is_maximized {
            let box_rect = Rect::from_center_size(center, Vec2::splat(9.0));
            ui.painter().rect_stroke(box_rect, 0.0, Stroke::new(1.1, max_fg));
            max_resp = max_resp.on_hover_text(tip_max);
        } else {
            let back_rect = Rect::from_center_size(center + Vec2::new(1.5, -1.5), Vec2::splat(7.5));
            let front_rect = Rect::from_center_size(center + Vec2::new(-1.5, 1.5), Vec2::splat(7.5));
            ui.painter().rect_stroke(back_rect, 0.0, Stroke::new(1.0, max_fg));
            ui.painter().rect_filled(
                front_rect,
                0.0,
                if max_bg == Color32::TRANSPARENT { p.bg_main } else { max_bg },
            );
            ui.painter().rect_stroke(front_rect, 0.0, Stroke::new(1.0, max_fg));
            max_resp = max_resp.on_hover_text(tip_res);
        }
        if max_resp.clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_maximized));
        }

        // 3. MINIMIZE BUTTON (Left of button group)
        let (min_rect, min_resp) = ui.allocate_exact_size(Vec2::new(36.0, 26.0), Sense::click());
        let min_hovered = min_resp.hovered();
        let min_active = min_resp.is_pointer_button_down_on();
        let min_bg = if min_active {
            crate::theme::alpha(p.cyan, 60)
        } else if min_hovered {
            hover_bg
        } else {
            Color32::TRANSPARENT
        };
        let min_fg = if min_hovered || min_active { p.text } else { p.text_dim };
        ui.painter().rect_filled(min_rect, 0.0, min_bg);
        let center = min_rect.center();
        ui.painter().line_segment(
            [center - Vec2::new(4.5, 0.0), center + Vec2::new(4.5, 0.0)],
            Stroke::new(1.2, min_fg),
        );
        let min_resp = min_resp.on_hover_text(tip_min);
        if min_resp.clicked() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
    }

    pub fn draw_status_bar(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        let tips = self.locale.cycling_tips();
        let cur_tip = tips
            .get(self.tip_index % tips.len().max(1))
            .copied()
            .unwrap_or("");
        let mut advance_tip = false;

        egui::TopBottomPanel::bottom("status_bar")
            .exact_height(26.0)
            .frame(crate::theme::toolbar_frame(self.dark_mode))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let page = &self.sim.pages[self.sim.current_page];
                    if self.sim.running {
                        crate::theme::status_pill(ui, "● RUN", p.green);
                    } else {
                        crate::theme::status_pill(ui, "■ HALT", p.orange);
                    }
                    crate::theme::group_separator(ui);

                    // Cycling shortcut & workflow tip right after RUN / HALT
                    let tip_resp = ui.add(
                        egui::Label::new(
                            RichText::new(cur_tip)
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        )
                        .truncate(true)
                        .sense(Sense::click()),
                    );
                    if tip_resp.clicked() {
                        advance_tip = true;
                    }

                    // Render right telemetry so it is cleanly reserved and never truncated
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{}: {:05} | {}: {:03} | {}: {:03} | {}: {:03.0}%",
                                tr.status_step.to_uppercase(),
                                self.sim.step_count,
                                tr.status_gates.to_uppercase(),
                                page.gates.len(),
                                tr.status_wires.to_uppercase(),
                                page.wires.len(),
                                tr.status_zoom.to_uppercase(),
                                (DEFAULT_VIEW_ZOOM / page.zoom_world_per_px) * 100.0
                            ))
                            .font(FontId::new(10.5, FontFamily::Monospace))
                            .strong()
                            .color(p.text_primary),
                        );
                        crate::theme::group_separator(ui);
                        for (st, name) in [
                            (LogicState::Unknown, "[?:UNK]"),
                            (LogicState::Conflict, "[X:ERR]"),
                            (LogicState::HiZ, "[Z:HIZ]"),
                            (LogicState::One, "[1:HIGH]"),
                            (LogicState::Zero, "[0:LOW]"),
                        ] {
                            ui.colored_label(
                                st.wire_color(self.dark_mode),
                                RichText::new(name)
                                    .font(FontId::new(10.0, FontFamily::Monospace))
                                    .strong(),
                            );
                        }
                    });
                });
            });

        if advance_tip {
            self.tip_index = (self.tip_index + 1) % tips.len().max(1);
            self.last_tip_switch = std::time::Instant::now();
        }
    }

    pub fn draw_oscope_panel(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        egui::TopBottomPanel::bottom("oscope_panel")
            .resizable(true)
            .default_height(OSCOPE_HEIGHT * 0.78)
            .min_height(145.0)
            .max_height(OSCOPE_WIDTH * 0.45)
            .frame(crate::theme::panel_frame(self.dark_mode))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("▣ {}", tr.oscope_heading.to_uppercase()))
                            .font(FontId::new(11.5, FontFamily::Monospace))
                            .strong()
                            .color(p.cyan),
                    );
                    crate::theme::group_separator(ui);
                    let pause_btn = if self.sim.oscope_paused {
                        tr.oscope_resume
                    } else {
                        tr.oscope_hold
                    };
                    if crate::theme::tool_button(
                        ui,
                        pause_btn,
                        self.sim.oscope_paused,
                        p.orange,
                    )
                    .clicked()
                    {
                        self.sim.oscope_paused = !self.sim.oscope_paused;
                    }
                    if crate::theme::tool_button(ui, tr.oscope_clear, false, p.red).clicked() {
                        for ch in &mut self.sim.oscope_channels {
                            ch.history.clear();
                        }
                    }
                    if crate::theme::tool_button(ui, tr.oscope_load, false, p.cyan).clicked() {
                        if let Some(p_file) = rfd::FileDialog::new()
                            .add_filter("CEDAR O-scope Layout (*.cdo)", &["cdo"])
                            .pick_file()
                        {
                            if let Ok(txt) = std::fs::read_to_string(&p_file) {
                                let loaded = import_cdo(&txt);
                                if !loaded.is_empty() {
                                    self.sim.oscope_channels = loaded;
                                }
                            }
                        }
                    }
                    if crate::theme::tool_button(ui, tr.oscope_save, false, p.cyan).clicked() {
                        if let Some(p_file) = rfd::FileDialog::new()
                            .add_filter("CEDAR O-scope Layout (*.cdo)", &["cdo"])
                            .set_file_name("layout.cdo")
                            .save_file()
                        {
                            let _ = std::fs::write(&p_file, export_cdo(&self.sim.oscope_channels));
                        }
                    }
                    crate::theme::group_separator(ui);
                    let page0 = &self.sim.pages[self.sim.current_page];
                    if !page0.wires.is_empty() {
                        self.new_channel_wire_id = self
                            .new_channel_wire_id
                            .min(page0.wires.len().saturating_sub(1));
                        let selected_w = &page0.wires[self.new_channel_wire_id];
                        egui::ComboBox::from_id_source("oscope_add_wire")
                            .selected_text(
                                RichText::new(format!("NET #{:02}: {}", selected_w.id, selected_w.label))
                                    .font(FontId::new(11.0, FontFamily::Monospace)),
                            )
                            .show_ui(ui, |ui| {
                                for (i, w) in page0.wires.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut self.new_channel_wire_id,
                                        i,
                                        format!("NET #{:02}: {}", w.id, w.label),
                                    );
                                }
                            });
                        if crate::theme::primary_button(ui, tr.oscope_probe, p.cyan).clicked() {
                            let w = &page0.wires[self.new_channel_wire_id];
                            self.sim.oscope_channels.push(OscopeChannel {
                                name: format!("W{}: {}", w.id, w.label),
                                target_wire_id: Some(w.id),
                                target_pin: w.endpoints.first().cloned(),
                                history: VecDeque::new(),
                            });
                        }
                    }
                });

                ui.add_space(4.0);
                let avail = ui.available_size();
                let (rect, _) = ui.allocate_exact_size(avail, Sense::hover());
                let painter = ui.painter_at(rect);

                painter.rect_filled(rect, 0.0, p.bg_grid);
                painter.rect_stroke(rect, 0.0, Stroke::new(1.0_f32, p.border_primary));

                let label_col_w = 150.0;
                let plot_rect = Rect::from_min_max(
                    Pos2::new(rect.left() + label_col_w, rect.top() + 2.0),
                    Pos2::new(rect.right() - 4.0, rect.bottom() - 2.0),
                );

                // Label column background & vertical separator
                let lbl_col_rect = Rect::from_min_max(
                    rect.left_top() + Vec2::splat(1.0),
                    Pos2::new(rect.left() + label_col_w, rect.bottom() - 1.0),
                );
                painter.rect_filled(lbl_col_rect, 0.0, p.bg_secondary);
                painter.line_segment(
                    [
                        Pos2::new(rect.left() + label_col_w, rect.top()),
                        Pos2::new(rect.left() + label_col_w, rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, p.border_primary),
                );

                let num_divs = (OSCOPE_TRACE_LENGTH / OSCOPE_GRID_STEP_X).max(1);
                for div in 0..=num_divs {
                    let x = plot_rect.left() + plot_rect.width() * (div as f32 / num_divs as f32);
                    painter.line_segment(
                        [Pos2::new(x, plot_rect.top()), Pos2::new(x, plot_rect.bottom())],
                        Stroke::new(1.0_f32, p.grid_major),
                    );
                }

                let num_ch = self.sim.oscope_channels.len().max(1);
                let row_h = plot_rect.height() / num_ch as f32;
                let max_pts = self.sim.max_history.max(2);

                for (ch_idx, ch) in self.sim.oscope_channels.iter().enumerate() {
                    let y_top = plot_rect.top() + ch_idx as f32 * row_h;
                    let y_bot = y_top + row_h;
                    let y_high = y_top + row_h * 0.22;
                    let y_low = y_top + row_h * 0.78;
                    let y_mid = (y_high + y_low) * 0.5;

                    painter.line_segment(
                        [Pos2::new(rect.left(), y_bot), Pos2::new(rect.right(), y_bot)],
                        Stroke::new(1.0_f32, p.border_secondary),
                    );

                    // Subtle center reference dashed/muted line
                    painter.line_segment(
                        [Pos2::new(plot_rect.left(), y_mid), Pos2::new(plot_rect.right(), y_mid)],
                        Stroke::new(0.7_f32, p.grid_minor),
                    );

                    let latest = ch.history.back().copied().unwrap_or(LogicState::HiZ);
                    // Left channel accent bar
                    painter.rect_filled(
                        Rect::from_min_max(
                            Pos2::new(rect.left() + 1.0, y_top + 2.0),
                            Pos2::new(rect.left() + 4.0, y_bot - 2.0),
                        ),
                        0.0,
                        latest.wire_color(self.dark_mode),
                    );

                    painter.text(
                        Pos2::new(rect.left() + 9.0, y_mid),
                        Align2::LEFT_CENTER,
                        format!("CH{:02} {}", ch_idx + 1, ch.name),
                        FontId::new(11.0, FontFamily::Monospace),
                        p.text_primary,
                    );

                    let st_badge = match latest {
                        LogicState::One => "HI:1",
                        LogicState::Zero => "LO:0",
                        LogicState::HiZ => "HZ:Z",
                        LogicState::Conflict => "ER:X",
                        LogicState::Unknown => "UN:?",
                    };
                    painter.text(
                        Pos2::new(rect.left() + label_col_w - 6.0, y_mid),
                        Align2::RIGHT_CENTER,
                        st_badge,
                        FontId::new(10.5, FontFamily::Monospace),
                        latest.wire_color(self.dark_mode),
                    );

                    let mut prev_pos: Option<(Pos2, LogicState)> = None;
                    for (idx, &st) in ch.history.iter().enumerate() {
                        let x = plot_rect.left()
                            + (idx as f32 / (max_pts - 1) as f32) * plot_rect.width();
                        let y = match st {
                            LogicState::One => y_high,
                            LogicState::Zero => y_low,
                            _ => y_mid,
                        };
                        let cur = Pos2::new(x, y);
                        let col = match st {
                            LogicState::One => p.green,
                            LogicState::Zero => p.cyan,
                            _ => st.wire_color(self.dark_mode),
                        };
                        if let Some((prev, _)) = prev_pos {
                            if (prev.y - cur.y).abs() > 0.5 {
                                painter.line_segment(
                                    [Pos2::new(cur.x, prev.y), cur],
                                    Stroke::new(1.8_f32, col),
                                );
                            }
                            painter.line_segment(
                                [prev, Pos2::new(cur.x, prev.y)],
                                Stroke::new(1.8_f32, col),
                            );
                        }
                        prev_pos = Some((cur, st));
                    }
                }
            });
    }

    pub fn draw_left_sidebar(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let locale = self.locale;
        let p = crate::theme::palette(self.dark_mode);

        egui::SidePanel::left("left_palette_sidebar")
            .resizable(true)
            .default_width(280.0)
            .min_width(240.0)
            .frame(crate::theme::panel_frame(self.dark_mode))
            .show(ctx, |ui| {
                crate::theme::section_header(ui, tr.palette_heading, p.cyan);

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("SEARCH //")
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    let has_search = !self.palette_search.is_empty();
                    ui.add(
                        egui::TextEdit::singleline(&mut self.palette_search)
                            .hint_text(tr.palette_search_hint)
                            .desired_width(ui.available_width() - if has_search { 24.0 } else { 0.0 }),
                    );
                    if has_search && ui.small_button("×").clicked() {
                        self.palette_search.clear();
                    }
                });

                if let Some(placing) = self.placing_gate_type.clone() {
                    ui.horizontal(|ui| {
                        crate::theme::status_pill(ui, &format!("PLACING: {placing}"), p.cyan);
                        if ui.small_button(tr.palette_cancel).clicked() {
                            self.placing_gate_type = None;
                            self.placing_multi_active = false;
                        }
                    });
                }

                ui.separator();

                if self.palette_search.trim().is_empty() {
                    let cur_cat_label =
                        locale.category_name(&self.lib.categories[self.active_category].name);
                    egui::ComboBox::from_id_source("library_cat_combo")
                        .width(ui.available_width() - 4.0)
                        .selected_text(
                            RichText::new(cur_cat_label)
                                .font(FontId::new(11.0, FontFamily::Monospace)),
                        )
                        .show_ui(ui, |ui| {
                            for (idx, cat) in self.lib.categories.iter().enumerate() {
                                ui.selectable_value(
                                    &mut self.active_category,
                                    idx,
                                    locale.category_name(&cat.name),
                                );
                            }
                        });
                }
                if crate::theme::tool_button(ui, tr.palette_create_custom_btn, false, p.orange)
                    .on_hover_text(tr.view_custom_ic)
                    .clicked()
                {
                    self.show_custom_ic_studio = true;
                }

                ui.add_space(3.0);

                egui::ScrollArea::vertical()
                    .id_source("palette_scroll")
                    .max_height(ui.available_height() - MINIMAP_HEIGHT - 210.0)
                    .show(ui, |ui| {
                        let query = self.palette_search.trim().to_lowercase();
                        let cats_to_show: Vec<usize> = if query.is_empty() {
                            vec![self.active_category]
                        } else {
                            (0..self.lib.categories.len()).collect()
                        };

                        for c_idx in cats_to_show {
                            let cat = &self.lib.categories[c_idx];
                            for gate in &cat.gates {
                                let localized_cap = locale.gate_caption(&gate.caption);
                                if !query.is_empty()
                                    && !localized_cap.to_lowercase().contains(&query)
                                    && !gate.caption.to_lowercase().contains(&query)
                                    && !gate.type_name.to_lowercase().contains(&query)
                                {
                                    continue;
                                }
                                let is_active = self.placing_gate_type.as_deref()
                                    == Some(gate.type_name.as_str());

                                let row_h = 36.0;
                                let (row_rect, resp) = ui.allocate_exact_size(
                                    Vec2::new(ui.available_width(), row_h),
                                    Sense::click(),
                                );
                                let bg = if is_active {
                                    p.bg_elevated
                                } else if resp.hovered() {
                                    p.bg_secondary
                                } else {
                                    p.bg_panel
                                };
                                let border_col = if is_active {
                                    p.cyan
                                } else if resp.hovered() {
                                    p.border_primary
                                } else {
                                    p.border_secondary
                                };
                                let title_col = if is_active {
                                    p.cyan
                                } else {
                                    p.text_primary
                                };

                                let painter = ui.painter_at(row_rect);
                                painter.rect_filled(row_rect, 0.0, bg);
                                painter.rect_stroke(row_rect, 0.0, Stroke::new(1.0, border_col));

                                if is_active {
                                    // Left 3px cyan indicator bar
                                    painter.rect_filled(
                                        Rect::from_min_max(
                                            row_rect.left_top(),
                                            Pos2::new(row_rect.left() + 3.0, row_rect.bottom()),
                                        ),
                                        0.0,
                                        p.cyan,
                                    );
                                }

                                let icon_rect = Rect::from_min_size(
                                    Pos2::new(row_rect.left() + 6.0, row_rect.top() + 3.0),
                                    Vec2::new(38.0, row_h - 6.0),
                                );
                                draw_mini_gate_icon(
                                    &painter,
                                    icon_rect,
                                    gate.shape_style,
                                    gate.out_inverted,
                                    gate.in_inverted,
                                    self.dark_mode,
                                );

                                painter.text(
                                    Pos2::new(icon_rect.right() + 8.0, row_rect.center().y - 6.0),
                                    Align2::LEFT_CENTER,
                                    &localized_cap,
                                    FontId::new(11.5, FontFamily::Monospace),
                                    title_col,
                                );
                                painter.text(
                                    Pos2::new(icon_rect.right() + 8.0, row_rect.center().y + 8.0),
                                    Align2::LEFT_CENTER,
                                    format!(
                                        "{} [{} IN / {} OUT]",
                                        gate.logic_type,
                                        gate.inputs.len(),
                                        gate.outputs.len()
                                    ),
                                    FontId::new(9.5, FontFamily::Monospace),
                                    p.text_muted,
                                );

                                if resp.clicked() {
                                    self.placing_gate_type = Some(gate.type_name.clone());
                                    self.placing_multi_active = false;
                                    self.status_message = format!(
                                        "{}: '{}'",
                                        tr.palette_placing, localized_cap
                                    );
                                }
                            }
                        }
                    });

                ui.separator();

                self.draw_inspector_section(ui);

                ui.separator();

                crate::theme::section_header(ui, tr.minimap_heading, p.purple);
                let mm_size = Vec2::new(ui.available_width().max(MINIMAP_WIDTH), MINIMAP_HEIGHT);
                let (mm_rect, mm_resp) = ui.allocate_exact_size(mm_size, Sense::click_and_drag());
                let mm_painter = ui.painter_at(mm_rect).with_clip_rect(mm_rect);
                mm_painter.rect_filled(mm_rect, 0.0, p.bg_grid);
                mm_painter.rect_stroke(mm_rect, 0.0, Stroke::new(1.0_f32, p.border_primary));

                let page = &mut self.sim.pages[self.sim.current_page];

                // Compute dynamic world bounding box for all circuit elements
                let mut min_x = f32::INFINITY;
                let mut max_x = f32::NEG_INFINITY;
                let mut min_y = f32::INFINITY;
                let mut max_y = f32::NEG_INFINITY;

                for g in &page.gates {
                    let (hw, hh) = self
                        .lib
                        .get(&g.type_name)
                        .map(|d| (d.half_size[0], d.half_size[1]))
                        .unwrap_or((2.0, 2.0));
                    min_x = min_x.min(g.pos[0] - hw);
                    max_x = max_x.max(g.pos[0] + hw);
                    min_y = min_y.min(g.pos[1] - hh);
                    max_y = max_y.max(g.pos[1] + hh);
                }

                for w in &page.wires {
                    for pt in &w.points {
                        min_x = min_x.min(pt[0]);
                        max_x = max_x.max(pt[0]);
                        min_y = min_y.min(pt[1]);
                        max_y = max_y.max(pt[1]);
                    }
                }

                // Also incorporate camera position so the viewport indicator is always visible
                let cam_world_w = self.canvas_size.x * page.zoom_world_per_px;
                let cam_world_h = self.canvas_size.y * page.zoom_world_per_px;
                let cam_half_w = cam_world_w * 0.5;
                let cam_half_h = cam_world_h * 0.5;

                min_x = min_x.min(page.pan_world[0] - cam_half_w * 0.8);
                max_x = max_x.max(page.pan_world[0] + cam_half_w * 0.8);
                min_y = min_y.min(page.pan_world[1] - cam_half_h * 0.8);
                max_y = max_y.max(page.pan_world[1] + cam_half_h * 0.8);

                // If empty or collapsed, use minimum default bounds
                if !min_x.is_finite() || !max_x.is_finite() {
                    min_x = -40.0;
                    max_x = 40.0;
                }
                if !min_y.is_finite() || !max_y.is_finite() {
                    min_y = -25.0;
                    max_y = 25.0;
                }

                let cx = (min_x + max_x) * 0.5;
                let cy = (min_y + max_y) * 0.5;
                let raw_w = (max_x - min_x).max(40.0);
                let raw_h = (max_y - min_y).max(25.0);

                // Add margin on each side so elements aren't clamped against the edge
                let span_w = raw_w * 1.30 + 8.0;
                let span_h = raw_h * 1.30 + 8.0;

                let inner_rect = mm_rect.shrink(4.0);
                let scale = (inner_rect.width() / span_w).min(inner_rect.height() / span_h);

                let to_mm = |pt: [f32; 2]| -> Pos2 {
                    Pos2::new(
                        inner_rect.center().x + (pt[0] - cx) * scale,
                        inner_rect.center().y - (pt[1] - cy) * scale,
                    )
                };

                for w in &page.wires {
                    for seg in w.points.windows(2) {
                        mm_painter.line_segment(
                            [to_mm(seg[0]), to_mm(seg[1])],
                            Stroke::new(1.2_f32, w.state.wire_color(self.dark_mode)),
                        );
                    }
                }
                for g in &page.gates {
                    let c = to_mm(g.pos);
                    let (hw, hh) = self
                        .lib
                        .get(&g.type_name)
                        .map(|d| (d.half_size[0], d.half_size[1]))
                        .unwrap_or((2.0, 2.0));
                    let mw = (hw * 2.0 * scale).clamp(3.0, 20.0);
                    let mh = (hh * 2.0 * scale).clamp(3.0, 20.0);
                    let r = Rect::from_center_size(c, Vec2::new(mw, mh));
                    mm_painter.rect_filled(
                        r,
                        1.0,
                        if g.selected {
                            p.cyan
                        } else {
                            p.border_primary
                        },
                    );
                }

                let vp_center = to_mm(page.pan_world);
                let vp_w = (cam_world_w * scale).clamp(8.0, inner_rect.width());
                let vp_h = (cam_world_h * scale).clamp(6.0, inner_rect.height());
                let vp_rect = Rect::from_center_size(vp_center, Vec2::new(vp_w, vp_h));
                mm_painter.rect_filled(vp_rect, 1.0, p.orange.linear_multiply(0.12));
                mm_painter.rect_stroke(
                    vp_rect,
                    1.0,
                    Stroke::new(1.2_f32, p.orange),
                );

                if mm_resp.dragged() || mm_resp.clicked() {
                    if let Some(pos) = mm_resp.interact_pointer_pos() {
                        page.pan_world[0] = cx + (pos.x - inner_rect.center().x) / scale;
                        page.pan_world[1] = cy - (pos.y - inner_rect.center().y) / scale;
                    }
                }
            });
    }

    pub fn draw_inspector_section(&mut self, ui: &mut egui::Ui) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        crate::theme::section_header(ui, "INSPECTOR", p.orange);

        let page_idx = self.sim.current_page;
        let selected_count = self.sim.pages[page_idx]
            .gates
            .iter()
            .filter(|g| g.selected)
            .count();
        let selected_gate_info = self.sim.pages[page_idx]
            .gates
            .iter()
            .find(|g| g.selected)
            .cloned();

        if selected_count > 1 {
            crate::theme::status_pill(
                ui,
                &format!("GROUP // {selected_count} COMPONENTS"),
                p.cyan,
            );
            ui.label(
                RichText::new(tr.inspector_group_hint)
                    .font(FontId::new(10.0, FontFamily::Monospace))
                    .color(p.text_muted),
            );
            ui.horizontal(|ui| {
                if crate::theme::tool_button(ui, tr.inspector_rotate_group, false, p.cyan).clicked() {
                    self.rotate_selected(90.0);
                }
                if crate::theme::tool_button(ui, tr.inspector_delete_group, false, p.red).clicked() {
                    self.delete_selected();
                }
            });
            ui.separator();
        }

        if let Some(g_snap) = selected_gate_info {
            let mut rotate_to: Option<f32> = None;
            let mut updated_label: Option<String> = None;
            let mut updated_hc: Option<String> = None;
            let mut updated_jid: Option<String> = None;
            let mut updated_scope_local: Option<bool> = None;

            ui.label(
                RichText::new(format!(
                    "{} #{} [{}]",
                    tr.inspector_component.to_uppercase(), g_snap.id, g_snap.type_name
                ))
                .font(FontId::new(11.0, FontFamily::Monospace))
                .strong()
                .color(p.text_primary),
            );
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(tr.inspector_angle)
                        .font(FontId::new(10.5, FontFamily::Monospace))
                        .color(p.text_secondary),
                );
                for deg in [0.0f32, 90.0, 180.0, 270.0] {
                    let is_sel = (g_snap.angle_deg - deg).abs() < 1.0;
                    if crate::theme::tool_button(ui, &format!("{deg:.0}°"), is_sel, p.cyan).clicked() {
                        rotate_to = Some(deg);
                    }
                }
            });

            if let Some(mut lbl) = g_snap.gparams.get("LABEL_TEXT").cloned() {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr.inspector_text)
                            .font(FontId::new(10.5, FontFamily::Monospace)),
                    );
                    if ui.text_edit_singleline(&mut lbl).changed() {
                        updated_label = Some(lbl);
                    }
                });
            }
            if let Some(hc_str) = g_snap.lparams.get("HALF_CYCLE") {
                let mut v = hc_str.parse::<u32>().unwrap_or(5);
                if ui
                    .add(egui::Slider::new(&mut v, 1..=50).text(tr.inspector_half_cycle))
                    .changed()
                {
                    updated_hc = Some(v.to_string());
                }
            }

            if let Some(cur_jid) = g_snap.lparams.get("JUNCTION_ID") {
                if self.connector_edit_gate_id != Some(g_snap.id) {
                    self.connector_edit_gate_id = Some(g_snap.id);
                    self.connector_name_draft = cur_jid.clone();
                    self.connector_warning = None;
                }
                let is_from = g_snap.type_name == "DA_FROM";
                let is_local = g_snap.is_tag_local();
                ui.label(
                    RichText::new(if is_from {
                        tr.inspector_from_title.to_uppercase()
                    } else {
                        tr.inspector_to_title.to_uppercase()
                    })
                    .font(FontId::new(10.5, FontFamily::Monospace))
                    .strong()
                    .color(if is_local { p.purple } else { p.cyan }),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr.inspector_scope)
                            .font(FontId::new(10.5, FontFamily::Monospace)),
                    );
                    if crate::theme::tool_button(ui, tr.inspector_scope_shared, !is_local, p.cyan).clicked() {
                        updated_scope_local = Some(false);
                    }
                    if crate::theme::tool_button(ui, tr.inspector_scope_local, is_local, p.purple).clicked() {
                        updated_scope_local = Some(true);
                    }
                });
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr.inspector_name)
                            .font(FontId::new(10.5, FontFamily::Monospace)),
                    );
                    let resp = ui.text_edit_singleline(&mut self.connector_name_draft);
                    if resp.changed() {
                        let candidate = self.connector_name_draft.trim().to_string();
                        if !candidate.is_empty() {
                            if is_from
                                && self.sim.is_from_name_taken_in_scope(
                                    &candidate,
                                    page_idx,
                                    is_local,
                                    Some(g_snap.id),
                                )
                            {
                                self.connector_warning = Some(format!(
                                    "Only one {} 'From' connector can use '{candidate}'!",
                                    if is_local { "Local" } else { "Shared" }
                                ));
                            } else {
                                self.connector_warning = None;
                                updated_jid = Some(candidate);
                            }
                        }
                    }
                });
                if let Some(ref warn) = self.connector_warning {
                    ui.colored_label(
                        p.red,
                        RichText::new(warn).font(FontId::new(10.0, FontFamily::Monospace)),
                    );
                }
                if !is_from {
                    let from_names = self.sim.from_connector_names_for_scope(page_idx, is_local);
                    if !from_names.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("FROM //")
                                    .font(FontId::new(10.0, FontFamily::Monospace))
                                    .color(p.text_secondary),
                            );
                            for fname in from_names {
                                let is_active = cur_jid == &fname;
                                if crate::theme::tool_button(ui, &fname, is_active, p.cyan).clicked() {
                                    self.connector_name_draft = fname.clone();
                                    updated_jid = Some(fname);
                                }
                            }
                        });
                    }
                }
            }

            if let Some(spec) = self.lib.custom_specs.get(&g_snap.type_name).cloned() {
                if !spec.description.is_empty() {
                    ui.label(
                        RichText::new(&spec.description)
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(p.text_muted),
                    );
                }
                if crate::theme::tool_button(ui, tr.inspector_view_custom, false, p.orange).clicked() {
                    self.load_spec_into_studio(&spec);
                    self.show_custom_ic_studio = true;
                }
            }

            if g_snap.is_memory_device() {
                if crate::theme::tool_button(ui, tr.inspector_edit_ram_rom, false, p.purple).clicked() {
                    self.memory_edit_gate_id = Some(g_snap.id);
                    self.show_memory_editor = true;
                }
            }

            // Inspector controls for Programmable CPU (`CPU_COMPONENT`)
            let mut updated_lparam: Option<(&'static str, String)> = None;
            if g_snap.type_name == "CPU_COMPONENT" {
                ui.separator();
                ui.label(
                    RichText::new("🖥 CPU PROCESSOR CONFIG")
                        .font(FontId::new(10.5, FontFamily::Monospace))
                        .strong()
                        .color(p.cyan),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("ARCH:")
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    for arch in crate::cpu::CpuArch::ALL {
                        let is_sel = self.sim.cpu.arch() == arch;
                        if crate::theme::tool_button(ui, arch.code_str(), is_sel, p.cyan)
                            .on_hover_text(arch.display_name())
                            .clicked()
                        {
                            self.sim.cpu.set_arch(arch);
                            updated_lparam = Some(("CPU_ARCH", arch.code_str().to_string()));
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("CLK MODE:")
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    let is_internal = self.sim.cpu.use_internal_clock;
                    if crate::theme::tool_button(ui, "INTERNAL (FAST)", is_internal, p.green)
                        .on_hover_text("High-speed internal clock (1 kHz - 1 MHz)")
                        .clicked()
                    {
                        self.sim.cpu.use_internal_clock = true;
                        updated_lparam = Some(("CLOCK_MODE", "INTERNAL".to_string()));
                    }
                    if crate::theme::tool_button(ui, "CIRCUIT PIN", !is_internal, p.orange)
                        .on_hover_text("Step 1 instruction per rising edge on CLK pin")
                        .clicked()
                    {
                        self.sim.cpu.use_internal_clock = false;
                        updated_lparam = Some(("CLOCK_MODE", "CIRCUIT".to_string()));
                    }
                });
                if self.sim.cpu.use_internal_clock {
                    let mut hz = self.sim.cpu.clock_hz;
                    if ui
                        .add(
                            egui::Slider::new(&mut hz, 1_000..=1_000_000)
                                .logarithmic(true)
                                .suffix(" Hz")
                                .text("CLK"),
                        )
                        .changed()
                    {
                        self.sim.cpu.clock_hz = hz;
                        updated_lparam = Some(("CLOCK_HZ", hz.to_string()));
                    }
                }
                if crate::theme::primary_button(ui, "🖥 Open CPU & Assembly Studio...", p.cyan)
                    .clicked()
                {
                    self.show_cpu_studio = true;
                }
            }

            // Inspector controls for VRAM Graphical Monitor (`DISPLAY_COMPONENT`)
            if g_snap.type_name == "DISPLAY_COMPONENT" {
                ui.separator();
                let mut disp = crate::cpu::DisplayComponent::from_lparams(&g_snap.lparams);
                ui.label(
                    RichText::new("🖵 VRAM MONITOR CONFIG (MMIO)")
                        .font(FontId::new(10.5, FontFamily::Monospace))
                        .strong()
                        .color(p.green),
                );
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("VRAM BASE:")
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    let mut hex_str = g_snap
                        .lparams
                        .get("VRAM_BASE")
                        .cloned()
                        .unwrap_or_else(|| format!("0x{:04X}", disp.vram_base));
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut hex_str)
                                .desired_width(68.0)
                                .font(FontId::new(11.0, FontFamily::Monospace)),
                        )
                        .changed()
                    {
                        let t = hex_str
                            .trim()
                            .trim_start_matches("0x")
                            .trim_start_matches("0X");
                        if let Ok(addr) = u16::from_str_radix(t, 16) {
                            updated_lparam = Some(("VRAM_BASE", format!("0x{addr:04X}")));
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for preset in [0x8000u16, 0x8800, 0x9000, 0xA000] {
                        let is_cur = disp.vram_base == preset;
                        if crate::theme::tool_button(
                            ui,
                            &format!("0x{preset:04X}"),
                            is_cur,
                            p.cyan,
                        )
                        .on_hover_text("Bind monitor to this VRAM base address")
                        .clicked()
                        {
                            disp.vram_base = preset;
                            updated_lparam = Some(("VRAM_BASE", format!("0x{preset:04X}")));
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("RES:")
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    for res in crate::cpu::DisplayResolution::ALL {
                        let is_sel = disp.resolution == res;
                        if crate::theme::tool_button(ui, res.label(), is_sel, p.cyan)
                            .on_hover_text(res.label())
                            .clicked()
                        {
                            updated_lparam = Some(("RESOLUTION", res.label().to_string()));
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("PALETTE:")
                            .font(FontId::new(10.0, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    for pal in crate::cpu::DisplayPalette::ALL {
                        let is_sel = disp.palette == pal;
                        if crate::theme::tool_button(ui, pal.code_str(), is_sel, p.purple)
                            .on_hover_text(pal.display_name())
                            .clicked()
                        {
                            updated_lparam = Some(("PALETTE", pal.code_str().to_string()));
                        }
                    }
                });
            }

            // Inspector info for Arcade Keyboard (`KEYBOARD_COMPONENT`)
            if g_snap.type_name == "KEYBOARD_COMPONENT" {
                ui.separator();
                let ks = &self.sim.cpu.keyboard.state;
                ui.label(
                    RichText::new("⌨ MMIO KEYBOARD INPUT (0xF000)")
                        .font(FontId::new(10.5, FontFamily::Monospace))
                        .strong()
                        .color(p.orange),
                );
                ui.label(
                    RichText::new(format!(
                        "P1 (W/S): [{}/{}] | P2 (↑/↓): [{}/{}] | START: [{}]",
                        u8::from(ks.p1_up),
                        u8::from(ks.p1_down),
                        u8::from(ks.p2_up),
                        u8::from(ks.p2_down),
                        u8::from(ks.start),
                    ))
                    .font(FontId::new(10.0, FontFamily::Monospace))
                    .color(p.text_primary),
                );
            }

            if let Some((k, v)) = updated_lparam {
                if let Some(g) = self.sim.pages[page_idx]
                    .gates
                    .iter_mut()
                    .find(|g| g.id == g_snap.id)
                {
                    g.lparams.insert(k.into(), v);
                }
            }

            if let Some(deg) = rotate_to {
                if let Some(g) = self.sim.pages[page_idx]
                    .gates
                    .iter_mut()
                    .find(|g| g.id == g_snap.id)
                {
                    g.angle_deg = deg;
                }
                self.sim.rebuild_wire_geometry(&self.lib, page_idx);
            }
            if let Some(lbl) = updated_label {
                if let Some(g) = self.sim.pages[page_idx]
                    .gates
                    .iter_mut()
                    .find(|g| g.id == g_snap.id)
                {
                    g.gparams.insert("LABEL_TEXT".into(), lbl);
                }
            }
            if let Some(hc) = updated_hc {
                if let Some(g) = self.sim.pages[page_idx]
                    .gates
                    .iter_mut()
                    .find(|g| g.id == g_snap.id)
                {
                    g.lparams.insert("HALF_CYCLE".into(), hc);
                }
            }
            if let Some(jid) = updated_jid {
                if let Some(g) = self.sim.pages[page_idx]
                    .gates
                    .iter_mut()
                    .find(|g| g.id == g_snap.id)
                {
                    g.lparams.insert("JUNCTION_ID".into(), jid);
                }
            }
            if let Some(new_local) = updated_scope_local {
                let target_jid = g_snap
                    .lparams
                    .get("JUNCTION_ID")
                    .cloned()
                    .unwrap_or_default();
                let is_from = g_snap.type_name == "DA_FROM";
                for g in &mut self.sim.pages[page_idx].gates {
                    if g.id == g_snap.id
                        || (is_from
                            && g.type_name == "DE_TO"
                            && g.lparams.get("JUNCTION_ID") == Some(&target_jid))
                    {
                        g.set_tag_local(new_local);
                    }
                }
            }
        } else if let Some(w_idx) = self.sim.pages[page_idx]
            .wires
            .iter()
            .position(|w| w.selected)
        {
            let mut reset_route = false;
            let mut delete_wire = false;
            {
                let w = &mut self.sim.pages[page_idx].wires[w_idx];
                ui.label(
                    RichText::new(format!(
                        "WIRE #{:02} // {}",
                        w.id,
                        w.state.label()
                    ))
                    .font(FontId::new(11.0, FontFamily::Monospace))
                    .strong()
                    .color(p.cyan),
                );
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(tr.inspector_wire_label)
                            .font(FontId::new(10.5, FontFamily::Monospace)),
                    );
                    ui.text_edit_singleline(&mut w.label);
                });
                ui.label(
                    RichText::new(tr.inspector_wire_hint)
                        .font(FontId::new(9.5, FontFamily::Monospace))
                        .color(p.text_muted),
                );
                ui.horizontal(|ui| {
                    if crate::theme::tool_button(ui, tr.inspector_wire_reset, false, p.orange).clicked() {
                        reset_route = true;
                    }
                    if crate::theme::tool_button(ui, tr.inspector_wire_delete, false, p.red).clicked() {
                        delete_wire = true;
                    }
                });
            }
            if reset_route {
                self.save_undo_snapshot();
                let w = &mut self.sim.pages[page_idx].wires[w_idx];
                w.custom_mid_x = None;
                w.custom_mid_y = None;
                w.waypoints.clear();
                w.breakpoints.clear();
                self.sim.rebuild_wire_geometry(&self.lib, page_idx);
            }
            if delete_wire {
                self.delete_selected();
            }
        } else {
            ui.label(
                RichText::new(tr.inspector_tip)
                    .font(FontId::new(10.0, FontFamily::Monospace))
                    .color(p.text_muted),
            );
        }
    }
}
