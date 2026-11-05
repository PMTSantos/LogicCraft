//! Floating Studio Windows & Modals (`ui_dialogs.rs`).
//!
//! Includes:
//! - Truth Table & Karnaugh Map (K-Map) Studio (`draw_kmap_studio_window`)
//! - Custom Component & AI Studio (`draw_custom_ic_studio_window`)
//! - RAM / ROM Hex Memory Editor (`draw_memory_editor_modal`)
//! - About Modal (`draw_about_modal`)

use std::collections::HashMap;

use eframe::egui::{self, Color32, FontFamily, FontId, RichText, Stroke};

use crate::cdl_io::{export_cdm, import_cdm_or_hex};
use crate::custom_component::{ai_prompt_template, builtin_custom_templates, CustomComponentSpec};
use crate::kmap::{gray_code, TruthTableKMap};
use crate::LogicCraftApp;

impl LogicCraftApp {
    pub fn draw_kmap_studio_window(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        let mut open = self.show_kmap_studio;
        egui::Window::new(tr.kmap_window_title)
            .open(&mut open)
            .default_size([760.0, 520.0])
            .resizable(true)
            .frame(crate::theme::window_frame(self.dark_mode))
            .show(ctx, |ui| {
                crate::theme::section_header(ui, "LOGIC SYNTHESIS // KARNAUGH MAP STUDIO", p.purple);

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    for idx in 0..self.sim.kmaps.len() {
                        let name = self.sim.kmaps[idx].name.clone();
                        let is_active = idx == self.sim.active_kmap;
                        if crate::theme::tool_button(ui, &format!("F{:02} // {}", idx + 1, name.to_uppercase()), is_active, p.purple).clicked() {
                            self.sim.active_kmap = idx;
                        }
                    }
                    crate::theme::group_separator(ui);
                    if crate::theme::tool_button(ui, tr.kmap_add_btn, false, p.cyan).clicked() {
                        let n = self.sim.kmaps.len() + 1;
                        self.sim
                            .kmaps
                            .push(TruthTableKMap::new_default(format!("Function F{n}")));
                        self.sim.active_kmap = self.sim.kmaps.len() - 1;
                    }
                });

                ui.separator();

                let k_idx = self
                    .sim
                    .active_kmap
                    .min(self.sim.kmaps.len().saturating_sub(1));
                let mut synthesize_clicked = false;
                if let Some(kmap) = self.sim.kmaps.get_mut(k_idx) {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(tr.kmap_title_lbl).font(FontId::new(10.5, FontFamily::Monospace)));
                        ui.text_edit_singleline(&mut kmap.name);
                        crate::theme::group_separator(ui);
                        ui.label(RichText::new(tr.kmap_vars_lbl).font(FontId::new(10.5, FontFamily::Monospace)));
                        for n in [2usize, 3, 4] {
                            let is_sel = kmap.num_vars == n;
                            if crate::theme::tool_button(ui, &format!("{n} VARS"), is_sel, p.cyan).clicked() {
                                kmap.num_vars = n;
                            }
                        }
                        crate::theme::group_separator(ui);
                        if ui
                            .checkbox(&mut kmap.pinned_on_canvas, tr.kmap_pin_card)
                            .changed()
                            && kmap.pinned_on_canvas
                        {
                            kmap.func_pinned_on_canvas = true;
                        }
                        ui.checkbox(&mut kmap.func_pinned_on_canvas, tr.kmap_pin_func);
                        crate::theme::group_separator(ui);
                        if crate::theme::primary_button(ui, tr.kmap_build_gates, p.green).clicked() {
                            synthesize_clicked = true;
                        }
                    });

                    ui.horizontal_wrapped(|ui| {
                        let n = kmap.num_vars.clamp(2, 4);
                        ui.label(
                            RichText::new(format!("{} //", tr.kmap_var_names.to_uppercase()))
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.cyan),
                        );
                        for i in 0..n {
                            ui.label(RichText::new(format!("IN.{}:", i + 1)).font(FontId::new(10.5, FontFamily::Monospace)));
                            ui.add(
                                egui::TextEdit::singleline(&mut kmap.var_names[i])
                                    .desired_width(34.0)
                                    .font(FontId::new(12.0, FontFamily::Monospace)),
                            );
                        }
                        crate::theme::group_separator(ui);
                        ui.label(
                            RichText::new(format!("{} //", tr.kmap_out_name.to_uppercase()))
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.purple),
                        );
                        ui.add(
                            egui::TextEdit::singleline(&mut kmap.output_name)
                                .desired_width(38.0)
                                .font(FontId::new(12.0, FontFamily::Monospace)),
                        );
                    });

                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("PRESETS //").font(FontId::new(10.5, FontFamily::Monospace)).color(p.text_secondary));
                        if crate::theme::tool_button(ui, "ALL 0", false, p.cyan).clicked() {
                            kmap.values.fill(0);
                        }
                        if crate::theme::tool_button(ui, "ALL 1", false, p.cyan).clicked() {
                            kmap.values.fill(1);
                        }
                        if crate::theme::tool_button(ui, "XOR (PARITY)", false, p.cyan).clicked() {
                            for m in 0..16usize {
                                kmap.values[m] = (m.count_ones() & 1) as u8;
                            }
                        }
                        if crate::theme::tool_button(ui, "MAJORITY", false, p.cyan).clicked() {
                            let n = kmap.num_vars;
                            for m in 0..(1usize << n) {
                                kmap.values[m] = if (m.count_ones() as usize) * 2 >= n {
                                    1
                                } else {
                                    0
                                };
                            }
                        }
                    });

                    let (imps, sop_expr) = kmap.solve_minimal_sop();
                    let pos_expr = kmap.solve_minimal_pos();
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new("MINIMAL SOP:").font(FontId::new(11.0, FontFamily::Monospace)).strong().color(p.text_secondary));
                        ui.label(
                            RichText::new(&sop_expr)
                                .font(FontId::new(12.0, FontFamily::Monospace))
                                .strong()
                                .color(p.green),
                        );
                        crate::theme::group_separator(ui);
                        ui.label(RichText::new("MINIMAL POS:").font(FontId::new(11.0, FontFamily::Monospace)).strong().color(p.text_secondary));
                        ui.label(RichText::new(&pos_expr).font(FontId::new(12.0, FontFamily::Monospace)).color(p.text_primary));
                    });
                    ui.separator();

                    let n = kmap.num_vars.clamp(2, 4);
                    let rows = 1usize << n;
                    ui.columns(2, |cols| {
                        crate::theme::section_header(&mut cols[0], tr.kmap_tt_heading, p.cyan);
                        egui::ScrollArea::vertical()
                            .id_source("truth_table_scroll")
                            .max_height(290.0)
                            .show(&mut cols[0], |ui| {
                                egui::Grid::new("truth_table_grid")
                                    .striped(true)
                                    .min_col_width(32.0)
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("#").font(FontId::new(11.0, FontFamily::Monospace)).strong());
                                        for i in 0..n {
                                            ui.add(
                                                egui::TextEdit::singleline(&mut kmap.var_names[i])
                                                    .desired_width(26.0)
                                                    .font(FontId::new(11.5, FontFamily::Monospace)),
                                            );
                                        }
                                        ui.add(
                                            egui::TextEdit::singleline(&mut kmap.output_name)
                                                .desired_width(30.0)
                                                .font(FontId::new(11.5, FontFamily::Monospace)),
                                        );
                                        ui.end_row();

                                        for m in 0..rows {
                                            ui.label(RichText::new(format!("{m:02}")).font(FontId::new(11.0, FontFamily::Monospace)).color(p.text_muted));
                                            for i in 0..n {
                                                let bit = (m >> (n - 1 - i)) & 1;
                                                ui.label(
                                                    RichText::new(format!("{bit}"))
                                                        .font(FontId::new(11.5, FontFamily::Monospace))
                                                        .color(if bit == 1 { p.green } else { p.text_secondary }),
                                                );
                                            }
                                            let cur = kmap.values[m];
                                            let (txt, fg, bg) = match cur {
                                                1 => (" 1 ", Color32::from_rgb(10, 14, 18), p.green),
                                                2 => (" X ", Color32::from_rgb(10, 14, 18), p.orange),
                                                _ => (" 0 ", p.text_secondary, p.bg_elevated),
                                            };
                                            let btn = egui::Button::new(
                                                RichText::new(txt)
                                                    .font(FontId::new(11.5, FontFamily::Monospace))
                                                    .strong()
                                                    .color(fg),
                                            )
                                            .fill(bg)
                                            .rounding(0.0);
                                            if ui.add(btn).clicked() {
                                                kmap.values[m] = (cur + 1) % 3;
                                            }
                                            ui.end_row();
                                        }
                                    });
                            });

                        crate::theme::section_header(&mut cols[1], tr.kmap_grid_heading, p.purple);
                        let row_bits = n / 2;
                        let col_bits = n - row_bits;
                        let r_gray = gray_code(row_bits);
                        let c_gray = gray_code(col_bits);
                        let row_vars: String = (0..row_bits)
                            .map(|i| kmap.effective_var_name(i))
                            .collect::<Vec<_>>()
                            .join("");
                        let col_vars: String = (row_bits..n)
                            .map(|i| kmap.effective_var_name(i))
                            .collect::<Vec<_>>()
                            .join("");

                        cols[1].label(
                            RichText::new(format!("{row_vars} \\ {col_vars}"))
                                .font(FontId::new(11.5, FontFamily::Monospace))
                                .strong()
                                .color(p.cyan),
                        );
                        egui::Grid::new("kmap_2d_grid")
                            .spacing([4.0, 4.0])
                            .min_col_width(46.0)
                            .show(&mut cols[1], |ui| {
                                ui.label("");
                                for &(_, clbl) in &c_gray {
                                    ui.label(RichText::new(clbl).font(FontId::new(11.5, FontFamily::Monospace)).strong().color(p.text_secondary));
                                }
                                ui.end_row();

                                for &(r_val, rlbl) in &r_gray {
                                    ui.label(RichText::new(rlbl).font(FontId::new(11.5, FontFamily::Monospace)).strong().color(p.text_secondary));
                                    for &(c_val, _) in &c_gray {
                                        let minterm = (r_val << col_bits) | c_val;
                                        let val = kmap.values[minterm];
                                        let txt = match val {
                                            1 => "1",
                                            2 => "X",
                                            _ => "0",
                                        };
                                        let covered =
                                            imps.iter().any(|imp| imp.covers(minterm, n));
                                        let (fg, bg, stroke) = if covered && val == 1 {
                                            (Color32::from_rgb(10, 14, 18), p.green, Stroke::new(1.2, p.cyan))
                                        } else if val == 1 {
                                            (p.green, p.bg_elevated, Stroke::new(1.0, p.border_primary))
                                        } else if val == 2 {
                                            (Color32::from_rgb(10, 14, 18), p.orange, Stroke::new(1.0, p.border_primary))
                                        } else {
                                            (p.text_muted, p.bg_secondary, Stroke::new(1.0, p.border_secondary))
                                        };
                                        let btn_text = RichText::new(format!("  {txt}  "))
                                            .font(FontId::new(14.0, FontFamily::Monospace))
                                            .strong()
                                            .color(fg);
                                        let btn = egui::Button::new(btn_text)
                                            .fill(bg)
                                            .stroke(stroke)
                                            .rounding(0.0);
                                        if ui
                                            .add_sized([48.0, 38.0], btn)
                                            .on_hover_text(format!("Minterm m{minterm}"))
                                            .clicked()
                                        {
                                            kmap.values[minterm] = (val + 1) % 3;
                                        }
                                    }
                                    ui.end_row();
                                }
                            });

                        cols[1].add_space(6.0);
                        for (gi, imp) in imps.iter().enumerate() {
                            cols[1].label(
                                RichText::new(format!(
                                    "▶ GROUP {:02}: {}",
                                    gi + 1,
                                    imp.format_sop_term(&kmap.var_names, n)
                                ))
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.purple),
                            );
                        }
                    });
                }

                if synthesize_clicked {
                    self.save_undo_snapshot();
                    let page_idx = self.sim.current_page;
                    self.sim.synthesize_kmap_to_page(&self.lib, page_idx, k_idx);
                    self.status_message =
                        "Synthesized K-Map minimal SOP logic circuit onto the active canvas page!"
                            .to_string();
                }
            });
        self.show_kmap_studio = open;
    }

    pub fn draw_memory_editor_modal(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        let mut open = self.show_memory_editor;
        egui::Window::new(tr.mem_window_title)
            .open(&mut open)
            .default_size([540.0, 390.0])
            .frame(crate::theme::window_frame(self.dark_mode))
            .show(ctx, |ui| {
                crate::theme::section_header(ui, "HEX MEMORY EDITOR // RAM / ROM DUMP", p.purple);

                let mut ram_gates: Vec<(usize, u32, String)> = Vec::new();
                for (p_idx, page) in self.sim.pages.iter().enumerate() {
                    for g in &page.gates {
                        if g.type_name.contains("RAM") || g.type_name.contains("ROM") {
                            ram_gates.push((
                                p_idx,
                                g.id,
                                format!("{} #{:02} ({})", g.type_name, g.id, page.name),
                            ));
                        }
                    }
                }

                if ram_gates.is_empty() {
                    ui.label(
                        RichText::new(tr.mem_empty_msg)
                            .font(FontId::new(11.0, FontFamily::Monospace))
                            .color(p.text_muted),
                    );
                    return;
                }

                let active_id = self
                    .memory_edit_gate_id
                    .unwrap_or(ram_gates[0].1);
                self.memory_edit_gate_id = Some(active_id);

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(tr.mem_target_device)
                            .font(FontId::new(10.5, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    for &(_, gid, ref label) in &ram_gates {
                        let is_active = active_id == gid;
                        if crate::theme::tool_button(ui, label, is_active, p.purple).clicked() {
                            self.memory_edit_gate_id = Some(gid);
                        }
                    }
                });

                ui.separator();

                if let Some(gate) = self
                    .sim
                    .pages
                    .iter_mut()
                    .flat_map(|page| page.gates.iter_mut())
                    .find(|g| g.id == active_id)
                {
                    ui.horizontal(|ui| {
                        if crate::theme::tool_button(ui, tr.mem_load_btn, false, p.cyan).clicked() {
                            if let Some(p_file) = rfd::FileDialog::new()
                                .add_filter("Memory Files (*.cdm, *.hex)", &["cdm", "hex"])
                                .pick_file()
                            {
                                if let Ok(txt) = std::fs::read_to_string(&p_file) {
                                    gate.memory = import_cdm_or_hex(&txt);
                                }
                            }
                        }
                        if crate::theme::tool_button(ui, tr.mem_save_btn, false, p.cyan).clicked() {
                            if let Some(p_file) = rfd::FileDialog::new()
                                .add_filter("CEDAR Memory File (*.cdm)", &["cdm"])
                                .set_file_name("memory.cdm")
                                .save_file()
                            {
                                let _ = std::fs::write(&p_file, export_cdm(&gate.memory));
                            }
                        }
                        if crate::theme::tool_button(ui, tr.mem_clear_btn, false, p.red).clicked() {
                            gate.memory.clear();
                        }
                    });

                    ui.add_space(6.0);
                    egui::Grid::new("ram_hex_grid")
                        .striped(true)
                        .min_col_width(36.0)
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("OFFSET")
                                    .font(FontId::new(11.0, FontFamily::Monospace))
                                    .strong()
                                    .color(p.cyan),
                            );
                            for col in 0..8 {
                                ui.label(
                                    RichText::new(format!("+{col:X}"))
                                        .font(FontId::new(11.0, FontFamily::Monospace))
                                        .strong()
                                        .color(p.text_secondary),
                                );
                            }
                            ui.end_row();

                            for row in 0..4u16 {
                                let base = row * 8;
                                ui.label(
                                    RichText::new(format!("0x{base:02X}:"))
                                        .font(FontId::new(11.5, FontFamily::Monospace))
                                        .color(p.orange),
                                );
                                for col in 0..8u16 {
                                    let addr = base + col;
                                    let val = gate.memory.get(&addr).copied().unwrap_or(0);
                                    let mut buf = format!("{val:02X}");
                                    let resp = ui.add(
                                        egui::TextEdit::singleline(&mut buf)
                                            .desired_width(28.0)
                                            .font(FontId::new(12.0, FontFamily::Monospace)),
                                    );
                                    if resp.changed() {
                                        if let Ok(parsed) = u16::from_str_radix(buf.trim(), 16) {
                                            gate.memory.insert(addr, parsed & 0xFF);
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        });
                }
            });
        self.show_memory_editor = open;
    }

    pub fn load_spec_into_studio(&mut self, spec: &CustomComponentSpec) {
        self.custom_ic_name = spec.name.clone();
        self.custom_ic_label = spec.chip_label.clone();
        self.custom_ic_desc = spec.description.clone();
        self.custom_ic_inputs = spec.inputs.join(", ");
        self.custom_ic_outputs = spec.outputs.join(", ");
        self.custom_ic_state_bits = spec.state_bits.join(", ");
        self.custom_ic_clock_pin = spec.clock_pin.clone().unwrap_or_default();
        self.custom_ic_reset_pin = spec.reset_pin.clone().unwrap_or_default();
        let mut eqs = Vec::new();
        for sb in &spec.state_bits {
            let e = spec.state_equations.get(sb).cloned().unwrap_or_default();
            eqs.push((sb.clone(), e));
        }
        for out in &spec.outputs {
            let e = spec.equations.get(out).cloned().unwrap_or_default();
            eqs.push((out.clone(), e));
        }
        self.custom_ic_equations = eqs;
        if let Ok(pretty) = serde_json::to_string_pretty(spec) {
            self.custom_ic_json_buf = pretty;
        }
    }

    pub fn build_spec_from_studio_form(&self) -> CustomComponentSpec {
        let parse_csv = |s: &str| -> Vec<String> {
            s.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        };
        let inputs = parse_csv(&self.custom_ic_inputs);
        let outputs = parse_csv(&self.custom_ic_outputs);
        let state_bits = parse_csv(&self.custom_ic_state_bits);

        let mut equations = HashMap::new();
        let mut state_equations = HashMap::new();
        for (k, v) in &self.custom_ic_equations {
            if state_bits.iter().any(|sb| sb.eq_ignore_ascii_case(k)) {
                state_equations.insert(k.clone(), v.clone());
            } else {
                equations.insert(k.clone(), v.clone());
            }
        }

        CustomComponentSpec {
            id: String::new(),
            name: self.custom_ic_name.trim().to_string(),
            chip_label: self.custom_ic_label.trim().to_string(),
            description: self.custom_ic_desc.trim().to_string(),
            inputs,
            outputs,
            state_bits,
            clock_pin: if self.custom_ic_clock_pin.trim().is_empty() {
                None
            } else {
                Some(self.custom_ic_clock_pin.trim().to_string())
            },
            clock_edge: Some("rising".to_string()),
            reset_pin: if self.custom_ic_reset_pin.trim().is_empty() {
                None
            } else {
                Some(self.custom_ic_reset_pin.trim().to_string())
            },
            equations,
            state_equations,
            truth_table: HashMap::new(),
        }
    }

    pub fn draw_custom_ic_studio_window(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        let mut open = self.show_custom_ic_studio;
        egui::Window::new(tr.custom_window_title)
            .open(&mut open)
            .default_size([740.0, 560.0])
            .resizable(true)
            .frame(crate::theme::window_frame(self.dark_mode))
            .show(ctx, |ui| {
                crate::theme::section_header(ui, "CUSTOM COMPONENT STUDIO // LOGIC & HARDWARE DEFINITION", p.orange);

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!("{} //", tr.custom_templates_lbl.to_uppercase()))
                            .font(FontId::new(10.5, FontFamily::Monospace))
                            .color(p.cyan),
                    );
                    for tpl in builtin_custom_templates() {
                        if crate::theme::tool_button(ui, &tpl.name, false, p.cyan).clicked() {
                            self.load_spec_into_studio(&tpl);
                            self.custom_ic_feedback =
                                Some((false, format!("Loaded template '{}'.", tpl.name)));
                        }
                    }
                });

                ui.separator();

                ui.horizontal(|ui| {
                    if crate::theme::tool_button(ui, tr.custom_tab_visual, self.custom_ic_tab == 0, p.cyan).clicked() {
                        self.custom_ic_tab = 0;
                    }
                    if crate::theme::tool_button(ui, tr.custom_tab_json, self.custom_ic_tab == 1, p.purple).clicked() {
                        let spec = self.build_spec_from_studio_form();
                        if let Ok(pretty) = serde_json::to_string_pretty(&spec) {
                            self.custom_ic_json_buf = pretty;
                        }
                        self.custom_ic_tab = 1;
                    }
                });

                ui.separator();

                if self.custom_ic_tab == 0 {
                    egui::Grid::new("custom_ic_meta_grid")
                        .num_columns(2)
                        .spacing([10.0, 6.0])
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(tr.custom_comp_name)
                                    .font(FontId::new(10.5, FontFamily::Monospace))
                                    .color(p.text_secondary),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_ic_name)
                                    .desired_width(340.0),
                            );
                            ui.end_row();

                            ui.label(
                                RichText::new(tr.custom_chip_label)
                                    .font(FontId::new(10.5, FontFamily::Monospace))
                                    .color(p.text_secondary),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_ic_label)
                                    .desired_width(120.0),
                            );
                            ui.end_row();

                            ui.label(
                                RichText::new(tr.custom_description)
                                    .font(FontId::new(10.5, FontFamily::Monospace))
                                    .color(p.text_secondary),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_ic_desc)
                                    .desired_width(440.0),
                            );
                            ui.end_row();

                            ui.label(
                                RichText::new(tr.custom_inputs)
                                    .font(FontId::new(10.5, FontFamily::Monospace))
                                    .color(p.cyan),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_ic_inputs)
                                    .desired_width(340.0),
                            );
                            ui.end_row();

                            ui.label(
                                RichText::new(tr.custom_outputs)
                                    .font(FontId::new(10.5, FontFamily::Monospace))
                                    .color(p.green),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_ic_outputs)
                                    .desired_width(340.0),
                            );
                            ui.end_row();

                            ui.label(
                                RichText::new(tr.custom_state_bits)
                                    .font(FontId::new(10.5, FontFamily::Monospace))
                                    .color(p.purple),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.custom_ic_state_bits)
                                    .desired_width(220.0)
                                    .hint_text("e.g. Q0, Q1"),
                            );
                            ui.end_row();

                            if !self.custom_ic_state_bits.trim().is_empty() {
                                ui.label(
                                    RichText::new("CLOCK / RESET")
                                        .font(FontId::new(10.5, FontFamily::Monospace))
                                        .color(p.orange),
                                );
                                ui.horizontal(|ui| {
                                    ui.label("CLK:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.custom_ic_clock_pin)
                                            .desired_width(70.0),
                                    );
                                    ui.label("RST:");
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.custom_ic_reset_pin)
                                            .desired_width(70.0),
                                    );
                                });
                                ui.end_row();
                            }
                        });

                    let desired_targets: Vec<(String, bool)> = self
                        .custom_ic_state_bits
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .map(|s| (s, true))
                        .chain(
                            self.custom_ic_outputs
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .map(|s| (s, false)),
                        )
                        .collect();

                    let mut synced_eqs = Vec::with_capacity(desired_targets.len());
                    for (t_name, _) in &desired_targets {
                        let existing = self
                            .custom_ic_equations
                            .iter()
                            .find(|(k, _)| k.eq_ignore_ascii_case(t_name))
                            .map(|(_, v)| v.clone())
                            .unwrap_or_else(|| "0".to_string());
                        synced_eqs.push((t_name.clone(), existing));
                    }
                    self.custom_ic_equations = synced_eqs;

                    ui.add_space(6.0);
                    crate::theme::section_header(ui, tr.custom_equations_lbl, p.cyan);
                    egui::ScrollArea::vertical()
                        .id_source("custom_ic_eq_scroll")
                        .max_height(150.0)
                        .show(ui, |ui| {
                            egui::Grid::new("custom_ic_eq_grid")
                                .num_columns(2)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    for ((pin_name, expr), (_, is_state)) in
                                        self.custom_ic_equations.iter_mut().zip(desired_targets.iter())
                                    {
                                        ui.label(
                                            RichText::new(if *is_state {
                                                format!("{pin_name}(next) =")
                                            } else {
                                                format!("{pin_name} =")
                                            })
                                            .font(FontId::new(11.5, FontFamily::Monospace))
                                            .strong()
                                            .color(if *is_state { p.purple } else { p.cyan }),
                                        );
                                        ui.add(
                                            egui::TextEdit::singleline(expr)
                                                .desired_width(480.0)
                                                .font(FontId::new(12.0, FontFamily::Monospace)),
                                        );
                                        ui.end_row();
                                    }
                                });
                        });

                    ui.add_space(8.0);
                    ui.horizontal_wrapped(|ui| {
                        if crate::theme::primary_button(ui, tr.custom_register_btn, p.cyan).clicked() {
                            let spec = self.build_spec_from_studio_form();
                            match self.lib.register_custom_component(spec.clone()) {
                                Ok(id) => {
                                    if let Some(idx) = self
                                        .lib
                                        .categories
                                        .iter()
                                        .position(|c| c.name.starts_with("11 - Custom"))
                                    {
                                        self.active_category = idx;
                                    }
                                    self.custom_ic_feedback = Some((
                                        false,
                                        format!("Registered '{}' ({id})!", spec.name),
                                    ));
                                }
                                Err(e) => {
                                    self.custom_ic_feedback = Some((true, e));
                                }
                            }
                        }
                        if crate::theme::primary_button(ui, tr.custom_register_place_btn, p.green).clicked() {
                            let spec = self.build_spec_from_studio_form();
                            match self.lib.register_custom_component(spec.clone()) {
                                Ok(id) => {
                                    if let Some(idx) = self
                                        .lib
                                        .categories
                                        .iter()
                                        .position(|c| c.name.starts_with("11 - Custom"))
                                    {
                                        self.active_category = idx;
                                    }
                                    self.placing_gate_type = Some(id.clone());
                                    self.custom_ic_feedback = Some((
                                        false,
                                        format!(
                                            "Registered '{}' ({id}) — click on the canvas to place it!",
                                            spec.name
                                        ),
                                    ));
                                }
                                Err(e) => {
                                    self.custom_ic_feedback = Some((true, e));
                                }
                            }
                        }
                        if crate::theme::tool_button(ui, tr.custom_export_json_btn, false, p.orange).clicked() {
                            let spec = self.build_spec_from_studio_form();
                            if let Ok(norm) = spec.normalized() {
                                if let Ok(json) = serde_json::to_string_pretty(&norm) {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter(
                                            "LogicCraft Component (*.json, *.lccomp)",
                                            &["json", "lccomp"],
                                        )
                                        .set_file_name(&format!("{}.json", norm.id.to_lowercase()))
                                        .save_file()
                                    {
                                        let _ = std::fs::write(&path, json);
                                        self.custom_ic_feedback = Some((
                                            false,
                                            format!("Saved custom component to {}", path.display()),
                                        ));
                                    }
                                }
                            }
                        }
                    });
                } else {
                    ui.label(
                        RichText::new(tr.custom_ai_hint)
                            .font(FontId::new(10.5, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    ui.horizontal_wrapped(|ui| {
                        if crate::theme::primary_button(ui, tr.custom_copy_ai_prompt, p.cyan).clicked() {
                            ctx.copy_text(ai_prompt_template().to_string());
                            self.custom_ic_feedback = Some((
                                false,
                                "Copied AI Prompt Template to clipboard!".to_string(),
                            ));
                        }
                        if crate::theme::tool_button(ui, tr.custom_import_file, false, p.purple).clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    "LogicCraft Component (*.json, *.lccomp)",
                                    &["json", "lccomp"],
                                )
                                .pick_file()
                            {
                                if let Ok(txt) = std::fs::read_to_string(&path) {
                                    self.custom_ic_json_buf = txt.clone();
                                    match serde_json::from_str::<CustomComponentSpec>(&txt) {
                                        Ok(spec) => {
                                            self.load_spec_into_studio(&spec);
                                            if let Ok(id) =
                                                 self.lib.register_custom_component(spec.clone())
                                            {
                                                if let Some(idx) = self
                                                    .lib
                                                    .categories
                                                    .iter()
                                                    .position(|c| c.name.starts_with("11 - Custom"))
                                                {
                                                    self.active_category = idx;
                                                }
                                                self.placing_gate_type = Some(id.clone());
                                                self.custom_ic_feedback = Some((
                                                    false,
                                                    format!(
                                                        "Imported & registered '{}' ({id})!",
                                                        spec.name
                                                    ),
                                                ));
                                            }
                                        }
                                        Err(e) => {
                                            self.custom_ic_feedback =
                                                Some((true, format!("JSON parse error: {e}")));
                                        }
                                    }
                                }
                            }
                        }
                        if crate::theme::primary_button(ui, tr.custom_load_json_below, p.green).clicked() {
                            match serde_json::from_str::<CustomComponentSpec>(
                                &self.custom_ic_json_buf,
                            ) {
                                Ok(spec) => {
                                    self.load_spec_into_studio(&spec);
                                    match self.lib.register_custom_component(spec.clone()) {
                                        Ok(id) => {
                                            if let Some(idx) = self
                                                .lib
                                                .categories
                                                .iter()
                                                .position(|c| c.name.starts_with("11 - Custom"))
                                            {
                                                self.active_category = idx;
                                            }
                                            self.placing_gate_type = Some(id.clone());
                                            self.custom_ic_feedback = Some((
                                                false,
                                                format!(
                                                    "Registered '{}' ({id}) — click on the canvas to place it!",
                                                    spec.name
                                                ),
                                            ));
                                        }
                                        Err(e) => {
                                            self.custom_ic_feedback = Some((true, e));
                                        }
                                    }
                                }
                                Err(e) => {
                                    self.custom_ic_feedback =
                                        Some((true, format!("Invalid JSON: {e}")));
                                }
                            }
                        }
                    });

                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_source("custom_ic_json_scroll")
                        .max_height(310.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut self.custom_ic_json_buf)
                                    .font(FontId::new(11.5, FontFamily::Monospace))
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(16),
                            );
                        });
                }

                if let Some((is_err, ref msg)) = self.custom_ic_feedback {
                    ui.separator();
                    crate::theme::status_pill(ui, msg, if is_err { p.red } else { p.green });
                }
            });
        self.show_custom_ic_studio = open;
    }

    pub fn draw_about_modal(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        let mut open = self.show_about_dialog;
        egui::Window::new(tr.about_window_title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .frame(crate::theme::window_frame(self.dark_mode))
            .show(ctx, |ui| {
                crate::theme::section_header(ui, "LOGICCRAFT // SYSTEM SPECIFICATION", p.cyan);
                ui.label(
                    RichText::new(tr.about_subtitle)
                        .font(FontId::new(11.0, FontFamily::Monospace))
                        .color(p.text_secondary),
                );
                ui.separator();
                let bullets: &[&str] = match self.locale {
                    crate::i18n::Locale::Pt => &[
                        "• 11 BIBLIOTECAS DE COMPONENTES (PORTAS, LIGAÇÕES, E/S, MUX/DESCOD, ARITMÉTICA, FLIP-FLOPS, REGISTOS, MEMÓRIA, CI, PERSONALIZADOS)",
                        "• LIGAÇÕES DE DUPLO ÂMBITO: TAGS 'FROM' & 'TO' COM ÂMBITO PARTILHADO (GLOBAL) E LOCAL (PÁGINA)",
                        "• MOTOR DE FIOS ORTOGONAIS: PONTOS DE SOLDA, REORDENAÇÃO DINÂMICA DE SEGMENTOS E ARRASTO EM GRUPO",
                        "• ANÁLISE TÉCNICA: ANALISADOR LÓGICO / OSCILOSCÓPIO MULTICANAL COM 180 AMOSTRAS",
                        "• SÍNTESE LÓGICA: TABELA DE VERDADE E MAPA DE KARNAUGH (QUINE-MCCLUSKEY) COM CARTÕES NO ESQUEMA",
                        "• ESTÚDIO DE COMPONENTES E IA: MOTOR DE HARDWARE JSON / LPOS COM VERIFICAÇÃO AUTOMÁTICA",
                        "• IDIOMAS SUPORTADOS: EN (INGLÊS), PT (PORTUGUÊS DE PORTUGAL), ES (ESPANHOL), FR (FRANCÊS), DE (ALEMÃO)",
                    ],
                    _ => &[
                        "• 11 COMPONENT LIBRARIES (GATES, CONNECTORS, I/O, MUX/DEC, ARITHMETIC, FF, REG, MEMORY, CHIPS, CUSTOM)",
                        "• DUAL-SCOPE JUNCTIONS: 'FROM' & 'TO' TAGS WITH SHARED (CROSS-PAGE) AND LOCAL (PAGE-ONLY) SCOPES",
                        "• ORTHOGONAL WIRE ENGINE: SOLDER JUNCTIONS, DYNAMIC WIRE SEGMENT RE-ROUTING, GROUP DRAG",
                        "• TECHNICAL ANALYSIS: 180-SAMPLE MULTI-CHANNEL LOGIC ANALYZER / OSCILLOSCOPE",
                        "• LOGIC SYNTHESIS: TRUTH TABLE & QUINE-MCCLUSKEY KARNAUGH MAP (K-MAP) SOLVER WITH CANVAS CARDS",
                        "• CUSTOM IC & AI STUDIO: JSON / LPOS HARDWARE SPECIFICATION ENGINE WITH AUTO-VERIFICATION",
                        "• RUNTIME LOCALIZATION: EN (ENGLISH), PT (PORTUGUÊS), ES (ESPAÑOL), FR (FRANÇAIS), DE (DEUTSCH)",
                    ],
                };
                for bullet in bullets {
                    ui.label(
                        RichText::new(*bullet)
                            .font(FontId::new(10.5, FontFamily::Monospace))
                            .color(p.text_primary),
                    );
                }
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Build: v1.0.1 // x86_64")
                            .font(FontId::new(11.0, FontFamily::Monospace))
                            .color(p.text_secondary),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(
                                RichText::new(format!("🔄 {}", tr.update_menu_item))
                                    .font(FontId::new(11.0, FontFamily::Monospace))
                                    .color(p.cyan),
                            )
                            .clicked()
                        {
                            self.show_update_dialog = true;
                        }
                    });
                });
            });
        self.show_about_dialog = open;
    }

    pub fn draw_update_modal(&mut self, ctx: &egui::Context) {
        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        let mut open = self.show_update_dialog;
        egui::Window::new(tr.update_window_title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .frame(crate::theme::window_frame(self.dark_mode))
            .show(ctx, |ui| {
                crate::theme::section_header(ui, "LOGICCRAFT CAD // NATIVE UPDATER", p.cyan);
                ui.label(
                    RichText::new(tr.update_status_latest)
                        .font(FontId::new(11.5, FontFamily::Monospace))
                        .color(p.green),
                );
                ui.separator();

                egui::Frame::none()
                    .fill(p.bg_secondary)
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(Stroke::new(1.0, p.border_secondary))
                    .show(ui, |ui| {
                        let (v_label, ch_label, arch_label, set_label) = match self.locale {
                            crate::i18n::Locale::Pt => (
                                "VERSÃO ATUAL INSTALADA: 1.0.1",
                                "CANAL DE ATUALIZAÇÃO: ESTÁVEL (OFICIAL)",
                                "ARQUITETURA ALVO: x86_64 WINDOWS (SUBSISTEMA GUI)",
                                "PERSISTÊNCIA DE DEFINIÇÕES: ATIVA (settings.json)",
                            ),
                            _ => (
                                "CURRENT INSTALLED VERSION: 1.0.1",
                                "RELEASE CHANNEL: STABLE (OFFICIAL)",
                                "TARGET ARCHITECTURE: x86_64 WINDOWS (GUI SUBSYSTEM)",
                                "SETTINGS PERSISTENCE: ENABLED (settings.json)",
                            ),
                        };
                        ui.label(
                            RichText::new(v_label)
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.text_primary),
                        );
                        ui.label(
                            RichText::new(ch_label)
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        );
                        ui.label(
                            RichText::new(arch_label)
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        );
                        ui.label(
                            RichText::new(set_label)
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        );
                    });

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                    if ui
                        .button(
                            RichText::new(tr.update_check_btn)
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.cyan),
                        )
                        .clicked()
                    {
                        self.status_message = format!("LogicCraft CAD v1.0.1: {}", tr.update_status_latest);
                    }

                    if ui
                        .button(
                            RichText::new(tr.update_run_btn)
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.orange),
                        )
                        .clicked()
                    {
                        #[cfg(windows)]
                        {
                            let mut updater_path = std::env::current_exe()
                                .ok()
                                .and_then(|p| p.parent().map(|d| d.join("logiccraft-updater.exe")));
                            if updater_path.as_ref().map(|p| !p.exists()).unwrap_or(true) {
                                if let Ok(dir) = std::env::var("LOCALAPPDATA") {
                                    let p = std::path::PathBuf::from(dir)
                                        .join("Programs")
                                        .join("LogicCraft")
                                        .join("logiccraft-updater.exe");
                                    if p.exists() {
                                        updater_path = Some(p);
                                    }
                                }
                            }
                            if let Some(exe) = updater_path {
                                if exe.exists() {
                                    let _ = std::process::Command::new(exe).spawn();
                                } else {
                                    let _ = std::process::Command::new("powershell")
                                        .args(["-ExecutionPolicy", "Bypass", "-File", "packaging\\windows\\update.ps1"])
                                        .spawn();
                                }
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(
                            RichText::new("🌐 GitHub Releases")
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        )
                        .clicked()
                    {
                        #[cfg(windows)]
                        let _ = std::process::Command::new("cmd")
                            .args(["/c", "start", "", "https://github.com/PMTSantos/LogicCraft/releases"])
                            .spawn();
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open")
                            .arg("https://github.com/PMTSantos/LogicCraft/releases")
                            .spawn();
                        #[cfg(target_os = "linux")]
                        let _ = std::process::Command::new("xdg-open")
                            .arg("https://github.com/PMTSantos/LogicCraft/releases")
                            .spawn();
                    }

                    if ui
                        .button(
                            RichText::new("📁 Install Directory")
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        )
                        .clicked()
                    {
                        let app_dir = std::env::current_exe()
                            .ok()
                            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                            .unwrap_or_else(|| std::path::PathBuf::from("."));
                        #[cfg(windows)]
                        let _ = std::process::Command::new("explorer").arg(app_dir).spawn();
                        #[cfg(target_os = "macos")]
                        let _ = std::process::Command::new("open").arg(app_dir).spawn();
                        #[cfg(target_os = "linux")]
                        let _ = std::process::Command::new("xdg-open").arg(app_dir).spawn();
                    }
                });
            });
        self.show_update_dialog = open;
    }
}
