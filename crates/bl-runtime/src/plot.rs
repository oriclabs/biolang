use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

pub fn plot_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("plot", Arity::Range(1, 2)),
        ("heatmap", Arity::Range(1, 2)),
        ("histogram", Arity::Range(1, 2)),
        ("volcano", Arity::Range(1, 2)),
        ("ma_plot", Arity::Range(1, 2)),
        ("save_svg", Arity::Exact(2)),
        ("save_plot", Arity::Exact(2)),
        ("genome_track", Arity::Range(1, 2)),
    ]
}

pub fn is_plot_builtin(name: &str) -> bool {
    matches!(
        name,
        "plot" | "heatmap" | "histogram" | "volcano" | "ma_plot" | "save_svg" | "save_plot" | "genome_track"
    )
}

/// Normalize single-Record calling convention for plot functions.
/// `func({data: table, title: "..."})` → `func(table, {title: "..."})`
/// `func({values: [...], bins: 8})` → `func([...], {bins: 8})`
fn normalize_plot_args(args: Vec<Value>) -> Vec<Value> {
    if args.len() == 1 {
        if let Value::Record(ref map) = args[0] {
            // Try "data" first, then "values" as the primary data key
            for key in &["data", "values"] {
                if let Some(data) = map.get(*key) {
                    let mut opts = map.clone();
                    opts.remove(*key);
                    if opts.is_empty() {
                        return vec![data.clone()];
                    }
                    return vec![data.clone(), Value::Record(opts)];
                }
            }
        }
    }
    args
}

pub fn call_plot_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    let args = normalize_plot_args(args);
    match name {
        "plot" => builtin_plot(args),
        "heatmap" => builtin_heatmap(args),
        "histogram" => builtin_histogram(args),
        "volcano" => builtin_volcano(args),
        "ma_plot" => builtin_ma_plot(args),
        "save_svg" | "save_plot" => builtin_save_svg(args),
        "genome_track" => builtin_genome_track(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown plot builtin '{name}'"),
            None,
        )),
    }
}

// ── SVG Infrastructure ──────────────────────────────────────────

pub(crate) const PALETTE: [&str; 8] = [
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
];

pub(crate) fn sequential_color(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (64.0 + t * 191.0) as u8;
    let g = (64.0 + (1.0 - (2.0 * t - 1.0).abs()) * 128.0) as u8;
    let b = (255.0 - t * 191.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

pub(crate) struct Scale {
    pub(crate) domain: (f64, f64),
    pub(crate) range: (f64, f64),
}

impl Scale {
    pub(crate) fn map(&self, v: f64) -> f64 {
        if (self.domain.1 - self.domain.0).abs() < f64::EPSILON {
            return (self.range.0 + self.range.1) / 2.0;
        }
        let t = (v - self.domain.0) / (self.domain.1 - self.domain.0);
        self.range.0 + t * (self.range.1 - self.range.0)
    }

    pub(crate) fn nice_ticks(&self, count: usize) -> Vec<f64> {
        let step = (self.domain.1 - self.domain.0) / count as f64;
        (0..=count).map(|i| self.domain.0 + step * i as f64).collect()
    }
}

pub(crate) struct SvgCanvas {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) margin: Margin,
    pub(crate) elements: Vec<String>,
}

pub(crate) struct Margin {
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
    pub(crate) left: f64,
}

impl Default for Margin {
    fn default() -> Self {
        Self { top: 40.0, right: 20.0, bottom: 50.0, left: 60.0 }
    }
}

impl SvgCanvas {
    pub(crate) fn new(width: f64, height: f64) -> Self {
        Self { width, height, margin: Margin::default(), elements: Vec::new() }
    }

    pub(crate) fn plot_width(&self) -> f64 { self.width - self.margin.left - self.margin.right }
    pub(crate) fn plot_height(&self) -> f64 { self.height - self.margin.top - self.margin.bottom }

    pub(crate) fn add_rect(&mut self, x: f64, y: f64, w: f64, h: f64, fill: &str) {
        self.elements.push(format!(
            r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="{fill}" />"#
        ));
    }

    pub(crate) fn add_circle(&mut self, cx: f64, cy: f64, r: f64, fill: &str) {
        self.elements.push(format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" opacity="0.7" />"#
        ));
    }

    pub(crate) fn add_line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, stroke: &str, width: f64) {
        self.elements.push(format!(
            r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke}" stroke-width="{width}" />"#
        ));
    }

    pub(crate) fn add_text(&mut self, x: f64, y: f64, text: &str, anchor: &str, size: f64) {
        let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="sans-serif">{escaped}</text>"#
        ));
    }

    pub(crate) fn add_text_rotated(&mut self, x: f64, y: f64, text: &str, angle: f64, anchor: &str, size: f64) {
        let escaped = text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;");
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="sans-serif" transform="rotate({angle},{x:.1},{y:.1})">{escaped}</text>"#
        ));
    }

    pub(crate) fn draw_x_axis(&mut self, scale: &Scale, label: &str) {
        let y = self.margin.top + self.plot_height();
        self.add_line(self.margin.left, y, self.margin.left + self.plot_width(), y, "#333", 1.0);
        let x_scale = Scale { domain: scale.domain, range: (self.margin.left, self.margin.left + self.plot_width()) };
        for tick in scale.nice_ticks(5) {
            let x = x_scale.map(tick);
            self.add_line(x, y, x, y + 5.0, "#333", 1.0);
            self.add_text(x, y + 18.0, &format!("{tick:.1}"), "middle", 11.0);
        }
        self.add_text(self.margin.left + self.plot_width() / 2.0, self.height - 5.0, label, "middle", 13.0);
    }

    pub(crate) fn draw_y_axis(&mut self, scale: &Scale, label: &str) {
        let x = self.margin.left;
        self.add_line(x, self.margin.top, x, self.margin.top + self.plot_height(), "#333", 1.0);
        let y_scale = Scale { domain: scale.domain, range: (self.margin.top + self.plot_height(), self.margin.top) };
        for tick in scale.nice_ticks(5) {
            let y = y_scale.map(tick);
            self.add_line(x - 5.0, y, x, y, "#333", 1.0);
            self.add_text(x - 8.0, y + 4.0, &format!("{tick:.1}"), "end", 11.0);
        }
        self.add_text_rotated(15.0, self.margin.top + self.plot_height() / 2.0, label, -90.0, "middle", 13.0);
    }

    pub(crate) fn draw_title(&mut self, title: &str) {
        self.add_text(self.width / 2.0, 25.0, title, "middle", 16.0);
    }

    pub(crate) fn render(&self) -> String {
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}">"#,
            self.width, self.height, self.width, self.height
        );
        svg.push_str(&format!(
            r#"<rect width="{}" height="{}" fill="white" />"#,
            self.width, self.height
        ));
        for el in &self.elements {
            svg.push_str(el);
        }
        svg.push_str("</svg>");
        svg
    }
}

