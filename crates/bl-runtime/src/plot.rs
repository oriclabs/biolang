use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

pub fn plot_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("plot", Arity::Range(1, 2)),
        ("heatmap", Arity::Range(1, 2)),
        ("histogram", Arity::Range(1, 2)),
        ("ecdf_plot", Arity::Range(1, 2)),
        ("density_plot", Arity::Range(1, 2)),
        ("volcano", Arity::Range(1, 2)),
        ("ma_plot", Arity::Range(1, 2)),
        ("save_svg", Arity::Exact(2)),
        ("save_plot", Arity::Exact(2)),
        ("save_png", Arity::Range(2, 3)),
        ("genome_track", Arity::Range(1, 2)),
    ]
}

pub fn is_plot_builtin(name: &str) -> bool {
    matches!(
        name,
        "plot"
            | "heatmap"
            | "histogram"
            | "ecdf_plot"
            | "density_plot"
            | "volcano"
            | "ma_plot"
            | "save_svg"
            | "save_plot"
            | "save_png"
            | "genome_track"
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
                    let mut opts = map.as_ref().clone();
                    opts.remove(*key);
                    if opts.is_empty() {
                        return vec![data.clone()];
                    }
                    return vec![data.clone(), Value::Record(opts.into())];
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
        "ecdf_plot" => builtin_ecdf_plot(args),
        "density_plot" => builtin_density_plot(args),
        "volcano" => builtin_volcano(args),
        "ma_plot" => builtin_ma_plot(args),
        "save_svg" | "save_plot" => builtin_save_svg(args),
        "save_png" => builtin_save_png(args),
        "genome_track" => builtin_genome_track(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown plot builtin '{name}'"),
            None,
        )),
    }
}

// ── SVG Infrastructure ──────────────────────────────────────────

/// Categorical colours, in the order they are handed out.
///
/// Callers index this modulo its length, so a plot with more groups than
/// colours silently reuses them. At eight that happened constantly: clustering
/// 2700 PBMCs gave eleven groups, and clusters 8, 9 and 10 came out the same
/// colours as 0, 1 and 2 - two different cell types sharing a colour on the one
/// figure the whole analysis is read from. Single-cell work routinely produces
/// fifteen to thirty clusters.
///
/// The first eight are unchanged, so existing figures keep the colours they
/// had. The rest extend Tableau's twenty with darker and lighter variants,
/// ordered so that adjacent entries stay distinguishable - neighbouring indices
/// are what a reader has to tell apart.
pub(crate) const PALETTE: [&str; 24] = [
    // Tableau 10 (the original eight, order preserved)
    "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
    // Tableau 10, remaining two
    "#9c755f", "#bab0ac", // Tableau 20 pairs, picked for contrast against the above
    "#a0cbe8", "#ffbe7d", "#8cd17d", "#b6992d", "#86bcb6", "#fabfd2", "#d37295", "#d4a6c8",
    // Deeper tones, so a long legend does not drift pale
    "#1b4965", "#7a4419", "#8b2e2e", "#2d6a4f", "#5a189a", "#0b525b",
];

/// A ggplot/Seurat-like discrete palette for single-cell figures.
///
/// This is opt-in through `{theme: "seurat"}` so generic BioLang plots retain
/// their existing colours. The ordering starts with the familiar ggplot hues
/// and then uses high-contrast extensions for datasets with many clusters.
pub(crate) const SEURAT_PALETTE: [&str; 24] = [
    "#f8766d", "#7cae00", "#00bfc4", "#c77cff", "#e58700", "#00ba38", "#619cff", "#f564e3",
    "#b79f00", "#00c08b", "#00a9ff", "#cd9600", "#7b61a8", "#00a08a", "#ff6f91", "#6a994e",
    "#1982c4", "#ff924c", "#8ac926", "#6a4c93", "#d81159", "#218380", "#fbb13c", "#5f0f40",
];

/// `#rrggbb` to premultiplied-ready RGBA, with the alpha add_circle applies.
///
/// The rastered and vector paths have to agree on colour as well as position,
/// or the same plot changes appearance when it crosses the raster threshold.
/// add_circle draws every point at opacity 0.7, so that is baked in here.
pub(crate) fn hex_to_rgba(hex: &str, alpha: f64) -> [u8; 4] {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    let channel = |at: usize| -> u8 {
        digits
            .get(at..at + 2)
            .and_then(|pair| u8::from_str_radix(pair, 16).ok())
            .unwrap_or(0)
    };
    if digits.len() < 6 {
        return [0, 0, 0, (alpha.clamp(0.0, 1.0) * 255.0) as u8];
    }
    [
        channel(0),
        channel(2),
        channel(4),
        (alpha.clamp(0.0, 1.0) * 255.0) as u8,
    ]
}

pub(crate) fn sequential_color(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (64.0 + t * 191.0) as u8;
    let g = (64.0 + (1.0 - (2.0 * t - 1.0).abs()) * 128.0) as u8;
    let b = (255.0 - t * 191.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Seurat FeaturePlot's familiar low-expression grey to high-expression blue.
pub(crate) fn seurat_feature_color(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let channel = |low: f64, high: f64| (low + t * (high - low)).round() as u8;
    let r = channel(211.0, 0.0);
    let g = channel(211.0, 0.0);
    let b = channel(211.0, 255.0);
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
        (0..=count)
            .map(|i| self.domain.0 + step * i as f64)
            .collect()
    }
}

pub(crate) struct SvgCanvas {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) margin: Margin,
    pub(crate) elements: Vec<String>,
    accessible_label: Option<String>,
}

pub(crate) struct Margin {
    pub(crate) top: f64,
    pub(crate) right: f64,
    pub(crate) bottom: f64,
    pub(crate) left: f64,
}

impl Default for Margin {
    fn default() -> Self {
        Self {
            top: 40.0,
            right: 20.0,
            bottom: 50.0,
            left: 60.0,
        }
    }
}

/// Decimal places that keep every tick label on one axis distinct.
///
/// A fixed one-decimal format collides as soon as the tick step falls below
/// 0.1 — a 0..0.4 axis draws "0.1" twice, which reads as a rendering fault —
/// and it wastes a decimal on integer axes ("10.0" for a component number).
/// The cheapest rule that fixes both is to ask for the fewest decimals at
/// which no two labels are equal, since that is the property a reader needs.
fn tick_decimals(ticks: &[f64]) -> usize {
    const MAX_DECIMALS: usize = 4;
    for decimals in 0..MAX_DECIMALS {
        let mut labels: Vec<String> = ticks.iter().map(|t| format!("{t:.decimals$}")).collect();
        labels.sort();
        labels.dedup();
        if labels.len() == ticks.len() {
            return decimals;
        }
    }
    MAX_DECIMALS
}

impl SvgCanvas {
    pub(crate) fn new(width: f64, height: f64) -> Self {
        Self {
            width,
            height,
            margin: Margin::default(),
            elements: Vec::new(),
            accessible_label: None,
        }
    }

    pub(crate) fn plot_width(&self) -> f64 {
        self.width - self.margin.left - self.margin.right
    }
    pub(crate) fn plot_height(&self) -> f64 {
        self.height - self.margin.top - self.margin.bottom
    }

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

    pub(crate) fn add_line(
        &mut self,
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
        stroke: &str,
        width: f64,
    ) {
        self.elements.push(format!(
            r#"<line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke}" stroke-width="{width}" />"#
        ));
    }

    /// Draw a cloud of points as one embedded raster instead of one element
    /// each.
    ///
    /// A scatter of n cells costs n DOM nodes and roughly 65 bytes of markup
    /// apiece. At the 2700 cells of PBMC3k that is nothing; at the hundreds of
    /// thousands a current atlas holds it is tens of megabytes of string and a
    /// browser that stops responding. Measured: one million points is 65.5 MB
    /// and 1,000,039 elements.
    ///
    /// Rasterising the points bounds both by the pixel area rather than the
    /// cell count, while the axes, ticks, labels and legend stay real SVG - so
    /// the text is still vector and still crisp at any zoom. This is what
    /// ggrastr and Seurat's `raster = TRUE` do, for the same reason.
    ///
    /// `points` are in the same user-space coordinates as every other element,
    /// so a caller can switch between this and add_circle without moving
    /// anything; the mapping into the pixmap happens here.
    pub(crate) fn add_point_raster(
        &mut self,
        points: &[(f64, f64, [u8; 4])],
        radius: f64,
        area: (f64, f64, f64, f64),
    ) {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let (x, y, width, height) = area;
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        // Supersampled, so a 3-point dot does not turn into a hard square and
        // the raster survives being viewed at 2x.
        const SCALE: f64 = 2.0;
        let pixel_width = (width * SCALE).ceil().max(1.0) as u32;
        let pixel_height = (height * SCALE).ceil().max(1.0) as u32;
        let Some(mut pixmap) = tiny_skia::Pixmap::new(pixel_width, pixel_height) else {
            return;
        };

        let mut paint = tiny_skia::Paint {
            anti_alias: true,
            ..Default::default()
        };
        for &(px, py, [r, g, b, a]) in points {
            // Into the pixmap's own coordinates: the raster covers the plot
            // area only, so subtract its origin.
            let cx = ((px - x) * SCALE) as f32;
            let cy = ((py - y) * SCALE) as f32;
            let Some(circle) = tiny_skia::PathBuilder::from_circle(cx, cy, (radius * SCALE) as f32)
            else {
                continue;
            };
            paint.set_color(tiny_skia::Color::from_rgba8(r, g, b, a));
            pixmap.fill_path(
                &circle,
                &paint,
                tiny_skia::FillRule::Winding,
                tiny_skia::Transform::identity(),
                None,
            );
        }

        let Ok(png) = pixmap.encode_png() else {
            return;
        };
        // `href` rather than `xlink:href`: SVG 2, understood by every current
        // browser and by resvg, which is what save_png rasterises with.
        self.elements.push(format!(
            r#"<image x="{x:.1}" y="{y:.1}" width="{width:.1}" height="{height:.1}" href="data:image/png;base64,{}" />"#,
            STANDARD.encode(png)
        ));
    }

