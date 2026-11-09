#![windows_subsystem = "windows"]
#![allow(float_literal_f32_fallback)]
//! LogicCraft — Digital Logic Circuit Studio (`main.rs`).
//!
//! ## Codebase Architecture
//! - [`main.rs`](file:///src/main.rs): Application entrypoint, [`LogicCraftApp`] state, keyboard shortcuts, and undo/redo/file actions.
//! - [`i18n.rs`](file:///src/i18n.rs): Runtime localization (`Locale`: English, Português, Español, Français, Deutsch).
//! - [`library.rs`](file:///src/library.rs): 11 component libraries, pin definitions, and US IEEE gate dimensions.
//! - [`simulator.rs`](file:///src/simulator.rs): 5-state digital logic simulation engine, wire routing, and wireless From/To tags.
//! - [`custom_component.rs`](file:///src/custom_component.rs): Custom Component specification, boolean expression evaluator, and AI prompt generator.
//! - [`kmap.rs`](file:///src/kmap.rs): Truth tables, Karnaugh Map (Quine-McCluskey) solver, and canvas card rendering.
//! - [`canvas_renderer.rs`](file:///src/canvas_renderer.rs): US IEEE vector gate symbols, IC packages, 7-seg displays, and palette icons.
//! - [`canvas_view.rs`](file:///src/canvas_view.rs): Interactive circuit canvas, marquee area selection, and lockstep group dragging.
//! - [`ui_panels.rs`](file:///src/ui_panels.rs): Top menu bar, toolbar, left gate palette, inspector, minimap, and oscilloscope panel.
//! - [`ui_dialogs.rs`](file:///src/ui_dialogs.rs): Floating studio windows (K-Map Studio, Custom IC Studio, RAM/ROM Editor, About).
//! - [`cdl_io.rs`](file:///src/cdl_io.rs): Circuit (`.cdl`), memory (`.cdm`/`.hex`), and oscilloscope (`.cdo`) file I/O.
//! - [`image_export.rs`](file:///src/image_export.rs): High-resolution raster circuit exporter (PNG/BMP & system clipboard).
//! - [`mechanics.rs`](file:///src/mechanics.rs): 5-state logic types (`0`, `1`, `Hi-Z`, `Conflict`, `Unknown`) and visual constants.

mod canvas_renderer;
mod canvas_view;
mod cdl_io;
mod custom_component;
mod i18n;
mod image_export;
mod kmap;
mod library;
mod mechanics;
mod settings;
mod simulator;
mod theme;
mod ui_dialogs;
mod ui_panels;
mod updater;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use eframe::egui::{self, Key, Pos2, Rect, Sense, Stroke, Vec2};

use crate::cdl_io::{export_cdl, import_cdl};
use crate::custom_component::builtin_custom_templates;
use crate::i18n::Locale;
use crate::settings::AppSettings;
use crate::image_export::{
    copy_image_to_system_clipboard, render_canvas_image, save_canvas_image_to_path,
};
use crate::library::GateLibrary;
use crate::mechanics::{
    DEFAULT_FRAME_HEIGHT, DEFAULT_FRAME_LEFT, DEFAULT_FRAME_TOP, DEFAULT_FRAME_WIDTH,
    DEFAULT_REFRESH_RATE_MS, INITIAL_FRAME_HEIGHT, INITIAL_FRAME_WIDTH,
};
use crate::simulator::{CircuitPage, CircuitSimulator, GateInstance, PinRef, WireInstance};
use crate::updater::{GitHubRelease, UpdateMessage, UpdateStatus};

fn main() -> eframe::Result<()> {
    let mut screenshot_path: Option<PathBuf> = None;
    let mut initial_circuit_path: Option<PathBuf> = None;
    let mut settings = AppSettings::load();
    let mut cli_locale: Option<Locale> = None;
    let mut renderer = if let Ok(r) = std::env::var("LOGICCRAFT_RENDERER") {
        if r.eq_ignore_ascii_case("wgpu") {
            eframe::Renderer::Wgpu
        } else {
            eframe::Renderer::Glow
        }
    } else {
        eframe::Renderer::Glow
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--screenshot" {
            if let Some(p) = args.next() {
                screenshot_path = Some(PathBuf::from(p));
            }
        } else if let Some(rest) = arg.strip_prefix("--screenshot=") {
            screenshot_path = Some(PathBuf::from(rest));
        } else if arg == "--lang" {
            if let Some(code) = args.next() {
                cli_locale = Some(parse_locale_code(&code));
            }
        } else if let Some(code) = arg.strip_prefix("--lang=") {
            cli_locale = Some(parse_locale_code(code));
        } else if arg == "--wgpu" {
            renderer = eframe::Renderer::Wgpu;
        } else if arg == "--glow" {
            renderer = eframe::Renderer::Glow;
        } else if !arg.starts_with('-') {
            let p = PathBuf::from(&arg);
            initial_circuit_path = Some(p);
        }
    }

    if let Some(loc) = cli_locale {
        settings.locale = loc;
    }

    let win_w = INITIAL_FRAME_WIDTH.max(DEFAULT_FRAME_WIDTH as f32).min(1440.0);
    let win_h = INITIAL_FRAME_HEIGHT.max(DEFAULT_FRAME_HEIGHT as f32).min(860.0);

    let app_icon = eframe::icon_data::from_png_bytes(include_bytes!("../packaging/assets/icon.png"))
        .ok();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("LogicCraft — Digital Logic Circuit Studio")
        .with_position(Pos2::new(DEFAULT_FRAME_LEFT, DEFAULT_FRAME_TOP))
        .with_inner_size([win_w, win_h])
        .with_min_inner_size([DEFAULT_FRAME_WIDTH as f32, DEFAULT_FRAME_HEIGHT as f32])
        .with_decorations(false)
        .with_resizable(true);

    if let Some(icon) = app_icon {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        renderer,
        ..Default::default()
    };

    eframe::run_native(
        "LogicCraft",
        native_options,
        Box::new(move |cc| {
            Box::new(LogicCraftApp::new(
                cc,
                screenshot_path,
                initial_circuit_path,
                settings,
            ))
        }),
    )
}