// ── Option parsing helpers ──────────────────────────────────────

pub(crate) fn get_opt_str<'a>(opts: &'a HashMap<String, Value>, key: &str, default: &'a str) -> &'a str {
    opts.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or(default)
}

pub(crate) fn get_opt_f64(opts: &HashMap<String, Value>, key: &str, default: f64) -> f64 {
    opts.get(key)
        .and_then(|v| v.as_float())
        .unwrap_or(default)
}

pub(crate) fn parse_options(args: &[Value]) -> HashMap<String, Value> {
    if args.len() > 1 {
        if let Value::Record(map) = &args[1] {
            return map.clone();
        }
    }
    HashMap::new()
}

pub(crate) fn extract_table_col(table: &Table, col: &str) -> Result<Vec<f64>> {
    let idx = table.col_index(col).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, format!("column '{col}' not found"), None)
    })?;
    let mut vals = Vec::with_capacity(table.num_rows());
    for row in &table.rows {
        match &row[idx] {
            Value::Int(n) => vals.push(*n as f64),
            Value::Float(f) => vals.push(*f),
            Value::Str(s) => vals.push(s.parse::<f64>().unwrap_or(f64::NAN)),
            _ => vals.push(f64::NAN),
        }
    }
    Ok(vals)
}

pub(crate) fn col_range(vals: &[f64]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in vals {
        if v.is_finite() {
            if v < min { min = v; }
            if v > max { max = v; }
        }
    }
    if min > max { (0.0, 1.0) } else { (min, max) }
}

pub(crate) fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table, got {}", other.type_of()), None,
        )),
    }
}

// ── Builtins ────────────────────────────────────────────────────