    pub(crate) fn add_text(&mut self, x: f64, y: f64, text: &str, anchor: &str, size: f64) {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="sans-serif">{escaped}</text>"#
        ));
    }

    pub(crate) fn add_text_rotated(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        angle: f64,
        anchor: &str,
        size: f64,
    ) {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="sans-serif" transform="rotate({angle},{x:.1},{y:.1})">{escaped}</text>"#
        ));
    }

    pub(crate) fn draw_x_axis(&mut self, scale: &Scale, label: &str) {
        let y = self.margin.top + self.plot_height();
        self.add_line(
            self.margin.left,
            y,
            self.margin.left + self.plot_width(),
            y,
            "#333",
            1.0,
        );
        let x_scale = Scale {
            domain: scale.domain,
            range: (self.margin.left, self.margin.left + self.plot_width()),
        };
        let ticks = scale.nice_ticks(5);
        let decimals = tick_decimals(&ticks);
        for tick in ticks {
            let x = x_scale.map(tick);
            self.add_line(x, y, x, y + 5.0, "#333", 1.0);
            self.add_text(x, y + 18.0, &format!("{tick:.decimals$}"), "middle", 11.0);
        }
        self.add_text(
            self.margin.left + self.plot_width() / 2.0,
            self.height - 5.0,
            label,
            "middle",
            13.0,
        );
    }

    /// An x axis of labels rather than numbers, for a bar chart.
    ///
    /// A bar chart's x column is almost always categories, and a category has
    /// no numeric position: reading it through `extract_table_col` turns
    /// "Biology" into NaN and draws an axis running 0.0 to 1.0 underneath bars
    /// that have nothing to do with those numbers. This puts each category's
    /// own name under its group instead.
    ///
    /// Labels are thinned when there are more of them than the axis can fit,
    /// because overlapping text is less readable than fewer labels.
    pub(crate) fn draw_category_axis(&mut self, labels: &[String], axis_label: &str) {
        let y = self.margin.top + self.plot_height();
        self.add_line(
            self.margin.left,
            y,
            self.margin.left + self.plot_width(),
            y,
            "#333",
            1.0,
        );
        if !labels.is_empty() {
            let slot = self.plot_width() / labels.len() as f64;
            // Roughly 46px of room per label before they start to collide.
            let step = (46.0 / slot).ceil().max(1.0) as usize;
            for (index, label) in labels.iter().enumerate().step_by(step) {
                let x = self.margin.left + slot * (index as f64 + 0.5);
                self.add_line(x, y, x, y + 5.0, "#333", 1.0);
                self.add_text(x, y + 18.0, label, "middle", 11.0);
            }
        }
        if !axis_label.is_empty() {
            self.add_text(
                self.margin.left + self.plot_width() / 2.0,
                self.height - 5.0,
                axis_label,
                "middle",
                13.0,
            );
        }
    }

    pub(crate) fn draw_y_axis(&mut self, scale: &Scale, label: &str) {
        let x = self.margin.left;
        self.add_line(
            x,
            self.margin.top,
            x,
            self.margin.top + self.plot_height(),
            "#333",
            1.0,
        );
        let y_scale = Scale {
            domain: scale.domain,
            range: (self.margin.top + self.plot_height(), self.margin.top),
        };
        let ticks = scale.nice_ticks(5);
        let decimals = tick_decimals(&ticks);
        for tick in ticks {
            let y = y_scale.map(tick);
            self.add_line(x - 5.0, y, x, y, "#333", 1.0);
            self.add_text(x - 8.0, y + 4.0, &format!("{tick:.decimals$}"), "end", 11.0);
        }
        self.add_text_rotated(
            15.0,
            self.margin.top + self.plot_height() / 2.0,
            label,
            -90.0,
            "middle",
            13.0,
        );
    }

    pub(crate) fn draw_title(&mut self, title: &str) {
        self.accessible_label = Some(title.to_string());
        self.add_text(self.width / 2.0, 25.0, title, "middle", 16.0);
    }

    pub(crate) fn render(&self) -> String {
        let label = self
            .accessible_label
            .as_deref()
            .unwrap_or("BioLang plot")
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" aria-label="{}">"#,
            self.width, self.height, self.width, self.height, label
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

pub(crate) fn get_opt_str<'a>(
    opts: &'a HashMap<String, Value>,
    key: &str,
    default: &'a str,
) -> &'a str {
    opts.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

/// The label for an axis: the caller's `xlabel`/`ylabel` when they gave one,
/// otherwise whatever the plot worked out for itself.
///
/// The worked-out defaults ("Value", "Count", a column name) are fine for a
/// quick look at data and useless in a figure meant to teach or to publish,
/// where the whole job of an axis is to say what was measured and in what unit.
/// Every plot here that draws an axis honours these two options.
pub(crate) fn axis_label(opts: &HashMap<String, Value>, key: &str, default: &str) -> String {
    opts.get(key)
        .and_then(|value| value.as_str())
        .unwrap_or(default)
        .to_string()
}

pub(crate) fn get_opt_f64(opts: &HashMap<String, Value>, key: &str, default: f64) -> f64 {
    opts.get(key).and_then(|v| v.as_float()).unwrap_or(default)
}

pub(crate) fn parse_options(args: &[Value]) -> HashMap<String, Value> {
    if args.len() > 1 {
        if let Value::Record(map) = &args[1] {
            return (map).as_ref().clone();
        }
    }
    HashMap::new()
}

pub(crate) fn extract_table_col(table: &Table, col: &str) -> Result<Vec<f64>> {
    let idx = table.col_index(col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{col}' not found"),
            None,
        )
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

/// A column as the text a reader would see, for labelling a category axis.
fn column_labels(table: &Table, col: &str) -> Vec<String> {
    let Some(idx) = table.col_index(col) else {
        return Vec::new();
    };
    table
        .rows
        .iter()
        .map(|row| match &row[idx] {
            Value::Str(s) => s.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => format!("{f}"),
            other => format!("{other:?}"),
        })
        .collect()
}

pub(crate) fn col_range(vals: &[f64]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in vals {
        if v.is_finite() {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }
    if min > max {
        (0.0, 1.0)
    } else {
        (min, max)
    }
}

pub(crate) fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table, got {}", other.type_of()),
            None,
        )),
    }
}

// ── Builtins ────────────────────────────────────────────────────

/// The y columns a plot draws: one, or several when `y` is given a list.
///
/// Several series on one pair of axes is the case that every hand-drawn figure
/// in the statistics book needed and no builtin could express. Drawing them
/// separately and placing them side by side is not the same picture: each panel
/// gets its own scale, so the comparison the figure exists to make is the one
/// thing it cannot show.
fn series_columns(opts: &HashMap<String, Value>, fallback: &str) -> Vec<String> {
    let named: Vec<String> = match opts.get("y") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        Some(value) => value.as_str().map(str::to_string).into_iter().collect(),
        None => Vec::new(),
    };
    if named.is_empty() {
        vec![fallback.to_string()]
    } else {
        named
    }
}

/// A key naming each series, drawn inside the top right of the plot area.
///
/// Only when there is more than one: a legend for a single series is a caption
/// repeating the axis label.
fn draw_legend(canvas: &mut SvgCanvas, names: &[String]) {
    if names.len() < 2 {
        return;
    }
    let right = canvas.margin.left + canvas.plot_width();
    for (index, name) in names.iter().enumerate() {
        let y = canvas.margin.top + 14.0 + 18.0 * index as f64;
        let swatch_end = right - 8.0;
        let swatch_start = swatch_end - 22.0;
        canvas.add_line(
            swatch_start,
            y,
            swatch_end,
            y,
            PALETTE[index % PALETTE.len()],
            3.0,
        );
        canvas.add_text(swatch_start - 6.0, y + 4.0, name, "end", 12.0);
    }
}

/// Type 7 quantiles — R's default, and what this runtime's `quantile()` gives.
///
/// The box plot used to take `sorted[n / 4]` and `sorted[3 * n / 4]`, which is
/// the nearest-rank rule. On the book's ozone column that puts the top of the
/// box at 64 while `quantile(ozone, 0.75)` reports 63.25, so the picture and
/// the numbers printed beside it disagreed about the same data; on the ten
/// values 1 to 10 the two rules give 3 and 8 against 3.25 and 7.75. Expects
/// `sorted` already sorted and non-empty.
fn quantile_type7(sorted: &[f64], p: f64) -> f64 {
    let h = (sorted.len() - 1) as f64 * p;
    let lower = h.floor() as usize;
    let upper = (lower + 1).min(sorted.len() - 1);
    sorted[lower] + (h - h.floor()) * (sorted[upper] - sorted[lower])
}

