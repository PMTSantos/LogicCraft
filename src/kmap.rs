use eframe::egui::{self, Align2, FontFamily, FontId, Pos2, Rect, Stroke, Vec2};

#[derive(Debug, Clone, PartialEq)]
pub struct TruthTableKMap {
    pub name: String,
    pub num_vars: usize, // 2, 3, or 4
    pub var_names: [String; 4],
    pub output_name: String,
    pub values: Vec<u8>, // 0 = 0, 1 = 1, 2 = X (Don't care); length = 16
    pub pinned_on_canvas: bool,
    pub func_pinned_on_canvas: bool,
    pub canvas_pos: [f32; 2],
    pub func_canvas_pos: [f32; 2],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Implicant {
    pub bits: u8,
    pub mask: u8, // 1 where variable is don't-care / merged
}

impl Implicant {
    pub fn covers(&self, minterm: usize, num_vars: usize) -> bool {
        let m = (minterm as u8) & ((1 << num_vars) - 1);
        (m & !self.mask) == (self.bits & !self.mask)
    }

    pub fn format_sop_term(&self, var_names: &[String; 4], num_vars: usize) -> String {
        let defaults = ["A", "B", "C", "D"];
        let mut parts = Vec::new();
        for i in 0..num_vars {
            let bit_pos = num_vars - 1 - i;
            let bit_mask = 1u8 << bit_pos;
            if (self.mask & bit_mask) == 0 {
                let vname = if var_names[i].trim().is_empty() {
                    defaults[i]
                } else {
                    var_names[i].trim()
                };
                if (self.bits & bit_mask) != 0 {
                    parts.push(vname.to_string());
                } else {
                    parts.push(format!("{vname}'"));
                }
            }
        }
        if parts.is_empty() {
            "1".to_string()
        } else {
            parts.join("·")
        }
    }
}

impl TruthTableKMap {
    pub fn new_default(name: impl Into<String>) -> Self {
        // Default 3-variable majority / carry logic: F = AB + BC + AC
        let mut values = vec![0u8; 16];
        for m in 0..8usize {
            let a = (m >> 2) & 1;
            let b = (m >> 1) & 1;
            let c = m & 1;
            if (a + b + c) >= 2 {
                values[m] = 1;
            }
        }
        Self {
            name: name.into(),
            num_vars: 3,
            var_names: ["A".into(), "B".into(), "C".into(), "D".into()],
            output_name: "F".into(),
            values,
            pinned_on_canvas: false,
            func_pinned_on_canvas: false,
            canvas_pos: [-8.0, -13.5],
            func_canvas_pos: [-8.0, -21.5],
        }
    }

    pub fn effective_output_name(&self) -> &str {
        let trimmed = self.output_name.trim();
        if trimmed.is_empty() {
            "F"
        } else {
            trimmed
        }
    }

    pub fn effective_var_name(&self, idx: usize) -> &str {
        const DEFAULTS: [&str; 4] = ["A", "B", "C", "D"];
        let trimmed = self.var_names.get(idx).map(|s| s.trim()).unwrap_or("");
        if trimmed.is_empty() {
            DEFAULTS[idx.min(3)]
        } else {
            trimmed
        }
    }

    #[allow(dead_code)]
    pub fn num_rows(&self) -> usize {
        1usize << self.num_vars.clamp(2, 4)
    }