fn builtin_plot(args: Vec<Value>) -> Result<Value> {
    // Handle Record with x/y lists: plot({x: [...], y: [...], title: "..."})
    let args = if args.len() == 1 {
        if let Value::Record(ref map) = args[0] {
            if map.contains_key("x") && map.contains_key("y") {
                if let (Value::List(xv), Value::List(yv)) = (&map["x"], &map["y"]) {
                    let rows: Vec<Vec<Value>> = xv.iter().zip(yv.iter())
                        .map(|(x, y)| vec![x.clone(), y.clone()]).collect();
                    let table = Value::Table(Table::new(vec!["x".into(), "y".into()], rows));
                    let mut opts = map.clone();
                    opts.remove("x");
                    opts.remove("y");
                    if opts.is_empty() { vec![table] } else { vec![table, Value::Record(opts)] }
                } else { args }
            } else { args }
        } else { args }
    } else { args };

    let opts = parse_options(&args);
    let plot_type = get_opt_str(&opts, "type", "scatter").to_string();
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "").to_string();

    let table = require_table(&args[0], "plot")?;

    if table.num_cols() < 2 {
        return Err(BioLangError::runtime(ErrorKind::TypeError, "plot() requires table with at least 2 columns", None));
    }

    let x_col = get_opt_str(&opts, "x", &table.columns[0]).to_string();
    let y_col = get_opt_str(&opts, "y", &table.columns[1]).to_string();

    let xs = extract_table_col(table, &x_col)?;
    let ys = extract_table_col(table, &y_col)?;

    let (x_min, x_max) = col_range(&xs);
    let (y_min, y_max) = col_range(&ys);

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale { domain: (x_min, x_max), range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()) };
    let y_scale = Scale { domain: (y_min, y_max), range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top) };

    match plot_type.as_str() {
        "scatter" => {
            for i in 0..xs.len() {
                if xs[i].is_finite() && ys[i].is_finite() {
                    canvas.add_circle(x_scale.map(xs[i]), y_scale.map(ys[i]), 4.0, PALETTE[0]);
                }
            }
        }
        "line" => {
            let points: Vec<String> = xs.iter().zip(ys.iter())
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .map(|(x, y)| format!("{:.1},{:.1}", x_scale.map(*x), y_scale.map(*y)))
                .collect();
            if !points.is_empty() {
                canvas.elements.push(format!(
                    r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
                    points.join(" "), PALETTE[0]
                ));
            }
        }
        "bar" => {
            let bar_w = canvas.plot_width() / xs.len() as f64 * 0.8;
            let gap = canvas.plot_width() / xs.len() as f64 * 0.1;
            let baseline = y_scale.map(0.0f64.max(y_min));
            for (i, (&_x, &y)) in xs.iter().zip(ys.iter()).enumerate() {
                let bx = canvas.margin.left + gap + i as f64 * (bar_w + 2.0 * gap);
                let by = y_scale.map(y);
                let bh = (baseline - by).abs();
                let top = by.min(baseline);
                canvas.add_rect(bx, top, bar_w, bh, PALETTE[i % PALETTE.len()]);
            }
        }
        "box" => {
            // Box plot per numeric column
            for (ci, col) in table.columns.iter().enumerate() {
                let vals = extract_table_col(table, col)?;
                let mut sorted: Vec<f64> = vals.into_iter().filter(|v| v.is_finite()).collect();
                if sorted.is_empty() { continue; }
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let n = sorted.len();
                let q1 = sorted[n / 4];
                let med = sorted[n / 2];
                let q3 = sorted[3 * n / 4];
                let lo = sorted[0];
                let hi = sorted[n - 1];

                let bx = canvas.margin.left + (ci as f64 + 0.2) * canvas.plot_width() / table.num_cols() as f64;
                let bw = canvas.plot_width() / table.num_cols() as f64 * 0.6;

                canvas.add_rect(bx, y_scale.map(q3), bw, (y_scale.map(q1) - y_scale.map(q3)).abs(), PALETTE[ci % PALETTE.len()]);
                canvas.add_line(bx, y_scale.map(med), bx + bw, y_scale.map(med), "#333", 2.0);
                canvas.add_line(bx + bw / 2.0, y_scale.map(q3), bx + bw / 2.0, y_scale.map(hi), "#333", 1.0);
                canvas.add_line(bx + bw / 2.0, y_scale.map(q1), bx + bw / 2.0, y_scale.map(lo), "#333", 1.0);
            }
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("plot() unknown type '{plot_type}', expected scatter/line/bar/box"),
                None,
            ));
        }
    }

    let d_x_scale = Scale { domain: (x_min, x_max), range: (x_min, x_max) };
    let d_y_scale = Scale { domain: (y_min, y_max), range: (y_min, y_max) };
    canvas.draw_x_axis(&d_x_scale, &x_col);
    canvas.draw_y_axis(&d_y_scale, &y_col);
    if !title.is_empty() { canvas.draw_title(&title); }

    Ok(Value::Str(canvas.render()))
}

// ── Heatmap color schemes ──────────────────────────────────────

fn interpolate_viridis(t: f64) -> String {
    // Viridis: dark purple → teal → yellow (5-stop approximation)
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (68.0, 1.0, 84.0),     // 0.00 — dark purple
        (59.0, 82.0, 139.0),   // 0.25 — blue-purple
        (33.0, 145.0, 140.0),  // 0.50 — teal
        (94.0, 201.0, 98.0),   // 0.75 — green
        (253.0, 231.0, 37.0),  // 1.00 — yellow
    ];
    heatmap_interp_stops(t, &stops)
}

fn interpolate_plasma(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (13.0, 8.0, 135.0),    // deep blue
        (126.0, 3.0, 168.0),   // purple
        (204.0, 71.0, 120.0),  // pink
        (248.0, 149.0, 64.0),  // orange
        (240.0, 249.0, 33.0),  // yellow
    ];
    heatmap_interp_stops(t, &stops)
}