fn parse_locale_code(code: &str) -> Locale {
    match code.trim().to_ascii_lowercase().as_str() {
        "pt" | "pt-br" | "pt-pt" => Locale::Pt,
        "es" => Locale::Es,
        "fr" => Locale::Fr,
        "de" => Locale::De,
        _ => Locale::En,
    }
}

pub struct LogicCraftApp {
    pub lib: GateLibrary,
    pub sim: CircuitSimulator,
    pub locale: Locale,
    pub undo_stack: Vec<Vec<CircuitPage>>,
    pub redo_stack: Vec<Vec<CircuitPage>>,
    pub clipboard_gates: Vec<GateInstance>,
    pub clipboard_wires: Vec<WireInstance>,

    // View & Settings
    pub dark_mode: bool,
    pub show_gridlines: bool,
    pub show_wire_conn: bool,
    pub show_oscope: bool,
    pub show_memory_editor: bool,
    pub show_kmap_studio: bool,
    pub show_custom_ic_studio: bool,
    pub show_about_dialog: bool,
    pub show_update_dialog: bool,
    pub export_with_grid: bool,
    pub export_with_color: bool,
    pub canvas_size: Vec2,

    // Palette & interaction state
    pub palette_search: String,
    pub active_category: usize,
    pub placing_gate_type: Option<String>,
    pub placing_multi_active: bool,
    pub wiring_from: Option<PinRef>,
    pub selection_box_start: Option<Pos2>,
    pub dragging_gate_id: Option<u32>,
    pub drag_anchor_world: [f32; 2],
    pub drag_initial_positions: Vec<(u32, [f32; 2])>,
    pub drag_initial_wire_mids: Vec<(u32, Option<f32>, Option<f32>, Vec<[f32; 2]>)>,
    pub dragging_wire: Option<(u32, usize, bool)>,
    pub dragging_wire_breakpoint: Option<(u32, usize)>,
    pub dragging_wire_group: Vec<u32>,
    pub dragging_kmap: Option<usize>,
    pub dragging_kmap_func: Option<usize>,
    pub drag_offset_world: [f32; 2],
    pub drag_snapshot_saved: bool,
    pub memory_edit_gate_id: Option<u32>,
    pub new_channel_wire_id: usize,
    pub connector_name_draft: String,
    pub connector_edit_gate_id: Option<u32>,
    pub connector_warning: Option<String>,
    pub status_message: String,
    pub current_file_path: Option<PathBuf>,
    pub tip_index: usize,
    pub last_tip_switch: Instant,

    // Page editing state
    pub editing_page_name: Option<usize>,
    pub page_name_draft: String,

    // Custom Component Studio state
    pub custom_ic_tab: usize,
    pub custom_ic_name: String,
    pub custom_ic_label: String,
    pub custom_ic_desc: String,
    pub custom_ic_inputs: String,
    pub custom_ic_outputs: String,
    pub custom_ic_state_bits: String,
    pub custom_ic_clock_pin: String,
    pub custom_ic_reset_pin: String,
    pub custom_ic_equations: Vec<(String, String)>,
    pub custom_ic_json_buf: String,
    pub custom_ic_feedback: Option<(bool, String)>,

    // Timing, Updater & Screenshot
    pub update_status: UpdateStatus,
    pub update_tx: Sender<UpdateMessage>,
    pub update_rx: Receiver<UpdateMessage>,
    pub last_sim_tick: Instant,
    pub frame_count: u32,
    pub screenshot_target: Option<PathBuf>,
    pub screenshot_requested: bool,
}