fn builtin_plot(args: Vec<Value>) -> Result<Value> {
    // Handle Record with x/y lists: plot({x: [...], y: [...], title: "..."})
    let args = if args.len() == 1 {
        if let Value::Record(ref map) = args[0] {
            if map.contains_key("x") && map.contains_key("y") {
                if let (Value::List(xv), Value::List(yv)) = (&map["x"], &map["y"]) {
                    let rows: Vec<Vec<Value>> = xv
                        .iter()
                        .zip(yv.iter())
                        .map(|(x, y)| vec![x.clone(), y.clone()])
                        .collect();
                    let table = Value::Table(Table::new(vec!["x".into(), "y".into()], rows));
                    let mut opts = map.as_ref().clone();
                    opts.remove("x");
                    opts.remove("y");
                    if opts.is_empty() {
                        vec![table]
                    } else {
                        vec![table, Value::Record(opts.into())]
                    }
                } else {
                    args
                }
            } else {
                args
            }
        } else {
            args
        }
    } else {
        args
    };

    let opts = parse_options(&args);
    let plot_type = get_opt_str(&opts, "type", "scatter").to_string();
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "").to_string();

    let table = require_table(&args[0], "plot")?;

    if table.num_cols() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot() requires table with at least 2 columns",
            None,
        ));
    }

    let x_col = get_opt_str(&opts, "x", &table.columns[0]).to_string();
    let y_cols = series_columns(&opts, &table.columns[1]);

    let xs = extract_table_col(table, &x_col)?;
    let mut series: Vec<Vec<f64>> = Vec::with_capacity(y_cols.len());
    for column in &y_cols {
        series.push(extract_table_col(table, column)?);
    }

    let (x_min, x_max) = col_range(&xs);
    // One vertical scale across every series, so the comparison is honest.
    let (mut y_min, mut y_max) = series.iter().fold((f64::MAX, f64::MIN), |(lo, hi), ys| {
        let (series_lo, series_hi) = col_range(ys);
        (lo.min(series_lo), hi.max(series_hi))
    });
    if plot_type == "bar" {
        // A bar says "this much", and the reader takes its length as the
        // quantity. Starting the axis at the smallest value instead of zero
        // draws counts of 100 and 104 as a bar of nothing beside a full-height
        // one -- the best-known way to mislead with a chart, and it was the
        // default here. A bar chart's axis includes zero.
        y_min = y_min.min(0.0);
        y_max = y_max.max(0.0);
    }

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale {
        domain: (x_min, x_max),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (y_min, y_max),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    match plot_type.as_str() {
        "scatter" => {
            for (index, ys) in series.iter().enumerate() {
                let colour = PALETTE[index % PALETTE.len()];
                for i in 0..xs.len().min(ys.len()) {
                    if xs[i].is_finite() && ys[i].is_finite() {
                        canvas.add_circle(x_scale.map(xs[i]), y_scale.map(ys[i]), 4.0, colour);
                    }
                }
            }
            draw_legend(&mut canvas, &y_cols);
        }
        "line" => {
            for (index, ys) in series.iter().enumerate() {
                let points: Vec<String> = xs
                    .iter()
                    .zip(ys.iter())
                    .filter(|(x, y)| x.is_finite() && y.is_finite())
                    .map(|(x, y)| format!("{:.1},{:.1}", x_scale.map(*x), y_scale.map(*y)))
                    .collect();
                if !points.is_empty() {
                    canvas.elements.push(format!(
                        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
                        points.join(" "),
                        PALETTE[index % PALETTE.len()]
                    ));
                }
            }
            draw_legend(&mut canvas, &y_cols);
        }
        "bar" => {
            // Grouped: one cluster per x position, one bar per series inside it.
            let group_w = canvas.plot_width() / xs.len() as f64;
            let bar_w = group_w * 0.8 / series.len() as f64;
            let baseline = y_scale.map(0.0f64.max(y_min));
            for (index, ys) in series.iter().enumerate() {
                // A single series keeps its old per-bar colouring, which is what
                // a bar chart of categories wants; several series colour by
                // series instead, because that is what the legend names.
                for (i, &y) in ys.iter().enumerate() {
                    if !y.is_finite() {
                        continue;
                    }
                    let bx = canvas.margin.left
                        + group_w * i as f64
                        + group_w * 0.1
                        + bar_w * index as f64;
                    let by = y_scale.map(y);
                    let bh = (baseline - by).abs();
                    let top = by.min(baseline);
                    let colour = if series.len() == 1 {
                        PALETTE[i % PALETTE.len()]
                    } else {
                        PALETTE[index % PALETTE.len()]
                    };
                    canvas.add_rect(bx, top, bar_w, bh, colour);
                }
            }
            draw_legend(&mut canvas, &y_cols);
        }
        "box" => {
            // Box plot per numeric column
            for (ci, col) in table.columns.iter().enumerate() {
                let vals = extract_table_col(table, col)?;
                let mut sorted: Vec<f64> = vals.into_iter().filter(|v| v.is_finite()).collect();
                if sorted.is_empty() {
                    continue;
                }
                sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let q1 = quantile_type7(&sorted, 0.25);
                let med = quantile_type7(&sorted, 0.5);
                let q3 = quantile_type7(&sorted, 0.75);
                // Tukey whiskers: out to the furthest point within 1.5 IQR of
                // the box, with anything beyond drawn as its own mark. Reaching
                // to the extremes instead makes every dataset look as though it
                // has none, which is the one thing a box plot is for.
                let fence = 1.5 * (q3 - q1);
                let lo = sorted
                    .iter()
                    .copied()
                    .find(|v| *v >= q1 - fence)
                    .unwrap_or(sorted[0]);
                let hi = sorted
                    .iter()
                    .copied()
                    .rev()
                    .find(|v| *v <= q3 + fence)
                    .unwrap_or(sorted[sorted.len() - 1]);

                let bx = canvas.margin.left
                    + (ci as f64 + 0.2) * canvas.plot_width() / table.num_cols() as f64;
                let bw = canvas.plot_width() / table.num_cols() as f64 * 0.6;

                canvas.add_rect(
                    bx,
                    y_scale.map(q3),
                    bw,
                    (y_scale.map(q1) - y_scale.map(q3)).abs(),
                    PALETTE[ci % PALETTE.len()],
                );
                canvas.add_line(bx, y_scale.map(med), bx + bw, y_scale.map(med), "#333", 2.0);
                canvas.add_line(
                    bx + bw / 2.0,
                    y_scale.map(q3),
                    bx + bw / 2.0,
                    y_scale.map(hi),
                    "#333",
                    1.0,
                );
                canvas.add_line(
                    bx + bw / 2.0,
                    y_scale.map(q1),
                    bx + bw / 2.0,
                    y_scale.map(lo),
                    "#333",
                    1.0,
                );
                for value in sorted.iter().filter(|v| **v < lo || **v > hi) {
                    canvas.add_circle(bx + bw / 2.0, y_scale.map(*value), 3.0, "#333");
                }
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

    let d_x_scale = Scale {
        domain: (x_min, x_max),
        range: (x_min, x_max),
    };
    let d_y_scale = Scale {
        domain: (y_min, y_max),
        range: (y_min, y_max),
    };
    // With several series the legend names them, so the default y label would
    // be one column's name standing for all of them. Better to say nothing.
    let default_ylabel = if y_cols.len() == 1 { &y_cols[0] } else { "" };
    if plot_type == "bar" {
        canvas.draw_category_axis(
            &column_labels(table, &x_col),
            &axis_label(&opts, "xlabel", &x_col),
        );
    } else {
        canvas.draw_x_axis(&d_x_scale, &axis_label(&opts, "xlabel", &x_col));
    }
    canvas.draw_y_axis(&d_y_scale, &axis_label(&opts, "ylabel", default_ylabel));
    if !title.is_empty() {
        canvas.draw_title(&title);
    }

    Ok(Value::Str(canvas.render()))
}

// ── Heatmap color schemes ──────────────────────────────────────

fn interpolate_viridis(t: f64) -> String {
    // Viridis: dark purple → teal → yellow (5-stop approximation)
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (68.0, 1.0, 84.0),    // 0.00 — dark purple
        (59.0, 82.0, 139.0),  // 0.25 — blue-purple
        (33.0, 145.0, 140.0), // 0.50 — teal
        (94.0, 201.0, 98.0),  // 0.75 — green
        (253.0, 231.0, 37.0), // 1.00 — yellow
    ];
    heatmap_interp_stops(t, &stops)
}

fn interpolate_plasma(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (13.0, 8.0, 135.0),   // deep blue
        (126.0, 3.0, 168.0),  // purple
        (204.0, 71.0, 120.0), // pink
        (248.0, 149.0, 64.0), // orange
        (240.0, 249.0, 33.0), // yellow
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
            if !(0.25..=0.75).contains(&t) {
                "white"
            } else {
                "#333"
            }
        }
        "blues" | "greens" | "reds" => {
            if t > 0.6 {
                "white"
            } else {
                "#333"
            }
        }
        // viridis, plasma, inferno: dark at low end, bright at high end
        _ => {
            if t < 0.55 {
                "white"
            } else {
                "#333"
            }
        }
    }
}