fn interpolate_inferno(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (0.0, 0.0, 4.0),       // black
        (87.0, 16.0, 110.0),   // dark purple
        (188.0, 55.0, 84.0),   // red
        (249.0, 142.0, 9.0),   // orange
        (252.0, 255.0, 164.0), // light yellow
    ];
    heatmap_interp_stops(t, &stops)
}

fn interpolate_rdbu(t: f64) -> String {
    // Diverging: blue (low) → white (mid) → red (high)
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (33.0, 102.0, 172.0),  // strong blue
        (146.0, 197.0, 222.0), // light blue
        (247.0, 247.0, 247.0), // white/near-white
        (239.0, 138.0, 98.0),  // light red
        (178.0, 24.0, 43.0),   // strong red
    ];
    heatmap_interp_stops(t, &stops)
}

fn interpolate_blues(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (247.0 - t * 239.0) as u8;
    let g = (251.0 - t * 183.0) as u8;
    let b = (255.0 - t * 69.0) as u8;
    format!("rgb({r},{g},{b})")
}

fn interpolate_reds(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (255.0 - t * 52.0) as u8;
    let g = (245.0 - t * 227.0) as u8;
    let b = (240.0 - t * 240.0) as u8;
    format!("rgb({r},{g},{b})")
}

fn interpolate_greens(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (247.0 - t * 247.0) as u8;
    let g = (252.0 - t * 102.0) as u8;
    let b = (245.0 - t * 200.0) as u8;
    format!("rgb({r},{g},{b})")
}

/// Linearly interpolate between N evenly-spaced color stops.
fn heatmap_interp_stops(t: f64, stops: &[(f64, f64, f64)]) -> String {
    let n = stops.len();
    if n == 0 {
        return "rgb(128,128,128)".into();
    }
    if n == 1 {
        let (r, g, b) = stops[0];
        return format!("rgb({},{},{})", r as u8, g as u8, b as u8);
    }
    let t = t.clamp(0.0, 1.0);
    let seg = t * (n - 1) as f64;
    let i = (seg.floor() as usize).min(n - 2);
    let f = seg - i as f64;
    let (r0, g0, b0) = stops[i];
    let (r1, g1, b1) = stops[i + 1];
    let r = (r0 + f * (r1 - r0)) as u8;
    let g = (g0 + f * (g1 - g0)) as u8;
    let b = (b0 + f * (b1 - b0)) as u8;
    format!("rgb({r},{g},{b})")
}

fn heatmap_color(t: f64, scheme: &str) -> String {
    match scheme {
        "viridis" => interpolate_viridis(t),
        "plasma" => interpolate_plasma(t),
        "inferno" => interpolate_inferno(t),
        "rdbu" => interpolate_rdbu(t),
        "blues" => interpolate_blues(t),
        "reds" => interpolate_reds(t),
        "greens" => interpolate_greens(t),
        _ => interpolate_viridis(t),
    }
}

/// Text color for readability: white on dark cells, black on light cells.
fn heatmap_text_color(t: f64, scheme: &str) -> &'static str {
    match scheme {
        "rdbu" => {
            // mid-range is white/light, extremes are dark
            if t < 0.25 || t > 0.75 { "white" } else { "#333" }
        }
        "blues" | "greens" | "reds" => {
            if t > 0.6 { "white" } else { "#333" }
        }
        // viridis, plasma, inferno: dark at low end, bright at high end
        _ => {
            if t < 0.55 { "white" } else { "#333" }
        }
    }
}

