use std::collections::{HashMap, VecDeque};

use crate::custom_component::CustomComponentSpec;
use crate::kmap::TruthTableKMap;
use crate::library::GateLibrary;
use crate::mechanics::{LogicState, DEFAULT_VIEW_ZOOM};
use crate::simulator::{
    CircuitPage, CircuitSimulator, GateInstance, OscopeChannel, PinRef, WireInstance,
};

pub fn export_cdl(sim: &CircuitSimulator, lib: Option<&GateLibrary>) -> String {
    let mut out = String::new();
    out.push_str("<circuit>\n");
    out.push_str(&format!("<CurrentPage>{}</CurrentPage>\n", sim.current_page));

    for (idx, page) in sim.pages.iter().enumerate() {
        out.push_str(&format!("<page {idx}>\n"));
        out.push_str(&format!("<PageName>{}</PageName>\n", page.name));
        out.push_str(&format!("<PageZoom>{:.4}</PageZoom>\n", page.zoom_world_per_px));
        out.push_str(&format!(
            "<PageViewport>{:.2},{:.2},{:.2},{:.2}</PageViewport>\n",
            page.pan_world[0] - 40.0,
            page.pan_world[1] + 30.0,
            page.pan_world[0] + 40.0,
            page.pan_world[1] - 30.0
        ));

        for g in &page.gates {
            out.push_str("<gate>\n");
            out.push_str(&format!("<ID>{}</ID>\n", g.id));
            out.push_str(&format!("<type>{}</type>\n", g.type_name));
            out.push_str(&format!("<position>{:.2},{:.2}</position>\n", g.pos[0], g.pos[1]));
            for w in &page.wires {
                for ep in &w.endpoints {
                    if ep.gate_id == g.id {
                        out.push_str(&format!(
                            "<input>\n<ID>{}</ID>\n{} </input>\n",
                            ep.pin_name, w.id
                        ));
                    }
                }
            }
            out.push_str(&format!("<gparam>angle {:.1}</gparam>\n", g.angle_deg));
            for (k, v) in &g.gparams {
                if k != "angle" {
                    out.push_str(&format!("<gparam>{k} {v}</gparam>\n"));
                }
            }
            for (k, v) in &g.lparams {
                out.push_str(&format!("<lparam>{k} {v}</lparam>\n"));
            }
            if !g.memory.is_empty() {
                out.push_str("<memory>");
                let mut keys: Vec<u16> = g.memory.keys().copied().collect();
                keys.sort_unstable();
                for addr in keys {
                    let val = g.memory.get(&addr).copied().unwrap_or(0);
                    out.push_str(&format!("{addr:04X}:{val:04X} "));
                }
                out.push_str("</memory>\n");
            }
            out.push_str("</gate>\n");
        }

        for w in &page.wires {
            out.push_str("<wire>\n");
            out.push_str(&format!("<ID>{}</ID>\n", w.id));
            if !w.label.is_empty() && w.label != format!("W{}", w.id) {
                out.push_str(&format!("<label>{}</label>\n", w.label));
            }
            if let (Some(mx), Some(my)) = (w.custom_mid_x, w.custom_mid_y) {
                out.push_str(&format!("<custom_mid>{:.2},{:.2}</custom_mid>\n", mx, my));
            }
            out.push_str("<shape>\n");
            for seg in w.points.windows(2) {
                let p0 = seg[0];
                let p1 = seg[1];
                if (p0[0] - p1[0]).abs() < 0.01 {
                    out.push_str(&format!(
                        "<vsegment>\n<ID>0</ID>\n<points>{:.2},{:.2},{:.2},{:.2}</points>\n</vsegment>\n",
                        p0[0], p0[1], p1[0], p1[1]
                    ));
                } else {
                    out.push_str(&format!(
                        "<hsegment>\n<ID>0</ID>\n<points>{:.2},{:.2},{:.2},{:.2}</points>\n</hsegment>\n",
                        p0[0], p0[1], p1[0], p1[1]
                    ));
                }
            }
            out.push_str("</shape>\n");
            out.push_str("</wire>\n");
        }

        out.push_str(&format!("</page {idx}>\n"));
    }

    // Export Karnaugh Maps and Truth Tables
    if !sim.kmaps.is_empty() {
        out.push_str("<kmaps>\n");
        out.push_str(&format!("<ActiveKMap>{}</ActiveKMap>\n", sim.active_kmap));
        for km in &sim.kmaps {
            out.push_str("<kmap>\n");
            out.push_str(&format!("<name>{}</name>\n", km.name));
            out.push_str(&format!("<num_vars>{}</num_vars>\n", km.num_vars));
            out.push_str(&format!(
                "<var_names>{},{},{},{}</var_names>\n",
                km.var_names[0], km.var_names[1], km.var_names[2], km.var_names[3]
            ));
            out.push_str(&format!("<output_name>{}</output_name>\n", km.output_name));
            let vals_str: Vec<String> = km.values.iter().map(|v| v.to_string()).collect();
            out.push_str(&format!("<values>{}</values>\n", vals_str.join(",")));
            out.push_str(&format!("<pinned_on_canvas>{}</pinned_on_canvas>\n", km.pinned_on_canvas));
            out.push_str(&format!("<func_pinned_on_canvas>{}</func_pinned_on_canvas>\n", km.func_pinned_on_canvas));
            out.push_str(&format!("<canvas_pos>{:.2},{:.2}</canvas_pos>\n", km.canvas_pos[0], km.canvas_pos[1]));
            out.push_str(&format!("<func_canvas_pos>{:.2},{:.2}</func_canvas_pos>\n", km.func_canvas_pos[0], km.func_canvas_pos[1]));
            out.push_str("</kmap>\n");
        }
        out.push_str("</kmaps>\n");
    }

    // Export Custom Components specifications so circuit is fully self-contained
    if let Some(lib) = lib {
        if !lib.custom_specs.is_empty() {
            out.push_str("<custom_components>\n");
            for spec in lib.custom_specs.values() {
                if let Ok(json) = serde_json::to_string(spec) {
                    out.push_str(&format!("<spec>{}</spec>\n", json));
                }
            }
            out.push_str("</custom_components>\n");
        }
    }

    out.push_str("</circuit>\n");
    out
}