/// Simple row clustering by sorting rows by their mean value.
fn cluster_rows(row_data: &mut Vec<Vec<f64>>, row_labels: &mut Vec<String>) {
    let mut indices: Vec<usize> = (0..row_data.len()).collect();
    indices.sort_by(|&a, &b| {
        let mean_a: f64 = row_data[a]
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .sum::<f64>()
            / row_data[a].len().max(1) as f64;
        let mean_b: f64 = row_data[b]
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .sum::<f64>()
            / row_data[b].len().max(1) as f64;
        mean_a
            .partial_cmp(&mean_b)
            .unwrap_or(std::cmp::Ordering::Equal)
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
    let show_values = opts
        .get("show_values")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let do_cluster = opts
        .get("cluster")
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
            let cl = m
                .col_names
                .clone()
                .unwrap_or_else(|| (0..m.ncol).map(|i| format!("col{i}")).collect());
            let mut rd = Vec::with_capacity(m.nrow);
            let rl: Vec<String> = m
                .row_names
                .clone()
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
                    ErrorKind::TypeError,
                    "heatmap() received empty list",
                    None,
                ));
            }
            match &items[0] {
                Value::List(_) => {
                    // List of Lists
                    let mut rd = Vec::with_capacity(items.len());
                    let mut max_cols = 0usize;
                    for item in items.iter() {
                        if let Value::List(row) = item {
                            let rv: Vec<f64> = row
                                .iter()
                                .map(|v| match v {
                                    Value::Int(n) => *n as f64,
                                    Value::Float(f) => *f,
                                    _ => f64::NAN,
                                })
                                .collect();
                            if rv.len() > max_cols {
                                max_cols = rv.len();
                            }
                            rd.push(rv);
                        } else {
                            return Err(BioLangError::type_error(
                                "heatmap() list items must all be Lists or Records",
                                None,
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
                    for item in items.iter() {
                        if let Value::Record(map) = item {
                            for k in map.keys() {
                                if key_set.insert(k.clone()) {
                                    all_keys.push(k.clone());
                                }
                            }
                        }
                    }
                    let mut rd = Vec::with_capacity(items.len());
                    for item in items.iter() {
                        if let Value::Record(map) = item {
                            let rv: Vec<f64> = all_keys
                                .iter()
                                .map(|k| {
                                    map.get(k)
                                        .map(|v| match v {
                                            Value::Int(n) => *n as f64,
                                            Value::Float(f) => *f,
                                            _ => f64::NAN,
                                        })
                                        .unwrap_or(f64::NAN)
                                })
                                .collect();
                            rd.push(rv);
                        }
                    }
                    let rl: Vec<String> = (0..rd.len()).map(|i| format!("{}", i + 1)).collect();
                    (all_keys, rd, rl)
                }
                _ => {
                    return Err(BioLangError::type_error(
                        "heatmap() requires Table, Matrix, List of Lists, or List of Records",
                        None,
                    ))
                }
            }
        }
        _ => {
            return Err(BioLangError::type_error(
                "heatmap() requires Table, Matrix, List of Lists, or List of Records",
                None,
            ))
        }
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
    let ncols = if nrows > 0 {
        row_data[0].len()
    } else {
        col_labels.len()
    };

    if nrows == 0 || ncols == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "heatmap() received empty data",
            None,
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
            if v.is_finite() {
                all_vals.push(v);
            }
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
            let t = if (vmax - vmin).abs() < f64::EPSILON {
                0.5
            } else {
                (v - vmin) / (vmax - vmin)
            };
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
            legend_x,
            y,
            legend_bar_w,
            step_h + 0.5,
            color
        ));
    }
    // Legend border
    canvas.elements.push(format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#333\" stroke-width=\"0.5\" />",
        legend_x, legend_top, legend_bar_w, legend_h
    ));
    // Legend tick labels
    let label_x = legend_x + legend_bar_w + 5.0;
    canvas.add_text(
        label_x,
        legend_top + 4.0,
        &format!("{vmax:.2}"),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h / 2.0 + 3.0,
        &format!("{:.2}", (vmin + vmax) / 2.0),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h + 3.0,
        &format!("{vmin:.2}"),
        "start",
        9.0,
    );

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
            for item in items.iter() {
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
        _ => {
            return Err(BioLangError::type_error(
                "histogram() requires List of numbers",
                None,
            ))
        }
    };

    if nums.is_empty() {
        return Err(BioLangError::runtime(ErrorKind::TypeError, "histogram() received no numeric values — check that your data contains numbers, not strings", None));
    }

    let (lo, hi) = col_range(&nums);
    let bin_w = if (hi - lo).abs() < f64::EPSILON {
        1.0
    } else {
        (hi - lo) / bins as f64
    };
    let mut counts = vec![0usize; bins];
    for &v in &nums {
        let mut idx = ((v - lo) / bin_w) as usize;
        if idx >= bins {
            idx = bins - 1;
        }
        counts[idx] += 1;
    }
    let max_count = *counts.iter().max().unwrap_or(&1);

    let mut canvas = SvgCanvas::new(width, height);
    let y_scale = Scale {
        domain: (0.0, max_count as f64),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    let rect_w = canvas.plot_width() / bins as f64;
    for (i, &count) in counts.iter().enumerate() {
        let x = canvas.margin.left + i as f64 * rect_w;
        let y = y_scale.map(count as f64);
        let h = canvas.margin.top + canvas.plot_height() - y;
        canvas.add_rect(x, y, rect_w - 1.0, h, PALETTE[0]);
    }

    let d_x_scale = Scale {
        domain: (lo, hi),
        range: (lo, hi),
    };
    let d_y_scale = Scale {
        domain: (0.0, max_count as f64),
        range: (0.0, max_count as f64),
    };
    canvas.draw_x_axis(&d_x_scale, &axis_label(&opts, "xlabel", "Value"));
    canvas.draw_y_axis(&d_y_scale, &axis_label(&opts, "ylabel", "Count"));
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}

/// The numbers in a list argument, for the plots that take one list.
///
/// Numeric strings are accepted because a column read from a CSV arrives as
/// text often enough that rejecting it would be the wrong default.
fn numeric_list(value: &Value, who: &str) -> Result<Vec<f64>> {
    let items = match value {
        Value::List(items) => items,
        _ => {
            return Err(BioLangError::type_error(
                format!("{who}() requires List of numbers"),
                None,
            ))
        }
    };
    let mut numbers = Vec::with_capacity(items.len());
    for item in items.iter() {
        match item {
            Value::Int(n) => numbers.push(*n as f64),
            Value::Float(f) => numbers.push(*f),
            Value::Str(s) => {
                if let Ok(f) = s.parse::<f64>() {
                    numbers.push(f);
                }
            }
            _ => {}
        }
    }
    if numbers.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{who}() received no numeric values - check that your data contains numbers, not strings"
            ),
            None,
        ));
    }
    Ok(numbers)
}

/// The empirical cumulative distribution: for each value, the fraction of the
/// data at or below it.
///
/// Drawn as the step function it actually is rather than joined with straight
/// lines, because the distribution really is flat between observations. Unlike
/// a histogram it has no bin width, so it shows the data without a parameter
/// that changes the story being told.
fn builtin_ecdf_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Empirical CDF").to_string();

    let mut values = numeric_list(&args[0], "ecdf_plot")?;
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len() as f64;

    let (lo, hi) = col_range(&values);
    let span = if (hi - lo).abs() < f64::EPSILON {
        1.0
    } else {
        hi - lo
    };

    let mut canvas = SvgCanvas::new(width, height);
    let right_edge = canvas.margin.left + canvas.plot_width();
    let x_scale = Scale {
        domain: (lo, lo + span),
        range: (canvas.margin.left, right_edge),
    };
    let y_scale = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // One polyline rather than two line elements per observation: the same
    // picture, and a tenth of the file for a column of a few hundred values,
    // which matters when the SVG is inlined into a page.
    let mut points = Vec::with_capacity(2 * values.len() + 2);
    points.push(format!(
        "{:.1},{:.1}",
        x_scale.map(values[0]),
        y_scale.map(0.0)
    ));
    for (index, value) in values.iter().enumerate() {
        let x = x_scale.map(*value);
        let y = y_scale.map((index + 1) as f64 / n);
        // The riser at the observation, then the flat run to the next one.
        points.push(format!("{x:.1},{y:.1}"));
        let next_x = match values.get(index + 1) {
            Some(next) => x_scale.map(*next),
            None => right_edge,
        };
        points.push(format!("{next_x:.1},{y:.1}"));
    }
    canvas.elements.push(format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="1.5" />"#,
        points.join(" "),
        PALETTE[0]
    ));

    canvas.draw_x_axis(
        &Scale {
            domain: (lo, lo + span),
            range: (lo, lo + span),
        },
        &axis_label(&opts, "xlabel", "Value"),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        },
        &axis_label(&opts, "ylabel", "Proportion at or below"),
    );
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}