/// Simple row clustering by sorting rows by their mean value.
fn cluster_rows(row_data: &mut Vec<Vec<f64>>, row_labels: &mut Vec<String>) {
    let mut indices: Vec<usize> = (0..row_data.len()).collect();
    indices.sort_by(|&a, &b| {
        let mean_a: f64 = row_data[a].iter().copied().filter(|v| v.is_finite()).sum::<f64>()
            / row_data[a].len().max(1) as f64;
        let mean_b: f64 = row_data[b].iter().copied().filter(|v| v.is_finite()).sum::<f64>()
            / row_data[b].len().max(1) as f64;
        mean_a.partial_cmp(&mean_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    let orig_rows = row_data.clone();
    let orig_labels = row_labels.clone();
    for (new_i, &old_i) in indices.iter().enumerate() {
        row_data[new_i] = orig_rows[old_i].clone();
        if old_i < orig_labels.len() {
            row_labels[new_i] = orig_labels[old_i].clone();
        }
    }
}

fn builtin_heatmap(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Heatmap").to_string();
    let scheme = get_opt_str(&opts, "colors", "viridis").to_string();
    let show_values = opts.get("show_values")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let do_cluster = opts.get("cluster")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // User-supplied row/col labels
    let user_row_labels: Option<Vec<String>> = opts.get("row_labels").and_then(|v| {
        if let Value::List(items) = v {
            Some(items.iter().map(|i| format!("{i}")).collect())
        } else {
            None
        }
    });
    let user_col_labels: Option<Vec<String>> = opts.get("col_labels").and_then(|v| {
        if let Value::List(items) = v {
            Some(items.iter().map(|i| format!("{i}")).collect())
        } else {
            None
        }
    });

    // Extract data into row-major matrix: row_data[row][col]
    let (mut col_labels, mut row_data, mut row_labels) = match &args[0] {
        Value::Table(table) => {
            let cl = table.columns.clone();
            let mut rd: Vec<Vec<f64>> = Vec::with_capacity(table.num_rows());
            let mut rl: Vec<String> = Vec::with_capacity(table.num_rows());
            for (ri, row) in table.rows.iter().enumerate() {
                let mut rv = Vec::with_capacity(row.len());
                for val in row {
                    rv.push(match val {
                        Value::Int(n) => *n as f64,
                        Value::Float(f) => *f,
                        Value::Str(s) => s.parse::<f64>().unwrap_or(f64::NAN),
                        _ => f64::NAN,
                    });
                }
                rd.push(rv);
                rl.push(format!("{}", ri + 1));
            }
            (cl, rd, rl)
        }
        Value::Matrix(m) => {
            let cl = m.col_names.clone()
                .unwrap_or_else(|| (0..m.ncol).map(|i| format!("col{i}")).collect());
            let mut rd = Vec::with_capacity(m.nrow);
            let rl: Vec<String> = m.row_names.clone()
                .unwrap_or_else(|| (0..m.nrow).map(|i| format!("{}", i + 1)).collect());
            for r in 0..m.nrow {
                let row_start = r * m.ncol;
                rd.push(m.data[row_start..row_start + m.ncol].to_vec());
            }
            (cl, rd, rl)
        }
        Value::List(items) => {
            // List of Lists (matrix) or List of Records
            if items.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError, "heatmap() received empty list", None,
                ));
            }
            match &items[0] {
                Value::List(_) => {
                    // List of Lists
                    let mut rd = Vec::with_capacity(items.len());
                    let mut max_cols = 0usize;
                    for item in items {
                        if let Value::List(row) = item {
                            let rv: Vec<f64> = row.iter().map(|v| match v {
                                Value::Int(n) => *n as f64,
                                Value::Float(f) => *f,
                                _ => f64::NAN,
                            }).collect();
                            if rv.len() > max_cols { max_cols = rv.len(); }
                            rd.push(rv);
                        } else {
                            return Err(BioLangError::type_error(
                                "heatmap() list items must all be Lists or Records", None,
                            ));
                        }
                    }
                    let cl: Vec<String> = (0..max_cols).map(|i| format!("col{i}")).collect();
                    let rl: Vec<String> = (0..rd.len()).map(|i| format!("{}", i + 1)).collect();
                    (cl, rd, rl)
                }
                Value::Record(_) => {
                    // List of Records — collect all field names as columns
                    let mut all_keys = Vec::new();
                    let mut key_set = std::collections::HashSet::new();
                    for item in items {
                        if let Value::Record(map) = item {
                            for k in map.keys() {
                                if key_set.insert(k.clone()) {
                                    all_keys.push(k.clone());
                                }
                            }
                        }
                    }
                    let mut rd = Vec::with_capacity(items.len());
                    for item in items {
                        if let Value::Record(map) = item {
                            let rv: Vec<f64> = all_keys.iter().map(|k| {
                                map.get(k).map(|v| match v {
                                    Value::Int(n) => *n as f64,
                                    Value::Float(f) => *f,
                                    _ => f64::NAN,
                                }).unwrap_or(f64::NAN)
                            }).collect();
                            rd.push(rv);
                        }
                    }
                    let rl: Vec<String> = (0..rd.len()).map(|i| format!("{}", i + 1)).collect();
                    (all_keys, rd, rl)
                }
                _ => return Err(BioLangError::type_error(
                    "heatmap() requires Table, Matrix, List of Lists, or List of Records", None,
                )),
            }
        }
        _ => return Err(BioLangError::type_error(
            "heatmap() requires Table, Matrix, List of Lists, or List of Records", None,
        )),
    };

    // Apply user-supplied labels if given
    if let Some(ul) = user_row_labels {
        for (i, label) in ul.into_iter().enumerate() {
            if i < row_labels.len() {
                row_labels[i] = label;
            }
        }
    }
    if let Some(ul) = user_col_labels {
        col_labels = ul;
    }

    let nrows = row_data.len();
    let ncols = if nrows > 0 { row_data[0].len() } else { col_labels.len() };

    if nrows == 0 || ncols == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError, "heatmap() received empty data", None,
        ));
    }

    // Optional clustering (sort rows by mean)
    if do_cluster {
        cluster_rows(&mut row_data, &mut row_labels);
    }

    // Compute global min/max
    let mut all_vals = Vec::new();
    for row in &row_data {
        for &v in row {
            if v.is_finite() { all_vals.push(v); }
        }
    }
    let (vmin, vmax) = col_range(&all_vals);

    // Compute margins based on label lengths
    let max_row_label_len = row_labels.iter().map(|s| s.len()).max().unwrap_or(0);
    let left_margin = 40.0 + (max_row_label_len as f64 * 7.0).min(120.0);
    let legend_width = 60.0;

    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = left_margin;
    canvas.margin.bottom = 70.0;
    canvas.margin.right = 20.0 + legend_width;
    canvas.margin.top = if title.is_empty() { 20.0 } else { 45.0 };

    let plot_w = canvas.plot_width();
    let plot_h = canvas.plot_height();
    let cell_w = plot_w / ncols as f64;
    let cell_h = plot_h / nrows as f64;

    // Draw cells
    for (ri, row) in row_data.iter().enumerate() {
        for (ci, &v) in row.iter().enumerate() {
            let t = if (vmax - vmin).abs() < f64::EPSILON { 0.5 } else { (v - vmin) / (vmax - vmin) };
            let color = if v.is_finite() {
                heatmap_color(t, &scheme)
            } else {
                "#cccccc".to_string()
            };
            let x = canvas.margin.left + ci as f64 * cell_w;
            let y = canvas.margin.top + ri as f64 * cell_h;
            canvas.add_rect(x, y, cell_w, cell_h, &color);

            // Cell border for visual separation
            canvas.elements.push(format!(
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#eee\" stroke-width=\"0.5\" />",
                x, y, cell_w, cell_h
            ));

            // Show numeric value in cell
            if show_values && v.is_finite() {
                let txt_color = heatmap_text_color(t, &scheme);
                let label = if v.abs() >= 100.0 || v == 0.0 {
                    format!("{:.0}", v)
                } else if v.abs() >= 1.0 {
                    format!("{:.1}", v)
                } else {
                    format!("{:.2}", v)
                };
                let font_size = (cell_w.min(cell_h) * 0.35).clamp(7.0, 14.0);
                canvas.elements.push(format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="central" font-size="{:.1}" font-family="sans-serif" fill="{}">{}</text>"#,
                    x + cell_w / 2.0, y + cell_h / 2.0, font_size, txt_color,
                    label.replace('&', "&amp;").replace('<', "&lt;")
                ));
            }
        }
    }

    // Column labels (rotated at bottom)
    for (ci, col) in col_labels.iter().enumerate() {
        if ci < ncols {
            let x = canvas.margin.left + (ci as f64 + 0.5) * cell_w;
            let y = canvas.margin.top + plot_h + 10.0;
            canvas.add_text_rotated(x, y, col, 45.0, "start", 10.0);
        }
    }

    // Row labels (on the left)
    for (ri, label) in row_labels.iter().enumerate() {
        if ri < nrows {
            let y = canvas.margin.top + (ri as f64 + 0.5) * cell_h + 4.0;
            canvas.add_text(canvas.margin.left - 6.0, y, label, "end", 10.0);
        }
    }

    // Color legend / scale bar (right side)
    let legend_x = canvas.margin.left + plot_w + 15.0;
    let legend_top = canvas.margin.top;
    let legend_h = plot_h.min(200.0);
    let legend_bar_w = 15.0;
    let legend_steps = 50usize;
    let step_h = legend_h / legend_steps as f64;
    for i in 0..legend_steps {
        let t = 1.0 - (i as f64 / (legend_steps - 1) as f64); // top = max
        let color = heatmap_color(t, &scheme);
        let y = legend_top + i as f64 * step_h;
        canvas.elements.push(format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" />"#,
            legend_x, y, legend_bar_w, step_h + 0.5, color
        ));
    }
    // Legend border
    canvas.elements.push(format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#333\" stroke-width=\"0.5\" />",
        legend_x, legend_top, legend_bar_w, legend_h
    ));
    // Legend tick labels
    let label_x = legend_x + legend_bar_w + 5.0;
    canvas.add_text(label_x, legend_top + 4.0, &format!("{vmax:.2}"), "start", 9.0);
    canvas.add_text(label_x, legend_top + legend_h / 2.0 + 3.0,
        &format!("{:.2}", (vmin + vmax) / 2.0), "start", 9.0);
    canvas.add_text(label_x, legend_top + legend_h + 3.0, &format!("{vmin:.2}"), "start", 9.0);

    // Title
    if !title.is_empty() {
        canvas.draw_title(&title);
    }

    Ok(Value::Str(canvas.render()))
}