impl LogicCraftApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        screenshot_target: Option<PathBuf>,
        initial_circuit_path: Option<PathBuf>,
        settings: AppSettings,
    ) -> Self {
        crate::updater::cleanup_old_binaries();
        let lib = GateLibrary::new();
        let sim = CircuitSimulator::new(&lib);
        let default_json = serde_json::to_string_pretty(&builtin_custom_templates()[0])
            .unwrap_or_default();
        let locale = settings.locale;
        let (update_tx, update_rx) = mpsc::channel();
        #[allow(unused_mut)]
        let mut app = Self {
            lib,
            sim,
            locale,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard_gates: Vec::new(),
            clipboard_wires: Vec::new(),
            dark_mode: settings.dark_mode,
            show_gridlines: settings.show_gridlines,
            show_wire_conn: settings.show_wire_conn,
            show_oscope: settings.show_oscope,
            show_memory_editor: false,
            show_kmap_studio: false,
            show_custom_ic_studio: false,
            show_about_dialog: false,
            show_update_dialog: false,
            export_with_grid: settings.export_with_grid,
            export_with_color: settings.export_with_color,
            canvas_size: Vec2::new(1000.0, 600.0),
            palette_search: String::new(),
            active_category: 0,
            placing_gate_type: None,
            placing_multi_active: false,
            wiring_from: None,
            selection_box_start: None,
            dragging_gate_id: None,
            drag_anchor_world: [0.0, 0.0],
            drag_initial_positions: Vec::new(),
            drag_initial_wire_mids: Vec::new(),
            dragging_wire: None,
            dragging_wire_breakpoint: None,
            dragging_wire_group: Vec::new(),
            dragging_kmap: None,
            dragging_kmap_func: None,
            drag_offset_world: [0.0, 0.0],
            drag_snapshot_saved: false,
            memory_edit_gate_id: None,
            new_channel_wire_id: 0,
            connector_name_draft: String::new(),
            connector_edit_gate_id: None,
            connector_warning: None,
            status_message: locale.tr().status_welcome.to_string(),
            current_file_path: None,
            tip_index: 0,
            last_tip_switch: Instant::now(),
            editing_page_name: None,
            page_name_draft: String::new(),
            custom_ic_tab: 0,
            custom_ic_name: "Parity & Majority Unit".to_string(),
            custom_ic_label: "PMAJ".to_string(),
            custom_ic_desc: "Outputs 3-bit majority and odd parity.".to_string(),
            custom_ic_inputs: "A, B, C".to_string(),
            custom_ic_outputs: "MAJ, PARITY".to_string(),
            custom_ic_state_bits: String::new(),
            custom_ic_clock_pin: String::new(),
            custom_ic_reset_pin: String::new(),
            custom_ic_equations: vec![
                ("MAJ".to_string(), "(A & B) | (B & C) | (A & C)".to_string()),
                ("PARITY".to_string(), "A ^ B ^ C".to_string()),
            ],
            custom_ic_json_buf: default_json,
            custom_ic_feedback: None,
            update_status: UpdateStatus::Idle,
            update_tx,
            update_rx,
            last_sim_tick: Instant::now(),
            frame_count: 0,
            screenshot_target,
            screenshot_requested: false,
        };
        if let Some(ref path) = initial_circuit_path {
            if path.exists() {
                app.load_circuit_from_path(path);
            } else {
                app.current_file_path = Some(path.clone());
            }
        }
        #[cfg(debug_assertions)]
        if app.current_file_path.is_none() && app.screenshot_target.is_some() && std::env::var("LOGICCRAFT_NO_DEMO").is_err() {
            app.sim.populate_demo_circuit(&app.lib);
            app.sim.pages.push(CircuitPage::new("ALU Subsystem"));
        }
        crate::theme::install_fonts(&cc.egui_ctx);
        app.apply_theme(&cc.egui_ctx);
        app.save_settings();

        // Automatically check for updates on startup (unless running in headless screenshot mode)
        if app.screenshot_target.is_none() {
            app.trigger_update_check(&cc.egui_ctx, false);
        }

        app
    }

    pub fn trigger_update_check(&mut self, ctx: &egui::Context, manual: bool) {
        self.update_status = UpdateStatus::Checking;
        if manual {
            self.status_message = match self.locale {
                Locale::Pt => "A verificar atualizações no GitHub...".to_string(),
                _ => "Checking GitHub for updates...".to_string(),
            };
        }
        crate::updater::spawn_update_check(self.update_tx.clone(), ctx.clone(), manual);
    }

    pub fn trigger_install_update(&mut self, ctx: &egui::Context, release: GitHubRelease) {
        let ver = release.version.clone();
        self.update_status = UpdateStatus::Downloading {
            release: release.clone(),
            progress: match self.locale {
                Locale::Pt => format!("A preparar transferência da versão v{ver}..."),
                _ => format!("Preparing download of v{ver}..."),
            },
        };
        crate::updater::spawn_download_and_install(self.update_tx.clone(), ctx.clone(), release);
    }

    pub fn poll_updater(&mut self) {
        while let Ok(msg) = self.update_rx.try_recv() {
            match msg {
                UpdateMessage::CheckResult { result, manual } => match result {
                    Ok(rel) => {
                        let pkg_ver = env!("CARGO_PKG_VERSION");
                        if crate::updater::is_version_newer(&rel.version, pkg_ver) {
                            self.status_message = match self.locale {
                                Locale::Pt => format!(
                                    "Nova atualização disponível no GitHub: v{} (Instalada: v{pkg_ver})",
                                    rel.version
                                ),
                                _ => format!(
                                    "New update available on GitHub: v{} (Installed: v{pkg_ver})",
                                    rel.version
                                ),
                            };
                            self.update_status = UpdateStatus::UpdateAvailable(rel);
                            // Automatically open the update dialog on startup when a new release exists!
                            self.show_update_dialog = true;
                        } else {
                            if manual {
                                self.status_message = format!(
                                    "LogicCraft CAD v{pkg_ver} (GitHub: v{}): {}",
                                    rel.version,
                                    self.locale.tr().update_status_latest
                                );
                            }
                            self.update_status = UpdateStatus::UpToDate(rel);
                        }
                    }
                    Err(err) => {
                        if manual {
                            self.status_message = format!("Update check error: {err}");
                        }
                        self.update_status = UpdateStatus::Error(err);
                    }
                },
                UpdateMessage::DownloadProgress { release, progress } => {
                    self.status_message = progress.clone();
                    self.update_status = UpdateStatus::Downloading { release, progress };
                }
                UpdateMessage::InstallResult(res) => match res {
                    Ok((ver, needs_restart)) => {
                        if needs_restart {
                            // Automatically restart into the newly installed binary!
                            crate::updater::restart_application();
                            self.update_status = UpdateStatus::ReadyToRestart {
                                version: ver.clone(),
                                message: format!(
                                    "Update v{ver} installed! Click Restart to launch the new version."
                                ),
                            };
                        } else {
                            self.update_status = UpdateStatus::ReadyToRestart {
                                version: ver.clone(),
                                message: format!(
                                    "Installer for v{ver} launched."
                                ),
                            };
                        }
                    }
                    Err(err) => {
                        self.status_message = format!("Update installation failed: {err}");
                        self.update_status = UpdateStatus::Error(err);
                    }
                },
            }
        }
    }

    pub fn apply_theme(&self, ctx: &egui::Context) {
        crate::theme::apply(ctx, self.dark_mode);
    }

    pub fn save_settings(&self) {
        let s = AppSettings {
            locale: self.locale,
            dark_mode: self.dark_mode,
            show_gridlines: self.show_gridlines,
            show_wire_conn: self.show_wire_conn,
            show_oscope: self.show_oscope,
            export_with_grid: self.export_with_grid,
            export_with_color: self.export_with_color,
        };
        s.save();
    }

    pub fn save_undo_snapshot(&mut self) {
        self.undo_stack.push(self.sim.pages.clone());
        if self.undo_stack.len() > 50 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.sim.pages.clone());
            self.sim.pages = prev;
            self.sim.current_page = self
                .sim
                .current_page
                .min(self.sim.pages.len().saturating_sub(1));
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.sim.pages.clone());
            self.sim.pages = next;
            self.sim.current_page = self
                .sim
                .current_page
                .min(self.sim.pages.len().saturating_sub(1));
        }
    }

    pub fn delete_selected(&mut self) {
        if self.sim.locked {
            return;
        }
        let page_idx = self.sim.current_page;
        let has_sel = self.sim.pages[page_idx].gates.iter().any(|g| g.selected)
            || self.sim.pages[page_idx].wires.iter().any(|w| w.selected);
        if !has_sel {
            return;
        }
        self.save_undo_snapshot();
        let page = &mut self.sim.pages[page_idx];
        let removed_gates: Vec<u32> = page
            .gates
            .iter()
            .filter(|g| g.selected)
            .map(|g| g.id)
            .collect();
        page.gates.retain(|g| !g.selected);
        page.wires.retain(|w| {
            !w.selected
                && !w
                    .endpoints
                    .iter()
                    .any(|ep| removed_gates.contains(&ep.gate_id))
        });
    }

    pub fn copy_selected(&mut self, ctx: &egui::Context) {
        let page = &self.sim.pages[self.sim.current_page];
        self.clipboard_gates = page.gates.iter().filter(|g| g.selected).cloned().collect();
        let sel_ids: Vec<u32> = self.clipboard_gates.iter().map(|g| g.id).collect();
        self.clipboard_wires = page
            .wires
            .iter()
            .filter(|w| {
                w.endpoints.len() >= 2
                    && w.endpoints.iter().all(|ep| sel_ids.contains(&ep.gate_id))
            })
            .cloned()
            .collect();
        if !self.clipboard_gates.is_empty() {
            ctx.copy_text(format!(
                "LOGICCRAFT_CLIPBOARD:{}",
                self.clipboard_gates.len()
            ));
            self.status_message = format!("Copied {} component(s).", self.clipboard_gates.len());
        }
    }

    pub fn paste_clipboard(&mut self) {
        if self.sim.locked || self.clipboard_gates.is_empty() {
            return;
        }
        self.save_undo_snapshot();
        let page_idx = self.sim.current_page;
        for g in &mut self.sim.pages[page_idx].gates {
            g.selected = false;
        }
        for w in &mut self.sim.pages[page_idx].wires {
            w.selected = false;
        }
        // Shift clipboard positions on each paste so repeated Ctrl+V cascades cleanly
        for g in &mut self.clipboard_gates {
            g.pos[0] = CircuitSimulator::snap_to_grid(g.pos[0] + 2.5);
            g.pos[1] = CircuitSimulator::snap_to_grid(g.pos[1] - 2.5);
        }
        for w in &mut self.clipboard_wires {
            w.custom_mid_x = w
                .custom_mid_x
                .map(|mx| CircuitSimulator::snap_to_grid(mx + 2.5));
            w.custom_mid_y = w
                .custom_mid_y
                .map(|my| CircuitSimulator::snap_to_grid(my - 2.5));
            for wp in &mut w.waypoints {
                wp[0] = CircuitSimulator::snap_to_grid(wp[0] + 2.5);
                wp[1] = CircuitSimulator::snap_to_grid(wp[1] - 2.5);
            }
        }

        let to_paste = self.clipboard_gates.clone();
        let mut id_map = std::collections::HashMap::new();
        for mut g in to_paste {
            let old_id = g.id;
            let new_id = self.sim.next_gate_id;
            self.sim.next_gate_id += 1;
            g.id = new_id;
            id_map.insert(old_id, new_id);
            if g.type_name == "DA_FROM" {
                let cur_name = g
                    .lparams
                    .get("JUNCTION_ID")
                    .cloned()
                    .unwrap_or_else(|| "NET_A".into());
                if self.sim.is_from_name_taken(&cur_name, None) {
                    let unique = self.sim.next_unique_from_name();
                    g.lparams.insert("JUNCTION_ID".into(), unique);
                }
            }
            g.selected = true;
            self.sim.pages[page_idx].gates.push(g);
        }
        let wires_to_paste = self.clipboard_wires.clone();
        for mut w in wires_to_paste {
            let mut valid = true;
            for ep in &mut w.endpoints {
                if let Some(&mapped_id) = id_map.get(&ep.gate_id) {
                    ep.gate_id = mapped_id;
                } else {
                    valid = false;
                }
            }
            if valid {
                w.id = self.sim.next_wire_id;
                self.sim.next_wire_id += 1;
                w.selected = true;
                self.sim.pages[page_idx].wires.push(w);
            }
        }
        self.sim.rebuild_wire_geometry(&self.lib, page_idx);
        self.status_message = format!("Pasted {} component(s).", self.clipboard_gates.len());
    }

    pub fn rotate_selected(&mut self, delta_deg: f32) {
        if self.sim.locked {
            return;
        }
        let page_idx = self.sim.current_page;
        if !self.sim.pages[page_idx].gates.iter().any(|g| g.selected) {
            return;
        }
        self.save_undo_snapshot();
        for g in &mut self.sim.pages[page_idx].gates {
            if g.selected {
                g.angle_deg = (g.angle_deg + delta_deg).rem_euclid(360.0);
            }
        }
        self.sim.rebuild_wire_geometry(&self.lib, page_idx);
    }

    pub fn copy_canvas_image_to_clipboard(&mut self) {
        let img = render_canvas_image(
            &self.sim,
            &self.lib,
            self.dark_mode,
            self.export_with_grid,
            self.export_with_color,
        );
        if let Err(e) = copy_image_to_system_clipboard(&img) {
            self.status_message = format!("Clipboard image copy note: {e}");
        }
    }

    pub fn export_canvas_image_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG Image (*.png)", &["png"])
            .add_filter("Bitmap Image (*.bmp)", &["bmp"])
            .set_file_name("logiccraft_circuit.png")
            .save_file()
        {
            let img = render_canvas_image(
                &self.sim,
                &self.lib,
                self.dark_mode,
                self.export_with_grid,
                self.export_with_color,
            );
            let _ = save_canvas_image_to_path(&img, &path);
        }
    }

    pub fn load_circuit_from_path(&mut self, path: &Path) -> bool {
        match std::fs::read_to_string(path) {
            Ok(txt) => {
                self.save_undo_snapshot();
                match import_cdl(&mut self.sim, &mut self.lib, &txt) {
                    Ok(()) => {
                        self.current_file_path = Some(path.to_path_buf());
                        self.status_message = format!("Opened circuit from {}", path.display());
                        true
                    }
                    Err(e) => {
                        self.status_message = format!("Open error: {e}");
                        false
                    }
                }
            }
            Err(e) => {
                self.status_message = format!("Failed to read file: {e}");
                false
            }
        }
    }

    pub fn save_circuit_dialog(&mut self, save_as: bool) {
        let target = if !save_as {
            self.current_file_path.clone()
        } else {
            None
        }
        .or_else(|| {
            rfd::FileDialog::new()
                .add_filter("LogicCraft Circuit (*.lcc)", &["lcc"])
                .add_filter("CEDAR Logic Legacy (*.cdl)", &["cdl"])
                .set_file_name("circuit.lcc")
                .save_file()
        });
        if let Some(path) = target {
            let xml = export_cdl(&self.sim, Some(&self.lib));
            if std::fs::write(&path, xml).is_ok() {
                self.current_file_path = Some(path.clone());
                self.status_message = format!("Saved circuit to {}", path.display());
            }
        }
    }

    pub fn add_new_tab(&mut self) {
        if self.sim.pages.len() >= 10 {
            return;
        }
        let n = self.sim.pages.len() + 1;
        self.sim.pages.push(CircuitPage::new(format!("Page {n}")));
        self.sim.current_page = self.sim.pages.len() - 1;
    }

    pub fn close_tab(&mut self, idx: usize) {
        if self.sim.pages.len() <= 1 {
            return;
        }
        if self.editing_page_name == Some(idx) {
            self.editing_page_name = None;
        } else if let Some(e_idx) = self.editing_page_name {
            if e_idx > idx {
                self.editing_page_name = Some(e_idx - 1);
            }
        }
        self.save_undo_snapshot();
        self.sim.pages.remove(idx);
        if self.sim.current_page >= self.sim.pages.len() {
            self.sim.current_page = self.sim.pages.len() - 1;
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let typing_in_text_edit = ctx.wants_keyboard_input();
        let mut do_copy = false;
        let mut do_paste = false;
        ctx.input(|i| {
            let ctrl = i.modifiers.ctrl || i.modifiers.command;
            let copy_ev = (ctrl && i.key_pressed(Key::C))
                || i.events.iter().any(|ev| matches!(ev, egui::Event::Copy));
            let paste_ev = (ctrl && i.key_pressed(Key::V))
                || i.events
                    .iter()
                    .any(|ev| matches!(ev, egui::Event::Paste(_)));
            if !typing_in_text_edit {
                if copy_ev {
                    do_copy = true;
                } else if paste_ev {
                    do_paste = true;
                } else if ctrl && i.key_pressed(Key::Z) {
                    self.undo();
                } else if ctrl && i.key_pressed(Key::Y) {
                    self.redo();
                } else if ctrl && i.key_pressed(Key::A) {
                    let page_idx = self.sim.current_page;
                    for g in &mut self.sim.pages[page_idx].gates {
                        g.selected = true;
                    }
                    for w in &mut self.sim.pages[page_idx].wires {
                        w.selected = true;
                    }
                } else if ctrl && i.key_pressed(Key::G) {
                    self.show_oscope = !self.show_oscope;
                } else if ctrl && i.key_pressed(Key::K) {
                    self.show_kmap_studio = !self.show_kmap_studio;
                } else if ctrl && i.key_pressed(Key::T) {
                    self.add_new_tab();
                } else if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                    self.delete_selected();
                } else if i.key_pressed(Key::R) {
                    self.rotate_selected(90.0);
                } else if i.key_pressed(Key::Escape) {
                    self.placing_gate_type = None;
                    self.placing_multi_active = false;
                    self.wiring_from = None;
                    self.dragging_wire = None;
                    self.dragging_wire_group.clear();
                } else if i.key_pressed(Key::Space) {
                    self.sim.running = !self.sim.running;
                }
            } else if i.key_pressed(Key::Escape) {
                self.placing_gate_type = None;
                self.placing_multi_active = false;
                self.wiring_from = None;
                self.dragging_wire = None;
                self.dragging_wire_group.clear();
            }
        });
        if do_copy {
            self.copy_selected(ctx);
        }
        if do_paste {
            self.paste_clipboard();
        }
    }

    /// Draws an outer border and handles edge/corner window resizing when running in borderless mode.
    pub fn draw_window_resize_handles(&self, ctx: &egui::Context) {
        let is_maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
        let screen_rect = ctx.input(|i| i.screen_rect());
        let p = crate::theme::palette(self.dark_mode);

        // Always draw a subtle 1px border around the entire window when not maximized
        if !is_maximized {
            let painter = ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("window_frame_stroke"),
            ));
            painter.rect_stroke(screen_rect, 0.0, Stroke::new(1.0, p.border));
        } else {
            return;
        }

        let border_thickness = 4.0;
        let corner_size = 12.0;

        let edges = [
            // Corners
            (
                Rect::from_min_size(screen_rect.min, Vec2::splat(corner_size)),
                egui::viewport::ResizeDirection::NorthWest,
                egui::CursorIcon::ResizeNorthWest,
                "win_resize_nw",
            ),
            (
                Rect::from_min_size(
                    Pos2::new(screen_rect.right() - corner_size, screen_rect.top()),
                    Vec2::splat(corner_size),
                ),
                egui::viewport::ResizeDirection::NorthEast,
                egui::CursorIcon::ResizeNorthEast,
                "win_resize_ne",
            ),
            (
                Rect::from_min_size(
                    Pos2::new(screen_rect.left(), screen_rect.bottom() - corner_size),
                    Vec2::splat(corner_size),
                ),
                egui::viewport::ResizeDirection::SouthWest,
                egui::CursorIcon::ResizeSouthWest,
                "win_resize_sw",
            ),
            (
                Rect::from_min_size(
                    screen_rect.max - Vec2::splat(corner_size),
                    Vec2::splat(corner_size),
                ),
                egui::viewport::ResizeDirection::SouthEast,
                egui::CursorIcon::ResizeSouthEast,
                "win_resize_se",
            ),
            // Edges
            (
                Rect::from_min_max(
                    Pos2::new(screen_rect.left() + corner_size, screen_rect.top()),
                    Pos2::new(screen_rect.right() - corner_size, screen_rect.top() + border_thickness),
                ),
                egui::viewport::ResizeDirection::North,
                egui::CursorIcon::ResizeNorth,
                "win_resize_n",
            ),
            (
                Rect::from_min_max(
                    Pos2::new(screen_rect.left() + corner_size, screen_rect.bottom() - border_thickness),
                    Pos2::new(screen_rect.right() - corner_size, screen_rect.bottom()),
                ),
                egui::viewport::ResizeDirection::South,
                egui::CursorIcon::ResizeSouth,
                "win_resize_s",
            ),
            (
                Rect::from_min_max(
                    Pos2::new(screen_rect.left(), screen_rect.top() + corner_size),
                    Pos2::new(screen_rect.left() + border_thickness, screen_rect.bottom() - corner_size),
                ),
                egui::viewport::ResizeDirection::West,
                egui::CursorIcon::ResizeWest,
                "win_resize_w",
            ),
            (
                Rect::from_min_max(
                    Pos2::new(screen_rect.right() - border_thickness, screen_rect.top() + corner_size),
                    Pos2::new(screen_rect.right(), screen_rect.bottom() - corner_size),
                ),
                egui::viewport::ResizeDirection::East,
                egui::CursorIcon::ResizeEast,
                "win_resize_e",
            ),
        ];

        egui::Area::new(egui::Id::new("window_resize_layer"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen_rect.min)
            .interactable(true)
            .show(ctx, |ui| {
                ui.set_clip_rect(screen_rect);
                for (rect, direction, cursor, id_str) in edges {
                    let resp = ui.interact(rect, egui::Id::new(id_str), Sense::drag());
                    if resp.hovered() {
                        ctx.set_cursor_icon(cursor);
                    }
                    if resp.drag_started_by(egui::PointerButton::Primary) {
                        ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
                    }
                }
            });
    }
}