pub fn import_cdl(sim: &mut CircuitSimulator, lib: &mut GateLibrary, content: &str) -> Result<(), String> {
    let mut pages = Vec::new();
    let mut max_gate_id = 1u32;
    let mut max_wire_id = 1u32;

    for page_idx in 0..10usize {
        let open_tag = format!("<page {page_idx}>");
        let close_tag = format!("</page {page_idx}>");
        let Some(start) = content.find(&open_tag) else {
            continue;
        };
        let Some(end_rel) = content[start..].find(&close_tag) else {
            continue;
        };
        let page_str = &content[start + open_tag.len()..start + end_rel];

        let page_name = extract_tag(page_str, "PageName")
            .unwrap_or_else(|| format!("Page {}", page_idx + 1));
        let page_zoom = extract_tag(page_str, "PageZoom")
            .and_then(|s| s.trim().parse::<f32>().ok())
            .unwrap_or(DEFAULT_VIEW_ZOOM);
        let pan_world = extract_tag(page_str, "PageViewport")
            .and_then(|s| {
                let mut sp = s.split(',');
                let x0 = sp.next()?.trim().parse::<f32>().ok()?;
                let y0 = sp.next()?.trim().parse::<f32>().ok()?;
                let x1 = sp.next()?.trim().parse::<f32>().ok()?;
                let y1 = sp.next()?.trim().parse::<f32>().ok()?;
                Some([(x0 + x1) * 0.5, (y0 + y1) * 0.5])
            })
            .unwrap_or([0.0, 0.0]);

        let mut page = CircuitPage {
            name: page_name,
            gates: Vec::new(),
            wires: Vec::new(),
            pan_world,
            zoom_world_per_px: page_zoom,
        };

        // Parse <gate>...</gate> blocks
        let mut wire_pin_links: HashMap<u32, Vec<PinRef>> = HashMap::new();
        let mut cursor = 0usize;
        while let Some(g_start_rel) = page_str[cursor..].find("<gate>") {
            let g_start = cursor + g_start_rel + 6;
            let Some(g_end_rel) = page_str[g_start..].find("</gate>") else {
                break;
            };
            let g_block = &page_str[g_start..g_start + g_end_rel];
            cursor = g_start + g_end_rel + 7;

            let id = extract_tag(g_block, "ID")
                .and_then(|s| s.trim().parse::<u32>().ok())
                .unwrap_or(max_gate_id);
            max_gate_id = max_gate_id.max(id + 1);
            let mut raw_type = extract_tag(g_block, "type")
                .unwrap_or_else(|| "AA_AND2".to_string())
                .trim()
                .to_string();

            // Replace deprecated flip-flops per sub_4172e0 (0x4172e0)
            if raw_type == "AA_DFF" {
                raw_type = "AE_DFF_LOW".to_string();
            } else if raw_type == "BA_JKFF" {
                raw_type = "BE_JKFF_LOW".to_string();
            } else if raw_type == "BA_JKFF_NT" {
                raw_type = "BE_JKFF_LOW_NT".to_string();
            }

            let pos = extract_tag(g_block, "position")
                .and_then(|s| {
                    let mut sp = s.split(',');
                    let x = sp.next()?.trim().parse::<f32>().ok()?;
                    let y = sp.next()?.trim().parse::<f32>().ok()?;
                    Some([x, y])
                })
                .unwrap_or([0.0, 0.0]);

            let def = lib
                .get(&raw_type)
                .or_else(|| lib.get("AA_AND2"))
                .ok_or_else(|| "Missing fallback gate definition".to_string())?;
            let mut inst = GateInstance::new(id, def, pos);

            for gp in extract_all_tags(g_block, "gparam") {
                let trimmed = gp.trim();
                if let Some((k, v)) = trimmed.split_once(' ') {
                    if k == "angle" {
                        inst.angle_deg = v.trim().parse::<f32>().unwrap_or(0.0);
                    } else {
                        inst.gparams.insert(k.trim().to_string(), v.trim().to_string());
                    }
                }
            }
            for lp in extract_all_tags(g_block, "lparam") {
                let trimmed = lp.trim();
                if let Some((k, v)) = trimmed.split_once(' ') {
                    inst.lparams.insert(k.trim().to_string(), v.trim().to_string());
                }
            }

            if let Some(mem_str) = extract_tag(g_block, "memory") {
                for pair in mem_str.split_whitespace() {
                    if let Some((a_str, d_str)) = pair.split_once(':') {
                        if let (Ok(addr), Ok(val)) = (
                            u16::from_str_radix(a_str.trim(), 16),
                            u16::from_str_radix(d_str.trim(), 16),
                        ) {
                            inst.memory.insert(addr, val);
                        }
                    }
                }
            }

            for tag_name in ["input", "output"] {
                for io_block in extract_all_tags(g_block, tag_name) {
                    if let Some(pin_name) = extract_tag(&io_block, "ID") {
                        let after_id = io_block
                            .find("</ID>")
                            .map(|i| &io_block[i + 5..])
                            .unwrap_or("");
                        for tok in after_id.split_whitespace() {
                            if let Ok(wid) = tok.parse::<u32>() {
                                max_wire_id = max_wire_id.max(wid + 1);
                                wire_pin_links.entry(wid).or_default().push(PinRef {
                                    gate_id: id,
                                    pin_name: pin_name.trim().to_string(),
                                });
                            }
                        }
                    }
                }
            }

            page.gates.push(inst);
        }

        // Parse wire labels and custom mid points from <wire>...</wire>
        let mut wire_labels: HashMap<u32, String> = HashMap::new();
        let mut wire_custom_mids: HashMap<u32, (f32, f32)> = HashMap::new();
        for w_block in extract_all_tags(page_str, "wire") {
            if let Some(wid) = extract_tag(&w_block, "ID").and_then(|s| s.trim().parse::<u32>().ok()) {
                if let Some(lbl) = extract_tag(&w_block, "label") {
                    wire_labels.insert(wid, lbl.trim().to_string());
                }
                if let Some(cm) = extract_tag(&w_block, "custom_mid") {
                    if let Some((x_str, y_str)) = cm.split_once(',') {
                        if let (Ok(x), Ok(y)) = (x_str.trim().parse::<f32>(), y_str.trim().parse::<f32>()) {
                            wire_custom_mids.insert(wid, (x, y));
                        }
                    }
                }
            }
        }

        // Construct WireInstances from wire_pin_links and wire metadata
        for (wid, endpoints) in wire_pin_links {
            if endpoints.len() >= 2 {
                let label = wire_labels.remove(&wid).unwrap_or_else(|| format!("W{wid}"));
                let (custom_mid_x, custom_mid_y) = match wire_custom_mids.remove(&wid) {
                    Some((x, y)) => (Some(x), Some(y)),
                    None => (None, None),
                };
                page.wires.push(WireInstance {
                    id: wid,
                    endpoints,
                    points: vec![[0.0, 0.0], [1.0, 0.0]],
                    solder_dots: Vec::new(),
                    state: LogicState::Zero,
                    bus_bits: 1,
                    bus_states: vec![LogicState::Zero],
                    label,
                    selected: false,
                    custom_mid_x,
                    custom_mid_y,
                });
            }
        }

        pages.push(page);
    }

    if pages.is_empty() {
        return Err("No valid <page> blocks found in circuit file".to_string());
    }

    sim.pages = pages;
    let loaded_cp = extract_tag(content, "CurrentPage")
        .and_then(|s| s.trim().parse::<usize>().ok())
        .unwrap_or(0);
    sim.current_page = loaded_cp.min(sim.pages.len().saturating_sub(1));
    sim.next_gate_id = max_gate_id;
    sim.next_wire_id = max_wire_id;

    // Parse Karnaugh Maps and Truth Tables
    if let Some(kmaps_block) = extract_tag(content, "kmaps") {
        let active_kmap = extract_tag(&kmaps_block, "ActiveKMap")
            .and_then(|s| s.trim().parse::<usize>().ok())
            .unwrap_or(0);
        let mut parsed_kmaps = Vec::new();
        for km_block in extract_all_tags(&kmaps_block, "kmap") {
            let name = extract_tag(&km_block, "name").unwrap_or_else(|| "K-Map".to_string());
            let num_vars = extract_tag(&km_block, "num_vars")
                .and_then(|s| s.trim().parse::<usize>().ok())
                .unwrap_or(3)
                .clamp(2, 4);
            let var_names = if let Some(v_str) = extract_tag(&km_block, "var_names") {
                let mut parts = v_str.split(',').map(|s| s.trim().to_string());
                [
                    parts.next().unwrap_or_else(|| "A".into()),
                    parts.next().unwrap_or_else(|| "B".into()),
                    parts.next().unwrap_or_else(|| "C".into()),
                    parts.next().unwrap_or_else(|| "D".into()),
                ]
            } else {
                ["A".into(), "B".into(), "C".into(), "D".into()]
            };
            let output_name = extract_tag(&km_block, "output_name").unwrap_or_else(|| "F".to_string());
            let mut values = vec![0u8; 16];
            if let Some(vals_str) = extract_tag(&km_block, "values") {
                for (idx, token) in vals_str.split(',').enumerate() {
                    if idx < 16 {
                        if let Ok(val) = token.trim().parse::<u8>() {
                            values[idx] = val.min(2);
                        }
                    }
                }
            }
            let pinned_on_canvas = extract_tag(&km_block, "pinned_on_canvas")
                .map(|s| s.trim().eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let func_pinned_on_canvas = extract_tag(&km_block, "func_pinned_on_canvas")
                .map(|s| s.trim().eq_ignore_ascii_case("true"))
                .unwrap_or(false);
            let canvas_pos = extract_tag(&km_block, "canvas_pos")
                .and_then(|s| {
                    let mut sp = s.split(',');
                    let x = sp.next()?.trim().parse::<f32>().ok()?;
                    let y = sp.next()?.trim().parse::<f32>().ok()?;
                    Some([x, y])
                })
                .unwrap_or([-8.0, -13.5]);
            let func_canvas_pos = extract_tag(&km_block, "func_canvas_pos")
                .and_then(|s| {
                    let mut sp = s.split(',');
                    let x = sp.next()?.trim().parse::<f32>().ok()?;
                    let y = sp.next()?.trim().parse::<f32>().ok()?;
                    Some([x, y])
                })
                .unwrap_or([-8.0, -21.5]);

            parsed_kmaps.push(TruthTableKMap {
                name,
                num_vars,
                var_names,
                output_name,
                values,
                pinned_on_canvas,
                func_pinned_on_canvas,
                canvas_pos,
                func_canvas_pos,
            });
        }
        if !parsed_kmaps.is_empty() {
            sim.kmaps = parsed_kmaps;
            sim.active_kmap = active_kmap.min(sim.kmaps.len().saturating_sub(1));
        }
    }

    // Parse Custom Components specifications so circuit is fully self-contained
    if let Some(custom_block) = extract_tag(content, "custom_components") {
        for spec_str in extract_all_tags(&custom_block, "spec") {
            if let Ok(spec) = serde_json::from_str::<CustomComponentSpec>(&spec_str) {
                let _ = lib.register_custom_component(spec);
            }
        }
    }

    for i in 0..sim.pages.len() {
        sim.rebuild_wire_geometry(lib, i);
    }
    Ok(())
}

/// Exports RAM/ROM contents in CedarLogic `.cdm` format (`sub_48a160` at `0x48a160`)
pub fn export_cdm(memory: &HashMap<u16, u16>) -> String {
    let mut out = String::from(
        "# CEDAR Logic RAM memory file.\n# Format: [Hex Address] : [Hex Data]\n# Note that if a memory location is not represented here,\n# then it is assumed to contain 0.\n\n",
    );
    let mut keys: Vec<u16> = memory.keys().copied().collect();
    keys.sort_unstable();
    for addr in keys {
        let val = memory.get(&addr).copied().unwrap_or(0);
        if val != 0 {
            out.push_str(&format!("{addr:02X} : {val:02X}\n"));
        }
    }
    out
}

/// Imports RAM/ROM contents from `.cdm` or Intel `.hex` format (`sub_463370`, `sub_48a160`)
pub fn import_cdm_or_hex(content: &str) -> HashMap<u16, u16> {
    let mut mem = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some(hex_body) = trimmed.strip_prefix(':') {
            // Intel HEX record :LLAAAATTDD...CC
            if hex_body.len() >= 10 {
                let byte_count = u8::from_str_radix(&hex_body[0..2], 16).unwrap_or(0) as usize;
                let base_addr = u16::from_str_radix(&hex_body[2..6], 16).unwrap_or(0);
                let rec_type = u8::from_str_radix(&hex_body[6..8], 16).unwrap_or(1);
                if rec_type == 0 {
                    for i in 0..byte_count {
                        let pos = 8 + i * 2;
                        if pos + 2 <= hex_body.len() {
                            if let Ok(b) = u16::from_str_radix(&hex_body[pos..pos + 2], 16) {
                                mem.insert(base_addr.wrapping_add(i as u16), b);
                            }
                        }
                    }
                }
            }
        } else if let Some((a_str, d_str)) = trimmed.split_once(':') {
            if let (Ok(addr), Ok(data)) = (
                u16::from_str_radix(a_str.trim(), 16),
                u16::from_str_radix(d_str.trim(), 16),
            ) {
                mem.insert(addr, data);
            }
        }
    }
    mem
}