fn builtin_histogram(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let bins = get_opt_f64(&opts, "bins", 20.0) as usize;
    let title = get_opt_str(&opts, "title", "Histogram").to_string();

    let nums = match &args[0] {
        Value::List(items) => {
            let mut v = Vec::new();
            for item in items {
                match item {
                    Value::Int(n) => v.push(*n as f64),
                    Value::Float(f) => v.push(*f),
                    Value::Str(s) => {
                        if let Ok(f) = s.parse::<f64>() {
                            v.push(f);
                        }
                    }
                    _ => {}
                }
            }
            v
        }
        _ => return Err(BioLangError::type_error("histogram() requires List of numbers", None)),
    };

    if nums.is_empty() {
        return Err(BioLangError::runtime(ErrorKind::TypeError, "histogram() received no numeric values — check that your data contains numbers, not strings", None));
    }

    let (lo, hi) = col_range(&nums);
    let bin_w = if (hi - lo).abs() < f64::EPSILON { 1.0 } else { (hi - lo) / bins as f64 };
    let mut counts = vec![0usize; bins];
    for &v in &nums {
        let mut idx = ((v - lo) / bin_w) as usize;
        if idx >= bins { idx = bins - 1; }
        counts[idx] += 1;
    }
    let max_count = *counts.iter().max().unwrap_or(&1);

    let mut canvas = SvgCanvas::new(width, height);
    let y_scale = Scale { domain: (0.0, max_count as f64), range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top) };

    let rect_w = canvas.plot_width() / bins as f64;
    for (i, &count) in counts.iter().enumerate() {
        let x = canvas.margin.left + i as f64 * rect_w;
        let y = y_scale.map(count as f64);
        let h = canvas.margin.top + canvas.plot_height() - y;
        canvas.add_rect(x, y, rect_w - 1.0, h, PALETTE[0]);
    }

    let d_x_scale = Scale { domain: (lo, hi), range: (lo, hi) };
    let d_y_scale = Scale { domain: (0.0, max_count as f64), range: (0.0, max_count as f64) };
    canvas.draw_x_axis(&d_x_scale, "Value");
    canvas.draw_y_axis(&d_y_scale, "Count");
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}