/// Silverman's rule of thumb for a kernel bandwidth: the width R's `bw.nrd0`
/// picks, computed the same way so the two agree.
///
/// `0.9 * min(sd, IQR/1.34) * n^(-1/5)`. The `min` is what keeps a long tail
/// from inflating the standard deviation and oversmoothing everything else,
/// and the IQR falls back to the sd when the middle half of the data is a
/// single repeated value. Expects `values` already sorted.
fn silverman_bandwidth(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let sd = if values.len() > 1 {
        (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    } else {
        1.0
    };
    // Type 7, matching quantile() elsewhere in the runtime and R's default.
    let quantile = |p: f64| -> f64 {
        let h = (n - 1.0) * p;
        let lower = h.floor() as usize;
        let upper = (lower + 1).min(values.len() - 1);
        values[lower] + (h - h.floor()) * (values[upper] - values[lower])
    };
    let iqr = quantile(0.75) - quantile(0.25);
    // R's fallback chain, in R's order: the IQR estimate, then the standard
    // deviation, then the magnitude of a single observation, then 1. Each step
    // exists because the one before it can be exactly zero -- on a column of
    // repeated values every measure of spread is -- and a bandwidth of zero
    // divides by zero and draws nothing.
    let mut spread = sd.min(iqr / 1.34);
    if spread <= 0.0 {
        spread = sd;
    }
    if spread <= 0.0 {
        spread = values[0].abs();
    }
    if spread <= 0.0 {
        spread = 1.0;
    }
    0.9 * spread * n.powf(-0.2)
}

/// A Gaussian kernel density estimate: a smooth stand-in for a histogram that
/// does not depend on where the bin edges happen to fall.
///
/// The default bandwidth is Silverman's rule of thumb,
/// `0.9 * min(sd, IQR/1.34) * n^(-1/5)`, which is what R's `bw.nrd0` computes,
/// so the two agree by construction. Pass `bandwidth` to override it - and do
/// look at more than one, because bandwidth is to a density what bin width is
/// to a histogram: a choice that changes the shape being argued for.
fn builtin_density_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Density").to_string();

    let mut values = numeric_list(&args[0], "density_plot")?;
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = values.len() as f64;

    let bandwidth =
        get_opt_f64(&opts, "bandwidth", silverman_bandwidth(&values)).max(f64::MIN_POSITIVE);

    // Reach three bandwidths past the data so the tails are not cut off.
    let (data_lo, data_hi) = col_range(&values);
    let lo = data_lo - 3.0 * bandwidth;
    let hi = data_hi + 3.0 * bandwidth;

    let steps = 256usize;
    let normaliser = 1.0 / (n * bandwidth * (2.0 * std::f64::consts::PI).sqrt());
    let mut densities = Vec::with_capacity(steps);
    for step in 0..steps {
        let x = lo + (hi - lo) * step as f64 / (steps - 1) as f64;
        let density = values
            .iter()
            .map(|v| {
                let z = (x - v) / bandwidth;
                (-0.5 * z * z).exp()
            })
            .sum::<f64>()
            * normaliser;
        densities.push((x, density));
    }
    let peak = densities
        .iter()
        .map(|(_, d)| *d)
        .fold(0.0f64, f64::max)
        .max(f64::MIN_POSITIVE);

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale {
        domain: (lo, hi),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, peak),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    let points: Vec<String> = densities
        .iter()
        .map(|(x, d)| format!("{:.1},{:.1}", x_scale.map(*x), y_scale.map(*d)))
        .collect();
    canvas.elements.push(format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
        points.join(" "),
        PALETTE[0]
    ));

    canvas.draw_x_axis(
        &Scale {
            domain: (lo, hi),
            range: (lo, hi),
        },
        &axis_label(&opts, "xlabel", "Value"),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, peak),
            range: (0.0, peak),
        },
        &axis_label(&opts, "ylabel", "Density"),
    );
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

    let neg_log_p: Vec<f64> = pvals
        .iter()
        .map(|&p| if p > 0.0 { -(p.log10()) } else { 0.0 })
        .collect();

    let (x_min, x_max) = col_range(&fcs);
    let x_abs = x_min.abs().max(x_max.abs());
    let (_, y_max) = col_range(&neg_log_p);

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale {
        domain: (-x_abs, x_abs),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, y_max),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    let neg_log_p_thresh = -(p_thresh.log10());

    // Threshold lines
    canvas.add_line(
        x_scale.map(-fc_thresh),
        canvas.margin.top,
        x_scale.map(-fc_thresh),
        canvas.margin.top + canvas.plot_height(),
        "#ccc",
        1.0,
    );
    canvas.add_line(
        x_scale.map(fc_thresh),
        canvas.margin.top,
        x_scale.map(fc_thresh),
        canvas.margin.top + canvas.plot_height(),
        "#ccc",
        1.0,
    );
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(neg_log_p_thresh),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(neg_log_p_thresh),
        "#ccc",
        1.0,
    );

    for i in 0..fcs.len() {
        let color = if neg_log_p[i] > neg_log_p_thresh && fcs[i].abs() > fc_thresh {
            if fcs[i] > 0.0 {
                "#e15759"
            } else {
                "#4e79a7"
            }
        } else {
            "#999"
        };
        canvas.add_circle(x_scale.map(fcs[i]), y_scale.map(neg_log_p[i]), 3.0, color);
    }

    let d_x_scale = Scale {
        domain: (-x_abs, x_abs),
        range: (-x_abs, x_abs),
    };
    let d_y_scale = Scale {
        domain: (0.0, y_max),
        range: (0.0, y_max),
    };
    canvas.draw_x_axis(
        &d_x_scale,
        &axis_label(&opts, "xlabel", &format!("log2(FC) [{fc_col}]")),
    );
    canvas.draw_y_axis(
        &d_y_scale,
        &axis_label(&opts, "ylabel", &format!("-log10(p) [{p_col}]")),
    );
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
    let a_log: Vec<f64> = a_vals
        .iter()
        .map(|&v| if v > 0.0 { v.log2() } else { 0.0 })
        .collect();

    let (x_min, x_max) = col_range(&a_log);
    let (y_min, y_max) = col_range(&m_vals);
    let y_abs = y_min.abs().max(y_max.abs());

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale {
        domain: (x_min, x_max),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (-y_abs, y_abs),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // Zero line
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(0.0),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(0.0),
        "#ccc",
        1.0,
    );

    for i in 0..a_log.len() {
        let color = if m_vals[i].abs() > 1.0 {
            "#e15759"
        } else {
            "#999"
        };
        canvas.add_circle(x_scale.map(a_log[i]), y_scale.map(m_vals[i]), 3.0, color);
    }

    let d_x_scale = Scale {
        domain: (x_min, x_max),
        range: (x_min, x_max),
    };
    let d_y_scale = Scale {
        domain: (-y_abs, y_abs),
        range: (-y_abs, y_abs),
    };
    canvas.draw_x_axis(
        &d_x_scale,
        &axis_label(&opts, "xlabel", &format!("A (log2 {a_col})")),
    );
    canvas.draw_y_axis(
        &d_y_scale,
        &axis_label(&opts, "ylabel", &format!("M ({m_col})")),
    );
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
        other => {
            return Err(BioLangError::type_error(
                format!("save_svg() requires Str (path), got {}", other.type_of()),
                None,
            ))
        }
    };
    std::fs::write(path, svg).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("save_svg() write failed: {e}"),
            None,
        )
    })?;
    Ok(Value::Str(path.clone()))
}

/// Write a plot as a PNG: `save_png(svg, "figure.png", { scale: 2 })`.
///
/// Rasterises the SVG the plot builtins already return rather than rendering
/// twice. Everything here is downstream of the string `save_svg` would have
/// written, so a PNG cannot disagree with its SVG - and every existing plot got
/// PNG support the moment this landed, including the ones added after it.
///
/// `scale` multiplies the pixel dimensions without changing the drawing: the
/// figure is the same size in inches and simply carries more pixels, which is
/// what a journal asking for 300 dpi wants. Default 2, because a 1x raster of a
/// 600-point figure looks soft on any modern display.
fn builtin_save_png(args: Vec<Value>) -> Result<Value> {
    let svg = match &args[0] {
        Value::Str(s) => s,
        Value::Nil => {
            return Err(BioLangError::type_error(
                "save_png() received Nil — the plot function before the pipe likely failed or returned nothing".to_string(),
                None,
            ))
        }
        other => {
            return Err(BioLangError::type_error(
                format!("save_png() requires Str (SVG), got {}", other.type_of()),
                None,
            ))
        }
    };
    let path = match &args[1] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("save_png() requires Str (path), got {}", other.type_of()),
                None,
            ))
        }
    };
    // Options are the third argument here, not the second - args[1] is the path.
    // parse_options() always reads args[1], so calling it would have silently
    // ignored every option and left `scale` at its default.
    let opts = parse_options(&args[1..]);
    let scale = get_opt_f64(&opts, "scale", 2.0);
    if !(scale.is_finite() && scale > 0.0) {
        return Err(BioLangError::type_error(
            format!("save_png() scale must be a positive number, got {scale}"),
            None,
        ));
    }

    render_png(svg, &path, scale)?;
    Ok(Value::Str(path))
}

#[cfg(feature = "native")]
fn configure_generic_font_families(db: &mut resvg::usvg::fontdb::Database) {
    // fontdb's generic defaults are Windows font names (Arial, Times New Roman,
    // and Courier New). A Linux machine can therefore have plenty of fonts but
    // still render every `font-family="sans-serif"` label as nothing. Point the
    // CSS generic families at fonts that are actually present on this machine.
    let families: Vec<String> = db
        .faces()
        .flat_map(|face| face.families.iter().map(|(name, _)| name.clone()))
        .collect();

    let choose = |preferred: &[&str]| {
        preferred
            .iter()
            .find_map(|candidate| {
                families
                    .iter()
                    .find(|name| name.eq_ignore_ascii_case(candidate))
                    .cloned()
            })
            .or_else(|| families.first().cloned())
    };

    if let Some(family) = choose(&[
        "Arial",
        "DejaVu Sans",
        "Liberation Sans",
        "Noto Sans",
        "Ubuntu",
        "Segoe UI",
        "Helvetica",
    ]) {
        db.set_sans_serif_family(family);
    }
    if let Some(family) = choose(&[
        "Times New Roman",
        "DejaVu Serif",
        "Liberation Serif",
        "Noto Serif",
        "Georgia",
    ]) {
        db.set_serif_family(family);
    }
    if let Some(family) = choose(&[
        "Courier New",
        "DejaVu Sans Mono",
        "Liberation Mono",
        "Noto Sans Mono",
        "Consolas",
    ]) {
        db.set_monospace_family(family);
    }
}