impl eframe::App for LogicCraftApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_count = self.frame_count.saturating_add(1);
        self.apply_theme(ctx);

        // Handle drag-and-drop of circuit files into the window
        let dropped_file = ctx.input(|i| {
            i.raw.dropped_files.first().and_then(|f| f.path.clone())
        });
        if let Some(path) = dropped_file {
            self.load_circuit_from_path(&path);
        }

        // Update window title with active file name
        let title = if let Some(ref p) = self.current_file_path {
            let fname = p.file_name().and_then(|n| n.to_str()).unwrap_or("circuit.lcc");
            format!("LogicCraft CAD — {fname}")
        } else {
            "LogicCraft CAD — Digital Logic Circuit Studio".to_string()
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

        // Check if a screenshot event arrived from egui ViewportCommand::Screenshot
        let mut received_screenshot: Option<std::sync::Arc<egui::ColorImage>> = None;
        ctx.input(|i| {
            for ev in &i.raw.events {
                if let egui::Event::Screenshot { image, .. } = ev {
                    received_screenshot = Some(image.clone());
                }
            }
        });
        if let (Some(target_path), Some(img)) = (&self.screenshot_target, received_screenshot) {
            if let Some(parent) = target_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let width = img.width() as u32;
            let height = img.height() as u32;
            let mut rgba = Vec::with_capacity((width * height * 4) as usize);
            for px in &img.pixels {
                rgba.extend_from_slice(&px.to_array());
            }
            let _ = image::save_buffer(
                target_path,
                &rgba,
                width,
                height,
                image::ColorType::Rgba8,
            );
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            std::process::exit(0);
        }

        // Advance simulation on timer or during screenshot warmup frames
        if self.sim.running {
            let interval = Duration::from_millis(self.sim.time_step_ms as u64);
            if self.screenshot_target.is_some() {
                if self.frame_count % 3 == 0 {
                    self.sim.step(&self.lib);
                }
            } else if self.last_sim_tick.elapsed() >= interval {
                self.sim.step(&self.lib);
                self.last_sim_tick = Instant::now();
            }
        }

        // Poll background GitHub updater channel
        self.poll_updater();

        // Advance cycling shortcut tips in the footer every 6 seconds
        if self.last_tip_switch.elapsed() >= Duration::from_secs(6) {
            let tips_len = self.locale.cycling_tips().len().max(1);
            self.tip_index = (self.tip_index + 1) % tips_len;
            self.last_tip_switch = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(6));

        self.handle_shortcuts(ctx);
        self.draw_top_bars(ctx);
        self.draw_status_bar(ctx);

        if self.show_oscope {
            self.draw_oscope_panel(ctx);
        }

        self.draw_left_sidebar(ctx);
        self.draw_central_canvas(ctx);

        if self.show_memory_editor {
            self.draw_memory_editor_modal(ctx);
        }
        if self.show_kmap_studio {
            self.draw_kmap_studio_window(ctx);
        }
        if self.show_custom_ic_studio {
            self.draw_custom_ic_studio_window(ctx);
        }
        if self.show_about_dialog {
            self.draw_about_modal(ctx);
        }
        if self.show_update_dialog {
            self.draw_update_modal(ctx);
        }

        self.draw_window_resize_handles(ctx);

        if self.screenshot_target.is_some() {
            if self.frame_count == 118 && !self.screenshot_requested {
                self.screenshot_requested = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot);
            } else if self.frame_count >= 125 {
                if let Some(target_path) = &self.screenshot_target {
                    if let Some(parent) = target_path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let img = render_canvas_image(&self.sim, &self.lib, self.dark_mode, true, true);
                    let _ = save_canvas_image_to_path(&img, target_path);
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                std::process::exit(0);
            }
            ctx.request_repaint();
        } else if self.sim.running {
            ctx.request_repaint_after(Duration::from_millis(
                self.sim.time_step_ms.min(DEFAULT_REFRESH_RATE_MS) as u64,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_app() -> LogicCraftApp {
        let lib = GateLibrary::new();
        let sim = CircuitSimulator::new(&lib);
        LogicCraftApp {
            lib,
            sim,
            locale: Locale::En,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            clipboard_gates: Vec::new(),
            clipboard_wires: Vec::new(),
            dark_mode: true,
            show_gridlines: true,
            show_wire_conn: true,
            show_oscope: true,
            show_memory_editor: false,
            show_kmap_studio: false,
            show_custom_ic_studio: false,
            show_about_dialog: false,
            show_update_dialog: false,
            export_with_grid: true,
            export_with_color: true,
            canvas_size: Vec2::new(1000.0, 600.0),
            palette_search: String::new(),
            active_category: 0,
            placing_gate_type: None,
            placing_multi_active: false,
            wiring_from: None,
            selection_box_start: None,
            dragging_gate_id: None,
            drag_anchor_world: [0.0, 0.0],
            drag_initial_positions: Vec::new(),
            drag_initial_wire_mids: Vec::new(),
            dragging_wire: None,
            dragging_wire_breakpoint: None,
            dragging_wire_group: Vec::new(),
            dragging_kmap: None,
            dragging_kmap_func: None,
            drag_offset_world: [0.0, 0.0],
            drag_snapshot_saved: false,
            memory_edit_gate_id: None,
            new_channel_wire_id: 0,
            connector_name_draft: String::new(),
            connector_edit_gate_id: None,
            connector_warning: None,
            status_message: String::new(),
            current_file_path: None,
            editing_page_name: None,
            page_name_draft: String::new(),
            custom_ic_tab: 0,
            custom_ic_name: String::new(),
            custom_ic_label: String::new(),
            custom_ic_desc: String::new(),
            custom_ic_inputs: String::new(),
            custom_ic_outputs: String::new(),
            custom_ic_state_bits: String::new(),
            custom_ic_clock_pin: String::new(),
            custom_ic_reset_pin: String::new(),
            custom_ic_equations: Vec::new(),
            custom_ic_json_buf: String::new(),
            custom_ic_feedback: None,
            update_status: UpdateStatus::Idle,
            update_tx: mpsc::channel().0,
            update_rx: mpsc::channel().1,
            last_sim_tick: Instant::now(),
            tip_index: 0,
            last_tip_switch: Instant::now(),
            frame_count: 0,
            screenshot_target: None,
            screenshot_requested: false,
        }
    }

    #[test]
    fn test_page_tab_close_and_rename() {
        let mut app = create_test_app();
        app.sim.pages = vec![
            CircuitPage::new("Page 1"),
            CircuitPage::new("Page 2"),
            CircuitPage::new("Page 3"),
        ];
        app.sim.current_page = 1;

        // Test inline renaming
        app.editing_page_name = Some(1);
        app.page_name_draft = "Memory Subsystem".to_string();
        // Commit rename
        let trimmed = app.page_name_draft.trim().to_string();
        app.sim.pages[1].name = trimmed;
        app.editing_page_name = None;
        assert_eq!(app.sim.pages[1].name, "Memory Subsystem");

        // Test tab closing with active page
        app.close_tab(1);
        assert_eq!(app.sim.pages.len(), 2);
        assert_eq!(app.sim.pages[0].name, "Page 1");
        assert_eq!(app.sim.pages[1].name, "Page 3");
    }

    #[test]
    fn test_load_circuit_from_path() {
        let lib = GateLibrary::new();
        let mut sim = CircuitSimulator::new(&lib);
        sim.populate_demo_circuit(&lib);
        let xml = export_cdl(&sim, Some(&lib));

        let tmp_dir = std::env::temp_dir();
        let tmp_path = tmp_dir.join("test_circuit_sample.lcc");
        std::fs::write(&tmp_path, xml).unwrap();

        let mut app = create_test_app();

        assert!(app.load_circuit_from_path(&tmp_path));
        assert_eq!(app.current_file_path, Some(tmp_path.clone()));
        assert!(!app.sim.pages.is_empty());
        assert!(!app.sim.pages[0].gates.is_empty());

        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_kmap_and_full_circuit_roundtrip() {
        let mut lib = GateLibrary::new();
        let mut sim = CircuitSimulator::new(&lib);
        sim.pages[0].name = "Custom Page Alpha".to_string();

        let mut km = crate::kmap::TruthTableKMap::new_default("Test Karnaugh Map");
        km.num_vars = 4;
        km.var_names = ["W".into(), "X".into(), "Y".into(), "Z".into()];
        km.output_name = "OUT_RESULT".into();
        km.values[3] = 1;
        km.values[7] = 2; // Don't care
        km.pinned_on_canvas = true;
        km.canvas_pos = [12.5, -45.0];
        sim.kmaps.push(km.clone());
        sim.active_kmap = sim.kmaps.len() - 1;

        let xml = export_cdl(&sim, Some(&lib));
        assert!(xml.contains("<kmaps>"));
        assert!(xml.contains("Test Karnaugh Map"));
        assert!(xml.contains("OUT_RESULT"));
        assert!(xml.contains("Custom Page Alpha"));

        let mut restored_sim = CircuitSimulator::new(&lib);
        import_cdl(&mut restored_sim, &mut lib, &xml).expect("Should successfully import CDL/LCC");

        assert_eq!(restored_sim.pages[0].name, "Custom Page Alpha");
        assert_eq!(restored_sim.kmaps.len(), sim.kmaps.len());
        let restored_km = &restored_sim.kmaps[restored_sim.active_kmap];
        assert_eq!(restored_km.name, "Test Karnaugh Map");
        assert_eq!(restored_km.num_vars, 4);
        assert_eq!(restored_km.var_names, ["W", "X", "Y", "Z"]);
        assert_eq!(restored_km.output_name, "OUT_RESULT");
        assert_eq!(restored_km.values[3], 1);
        assert_eq!(restored_km.values[7], 2);
        assert!(restored_km.pinned_on_canvas);
        assert_eq!(restored_km.canvas_pos, [12.5, -45.0]);
    }

    #[test]
    fn test_copy_and_paste_group() {
        let mut app = create_test_app();
        let ctx = egui::Context::default();
        app.sim.pages[0].gates.clear();
        app.sim.pages[0].wires.clear();

        let def_and = app.lib.get("AA_AND2").unwrap().clone();
        let def_led = app.lib.get("GA_LED").unwrap().clone();
        let mut g1 = GateInstance::new(10, &def_and, [0.0, 0.0]);
        g1.selected = true;
        let mut g2 = GateInstance::new(11, &def_led, [10.0, 0.0]);
        g2.selected = true;
        app.sim.pages[0].gates.push(g1);
        app.sim.pages[0].gates.push(g2);
        app.sim.next_gate_id = 12;
        app.sim.connect_pins(&app.lib, 0, 10, "OUT", 11, "IN_0", "AND_TO_LED");

        app.copy_selected(&ctx);
        assert_eq!(app.clipboard_gates.len(), 2);
        assert_eq!(app.clipboard_wires.len(), 1);

        app.paste_clipboard();
        assert_eq!(app.sim.pages[0].gates.len(), 4);
        assert_eq!(app.sim.pages[0].wires.len(), 2);
        let pasted_wire = &app.sim.pages[0].wires[1];
        assert_ne!(pasted_wire.endpoints[0].gate_id, 10);
        assert_ne!(pasted_wire.endpoints[1].gate_id, 11);
    }

    #[test]
    fn test_to_from_connectors_not_memory() {
        let lib = GateLibrary::new();
        let from_inst = GateInstance::new(1, lib.get("DA_FROM").unwrap(), [0.0, 0.0]);
        let to_inst = GateInstance::new(2, lib.get("DE_TO").unwrap(), [5.0, 0.0]);
        let ram_inst = GateInstance::new(3, lib.get("AA_RAM_4x4").unwrap(), [10.0, 0.0]);
        let rom_inst = GateInstance::new(4, lib.get("BA_ROM_4x4").unwrap(), [15.0, 0.0]);

        assert!(!from_inst.is_memory_device());
        assert!(!to_inst.is_memory_device());
        assert!(ram_inst.is_memory_device());
        assert!(rom_inst.is_memory_device());
    }

    #[test]
    fn test_vertical_wire_custom_mid_x() {
        let lib = GateLibrary::new();
        let mut sim = CircuitSimulator::new(&lib);
        sim.pages[0].gates.clear();
        sim.pages[0].wires.clear();

        let g1 = GateInstance::new(1, lib.get("AA_AND2").unwrap(), [-10.0, 5.0]);
        let g2 = GateInstance::new(2, lib.get("GA_LED").unwrap(), [10.0, -5.0]);
        sim.pages[0].gates.push(g1);
        sim.pages[0].gates.push(g2);
        let wid = sim.connect_pins(&lib, 0, 1, "OUT", 2, "IN_0", "W");

        sim.pages[0].wires[0].custom_mid_x = Some(3.5);
        sim.pages[0].wires[0].custom_mid_y = None;
        sim.rebuild_wire_geometry(&lib, 0);

        let w = &sim.pages[0].wires.iter().find(|w| w.id == wid).unwrap();
        assert_eq!(w.points.len(), 4);
        assert!((w.points[1][0] - 3.5).abs() < 1e-4);
        assert!((w.points[2][0] - 3.5).abs() < 1e-4);
    }

    #[test]
    fn test_cycling_tips_all_locales() {
        for loc in Locale::ALL {
            assert!(!loc.cycling_tips().is_empty());
        }
    }

    #[test]
    fn test_updater_version_comparison() {
        assert!(crate::updater::is_version_newer("1.0.5", "1.0.4"));
        assert!(crate::updater::is_version_newer("v1.1.0", "1.0.9"));
        assert!(!crate::updater::is_version_newer("1.0.5", "1.0.5"));
        assert!(!crate::updater::is_version_newer("1.0.2", "1.0.5"));
    }

    #[test]
    fn test_wire_breakpoints_add_move_remove_and_roundtrip() {
        let mut lib = GateLibrary::new();
        let mut sim = CircuitSimulator::new(&lib);
        sim.pages[0].gates.clear();
        sim.pages[0].wires.clear();

        let g1 = GateInstance::new(1, lib.get("AA_AND2").unwrap(), [-10.0, 5.0]);
        let g2 = GateInstance::new(2, lib.get("GA_LED").unwrap(), [10.0, -5.0]);
        sim.pages[0].gates.push(g1);
        sim.pages[0].gates.push(g2);
        let wid = sim.connect_pins(&lib, 0, 1, "OUT", 2, "IN_0", "W_BP");

        // Double-click middle vertical segment (seg_idx = 1) to create a breakpoint
        {
            let w = sim.pages[0].wires.iter_mut().find(|w| w.id == wid).unwrap();
            assert_eq!(w.points.len(), 4);
            assert!(w.add_breakpoint(1, [0.0, 0.0]).is_some());
            assert_eq!(w.breakpoints.len(), 1);
            assert_eq!(w.points.len(), 6);

            // Move the breakpoint in 2D
            w.move_breakpoint(0, [2.0, 1.5]);
            let bp_pt_idx = w.breakpoints[0].0 + 1;
            assert!((w.points[bp_pt_idx][0] - 2.0).abs() < 1e-4);
            assert!((w.points[bp_pt_idx][1] - 1.5).abs() < 1e-4);

            // Drag one of the split vertical sub-segments orthogonally
            w.move_segment_orthogonal(1, true, -3.5);
            assert!((w.points[1][0] - (-3.5)).abs() < 1e-4);
        }

        sim.rebuild_wire_geometry(&lib, 0);

        // Export and import CDL/LCC to verify persistence
        let xml = export_cdl(&sim, Some(&lib));
        assert!(xml.contains("<waypoints>"));
        assert!(xml.contains("<breakpoints>"));

        let mut restored_sim = CircuitSimulator::new(&lib);
        import_cdl(&mut restored_sim, &mut lib, &xml).unwrap();
        let rw = &restored_sim.pages[0].wires[0];
        assert_eq!(rw.breakpoints.len(), 1);
        assert_eq!(rw.waypoints.len(), 4);

        // Remove the breakpoint
        let mut rw_mut = rw.clone();
        rw_mut.remove_breakpoint(0);
        assert!(rw_mut.breakpoints.is_empty());
    }
}