fn builtin_volcano(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "volcano")?;
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let fc_col = get_opt_str(&opts, "fc", "log2fc").to_string();
    let p_col = get_opt_str(&opts, "p", "pvalue").to_string();
    let fc_thresh = get_opt_f64(&opts, "fc_threshold", 1.0);
    let p_thresh = get_opt_f64(&opts, "p_threshold", 0.05);

    let fcs = extract_table_col(table, &fc_col)?;
    let pvals = extract_table_col(table, &p_col)?;

    let neg_log_p: Vec<f64> = pvals.iter().map(|&p| if p > 0.0 { -(p.log10()) } else { 0.0 }).collect();

    let (x_min, x_max) = col_range(&fcs);
    let x_abs = x_min.abs().max(x_max.abs());
    let (_, y_max) = col_range(&neg_log_p);

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale { domain: (-x_abs, x_abs), range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()) };
    let y_scale = Scale { domain: (0.0, y_max), range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top) };

    let neg_log_p_thresh = -(p_thresh.log10());

    // Threshold lines
    canvas.add_line(x_scale.map(-fc_thresh), canvas.margin.top, x_scale.map(-fc_thresh), canvas.margin.top + canvas.plot_height(), "#ccc", 1.0);
    canvas.add_line(x_scale.map(fc_thresh), canvas.margin.top, x_scale.map(fc_thresh), canvas.margin.top + canvas.plot_height(), "#ccc", 1.0);
    canvas.add_line(canvas.margin.left, y_scale.map(neg_log_p_thresh), canvas.margin.left + canvas.plot_width(), y_scale.map(neg_log_p_thresh), "#ccc", 1.0);

    for i in 0..fcs.len() {
        let color = if neg_log_p[i] > neg_log_p_thresh && fcs[i].abs() > fc_thresh {
            if fcs[i] > 0.0 { "#e15759" } else { "#4e79a7" }
        } else {
            "#999"
        };
        canvas.add_circle(x_scale.map(fcs[i]), y_scale.map(neg_log_p[i]), 3.0, color);
    }

    let d_x_scale = Scale { domain: (-x_abs, x_abs), range: (-x_abs, x_abs) };
    let d_y_scale = Scale { domain: (0.0, y_max), range: (0.0, y_max) };
    canvas.draw_x_axis(&d_x_scale, &format!("log2(FC) [{fc_col}]"));
    canvas.draw_y_axis(&d_y_scale, &format!("-log10(p) [{p_col}]"));
    canvas.draw_title("Volcano Plot");

    Ok(Value::Str(canvas.render()))
}