    /// Computes minimal prime implicants using Quine-McCluskey + Petrick's coverage
    pub fn solve_minimal_sop(&self) -> (Vec<Implicant>, String) {
        let n = self.num_vars.clamp(2, 4);
        let rows = 1usize << n;
        let mut ones = Vec::new();
        let mut dont_cares = Vec::new();
        for m in 0..rows {
            match self.values.get(m).copied().unwrap_or(0) {
                1 => ones.push(m as u8),
                2 => dont_cares.push(m as u8),
                _ => {}
            }
        }
        if ones.is_empty() {
            return (Vec::new(), format!("{} = 0", self.effective_output_name()));
        }
        if ones.len() + dont_cares.len() == rows {
            let all_mask = (1u8 << n) - 1;
            let imp = Implicant {
                bits: 0,
                mask: all_mask,
            };
            return (vec![imp], format!("{} = 1", self.effective_output_name()));
        }

        let mut current: Vec<Implicant> = ones
            .iter()
            .chain(dont_cares.iter())
            .map(|&m| Implicant { bits: m, mask: 0 })
            .collect();
        let mut primes: Vec<Implicant> = Vec::new();

        loop {
            let mut next = Vec::new();
            let mut used = vec![false; current.len()];
            for i in 0..current.len() {
                for j in (i + 1)..current.len() {
                    if current[i].mask == current[j].mask {
                        let diff = (current[i].bits ^ current[j].bits) & !current[i].mask;
                        if diff.count_ones() == 1 {
                            used[i] = true;
                            used[j] = true;
                            let merged = Implicant {
                                bits: current[i].bits & !diff,
                                mask: current[i].mask | diff,
                            };
                            if !next.contains(&merged) {
                                next.push(merged);
                            }
                        }
                    }
                }
            }
            for (i, imp) in current.into_iter().enumerate() {
                if !used[i] && !primes.contains(&imp) {
                    primes.push(imp);
                }
            }
            if next.is_empty() {
                break;
            }
            current = next;
        }

        // Greedy + exact essential prime implicant selection covering all `ones`
        let mut uncovered = ones.clone();
        let mut chosen: Vec<Implicant> = Vec::new();

        // 1. Essential prime implicants
        loop {
            let mut added_essential = false;
            for &m in &uncovered.clone() {
                let covering: Vec<&Implicant> = primes
                    .iter()
                    .filter(|p| p.covers(m as usize, n))
                    .collect();
                if covering.len() == 1 {
                    let ess = covering[0].clone();
                    if !chosen.contains(&ess) {
                        chosen.push(ess.clone());
                        uncovered.retain(|&u| !ess.covers(u as usize, n));
                        added_essential = true;
                    }
                }
            }
            if !added_essential {
                break;
            }
        }

        // 2. Cover remaining minterms with largest implicants first
        while !uncovered.is_empty() {
            if let Some(best) = primes
                .iter()
                .max_by_key(|p| {
                    let cov = uncovered
                        .iter()
                        .filter(|&&u| p.covers(u as usize, n))
                        .count();
                    (cov, p.mask.count_ones() as usize)
                })
                .cloned()
            {
                if uncovered.iter().all(|&u| !best.covers(u as usize, n)) {
                    break;
                }
                uncovered.retain(|&u| !best.covers(u as usize, n));
                if !chosen.contains(&best) {
                    chosen.push(best);
                }
            } else {
                break;
            }
        }

        let terms: Vec<String> = chosen
            .iter()
            .map(|imp| imp.format_sop_term(&self.var_names, n))
            .collect();
        let expr = format!("{} = {}", self.effective_output_name(), terms.join(" + "));
        (chosen, expr)
    }