#[cfg(feature = "native")]
fn render_png(svg: &str, path: &str, scale: f64) -> Result<()> {
    use resvg::{tiny_skia, usvg};

    let png_error = |message: String| BioLangError::runtime(ErrorKind::IOError, message, None);

    // Loading system fonts costs tens of milliseconds, and a script that writes
    // a figure per chapter would pay it once per call. One database, built on
    // first use, shared by every later call.
    static FONTS: std::sync::OnceLock<std::sync::Arc<usvg::fontdb::Database>> =
        std::sync::OnceLock::new();
    let fontdb = FONTS.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        configure_generic_font_families(&mut db);
        std::sync::Arc::new(db)
    });

    let options = usvg::Options {
        fontdb: fontdb.clone(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|e| png_error(format!("save_png() could not parse the SVG: {e}")))?;

    let size = tree.size();
    let width = (size.width() as f64 * scale).round().max(1.0);
    let height = (size.height() as f64 * scale).round().max(1.0);
    // A scale of 1e6 on a 600-point figure asks for a 360-gigapixel buffer.
    // Pixmap::new returns None rather than aborting, so say why.
    let mut pixmap = tiny_skia::Pixmap::new(width as u32, height as u32).ok_or_else(|| {
        png_error(format!(
            "save_png() cannot allocate a {width:.0}x{height:.0} image — lower the scale"
        ))
    })?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale as f32, scale as f32),
        &mut pixmap.as_mut(),
    );
    pixmap
        .save_png(path)
        .map_err(|e| png_error(format!("save_png() write failed: {e}")))?;
    Ok(())
}

#[cfg(not(feature = "native"))]
fn render_png(_svg: &str, _path: &str, _scale: f64) -> Result<()> {
    // The WASM build has no rasteriser, but the browser does. Failing with the
    // alternative named beats failing with "unknown builtin".
    Err(BioLangError::runtime(
        ErrorKind::IOError,
        "save_png() is not available in this build — use save_svg(), which every browser and \
         image tool can rasterise"
            .to_string(),
        None,
    ))
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

    let x_scale = Scale {
        domain: (global_start, global_end),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };

    // Draw backbone
    let track_y = canvas.margin.top + canvas.plot_height() / 2.0;
    canvas.add_line(
        canvas.margin.left,
        track_y,
        canvas.margin.left + canvas.plot_width(),
        track_y,
        "#ccc",
        2.0,
    );

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
                    arrow_x,
                    arrow_y,
                    arrow_x + dx,
                    arrow_y - 4.0,
                    arrow_x + dx,
                    arrow_y + 4.0
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

    let d_x_scale = Scale {
        domain: (global_start, global_end),
        range: (global_start, global_end),
    };
    canvas.draw_x_axis(&d_x_scale, &axis_label(&opts, "xlabel", "Position"));
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}

#[cfg(test)]
mod palette_tests {
    use super::{SvgCanvas, PALETTE};
    use std::collections::HashSet;

    // Callers index PALETTE modulo its length, so a plot with more groups than
    // colours draws two groups the same. At eight entries that bit constantly:
    // clustering 2700 PBMCs gave eleven groups and three of them reused colours
    // already spent, on the one figure the analysis is read from.

    #[test]
    fn palette_covers_a_realistic_cluster_count() {
        assert!(
            PALETTE.len() >= 20,
            "single-cell work routinely yields 15-30 clusters; palette has {}",
            PALETTE.len()
        );
    }

    #[test]
    fn palette_entries_are_distinct() {
        let unique: HashSet<&&str> = PALETTE.iter().collect();
        assert_eq!(
            unique.len(),
            PALETTE.len(),
            "palette repeats a colour, so two groups share one"
        );
    }

    #[test]
    fn palette_entries_are_well_formed_hex() {
        for colour in PALETTE {
            assert!(
                colour.len() == 7 && colour.starts_with('#'),
                "malformed colour: {colour}"
            );
            assert!(
                colour[1..].chars().all(|c| c.is_ascii_hexdigit()),
                "non-hex colour: {colour}"
            );
        }
    }

    #[test]
    fn the_original_eight_are_unchanged() {
        // Existing figures keep the colours they had.
        let original = [
            "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
        ];
        assert_eq!(&PALETTE[..8], &original[..]);
    }

    #[test]
    fn rendered_svg_uses_the_title_as_its_accessible_label() {
        let mut canvas = SvgCanvas::new(320.0, 180.0);
        canvas.draw_title("A & B");

        let svg = canvas.render();
        assert!(svg.contains("role=\"img\""));
        assert!(svg.contains("aria-label=\"A &amp; B\""));
    }

    #[test]
    fn rendered_svg_has_a_default_accessible_label() {
        let canvas = SvgCanvas::new(320.0, 180.0);
        assert!(canvas.render().contains("aria-label=\"BioLang plot\""));
    }
}

#[cfg(test)]
mod axis_label_tests {
    use super::{axis_label, call_plot_builtin};
    use bl_core::value::Value;
    use std::collections::HashMap;
    use std::sync::Arc;

    // The defaults ("Value", "Count") are what a histogram draws when nobody
    // says otherwise. They were previously the only thing it could draw: an
    // xlabel in the options record was accepted and silently ignored, so a
    // figure asking for "minutes until the next eruption" got "Value" and no
    // warning. Anything building teaching or publication figures had to
    // string-replace the rendered SVG afterwards.

    fn options(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), Value::Str((*v).into())))
            .collect()
    }

    #[test]
    fn an_explicit_label_replaces_the_default() {
        let opts = options(&[("xlabel", "minutes until the next eruption")]);
        assert_eq!(
            axis_label(&opts, "xlabel", "Value"),
            "minutes until the next eruption"
        );
    }

    #[test]
    fn the_default_is_used_when_no_label_is_given() {
        assert_eq!(axis_label(&HashMap::new(), "xlabel", "Value"), "Value");
    }

    #[test]
    fn a_histogram_renders_the_labels_it_was_given() {
        let values = Value::List(Arc::new(
            (1..=40).map(|n| Value::Float(f64::from(n))).collect(),
        ));
        let mut opts = HashMap::new();
        opts.insert("xlabel".to_string(), Value::Str("waiting (minutes)".into()));
        opts.insert("ylabel".to_string(), Value::Str("eruptions".into()));

        let svg = match call_plot_builtin("histogram", vec![values, Value::Record(Arc::new(opts))])
            .expect("histogram renders")
        {
            Value::Str(svg) => svg.to_string(),
            other => panic!("histogram should return SVG, got {other:?}"),
        };

        assert!(
            svg.contains(">waiting (minutes)<"),
            "x label missing: {svg:.400}"
        );
        assert!(svg.contains(">eruptions<"), "y label missing");
        assert!(
            !svg.contains(">Value<"),
            "the default x label should be gone"
        );
        assert!(
            !svg.contains(">Count<"),
            "the default y label should be gone"
        );
    }
}

#[cfg(test)]
mod distribution_plot_tests {
    use super::{call_plot_builtin, silverman_bandwidth};
    use bl_core::value::Value;
    use std::collections::HashMap;
    use std::sync::Arc;

    // `ecdf_plot` and `density_plot` are the two ways of showing a distribution
    // that a histogram's bin width cannot distort: the ECDF has no parameter at
    // all, and a density has one that is stated rather than implied by where
    // the bin edges happened to fall.

    fn numbers(values: &[f64]) -> Value {
        Value::List(Arc::new(values.iter().copied().map(Value::Float).collect()))
    }

    fn options(pairs: &[(&str, Value)]) -> Value {
        let mut record = HashMap::new();
        for (key, value) in pairs {
            record.insert((*key).to_string(), value.clone());
        }
        Value::Record(Arc::new(record))
    }

    fn render(name: &str, args: Vec<Value>) -> String {
        match call_plot_builtin(name, args).expect("plot renders") {
            Value::Str(svg) => svg.to_string(),
            other => panic!("{name} should return SVG, got {other:?}"),
        }
    }

    /// Every case here is `bw.nrd0` from R 4.6.1, printed to twelve places. A
    /// density is only comparable with one drawn in R if the default smoothing
    /// agrees, so these are checked against the reference rather than against
    /// whatever the formula currently returns.
    fn assert_matches_r(label: &str, values: &[f64], expected: f64) {
        let got = silverman_bandwidth(values);
        assert!(
            (got - expected).abs() < 1e-11,
            "{label}: bw.nrd0 is {expected}, got {got}"
        );
    }

    #[test]
    fn the_default_bandwidth_is_the_one_r_picks() {
        assert_matches_r(
            "primes",
            &[2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0],
            5.124_406_997_583,
        );
    }

