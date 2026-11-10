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
                        if g.is_memory_device() {
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
                                    self.placing_multi_active = false;
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
                                    self.placing_multi_active = false;
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
                                    self.placing_multi_active = false;
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
                        RichText::new(format!("Build: v{} // {}", env!("CARGO_PKG_VERSION"), std::env::consts::ARCH))
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
                            self.trigger_update_check(ctx, true);
                        }
                    });
                });
            });
        self.show_about_dialog = open;
    }

    pub fn draw_update_modal(&mut self, ctx: &egui::Context) {
        if matches!(self.update_status, crate::updater::UpdateStatus::Idle) {
            self.trigger_update_check(ctx, false);
        }

        let tr = self.locale.tr();
        let p = crate::theme::palette(self.dark_mode);
        let pkg_ver = env!("CARGO_PKG_VERSION");
        let is_pt = matches!(self.locale, crate::i18n::Locale::Pt);

        let mut open = self.show_update_dialog;
        let mut action_check = false;
        let mut action_install: Option<crate::updater::GitHubRelease> = None;
        let mut action_standalone = false;
        let mut action_restart = false;
        let mut action_close = false;

        let status_snapshot = self.update_status.clone();

        egui::Window::new(tr.update_window_title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .min_width(480.0)
            .frame(crate::theme::window_frame(self.dark_mode))
            .show(ctx, |ui| {
                crate::theme::section_header(ui, "LOGICCRAFT CAD // GITHUB AUTO-UPDATER", p.cyan);

                // Live Status Banner based on GitHub API state
                match &status_snapshot {
                    crate::updater::UpdateStatus::Idle
                    | crate::updater::UpdateStatus::Checking => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            let msg = if is_pt {
                                "A contactar o GitHub Releases (PMTSantos/LogicCraft)..."
                            } else {
                                "Checking GitHub Releases (PMTSantos/LogicCraft)..."
                            };
                            ui.label(
                                RichText::new(msg)
                                    .font(FontId::new(11.5, FontFamily::Monospace))
                                    .color(p.cyan),
                            );
                        });
                    }
                    crate::updater::UpdateStatus::UpToDate(rel) => {
                        let msg = if is_pt {
                            format!(
                                "✔ ATUALIZADO — A sua instalação (v{pkg_ver}) corresponde à última versão no GitHub (v{}).",
                                rel.version
                            )
                        } else {
                            format!(
                                "✔ UP TO DATE — Installed v{pkg_ver} matches latest GitHub release (v{}).",
                                rel.version
                            )
                        };
                        ui.label(
                            RichText::new(msg)
                                .font(FontId::new(11.5, FontFamily::Monospace))
                                .strong()
                                .color(p.green),
                        );
                    }
                    crate::updater::UpdateStatus::UpdateAvailable(rel) => {
                        let banner = if is_pt {
                            format!(
                                "🚀 NOVA ATUALIZAÇÃO DISPONÍVEL NO GITHUB: v{pkg_ver} ➔ v{}",
                                rel.version
                            )
                        } else {
                            format!(
                                "🚀 NEW UPDATE AVAILABLE ON GITHUB: v{pkg_ver} ➔ v{}",
                                rel.version
                            )
                        };
                        ui.label(
                            RichText::new(banner)
                                .font(FontId::new(12.0, FontFamily::Monospace))
                                .strong()
                                .color(p.orange),
                        );
                        let perm_txt = if is_pt {
                            "Deseja transferir e instalar automaticamente a nova versão agora?"
                        } else {
                            "Would you like to automatically download and install the new version now?"
                        };
                        ui.label(
                            RichText::new(perm_txt)
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.text_primary),
                        );
                    }
                    crate::updater::UpdateStatus::Downloading { release, progress } => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(
                                RichText::new(format!("⬇ [v{}] {}", release.version, progress))
                                    .font(FontId::new(11.5, FontFamily::Monospace))
                                    .strong()
                                    .color(p.cyan),
                            );
                        });
                    }
                    crate::updater::UpdateStatus::ReadyToRestart { version, message } => {
                        ui.label(
                            RichText::new(format!("✔ [v{version}] {message}"))
                                .font(FontId::new(11.5, FontFamily::Monospace))
                                .strong()
                                .color(p.green),
                        );
                    }
                    crate::updater::UpdateStatus::Error(err) => {
                        ui.label(
                            RichText::new(format!("⚠ {err}"))
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.red),
                        );
                    }
                }

                ui.separator();

                let remote_rel = match &status_snapshot {
                    crate::updater::UpdateStatus::UpToDate(r)
                    | crate::updater::UpdateStatus::UpdateAvailable(r)
                    | crate::updater::UpdateStatus::Downloading { release: r, .. } => {
                        Some(r.clone())
                    }
                    _ => None,
                };

                egui::Frame::none()
                    .fill(p.bg_secondary)
                    .inner_margin(egui::Margin::same(8.0))
                    .stroke(Stroke::new(1.0, p.border_secondary))
                    .show(ui, |ui| {
                        let arch_desc = if cfg!(target_os = "windows") {
                            "x86_64 WINDOWS (GUI SUBSYSTEM)"
                        } else if cfg!(target_os = "macos") {
                            "macOS (UNIVERSAL / COCOA)"
                        } else if cfg!(target_os = "linux") {
                            "LINUX (X11 / WAYLAND)"
                        } else {
                            "CROSS-PLATFORM"
                        };
                        let gh_ver_str = match &remote_rel {
                            Some(r) if !r.published_at.is_empty() => {
                                format!("v{} ({})", r.version, r.published_at)
                            }
                            Some(r) => format!("v{}", r.version),
                            None => {
                                if is_pt {
                                    "A verificar...".to_string()
                                } else {
                                    "Checking...".to_string()
                                }
                            }
                        };
                        let (v_label, gh_label, arch_label, set_label) = if is_pt {
                            (
                                format!("VERSÃO ATUAL INSTALADA:   v{pkg_ver}"),
                                format!("ÚLTIMA VERSÃO NO GITHUB:  {gh_ver_str}"),
                                format!("ARQUITETURA ALVO:         {arch_desc}"),
                                "PERSISTÊNCIA DEFINIÇÕES:  ATIVA (settings.json)".to_string(),
                            )
                        } else {
                            (
                                format!("CURRENT INSTALLED VERSION: v{pkg_ver}"),
                                format!("LATEST GITHUB RELEASE:     {gh_ver_str}"),
                                format!("TARGET ARCHITECTURE:       {arch_desc}"),
                                "SETTINGS PERSISTENCE:      ENABLED (settings.json)".to_string(),
                            )
                        };
                        ui.label(
                            RichText::new(v_label)
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.text_primary),
                        );
                        ui.label(
                            RichText::new(gh_label)
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .strong()
                                .color(p.cyan),
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

                // Show release notes if available
                if let Some(ref rel) = remote_rel {
                    if !rel.body.is_empty() {
                        ui.add_space(4.0);
                        let notes_hdr = if is_pt {
                            format!("NOTAS DE LANÇAMENTO // {}", rel.name.to_uppercase())
                        } else {
                            format!("RELEASE NOTES // {}", rel.name.to_uppercase())
                        };
                        ui.label(
                            RichText::new(notes_hdr)
                                .font(FontId::new(10.0, FontFamily::Monospace))
                                .strong()
                                .color(p.text_secondary),
                        );
                        egui::Frame::none()
                            .fill(p.bg_elevated)
                            .inner_margin(egui::Margin::same(6.0))
                            .stroke(Stroke::new(1.0, p.border_secondary))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_source("update_release_notes_scroll")
                                    .max_height(110.0)
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new(&rel.body)
                                                .font(FontId::new(10.5, FontFamily::Monospace))
                                                .color(p.text_primary),
                                        );
                                    });
                            });
                    }
                }

                ui.add_space(8.0);

                // Prominent 1-Click Permission Banner when an update is available
                if let crate::updater::UpdateStatus::UpdateAvailable(ref rel) = status_snapshot {
                    ui.horizontal(|ui| {
                        let install_lbl = if is_pt {
                            format!("⬇ Transferir e Instalar v{} Agora (Automático)", rel.version)
                        } else {
                            format!("⬇ Download & Install v{} Now (Automatic)", rel.version)
                        };
                        let btn = egui::Button::new(
                            RichText::new(install_lbl)
                                .font(FontId::new(11.5, FontFamily::Monospace))
                                .strong()
                                .color(Color32::from_rgb(10, 14, 18)),
                        )
                        .fill(p.green)
                        .rounding(0.0);
                        if ui.add_sized([330.0, 28.0], btn).clicked() {
                            action_install = Some(rel.clone());
                        }

                        let later_lbl = if is_pt { "Mais Tarde" } else { "Later" };
                        if ui
                            .add_sized(
                                [100.0, 28.0],
                                egui::Button::new(
                                    RichText::new(later_lbl)
                                        .font(FontId::new(11.0, FontFamily::Monospace))
                                        .color(p.text_secondary),
                                )
                                .rounding(0.0),
                            )
                            .clicked()
                        {
                            action_close = true;
                        }
                    });
                    ui.add_space(4.0);
                } else if let crate::updater::UpdateStatus::ReadyToRestart { ref version, .. } =
                    status_snapshot
                {
                    let restart_lbl = if is_pt {
                        format!("🚀 Reiniciar LogicCraft Agora (v{version})")
                    } else {
                        format!("🚀 Restart LogicCraft Now (v{version})")
                    };
                    let btn = egui::Button::new(
                        RichText::new(restart_lbl)
                            .font(FontId::new(11.5, FontFamily::Monospace))
                            .strong()
                            .color(Color32::from_rgb(10, 14, 18)),
                    )
                    .fill(p.green)
                    .rounding(0.0);
                    if ui.add_sized([340.0, 28.0], btn).clicked() {
                        action_restart = true;
                    }
                    ui.add_space(4.0);
                }

                let is_busy = matches!(
                    status_snapshot,
                    crate::updater::UpdateStatus::Checking
                        | crate::updater::UpdateStatus::Downloading { .. }
                );

                ui.horizontal(|ui| {
                    ui.add_enabled_ui(!is_busy, |ui| {
                        if ui
                            .button(
                                RichText::new(tr.update_check_btn)
                                    .font(FontId::new(11.0, FontFamily::Monospace))
                                    .color(p.cyan),
                            )
                            .clicked()
                        {
                            action_check = true;
                        }

                        if ui
                            .button(
                                RichText::new(tr.update_run_btn)
                                    .font(FontId::new(11.0, FontFamily::Monospace))
                                    .color(p.orange),
                            )
                            .clicked()
                        {
                            if let Some(ref rel) = remote_rel {
                                action_install = Some(rel.clone());
                            } else {
                                action_standalone = true;
                            }
                        }
                    });
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let standalone_lbl = if is_pt {
                        "🖥 Abrir Utilitário Consola (logiccraft-updater.exe)"
                    } else {
                        "🖥 Launch Console Updater (logiccraft-updater.exe)"
                    };
                    if ui
                        .button(
                            RichText::new(standalone_lbl)
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        )
                        .clicked()
                    {
                        action_standalone = true;
                    }

                    if ui
                        .button(
                            RichText::new("🌐 GitHub Releases")
                                .font(FontId::new(10.5, FontFamily::Monospace))
                                .color(p.text_secondary),
                        )
                        .clicked()
                    {
                        crate::updater::open_url("https://github.com/PMTSantos/LogicCraft/releases");
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

        if action_check {
            self.trigger_update_check(ctx, true);
        }
        if let Some(rel) = action_install {
            self.trigger_install_update(ctx, rel);
        }
        if action_standalone {
            if !crate::updater::launch_standalone_updater() {
                if let Some(rel) = match &self.update_status {
                    crate::updater::UpdateStatus::UpToDate(r)
                    | crate::updater::UpdateStatus::UpdateAvailable(r) => Some(r.clone()),
                    _ => None,
                } {
                    self.trigger_install_update(ctx, rel);
                } else {
                    crate::updater::open_url(
                        "https://github.com/PMTSantos/LogicCraft/releases/latest",
                    );
                }
            }
        }
        if action_restart {
            crate::updater::restart_application();
        }
        if action_close {
            open = false;
        }
        self.show_update_dialog = open;
    }

    /// Floating Programmable CPU Studio, Integrated Assembler & Real-Time Debugger Window.
    pub fn draw_cpu_studio_window(&mut self, ctx: &egui::Context) {
        let p = crate::theme::palette(self.dark_mode);
        let mut open = self.show_cpu_studio;

        egui::Window::new("🖥 CPU & Assembly Studio — Multi-Architecture Debugger & MMIO")
            .open(&mut open)
            .default_size([980.0, 650.0])
            .min_width(780.0)
            .min_height(480.0)
            .resizable(true)
            .frame(crate::theme::window_frame(self.dark_mode))
            .show(ctx, |ui| {
                // -----------------------------------------------------------------
                // 1. TOP SIMULATION & ARCHITECTURE CONTROL BAR
                // -----------------------------------------------------------------
                crate::theme::section_header(
                    ui,
                    "CPU CORE ARCHITECTURE & REAL-TIME EXECUTION CONTROLS",
                    p.cyan,
                );

                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;

                    // CPU Architecture Selector (Dropdown + quick badge)
                    ui.label(
                        RichText::new("ARCH //")
                            .font(FontId::new(10.5, FontFamily::Monospace))
                            .strong()
                            .color(p.cyan),
                    );
                    let cur_arch = self.sim.cpu.arch();
                    let mut selected_arch = cur_arch;
                    egui::ComboBox::from_id_source("cpu_arch_selector_combo")
                        .width(235.0)
                        .selected_text(
                            RichText::new(cur_arch.display_name())
                                .font(FontId::new(11.0, FontFamily::Monospace))
                                .color(p.text_primary),
                        )
                        .show_ui(ui, |ui| {
                            for arch in crate::cpu::CpuArch::ALL {
                                ui.selectable_value(
                                    &mut selected_arch,
                                    arch,
                                    arch.display_name(),
                                );
                            }
                        });
                    if selected_arch != cur_arch {
                        self.sim.cpu.set_arch(selected_arch);
                    }

                    crate::theme::group_separator(ui);

                    // Run / Pause / Single Step / Reset Controls
                    let is_running = self.sim.cpu.running;
                    if crate::theme::primary_button(
                        ui,
                        if is_running { "⏸ Pause" } else { "▶ Run" },
                        if is_running { p.orange } else { p.green },
                    )
                    .on_hover_text("Start or Pause continuous CPU instruction execution")
                    .clicked()
                    {
                        self.sim.cpu.running = !self.sim.cpu.running;
                    }

                    if crate::theme::tool_button(ui, "⏭ Single Step", false, p.cyan)
                        .on_hover_text("Execute 1 assembly instruction and inspect registers/PC")
                        .clicked()
                    {
                        self.sim.cpu.running = false;
                        self.sim.cpu.single_step();
                    }

                    if crate::theme::tool_button(ui, "↺ Reset", false, p.red)
                        .on_hover_text("Reset CPU registers, PC = 0x0000, and reload assembled ROM")
                        .clicked()
                    {
                        self.sim.cpu.reset();
                    }

                    crate::theme::group_separator(ui);

                    // Clock Mode & Frequency Slider (1 kHz .. 1 MHz)
                    let is_internal_clk = self.sim.cpu.use_internal_clock;
                    if crate::theme::tool_button(
                        ui,
                        if is_internal_clk {
                            "CLK: INTERNAL"
                        } else {
                            "CLK: CIRCUIT PIN"
                        },
                        is_internal_clk,
                        p.purple,
                    )
                    .on_hover_text("Toggle between internal high-speed clock (1 kHz - 1 MHz) and external circuit CLK pin")
                    .clicked()
                    {
                        self.sim.cpu.use_internal_clock = !self.sim.cpu.use_internal_clock;
                    }

                    if is_internal_clk {
                        ui.add(
                            egui::Slider::new(&mut self.sim.cpu.clock_hz, 1_000..=1_000_000)
                                .logarithmic(true)
                                .suffix(" Hz"),
                        )
                        .on_hover_text("Real-time CPU Clock Speed (1 kHz to 1 MHz)");
                    }
                });

                ui.separator();

                // -----------------------------------------------------------------
                // 2. SPLIT VIEW: LEFT = ASSEMBLY CODE EDITOR, RIGHT = DEBUGGER
                // -----------------------------------------------------------------
                ui.columns(2, |cols| {
                    // =============================================================
                    // LEFT COLUMN: INTEGRATED ASSEMBLY EDITOR & LOADER
                    // =============================================================
                    let left_ui = &mut cols[0];
                    crate::theme::section_header(
                        left_ui,
                        "ASSEMBLY CODE EDITOR & ASSEMBLER",
                        p.green,
                    );

                    left_ui.horizontal_wrapped(|ui| {
                        if crate::theme::primary_button(ui, "⚡ Assemble & Load", p.green)
                            .on_hover_text("Assemble source code and load machine code into ROM (0x0000)")
                            .clicked()
                        {
                            if let Ok(cnt) = self.sim.cpu.assemble_and_load() {
                                self.status_message = format!(
                                    "Assembled & loaded {} program ({cnt} instructions)!",
                                    self.sim.cpu.arch().code_str()
                                );
                            }
                        }

                        if crate::theme::tool_button(ui, "📂 Load .asm", false, p.cyan)
                            .on_hover_text("Load an Assembly source file (.asm / .s) from disk")
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Assembly Source (*.asm, *.s)", &["asm", "s", "txt"])
                                .pick_file()
                            {
                                if let Ok(txt) = std::fs::read_to_string(&path) {
                                    self.sim.cpu.source_code = txt;
                                    let _ = self.sim.cpu.assemble_and_load();
                                    self.status_message =
                                        format!("Loaded assembly from {}", path.display());
                                }
                            }
                        }

                        if crate::theme::tool_button(ui, "💾 Save .asm", false, p.cyan)
                            .on_hover_text("Save current Assembly source code to a .asm file")
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Assembly Source (*.asm)", &["asm"])
                                .set_file_name("program.asm")
                                .save_file()
                            {
                                if std::fs::write(&path, &self.sim.cpu.source_code).is_ok() {
                                    self.status_message =
                                        format!("Saved assembly to {}", path.display());
                                }
                            }
                        }

                        if crate::theme::tool_button(ui, "🏓 Pong.asm", false, p.orange)
                            .on_hover_text("Load the 2-Player Arcade Pong game (examples/pong.asm)")
                            .clicked()
                        {
                            self.sim.cpu.load_pong_demo();
                            self.sim.cpu.running = true;
                            self.status_message =
                                "Loaded & started examples/pong.asm!".to_string();
                        }
                    });

                    // Status / Syntax Error Banner with Highlighted Line Preview
                    if !self.sim.cpu.assembly_errors.is_empty() {
                        egui::Frame::none()
                            .fill(Color32::from_rgb(58, 18, 24))
                            .stroke(Stroke::new(1.2, p.red))
                            .inner_margin(egui::Margin::same(6.0))
                            .show(left_ui, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "⚠ {} SYNTAX ERROR(S) DETECTED:",
                                        self.sim.cpu.assembly_errors.len()
                                    ))
                                    .font(FontId::new(11.0, FontFamily::Monospace))
                                    .strong()
                                    .color(p.red),
                                );
                                let src_lines: Vec<&str> =
                                    self.sim.cpu.source_code.lines().collect();
                                for err in self.sim.cpu.assembly_errors.iter().take(4) {
                                    ui.label(
                                        RichText::new(format!(
                                            "• Line {:03}: {}",
                                            err.line, err.message
                                        ))
                                        .font(FontId::new(10.5, FontFamily::Monospace))
                                        .color(Color32::from_rgb(255, 190, 190)),
                                    );
                                    if let Some(bad_line) =
                                        src_lines.get(err.line.saturating_sub(1))
                                    {
                                        ui.label(
                                            RichText::new(format!(
                                                "    >> {:03} | {}",
                                                err.line,
                                                bad_line.trim()
                                            ))
                                            .font(FontId::new(10.5, FontFamily::Monospace))
                                            .background_color(Color32::from_rgb(95, 24, 34))
                                            .color(Color32::WHITE),
                                        );
                                    }
                                }
                            });
                    } else {
                        left_ui.label(
                            RichText::new(&self.sim.cpu.status_log)
                                .font(FontId::new(10.0, FontFamily::Monospace))
                                .color(p.green),
                        );
                    }

                    left_ui.add_space(4.0);

                    // Monospace Assembly Editor with line numbers gutter
                    let err_line_set: std::collections::HashSet<usize> = self
                        .sim
                        .cpu
                        .assembly_errors
                        .iter()
                        .map(|e| e.line)
                        .collect();
                    let cur_pc = self.sim.cpu.core.get_state().pc;
                    let active_src_line = self
                        .sim
                        .cpu
                        .last_assembled
                        .as_ref()
                        .and_then(|prog| prog.addr_to_line.get(&cur_pc).copied());

                    egui::ScrollArea::vertical()
                        .id_source("cpu_asm_editor_scroll")
                        .max_height(left_ui.available_height() - 8.0)
                        .show(left_ui, |ui| {
                            let line_count = self.sim.cpu.source_code.lines().count().max(1);
                            let mut gutter = String::with_capacity(line_count * 6);
                            for ln in 1..=line_count {
                                let marker = if err_line_set.contains(&ln) {
                                    "!"
                                } else if active_src_line == Some(ln) {
                                    "▶"
                                } else {
                                    " "
                                };
                                gutter.push_str(&format!("{marker}{ln:3}\n"));
                            }

                            ui.horizontal_top(|ui| {
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(gutter.trim_end())
                                            .font(FontId::new(11.5, FontFamily::Monospace))
                                            .color(if !err_line_set.is_empty() {
                                                p.orange
                                            } else {
                                                p.text_muted
                                            }),
                                    )
                                    .wrap(false),
                                );
                                ui.add(
                                    egui::TextEdit::multiline(&mut self.sim.cpu.source_code)
                                        .font(FontId::new(11.5, FontFamily::Monospace))
                                        .code_editor()
                                        .desired_width(f32::INFINITY)
                                        .desired_rows(28),
                                );
                            });
                        });

                    // =============================================================
                    // RIGHT COLUMN: REAL-TIME REGISTERS, DISASSEMBLER & HEX DUMP
                    // =============================================================
                    let right_ui = &mut cols[1];
                    let cpu_state = self.sim.cpu.core.get_state();

                    // --- A. CPU Status & Registers (Hex + Decimal) ---
                    crate::theme::section_header(
                        right_ui,
                        "REAL-TIME REGISTERS (HEX & DECIMAL)",
                        p.orange,
                    );

                    right_ui.horizontal_wrapped(|ui| {
                        crate::theme::status_pill(
                            ui,
                            &format!("PC: 0x{:04X} ({})", cpu_state.pc, cpu_state.pc),
                            p.cyan,
                        );
                        crate::theme::status_pill(
                            ui,
                            &format!("SP: 0x{:04X}", cpu_state.sp),
                            p.purple,
                        );
                        crate::theme::status_pill(
                            ui,
                            &format!("CYCLES: {}", cpu_state.cycles),
                            p.green,
                        );
                        crate::theme::status_pill(
                            ui,
                            &format!(
                                "FLAGS [Z:{} N:{} C:{} V:{}]",
                                u8::from(cpu_state.flags.z),
                                u8::from(cpu_state.flags.n),
                                u8::from(cpu_state.flags.c),
                                u8::from(cpu_state.flags.v),
                            ),
                            p.orange,
                        );
                    });

                    let regs = self.sim.cpu.core.registers();
                    egui::ScrollArea::vertical()
                        .id_source("cpu_registers_scroll")
                        .max_height(135.0)
                        .show(right_ui, |ui| {
                            egui::Grid::new("cpu_registers_grid")
                                .striped(true)
                                .num_columns(4)
                                .min_col_width(105.0)
                                .spacing([8.0, 2.0])
                                .show(ui, |ui| {
                                    for (idx, reg) in regs.iter().enumerate() {
                                        let signed_val = match reg.bits {
                                            8 => (reg.value as i8) as i64,
                                            16 => (reg.value as i16) as i64,
                                            _ => (reg.value as i32) as i64,
                                        };
                                        let hex_txt = match reg.bits {
                                            8 => format!("0x{:02X}", reg.value & 0xFF),
                                            16 => format!("0x{:04X}", reg.value & 0xFFFF),
                                            _ => format!("0x{:08X}", reg.value),
                                        };
                                        let col = if reg.changed {
                                            p.green
                                        } else if reg.value != 0 {
                                            p.text_primary
                                        } else {
                                            p.text_muted
                                        };
                                        ui.label(
                                            RichText::new(format!(
                                                "{:<7} {} ({})",
                                                reg.name, hex_txt, signed_val
                                            ))
                                            .font(FontId::new(10.5, FontFamily::Monospace))
                                            .color(col),
                                        );
                                        if (idx + 1) % 2 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });

                    right_ui.add_space(4.0);

                    // --- B. Live Disassembler (Current PC & Upcoming Instructions) ---
                    crate::theme::section_header(
                        right_ui,
                        "LIVE DISASSEMBLER (CURRENT PC & UPCOMING)",
                        p.purple,
                    );

                    egui::Frame::none()
                        .fill(p.bg_secondary)
                        .stroke(Stroke::new(1.0, p.border_secondary))
                        .inner_margin(egui::Margin::same(5.0))
                        .show(right_ui, |ui| {
                            let mut addr = cpu_state.pc;
                            for row in 0..6 {
                                let (asm_text, len) =
                                    self.sim.cpu.core.disassemble_at(addr, &self.sim.cpu.bus);
                                let is_cur = row == 0;
                                let prefix = if is_cur { "▶" } else { " " };
                                let src_ln = self
                                    .sim
                                    .cpu
                                    .last_assembled
                                    .as_ref()
                                    .and_then(|prog| prog.addr_to_line.get(&addr).copied())
                                    .map(|l| format!("L{l:03}"))
                                    .unwrap_or_else(|| "    ".to_string());
                                let row_txt = format!(
                                    "{prefix} 0x{addr:04X} [{src_ln}]  {asm_text}"
                                );
                                ui.label(
                                    RichText::new(row_txt)
                                        .font(FontId::new(11.0, FontFamily::Monospace))
                                        .strong()
                                        .color(if is_cur { p.green } else { p.text_secondary }),
                                );
                                addr = addr.wrapping_add(len.max(1));
                            }
                        });

                    right_ui.add_space(4.0);

                    // --- C. Memory Hex Dump Viewer with Direct Jump to VRAM ---
                    crate::theme::section_header(
                        right_ui,
                        "MEMORY HEX DUMP & MMIO / VRAM INSPECTOR",
                        p.cyan,
                    );

                    right_ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("JUMP:")
                                .font(FontId::new(10.0, FontFamily::Monospace))
                                .color(p.text_secondary),
                        );
                        for (label, target) in [
                            ("ROM 0x0000", 0x0000u16),
                            ("RAM 0x4000", 0x4000u16),
                            ("VRAM 0x8000", crate::cpu::bus::VRAM_BASE),
                            ("MMIO 0xF000", crate::cpu::bus::MMIO_BASE),
                        ] {
                            let is_active =
                                (self.sim.cpu.hex_dump_base & 0xFFF0) == (target & 0xFFF0);
                            if crate::theme::tool_button(ui, label, is_active, p.cyan).clicked() {
                                self.sim.cpu.hex_dump_base = target;
                                self.cpu_mem_addr_input = format!("0x{target:04X}");
                            }
                        }

                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut self.cpu_mem_addr_input)
                                    .desired_width(64.0)
                                    .font(FontId::new(11.0, FontFamily::Monospace)),
                            )
                            .changed()
                        {
                            let cleaned = self
                                .cpu_mem_addr_input
                                .trim()
                                .trim_start_matches("0x")
                                .trim_start_matches("0X");
                            if let Ok(parsed) = u16::from_str_radix(cleaned, 16) {
                                self.sim.cpu.hex_dump_base = parsed & 0xFFF0;
                            }
                        }
                    });

                    egui::ScrollArea::vertical()
                        .id_source("cpu_mem_hexdump_scroll")
                        .max_height(right_ui.available_height() - 6.0)
                        .show(right_ui, |ui| {
                            let base_start = self.sim.cpu.hex_dump_base & 0xFFF0;
                            for r in 0..16u16 {
                                let row_addr = base_start.wrapping_add(r * 16);
                                let mut hex_part = String::with_capacity(48);
                                let mut ascii_part = String::with_capacity(16);
                                for c in 0..16u16 {
                                    let b = self
                                        .sim
                                        .cpu
                                        .bus
                                        .peek8(u32::from(row_addr.wrapping_add(c)));
                                    hex_part.push_str(&format!("{b:02X} "));
                                    let ch = if (0x20..=0x7E).contains(&b) {
                                        b as char
                                    } else {
                                        '.'
                                    };
                                    ascii_part.push(ch);
                                }
                                ui.label(
                                    RichText::new(format!(
                                        "0x{row_addr:04X}: {hex_part} |{ascii_part}|"
                                    ))
                                    .font(FontId::new(10.5, FontFamily::Monospace))
                                    .color(p.text_primary),
                                );
                            }
                        });
                });
            });

        self.show_cpu_studio = open;
    }
}