    pub fn solve_minimal_pos(&self) -> String {
        let mut inverted = self.clone();
        for v in &mut inverted.values {
            if *v == 0 {
                *v = 1;
            } else if *v == 1 {
                *v = 0;
            }
        }
        let n = self.num_vars.clamp(2, 4);
        let rows = 1usize << n;
        let zeros_count = (0..rows).filter(|&m| self.values[m] == 0).count();
        if zeros_count == 0 {
            return format!("{} = 1", self.effective_output_name());
        }
        if zeros_count == rows {
            return format!("{} = 0", self.effective_output_name());
        }
        let (imps, _) = inverted.solve_minimal_sop();
        let mut clauses = Vec::new();
        for imp in imps {
            let mut lits = Vec::new();
            for i in 0..n {
                let bit_pos = n - 1 - i;
                let bit_mask = 1u8 << bit_pos;
                if (imp.mask & bit_mask) == 0 {
                    let vname = self.effective_var_name(i);
                    if (imp.bits & bit_mask) != 0 {
                        lits.push(format!("{vname}'"));
                    } else {
                        lits.push(vname.to_string());
                    }
                }
            }
            if !lits.is_empty() {
                clauses.push(format!("({})", lits.join(" + ")));
            }
        }
        format!("{} = {}", self.effective_output_name(), clauses.join("·"))
    }
}

/// Returns Gray code sequence for `bits` (1 -> [0,1], 2 -> [0,1,3,2])
pub fn gray_code(bits: usize) -> Vec<(usize, &'static str)> {
    if bits == 1 {
        vec![(0, "0"), (1, "1")]
    } else {
        vec![(0, "00"), (1, "01"), (3, "11"), (2, "10")]
    }
}

/// Draws a pinned K-Map card directly on the circuit canvas and returns its screen `Rect` for dragging
pub fn draw_kmap_on_canvas(
    painter: &egui::Painter,
    screen_pos: Pos2,
    px_per_world: f32,
    kmap: &TruthTableKMap,
    dark_mode: bool,
    is_dragging: bool,
) -> Rect {
    let scale = (px_per_world / 12.5).clamp(0.65, 1.45);
    let n = kmap.num_vars.clamp(2, 4);
    let row_bits = n / 2;
    let col_bits = n - row_bits;
    let rows = 1usize << row_bits;
    let cols = 1usize << col_bits;

    let cell_w = 26.0 * scale;
    let cell_h = 21.0 * scale;
    let pad = 10.0 * scale;
    let header_h = 26.0 * scale;
    let w = (cols as f32 * cell_w + 62.0 * scale).max(175.0 * scale);
    let h = header_h + rows as f32 * cell_h + 30.0 * scale;

    let p = crate::theme::palette(dark_mode);
    let card_rect = Rect::from_min_size(screen_pos, Vec2::new(w, h));
    let bg = p.bg_secondary;
    let border = if is_dragging { p.cyan } else { p.purple };
    let text_col = p.text;

    painter.rect_filled(card_rect, crate::theme::RADIUS, bg);
    painter.rect_filled(
        Rect::from_min_size(card_rect.min, Vec2::new(3.0, h)),
        crate::theme::RADIUS,
        border,
    );
    painter.rect_stroke(
        card_rect,
        crate::theme::RADIUS,
        Stroke::new(if is_dragging { 2.0_f32 } else { 1.0_f32 }, border),
    );

    let (imps, _) = kmap.solve_minimal_sop();
    let row_vars: String = (0..row_bits).map(|i| kmap.effective_var_name(i)).collect::<Vec<_>>().join("");
    let col_vars: String = (row_bits..n).map(|i| kmap.effective_var_name(i)).collect::<Vec<_>>().join("");

    painter.text(
        Pos2::new(card_rect.left() + pad, card_rect.top() + 13.0 * scale),
        Align2::LEFT_CENTER,
        format!("K-MAP // {} [{row_vars}\\{col_vars}]", kmap.name.to_uppercase()),
        FontId::new(10.0 * scale, FontFamily::Monospace),
        text_col,
    );

    let r_gray = gray_code(row_bits);
    let c_gray = gray_code(col_bits);
    let grid_left = card_rect.left() + 40.0 * scale;
    let grid_top = card_rect.top() + header_h + 14.0 * scale;

    for (ci, &(_, clbl)) in c_gray.iter().enumerate() {
        let cx = grid_left + (ci as f32 + 0.5) * cell_w;
        painter.text(
            Pos2::new(cx, grid_top - 8.0 * scale),
            Align2::CENTER_CENTER,
            clbl,
            FontId::new(9.5 * scale, FontFamily::Monospace),
            p.text_dim,
        );
    }

    let group_colors = [
        crate::theme::alpha(p.green, 55),
        crate::theme::alpha(p.cyan, 55),
        crate::theme::alpha(p.purple, 55),
        crate::theme::alpha(p.orange, 55),
    ];

    for (ri, &(r_val, rlbl)) in r_gray.iter().enumerate() {
        let cy = grid_top + (ri as f32 + 0.5) * cell_h;
        painter.text(
            Pos2::new(grid_left - 8.0 * scale, cy),
            Align2::RIGHT_CENTER,
            rlbl,
            FontId::new(9.5 * scale, FontFamily::Monospace),
            p.text_dim,
        );
        for (ci, &(c_val, _)) in c_gray.iter().enumerate() {
            let minterm = (r_val << col_bits) | c_val;
            let val = kmap.values.get(minterm).copied().unwrap_or(0);
            let cell_r = Rect::from_min_size(
                Pos2::new(grid_left + ci as f32 * cell_w, grid_top + ri as f32 * cell_h),
                Vec2::new(cell_w, cell_h),
            );
            painter.rect_filled(cell_r, 0.0, p.bg_main);
            for (gi, imp) in imps.iter().enumerate() {
                if imp.covers(minterm, n) {
                    painter.rect_filled(cell_r.shrink(1.0), 0.0, group_colors[gi % group_colors.len()]);
                    break;
                }
            }
            painter.rect_stroke(cell_r, 0.0, Stroke::new(1.0_f32, p.border));
            let val_str = match val {
                1 => "1",
                2 => "X",
                _ => "0",
            };
            let v_col = match val {
                1 => p.green,
                2 => p.orange,
                _ => p.text_dim,
            };
            painter.text(
                cell_r.center(),
                Align2::CENTER_CENTER,
                val_str,
                FontId::new(10.5 * scale, FontFamily::Monospace),
                v_col,
            );
        }
    }
    card_rect
}

/// Draws a movable Boolean Function (SOP / POS) card directly on the circuit canvas and returns its `Rect`
pub fn draw_kmap_function_on_canvas(
    painter: &egui::Painter,
    screen_pos: Pos2,
    px_per_world: f32,
    kmap: &TruthTableKMap,
    dark_mode: bool,
    is_dragging: bool,
) -> Rect {
    let p = crate::theme::palette(dark_mode);
    let scale = (px_per_world / 12.5).clamp(0.65, 1.45);
    let (_, sop_expr) = kmap.solve_minimal_sop();
    let pos_expr = kmap.solve_minimal_pos();

    let max_chars = sop_expr.chars().count().max(pos_expr.chars().count()).max(18) as f32;
    let w = (max_chars * 7.2 * scale + 28.0 * scale).max(175.0 * scale);
    let h = 48.0 * scale;
    let pad = 10.0 * scale;

    let card_rect = Rect::from_min_size(screen_pos, Vec2::new(w, h));
    let bg = p.bg_secondary;
    let border = if is_dragging { p.cyan } else { p.green };

    painter.rect_filled(card_rect, crate::theme::RADIUS, bg);
    painter.rect_filled(
        Rect::from_min_size(card_rect.min, Vec2::new(3.0, h)),
        crate::theme::RADIUS,
        border,
    );
    painter.rect_stroke(
        card_rect,
        crate::theme::RADIUS,
        Stroke::new(if is_dragging { 2.0_f32 } else { 1.0_f32 }, border),
    );

    painter.text(
        Pos2::new(card_rect.left() + pad, card_rect.top() + 15.0 * scale),
        Align2::LEFT_CENTER,
        format!("SOP: {sop_expr}"),
        FontId::new(10.5 * scale, FontFamily::Monospace),
        p.green,
    );
    painter.text(
        Pos2::new(card_rect.left() + pad, card_rect.top() + 33.0 * scale),
        Align2::LEFT_CENTER,
        format!("POS: {pos_expr}"),
        FontId::new(10.0 * scale, FontFamily::Monospace),
        p.cyan,
    );

    card_rect
}