fn builtin_ma_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "ma_plot")?;
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let a_col = get_opt_str(&opts, "a", "baseMean").to_string();
    let m_col = get_opt_str(&opts, "m", "log2fc").to_string();

    let a_vals = extract_table_col(table, &a_col)?;
    let m_vals = extract_table_col(table, &m_col)?;

    // A = log2(mean), M = log2(fc) — assume already in log space if column name suggests
    let a_log: Vec<f64> = a_vals.iter().map(|&v| if v > 0.0 { v.log2() } else { 0.0 }).collect();

    let (x_min, x_max) = col_range(&a_log);
    let (y_min, y_max) = col_range(&m_vals);
    let y_abs = y_min.abs().max(y_max.abs());

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale { domain: (x_min, x_max), range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()) };
    let y_scale = Scale { domain: (-y_abs, y_abs), range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top) };

    // Zero line
    canvas.add_line(canvas.margin.left, y_scale.map(0.0), canvas.margin.left + canvas.plot_width(), y_scale.map(0.0), "#ccc", 1.0);

    for i in 0..a_log.len() {
        let color = if m_vals[i].abs() > 1.0 { "#e15759" } else { "#999" };
        canvas.add_circle(x_scale.map(a_log[i]), y_scale.map(m_vals[i]), 3.0, color);
    }

    let d_x_scale = Scale { domain: (x_min, x_max), range: (x_min, x_max) };
    let d_y_scale = Scale { domain: (-y_abs, y_abs), range: (-y_abs, y_abs) };
    canvas.draw_x_axis(&d_x_scale, &format!("A (log2 {a_col})"));
    canvas.draw_y_axis(&d_y_scale, &format!("M ({m_col})"));
    canvas.draw_title("MA Plot");

    Ok(Value::Str(canvas.render()))
}

fn builtin_save_svg(args: Vec<Value>) -> Result<Value> {
    let svg = match &args[0] {
        Value::Str(s) => s,
        Value::Nil => return Err(BioLangError::type_error(
            "save_svg()/save_plot() received Nil — the plot function before the pipe likely failed or returned nothing".to_string(), None,
        )),
        other => return Err(BioLangError::type_error(
            format!("save_svg() requires Str (SVG), got {}", other.type_of()), None,
        )),
    };
    let path = match &args[1] {
        Value::Str(s) => s,
        other => return Err(BioLangError::type_error(
            format!("save_svg() requires Str (path), got {}", other.type_of()), None,
        )),
    };
    std::fs::write(path, svg).map_err(|e| {
        BioLangError::runtime(ErrorKind::IOError, format!("save_svg() write failed: {e}"), None)
    })?;
    Ok(Value::Str(path.clone()))
}

fn builtin_genome_track(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "genome_track")?;
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 1000.0);
    let height = get_opt_f64(&opts, "height", 300.0);
    let title = get_opt_str(&opts, "title", "Genome Track").to_string();

    // Expect columns: chrom, start, end, [name], [strand]
    let starts = extract_table_col(table, "start")?;
    let ends = extract_table_col(table, "end")?;

    let name_idx = table.col_index("name");
    let strand_idx = table.col_index("strand");

    let global_start = starts.iter().cloned().fold(f64::INFINITY, f64::min);
    let global_end = ends.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.top = 50.0;
    canvas.margin.bottom = 40.0;

    let x_scale = Scale { domain: (global_start, global_end), range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()) };

    // Draw backbone
    let track_y = canvas.margin.top + canvas.plot_height() / 2.0;
    canvas.add_line(canvas.margin.left, track_y, canvas.margin.left + canvas.plot_width(), track_y, "#ccc", 2.0);

    // Draw features
    let feature_h = 16.0;
    for i in 0..starts.len() {
        let x1 = x_scale.map(starts[i]);
        let x2 = x_scale.map(ends[i]);
        let w = (x2 - x1).max(2.0);
        let color = PALETTE[i % PALETTE.len()];

        // Alternate vertical position to avoid overlap
        let y_off = if i % 2 == 0 { -feature_h - 2.0 } else { 4.0 };
        canvas.add_rect(x1, track_y + y_off, w, feature_h, color);

        // Direction arrow if strand info exists
        if let Some(si) = strand_idx {
            if let Value::Str(s) = &table.rows[i][si] {
                let arrow_x = if s == "+" { x2 } else { x1 };
                let arrow_y = track_y + y_off + feature_h / 2.0;
                let dx = if s == "+" { 6.0 } else { -6.0 };
                canvas.elements.push(format!(
                    r#"<polygon points="{:.1},{:.1} {:.1},{:.1} {:.1},{:.1}" fill="{color}" />"#,
                    arrow_x, arrow_y, arrow_x + dx, arrow_y - 4.0, arrow_x + dx, arrow_y + 4.0
                ));
            }
        }

        // Label
        if let Some(ni) = name_idx {
            if let Value::Str(name) = &table.rows[i][ni] {
                canvas.add_text(x1, track_y + y_off - 2.0, name, "start", 9.0);
            }
        }
    }

    let d_x_scale = Scale { domain: (global_start, global_end), range: (global_start, global_end) };
    canvas.draw_x_axis(&d_x_scale, "Position");
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}