/// Exports O-Scope layout in `.cdo` format (`sub_45dc70` at `0x45dc70`)
pub fn export_cdo(channels: &[OscopeChannel]) -> String {
    let mut out = String::from("OSCOPE LAYOUT FILE\n");
    for ch in channels {
        out.push_str(&format!("{}\n", ch.name));
    }
    out
}

/// Imports O-Scope layout from `.cdo` format (`sub_45d770` at `0x45d770`)
pub fn import_cdo(content: &str) -> Vec<OscopeChannel> {
    let mut out = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if idx == 0 && trimmed == "OSCOPE LAYOUT FILE" {
            continue;
        }
        if !trimmed.is_empty() && trimmed != "[None]" {
            out.push(OscopeChannel {
                name: trimmed.to_string(),
                target_wire_id: None,
                target_pin: None,
                history: VecDeque::new(),
            });
        }
    }
    out
}

fn extract_tag(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = block.find(&open)? + open.len();
    let end = block[start..].find(&close)? + start;
    Some(block[start..end].to_string())
}

fn extract_all_tags(block: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut results = Vec::new();
    let mut cursor = 0usize;
    while let Some(s_rel) = block[cursor..].find(&open) {
        let s = cursor + s_rel + open.len();
        let Some(e_rel) = block[s..].find(&close) else {
            break;
        };
        results.push(block[s..s + e_rel].to_string());
        cursor = s + e_rel + close.len();
    }
    results
}