    #[test]
    fn one_far_out_value_does_not_widen_the_bandwidth() {
        // sd is 30.9 here and the IQR is 4.5. Taking the smaller is the whole
        // point of the rule: one outlier must not smooth the other nine values
        // into a single hill. It also pins the divisor -- R uses 1.34, and 1.349
        // (the exact interquartile range of a standard normal, and what the
        // comment in R's own source says) would give 1.894 instead.
        assert_matches_r(
            "heavy tail",
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0],
            1.906_997_944_138,
        );
    }

    #[test]
    fn a_tied_middle_half_falls_back_to_the_standard_deviation() {
        // The IQR is exactly zero here, so the rule's usual estimate is zero --
        // a bandwidth that would make the density a row of infinite spikes.
        assert_matches_r(
            "tied middle",
            &[0.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 10.0],
            1.338_462_650_764,
        );
    }

    #[test]
    fn a_constant_column_still_gets_a_positive_bandwidth() {
        // sd and IQR are both zero. Nothing here is worth plotting, but the
        // bandwidth is a divisor and must not be zero. R uses the magnitude of
        // an observation, and then 1 when even that is zero.
        assert_matches_r("constant 7", &[7.0; 6], 4.402_610_848_261);
        assert_matches_r("constant 0", &[0.0; 6], 0.628_944_406_894);
    }

    #[test]
    fn the_ecdf_is_drawn_as_steps_not_as_a_joined_line() {
        // Two segments per observation -- the riser at the value, then the flat
        // run to the next one. Joining the points directly would draw a
        // distribution that is smooth between observations, which is exactly
        // what an ECDF is not.
        let svg = render("ecdf_plot", vec![numbers(&[1.0, 2.0, 3.0, 4.0])]);
        let points = svg
            .split(r#"<polyline points=""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("the step is drawn as a polyline");
        // Two points per observation -- the top of the riser and the end of the
        // flat run -- plus the corner it starts from on the axis.
        assert_eq!(
            points.split(' ').count(),
            9,
            "expected 2 vertices per observation: {points}"
        );
        // Consecutive pairs must share alternately an x then a y: that is what
        // makes it a staircase rather than a line joining the observations.
        let vertices: Vec<(&str, &str)> = points
            .split(' ')
            .map(|point| point.split_once(',').expect("x,y"))
            .collect();
        for pair in vertices.windows(2) {
            assert!(
                pair[0].0 == pair[1].0 || pair[0].1 == pair[1].1,
                "{:?} -> {:?} is a diagonal, not a step",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn the_ecdf_says_what_its_y_axis_means() {
        let svg = render("ecdf_plot", vec![numbers(&[1.0, 2.0, 3.0])]);
        assert!(svg.contains(">Proportion at or below<"), "{svg:.400}");

        let labelled = render(
            "ecdf_plot",
            vec![
                numbers(&[1.0, 2.0, 3.0]),
                options(&[("ylabel", Value::Str("fraction of samples".into()))]),
            ],
        );
        assert!(labelled.contains(">fraction of samples<"));
        assert!(!labelled.contains(">Proportion at or below<"));
    }

    #[test]
    fn an_explicit_bandwidth_changes_the_curve() {
        // The failure this guards against is the one that made every axis label
        // in this file wrong for so long: an option accepted, parsed, and then
        // not used. A bandwidth ten times wider must draw a different picture.
        let values = numbers(&[1.0, 2.0, 3.0, 8.0, 9.0, 10.0]);
        let narrow = render(
            "density_plot",
            vec![values.clone(), options(&[("bandwidth", Value::Float(0.2))])],
        );
        let wide = render(
            "density_plot",
            vec![values, options(&[("bandwidth", Value::Float(2.0))])],
        );
        assert_ne!(narrow, wide, "the bandwidth option was ignored");
    }

    #[test]
    fn a_list_with_no_numbers_in_it_is_an_error_rather_than_an_empty_plot() {
        let text = Value::List(Arc::new(vec![Value::Str("setosa".into())]));
        let error = call_plot_builtin("density_plot", vec![text])
            .expect_err("a column of species names is not a distribution");
        assert!(
            error.to_string().contains("no numeric values"),
            "unhelpful message: {error}"
        );
    }

    #[test]
    fn a_numeric_column_read_from_a_csv_still_plots() {
        // Columns arrive as text often enough that rejecting them would be the
        // wrong default -- this is what a `col()` on an unparsed CSV gives you.
        let as_text = Value::List(Arc::new(
            ["1.5", "2.5", "3.5", "4.5"]
                .iter()
                .map(|s| Value::Str((*s).into()))
                .collect(),
        ));
        assert!(render("ecdf_plot", vec![as_text]).starts_with("<svg"));
    }
}

#[cfg(test)]
mod series_and_box_tests {
    use super::{call_plot_builtin, quantile_type7};
    use bl_core::value::{Table, Value};
    use std::collections::HashMap;
    use std::sync::Arc;

    // Two things a plot could not do: draw more than one series on one pair of
    // axes, and agree with `quantile()` about where the quartiles are.

    fn table(columns: &[&str], rows: &[&[f64]]) -> Value {
        Value::Table(Table::new(
            columns.iter().map(|c| (*c).to_string()).collect(),
            rows.iter()
                .map(|row| row.iter().copied().map(Value::Float).collect())
                .collect(),
        ))
    }

    fn options(pairs: &[(&str, Value)]) -> Value {
        let mut record = HashMap::new();
        for (key, value) in pairs {
            record.insert((*key).to_string(), value.clone());
        }
        Value::Record(Arc::new(record))
    }

    fn strings(names: &[&str]) -> Value {
        Value::List(Arc::new(
            names.iter().map(|n| Value::Str((*n).into())).collect(),
        ))
    }

    fn render(args: Vec<Value>) -> String {
        match call_plot_builtin("plot", args).expect("plot renders") {
            Value::Str(svg) => svg.to_string(),
            other => panic!("plot should return SVG, got {other:?}"),
        }
    }

    fn wide() -> Value {
        table(
            &["x", "small", "large"],
            &[
                &[1.0, 1.0, 10.0],
                &[2.0, 2.0, 20.0],
                &[3.0, 3.0, 30.0],
                &[4.0, 4.0, 40.0],
            ],
        )
    }

    fn line_options(columns: &[&str]) -> Value {
        options(&[("type", Value::Str("line".into())), ("y", strings(columns))])
    }

    #[test]
    fn a_list_of_y_columns_draws_one_line_each() {
        let svg = render(vec![wide(), line_options(&["small", "large"])]);
        assert_eq!(
            svg.matches("<polyline").count(),
            2,
            "expected one polyline per series: {svg:.600}"
        );
    }

    #[test]
    fn the_series_share_one_vertical_scale() {
        // The reason to draw them together rather than side by side. If each
        // series were scaled to itself, "small" would occupy the same height
        // alone as it does beside a series ten times larger.
        let alone = render(vec![wide(), line_options(&["small"])]);
        let together = render(vec![wide(), line_options(&["small", "large"])]);
        let first_line = |svg: &str| -> String {
            svg.split(r#"<polyline points=""#)
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .expect("a polyline")
                .to_string()
        };
        assert_ne!(
            first_line(&alone),
            first_line(&together),
            "the small series was not rescaled to the shared range"
        );
    }

    #[test]
    fn a_legend_names_the_series_and_only_when_there_are_several() {
        let several = render(vec![wide(), line_options(&["small", "large"])]);
        assert!(several.contains(">small<"), "{several:.600}");
        assert!(several.contains(">large<"));
        // Once each, in the legend. Letting the y axis default to one of the
        // column names would label the whole scale after half the data.
        assert_eq!(several.matches(">small<").count(), 1, "{several:.600}");
        assert_eq!(several.matches(">large<").count(), 1);

        // One series is named by the y axis, so a legend would only repeat it.
        let single = render(vec![wide(), line_options(&["small"])]);
        assert_eq!(single.matches(">small<").count(), 1, "{single:.600}");
    }

    #[test]
    fn a_plain_string_y_still_selects_one_column() {
        let svg = render(vec![
            wide(),
            options(&[
                ("type", Value::Str("line".into())),
                ("y", Value::Str("large".into())),
            ]),
        ]);
        assert_eq!(svg.matches("<polyline").count(), 1);
        assert!(svg.contains(">large<"), "the y axis should name the column");
    }

    #[test]
    fn grouped_bars_draw_one_bar_per_series_per_category() {
        let svg = render(vec![
            wide(),
            options(&[
                ("type", Value::Str("bar".into())),
                ("y", strings(&["small", "large"])),
            ]),
        ]);
        // Four categories, two series, plus the background rect.
        assert_eq!(svg.matches("<rect").count(), 9, "{svg:.600}");

        // And side by side rather than on top of each other: eight bars at
        // eight distinct x positions. Drawing them at the same x hides one
        // series behind the other, which is the failure a grouped bar chart
        // exists to avoid.
        let mut positions: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .filter_map(|x| x.parse::<f64>().ok())
            .collect();
        positions.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let distinct = {
            let mut copy = positions.clone();
            copy.dedup();
            copy.len()
        };
        assert_eq!(distinct, 8, "bars overlap: {positions:?}");

        // And the leftmost one clear of the y axis, which sits at the default
        // left margin of 60. A bar drawn on the axis reads as part of it.
        assert!(
            positions[0] > 60.0,
            "first bar sits on the axis: {positions:?}"
        );
    }

    #[test]
    fn a_missing_column_is_an_error_rather_than_a_silently_empty_plot() {
        let error = call_plot_builtin(
            "plot",
            vec![wide(), options(&[("y", strings(&["small", "enormous"]))])],
        )
        .expect_err("a column that is not in the table");
        assert!(
            error.to_string().contains("enormous"),
            "the message should name the column: {error}"
        );
    }

    #[test]
    fn the_quartiles_are_the_ones_quantile_reports() {
        // R 4.6.1: quantile(1:10) gives 3.25, 5.5, 7.75. The nearest-rank rule
        // the box plot used gives 3, 5.5 and 8 for the same ten values.
        let ten: Vec<f64> = (1..=10).map(f64::from).collect();
        for (p, expected) in [(0.25, 3.25), (0.5, 5.5), (0.75, 7.75)] {
            let got = quantile_type7(&ten, p);
            assert!(
                (got - expected).abs() < 1e-12,
                "quantile at {p} is {expected}, got {got}"
            );
        }
        // And on the primes, where the interpolation lands off a data point.
        let primes = [2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0];
        assert!((quantile_type7(&primes, 0.75) - 18.5).abs() < 1e-12);
    }

    fn categories(names: &[&str], values: &[f64]) -> Value {
        Value::Table(Table::new(
            vec!["dept".to_string(), "rate".to_string()],
            names
                .iter()
                .zip(values.iter())
                .map(|(name, value)| vec![Value::Str((*name).into()), Value::Float(*value)])
                .collect(),
        ))
    }

    #[test]
    fn a_bar_chart_puts_the_category_names_under_the_bars() {
        // A category has no numeric position, so reading the column as numbers
        // gives NaN and an axis running 0.0 to 1.0 under bars that have nothing
        // to do with those numbers. That is what this used to draw.
        let svg = render(vec![
            categories(&["Biology", "Chemistry", "Physics"], &[0.62, 0.34, 0.51]),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        for department in ["Biology", "Chemistry", "Physics"] {
            assert!(
                svg.contains(&format!(">{department}<")),
                "{department} is missing: {svg:.700}"
            );
        }
        // And nothing else on that row: the tick labels under a bar chart are
        // the categories, not numbers interpolated from a column of NaN. The
        // default 600px canvas puts them at y=568.
        let under_the_axis: Vec<&str> = svg
            .split(r#" y="568.0""#)
            .skip(1)
            .filter_map(|fragment| fragment.split('>').nth(1))
            .filter_map(|text| text.split('<').next())
            .collect();
        assert_eq!(
            under_the_axis,
            vec!["Biology", "Chemistry", "Physics"],
            "the x tick labels are not the categories"
        );

        // Centred in its slot, not on the edge between two of them. With three
        // categories across the default 720px plot area, the first belongs at
        // 60 + 240/2 = 180.
        let biology_x = svg
            .split(r#"<text x=""#)
            .find(|fragment| fragment.contains(">Biology<"))
            .and_then(|fragment| fragment.split('"').next())
            .and_then(|x| x.parse::<f64>().ok())
            .expect("a Biology label with an x");
        assert!(
            (biology_x - 180.0).abs() < 1.0,
            "the first category label sits at {biology_x}, not centred at 180"
        );
    }

    #[test]
    fn a_scatter_keeps_its_numeric_axis() {
        // The x values are deliberately not round, so a numeric axis puts ticks
        // between them and a category axis would print the values themselves.
        let uneven = Value::Table(Table::new(
            vec!["x".to_string(), "y".to_string()],
            [0.0, 33.0, 67.0, 100.0]
                .iter()
                .map(|x| vec![Value::Float(*x), Value::Float(*x)])
                .collect(),
        ));
        let svg = render(vec![uneven, options(&[("x", Value::Str("x".into()))])]);
        assert!(
            svg.contains(">20<") || svg.contains(">20.0<"),
            "the x axis should be numbered, not labelled: {svg:.700}"
        );
        assert!(
            !svg.contains(">33<"),
            "the data values are being used as tick labels: {svg:.700}"
        );
    }

    #[test]
    fn too_many_categories_to_fit_are_thinned_rather_than_overlapped() {
        // Sixty labels across seven hundred pixels is unreadable mush. Fewer
        // labels is worse than all of them only when they fit.
        let names: Vec<String> = (1..=60).map(|n| format!("sample{n}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let values: Vec<f64> = (1..=60).map(f64::from).collect();
        let svg = render(vec![
            categories(&refs, &values),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        let drawn = names
            .iter()
            .filter(|name| svg.contains(&format!(">{name}<")))
            .count();
        assert!(drawn > 0, "no labels at all");
        assert!(drawn < 60, "all 60 labels were drawn: they cannot fit");
        // Still thinned evenly rather than truncated: the last one is present.
        assert!(svg.contains(">sample1<"), "the first label is missing");
    }

    #[test]
    fn a_bar_chart_measures_from_zero() {
        // Two counts, one twice the other, must be drawn as one bar twice the
        // height of the other. Scaling to the data's own range instead makes
        // the smaller bar vanish entirely, which is how a 3% difference gets
        // published as a dramatic one.
        let svg = render(vec![
            categories(&["few", "many"], &[10.0, 20.0]),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        let heights: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|fragment| {
                fragment
                    .split(r#"height=""#)
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|h| h.parse::<f64>().ok())
            })
            .collect();
        assert_eq!(heights.len(), 2, "expected two bars: {svg:.700}");
        assert!(heights[0] > 1.0, "the smaller bar collapsed to {heights:?}");
        assert!(
            (heights[1] / heights[0] - 2.0).abs() < 0.05,
            "20 should be twice the height of 10, got {heights:?}"
        );
    }

    #[test]
    fn a_negative_bar_hangs_below_the_zero_line() {
        // Bars grow from zero in both directions. Growing them from the
        // smallest value instead flattens the negative one to nothing, and it
        // is the negative bar that usually carries the news.
        let svg = render(vec![
            categories(&["loss", "gain"], &[-5.0, 10.0]),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        let heights: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|fragment| {
                fragment
                    .split(r#"height=""#)
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|h| h.parse::<f64>().ok())
            })
            .collect();
        assert_eq!(heights.len(), 2, "expected two bars: {svg:.700}");
        assert!(
            heights[0] > 1.0,
            "the negative bar collapsed to {heights:?}"
        );
        assert!(
            (heights[1] / heights[0] - 2.0).abs() < 0.05,
            "10 should be twice the length of -5, got {heights:?}"
        );

        // Both bars must meet at the zero line: the negative one starts there
        // and hangs down, the positive one ends there. A rect placed at the far
        // end of its own length instead is the right size in the wrong place.
        let tops: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|fragment| {
                fragment
                    .split(r#" y=""#)
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|y| y.parse::<f64>().ok())
            })
            .collect();
        let zero_line = tops[1] + heights[1];
        assert!(
            (tops[0] - zero_line).abs() < 0.2,
            "the negative bar starts at {}, the zero line is at {zero_line}",
            tops[0]
        );
    }

    #[test]
    fn a_scatter_still_frames_the_data_it_has() {
        // The zero rule is for bars only. Forcing a scatter of values around
        // 1000 to include the origin would waste the whole plot area.
        let svg = render(vec![
            Value::Table(Table::new(
                vec!["x".to_string(), "y".to_string()],
                (0..4)
                    .map(|n| {
                        vec![
                            Value::Float(f64::from(n)),
                            Value::Float(1000.0 + f64::from(n)),
                        ]
                    })
                    .collect(),
            )),
            options(&[("type", Value::Str("scatter".into()))]),
        ]);
        assert!(
            !svg.contains(">0<") || svg.contains(">1000<"),
            "the y axis was stretched down to zero: {svg:.700}"
        );
        assert!(svg.contains(">1000<") || svg.contains(">1000.0<"));
    }

    #[test]
    fn a_box_plot_marks_the_points_beyond_the_whiskers() {
        // Tukey's rule is the reason to draw a box plot at all: the whisker
        // stops at the last value within 1.5 IQR of the box and everything past
        // it gets its own mark. Reaching to the extremes instead draws every
        // dataset as though it had no outliers.
        // Three of the columns run 1 to 7 with the last value replaced, so
        // each has a box from 2.75 to 6.25 and an IQR of 3.5, putting the fence
        // at 11.5. The odd values sit either side of it: 14 is past it and must
        // be marked, 10.5 is inside it and must not. A wider 3 IQR rule (fence
        // 16.75) marks neither; a narrower 1 IQR rule (fence 9.75) marks both.
        // Something merely enormous would pass whatever multiple was used.
        //
        // The fourth column puts values below the box instead of above it,
        // because the two whiskers are separate pieces of code. It runs
        // -6, -1, 3..8: the box is 2 to 6.25, the fence 6.375, so -6 is out and
        // -1 is not. Two low values rather than one, so that reaching down from
        // the wrong quartile is visible as an extra mark rather than landing on
        // the same answer by luck.
        let rows: [[f64; 4]; 8] = [
            [1.0, 1.0, 1.0, -6.0],
            [2.0, 2.0, 2.0, -1.0],
            [3.0, 3.0, 3.0, 3.0],
            [4.0, 4.0, 4.0, 4.0],
            [5.0, 5.0, 5.0, 5.0],
            [6.0, 6.0, 6.0, 6.0],
            [7.0, 7.0, 7.0, 7.0],
            [8.0, 10.5, 14.0, 8.0],
        ];
        let both = Value::Table(Table::new(
            vec![
                "clean".to_string(),
                "borderline".to_string(),
                "spiky".to_string(),
                "low".to_string(),
            ],
            rows.iter()
                .map(|row| row.iter().copied().map(Value::Float).collect())
                .collect(),
        ));
        let clean_only = table(
            &["clean", "also_clean"],
            &[
                &[1.0, 1.0],
                &[2.0, 2.0],
                &[3.0, 3.0],
                &[4.0, 4.0],
                &[5.0, 5.0],
                &[6.0, 6.0],
                &[7.0, 7.0],
                &[8.0, 8.0],
            ],
        );
        let box_opts = options(&[("type", Value::Str("box".into()))]);

        let plain = render(vec![clean_only, box_opts.clone()]);
        assert_eq!(
            plain.matches("<circle").count(),
            0,
            "nothing here is beyond a fence: {plain:.600}"
        );

        let flagged = render(vec![both, box_opts]);
        assert_eq!(
            flagged.matches("<circle").count(),
            2,
            "the 14 and the -6 should both be marked: {flagged:.600}"
        );

        // The median line is the most-read mark on a box plot, and nothing
        // above would notice it being drawn at the wrong height. The first
        // column runs 1 to 8, whose quartiles are 2.75, 4.5 and 6.25 -- the
        // median exactly halfway up the box -- so the line has to land on the
        // box's midpoint whatever the vertical scale turns out to be.
        let attribute = |fragment: &str, name: &str| -> f64 {
            fragment
                .split(&format!("{name}=\""))
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or_else(|| panic!("no {name} in {fragment:.120}"))
        };
        let first_box = plain.split(r#"<rect x=""#).nth(1).expect("a box is drawn");
        let box_top = attribute(first_box, "y");
        let box_height = attribute(first_box, "height");
        let median_line = plain
            .split(r#"stroke-width="2""#)
            .next()
            .and_then(|before| before.rsplit("<line ").next())
            .expect("a median line");
        let median_y = attribute(median_line, "y1");
        assert!(
            (median_y - (box_top + box_height / 2.0)).abs() < 0.2,
            "median at {median_y}, box from {box_top} spanning {box_height}"
        );
    }
}
