use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

pub fn plot_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("plot", Arity::Range(1, 2)),
        ("plot_spec", Arity::Range(1, 2)),
        ("render_plot", Arity::Range(1, 2)),
        ("plot_grid", Arity::Range(1, 2)),
        ("heatmap", Arity::Range(1, 2)),
        ("mosaic_plot", Arity::Range(1, 2)),
        ("mosaic_data", Arity::Range(1, 2)),
        ("histogram", Arity::Range(1, 2)),
        ("histogram_data", Arity::Range(1, 2)),
        ("boxplot_data", Arity::Range(1, 2)),
        ("ecdf_data", Arity::Range(1, 2)),
        ("normal_qq_data", Arity::Range(1, 2)),
        ("violin_data", Arity::Range(1, 2)),
        ("linear_fit_data", Arity::Range(2, 3)),
        ("categorical_data", Arity::Exact(1)),
        ("missingness_data", Arity::Range(1, 2)),
        ("ecdf_plot", Arity::Range(1, 2)),
        ("density_plot", Arity::Range(1, 2)),
        ("volcano", Arity::Range(1, 2)),
        ("ma_plot", Arity::Range(1, 2)),
        ("save_svg", Arity::Range(2, 3)),
        ("save_plot", Arity::Range(2, 3)),
        ("save_png", Arity::Range(2, 3)),
        ("genome_track", Arity::Range(1, 2)),
    ]
}

pub fn is_plot_builtin(name: &str) -> bool {
    matches!(
        name,
        "plot"
            | "plot_spec"
            | "render_plot"
            | "plot_grid"
            | "heatmap"
            | "mosaic_plot"
            | "mosaic_data"
            | "histogram"
            | "histogram_data"
            | "boxplot_data"
            | "ecdf_data"
            | "normal_qq_data"
            | "violin_data"
            | "linear_fit_data"
            | "categorical_data"
            | "missingness_data"
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
    // A plot specification deliberately contains a `data` table. It is the
    // object render_plot() consumes, not the single-record convenience form
    // that normalize_plot_args() expands for ordinary plotting calls.
    let args = if name == "render_plot" {
        args
    } else {
        normalize_plot_args(args)
    };
    match name {
        "plot" => builtin_plot(args),
        "plot_spec" => builtin_plot_spec(args),
        "render_plot" => builtin_render_plot(args),
        "plot_grid" => builtin_plot_grid(args),
        "heatmap" => builtin_heatmap(args),
        "mosaic_plot" => builtin_mosaic_plot(args),
        "mosaic_data" => builtin_mosaic_data(args),
        "histogram" => builtin_histogram(args),
        "histogram_data" => builtin_histogram_data(args),
        "boxplot_data" => builtin_boxplot_data(args),
        "ecdf_data" => builtin_ecdf_data(args),
        "normal_qq_data" => builtin_normal_qq_data(args),
        "violin_data" => builtin_violin_data(args),
        "linear_fit_data" => builtin_linear_fit_data(args),
        "categorical_data" => builtin_categorical_data(args),
        "missingness_data" => builtin_missingness_data(args),
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

/// R `scales::hue_pal()`, the discrete colour scale ggplot2 uses by default.
///
/// ggplot recomputes evenly spaced hues for each group count, so a fixed table
/// is only correct at the single `n` it was copied from. Hues start at 15
/// degrees and step by 360/n at chroma 100 and luminance 65 in CIE-LUV, then
/// convert to sRGB against the D65 white point that R's `hcl()` assumes.
pub(crate) fn hue_palette(count: usize) -> Vec<String> {
    let count = count.max(1);
    (0..count)
        .map(|index| {
            let hue = (15.0 + 360.0 * index as f64 / count as f64).to_radians();
            luv_to_hex(65.0, 100.0 * hue.cos(), 100.0 * hue.sin())
        })
        .collect()
}

/// CIE-LUV to sRGB, matching R's `hcl()` including its out-of-gamut clamp.
fn luv_to_hex(lightness: f64, u_star: f64, v_star: f64) -> String {
    const WHITE_X: f64 = 95.047;
    const WHITE_Y: f64 = 100.0;
    const WHITE_Z: f64 = 108.883;
    if lightness <= 0.0 {
        return "#000000".to_string();
    }
    let denominator = WHITE_X + 15.0 * WHITE_Y + 3.0 * WHITE_Z;
    let u = u_star / (13.0 * lightness) + 4.0 * WHITE_X / denominator;
    let v = v_star / (13.0 * lightness) + 9.0 * WHITE_Y / denominator;
    let y = WHITE_Y
        * if lightness > 8.0 {
            ((lightness + 16.0) / 116.0).powi(3)
        } else {
            lightness / 903.3
        };
    let x = y * 9.0 * u / (4.0 * v);
    let z = y * (12.0 - 3.0 * u - 20.0 * v) / (4.0 * v);
    let (x, y, z) = (x / 100.0, y / 100.0, z / 100.0);
    let linear = [
        3.240479 * x - 1.537150 * y - 0.498535 * z,
        -0.969256 * x + 1.875992 * y + 0.041556 * z,
        0.055648 * x - 0.204043 * y + 1.057311 * z,
    ];
    // `hcl(fixup = TRUE)` is R's default: clamp rather than refuse a hue that
    // falls outside sRGB.
    let channel = |value: f64| -> u8 {
        let gamma = if value <= 0.003_130_8 {
            12.92 * value
        } else {
            1.055 * value.powf(1.0 / 2.4) - 0.055
        };
        (gamma * 255.0).round().clamp(0.0, 255.0) as u8
    };
    format!(
        "#{:02x}{:02x}{:02x}",
        channel(linear[0]),
        channel(linear[1]),
        channel(linear[2])
    )
}

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

/// The opacity `add_circle` writes into every point. The raster path applies it
/// as an alpha byte instead, so a plot looks the same either side of the
/// threshold.
pub(crate) const POINT_ALPHA: f64 = 0.7;

/// Vector circles are one DOM node each. They are ruinous in quantity: at
/// 200,000 points a scatter measures 13.8 MB and 200,012 elements, against
/// 892 KB and 12 for the same figure rasterised. The raster is flat in the
/// point count -- it is bounded by the pixels of the plot area, not the data --
/// so the crossover is a size, not a ratio.
///
/// Below the threshold vector wins on both size and speed, which is why this is
/// a threshold rather than a mode.
pub(crate) const DEFAULT_RASTER_THRESHOLD: usize = 20_000;

/// Whether a scatter draws as circles or as one image, and at what
/// supersampling.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RasterChoice {
    pub(crate) enabled: bool,
    pub(crate) scale: f64,
}

/// Read the shared `raster`, `raster_threshold` and `raster_scale` options.
///
/// Every scatter-like plot takes the same three, spelled the same way and
/// erroring the same way, so a caller who learns them on one plot knows them on
/// all of them. `builtin` only names the plot in those errors.
pub(crate) fn raster_choice(
    opts: &HashMap<String, Value>,
    builtin: &str,
    count: usize,
) -> Result<RasterChoice> {
    let threshold = match opts.get("raster_threshold") {
        None => DEFAULT_RASTER_THRESHOLD,
        Some(Value::Int(value)) if *value > 0 => *value as usize,
        Some(Value::Float(value)) if value.is_finite() && *value >= 1.0 => *value as usize,
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'raster_threshold' must be a positive number"),
                None,
            ))
        }
    };
    let enabled = match opts.get("raster") {
        None => count >= threshold,
        Some(Value::Bool(value)) => *value,
        Some(Value::Str(value)) => match value.to_ascii_lowercase().as_str() {
            "auto" => count >= threshold,
            "on" | "true" => true,
            "off" | "false" => false,
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("{builtin}() option 'raster' must be 'auto', 'on', 'off', or Bool"),
                    None,
                ))
            }
        },
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'raster' must be 'auto', 'on', 'off', or Bool"),
                None,
            ))
        }
    };
    let scale = match opts.get("raster_scale") {
        None => 2.0,
        Some(Value::Int(value)) if (1..=4).contains(value) => *value as f64,
        Some(Value::Float(value)) if value.is_finite() && (1.0..=4.0).contains(value) => *value,
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'raster_scale' must be between 1 and 4"),
                None,
            ))
        }
    };
    Ok(RasterChoice { enabled, scale })
}

/// Whether a plot was asked to thin, and at what pixel size.
pub(crate) fn thin_requested(opts: &HashMap<String, Value>, builtin: &str) -> Result<bool> {
    match opts.get("thin") {
        None => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(Value::Str(value)) => match value.to_ascii_lowercase().as_str() {
            "on" | "true" => Ok(true),
            "off" | "false" => Ok(false),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() option 'thin' must be 'on', 'off', or Bool"),
                None,
            )),
        },
        Some(_) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{builtin}() option 'thin' must be 'on', 'off', or Bool"),
            None,
        )),
    }
}

/// Keep at most one point per device pixel and drop the rest.
///
/// The contract is deliberately narrow, because this changes what a figure
/// shows and no figure should change quietly. A point is dropped only when
/// another point in the same set lands on the same device pixel, so coverage
/// is very nearly preserved and the extent of the cloud does not move in
/// either axis.
///
/// Very nearly, not exactly: a point is a disc, not a pixel. Two points can
/// share a cell and still have their discs reach a fraction of a pixel in
/// different directions, so an anti-aliased edge sliver can go unpainted.
/// Measured on a 60,000-variant Manhattan plot that was 54 pixels out of
/// 687,514 painted, or 0.008%, and no pixel was painted that would not have
/// been painted anyway.
///
/// What is genuinely lost is overdraw. Points are drawn at alpha 0.7, so a
/// saturated region normally accumulates towards opaque; painted once it
/// reads lighter. On that same figure the median painted pixel did not change
/// alpha at all and the 95th percentile moved by 3 of 255, but in the densest
/// pileups the change reached 138. Density stops being legible as shade, and
/// that is a real change to the figure -- which is why callers must opt in and
/// why they record the counts in the figure itself.
///
/// `rank` chooses the survivor within a cell: the largest value wins, so a
/// Manhattan plot passes -log10(p) and keeps the most significant variant in
/// every pixel -- the one a reader is looking for. Equal ranks resolve by
/// input order, so the result does not depend on hash iteration order.
///
/// Returns ascending indices into `points`, so relative draw order survives.
pub(crate) fn thin_to_pixel_grid(
    points: &[(f64, f64)],
    area: (f64, f64, f64, f64),
    scale: f64,
    rank: &[f64],
) -> Vec<usize> {
    use std::collections::hash_map::Entry;

    let (origin_x, origin_y, _, _) = area;
    let scale = scale.clamp(1.0, 4.0);
    let mut best: HashMap<(i64, i64), usize> = HashMap::with_capacity(points.len() / 4 + 1);
    for (index, &(px, py)) in points.iter().enumerate() {
        if !px.is_finite() || !py.is_finite() {
            continue;
        }
        let cell = (
            ((px - origin_x) * scale).floor() as i64,
            ((py - origin_y) * scale).floor() as i64,
        );
        match best.entry(cell) {
            Entry::Vacant(slot) => {
                slot.insert(index);
            }
            Entry::Occupied(mut slot) => {
                let held = *slot.get();
                let challenger = rank.get(index).copied().unwrap_or(0.0);
                let incumbent = rank.get(held).copied().unwrap_or(0.0);
                if challenger > incumbent {
                    slot.insert(index);
                }
            }
        }
    }
    let mut kept: Vec<usize> = best.into_values().collect();
    kept.sort_unstable();
    kept
}

pub(crate) fn sequential_color(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (64.0 + t * 191.0) as u8;
    let g = (64.0 + (1.0 - (2.0 * t - 1.0).abs()) * 128.0) as u8;
    let b = (255.0 - t * 191.0) as u8;
    format!("#{r:02x}{g:02x}{b:02x}")
}

/// Perceptually ordered blue-green-yellow ramp for publication figures.
///
/// Equal numerical steps should look approximately equal in the legend. The
/// historical blue-red ramp is retained by the default theme; this ramp is
/// intentionally opt-in because colour changes are analytically visible.
pub(crate) fn publication_sequential_color(t: f64) -> String {
    const STOPS: [(f64, [u8; 3]); 5] = [
        (0.00, [68, 1, 84]),
        (0.25, [59, 82, 139]),
        (0.50, [33, 145, 140]),
        (0.75, [94, 201, 98]),
        (1.00, [253, 231, 37]),
    ];
    let value = t.clamp(0.0, 1.0);
    let upper = STOPS
        .iter()
        .position(|(at, _)| value <= *at)
        .unwrap_or(STOPS.len() - 1);
    let lower = upper.saturating_sub(1);
    let (lo_at, lo) = STOPS[lower];
    let (hi_at, hi) = STOPS[upper];
    let local = if (hi_at - lo_at).abs() < f64::EPSILON {
        0.0
    } else {
        (value - lo_at) / (hi_at - lo_at)
    };
    // Work in floating point because some channels decrease between stops.
    let channel = |index: usize| {
        (f64::from(lo[index]) + local * (f64::from(hi[index]) - f64::from(lo[index]))).round() as u8
    };
    format!("#{:02x}{:02x}{:02x}", channel(0), channel(1), channel(2))
}

/// Perceptually balanced blue-white-red ramp for values centred on zero.
///
/// Dot plots encode a per-gene z-score, where zero has scientific meaning.
/// A sequential ramp makes a neutral value look like an intermediate amount;
/// this diverging ramp instead gives negative and positive departures equal
/// visual weight. As with the other publication colours, it is opt-in so
/// existing figures retain their historical output.
pub(crate) fn publication_diverging_color(t: f64) -> String {
    const STOPS: [(f64, [u8; 3]); 3] = [
        (0.0, [59, 76, 192]),
        (0.5, [247, 247, 247]),
        (1.0, [180, 4, 38]),
    ];
    let value = t.clamp(0.0, 1.0);
    let upper = STOPS
        .iter()
        .position(|(at, _)| value <= *at)
        .unwrap_or(STOPS.len() - 1);
    let lower = upper.saturating_sub(1);
    let (lo_at, lo) = STOPS[lower];
    let (hi_at, hi) = STOPS[upper];
    let local = if (hi_at - lo_at).abs() < f64::EPSILON {
        0.0
    } else {
        (value - lo_at) / (hi_at - lo_at)
    };
    let channel = |index: usize| {
        (f64::from(lo[index]) + local * (f64::from(hi[index]) - f64::from(lo[index]))).round() as u8
    };
    format!("#{:02x}{:02x}{:02x}", channel(0), channel(1), channel(2))
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
        if count == 0 || !self.domain.0.is_finite() || !self.domain.1.is_finite() {
            return Vec::new();
        }
        let reversed = self.domain.0 > self.domain.1;
        let lo = self.domain.0.min(self.domain.1);
        let hi = self.domain.0.max(self.domain.1);
        let span = hi - lo;
        if span <= f64::EPSILON {
            return vec![lo];
        }

        // Human-readable 1/2/5 × 10^k spacing, bounded to the data domain.
        // This avoids labels such as 1.4, 2.8, 4.2 on a 0..7 axis while not
        // moving marks or expanding the plotting domain.
        let raw_step = span / count as f64;
        let magnitude = 10.0_f64.powf(raw_step.log10().floor());
        let fraction = raw_step / magnitude;
        let nice_fraction = if fraction <= 1.0 {
            1.0
        } else if fraction <= 2.0 {
            2.0
        } else if fraction <= 5.0 {
            5.0
        } else {
            10.0
        };
        let step = nice_fraction * magnitude;
        let first = (lo / step).ceil() * step;
        let last = (hi / step).floor() * step;
        let mut ticks = Vec::new();
        let mut tick = first;
        while tick <= last + step * 1e-10 && ticks.len() <= count.saturating_mul(2) + 2 {
            ticks.push(if tick.abs() < step * 1e-12 { 0.0 } else { tick });
            tick += step;
        }
        if ticks.is_empty() {
            ticks.extend([lo, hi]);
        }
        if reversed {
            ticks.reverse();
        }
        ticks
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlotThemeKind {
    Legacy,
    Publication,
    Minimal,
    Ggplot,
    Classic,
}

/// Presentation tokens shared by runtime and biological plots.
///
/// Plot geometry must not know about fonts, grids, or journal sizing. Keeping
/// those decisions here lets an existing figure retain its historical output
/// while `{theme: "publication"}` opts into the more carefully laid-out form.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PlotTheme {
    pub(crate) kind: PlotThemeKind,
    pub(crate) name: &'static str,
    pub(crate) font_family: &'static str,
    pub(crate) text_colour: &'static str,
    pub(crate) axis_colour: &'static str,
    pub(crate) grid_colour: &'static str,
    pub(crate) panel_colour: &'static str,
    pub(crate) background_colour: &'static str,
    pub(crate) title_size: f64,
    pub(crate) subtitle_size: f64,
    pub(crate) axis_title_size: f64,
    pub(crate) tick_size: f64,
    pub(crate) legend_size: f64,
    pub(crate) caption_size: f64,
    pub(crate) axis_width: f64,
    pub(crate) grid_width: f64,
}

impl PlotTheme {
    pub(crate) fn from_name(name: &str) -> Self {
        match name.to_ascii_lowercase().as_str() {
            "publication" | "paper" => Self {
                kind: PlotThemeKind::Publication,
                name: "publication",
                font_family: "Arial, Helvetica, sans-serif",
                text_colour: "#202124",
                axis_colour: "#303238",
                grid_colour: "#e5e7eb",
                panel_colour: "#ffffff",
                background_colour: "#ffffff",
                title_size: 17.0,
                subtitle_size: 11.5,
                axis_title_size: 12.0,
                tick_size: 10.5,
                legend_size: 10.5,
                caption_size: 9.0,
                axis_width: 1.0,
                grid_width: 0.75,
            },
            // ggplot2 `theme_classic()`: white panel, no grid, black axes.
            "classic" => Self {
                kind: PlotThemeKind::Classic,
                name: "classic",
                font_family: "Arial, Helvetica, sans-serif",
                text_colour: "#333333",
                axis_colour: "#000000",
                grid_colour: "#ffffff",
                panel_colour: "#ffffff",
                background_colour: "#ffffff",
                title_size: 16.0,
                subtitle_size: 11.0,
                axis_title_size: 13.0,
                tick_size: 11.0,
                legend_size: 12.0,
                caption_size: 9.0,
                axis_width: 1.0,
                grid_width: 0.0,
            },
            "minimal" => Self {
                kind: PlotThemeKind::Minimal,
                name: "minimal",
                font_family: "Arial, Helvetica, sans-serif",
                text_colour: "#202124",
                axis_colour: "#4b4f58",
                grid_colour: "#eef0f2",
                panel_colour: "#ffffff",
                background_colour: "#ffffff",
                title_size: 16.0,
                subtitle_size: 11.0,
                axis_title_size: 11.5,
                tick_size: 10.0,
                legend_size: 10.0,
                caption_size: 8.5,
                axis_width: 0.8,
                grid_width: 0.65,
            },
            // Compatibility preset for lessons that reproduce an analysis
            // originally taught with ggplot2. This is deliberately a visual
            // preset only; it does not change the data or plot geometry.
            "ggplot" | "ggplot2" => Self {
                kind: PlotThemeKind::Ggplot,
                name: "ggplot",
                font_family: "Arial, Helvetica, sans-serif",
                text_colour: "#333333",
                axis_colour: "#333333",
                grid_colour: "#ffffff",
                panel_colour: "#ebebeb",
                background_colour: "#ffffff",
                title_size: 16.0,
                subtitle_size: 11.0,
                axis_title_size: 13.0,
                tick_size: 11.0,
                legend_size: 12.0,
                caption_size: 9.0,
                axis_width: 0.8,
                grid_width: 1.0,
            },
            // `seurat` intentionally remains presentation-compatible with the
            // old renderer. It changes palettes in biological plots, not the
            // whole layout. Existing saved figures therefore do not move.
            _ => Self {
                kind: PlotThemeKind::Legacy,
                name: "biolang",
                font_family: "sans-serif",
                text_colour: "#111111",
                axis_colour: "#333333",
                grid_colour: "#ffffff",
                panel_colour: "#ffffff",
                background_colour: "#ffffff",
                title_size: 16.0,
                subtitle_size: 11.0,
                axis_title_size: 13.0,
                tick_size: 11.0,
                legend_size: 12.0,
                caption_size: 9.0,
                axis_width: 1.0,
                grid_width: 0.0,
            },
        }
    }

    pub(crate) fn is_adaptive(self) -> bool {
        self.kind != PlotThemeKind::Legacy
    }
}

pub(crate) fn plot_theme(opts: &HashMap<String, Value>) -> PlotTheme {
    PlotTheme::from_name(get_opt_str(opts, "theme", "biolang"))
}

/// Theme for the statistical plot family.
///
/// These plots reproduce analyses that are taught in R, so they default to
/// ggplot2's appearance rather than BioLang's legacy palette. Biological
/// figures keep `plot_theme` and its historical look. `{theme: "..."}`
/// overrides either.
pub(crate) fn stats_plot_theme(opts: &HashMap<String, Value>) -> PlotTheme {
    PlotTheme::from_name(get_opt_str(opts, "theme", "ggplot"))
}

/// Advance widths for the Arial / Helvetica / Liberation Sans stack, in units
/// per 1000 em, for ASCII 32 through 126.
///
/// Those three faces are metric-compatible by design and are exactly the stack
/// every BioLang plot names, so these numbers are the real ones for almost
/// every viewer rather than an approximation of them.
const ADVANCE_PER_MILLE: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556,
    556, 556, 556, 556, 556, 556, 556, 278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667,
    611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667,
    667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Deterministic text width, used for margins and legends.
///
/// SVG leaves final shaping to the viewer, but a layout engine needs a width
/// before a browser exists. A table of real advance widths is both exact for
/// the named fonts and stable across the CLI, WASM, and tests, where the
/// character-class estimate this replaces ran up to 16% wide - enough to
/// visibly inflate margins and legend boxes.
pub(crate) fn estimate_text_width(text: &str, size: f64) -> f64 {
    let per_mille: u32 = text
        .chars()
        .map(|character| {
            let code = character as u32;
            if (32..127).contains(&code) {
                u32::from(ADVANCE_PER_MILLE[(code - 32) as usize])
            } else if character.is_control() {
                0
            } else {
                // Outside the table. Non-Latin glyphs are usually wider than
                // the Latin mean, so err on the generous side: a slightly wide
                // margin is survivable, a clipped label is not.
                820
            }
        })
        .sum();
    f64::from(per_mille) / 1000.0 * size
}

pub(crate) struct SvgCanvas {
    pub(crate) width: f64,
    pub(crate) height: f64,
    pub(crate) margin: Margin,
    pub(crate) elements: Vec<String>,
    pub(crate) theme: PlotTheme,
    accessible_label: Option<String>,
    accessible_description: Option<String>,
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
pub(crate) fn tick_decimals(ticks: &[f64]) -> usize {
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
        Self::with_theme(width, height, PlotTheme::from_name("biolang"))
    }

    pub(crate) fn with_theme(width: f64, height: f64, theme: PlotTheme) -> Self {
        Self {
            width,
            height,
            margin: Margin::default(),
            elements: Vec::new(),
            theme,
            accessible_label: None,
            accessible_description: None,
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

    /// `add_rect` with an outline, as ggplot2's `geom_boxplot()` draws its box.
    pub(crate) fn add_stroked_rect(
        &mut self,
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        fill: &str,
        stroke: &str,
        stroke_width: f64,
    ) {
        self.elements.push(format!(
            r#"<rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" fill="{fill}" stroke="{stroke}" stroke-width="{stroke_width:.2}" />"#
        ));
    }

    pub(crate) fn add_circle(&mut self, cx: f64, cy: f64, r: f64, fill: &str) {
        self.add_circle_with_opacity(cx, cy, r, fill, 0.7);
    }

    /// `add_circle` with the point opacity chosen by the caller.
    ///
    /// ggplot2's `geom_point()` is opaque unless `alpha` is set, so plots that
    /// reproduce an R figure need to override the 0.7 default.
    pub(crate) fn add_circle_with_opacity(
        &mut self,
        cx: f64,
        cy: f64,
        r: f64,
        fill: &str,
        opacity: f64,
    ) {
        self.elements.push(format!(
            r#"<circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{fill}" opacity="{opacity:.2}" />"#
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
        raster_scale: f64,
    ) {
        use base64::{engine::general_purpose::STANDARD, Engine};

        let (x, y, width, height) = area;
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        // Supersampled, so a 3-point dot does not turn into a hard square and
        // the raster survives being viewed at 2x.
        let scale = raster_scale.clamp(1.0, 4.0);
        let pixel_width = (width * scale).ceil().max(1.0) as u32;
        let pixel_height = (height * scale).ceil().max(1.0) as u32;
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
            let cx = ((px - x) * scale) as f32;
            let cy = ((py - y) * scale) as f32;
            let Some(circle) = tiny_skia::PathBuilder::from_circle(cx, cy, (radius * scale) as f32)
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

    /// Draw a scatter as vector circles or as one raster image.
    ///
    /// The choice is the caller's; this exists so every plot makes it the same
    /// way and so the two paths cannot drift apart in appearance. Colours stay
    /// strings on the vector path -- converting them to bytes and back would
    /// silently blacken any named colour, since `hex_to_rgba` only understands
    /// hex.
    pub(crate) fn add_scatter<S: AsRef<str>>(
        &mut self,
        points: &[(f64, f64, S)],
        radius: f64,
        area: (f64, f64, f64, f64),
        choice: RasterChoice,
    ) {
        if choice.enabled {
            let dots: Vec<(f64, f64, [u8; 4])> = points
                .iter()
                .map(|(x, y, fill)| (*x, *y, hex_to_rgba(fill.as_ref(), POINT_ALPHA)))
                .collect();
            self.add_point_raster(&dots, radius, area, choice.scale);
            return;
        }
        for (x, y, fill) in points {
            self.add_circle(*x, *y, radius, fill.as_ref());
        }
    }

    /// The rectangle a scatter's points occupy, for `add_scatter`.
    pub(crate) fn point_area(&self) -> (f64, f64, f64, f64) {
        (
            self.margin.left,
            self.margin.top,
            self.plot_width(),
            self.plot_height(),
        )
    }

    pub(crate) fn add_text(&mut self, x: f64, y: f64, text: &str, anchor: &str, size: f64) {
        self.add_text_styled(x, y, text, anchor, size, "normal", self.theme.text_colour);
    }

    pub(crate) fn add_text_styled(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        anchor: &str,
        size: f64,
        weight: &str,
        fill: &str,
    ) {
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="{}" font-weight="{weight}" fill="{fill}">{escaped}</text>"#,
            self.theme.font_family
        ));
    }

    pub(crate) fn add_axis_title(
        &mut self,
        x: f64,
        y: f64,
        text: &str,
        axis: &str,
        angle: Option<f64>,
    ) {
        if text.is_empty() {
            return;
        }
        let escaped = text
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        let transform = angle
            .map(|angle| format!(r#" transform="rotate({angle},{x:.1},{y:.1})""#))
            .unwrap_or_default();
        self.elements.push(format!(
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="middle" font-size="{}" font-family="{}" fill="{}" data-biolang-axis-title="{axis}"{transform}>{escaped}</text>"#,
            self.theme.axis_title_size, self.theme.font_family, self.theme.text_colour
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
            r#"<text x="{x:.1}" y="{y:.1}" text-anchor="{anchor}" font-size="{size}" font-family="{}" fill="{}" transform="rotate({angle},{x:.1},{y:.1})">{escaped}</text>"#,
            self.theme.font_family,
            self.theme.text_colour
        ));
    }

    /// Fit the panel around its actual labels rather than relying on one set of
    /// margins for every data range and figure size.
    pub(crate) fn fit_cartesian_layout(
        &mut self,
        x_ticks: &[f64],
        y_ticks: &[f64],
        x_label: &str,
        y_label: &str,
        title: &str,
        subtitle: &str,
        caption: &str,
        right_reserve: f64,
    ) {
        if !self.theme.is_adaptive() {
            return;
        }
        let x_decimals = tick_decimals(x_ticks);
        let y_decimals = tick_decimals(y_ticks);
        let widest_y = y_ticks
            .iter()
            .map(|tick| estimate_text_width(&format!("{tick:.y_decimals$}"), self.theme.tick_size))
            .fold(0.0, f64::max);
        let widest_x_half = x_ticks
            .iter()
            .map(|tick| {
                estimate_text_width(&format!("{tick:.x_decimals$}"), self.theme.tick_size) / 2.0
            })
            .fold(0.0, f64::max);

        self.margin.left = (widest_y + if y_label.is_empty() { 24.0 } else { 42.0 })
            .max(46.0)
            .min(self.width * 0.32);
        self.margin.right = (18.0 + right_reserve + widest_x_half * 0.25)
            .max(20.0)
            .min(self.width * 0.42);
        self.margin.top = if title.is_empty() {
            22.0
        } else if subtitle.is_empty() {
            48.0
        } else {
            66.0
        };
        self.margin.bottom = 24.0
            + if x_label.is_empty() { 10.0 } else { 24.0 }
            + if caption.is_empty() { 0.0 } else { 20.0 };
    }

    /// Draw publication-theme grid lines before marks are added.
    pub(crate) fn draw_cartesian_grid(&mut self, x_scale: &Scale, y_scale: &Scale) {
        if self.theme.grid_width <= 0.0 {
            return;
        }
        let left = self.margin.left;
        let right = left + self.plot_width();
        let top = self.margin.top;
        let bottom = top + self.plot_height();
        self.add_rect(
            left,
            top,
            self.plot_width(),
            self.plot_height(),
            self.theme.panel_colour,
        );
        let mapped_x = Scale {
            domain: x_scale.domain,
            range: (left, right),
        };
        for tick in x_scale.nice_ticks(5) {
            let x = mapped_x.map(tick);
            self.add_line(
                x,
                top,
                x,
                bottom,
                self.theme.grid_colour,
                self.theme.grid_width,
            );
        }
        let mapped_y = Scale {
            domain: y_scale.domain,
            range: (bottom, top),
        };
        for tick in y_scale.nice_ticks(5) {
            let y = mapped_y.map(tick);
            self.add_line(
                left,
                y,
                right,
                y,
                self.theme.grid_colour,
                self.theme.grid_width,
            );
        }
    }

    /// The panel and its horizontal gridlines only.
    ///
    /// A categorical x axis has no numeric ticks to rule against, so a plot
    /// with groups along the bottom still needs the themed panel that
    /// `draw_cartesian_grid` would otherwise supply.
    pub(crate) fn draw_categorical_grid(&mut self, y_scale: &Scale) {
        if self.theme.grid_width <= 0.0 {
            return;
        }
        let left = self.margin.left;
        let right = left + self.plot_width();
        let top = self.margin.top;
        let bottom = top + self.plot_height();
        self.add_rect(
            left,
            top,
            self.plot_width(),
            self.plot_height(),
            self.theme.panel_colour,
        );
        let mapped_y = Scale {
            domain: y_scale.domain,
            range: (bottom, top),
        };
        for tick in y_scale.nice_ticks(5) {
            let y = mapped_y.map(tick);
            self.add_line(
                left,
                y,
                right,
                y,
                self.theme.grid_colour,
                self.theme.grid_width,
            );
        }
    }

    pub(crate) fn draw_x_axis(&mut self, scale: &Scale, label: &str) {
        let y = self.margin.top + self.plot_height();
        self.add_line(
            self.margin.left,
            y,
            self.margin.left + self.plot_width(),
            y,
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        let x_scale = Scale {
            domain: scale.domain,
            range: (self.margin.left, self.margin.left + self.plot_width()),
        };
        let ticks = scale.nice_ticks(5);
        let decimals = tick_decimals(&ticks);
        for tick in ticks {
            let x = x_scale.map(tick);
            self.add_line(
                x,
                y,
                x,
                y + 5.0,
                self.theme.axis_colour,
                self.theme.axis_width,
            );
            self.add_text(
                x,
                y + 18.0,
                &format!("{tick:.decimals$}"),
                "middle",
                self.theme.tick_size,
            );
        }
        self.add_axis_title(
            self.margin.left + self.plot_width() / 2.0,
            if self.theme.is_adaptive() {
                y + 36.0
            } else {
                self.height - 5.0
            },
            label,
            "x",
            None,
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
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        if !labels.is_empty() {
            let slot = self.plot_width() / labels.len() as f64;
            // Roughly 46px of room per label before they start to collide.
            let step = (46.0 / slot).ceil().max(1.0) as usize;
            for (index, label) in labels.iter().enumerate().step_by(step) {
                let x = self.margin.left + slot * (index as f64 + 0.5);
                self.add_line(
                    x,
                    y,
                    x,
                    y + 5.0,
                    self.theme.axis_colour,
                    self.theme.axis_width,
                );
                self.add_text(x, y + 18.0, label, "middle", self.theme.tick_size);
            }
        }
        if !axis_label.is_empty() {
            self.add_axis_title(
                self.margin.left + self.plot_width() / 2.0,
                if self.theme.is_adaptive() {
                    y + 36.0
                } else {
                    self.height - 5.0
                },
                axis_label,
                "x",
                None,
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
            self.theme.axis_colour,
            self.theme.axis_width,
        );
        let y_scale = Scale {
            domain: scale.domain,
            range: (self.margin.top + self.plot_height(), self.margin.top),
        };
        let ticks = scale.nice_ticks(5);
        let decimals = tick_decimals(&ticks);
        for tick in ticks {
            let y = y_scale.map(tick);
            self.add_line(
                x - 5.0,
                y,
                x,
                y,
                self.theme.axis_colour,
                self.theme.axis_width,
            );
            self.add_text(
                x - 8.0,
                y + 4.0,
                &format!("{tick:.decimals$}"),
                "end",
                self.theme.tick_size,
            );
        }
        self.add_axis_title(
            15.0,
            self.margin.top + self.plot_height() / 2.0,
            label,
            "y",
            Some(-90.0),
        );
    }

    pub(crate) fn draw_title(&mut self, title: &str) {
        self.accessible_label = Some(title.to_string());
        if self.theme.is_adaptive() {
            self.add_text_styled(
                self.margin.left,
                24.0,
                title,
                "start",
                self.theme.title_size,
                "600",
                self.theme.text_colour,
            );
        } else {
            self.add_text(
                self.width / 2.0,
                25.0,
                title,
                "middle",
                self.theme.title_size,
            );
        }
    }

    pub(crate) fn draw_subtitle(&mut self, subtitle: &str) {
        if subtitle.is_empty() {
            return;
        }
        self.add_text_styled(
            self.margin.left,
            42.0,
            subtitle,
            "start",
            self.theme.subtitle_size,
            "normal",
            "#5f6368",
        );
    }

    pub(crate) fn draw_caption(&mut self, caption: &str) {
        if caption.is_empty() {
            return;
        }
        self.add_text_styled(
            self.width - self.margin.right,
            self.height - 5.0,
            caption,
            "end",
            self.theme.caption_size,
            "normal",
            "#686d76",
        );
    }

    pub(crate) fn set_accessible_description(&mut self, description: impl Into<String>) {
        self.accessible_description = Some(description.into());
    }

    pub(crate) fn render(&self) -> String {
        let escape = |value: &str| {
            value
                .replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
        };
        let label = escape(self.accessible_label.as_deref().unwrap_or("BioLang plot"));
        let description = escape(
            self.accessible_description
                .as_deref()
                .unwrap_or("BioLang data visualization."),
        );
        let mut svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="{}" height="{}" viewBox="0 0 {} {}" role="img" aria-label="{}" data-biolang-theme="{}" focusable="false"><title>{}</title><desc>{}</desc>"#,
            self.width,
            self.height,
            self.width,
            self.height,
            label,
            self.theme.name,
            label,
            description
        );
        svg.push_str(&format!(
            r#"<rect width="{}" height="{}" fill="{}" />"#,
            self.width, self.height, self.theme.background_colour
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
    let panel_right = canvas.margin.left + canvas.plot_width();
    let outside = canvas.theme.is_adaptive();
    for (index, name) in names.iter().enumerate() {
        let y = canvas.margin.top + 14.0 + 18.0 * index as f64;
        let swatch_start = if outside {
            panel_right + 14.0
        } else {
            panel_right - 30.0
        };
        let swatch_end = swatch_start + 22.0;
        canvas.add_line(
            swatch_start,
            y,
            swatch_end,
            y,
            PALETTE[index % PALETTE.len()],
            3.0,
        );
        canvas.add_text(
            if outside {
                swatch_end + 6.0
            } else {
                swatch_start - 6.0
            },
            y + 4.0,
            name,
            if outside { "start" } else { "end" },
            canvas.theme.legend_size,
        );
    }
}

fn legend_reserve_width(theme: PlotTheme, names: &[String]) -> f64 {
    if !theme.is_adaptive() || names.len() < 2 {
        return 0.0;
    }
    let widest = names
        .iter()
        .map(|name| estimate_text_width(name, theme.legend_size))
        .fold(0.0, f64::max);
    (52.0 + widest).clamp(90.0, 210.0)
}

/// Type 7 quantiles — R's default, and what this runtime's `quantile()` gives.
///
/// The box plot used to take `sorted[n / 4]` and `sorted[3 * n / 4]`, which is
/// the nearest-rank rule. On the book's ozone column that puts the top of the
/// box at 64 while `quantile(ozone, 0.75)` reports 63.25, so the picture and
/// the numbers printed beside it disagreed about the same data; on the ten
/// values 1 to 10 the two rules give 3 and 8 against 3.25 and 7.75. Expects
/// `sorted` already sorted and non-empty.
pub(crate) fn quantile_type7(sorted: &[f64], p: f64) -> f64 {
    let h = (sorted.len() - 1) as f64 * p;
    let lower = h.floor() as usize;
    let upper = (lower + 1).min(sorted.len() - 1);
    sorted[lower] + (h - h.floor()) * (sorted[upper] - sorted[lower])
}

pub(crate) const PLOT_SPEC_SCHEMA: &str = "biolang.plot.spec/v1";

#[derive(Clone, Debug)]
struct CartesianPoint {
    source_row: usize,
    x: f64,
    y: f64,
    lower: Option<f64>,
    upper: Option<f64>,
}

#[derive(Clone, Debug)]
struct CartesianSeries {
    name: String,
    colour: String,
    points: Vec<CartesianPoint>,
}

#[derive(Clone, Debug)]
struct CartesianPlotSpec {
    kind: String,
    width: f64,
    height: f64,
    theme: String,
    title: String,
    subtitle: String,
    caption: String,
    x_label: String,
    y_label: String,
    x_domain: (f64, f64),
    y_domain: (f64, f64),
    series: Vec<CartesianSeries>,
    dropped_non_finite: usize,
    x_column: String,
    y_columns: Vec<String>,
    lower_column: Option<String>,
    upper_column: Option<String>,
}

fn interval_column(opts: &HashMap<String, Value>, primary: &str, alias: &str) -> Option<String> {
    opts.get(primary)
        .or_else(|| opts.get(alias))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn build_cartesian_plot_spec(
    table: &Table,
    opts: &HashMap<String, Value>,
    who: &str,
) -> Result<CartesianPlotSpec> {
    if table.num_cols() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() requires table with at least 2 columns"),
            None,
        ));
    }
    let kind = get_opt_str(opts, "type", "scatter").to_ascii_lowercase();
    if !matches!(
        kind.as_str(),
        "scatter" | "line" | "errorbar" | "confidence"
    ) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{who}() specification type '{kind}' is unsupported; expected scatter/line/errorbar/confidence"
            ),
            None,
        ));
    }
    let x_column = get_opt_str(opts, "x", &table.columns[0]).to_string();
    let y_columns = series_columns(opts, &table.columns[1]);
    let lower_column = interval_column(opts, "ymin", "lower");
    let upper_column = interval_column(opts, "ymax", "upper");
    if matches!(kind.as_str(), "errorbar" | "confidence")
        && (lower_column.is_none() || upper_column.is_none())
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() type '{kind}' requires ymin/ymax (or lower/upper) column names"),
            None,
        ));
    }

    let xs = extract_table_col(table, &x_column)?;
    let lowers = lower_column
        .as_deref()
        .map(|column| extract_table_col(table, column))
        .transpose()?;
    let uppers = upper_column
        .as_deref()
        .map(|column| extract_table_col(table, column))
        .transpose()?;
    let mut dropped_non_finite = 0usize;
    let mut series = Vec::with_capacity(y_columns.len());
    for (series_index, column) in y_columns.iter().enumerate() {
        let ys = extract_table_col(table, column)?;
        let mut points = Vec::with_capacity(xs.len().min(ys.len()));
        for row in 0..xs.len().min(ys.len()) {
            let x = xs[row];
            let y = ys[row];
            let lower = lowers.as_ref().and_then(|values| values.get(row)).copied();
            let upper = uppers.as_ref().and_then(|values| values.get(row)).copied();
            let interval_is_valid = match (lower, upper) {
                (Some(lo), Some(hi)) => lo.is_finite() && hi.is_finite() && lo <= hi,
                (None, None) => true,
                _ => false,
            };
            if !x.is_finite() || !y.is_finite() || !interval_is_valid {
                dropped_non_finite += 1;
                continue;
            }
            points.push(CartesianPoint {
                source_row: row,
                x,
                y,
                lower,
                upper,
            });
        }
        series.push(CartesianSeries {
            name: column.clone(),
            colour: PALETTE[series_index % PALETTE.len()].to_string(),
            points,
        });
    }
    if series.iter().all(|item| item.points.is_empty()) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() received no complete finite observations"),
            None,
        ));
    }

    let mut x_values = Vec::new();
    let mut y_values = Vec::new();
    for item in &series {
        for point in &item.points {
            x_values.push(point.x);
            y_values.push(point.lower.unwrap_or(point.y));
            y_values.push(point.upper.unwrap_or(point.y));
        }
    }
    let default_y_label = if y_columns.len() == 1 {
        y_columns[0].as_str()
    } else {
        ""
    };
    Ok(CartesianPlotSpec {
        kind,
        width: get_opt_f64(opts, "width", 800.0).max(1.0),
        height: get_opt_f64(opts, "height", 600.0).max(1.0),
        theme: get_opt_str(opts, "theme", "biolang").to_string(),
        title: get_opt_str(opts, "title", "").to_string(),
        subtitle: get_opt_str(opts, "subtitle", "").to_string(),
        caption: get_opt_str(opts, "caption", "").to_string(),
        x_label: axis_label(opts, "xlabel", &x_column),
        y_label: axis_label(opts, "ylabel", default_y_label),
        x_domain: col_range(&x_values),
        y_domain: col_range(&y_values),
        series,
        dropped_non_finite,
        x_column,
        y_columns,
        lower_column,
        upper_column,
    })
}

fn optional_number(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(Value::as_float)
        .filter(|number| number.is_finite())
}

fn plot_spec_to_value(spec: &CartesianPlotSpec) -> Value {
    let mut rows = Vec::new();
    for item in &spec.series {
        for point in &item.points {
            rows.push(vec![
                Value::Int(point.source_row as i64),
                Value::Str(item.name.clone().into()),
                Value::Str(item.colour.clone().into()),
                Value::Float(point.x),
                Value::Float(point.y),
                point.lower.map(Value::Float).unwrap_or(Value::Nil),
                point.upper.map(Value::Float).unwrap_or(Value::Nil),
            ]);
        }
    }
    let data = Value::Table(Table::new(
        vec![
            "source_row".into(),
            "series".into(),
            "colour".into(),
            "x".into(),
            "y".into(),
            "lower".into(),
            "upper".into(),
        ],
        rows,
    ));
    let provenance = Value::Record(
        HashMap::from([
            ("x_column".into(), Value::Str(spec.x_column.clone().into())),
            (
                "y_columns".into(),
                Value::List(
                    spec.y_columns
                        .iter()
                        .map(|name| Value::Str(name.clone().into()))
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            (
                "lower_column".into(),
                spec.lower_column
                    .as_ref()
                    .map(|name| Value::Str(name.clone().into()))
                    .unwrap_or(Value::Nil),
            ),
            (
                "upper_column".into(),
                spec.upper_column
                    .as_ref()
                    .map(|name| Value::Str(name.clone().into()))
                    .unwrap_or(Value::Nil),
            ),
        ])
        .into(),
    );
    Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str(spec.kind.clone().into())),
            ("width".into(), Value::Float(spec.width)),
            ("height".into(), Value::Float(spec.height)),
            ("theme".into(), Value::Str(spec.theme.clone().into())),
            ("title".into(), Value::Str(spec.title.clone().into())),
            ("subtitle".into(), Value::Str(spec.subtitle.clone().into())),
            ("caption".into(), Value::Str(spec.caption.clone().into())),
            ("xlabel".into(), Value::Str(spec.x_label.clone().into())),
            ("ylabel".into(), Value::Str(spec.y_label.clone().into())),
            (
                "x_domain".into(),
                Value::List(
                    vec![Value::Float(spec.x_domain.0), Value::Float(spec.x_domain.1)].into(),
                ),
            ),
            (
                "y_domain".into(),
                Value::List(
                    vec![Value::Float(spec.y_domain.0), Value::Float(spec.y_domain.1)].into(),
                ),
            ),
            ("data".into(), data),
            (
                "dropped_non_finite".into(),
                Value::Int(spec.dropped_non_finite as i64),
            ),
            ("provenance".into(), provenance),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

fn required_record_string(map: &HashMap<String, Value>, key: &str) -> Result<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() specification field '{key}' must be Str"),
                None,
            )
        })
}

fn valid_spec_colour(colour: &str) -> bool {
    colour.len() == 7
        && colour.starts_with('#')
        && colour[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

fn record_domain(map: &HashMap<String, Value>, key: &str) -> Result<(f64, f64)> {
    let values = match map.get(key) {
        Some(Value::List(values)) if values.len() == 2 => values,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() specification field '{key}' must contain two numbers"),
                None,
            ))
        }
    };
    match (values[0].as_float(), values[1].as_float()) {
        (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() => Ok((lo, hi)),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() specification field '{key}' contains a non-finite value"),
            None,
        )),
    }
}

fn plot_spec_from_value(value: &Value) -> Result<CartesianPlotSpec> {
    let map = match value {
        Value::Record(map) => map,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "render_plot() requires plot specification Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    if !matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() requires schema '{PLOT_SPEC_SCHEMA}'"),
            None,
        ));
    }
    let kind = required_record_string(map, "kind")?;
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() specification field 'data' must be Table",
                None,
            ))
        }
    };
    let indexes = ["source_row", "series", "colour", "x", "y", "lower", "upper"].map(|column| {
        table.col_index(column).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() specification data is missing '{column}'"),
                None,
            )
        })
    });
    let [source_index, series_index, colour_index, x_index, y_index, lower_index, upper_index] = [
        indexes[0].clone()?,
        indexes[1].clone()?,
        indexes[2].clone()?,
        indexes[3].clone()?,
        indexes[4].clone()?,
        indexes[5].clone()?,
        indexes[6].clone()?,
    ];
    let mut series: Vec<CartesianSeries> = Vec::new();
    for row in &table.rows {
        let name = row[series_index].as_str().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data series must be Str",
                None,
            )
        })?;
        let colour = row[colour_index]
            .as_str()
            .unwrap_or(PALETTE[series.len() % PALETTE.len()]);
        if !valid_spec_colour(colour) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data colour must be a #rrggbb value",
                None,
            ));
        }
        let x = optional_number(row.get(x_index)).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data x must be finite",
                None,
            )
        })?;
        let y = optional_number(row.get(y_index)).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data y must be finite",
                None,
            )
        })?;
        let source_row = row[source_index].as_float().unwrap_or(0.0).max(0.0) as usize;
        let position = series.iter().position(|item| item.name == name);
        let target = match position {
            Some(index) => &mut series[index],
            None => {
                series.push(CartesianSeries {
                    name: name.to_string(),
                    colour: colour.to_string(),
                    points: Vec::new(),
                });
                series.last_mut().unwrap()
            }
        };
        target.points.push(CartesianPoint {
            source_row,
            x,
            y,
            lower: optional_number(row.get(lower_index)),
            upper: optional_number(row.get(upper_index)),
        });
    }
    if series.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() specification contains no marks",
            None,
        ));
    }
    let provenance = match map.get("provenance") {
        Some(Value::Record(record)) => Some(record),
        _ => None,
    };
    let x_column = provenance
        .and_then(|record| record.get("x_column"))
        .and_then(Value::as_str)
        .unwrap_or("x")
        .to_string();
    let y_columns = series.iter().map(|item| item.name.clone()).collect();
    Ok(CartesianPlotSpec {
        kind,
        width: optional_number(map.get("width")).unwrap_or(800.0).max(1.0),
        height: optional_number(map.get("height")).unwrap_or(600.0).max(1.0),
        theme: map
            .get("theme")
            .and_then(Value::as_str)
            .unwrap_or("biolang")
            .to_string(),
        title: required_record_string(map, "title")?,
        subtitle: map
            .get("subtitle")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        caption: map
            .get("caption")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        x_label: required_record_string(map, "xlabel")?,
        y_label: required_record_string(map, "ylabel")?,
        x_domain: record_domain(map, "x_domain")?,
        y_domain: record_domain(map, "y_domain")?,
        series,
        dropped_non_finite: optional_number(map.get("dropped_non_finite"))
            .unwrap_or(0.0)
            .max(0.0) as usize,
        x_column,
        y_columns,
        lower_column: None,
        upper_column: None,
    })
}

fn render_cartesian_plot_spec(spec: &CartesianPlotSpec, raster: RasterChoice) -> Result<String> {
    let mut canvas =
        SvgCanvas::with_theme(spec.width, spec.height, PlotTheme::from_name(&spec.theme));
    let point_count = spec
        .series
        .iter()
        .map(|series| series.points.len())
        .sum::<usize>();
    canvas.set_accessible_description(format!(
        "{} plot with {} series and {} rendered marks; {} non-finite rows were excluded.",
        spec.kind,
        spec.series.len(),
        point_count,
        spec.dropped_non_finite
    ));
    let domain_x = Scale {
        domain: spec.x_domain,
        range: spec.x_domain,
    };
    let domain_y = Scale {
        domain: spec.y_domain,
        range: spec.y_domain,
    };
    let series_names = spec
        .series
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();
    canvas.fit_cartesian_layout(
        &domain_x.nice_ticks(5),
        &domain_y.nice_ticks(5),
        &spec.x_label,
        &spec.y_label,
        &spec.title,
        &spec.subtitle,
        &spec.caption,
        legend_reserve_width(canvas.theme, &series_names),
    );
    canvas.draw_cartesian_grid(&domain_x, &domain_y);
    let x_scale = Scale {
        domain: spec.x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: spec.y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    for item in &spec.series {
        match spec.kind.as_str() {
            "scatter" => {
                let points: Vec<(f64, f64, &str)> = item
                    .points
                    .iter()
                    .map(|point| {
                        (
                            x_scale.map(point.x),
                            y_scale.map(point.y),
                            item.colour.as_str(),
                        )
                    })
                    .collect();
                let area = canvas.point_area();
                canvas.add_scatter(&points, 4.0, area, raster);
            }
            "line" => {
                let points = item
                    .points
                    .iter()
                    .map(|point| format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(point.y)))
                    .collect::<Vec<_>>();
                if !points.is_empty() {
                    canvas.elements.push(format!(
                        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
                        points.join(" "),
                        item.colour
                    ));
                }
            }
            "errorbar" => {
                for point in &item.points {
                    let (Some(lower), Some(upper)) = (point.lower, point.upper) else {
                        continue;
                    };
                    let x = x_scale.map(point.x);
                    let top = y_scale.map(upper);
                    let bottom = y_scale.map(lower);
                    canvas.add_line(x, top, x, bottom, &item.colour, 1.5);
                    canvas.add_line(x - 5.0, top, x + 5.0, top, &item.colour, 1.5);
                    canvas.add_line(x - 5.0, bottom, x + 5.0, bottom, &item.colour, 1.5);
                    canvas.add_circle(x, y_scale.map(point.y), 3.5, &item.colour);
                }
            }
            "confidence" => {
                let upper = item.points.iter().filter_map(|point| {
                    point.upper.map(|value| {
                        format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(value))
                    })
                });
                let lower = item.points.iter().rev().filter_map(|point| {
                    point.lower.map(|value| {
                        format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(value))
                    })
                });
                let band = upper.chain(lower).collect::<Vec<_>>();
                if band.len() >= 4 {
                    canvas.elements.push(format!(
                        r#"<polygon points="{}" fill="{}" fill-opacity="0.18" stroke="none" />"#,
                        band.join(" "),
                        item.colour
                    ));
                }
                let centre = item
                    .points
                    .iter()
                    .map(|point| format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(point.y)))
                    .collect::<Vec<_>>();
                if !centre.is_empty() {
                    canvas.elements.push(format!(
                        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
                        centre.join(" "),
                        item.colour
                    ));
                }
            }
            other => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() unsupported specification kind '{other}'"),
                    None,
                ))
            }
        }
    }
    draw_legend(&mut canvas, &series_names);
    canvas.draw_x_axis(
        &Scale {
            domain: spec.x_domain,
            range: spec.x_domain,
        },
        &spec.x_label,
    );
    canvas.draw_y_axis(
        &Scale {
            domain: spec.y_domain,
            range: spec.y_domain,
        },
        &spec.y_label,
    );
    if !spec.title.is_empty() {
        canvas.draw_title(&spec.title);
    }
    canvas.draw_subtitle(&spec.subtitle);
    canvas.draw_caption(&spec.caption);
    Ok(canvas.render())
}

pub(crate) fn standalone_plot_html(svg: &str, title: &str) -> String {
    let title = if title.trim().is_empty() {
        "BioLang plot"
    } else {
        title
    };
    let escaped_title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>{escaped_title}</title><style>body{{margin:0;padding:1rem;font-family:system-ui,sans-serif}}figure{{margin:0;overflow:auto}}svg,canvas{{max-width:100%;height:auto}}button{{margin:0 0 .5rem .35rem}}</style></head><body><figure id="bl-figure" aria-labelledby="bl-caption"><figcaption id="bl-caption" style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)">{escaped_title}</figcaption><button type="button" id="bl-toggle" aria-controls="bl-svg bl-canvas" aria-pressed="false" disabled>Use canvas</button><button type="button" id="bl-download" aria-controls="bl-canvas" disabled>Download PNG</button>{svg}<canvas id="bl-canvas" hidden role="img" aria-label="{escaped_title} canvas fallback"></canvas></figure><script>(function(){{const f=document.getElementById('bl-figure'),s=f.querySelector('svg'),c=document.getElementById('bl-canvas'),t=document.getElementById('bl-toggle'),d=document.getElementById('bl-download');s.id='bl-svg';const v=s.viewBox.baseVal,w=v.width||+s.getAttribute('width')||800,h=v.height||+s.getAttribute('height')||600,scale=Math.min(devicePixelRatio||1,2);c.width=Math.round(w*scale);c.height=Math.round(h*scale);c.style.width=w+'px';const blob=new Blob([new XMLSerializer().serializeToString(s)],{{type:'image/svg+xml'}}),u=URL.createObjectURL(blob),i=new Image;i.onload=()=>{{const x=c.getContext('2d');x.setTransform(scale,0,0,scale,0,0);x.drawImage(i,0,0,w,h);URL.revokeObjectURL(u);t.disabled=false;d.disabled=false}};i.onerror=()=>URL.revokeObjectURL(u);i.src=u;t.onclick=()=>{{const show=c.hidden;c.hidden=!show;s.hidden=show;t.setAttribute('aria-pressed',String(show));t.textContent=show?'Use SVG':'Use canvas'}};d.onclick=()=>{{const a=document.createElement('a');a.download='biolang-plot.png';a.href=c.toDataURL('image/png');a.click()}}}})();</script></body></html>"#
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn svg_dimensions(svg: &str) -> Result<(f64, f64)> {
    let opening = svg.find('>').map(|index| &svg[..=index]).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() received malformed SVG",
            None,
        )
    })?;
    let attribute = |name: &str| -> Option<f64> {
        let pattern = regex::Regex::new(&format!(r#"\b{name}="([0-9]+(?:\.[0-9]+)?)""#)).ok()?;
        pattern
            .captures(opening)
            .and_then(|capture| capture.get(1))
            .and_then(|value| value.as_str().parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value > 0.0)
    };
    if let (Some(width), Some(height)) = (attribute("width"), attribute("height")) {
        return Ok((width, height));
    }
    let viewbox = regex::Regex::new(
        r#"\bviewBox="[-+0-9.eE]+\s+[-+0-9.eE]+\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)""#,
    )
    .unwrap();
    let Some(capture) = viewbox.captures(opening) else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() SVG needs numeric width/height or a viewBox",
            None,
        ));
    };
    let width = capture[1].parse::<f64>().unwrap_or(f64::NAN);
    let height = capture[2].parse::<f64>().unwrap_or(f64::NAN);
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() SVG viewBox must have positive dimensions",
            None,
        ));
    }
    Ok((width, height))
}

fn safe_nested_svg(svg: &str) -> Result<()> {
    let trimmed = svg.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if !trimmed.starts_with("<svg") || !trimmed.ends_with("</svg>") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() accepts SVG plot strings or PlotSpec records",
            None,
        ));
    }
    if lowered.contains("<script")
        || lowered.contains("<foreignobject")
        || lowered.contains("javascript:")
        || regex::Regex::new(r#"\son[a-z]+\s*="#)
            .unwrap()
            .is_match(&lowered)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() refuses active content inside SVG panels",
            None,
        ));
    }
    Ok(())
}

fn without_child_axis_title(svg: &str, axis: &str) -> String {
    let pattern = regex::Regex::new(&format!(
        r#"(?i)<text\b[^>]*\bdata-biolang-axis-title="{axis}"[^>]*>[^<]*</text>"#
    ))
    .unwrap();
    pattern.replace_all(svg, "").into_owned()
}

fn spreadsheet_panel_tag(mut index: usize) -> String {
    let mut tag = String::new();
    loop {
        tag.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    tag
}

fn is_plot_grid_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA)
            && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "plot-grid"))
}

fn plot_grid_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let Value::List(items) = value else {
        return Err(BioLangError::type_error(
            format!(
                "plot_grid() requires a List of plots, got {}",
                value.type_of()
            ),
            None,
        ));
    };
    if items.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() requires at least one plot",
            None,
        ));
    }
    let columns = get_opt_f64(opts, "columns", (items.len() as f64).sqrt().ceil()) as usize;
    if columns == 0 || columns > items.len().max(1) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() columns must be an integer between 1 and the panel count",
            None,
        ));
    }
    if get_opt_f64(opts, "columns", columns as f64).fract() != 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() columns must be an integer",
            None,
        ));
    }
    let rows = items.len().div_ceil(columns);
    let gap = get_opt_f64(opts, "gap", 18.0);
    let panel_width = get_opt_f64(opts, "panel_width", 420.0);
    let panel_height = get_opt_f64(opts, "panel_height", 330.0);
    let title = get_opt_str(opts, "title", "");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let shared_xlabel = get_opt_str(opts, "shared_xlabel", "");
    let shared_ylabel = get_opt_str(opts, "shared_ylabel", "");
    let header = if title.is_empty() {
        16.0
    } else if subtitle.is_empty() {
        46.0
    } else {
        64.0
    };
    let footer = 14.0
        + if shared_xlabel.is_empty() { 0.0 } else { 24.0 }
        + if caption.is_empty() { 0.0 } else { 18.0 };
    let legend_width = if opts.contains_key("legend") {
        140.0
    } else {
        0.0
    };
    let calculated_width = 20.0
        + columns as f64 * panel_width
        + columns.saturating_sub(1) as f64 * gap
        + legend_width
        + 20.0;
    let calculated_height =
        header + rows as f64 * panel_height + rows.saturating_sub(1) as f64 * gap + footer;
    let width = get_opt_f64(opts, "width", calculated_width);
    let height = get_opt_f64(opts, "height", calculated_height);
    if ![gap, panel_width, panel_height, width, height]
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() dimensions must be finite and positive",
            None,
        ));
    }
    let labels = match opts.get("panel_labels") {
        Some(Value::List(labels)) if labels.len() == items.len() => labels
            .iter()
            .map(|label| {
                label.as_str().map(str::to_string).ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "plot_grid() panel_labels must contain strings",
                        None,
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?,
        Some(Value::List(_)) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "plot_grid() panel_labels length must equal the panel count",
                None,
            ))
        }
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "plot_grid() panel_labels must be a List",
                None,
            ))
        }
        None => (0..items.len()).map(spreadsheet_panel_tag).collect(),
    };
    let mut panel_rows = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let mut svg = match item {
            Value::Str(svg) => svg.to_string(),
            Value::Record(_) => match builtin_render_plot(vec![item.clone()])? {
                Value::Str(svg) => svg.to_string(),
                other => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "plot_grid() PlotSpec rendered as {}, expected SVG",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            },
            other => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "plot_grid() panel {} is {}, expected SVG or PlotSpec",
                        index + 1,
                        other.type_of()
                    ),
                    None,
                ))
            }
        };
        safe_nested_svg(&svg)?;
        if !shared_xlabel.is_empty() {
            svg = without_child_axis_title(&svg, "x");
        }
        if !shared_ylabel.is_empty() {
            svg = without_child_axis_title(&svg, "y");
        }
        let (source_width, source_height) = svg_dimensions(&svg)?;
        let row = index / columns;
        let column = index % columns;
        panel_rows.push(vec![
            Value::Int(index as i64),
            Value::Int(row as i64),
            Value::Int(column as i64),
            Value::Str(labels[index].clone().into()),
            Value::Float(20.0 + column as f64 * (panel_width + gap)),
            Value::Float(header + row as f64 * (panel_height + gap)),
            Value::Float(panel_width),
            Value::Float(panel_height),
            Value::Float(source_width),
            Value::Float(source_height),
            Value::Str(svg.into()),
        ]);
    }
    let legend = match opts.get("legend") {
        None => Table::new(vec!["label".into(), "color".into()], Vec::new()),
        Some(Value::Table(table)) => {
            let label = table.col_index("label").ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "plot_grid() legend needs label and color columns",
                    None,
                )
            })?;
            let color = table
                .col_index("color")
                .or_else(|| table.col_index("colour"))
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "plot_grid() legend needs label and color columns",
                        None,
                    )
                })?;
            let mut rows = Vec::new();
            for row in &table.rows {
                let label = row[label].as_str().ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "plot_grid() legend labels must be strings",
                        None,
                    )
                })?;
                let color = row[color]
                    .as_str()
                    .filter(|color| valid_spec_colour(color))
                    .ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "plot_grid() legend colors must be #rrggbb",
                            None,
                        )
                    })?;
                rows.push(vec![Value::Str(label.into()), Value::Str(color.into())]);
            }
            Table::new(vec!["label".into(), "color".into()], rows)
        }
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "plot_grid() legend must be a Table with label and color columns",
                None,
            ))
        }
    };
    Ok(Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str("figure-composition".into())),
            ("plot".into(), Value::Str("plot-grid".into())),
            ("title".into(), Value::Str(title.into())),
            ("subtitle".into(), Value::Str(subtitle.into())),
            ("caption".into(), Value::Str(caption.into())),
            ("shared_xlabel".into(), Value::Str(shared_xlabel.into())),
            ("shared_ylabel".into(), Value::Str(shared_ylabel.into())),
            (
                "panels".into(),
                Value::Table(Table::new(
                    vec![
                        "panel_index".into(),
                        "row".into(),
                        "column".into(),
                        "tag".into(),
                        "x".into(),
                        "y".into(),
                        "width".into(),
                        "height".into(),
                        "source_width".into(),
                        "source_height".into(),
                        "svg".into(),
                    ],
                    panel_rows,
                )),
            ),
            ("legend".into(), Value::Table(legend)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("width".into(), Value::Float(width)),
                        ("height".into(), Value::Float(height)),
                        ("columns".into(), Value::Int(columns as i64)),
                        ("rows".into(), Value::Int(rows as i64)),
                        ("gap".into(), Value::Float(gap)),
                        ("panel_width".into(), Value::Float(panel_width)),
                        ("panel_height".into(), Value::Float(panel_height)),
                        ("header".into(), Value::Float(header)),
                        ("footer".into(), Value::Float(footer)),
                        ("legend_width".into(), Value::Float(legend_width)),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "publication").into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("panel_count".into(), Value::Int(items.len() as i64)),
                        ("layout".into(), Value::Str("equal_cells".into())),
                        ("child_svg_frozen".into(), Value::Bool(true)),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

fn render_plot_grid_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let Value::Record(map) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a plot-grid PlotSpec",
            None,
        ));
    };
    if !is_plot_grid_spec(value) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a biolang.plot.spec/v1 plot-grid Record",
            None,
        ));
    }
    let panels = match map.get("panels") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panels must be a Table",
                None,
            ))
        }
    };
    let legend = match map.get("legend") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid legend must be a Table",
                None,
            ))
        }
    };
    let options = match map.get("options") {
        Some(Value::Record(options)) => options,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid options must be a Record",
                None,
            ))
        }
    };
    for column in [
        "panel_index",
        "row",
        "column",
        "tag",
        "x",
        "y",
        "width",
        "height",
        "source_width",
        "source_height",
        "svg",
    ] {
        if panels.col_index(column).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() plot-grid panels are missing '{column}'"),
                None,
            ));
        }
    }
    let width = options
        .get("width")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid width is invalid",
                None,
            )
        })?;
    let height = options
        .get("height")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid height is invalid",
                None,
            )
        })?;
    let theme = PlotTheme::from_name(
        options
            .get("theme")
            .and_then(Value::as_str)
            .unwrap_or("publication"),
    );
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let index_column = panels.col_index("panel_index").unwrap();
    let x_column = panels.col_index("x").unwrap();
    let y_column = panels.col_index("y").unwrap();
    let width_column = panels.col_index("width").unwrap();
    let height_column = panels.col_index("height").unwrap();
    let source_width_column = panels.col_index("source_width").unwrap();
    let source_height_column = panels.col_index("source_height").unwrap();
    let tag_column = panels.col_index("tag").unwrap();
    let svg_column = panels.col_index("svg").unwrap();
    for (index, row) in panels.rows.iter().enumerate() {
        if row[index_column].as_float() != Some(index as f64) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panel indexes are inconsistent",
                None,
            ));
        }
        let values = [
            x_column,
            y_column,
            width_column,
            height_column,
            source_width_column,
            source_height_column,
        ]
        .map(|column| row[column].as_float().unwrap_or(f64::NAN));
        if values
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
            || values[0] + values[2] > width + 1e-8
            || values[1] + values[3] > height + 1e-8
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panel geometry is inconsistent",
                None,
            ));
        }
        let svg = row[svg_column].as_str().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panel SVG must be a string",
                None,
            )
        })?;
        safe_nested_svg(svg)?;
        let measured = svg_dimensions(svg)?;
        if (measured.0 - values[4]).abs() > 1e-8 || (measured.1 - values[5]).abs() > 1e-8 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid source dimensions were altered",
                None,
            ));
        }
        canvas.elements.push(format!(
            r#"<svg x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" viewBox="0 0 {:.2} {:.2}" preserveAspectRatio="xMidYMid meet" data-panel-index="{index}">{svg}</svg>"#,
            values[0], values[1], values[2], values[3], values[4], values[5]
        ));
        let tag = row[tag_column].as_str().unwrap_or("");
        canvas.add_text_styled(
            values[0] + 6.0,
            values[1] + 18.0,
            tag,
            "start",
            15.0,
            "bold",
            theme.text_colour,
        );
    }
    let title = map.get("title").and_then(Value::as_str).unwrap_or("");
    let subtitle = map.get("subtitle").and_then(Value::as_str).unwrap_or("");
    let caption = map.get("caption").and_then(Value::as_str).unwrap_or("");
    let shared_xlabel = map
        .get("shared_xlabel")
        .and_then(Value::as_str)
        .unwrap_or("");
    let shared_ylabel = map
        .get("shared_ylabel")
        .and_then(Value::as_str)
        .unwrap_or("");
    canvas.margin.left = 20.0;
    canvas.margin.right = 20.0;
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    if !shared_xlabel.is_empty() {
        canvas.add_text(
            width / 2.0,
            height - if caption.is_empty() { 8.0 } else { 22.0 },
            shared_xlabel,
            "middle",
            theme.axis_title_size,
        );
    }
    if !shared_ylabel.is_empty() {
        canvas.add_text_rotated(
            12.0,
            height / 2.0,
            shared_ylabel,
            -90.0,
            "middle",
            theme.axis_title_size,
        );
    }
    if !legend.rows.is_empty() {
        let label_column = legend.col_index("label").ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid legend is missing label",
                None,
            )
        })?;
        let color_column = legend.col_index("color").ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid legend is missing color",
                None,
            )
        })?;
        let legend_width = options
            .get("legend_width")
            .and_then(Value::as_float)
            .unwrap_or(140.0);
        let mut y = options
            .get("header")
            .and_then(Value::as_float)
            .unwrap_or(48.0)
            + 12.0;
        let x = width - legend_width + 12.0;
        for row in &legend.rows {
            let label = row[label_column].as_str().unwrap_or("");
            let color = row[color_column]
                .as_str()
                .filter(|color| valid_spec_colour(color))
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "render_plot() plot-grid legend color is invalid",
                        None,
                    )
                })?;
            canvas.add_rect(x, y - 9.0, 10.0, 10.0, color);
            canvas.add_text(x + 15.0, y, label, "start", theme.legend_size);
            y += 17.0;
        }
    }
    canvas.set_accessible_description(format!(
        "Multi-panel BioLang figure containing {} panels and {} shared legend entries.",
        panels.rows.len(),
        legend.rows.len()
    ));
    let svg = canvas.render();
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    match format.as_str() {
        "spec" | "data" => Ok(value.clone()),
        "svg" | "raw" => Ok(Value::Str(svg.into())),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, title).into())),
        #[cfg(feature = "native")]
        "ascii" => render_svg_terminal(&svg, 100, 32, TerminalPlotStyle::Ascii)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => render_svg_terminal(&svg, 100, 32, TerminalPlotStyle::Braille)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal plot-grid output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() unknown plot-grid format '{format}'"),
            None,
        )),
    }
}

fn builtin_plot_grid(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let spec = plot_grid_spec_value(&args[0], &opts)?;
    render_plot_grid_spec_value(&spec, &opts)
}

fn render_plot_spec_value(
    spec: &CartesianPlotSpec,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let format = get_opt_str(opts, "format", "svg").to_ascii_lowercase();
    if format == "spec" || format == "data" {
        return Ok(plot_spec_to_value(spec));
    }
    // Counted across every series: they share the plot area, so they share the
    // one raster, and the threshold is about how many marks land in it.
    let point_count = spec
        .series
        .iter()
        .map(|series| series.points.len())
        .sum::<usize>();
    let raster = raster_choice(opts, "plot", point_count)?;
    let svg = render_cartesian_plot_spec(spec, raster)?;
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg.into())),
        // The terminal preview rasterises through resvg, which the browser
        // build deliberately leaves out. Guard the arms rather than the
        // function so a WASM caller asking for one gets a real message.
        #[cfg(feature = "native")]
        "ascii" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Ascii)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Braille)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() format '{format}' needs the native build; this runtime can emit svg/html/spec"
            ),
            None,
        )),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, &spec.title).into())),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_plot_spec(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let table = require_table(&args[0], "plot_spec")?;
    Ok(plot_spec_to_value(&build_cartesian_plot_spec(
        table,
        &opts,
        "plot_spec",
    )?))
}

fn builtin_mosaic_data(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    crate::mosaic_plot::specification(require_table(&args[0], "mosaic_data")?, &opts)
}

fn builtin_mosaic_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification =
        crate::mosaic_plot::specification(require_table(&args[0], "mosaic_plot")?, &opts)?;
    if matches!(
        get_opt_str(&opts, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        Ok(specification)
    } else {
        crate::mosaic_plot::render(&specification, &opts)
    }
}

fn builtin_render_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let map = match &args[0] {
        Value::Record(map) => map,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "render_plot() requires plot specification Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    if !matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() requires schema '{PLOT_SPEC_SCHEMA}'"),
            None,
        ));
    }
    let kind = map.get("kind").and_then(Value::as_str).unwrap_or("");
    let plot = map.get("plot").and_then(Value::as_str).unwrap_or("");
    match (kind, plot) {
        (_, "plot-grid") => render_plot_grid_spec_value(&args[0], &opts),
        ("manhattan", _) => crate::bio_plots::render_manhattan_plot_spec_value(&args[0], &opts),
        ("genetic_qq", _) => crate::bio_plots::render_genetic_qq_plot_spec_value(&args[0], &opts),
        ("rainfall", _) => crate::bio_plots::render_rainfall_plot_spec_value(&args[0], &opts),
        ("ideogram", _) => crate::bio_plots::render_ideogram_plot_spec_value(&args[0], &opts),
        ("cnv", _) => crate::bio_plots::render_cnv_plot_spec_value(&args[0], &opts),
        ("coverage_track", _) => {
            crate::bio_plots::render_coverage_track_plot_spec_value(&args[0], &opts)
        }
        ("genome_track", _) => {
            crate::bio_plots::render_genome_track_plot_spec_value(&args[0], &opts)
        }
        ("lollipop", _) => crate::bio_plots::render_lollipop_plot_spec_value(&args[0], &opts),
        ("sashimi", _) => crate::bio_plots::render_sashimi_plot_spec_value(&args[0], &opts),
        (_, "circos") => crate::bio_plots::render_circos_plot_spec_value(&args[0], &opts),
        ("survival", _) => crate::bio_plots::render_survival_plot_spec_value(&args[0], &opts),
        ("forest", _) => crate::bio_plots::render_forest_plot_spec_value(&args[0], &opts),
        ("roc", _) => crate::bio_plots::render_roc_plot_spec_value(&args[0], &opts),
        ("heatmap", "clustered_heatmap") => {
            crate::bio_plots::render_clustered_heatmap_spec_value(&args[0], &opts)
        }
        ("heatmap", _) => render_heatmap_plot_spec_value(&args[0], &opts),
        ("mosaic", _) => crate::mosaic_plot::render(&args[0], &opts),
        ("violin", _) => crate::bio_plots::render_violin_plot_spec_value(&args[0], &opts),
        ("dot_plot", _) => crate::bio_plots::render_dot_plot_spec_value(&args[0], &opts),
        ("embedding", _) => crate::bio_plots::render_embedding_plot_spec_value(&args[0], &opts),
        ("pca", _) => crate::bio_plots::render_pca_plot_spec_value(&args[0], &opts),
        ("differential_expression", _) => render_differential_plot_spec_value(&args[0], &opts),
        ("scatter" | "line" | "errorbar" | "confidence", _) => {
            let spec = plot_spec_from_value(&args[0])?;
            render_plot_spec_value(&spec, &opts)
        }
        ("", _) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() specification field 'kind' must be Str",
            None,
        )),
        (unknown, _) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() unknown plot kind '{unknown}'"),
            None,
        )),
    }
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

    // These plot families now have one renderer-neutral specification. The
    // SVG, terminal preview and standalone HTML/canvas fallback all originate
    // from this object; none of those display paths recomputes statistics.
    if matches!(
        plot_type.to_ascii_lowercase().as_str(),
        "scatter" | "line" | "errorbar" | "confidence"
    ) {
        let spec = build_cartesian_plot_spec(table, &opts, "plot")?;
        return render_plot_spec_value(&spec, &opts);
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
    if plot_type == "box" {
        // Every numeric column becomes a group. The former scale came only
        // from the default y column, so a wider first or later column could be
        // clipped even though its geometry was still drawn.
        (y_min, y_max) = table.columns.iter().try_fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(low, high), column| {
                let values = extract_table_col(table, column)?;
                let (column_low, column_high) = col_range(&values);
                Ok::<_, BioLangError>((low.min(column_low), high.max(column_high)))
            },
        )?;
    }
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
    // No x_scale here: bar positions come from the category layout below and
    // box positions from the column index, so only the vertical scale is
    // shared. The scatter and line arms that needed one now build their own
    // spec and return above.
    let y_scale = Scale {
        domain: (y_min, y_max),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    match plot_type.as_str() {
        // scatter, line, errorbar and confidence never reach this match: they
        // are built as a CartesianPlotSpec and returned above. Only the
        // families that have no spec yet are rendered directly here.
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
                if !vals.iter().any(|value| value.is_finite()) {
                    continue;
                }
                // The renderer consumes the same inspectable geometry exposed
                // by boxplot_data(), including its type-7 quartiles and Tukey
                // whisker coefficient. No summary statistic is recalculated in
                // screen coordinates.
                let geometry = box_geometry(col, &vals, "type7", 1.5);

                let bx = canvas.margin.left
                    + (ci as f64 + 0.2) * canvas.plot_width() / table.num_cols() as f64;
                let bw = canvas.plot_width() / table.num_cols() as f64 * 0.6;

                canvas.add_rect(
                    bx,
                    y_scale.map(geometry.q3),
                    bw,
                    (y_scale.map(geometry.q1) - y_scale.map(geometry.q3)).abs(),
                    PALETTE[ci % PALETTE.len()],
                );
                canvas.add_line(
                    bx,
                    y_scale.map(geometry.median),
                    bx + bw,
                    y_scale.map(geometry.median),
                    "#333",
                    2.0,
                );
                canvas.add_line(
                    bx + bw / 2.0,
                    y_scale.map(geometry.q3),
                    bx + bw / 2.0,
                    y_scale.map(geometry.whisker_high),
                    "#333",
                    1.0,
                );
                canvas.add_line(
                    bx + bw / 2.0,
                    y_scale.map(geometry.q1),
                    bx + bw / 2.0,
                    y_scale.map(geometry.whisker_low),
                    "#333",
                    1.0,
                );
                for (_, value) in &geometry.outliers {
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
fn cluster_rows(row_data: &mut Vec<Vec<f64>>, row_labels: &mut Vec<String>) -> Vec<usize> {
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
    indices
}

fn render_heatmap_geometry_svg(
    row_data: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    scale_min: f64,
    scale_max: f64,
    use_diverging: bool,
    scheme_explicit: bool,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let title = get_opt_str(opts, "title", "Heatmap").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let legend_title = get_opt_str(opts, "legend_title", "value").to_string();
    let na_colour = get_opt_str(opts, "na_color", "#cccccc").to_string();
    let theme = plot_theme(opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let scheme = get_opt_str(opts, "colors", "viridis").to_string();
    let show_values = opts
        .get("show_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let do_cluster = opts
        .get("cluster")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let nrows = row_data.len();
    let ncols = row_data.first().map(Vec::len).unwrap_or(0);
    if nrows == 0 || ncols == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() heatmap specification is empty",
            None,
        ));
    }
    let cell_colour = |t: f64| {
        if publication_theme && !scheme_explicit {
            if use_diverging {
                publication_diverging_color(t)
            } else {
                publication_sequential_color(t)
            }
        } else {
            heatmap_color(t, &scheme)
        }
    };
    let max_row_label_len = row_labels.iter().map(String::len).max().unwrap_or(0);
    let left_margin = 40.0 + (max_row_label_len as f64 * 7.0).min(120.0);
    let legend_width = 60.0;
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    if theme.is_adaptive() {
        let widest_row = row_labels
            .iter()
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let widest_col = col_labels
            .iter()
            .take(ncols)
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let legend_label = [scale_min, (scale_min + scale_max) / 2.0, scale_max]
            .iter()
            .map(|value| estimate_text_width(&format!("{value:.2}"), theme.legend_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_row + 12.0).clamp(48.0, width * 0.31);
        canvas.margin.right = (42.0
            + legend_label.max(estimate_text_width(&legend_title, theme.legend_size)))
        .clamp(76.0, width * 0.31);
        canvas.margin.top = if title.is_empty() {
            20.0
        } else if subtitle.is_empty() {
            48.0
        } else {
            66.0
        };
        canvas.margin.bottom = (widest_col * 0.72 + 18.0).clamp(48.0, height * 0.28)
            + if caption.is_empty() { 0.0 } else { 18.0 };
    } else {
        canvas.margin.left = left_margin;
        canvas.margin.bottom = 70.0;
        canvas.margin.right = 20.0 + legend_width;
        canvas.margin.top = if title.is_empty() { 20.0 } else { 45.0 };
    }
    let plot_w = canvas.plot_width();
    let plot_h = canvas.plot_height();
    let cell_w = plot_w / ncols as f64;
    let cell_h = plot_h / nrows as f64;
    for (ri, row) in row_data.iter().enumerate() {
        for (ci, &value) in row.iter().enumerate() {
            let t = if (scale_max - scale_min).abs() < f64::EPSILON {
                0.5
            } else {
                (value - scale_min) / (scale_max - scale_min)
            };
            let colour = if value.is_finite() {
                cell_colour(t)
            } else {
                na_colour.clone()
            };
            let x = canvas.margin.left + ci as f64 * cell_w;
            let y = canvas.margin.top + ri as f64 * cell_h;
            canvas.add_rect(x, y, cell_w, cell_h, &colour);
            if !theme.is_adaptive() || cell_w.min(cell_h) >= 4.0 {
                canvas.elements.push(format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.5\" />",
                    x,
                    y,
                    cell_w,
                    cell_h,
                    if theme.is_adaptive() { theme.grid_colour } else { "#eee" }
                ));
            }
            if show_values && value.is_finite() {
                let text_colour = heatmap_text_color(t, &scheme);
                let label = if value.abs() >= 100.0 || value == 0.0 {
                    format!("{value:.0}")
                } else if value.abs() >= 1.0 {
                    format!("{value:.1}")
                } else {
                    format!("{value:.2}")
                };
                let font_size = (cell_w.min(cell_h) * 0.35).clamp(7.0, 14.0);
                canvas.elements.push(format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="central" font-size="{:.1}" font-family="{}" fill="{}">{}</text>"#,
                    x + cell_w / 2.0,
                    y + cell_h / 2.0,
                    font_size,
                    theme.font_family,
                    text_colour,
                    label.replace('&', "&amp;").replace('<', "&lt;")
                ));
            }
        }
    }
    let col_step = if theme.is_adaptive() {
        (10.0 / cell_w.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ci, label) in col_labels.iter().enumerate().step_by(col_step) {
        if ci < ncols {
            canvas.add_text_rotated(
                canvas.margin.left + (ci as f64 + 0.5) * cell_w,
                canvas.margin.top + plot_h + 10.0,
                label,
                45.0,
                "start",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }
    let row_step = if theme.is_adaptive() {
        (10.0 / cell_h.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ri, label) in row_labels.iter().enumerate().step_by(row_step) {
        if ri < nrows {
            canvas.add_text(
                canvas.margin.left - 6.0,
                canvas.margin.top + (ri as f64 + 0.5) * cell_h + 4.0,
                label,
                "end",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }
    let legend_x = canvas.margin.left + plot_w + 15.0;
    let legend_top = canvas.margin.top;
    let legend_h = plot_h.min(200.0);
    let legend_bar_w = 15.0;
    let legend_steps = 50usize;
    let step_h = legend_h / legend_steps as f64;
    if theme.is_adaptive() && !legend_title.is_empty() {
        canvas.add_text(
            legend_x,
            legend_top - 8.0,
            &legend_title,
            "start",
            theme.legend_size,
        );
    }
    for i in 0..legend_steps {
        let t = 1.0 - i as f64 / (legend_steps - 1) as f64;
        canvas.elements.push(format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" />"#,
            legend_x,
            legend_top + i as f64 * step_h,
            legend_bar_w,
            step_h + 0.5,
            cell_colour(t)
        ));
    }
    canvas.elements.push(format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#333\" stroke-width=\"0.5\" />",
        legend_x, legend_top, legend_bar_w, legend_h
    ));
    let label_x = legend_x + legend_bar_w + 5.0;
    canvas.add_text(
        label_x,
        legend_top + 4.0,
        &format!("{scale_max:.2}"),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h / 2.0 + 3.0,
        &format!("{:.2}", (scale_min + scale_max) / 2.0),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h + 3.0,
        &format!("{scale_min:.2}"),
        "start",
        9.0,
    );
    canvas.set_accessible_description(format!(
        "Heatmap with {nrows} rows and {ncols} columns. Rows are {}.",
        if do_cluster {
            "sorted by their mean value"
        } else {
            "shown in input order"
        }
    ));
    if !title.is_empty() {
        canvas.draw_title(&title);
    }
    if theme.is_adaptive() {
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    }
    Ok(canvas.render())
}

fn heatmap_plot_spec_value(
    row_data: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    row_order: &[usize],
    value_min: f64,
    value_max: f64,
    scale_min: f64,
    scale_max: f64,
    use_diverging: bool,
    scheme_explicit: bool,
    opts: &HashMap<String, Value>,
) -> Value {
    let cells = row_data
        .iter()
        .enumerate()
        .flat_map(|(display_row, row)| {
            row.iter().enumerate().map(move |(display_col, &value)| {
                vec![
                    Value::Int(display_row as i64),
                    Value::Int(row_order[display_row] as i64),
                    Value::Int(display_col as i64),
                    Value::Int(display_col as i64),
                    Value::Float(value),
                ]
            })
        })
        .collect();
    let row_rows = row_labels
        .iter()
        .enumerate()
        .map(|(display_row, label)| {
            vec![
                Value::Int(display_row as i64),
                Value::Int(row_order[display_row] as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let col_rows = col_labels
        .iter()
        .enumerate()
        .map(|(display_col, label)| {
            vec![
                Value::Int(display_col as i64),
                Value::Int(display_col as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let non_finite = row_data
        .iter()
        .flat_map(|row| row.iter())
        .filter(|value| !value.is_finite())
        .count();
    let options = HashMap::from([
        ("plot".into(), Value::Str("heatmap".into())),
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", "Heatmap").into()),
        ),
        (
            "subtitle".into(),
            Value::Str(get_opt_str(opts, "subtitle", "").into()),
        ),
        (
            "caption".into(),
            Value::Str(get_opt_str(opts, "caption", "").into()),
        ),
        (
            "legend_title".into(),
            Value::Str(get_opt_str(opts, "legend_title", "value").into()),
        ),
        (
            "na_color".into(),
            Value::Str(get_opt_str(opts, "na_color", "#cccccc").into()),
        ),
        (
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        (
            "colors".into(),
            Value::Str(get_opt_str(opts, "colors", "viridis").into()),
        ),
        ("colors_explicit".into(), Value::Bool(scheme_explicit)),
        (
            "show_values".into(),
            Value::Bool(
                opts.get("show_values")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
        (
            "cluster".into(),
            Value::Bool(
                opts.get("cluster")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
        ("value_min".into(), Value::Float(value_min)),
        ("value_max".into(), Value::Float(value_max)),
        (
            "center".into(),
            opts.get("center").cloned().unwrap_or(Value::Nil),
        ),
        ("scale_min".into(), Value::Float(scale_min)),
        ("scale_max".into(), Value::Float(scale_max)),
        ("diverging".into(), Value::Bool(use_diverging)),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 800.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 600.0)),
        ),
    ]);
    Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str("heatmap".into())),
            ("plot".into(), Value::Str("heatmap".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "Heatmap").into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "display_row",
                        "source_row",
                        "display_col",
                        "source_col",
                        "value",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    cells,
                )),
            ),
            (
                "rows".into(),
                Value::Table(Table::new(
                    ["display_row", "source_row", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    row_rows,
                )),
            ),
            (
                "columns".into(),
                Value::Table(Table::new(
                    ["display_col", "source_col", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    col_rows,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("heatmap".into())),
                        ("input_rows".into(), Value::Int(row_data.len() as i64)),
                        (
                            "input_columns".into(),
                            Value::Int(row_data.first().map(Vec::len).unwrap_or(0) as i64),
                        ),
                        ("non_finite_cells".into(), Value::Int(non_finite as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "warnings".into(),
                Value::List(
                    if non_finite == 0 {
                        Vec::new()
                    } else {
                        vec![Value::Str(format!(
                            "{non_finite} heatmap cells are non-finite and use na_color"
                        ))]
                    }
                    .into(),
                ),
            ),
        ])
        .into(),
    )
}

fn is_heatmap_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "heatmap")
                && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "heatmap")
    )
}

fn render_heatmap_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_heatmap_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 heatmap Record",
                None,
            ))
        }
    };
    let table_field = |name: &str| -> Result<&Table> {
        match map.get(name) {
            Some(Value::Table(table)) => Ok(table),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() heatmap specification field '{name}' must be Table"),
                None,
            )),
        }
    };
    let cells = table_field("data")?;
    let rows = table_field("rows")?;
    let columns = table_field("columns")?;
    for required in [
        "display_row",
        "source_row",
        "display_col",
        "source_col",
        "value",
    ] {
        if cells.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() heatmap data is missing '{required}'"),
                None,
            ));
        }
    }
    for (table, axis, required) in [
        (rows, "row", ["display_row", "source_row", "label"]),
        (columns, "column", ["display_col", "source_col", "label"]),
    ] {
        for field in required {
            if table.col_index(field).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() heatmap {axis} metadata is missing '{field}'"),
                    None,
                ));
            }
        }
    }
    if rows.num_rows() == 0 || columns.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() heatmap specification is empty",
            None,
        ));
    }
    let labels = |table: &Table| {
        let index = table.col_index("label").unwrap();
        table
            .rows
            .iter()
            .map(|row| format!("{}", row[index]))
            .collect::<Vec<_>>()
    };
    let row_labels = labels(rows);
    let col_labels = labels(columns);
    let expected = rows.num_rows() * columns.num_rows();
    if cells.num_rows() != expected {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() heatmap data must contain one cell per displayed row and column",
            None,
        ));
    }
    let ri = cells.col_index("display_row").unwrap();
    let ci = cells.col_index("display_col").unwrap();
    let vi = cells.col_index("value").unwrap();
    let mut row_data = vec![vec![f64::NAN; columns.num_rows()]; rows.num_rows()];
    for (expected_index, row) in cells.rows.iter().enumerate() {
        let display_row = row[ri]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() heatmap display_row must be numeric",
                    None,
                )
            })?;
        let display_col = row[ci]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() heatmap display_col must be numeric",
                    None,
                )
            })?;
        if display_row >= rows.num_rows()
            || display_col >= columns.num_rows()
            || expected_index != display_row * columns.num_rows() + display_col
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() heatmap cells must be complete and ordered by display row and column",
                None,
            ));
        }
        row_data[display_row][display_col] = row[vi].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() heatmap cell values must be numeric",
                None,
            )
        })?;
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() heatmap specification field 'options' must be Record",
                None,
            ))
        }
    };
    let number = |name: &str| -> Result<f64> {
        options.get(name).and_then(Value::as_float).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() heatmap options are missing numeric '{name}'"),
                None,
            )
        })
    };
    let scale_min = number("scale_min")?;
    let scale_max = number("scale_max")?;
    let use_diverging = options.get("diverging").is_some_and(Value::is_truthy);
    let scheme_explicit = options.get("colors_explicit").is_some_and(Value::is_truthy);
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in ["width", "height"] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    let svg = render_heatmap_geometry_svg(
        &row_data,
        &row_labels,
        &col_labels,
        scale_min,
        scale_max,
        use_diverging,
        scheme_explicit,
        &options,
    )?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Heatmap");
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Ascii)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Braille)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal heatmap output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown heatmap format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_heatmap(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Heatmap").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let legend_title = get_opt_str(&opts, "legend_title", "value").to_string();
    let na_colour = get_opt_str(&opts, "na_color", "#cccccc").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let scheme_explicit = opts.contains_key("colors");
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
    let row_order = if do_cluster {
        cluster_rows(&mut row_data, &mut row_labels)
    } else {
        (0..nrows).collect()
    };

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
    let requested_centre = opts.get("center").and_then(Value::as_float);
    let use_diverging = publication_theme
        && !scheme_explicit
        && (requested_centre.is_some() || (vmin < 0.0 && vmax > 0.0));
    let (scale_min, scale_max) = if use_diverging {
        let centre = requested_centre.unwrap_or(0.0);
        let radius = (vmin - centre)
            .abs()
            .max((vmax - centre).abs())
            .max(f64::EPSILON);
        (centre - radius, centre + radius)
    } else {
        (vmin, vmax)
    };

    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        if row_data.iter().any(|row| row.len() != ncols) || col_labels.len() < ncols {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "heatmap() inspectable output requires rectangular data and one label per column",
                None,
            ));
        }
        let spec = heatmap_plot_spec_value(
            &row_data,
            &row_labels,
            &col_labels[..ncols],
            &row_order,
            vmin,
            vmax,
            scale_min,
            scale_max,
            use_diverging,
            scheme_explicit,
            &opts,
        );
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_heatmap_plot_spec_value(&spec, &opts);
    }
    if matches!(format.as_str(), "svg" | "raw") {
        return render_heatmap_geometry_svg(
            &row_data,
            &row_labels,
            &col_labels,
            scale_min,
            scale_max,
            use_diverging,
            scheme_explicit,
            &opts,
        )
        .map(Value::Str);
    }
    let cell_colour = |t: f64| {
        if publication_theme && !scheme_explicit {
            if use_diverging {
                publication_diverging_color(t)
            } else {
                publication_sequential_color(t)
            }
        } else {
            heatmap_color(t, &scheme)
        }
    };

    // Compute margins based on label lengths
    let max_row_label_len = row_labels.iter().map(|s| s.len()).max().unwrap_or(0);
    let left_margin = 40.0 + (max_row_label_len as f64 * 7.0).min(120.0);
    let legend_width = 60.0;

    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    if theme.is_adaptive() {
        let widest_row = row_labels
            .iter()
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let widest_col = col_labels
            .iter()
            .take(ncols)
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let legend_label = [scale_min, (scale_min + scale_max) / 2.0, scale_max]
            .iter()
            .map(|value| estimate_text_width(&format!("{value:.2}"), theme.legend_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_row + 12.0).clamp(48.0, width * 0.31);
        canvas.margin.right = (42.0
            + legend_label.max(estimate_text_width(&legend_title, theme.legend_size)))
        .clamp(76.0, width * 0.31);
        canvas.margin.top = if title.is_empty() {
            20.0
        } else if subtitle.is_empty() {
            48.0
        } else {
            66.0
        };
        canvas.margin.bottom = (widest_col * 0.72 + 18.0).clamp(48.0, height * 0.28)
            + if caption.is_empty() { 0.0 } else { 18.0 };
    } else {
        canvas.margin.left = left_margin;
        canvas.margin.bottom = 70.0;
        canvas.margin.right = 20.0 + legend_width;
        canvas.margin.top = if title.is_empty() { 20.0 } else { 45.0 };
    }

    let plot_w = canvas.plot_width();
    let plot_h = canvas.plot_height();
    let cell_w = plot_w / ncols as f64;
    let cell_h = plot_h / nrows as f64;

    // Draw cells
    for (ri, row) in row_data.iter().enumerate() {
        for (ci, &v) in row.iter().enumerate() {
            let t = if (scale_max - scale_min).abs() < f64::EPSILON {
                0.5
            } else {
                (v - scale_min) / (scale_max - scale_min)
            };
            let color = if v.is_finite() {
                cell_colour(t)
            } else {
                na_colour.clone()
            };
            let x = canvas.margin.left + ci as f64 * cell_w;
            let y = canvas.margin.top + ri as f64 * cell_h;
            canvas.add_rect(x, y, cell_w, cell_h, &color);

            // Cell border for visual separation
            if !theme.is_adaptive() || cell_w.min(cell_h) >= 4.0 {
                canvas.elements.push(format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.5\" />",
                    x,
                    y,
                    cell_w,
                    cell_h,
                    if theme.is_adaptive() { theme.grid_colour } else { "#eee" }
                ));
            }

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
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="central" font-size="{:.1}" font-family="{}" fill="{}">{}</text>"#,
                    x + cell_w / 2.0, y + cell_h / 2.0, font_size,
                    theme.font_family, txt_color,
                    label.replace('&', "&amp;").replace('<', "&lt;")
                ));
            }
        }
    }

    // Column labels (rotated at bottom)
    let col_step = if theme.is_adaptive() {
        (10.0 / cell_w.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ci, col) in col_labels.iter().enumerate().step_by(col_step) {
        if ci < ncols {
            let x = canvas.margin.left + (ci as f64 + 0.5) * cell_w;
            let y = canvas.margin.top + plot_h + 10.0;
            canvas.add_text_rotated(
                x,
                y,
                col,
                45.0,
                "start",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }

    // Row labels (on the left)
    let row_step = if theme.is_adaptive() {
        (10.0 / cell_h.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ri, label) in row_labels.iter().enumerate().step_by(row_step) {
        if ri < nrows {
            let y = canvas.margin.top + (ri as f64 + 0.5) * cell_h + 4.0;
            canvas.add_text(
                canvas.margin.left - 6.0,
                y,
                label,
                "end",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }

    // Color legend / scale bar (right side)
    let legend_x = canvas.margin.left + plot_w + 15.0;
    let legend_top = canvas.margin.top;
    let legend_h = plot_h.min(200.0);
    let legend_bar_w = 15.0;
    let legend_steps = 50usize;
    let step_h = legend_h / legend_steps as f64;
    if theme.is_adaptive() && !legend_title.is_empty() {
        canvas.add_text(
            legend_x,
            legend_top - 8.0,
            &legend_title,
            "start",
            theme.legend_size,
        );
    }
    for i in 0..legend_steps {
        let t = 1.0 - (i as f64 / (legend_steps - 1) as f64); // top = max
        let color = cell_colour(t);
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
        &format!("{scale_max:.2}"),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h / 2.0 + 3.0,
        &format!("{:.2}", (scale_min + scale_max) / 2.0),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h + 3.0,
        &format!("{scale_min:.2}"),
        "start",
        9.0,
    );

    // Title
    canvas.set_accessible_description(format!(
        "Heatmap with {nrows} rows and {ncols} columns. Rows are {}.",
        if do_cluster {
            "sorted by their mean value"
        } else {
            "shown in input order"
        }
    ));
    if !title.is_empty() {
        canvas.draw_title(&title);
    }
    if theme.is_adaptive() {
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    }

    Ok(Value::Str(canvas.render()))
}

const HISTOGRAM_SCHEMA: &str = "biolang.plot.geometry/v1";
const MAX_HISTOGRAM_BINS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HistogramClosure {
    Left,
    Right,
}

impl HistogramClosure {
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HistogramGeometry {
    pub(crate) edges: Vec<f64>,
    pub(crate) counts: Vec<usize>,
    method: String,
    closure: HistogramClosure,
    include_lowest: bool,
    n_total: usize,
    n_finite: usize,
    n_included: usize,
    dropped_invalid: usize,
    dropped_non_finite: usize,
    dropped_outside: usize,
}

fn histogram_bool_option(opts: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    match opts.get(key) {
        Some(Value::Bool(value)) => *value,
        _ => default,
    }
}

fn histogram_values(value: &Value, who: &str) -> Result<(Vec<f64>, usize, usize, usize)> {
    let items = match value {
        Value::List(items) => items,
        _ => {
            return Err(BioLangError::type_error(
                format!("{who}() requires List of numbers"),
                None,
            ))
        }
    };

    let mut values = Vec::with_capacity(items.len());
    let mut invalid = 0usize;
    let mut non_finite = 0usize;
    for item in items.iter() {
        let parsed = match item {
            Value::Int(value) => Some(*value as f64),
            Value::Float(value) => Some(*value),
            Value::Str(value) => value.parse::<f64>().ok(),
            _ => None,
        };
        match parsed {
            Some(value) if value.is_finite() => values.push(value),
            Some(_) => non_finite += 1,
            None => invalid += 1,
        }
    }
    if values.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{who}() received no finite numeric values - check the input and missing-value encoding"
            ),
            None,
        ));
    }
    Ok((values, items.len(), invalid, non_finite))
}

fn histogram_bin_count(value: f64, option: &str) -> Result<usize> {
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("histogram() option '{option}' must be a positive whole number"),
            None,
        ));
    }
    let bins = value as usize;
    if bins > MAX_HISTOGRAM_BINS {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("histogram() requested {bins} bins; the safety limit is {MAX_HISTOGRAM_BINS}"),
            None,
        ));
    }
    Ok(bins)
}

fn histogram_quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted[lower] + fraction * (sorted[upper] - sorted[lower])
}

fn histogram_automatic_bin_count(values: &[f64], method: &str) -> usize {
    let n = values.len().max(1) as f64;
    let sturges = (n.log2() + 1.0).ceil().max(1.0) as usize;
    let (lo, hi) = col_range(values);
    let span = hi - lo;
    if span <= f64::EPSILON {
        return 1;
    }

    let width = match method {
        "freedman-diaconis" => {
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let iqr = histogram_quantile(&sorted, 0.75) - histogram_quantile(&sorted, 0.25);
            2.0 * iqr / n.cbrt()
        }
        "scott" => {
            let mean = values.iter().sum::<f64>() / n;
            let variance = if values.len() > 1 {
                values
                    .iter()
                    .map(|value| (value - mean).powi(2))
                    .sum::<f64>()
                    / (n - 1.0)
            } else {
                0.0
            };
            3.5 * variance.sqrt() / n.cbrt()
        }
        _ => return sturges,
    };
    if !width.is_finite() || width <= f64::EPSILON {
        sturges
    } else {
        ((span / width).ceil() as usize).clamp(1, MAX_HISTOGRAM_BINS)
    }
}

fn histogram_equal_edges(values: &[f64], bins: usize) -> Vec<f64> {
    let (mut lo, mut hi) = col_range(values);
    if (hi - lo).abs() < f64::EPSILON {
        let padding = (lo.abs() * 0.01).max(0.5);
        lo -= padding;
        hi += padding;
    }
    let width = (hi - lo) / bins as f64;
    (0..=bins)
        .map(|index| {
            if index == bins {
                hi
            } else {
                lo + index as f64 * width
            }
        })
        .collect()
}

/// ggplot2's `bin_breaks_bins()`, which is not an equal split of the range.
///
/// `bins = n` in ggplot2 uses a width of `range / (n - 1)` and a boundary of
/// half a width, so the first bin is centred on the minimum and the outer
/// edges sit half a bin beyond the data. Cutting `[min, max]` into `n` equal
/// parts instead — the matplotlib and `hist(breaks = n)` reading — gives
/// different bar widths and different counts from the same `bins` value.
pub(crate) fn histogram_ggplot_edges(values: &[f64], bins: usize) -> Vec<f64> {
    let (mut lo, mut hi) = col_range(values);
    if (hi - lo).abs() < f64::EPSILON {
        let padding = (lo.abs() * 0.01).max(0.5);
        lo -= padding;
        hi += padding;
    }
    if bins < 2 {
        return vec![lo, hi];
    }
    let width = (hi - lo) / (bins - 1) as f64;
    let boundary = width / 2.0;
    // find_origin(): the boundary-aligned edge at or below the minimum.
    let origin = boundary + ((lo - boundary) / width).floor() * width;
    // ggplot2 nudges the upper limit so an exact multiple does not add a bin.
    let limit = hi + (1.0 - 1e-8) * width;
    let breaks = (((limit - origin) / width).floor() as i64 + 1).max(2) as usize;
    (0..breaks)
        .map(|index| origin + index as f64 * width)
        .collect()
}

fn histogram_explicit_edges(items: &[Value]) -> Result<Vec<f64>> {
    let mut edges = Vec::with_capacity(items.len());
    for item in items {
        let edge = match item {
            Value::Int(value) => *value as f64,
            Value::Float(value) => *value,
            Value::Str(value) => value.parse::<f64>().map_err(|_| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "histogram() explicit breaks must all be numeric",
                    None,
                )
            })?,
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "histogram() explicit breaks must all be numeric",
                    None,
                ))
            }
        };
        if !edge.is_finite() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "histogram() explicit breaks must be finite",
                None,
            ));
        }
        edges.push(edge);
    }
    if edges.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "histogram() explicit breaks require at least two edges",
            None,
        ));
    }
    if edges.len() - 1 > MAX_HISTOGRAM_BINS {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("histogram() explicit breaks exceed the {MAX_HISTOGRAM_BINS}-bin limit"),
            None,
        ));
    }
    if edges.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "histogram() explicit breaks must be strictly increasing",
            None,
        ));
    }
    Ok(edges)
}

pub(crate) fn histogram_geometry(args: &[Value], who: &str) -> Result<HistogramGeometry> {
    let opts = parse_options(args);
    let (values, n_total, dropped_invalid, dropped_non_finite) = histogram_values(&args[0], who)?;
    let closure = match opts.get("closed").and_then(Value::as_str) {
        Some("left") => HistogramClosure::Left,
        Some("right") => HistogramClosure::Right,
        Some(other) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("histogram() option 'closed' must be 'left' or 'right', got '{other}'"),
                None,
            ))
        }
        None if histogram_bool_option(&opts, "right", false) => HistogramClosure::Right,
        None => HistogramClosure::Left,
    };
    let include_lowest = histogram_bool_option(&opts, "include_lowest", true);

    let (edges, method) = match opts.get("breaks") {
        Some(Value::List(items)) => (histogram_explicit_edges(items)?, "explicit".to_string()),
        Some(Value::Int(value)) => {
            let bins = histogram_bin_count(*value as f64, "breaks")?;
            (
                histogram_equal_edges(&values, bins),
                format!("equal-width:{bins}"),
            )
        }
        Some(Value::Float(value)) => {
            let bins = histogram_bin_count(*value, "breaks")?;
            (
                histogram_equal_edges(&values, bins),
                format!("equal-width:{bins}"),
            )
        }
        Some(Value::Str(value)) => {
            let method = match value.to_ascii_lowercase().as_str() {
                "sturges" => "sturges",
                "fd" | "freedman-diaconis" | "freedman_diaconis" => "freedman-diaconis",
                "scott" => "scott",
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "histogram() unknown break rule '{value}'; use 'sturges', 'freedman-diaconis', 'scott', a bin count, or an explicit List"
                        ),
                        None,
                    ))
                }
            };
            let bins = histogram_automatic_bin_count(&values, method);
            (
                histogram_equal_edges(&values, bins),
                format!("{method}:equal-width:{bins}"),
            )
        }
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "histogram() option 'breaks' must be a rule name, bin count, or List of edges",
                None,
            ))
        }
        None => {
            let bins = match opts.get("bins") {
                Some(Value::Int(value)) => histogram_bin_count(*value as f64, "bins")?,
                Some(Value::Float(value)) => histogram_bin_count(*value, "bins")?,
                Some(_) => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "histogram() option 'bins' must be a positive whole number",
                        None,
                    ))
                }
                None => 20,
            };
            // ggplot2's reading of `bins`; `span` keeps the equal split of
            // the range that matplotlib and `hist(breaks = n)` use.
            match opts
                .get("bin_rule")
                .and_then(Value::as_str)
                .unwrap_or("ggplot")
            {
                "span" => (
                    histogram_equal_edges(&values, bins),
                    format!("equal-width:{bins}"),
                ),
                "ggplot" | "ggplot2" => (
                    histogram_ggplot_edges(&values, bins),
                    format!("ggplot:{bins}"),
                ),
                other => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                        "histogram() option 'bin_rule' must be 'span' or 'ggplot', got '{other}'"
                    ),
                        None,
                    ))
                }
            }
        }
    };

    let bins = edges.len() - 1;
    let first = edges[0];
    let last = edges[bins];
    let mut counts = vec![0usize; bins];
    let mut dropped_outside = 0usize;
    for value in &values {
        let index = match closure {
            HistogramClosure::Left => {
                if *value < first || *value > last || (*value == last && !include_lowest) {
                    None
                } else if *value == last {
                    Some(bins - 1)
                } else {
                    let upper = edges.partition_point(|edge| *edge <= *value);
                    Some(upper.saturating_sub(1).min(bins - 1))
                }
            }
            HistogramClosure::Right => {
                if *value < first || *value > last || (*value == first && !include_lowest) {
                    None
                } else if *value == first {
                    Some(0)
                } else {
                    let lower = edges.partition_point(|edge| *edge < *value);
                    Some(lower.saturating_sub(1).min(bins - 1))
                }
            }
        };
        if let Some(index) = index {
            counts[index] += 1;
        } else {
            dropped_outside += 1;
        }
    }
    let n_included = counts.iter().sum();

    Ok(HistogramGeometry {
        edges,
        counts,
        method,
        closure,
        include_lowest,
        n_total,
        n_finite: values.len(),
        n_included,
        dropped_invalid,
        dropped_non_finite,
        dropped_outside,
    })
}

fn histogram_geometry_value(geometry: &HistogramGeometry) -> Value {
    let mut cumulative = 0usize;
    let rows = geometry
        .counts
        .iter()
        .enumerate()
        .map(|(index, count)| {
            cumulative += count;
            let left = geometry.edges[index];
            let right = geometry.edges[index + 1];
            let width = right - left;
            let density = if geometry.n_included == 0 || width <= 0.0 {
                0.0
            } else {
                *count as f64 / (geometry.n_included as f64 * width)
            };
            let cumulative_fraction = if geometry.n_included == 0 {
                0.0
            } else {
                cumulative as f64 / geometry.n_included as f64
            };
            let left_closed = geometry.closure == HistogramClosure::Left
                || (index == 0 && geometry.include_lowest);
            let right_closed = geometry.closure == HistogramClosure::Right
                || (index + 1 == geometry.counts.len() && geometry.include_lowest);
            vec![
                Value::Int(index as i64),
                Value::Float(left),
                Value::Float(right),
                Value::Bool(left_closed),
                Value::Bool(right_closed),
                Value::Int(*count as i64),
                Value::Float(density),
                Value::Int(cumulative as i64),
                Value::Float(cumulative_fraction),
            ]
        })
        .collect();
    let table = Table::new(
        [
            "bin",
            "left",
            "right",
            "left_closed",
            "right_closed",
            "count",
            "density",
            "cumulative_count",
            "cumulative_fraction",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        rows,
    );
    let record = HashMap::from([
        ("schema".into(), Value::Str(HISTOGRAM_SCHEMA.into())),
        ("kind".into(), Value::Str("histogram".into())),
        ("method".into(), Value::Str(geometry.method.clone())),
        ("closure".into(), Value::Str(geometry.closure.name().into())),
        (
            "include_lowest".into(),
            Value::Bool(geometry.include_lowest),
        ),
        ("n_total".into(), Value::Int(geometry.n_total as i64)),
        ("n_finite".into(), Value::Int(geometry.n_finite as i64)),
        ("n_included".into(), Value::Int(geometry.n_included as i64)),
        (
            "dropped_invalid".into(),
            Value::Int(geometry.dropped_invalid as i64),
        ),
        (
            "dropped_non_finite".into(),
            Value::Int(geometry.dropped_non_finite as i64),
        ),
        (
            "dropped_outside".into(),
            Value::Int(geometry.dropped_outside as i64),
        ),
        ("bins".into(), Value::Table(table)),
    ]);
    Value::Record(record.into())
}

fn builtin_histogram_data(args: Vec<Value>) -> Result<Value> {
    let geometry = histogram_geometry(&args, "histogram_data")?;
    Ok(histogram_geometry_value(&geometry))
}

fn builtin_histogram(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Histogram").to_string();
    let geometry = histogram_geometry(&args, "histogram")?;
    let max_count = geometry.counts.iter().copied().max().unwrap_or(0).max(1);

    let theme = stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let x_scale = Scale {
        domain: (geometry.edges[0], *geometry.edges.last().unwrap()),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, max_count as f64),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // The default theme keeps its historical bare panel; a named theme draws
    // the panel and grid it implies.
    if !matches!(theme.kind, PlotThemeKind::Legacy) {
        canvas.draw_cartesian_grid(&x_scale, &y_scale);
    }

    // ggplot2 `geom_histogram()` fills with grey35 and draws no border, so its
    // bars abut instead of being separated by a gap.
    let ggplot_like = matches!(theme.kind, PlotThemeKind::Ggplot);
    let bar_fill = if ggplot_like { "#595959" } else { PALETTE[0] };
    let bar_gap = if ggplot_like { 0.0 } else { 1.0 };
    for (index, count) in geometry.counts.iter().enumerate() {
        let x = x_scale.map(geometry.edges[index]);
        let right = x_scale.map(geometry.edges[index + 1]);
        let y = y_scale.map(*count as f64);
        let height = canvas.margin.top + canvas.plot_height() - y;
        canvas.add_rect(x, y, (right - x - bar_gap).max(0.0), height, bar_fill);
    }

    let data_x_scale = Scale {
        domain: x_scale.domain,
        range: x_scale.domain,
    };
    let data_y_scale = Scale {
        domain: (0.0, max_count as f64),
        range: (0.0, max_count as f64),
    };
    canvas.draw_x_axis(&data_x_scale, &axis_label(&opts, "xlabel", "Value"));
    canvas.draw_y_axis(&data_y_scale, &axis_label(&opts, "ylabel", "Count"));
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

fn finite_numeric_list(value: &Value, who: &str) -> Result<(Vec<f64>, usize)> {
    let numbers = numeric_list(value, who)?;
    let original_len = numbers.len();
    let finite = numbers
        .into_iter()
        .filter(|number| number.is_finite())
        .collect::<Vec<_>>();
    if finite.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() received no finite numeric values"),
            None,
        ));
    }
    let dropped = original_len - finite.len();
    Ok((finite, dropped))
}

fn median_sorted(sorted: &[f64]) -> f64 {
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn tukey_hinges(sorted: &[f64]) -> (f64, f64) {
    let half = sorted.len().div_ceil(2);
    (
        median_sorted(&sorted[..half]),
        median_sorted(&sorted[sorted.len() - half..]),
    )
}

#[derive(Clone)]
pub(crate) struct BoxGeometry {
    pub(crate) group: String,
    pub(crate) n: usize,
    pub(crate) q1: f64,
    pub(crate) median: f64,
    pub(crate) q3: f64,
    pub(crate) whisker_low: f64,
    pub(crate) whisker_high: f64,
    pub(crate) outliers: Vec<(usize, f64)>,
    pub(crate) dropped: usize,
}

pub(crate) fn box_geometry(
    name: &str,
    values: &[f64],
    method: &str,
    coefficient: f64,
) -> BoxGeometry {
    let mut indexed = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .collect::<Vec<_>>();
    let dropped = values.len() - indexed.len();
    indexed.sort_by(|left, right| left.1.total_cmp(&right.1));
    let sorted = indexed.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    let (q1, q3) = if method == "tukey" {
        tukey_hinges(&sorted)
    } else {
        (quantile_type7(&sorted, 0.25), quantile_type7(&sorted, 0.75))
    };
    let median = median_sorted(&sorted);
    let iqr = q3 - q1;
    let low_fence = q1 - coefficient * iqr;
    let high_fence = q3 + coefficient * iqr;
    let whisker_low = sorted
        .iter()
        .copied()
        .find(|value| *value >= low_fence)
        .unwrap_or(sorted[0]);
    let whisker_high = sorted
        .iter()
        .copied()
        .rev()
        .find(|value| *value <= high_fence)
        .unwrap_or(sorted[sorted.len() - 1]);
    let outliers = indexed
        .iter()
        .filter(|(_, value)| *value < whisker_low || *value > whisker_high)
        .copied()
        .collect();
    BoxGeometry {
        group: name.to_string(),
        n: sorted.len(),
        q1,
        median,
        q3,
        whisker_low,
        whisker_high,
        outliers,
        dropped,
    }
}

fn builtin_boxplot_data(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let method = get_opt_str(&opts, "method", "type7").to_ascii_lowercase();
    if !matches!(method.as_str(), "type7" | "tukey") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "boxplot_data() method must be 'type7' or 'tukey'",
            None,
        ));
    }
    let coefficient = get_opt_f64(&opts, "coef", 1.5);
    if !coefficient.is_finite() || coefficient < 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "boxplot_data() coef must be a finite non-negative number",
            None,
        ));
    }
    let groups = match &args[0] {
        Value::List(_) => {
            let (values, dropped) = finite_numeric_list(&args[0], "boxplot_data")?;
            let mut geometry = box_geometry("values", &values, &method, coefficient);
            geometry.dropped += dropped;
            vec![geometry]
        }
        Value::Table(table) => {
            let mut groups = Vec::new();
            for column in &table.columns {
                let values = extract_table_col(table, column)?;
                if values.iter().any(|value| value.is_finite()) {
                    groups.push(box_geometry(column, &values, &method, coefficient));
                }
            }
            if groups.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "boxplot_data() table contains no numeric columns",
                    None,
                ));
            }
            groups
        }
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "boxplot_data() requires List or Table, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let group_rows = groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            vec![
                Value::Int(index as i64),
                Value::Str(group.group.clone().into()),
                Value::Int(group.n as i64),
                Value::Float(group.q1),
                Value::Float(group.median),
                Value::Float(group.q3),
                Value::Float(group.q3 - group.q1),
                Value::Float(group.whisker_low),
                Value::Float(group.whisker_high),
                Value::Int(group.outliers.len() as i64),
                Value::Int(group.dropped as i64),
            ]
        })
        .collect();
    let outlier_rows = groups
        .iter()
        .enumerate()
        .flat_map(|(group_index, group)| {
            group.outliers.iter().map(move |(source_row, value)| {
                vec![
                    Value::Int(group_index as i64),
                    Value::Str(group.group.clone().into()),
                    Value::Int(*source_row as i64),
                    Value::Float(*value),
                ]
            })
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("boxplot".into())),
            ("method".into(), Value::Str(method.into())),
            ("coefficient".into(), Value::Float(coefficient)),
            (
                "groups".into(),
                Value::Table(Table::new(
                    vec![
                        "group_index".into(),
                        "group".into(),
                        "n".into(),
                        "q1".into(),
                        "median".into(),
                        "q3".into(),
                        "iqr".into(),
                        "whisker_low".into(),
                        "whisker_high".into(),
                        "outlier_count".into(),
                        "dropped_non_finite".into(),
                    ],
                    group_rows,
                )),
            ),
            (
                "outliers".into(),
                Value::Table(Table::new(
                    vec![
                        "group_index".into(),
                        "group".into(),
                        "source_row".into(),
                        "value".into(),
                    ],
                    outlier_rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone)]
struct EcdfPoint {
    row: usize,
    x: f64,
    count: usize,
    cumulative: usize,
    fraction_before: f64,
    fraction: f64,
}

fn ecdf_geometry(value: &Value, who: &str) -> Result<(Vec<EcdfPoint>, usize, usize)> {
    let (mut values, dropped) = finite_numeric_list(value, who)?;
    values.sort_by(f64::total_cmp);
    let n = values.len();
    let mut points = Vec::new();
    let mut start = 0usize;
    while start < n {
        let x = values[start];
        let mut end = start + 1;
        while end < n && values[end] == x {
            end += 1;
        }
        points.push(EcdfPoint {
            row: points.len(),
            x,
            count: end - start,
            cumulative: end,
            fraction_before: start as f64 / n as f64,
            fraction: end as f64 / n as f64,
        });
        start = end;
    }
    Ok((points, n, dropped))
}

fn builtin_ecdf_data(args: Vec<Value>) -> Result<Value> {
    let (points, n, dropped) = ecdf_geometry(&args[0], "ecdf_data")?;
    let rows = points
        .into_iter()
        .map(|point| {
            vec![
                Value::Int(point.row as i64),
                Value::Float(point.x),
                Value::Int(point.count as i64),
                Value::Int(point.cumulative as i64),
                Value::Float(point.fraction_before),
                Value::Float(point.fraction),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("ecdf".into())),
            ("n".into(), Value::Int(n as i64)),
            ("dropped_non_finite".into(), Value::Int(dropped as i64)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "row".into(),
                        "x".into(),
                        "count".into(),
                        "cumulative_count".into(),
                        "fraction_before".into(),
                        "fraction".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

fn refined_normal_quantile(probability: f64) -> f64 {
    let mut estimate = bl_core::bio_core::stats_ops::normal_quantile(probability);
    // One Newton correction removes the final few ulps left by the fast
    // rational approximation used by the general qnorm builtin. Plotting
    // positions are deterministic and only O(n), so the extra CDF evaluation
    // is preferable to visibly asymmetric or oracle-dependent Q-Q tails.
    for _ in 0..2 {
        let density = (-0.5 * estimate * estimate).exp() / (2.0 * std::f64::consts::PI).sqrt();
        if density <= f64::MIN_POSITIVE {
            break;
        }
        estimate -= (bl_core::bio_core::stats_ops::normal_cdf(estimate) - probability) / density;
    }
    estimate
}

#[derive(Clone, Debug)]
pub(crate) struct NormalQqGeometry {
    pub(crate) probabilities: Vec<f64>,
    pub(crate) theoretical: Vec<f64>,
    pub(crate) sample: Vec<f64>,
    pub(crate) line_intercept: f64,
    pub(crate) line_slope: f64,
    pub(crate) dropped: usize,
}

/// Renderer-independent normal Q-Q coordinates using R's `ppoints()` rule and
/// the quartile line drawn by `qqline()`. Keeping this here makes the guided
/// statistics plot, diagnostics, and public geometry builtin use one declared
/// convention instead of three subtly different approximations.
pub(crate) fn normal_qq_geometry(values: &[f64]) -> Result<NormalQqGeometry> {
    let mut sample = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    let dropped = values.len() - sample.len();
    if sample.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "normal Q-Q geometry requires at least one finite value",
            None,
        ));
    }
    sample.sort_by(f64::total_cmp);
    let n = sample.len();
    let offset = if n <= 10 { 0.375 } else { 0.5 };
    let denominator = n as f64 + 1.0 - 2.0 * offset;
    let probabilities = (0..n)
        .map(|index| (index as f64 + 1.0 - offset) / denominator)
        .collect::<Vec<_>>();
    let theoretical = probabilities
        .iter()
        .map(|probability| refined_normal_quantile(*probability))
        .collect::<Vec<_>>();
    let sample_q1 = quantile_type7(&sample, 0.25);
    let sample_q3 = quantile_type7(&sample, 0.75);
    let theoretical_q1 = refined_normal_quantile(0.25);
    let theoretical_q3 = refined_normal_quantile(0.75);
    let line_slope = (sample_q3 - sample_q1) / (theoretical_q3 - theoretical_q1);
    let line_intercept = sample_q1 - line_slope * theoretical_q1;
    Ok(NormalQqGeometry {
        probabilities,
        theoretical,
        sample,
        line_intercept,
        line_slope,
        dropped,
    })
}

fn builtin_normal_qq_data(args: Vec<Value>) -> Result<Value> {
    let (values, separately_dropped) = finite_numeric_list(&args[0], "normal_qq_data")?;
    let geometry = normal_qq_geometry(&values)?;
    let rows = geometry
        .sample
        .iter()
        .enumerate()
        .map(|(index, value)| {
            vec![
                Value::Int(index as i64),
                Value::Float(geometry.probabilities[index]),
                Value::Float(geometry.theoretical[index]),
                Value::Float(*value),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("normal_qq".into())),
            ("plotting_position".into(), Value::Str("R_ppoints".into())),
            ("n".into(), Value::Int(geometry.sample.len() as i64)),
            (
                "dropped_non_finite".into(),
                Value::Int((separately_dropped + geometry.dropped) as i64),
            ),
            (
                "line_intercept".into(),
                Value::Float(geometry.line_intercept),
            ),
            ("line_slope".into(), Value::Float(geometry.line_slope)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "row".into(),
                        "probability".into(),
                        "theoretical".into(),
                        "sample".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

fn builtin_violin_data(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let (mut values, dropped) = finite_numeric_list(&args[0], "violin_data")?;
    values.sort_by(f64::total_cmp);
    let adjust = get_opt_f64(&opts, "adjust", 1.0);
    if !adjust.is_finite() || adjust <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin_data() adjust must be a positive finite number",
            None,
        ));
    }
    let points_number = get_opt_f64(&opts, "points", 256.0);
    if !points_number.is_finite()
        || points_number.fract() != 0.0
        || !(16.0..=4096.0).contains(&points_number)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin_data() points must be a whole number from 16 to 4096",
            None,
        ));
    }
    let points = points_number as usize;
    let bandwidth = match opts.get("bandwidth") {
        Some(value) => value
            .as_float()
            .filter(|number| number.is_finite() && *number > 0.0)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "violin_data() bandwidth must be a positive finite number",
                    None,
                )
            })?,
        None => silverman_bandwidth(&values) * adjust,
    };
    let density = gaussian_kde(&values, bandwidth, points);
    let peak = density.iter().map(|(_, value)| *value).fold(0.0, f64::max);
    let rows = density
        .into_iter()
        .enumerate()
        .map(|(index, (x, value))| {
            vec![
                Value::Int(index as i64),
                Value::Float(x),
                Value::Float(value),
                Value::Float(if peak > 0.0 { value / peak } else { 0.0 }),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("violin".into())),
            ("kernel".into(), Value::Str("gaussian".into())),
            ("bandwidth_method".into(), Value::Str("bw.nrd0".into())),
            ("bandwidth".into(), Value::Float(bandwidth)),
            ("n".into(), Value::Int(values.len() as i64)),
            ("dropped_non_finite".into(), Value::Int(dropped as i64)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec!["row".into(), "x".into(), "density".into(), "scaled".into()],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct LinearFitPoint {
    pub(crate) x: f64,
    pub(crate) fitted: f64,
    pub(crate) confidence_lower: f64,
    pub(crate) confidence_upper: f64,
    pub(crate) prediction_lower: f64,
    pub(crate) prediction_upper: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct LinearFitGeometry {
    pub(crate) n: usize,
    pub(crate) slope: f64,
    pub(crate) intercept: f64,
    pub(crate) degrees_of_freedom: usize,
    pub(crate) residual_mse: f64,
    pub(crate) residual_standard_error: f64,
    pub(crate) confidence_level: f64,
    pub(crate) critical_value: f64,
    pub(crate) points: Vec<LinearFitPoint>,
}

/// Ordinary least-squares line geometry with intervals for the mean response
/// and for a new observation. These are deliberately distinct: a prediction
/// band contains the irreducible residual variance and must therefore be wider.
pub(crate) fn linear_fit_geometry(
    xs: &[f64],
    ys: &[f64],
    at: &[f64],
    confidence_level: f64,
) -> Result<LinearFitGeometry> {
    if xs.len() != ys.len() || xs.len() < 3 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear fit geometry requires at least three paired values",
            None,
        ));
    }
    if !confidence_level.is_finite() || !(0.0..1.0).contains(&confidence_level) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear_fit_data() confidence must be between 0 and 1",
            None,
        ));
    }
    if xs
        .iter()
        .chain(ys)
        .chain(at)
        .any(|value| !value.is_finite())
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear fit geometry requires finite numeric values",
            None,
        ));
    }
    let n = xs.len();
    let mean_x = xs.iter().sum::<f64>() / n as f64;
    let mean_y = ys.iter().sum::<f64>() / n as f64;
    let sum_xx = xs.iter().map(|x| (x - mean_x).powi(2)).sum::<f64>();
    if sum_xx <= f64::EPSILON {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear_fit_data() requires variation in x",
            None,
        ));
    }
    let sum_xy = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let slope = sum_xy / sum_xx;
    let intercept = mean_y - slope * mean_x;
    let residual_sum_squares = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| (y - (intercept + slope * x)).powi(2))
        .sum::<f64>();
    let degrees_of_freedom = n - 2;
    let residual_mse = residual_sum_squares / degrees_of_freedom as f64;
    let residual_standard_error = residual_mse.sqrt();
    let critical_value = bl_core::bio_core::stats_ops::students_t_quantile(
        0.5 + confidence_level / 2.0,
        degrees_of_freedom as f64,
    );
    let points = at
        .iter()
        .map(|x| {
            let fitted = intercept + slope * x;
            let mean_leverage = 1.0 / n as f64 + (x - mean_x).powi(2) / sum_xx;
            let confidence_margin = critical_value * (residual_mse * mean_leverage).sqrt();
            let prediction_margin = critical_value * (residual_mse * (1.0 + mean_leverage)).sqrt();
            LinearFitPoint {
                x: *x,
                fitted,
                confidence_lower: fitted - confidence_margin,
                confidence_upper: fitted + confidence_margin,
                prediction_lower: fitted - prediction_margin,
                prediction_upper: fitted + prediction_margin,
            }
        })
        .collect();
    Ok(LinearFitGeometry {
        n,
        slope,
        intercept,
        degrees_of_freedom,
        residual_mse,
        residual_standard_error,
        confidence_level,
        critical_value,
        points,
    })
}

fn paired_finite_lists(x: &Value, y: &Value, who: &str) -> Result<(Vec<f64>, Vec<f64>, usize)> {
    let (Value::List(x_items), Value::List(y_items)) = (x, y) else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() requires two Lists"),
            None,
        ));
    };
    if x_items.len() != y_items.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() x and y must have equal length"),
            None,
        ));
    }
    let mut xs = Vec::with_capacity(x_items.len());
    let mut ys = Vec::with_capacity(y_items.len());
    let mut dropped = 0usize;
    for (x, y) in x_items.iter().zip(y_items.iter()) {
        match (x.as_float(), y.as_float()) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {
                xs.push(x);
                ys.push(y);
            }
            _ => dropped += 1,
        }
    }
    Ok((xs, ys, dropped))
}

fn builtin_linear_fit_data(args: Vec<Value>) -> Result<Value> {
    let opts = match args.get(2) {
        None | Some(Value::Nil) => HashMap::new(),
        Some(Value::Record(values)) => values.as_ref().clone(),
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "linear_fit_data() options must be a Record",
                None,
            ))
        }
    };
    let (xs, ys, dropped) = paired_finite_lists(&args[0], &args[1], "linear_fit_data")?;
    let confidence = get_opt_f64(&opts, "confidence", 0.95);
    let mut at = match opts.get("at") {
        Some(value) => finite_numeric_list(value, "linear_fit_data")?.0,
        None => {
            let mut values = xs.clone();
            values.sort_by(f64::total_cmp);
            values.dedup_by(|left, right| *left == *right);
            values
        }
    };
    at.sort_by(f64::total_cmp);
    let geometry = linear_fit_geometry(&xs, &ys, &at, confidence)?;
    let rows = geometry
        .points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            vec![
                Value::Int(index as i64),
                Value::Float(point.x),
                Value::Float(point.fitted),
                Value::Float(point.confidence_lower),
                Value::Float(point.confidence_upper),
                Value::Float(point.prediction_lower),
                Value::Float(point.prediction_upper),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("linear_fit".into())),
            ("n".into(), Value::Int(geometry.n as i64)),
            ("dropped_incomplete".into(), Value::Int(dropped as i64)),
            ("slope".into(), Value::Float(geometry.slope)),
            ("intercept".into(), Value::Float(geometry.intercept)),
            (
                "degrees_of_freedom".into(),
                Value::Int(geometry.degrees_of_freedom as i64),
            ),
            ("residual_mse".into(), Value::Float(geometry.residual_mse)),
            (
                "residual_standard_error".into(),
                Value::Float(geometry.residual_standard_error),
            ),
            (
                "confidence_level".into(),
                Value::Float(geometry.confidence_level),
            ),
            (
                "critical_value".into(),
                Value::Float(geometry.critical_value),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "row".into(),
                        "x".into(),
                        "fitted".into(),
                        "confidence_lower".into(),
                        "confidence_upper".into(),
                        "prediction_lower".into(),
                        "prediction_upper".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct CategoricalGeometry {
    pub(crate) labels: Vec<String>,
    pub(crate) counts: Vec<usize>,
    pub(crate) n_total: usize,
    pub(crate) n_observed: usize,
    pub(crate) missing: usize,
}

fn categorical_label(value: &Value) -> Option<String> {
    match value {
        Value::Str(value) => Some(value.to_string()),
        Value::Int(value) => Some(value.to_string()),
        Value::Float(value) if value.is_finite() => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

/// First-observed categorical frequencies. The order is part of the geometry:
/// silently sorting labels changes which bar a reader associates with a group.
pub(crate) fn categorical_geometry(value: &Value, who: &str) -> Result<CategoricalGeometry> {
    let Value::List(items) = value else {
        return Err(BioLangError::type_error(
            format!("{who}() requires a List, got {}", value.type_of()),
            None,
        ));
    };
    let mut labels = Vec::new();
    let mut counts = Vec::new();
    let mut positions = HashMap::<String, usize>::new();
    let mut missing = 0usize;
    for item in items.iter() {
        if matches!(item, Value::Nil) {
            missing += 1;
            continue;
        }
        let Some(label) = categorical_label(item) else {
            return Err(BioLangError::type_error(
                format!("{who}() categories must be finite scalar values or Nil"),
                None,
            ));
        };
        let position = *positions.entry(label.clone()).or_insert_with(|| {
            labels.push(label);
            counts.push(0);
            labels.len() - 1
        });
        counts[position] += 1;
    }
    if labels.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() has no observed categories"),
            None,
        ));
    }
    Ok(CategoricalGeometry {
        labels,
        counts,
        n_total: items.len(),
        n_observed: items.len() - missing,
        missing,
    })
}

fn builtin_categorical_data(args: Vec<Value>) -> Result<Value> {
    let geometry = categorical_geometry(&args[0], "categorical_data")?;
    let rows = geometry
        .labels
        .iter()
        .zip(&geometry.counts)
        .enumerate()
        .map(|(index, (label, count))| {
            vec![
                Value::Int(index as i64),
                Value::Str(label.clone().into()),
                Value::Int(*count as i64),
                Value::Float(*count as f64 / geometry.n_observed as f64),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("categorical".into())),
            ("ordering".into(), Value::Str("first_observed".into())),
            ("n_total".into(), Value::Int(geometry.n_total as i64)),
            ("n_observed".into(), Value::Int(geometry.n_observed as i64)),
            ("missing".into(), Value::Int(geometry.missing as i64)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "category_index".into(),
                        "label".into(),
                        "count".into(),
                        "proportion".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct MissingnessCell {
    pub(crate) display_row: usize,
    pub(crate) display_column: usize,
    pub(crate) source_row: usize,
    pub(crate) source_column: usize,
    pub(crate) missing: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct MissingnessGeometry {
    pub(crate) n_rows: usize,
    pub(crate) n_columns: usize,
    pub(crate) missing_cells: usize,
    pub(crate) row_stride: usize,
    pub(crate) column_stride: usize,
    pub(crate) displayed_rows: Vec<usize>,
    pub(crate) displayed_columns: Vec<usize>,
    pub(crate) column_missing: Vec<usize>,
    pub(crate) cells: Vec<MissingnessCell>,
}

pub(crate) fn value_is_missing(value: &Value) -> bool {
    matches!(value, Value::Nil) || matches!(value, Value::Float(number) if !number.is_finite())
}

/// Full missing counts plus a deterministic, bounded display grid. Counts use
/// every table cell; strides affect only the cells handed to a renderer.
pub(crate) fn missingness_geometry(
    table: &Table,
    max_rows: usize,
    max_columns: usize,
) -> MissingnessGeometry {
    let row_stride = table.rows.len().div_ceil(max_rows.max(1)).max(1);
    let column_stride = table.columns.len().div_ceil(max_columns.max(1)).max(1);
    let displayed_rows = (0..table.rows.len())
        .step_by(row_stride)
        .collect::<Vec<_>>();
    let displayed_columns = (0..table.columns.len())
        .step_by(column_stride)
        .collect::<Vec<_>>();
    let mut column_missing = vec![0usize; table.columns.len()];
    for row in &table.rows {
        for (column, missing) in column_missing.iter_mut().enumerate() {
            if value_is_missing(row.get(column).unwrap_or(&Value::Nil)) {
                *missing += 1;
            }
        }
    }
    let missing_cells = column_missing.iter().sum();
    let cells = displayed_rows
        .iter()
        .enumerate()
        .flat_map(|(display_row, source_row)| {
            displayed_columns
                .iter()
                .enumerate()
                .map(move |(display_column, source_column)| MissingnessCell {
                    display_row,
                    display_column,
                    source_row: *source_row,
                    source_column: *source_column,
                    missing: value_is_missing(
                        table.rows[*source_row]
                            .get(*source_column)
                            .unwrap_or(&Value::Nil),
                    ),
                })
        })
        .collect();
    MissingnessGeometry {
        n_rows: table.rows.len(),
        n_columns: table.columns.len(),
        missing_cells,
        row_stride,
        column_stride,
        displayed_rows,
        displayed_columns,
        column_missing,
        cells,
    }
}

fn builtin_missingness_data(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "missingness_data")?;
    let opts = parse_options(&args);
    let max_rows = geometry_limit(&opts, "max_rows", 100, 10_000)?;
    let max_columns = geometry_limit(&opts, "max_columns", 40, 1_000)?;
    let geometry = missingness_geometry(table, max_rows, max_columns);
    let row_rows = geometry
        .displayed_rows
        .iter()
        .enumerate()
        .map(|(display_row, source_row)| {
            vec![
                Value::Int(display_row as i64),
                Value::Int(*source_row as i64),
            ]
        })
        .collect();
    let column_rows = geometry
        .displayed_columns
        .iter()
        .enumerate()
        .map(|(display_column, source_column)| {
            vec![
                Value::Int(display_column as i64),
                Value::Int(*source_column as i64),
                Value::Str(table.columns[*source_column].clone().into()),
            ]
        })
        .collect();
    let summary_rows = table
        .columns
        .iter()
        .enumerate()
        .map(|(source_column, name)| {
            let count = geometry.column_missing[source_column];
            vec![
                Value::Int(source_column as i64),
                Value::Str(name.clone().into()),
                Value::Int(count as i64),
                Value::Float(if geometry.n_rows == 0 {
                    0.0
                } else {
                    count as f64 / geometry.n_rows as f64
                }),
            ]
        })
        .collect();
    let cell_rows = geometry
        .cells
        .iter()
        .map(|cell| {
            vec![
                Value::Int(cell.display_row as i64),
                Value::Int(cell.display_column as i64),
                Value::Int(cell.source_row as i64),
                Value::Int(cell.source_column as i64),
                Value::Bool(cell.missing),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("missingness".into())),
            ("n_rows".into(), Value::Int(geometry.n_rows as i64)),
            ("n_columns".into(), Value::Int(geometry.n_columns as i64)),
            (
                "missing_cells".into(),
                Value::Int(geometry.missing_cells as i64),
            ),
            ("row_stride".into(), Value::Int(geometry.row_stride as i64)),
            (
                "column_stride".into(),
                Value::Int(geometry.column_stride as i64),
            ),
            (
                "displayed_rows".into(),
                Value::Table(Table::new(
                    vec!["display_row".into(), "source_row".into()],
                    row_rows,
                )),
            ),
            (
                "displayed_columns".into(),
                Value::Table(Table::new(
                    vec![
                        "display_column".into(),
                        "source_column".into(),
                        "column".into(),
                    ],
                    column_rows,
                )),
            ),
            (
                "column_summary".into(),
                Value::Table(Table::new(
                    vec![
                        "source_column".into(),
                        "column".into(),
                        "missing_count".into(),
                        "missing_fraction".into(),
                    ],
                    summary_rows,
                )),
            ),
            (
                "cells".into(),
                Value::Table(Table::new(
                    vec![
                        "display_row".into(),
                        "display_column".into(),
                        "source_row".into(),
                        "source_column".into(),
                        "missing".into(),
                    ],
                    cell_rows,
                )),
            ),
        ])
        .into(),
    ))
}

fn geometry_limit(
    opts: &HashMap<String, Value>,
    key: &str,
    default: usize,
    maximum: usize,
) -> Result<usize> {
    let Some(value) = opts.get(key) else {
        return Ok(default);
    };
    let number = match value {
        Value::Int(value) if *value > 0 => *value as usize,
        Value::Float(value) if value.is_finite() && *value >= 1.0 && value.fract() == 0.0 => {
            *value as usize
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("missingness_data() option '{key}' must be a positive whole number"),
                None,
            ))
        }
    };
    if number > maximum {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("missingness_data() option '{key}' exceeds the safety limit of {maximum}"),
            None,
        ));
    }
    Ok(number)
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

    let (geometry, _, _) = ecdf_geometry(&args[0], "ecdf_plot")?;
    let x_values = geometry.iter().map(|point| point.x).collect::<Vec<_>>();
    let (lo, hi) = col_range(&x_values);
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
    let mut points = Vec::with_capacity(2 * geometry.len() + 2);
    points.push(format!(
        "{:.1},{:.1}",
        x_scale.map(geometry[0].x),
        y_scale.map(0.0)
    ));
    for (index, point) in geometry.iter().enumerate() {
        let x = x_scale.map(point.x);
        let y = y_scale.map(point.fraction);
        // The riser at the observation, then the flat run to the next one.
        points.push(format!("{x:.1},{y:.1}"));
        let next_x = match geometry.get(index + 1) {
            Some(next) => x_scale.map(next.x),
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
pub(crate) fn silverman_bandwidth(values: &[f64]) -> f64 {
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

pub(crate) fn gaussian_kde(values: &[f64], bandwidth: f64, steps: usize) -> Vec<(f64, f64)> {
    let bandwidth = bandwidth.max(f64::MIN_POSITIVE);
    let steps = steps.max(2);
    let (data_lo, data_hi) = col_range(values);
    let lo = data_lo - 3.0 * bandwidth;
    let hi = data_hi + 3.0 * bandwidth;
    let normaliser = 1.0 / (values.len() as f64 * bandwidth * (2.0 * std::f64::consts::PI).sqrt());
    (0..steps)
        .map(|step| {
            let x = lo + (hi - lo) * step as f64 / (steps - 1) as f64;
            let density = values
                .iter()
                .map(|value| {
                    let z = (x - value) / bandwidth;
                    (-0.5 * z * z).exp()
                })
                .sum::<f64>()
                * normaliser;
            (x, density)
        })
        .collect()
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

    let bandwidth =
        get_opt_f64(&opts, "bandwidth", silverman_bandwidth(&values)).max(f64::MIN_POSITIVE);

    let steps = 256usize;
    let densities = gaussian_kde(&values, bandwidth, steps);
    let lo = densities[0].0;
    let hi = densities[densities.len() - 1].0;
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

fn render_volcano_svg(
    fcs: &[f64],
    pvals: &[f64],
    fc_thresh: f64,
    p_thresh: f64,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let fc_col = get_opt_str(opts, "fc", "log2fc");
    let p_col = get_opt_str(opts, "p", "pvalue");
    let neg_log_p: Vec<f64> = pvals
        .iter()
        .map(|&p| if p > 0.0 { -(p.log10()) } else { 0.0 })
        .collect();
    let (x_min, x_max) = col_range(fcs);
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
    let renderable = (0..fcs.len().min(neg_log_p.len()))
        .filter(|&index| fcs[index].is_finite() && neg_log_p[index].is_finite())
        .collect::<Vec<_>>();
    let raster = raster_choice(opts, "volcano", renderable.len())?;
    let points: Vec<(f64, f64, &str)> = renderable
        .iter()
        .map(|&index| {
            let colour = if neg_log_p[index] > neg_log_p_thresh && fcs[index].abs() > fc_thresh {
                if fcs[index] > 0.0 {
                    "#e15759"
                } else {
                    "#4e79a7"
                }
            } else {
                "#999"
            };
            (
                x_scale.map(fcs[index]),
                y_scale.map(neg_log_p[index]),
                colour,
            )
        })
        .collect();
    let area = canvas.point_area();
    canvas.add_scatter(&points, 3.0, area, raster);
    canvas.draw_x_axis(
        &Scale {
            domain: (-x_abs, x_abs),
            range: (-x_abs, x_abs),
        },
        &axis_label(opts, "xlabel", &format!("log2(FC) [{fc_col}]")),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, y_max),
            range: (0.0, y_max),
        },
        &axis_label(opts, "ylabel", &format!("-log10(p) [{p_col}]")),
    );
    canvas.draw_title("Volcano Plot");
    canvas.set_accessible_description(format!(
        "Volcano plot with {} rendered of {} rows; fold-change threshold {fc_thresh} and p-value threshold {p_thresh}.",
        renderable.len(),
        fcs.len().min(pvals.len())
    ));
    Ok(canvas.render())
}

fn render_ma_svg(
    a_vals: &[f64],
    m_vals: &[f64],
    m_threshold: f64,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let a_col = get_opt_str(opts, "a", "baseMean");
    let m_col = get_opt_str(opts, "m", "log2fc");
    let a_log: Vec<f64> = a_vals
        .iter()
        .map(|&value| if value > 0.0 { value.log2() } else { 0.0 })
        .collect();
    let (x_min, x_max) = col_range(&a_log);
    let (y_min, y_max) = col_range(m_vals);
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
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(0.0),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(0.0),
        "#ccc",
        1.0,
    );
    let renderable = (0..a_log.len().min(m_vals.len()))
        .filter(|&index| a_log[index].is_finite() && m_vals[index].is_finite())
        .collect::<Vec<_>>();
    let raster = raster_choice(opts, "ma_plot", renderable.len())?;
    let points: Vec<(f64, f64, &str)> = renderable
        .iter()
        .map(|&index| {
            let colour = if m_vals[index].abs() > m_threshold {
                "#e15759"
            } else {
                "#999"
            };
            (
                x_scale.map(a_log[index]),
                y_scale.map(m_vals[index]),
                colour,
            )
        })
        .collect();
    let area = canvas.point_area();
    canvas.add_scatter(&points, 3.0, area, raster);
    canvas.draw_x_axis(
        &Scale {
            domain: (x_min, x_max),
            range: (x_min, x_max),
        },
        &axis_label(opts, "xlabel", &format!("A (log2 {a_col})")),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (-y_abs, y_abs),
            range: (-y_abs, y_abs),
        },
        &axis_label(opts, "ylabel", &format!("M ({m_col})")),
    );
    canvas.draw_title("MA Plot");
    canvas.set_accessible_description(format!(
        "MA plot with {} rendered of {} rows; absolute log2 fold-change threshold {m_threshold}.",
        renderable.len(),
        a_vals.len().min(m_vals.len())
    ));
    Ok(canvas.render())
}

fn differential_plot_spec_value(
    plot_kind: &str,
    raw_x: &[f64],
    raw_y: &[f64],
    labels: &[String],
    x_column: &str,
    y_column: &str,
    fc_threshold: f64,
    p_threshold: Option<f64>,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let point_count = raw_x.len().min(raw_y.len());
    let transformed_y = if plot_kind == "volcano" {
        raw_y
            .iter()
            .map(|&value| if value > 0.0 { -(value.log10()) } else { 0.0 })
            .collect::<Vec<_>>()
    } else {
        raw_y.to_vec()
    };
    let transformed_x = if plot_kind == "ma" {
        raw_x
            .iter()
            .map(|&value| if value > 0.0 { value.log2() } else { 0.0 })
            .collect::<Vec<_>>()
    } else {
        raw_x.to_vec()
    };
    let rendered_points = (0..point_count)
        .filter(|&index| transformed_x[index].is_finite() && transformed_y[index].is_finite())
        .count();
    let raster = raster_choice(
        opts,
        if plot_kind == "volcano" {
            "volcano"
        } else {
            "ma_plot"
        },
        rendered_points,
    )?;
    let neg_log_p_threshold = p_threshold.map(|value| -(value.log10()));
    let rows = (0..point_count)
        .map(|index| {
            let status = if !transformed_x[index].is_finite() || !transformed_y[index].is_finite() {
                "not_rendered"
            } else if plot_kind == "volcano" {
                if transformed_y[index] > neg_log_p_threshold.unwrap_or(f64::INFINITY)
                    && raw_x[index].abs() > fc_threshold
                {
                    if raw_x[index] > 0.0 {
                        "up"
                    } else {
                        "down"
                    }
                } else {
                    "not_significant"
                }
            } else if raw_y[index].abs() > fc_threshold {
                "changed"
            } else {
                "not_changed"
            };
            vec![
                Value::Int(index as i64),
                Value::Float(raw_x[index]),
                Value::Float(raw_y[index]),
                Value::Float(transformed_x[index]),
                Value::Float(transformed_y[index]),
                labels
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .map(|value| Value::Str(value.clone()))
                    .unwrap_or(Value::Nil),
                Value::Str(status.into()),
            ]
        })
        .collect::<Vec<_>>();
    let title = if plot_kind == "volcano" {
        "Volcano Plot"
    } else {
        "MA Plot"
    };
    let mut spec_options = HashMap::from([
        ("plot".into(), Value::Str(plot_kind.into())),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 800.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 600.0)),
        ),
        ("x_column".into(), Value::Str(x_column.into())),
        ("y_column".into(), Value::Str(y_column.into())),
        ("fold_change_threshold".into(), Value::Float(fc_threshold)),
        ("raster".into(), Value::Bool(raster.enabled)),
        ("raster_scale".into(), Value::Float(raster.scale)),
    ]);
    if let Some(value) = p_threshold {
        spec_options.insert("p_value_threshold".into(), Value::Float(value));
    }
    let default_x_label = if plot_kind == "volcano" {
        format!("log2(FC) [{x_column}]")
    } else {
        format!("A (log2 {x_column})")
    };
    let default_y_label = if plot_kind == "volcano" {
        format!("-log10(p) [{y_column}]")
    } else {
        format!("M ({y_column})")
    };
    spec_options.insert(
        "xlabel".into(),
        Value::Str(axis_label(opts, "xlabel", &default_x_label)),
    );
    spec_options.insert(
        "ylabel".into(),
        Value::Str(axis_label(opts, "ylabel", &default_y_label)),
    );
    spec_options.insert(
        if plot_kind == "volcano" { "fc" } else { "a" }.into(),
        Value::Str(x_column.into()),
    );
    spec_options.insert(
        if plot_kind == "volcano" { "p" } else { "m" }.into(),
        Value::Str(y_column.into()),
    );
    let non_finite_coordinates = point_count - rendered_points;
    let warnings = if non_finite_coordinates == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{non_finite_coordinates} rows have non-finite plot coordinates"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str("differential_expression".into())),
            ("plot".into(), Value::Str(plot_kind.into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    ["source_row", "raw_x", "raw_y", "x", "y", "label", "status"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    rows,
                )),
            ),
            ("options".into(), Value::Record(spec_options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "builtin".into(),
                            Value::Str(if plot_kind == "volcano" {
                                "volcano".into()
                            } else {
                                "ma_plot".into()
                            }),
                        ),
                        ("input_rows".into(), Value::Int(point_count as i64)),
                        ("rendered_points".into(), Value::Int(rendered_points as i64)),
                        (
                            "non_finite_coordinates".into(),
                            Value::Int(non_finite_coordinates as i64),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(warnings.into())),
        ])
        .into(),
    ))
}

fn is_differential_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "differential_expression")
    )
}

fn render_differential_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_differential_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 differential-expression Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() differential-expression field 'data' must be Table",
                None,
            ))
        }
    };
    for required in ["source_row", "raw_x", "raw_y", "x", "y", "label", "status"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() differential-expression data is missing '{required}'"),
                None,
            ));
        }
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() differential-expression field 'options' must be Record",
                None,
            ))
        }
    };
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in [
        "raster",
        "raster_threshold",
        "raster_scale",
        "width",
        "height",
    ] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    let raw_x = extract_table_col(table, "raw_x")?;
    let raw_y = extract_table_col(table, "raw_y")?;
    let plot_kind = map.get("plot").and_then(Value::as_str).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() differential-expression specification is missing 'plot'",
            None,
        )
    })?;
    let threshold = options
        .get("fold_change_threshold")
        .and_then(Value::as_float)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() differential-expression options are missing numeric 'fold_change_threshold'",
                None,
            )
        })?;
    let svg = match plot_kind {
        "volcano" => {
            let p_threshold = options
                .get("p_value_threshold")
                .and_then(Value::as_float)
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "render_plot() volcano options are missing numeric 'p_value_threshold'",
                        None,
                    )
                })?;
            render_volcano_svg(&raw_x, &raw_y, threshold, p_threshold, &options)?
        }
        "ma" => render_ma_svg(&raw_x, &raw_y, threshold, &options)?,
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() unknown differential-expression plot '{other}'"),
                None,
            ))
        }
    };
    let title = map.get("title").and_then(Value::as_str).unwrap_or("Plot");
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Ascii)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Braille)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal differential-expression output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown differential-expression format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn extract_optional_plot_labels(table: &Table) -> Vec<String> {
    ["gene", "name", "id"]
        .iter()
        .find_map(|column| {
            let index = table.col_index(column)?;
            Some(
                table
                    .rows
                    .iter()
                    .map(|row| match &row[index] {
                        Value::Str(value) => value.clone(),
                        Value::Nil => String::new(),
                        other => format!("{other}"),
                    })
                    .collect(),
            )
        })
        .unwrap_or_else(|| vec![String::new(); table.num_rows()])
}

fn builtin_volcano(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "volcano")?;
    let opts = parse_options(&args);
    let fc_col = get_opt_str(&opts, "fc", "log2fc").to_string();
    let p_col = get_opt_str(&opts, "p", "pvalue").to_string();
    let fc_thresh = get_opt_f64(&opts, "fc_threshold", 1.0);
    let p_thresh = get_opt_f64(&opts, "p_threshold", 0.05);
    let fcs = extract_table_col(table, &fc_col)?;
    let pvals = extract_table_col(table, &p_col)?;
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let labels = extract_optional_plot_labels(table);
        let spec = differential_plot_spec_value(
            "volcano",
            &fcs,
            &pvals,
            &labels,
            &fc_col,
            &p_col,
            fc_thresh,
            Some(p_thresh),
            &opts,
        )?;
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_differential_plot_spec_value(&spec, &opts);
    }
    render_volcano_svg(&fcs, &pvals, fc_thresh, p_thresh, &opts).map(Value::Str)
}

fn builtin_ma_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "ma_plot")?;
    let opts = parse_options(&args);
    let a_col = get_opt_str(&opts, "a", "baseMean").to_string();
    let m_col = get_opt_str(&opts, "m", "log2fc").to_string();

    let a_vals = extract_table_col(table, &a_col)?;
    let m_vals = extract_table_col(table, &m_col)?;

    // Preserve the legacy MA classification boundary exactly.
    const M_THRESHOLD: f64 = 1.0;
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let labels = extract_optional_plot_labels(table);
        let spec = differential_plot_spec_value(
            "ma",
            &a_vals,
            &m_vals,
            &labels,
            &a_col,
            &m_col,
            M_THRESHOLD,
            None,
            &opts,
        )?;
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_differential_plot_spec_value(&spec, &opts);
    }
    render_ma_svg(&a_vals, &m_vals, M_THRESHOLD, &opts).map(Value::Str)
}

fn builtin_save_svg(args: Vec<Value>) -> Result<Value> {
    let svg = match &args[0] {
        Value::Str(s) => s.to_string(),
        Value::Record(_) => match builtin_render_plot(vec![args[0].clone()])? {
            Value::Str(svg) if svg.trim_start().starts_with("<svg") => svg.to_string(),
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "save_svg() rendered PlotSpec as {}, expected SVG",
                        other.type_of()
                    ),
                    None,
                ))
            }
        },
        Value::Nil => return Err(BioLangError::type_error(
            "save_svg()/save_plot() received Nil — the plot function before the pipe likely failed or returned nothing".to_string(), None,
        )),
        other => return Err(BioLangError::type_error(
            format!("save_svg() requires SVG Str or PlotSpec Record, got {}", other.type_of()), None,
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
    let opts = parse_options(&args[1..]);
    let profile = get_opt_str(&opts, "profile", "screen").to_ascii_lowercase();
    if !matches!(profile.as_str(), "screen" | "publication" | "journal") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "save_svg() profile must be screen or publication",
            None,
        ));
    }
    let mut output = svg;
    let svg_start = output.find("<svg").ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "save_svg() requires an SVG document or PlotSpec",
            None,
        )
    })?;
    if matches!(profile.as_str(), "publication" | "journal") {
        let font = match get_opt_str(&opts, "font", "sans")
            .to_ascii_lowercase()
            .as_str()
        {
            "sans" | "sans-serif" | "arial" | "helvetica" => "Arial,Helvetica,sans-serif",
            "serif" | "times" => "Times New Roman,Times,serif",
            "mono" | "monospace" => "Courier New,monospace",
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "save_svg() publication font must be sans, serif, or mono",
                    None,
                ))
            }
        };
        let metadata = format!(
            "<metadata>BioLang publication figure; vector text; font profile: {}</metadata><style>text{{font-family:{font}}}</style>",
            xml_escape(font)
        );
        let opening = output[svg_start..]
            .find('>')
            .map(|offset| svg_start + offset)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "save_svg() received malformed SVG",
                    None,
                )
            })?;
        output.insert_str(opening + 1, &metadata);
        output.insert_str(svg_start + 4, " data-biolang-export=\"publication\"");
    }
    let width_mm = opts.get("width_mm").and_then(Value::as_float);
    let height_mm = opts.get("height_mm").and_then(Value::as_float);
    match (width_mm, height_mm) {
        (None, None) => {}
        (Some(width_mm), Some(height_mm))
            if width_mm.is_finite()
                && height_mm.is_finite()
                && width_mm > 0.0
                && height_mm > 0.0 =>
        {
            let width_pattern = regex::Regex::new(r#"\bwidth="[^"]+""#).unwrap();
            let height_pattern = regex::Regex::new(r#"\bheight="[^"]+""#).unwrap();
            output = width_pattern
                .replacen(&output, 1, format!("width=\"{width_mm}mm\""))
                .into_owned();
            output = height_pattern
                .replacen(&output, 1, format!("height=\"{height_mm}mm\""))
                .into_owned();
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "save_svg() width_mm and height_mm must be supplied together as positive numbers",
                None,
            ))
        }
    }
    std::fs::write(path, output).map_err(|e| {
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
/// `scale` multiplies the pixel dimensions without changing the drawing. The
/// equivalent `dpi` option uses the SVG/CSS baseline of 96 dpi, so `{dpi: 300}`
/// produces exactly 300/96 times the source pixels. Give one, not both.
///
/// The figure is the same size in inches and simply carries more pixels, which is
/// what a journal asking for 300 dpi wants. Default 2, because a 1x raster of a
/// 600-point figure looks soft on any modern display.
fn builtin_save_png(args: Vec<Value>) -> Result<Value> {
    let svg = match &args[0] {
        Value::Str(s) => s.to_string(),
        Value::Record(_) => match builtin_render_plot(vec![args[0].clone()])? {
            Value::Str(svg) if svg.trim_start().starts_with("<svg") => svg.to_string(),
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "save_png() rendered PlotSpec as {}, expected SVG",
                        other.type_of()
                    ),
                    None,
                ))
            }
        },
        Value::Nil => {
            return Err(BioLangError::type_error(
                "save_png() received Nil — the plot function before the pipe likely failed or returned nothing".to_string(),
                None,
            ))
        }
        other => {
            return Err(BioLangError::type_error(
                format!("save_png() requires SVG Str or PlotSpec Record, got {}", other.type_of()),
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
    if opts.contains_key("scale") && opts.contains_key("dpi") {
        return Err(BioLangError::type_error(
            "save_png() accepts either scale or dpi, not both",
            None,
        ));
    }
    let scale = match opts.get("dpi") {
        Some(value) => {
            let dpi = value.as_float().unwrap_or(f64::NAN);
            if !dpi.is_finite() || dpi <= 0.0 {
                return Err(BioLangError::type_error(
                    format!("save_png() dpi must be a positive number, got {dpi}"),
                    None,
                ));
            }
            dpi / 96.0
        }
        None => get_opt_f64(&opts, "scale", 2.0),
    };
    if !(scale.is_finite() && scale > 0.0) {
        return Err(BioLangError::type_error(
            format!("save_png() scale must be a positive number, got {scale}"),
            None,
        ));
    }

    render_png(&svg, &path, scale)?;
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

/// Character set used when an SVG plot is previewed in a terminal.
///
/// Braille keeps two-by-four subpixels in each character and is the most useful
/// interactive preview. ASCII is lower resolution but survives restricted
/// terminals and plain-text logs.
#[cfg(feature = "native")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalPlotStyle {
    Braille,
    Ascii,
}

#[cfg(feature = "native")]
fn svg_font_database() -> std::sync::Arc<resvg::usvg::fontdb::Database> {
    use resvg::usvg;

    // Both PNG export and terminal previews use this cache. Loading the system
    // font database for every plot makes a quick REPL preview feel needlessly
    // slow and can also make two render paths choose different fallback fonts.
    static FONTS: std::sync::OnceLock<std::sync::Arc<usvg::fontdb::Database>> =
        std::sync::OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = usvg::fontdb::Database::new();
            db.load_system_fonts();
            configure_generic_font_families(&mut db);
            std::sync::Arc::new(db)
        })
        .clone()
}

/// Rasterise a complete SVG document into a compact terminal preview.
///
/// This deliberately consumes the SVG that every plot builtin already
/// produces. It therefore cannot disagree with the saved figure about scales,
/// points, labels, or clipping. The result contains no ANSI escapes, so callers
/// may safely colour it or place it in a plain-text log.
#[cfg(feature = "native")]
pub fn render_svg_terminal(
    svg: &str,
    columns: usize,
    max_rows: usize,
    style: TerminalPlotStyle,
) -> std::result::Result<String, String> {
    use resvg::{tiny_skia, usvg};

    let options = usvg::Options {
        fontdb: svg_font_database(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|error| format!("could not parse SVG: {error}"))?;
    let size = tree.size();
    let source_width = size.width().max(1.0);
    let source_height = size.height().max(1.0);

    let columns = columns.clamp(12, 160);
    let max_rows = max_rows.clamp(4, 60);
    // One character covers a 2x4 pixel cell in either style: Braille encodes
    // exactly that grid, and matching it for ASCII keeps both previews the same
    // shape at the same requested width.
    let cell_width = 2usize;
    let cell_height = 4usize;
    let target_width = (columns * cell_width) as f32;
    let target_height = (max_rows * cell_height) as f32;
    let scale = (target_width / source_width)
        .min(target_height / source_height)
        .max(0.001);
    let pixel_width = (source_width * scale).ceil().max(1.0) as u32;
    let pixel_height = (source_height * scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(pixel_width, pixel_height)
        .ok_or_else(|| "could not allocate terminal plot raster".to_string())?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // resvg stores premultiplied RGBA. Composite each channel over white before
    // measuring darkness so an untouched transparent pixel remains blank.
    let ink_at = |x: u32, y: u32| -> f32 {
        let Some(pixel) = pixmap.pixel(x, y) else {
            return 0.0;
        };
        let transparent = 255u16.saturating_sub(pixel.alpha() as u16);
        let red = (pixel.red() as u16 + transparent).min(255) as f32;
        let green = (pixel.green() as u16 + transparent).min(255) as f32;
        let blue = (pixel.blue() as u16 + transparent).min(255) as f32;
        255.0 - (0.2126 * red + 0.7152 * green + 0.0722 * blue)
    };

    let output_columns = (pixel_width as usize).div_ceil(cell_width);
    let output_rows = (pixel_height as usize).div_ceil(cell_height);
    let mut lines = Vec::with_capacity(output_rows);
    for row in 0..output_rows {
        let mut line = String::with_capacity(output_columns);
        for column in 0..output_columns {
            let x0 = column * cell_width;
            let y0 = row * cell_height;
            match style {
                TerminalPlotStyle::Braille => {
                    const DOTS: [[u8; 2]; 4] =
                        [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
                    let mut bits = 0u8;
                    for dy in 0..cell_height {
                        for dx in 0..cell_width {
                            if ink_at((x0 + dx) as u32, (y0 + dy) as u32) >= 28.0 {
                                bits |= DOTS[dy][dx];
                            }
                        }
                    }
                    line.push(char::from_u32(0x2800 + bits as u32).unwrap_or(' '));
                }
                TerminalPlotStyle::Ascii => {
                    const LEVELS: &[u8] = b" .:-=+*#%@";
                    let mut total = 0.0f32;
                    let mut peak = 0.0f32;
                    for dy in 0..cell_height {
                        for dx in 0..cell_width {
                            let ink = ink_at((x0 + dx) as u32, (y0 + dy) as u32);
                            total += ink;
                            peak = peak.max(ink);
                        }
                    }
                    let average = total / (cell_width * cell_height) as f32;
                    // Thin axes and lines would disappear under a pure average;
                    // retain their peak while still giving solid areas weight.
                    let density = (0.65 * peak + 0.35 * average) / 255.0;
                    let index = (density * (LEVELS.len() - 1) as f32).round() as usize;
                    line.push(LEVELS[index.min(LEVELS.len() - 1)] as char);
                }
            }
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return Err("SVG rendered as an empty terminal preview".to_string());
    }
    Ok(lines.join("\n"))
}

#[cfg(feature = "native")]
fn render_png(svg: &str, path: &str, scale: f64) -> Result<()> {
    use resvg::{tiny_skia, usvg};

    let png_error = |message: String| BioLangError::runtime(ErrorKind::IOError, message, None);

    let options = usvg::Options {
        fontdb: svg_font_database(),
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
    crate::bio_plots::builtin_genome_track(args)
}

#[cfg(test)]
mod palette_tests {
    use super::{estimate_text_width, PlotTheme, Scale, SvgCanvas, PALETTE};
    #[cfg(feature = "native")]
    use super::{render_svg_terminal, TerminalPlotStyle};
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

    #[cfg(feature = "native")]
    #[test]
    fn terminal_renderers_turn_svg_into_text_without_leaking_markup() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect width="120" height="80" fill="white"/><line x1="5" y1="70" x2="115" y2="10" stroke="black" stroke-width="5"/></svg>"#;
        for style in [TerminalPlotStyle::Braille, TerminalPlotStyle::Ascii] {
            let preview = render_svg_terminal(svg, 40, 12, style).expect("terminal preview");
            assert!(!preview.trim().is_empty());
            assert!(!preview.contains("<svg"));
            assert!(preview.lines().count() <= 12);
            assert!(preview.lines().all(|line| line.chars().count() <= 40));
        }
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
    fn axis_ticks_use_readable_one_two_five_steps() {
        let ticks = Scale {
            domain: (0.0, 7.0),
            range: (0.0, 700.0),
        }
        .nice_ticks(5);
        assert_eq!(ticks, vec![0.0, 2.0, 4.0, 6.0]);

        let crossing_zero = Scale {
            domain: (-3.0, 7.0),
            range: (0.0, 700.0),
        }
        .nice_ticks(5);
        assert_eq!(crossing_zero, vec![-2.0, 0.0, 2.0, 4.0, 6.0]);
    }

    #[test]
    fn axis_ticks_handle_small_constant_and_reversed_domains() {
        let small = Scale {
            domain: (0.0012, 0.0019),
            range: (0.0, 1.0),
        }
        .nice_ticks(5);
        assert_eq!(small.len(), 4);
        for (actual, expected) in small.iter().zip([0.0012, 0.0014, 0.0016, 0.0018]) {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert_eq!(
            Scale {
                domain: (3.0, 3.0),
                range: (0.0, 1.0)
            }
            .nice_ticks(5),
            vec![3.0]
        );
        assert_eq!(
            Scale {
                domain: (7.0, 0.0),
                range: (0.0, 1.0)
            }
            .nice_ticks(5),
            vec![6.0, 4.0, 2.0, 0.0]
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
        canvas.set_accessible_description("Values < reference & finite");

        let svg = canvas.render();
        assert!(svg.contains("role=\"img\""));
        assert!(svg.contains("focusable=\"false\""));
        assert!(svg.contains("aria-label=\"A &amp; B\""));
        assert!(svg.contains("<title>A &amp; B</title>"));
        assert!(svg.contains("<desc>Values &lt; reference &amp; finite</desc>"));
    }

    #[test]
    fn rendered_svg_has_a_default_accessible_label() {
        let canvas = SvgCanvas::new(320.0, 180.0);
        let svg = canvas.render();
        assert!(svg.contains("aria-label=\"BioLang plot\""));
        assert!(svg.contains("<title>BioLang plot</title>"));
        assert!(svg.contains("<desc>BioLang data visualization.</desc>"));
    }

    #[test]
    fn publication_theme_is_opt_in_and_structurally_identified() {
        let legacy = SvgCanvas::new(320.0, 180.0).render();
        let publication =
            SvgCanvas::with_theme(320.0, 180.0, PlotTheme::from_name("publication")).render();
        assert!(legacy.contains("data-biolang-theme=\"biolang\""));
        assert!(publication.contains("data-biolang-theme=\"publication\""));
        assert!(!legacy.contains("Arial, Helvetica"));
    }

    #[test]
    fn adaptive_layout_reserves_room_for_wide_tick_labels() {
        let theme = PlotTheme::from_name("publication");
        let mut short = SvgCanvas::with_theme(500.0, 320.0, theme);
        short.fit_cartesian_layout(&[0.0, 1.0], &[0.0, 1.0], "x", "y", "t", "", "", 0.0);
        let mut wide = SvgCanvas::with_theme(500.0, 320.0, theme);
        wide.fit_cartesian_layout(
            &[0.0, 1.0],
            &[10_000_000.0, 90_000_000.0],
            "x",
            "y",
            "t",
            "",
            "",
            0.0,
        );
        assert!(wide.margin.left > short.margin.left);
    }

    #[test]
    fn text_measurement_distinguishes_narrow_and_wide_labels() {
        assert!(estimate_text_width("MMMM", 12.0) > estimate_text_width("iiii", 12.0) * 2.0);
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

#[cfg(test)]
mod thinning_tests {
    use super::thin_to_pixel_grid;

    // The whole value of thinning rests on one promise: it removes overdraw,
    // never coverage. These check that promise directly, because the failure
    // mode -- a variant quietly disappearing from a GWAS figure -- is invisible
    // in the rendered output.

    const AREA: (f64, f64, f64, f64) = (0.0, 0.0, 100.0, 100.0);

    #[test]
    fn points_in_distinct_pixels_all_survive() {
        let points = [(1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (1.0, 2.0)];
        let rank = [0.0; 4];
        assert_eq!(
            thin_to_pixel_grid(&points, AREA, 1.0, &rank),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn a_shared_pixel_keeps_the_highest_rank() {
        // All four land in pixel (5, 5) at scale 1; only the strongest signal
        // is worth the pixel, and in a Manhattan plot that is the smallest p.
        let points = [(5.1, 5.1), (5.9, 5.2), (5.4, 5.7), (5.2, 5.5)];
        let rank = [1.0, 9.0, 3.0, 2.0];
        assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank), vec![1]);
    }

    #[test]
    fn ties_resolve_by_input_order_not_hash_order() {
        // Equal ranks must not leave the survivor up to HashMap iteration, or
        // the same data would render differently between runs.
        let points = [(5.1, 5.1), (5.6, 5.6), (5.8, 5.2)];
        let rank = [4.0, 4.0, 4.0];
        for _ in 0..64 {
            assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank), vec![0]);
        }
    }

    #[test]
    fn a_finer_grid_keeps_more_points() {
        // Same data, four times the pixels: separations too small to see at
        // scale 1 become visible at scale 4, so nothing is merged.
        let points = [(5.05, 5.05), (5.55, 5.55), (5.80, 5.30)];
        let rank = [1.0, 2.0, 3.0];
        assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank).len(), 1);
        assert_eq!(thin_to_pixel_grid(&points, AREA, 4.0, &rank).len(), 3);
    }

    #[test]
    fn the_area_origin_is_subtracted_before_gridding() {
        // Cells are measured from the plot area, not the page. An offset panel
        // must thin the same way an unoffset one does.
        let flush = [(0.2, 0.2), (0.7, 0.7), (1.4, 0.3)];
        let offset: Vec<(f64, f64)> = flush.iter().map(|(x, y)| (x + 60.0, y + 40.0)).collect();
        let rank = [1.0, 2.0, 3.0];
        assert_eq!(
            thin_to_pixel_grid(&flush, (0.0, 0.0, 100.0, 100.0), 1.0, &rank),
            thin_to_pixel_grid(&offset, (60.0, 40.0, 100.0, 100.0), 1.0, &rank)
        );
    }

    #[test]
    fn non_finite_coordinates_are_dropped_rather_than_gridded() {
        // NaN would floor to a garbage cell and could evict a real point.
        let points = [(f64::NAN, 1.0), (1.0, f64::INFINITY), (2.0, 2.0)];
        let rank = [9.0, 9.0, 0.0];
        assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank), vec![2]);
    }

    #[test]
    fn every_occupied_pixel_still_gets_a_point() {
        // The contract in one assertion: the set of occupied cells before and
        // after thinning is identical, so no pixel goes unpainted.
        use std::collections::HashSet;
        let points: Vec<(f64, f64)> = (0..5000)
            .map(|i| {
                let f = i as f64;
                ((f * 0.037) % 100.0, (f * 0.611) % 100.0)
            })
            .collect();
        let rank: Vec<f64> = (0..5000).map(|i| (i % 97) as f64).collect();
        let cell = |&(x, y): &(f64, f64)| (x.floor() as i64, y.floor() as i64);
        let before: HashSet<(i64, i64)> = points.iter().map(cell).collect();
        let kept = thin_to_pixel_grid(&points, AREA, 1.0, &rank);
        let after: HashSet<(i64, i64)> = kept.iter().map(|&i| cell(&points[i])).collect();
        assert_eq!(before, after);
        assert!(kept.len() < points.len(), "nothing was thinned at all");
    }
}

#[cfg(test)]
mod hue_palette_tests {
    use super::hue_palette;

    /// Reference values printed by R: `scales::hue_pal()(n)`.
    ///
    /// A fixed table is only right at the one `n` it was copied from, which is
    /// how a two-group plot could look correct while every other count was
    /// silently off-palette.
    #[test]
    fn hue_palette_matches_r_scales_hue_pal() {
        let expected: [&[&str]; 5] = [
            &["#f8766d", "#00bfc4"],
            &["#f8766d", "#00ba38", "#619cff"],
            &["#f8766d", "#7cae00", "#00bfc4", "#c77cff"],
            &["#f8766d", "#a3a500", "#00bf7d", "#00b0f6", "#e76bf3"],
            &[
                "#f8766d", "#b79f00", "#00ba38", "#00bfc4", "#619cff", "#f564e3",
            ],
        ];
        for (index, reference) in expected.iter().enumerate() {
            let count = index + 2;
            assert_eq!(
                hue_palette(count),
                *reference,
                "hue_pal({count}) disagrees with R"
            );
        }
    }

    #[test]
    fn hue_palette_always_returns_one_colour_per_group() {
        for count in 1..=24 {
            let palette = hue_palette(count);
            assert_eq!(palette.len(), count);
            assert!(palette
                .iter()
                .all(|colour| colour.len() == 7 && colour.starts_with('#')));
        }
    }
}

#[cfg(test)]
mod ggplot_binning_tests {
    use super::{histogram_equal_edges, histogram_ggplot_edges};

    /// `bins = n` means different things in ggplot2 and in matplotlib.
    ///
    /// ggplot2 centres the first bin on the minimum with width range/(n-1);
    /// an equal split of [min, max] uses width range/n starting at the
    /// minimum. Same `bins`, different bars.
    #[test]
    fn ggplot_bins_are_not_an_equal_split_of_the_range() {
        let values: Vec<f64> = (0..100).map(|value| value as f64).collect();
        let ggplot = histogram_ggplot_edges(&values, 30);
        let span = histogram_equal_edges(&values, 30);
        assert_eq!(ggplot.len(), 31, "ggplot rule must still yield 30 bins");
        assert_eq!(span.len(), 31);

        let ggplot_width = ggplot[1] - ggplot[0];
        assert!(
            (ggplot_width - 99.0 / 29.0).abs() < 1e-9,
            "width must be range/(bins - 1), got {ggplot_width}"
        );
        assert!(
            ggplot[0] < 0.0,
            "the first edge sits half a bin below the minimum, got {}",
            ggplot[0]
        );
        assert!(
            *ggplot.last().unwrap() > 99.0,
            "the last edge sits above the maximum"
        );
        assert!(
            span[0].abs() < 1e-9 && (span[30] - 99.0).abs() < 1e-9,
            "the span rule still runs edge to edge"
        );
    }

    #[test]
    fn ggplot_edges_are_evenly_spaced_and_cover_the_data() {
        let values = [12.88_f64, 20.1, 33.4, 47.9, 81.25];
        let edges = histogram_ggplot_edges(&values, 30);
        let width = edges[1] - edges[0];
        for pair in edges.windows(2) {
            assert!((pair[1] - pair[0] - width).abs() < 1e-9, "uneven bin width");
        }
        assert!(edges[0] <= 12.88 && *edges.last().unwrap() >= 81.25);
    }
}

#[cfg(test)]
mod text_metric_tests {
    use super::estimate_text_width;

    /// Advance widths straight out of Arial's `hmtx` table, which matches the
    /// Helvetica AFM values these three faces have shared since PostScript.
    #[test]
    fn widths_match_the_published_font_metrics() {
        for (character, per_mille) in [
            (' ', 278.0),
            ('.', 278.0),
            ('i', 222.0),
            ('m', 833.0),
            ('0', 556.0),
            ('A', 667.0),
            ('W', 944.0),
        ] {
            let width = estimate_text_width(&character.to_string(), 1000.0);
            assert!(
                (width - per_mille).abs() < 0.5,
                "{character:?} should advance {per_mille} per em, got {width}"
            );
        }
    }

    #[test]
    fn a_string_is_the_sum_of_its_glyphs_and_scales_with_size() {
        let label = "Height (cm)";
        let at_ten = estimate_text_width(label, 10.0);
        let at_twenty = estimate_text_width(label, 20.0);
        assert!(
            (at_twenty - 2.0 * at_ten).abs() < 1e-9,
            "width must be linear in size"
        );
        // 5.167 em from the real table; the character-class rule said 5.94.
        assert!(
            (at_ten - 51.67).abs() < 0.05,
            "expected 51.67px at size 10, got {at_ten}"
        );
    }

    #[test]
    fn characters_outside_the_table_still_get_a_width() {
        assert!(estimate_text_width("\u{4e2d}\u{6587}", 10.0) > 0.0);
        assert_eq!(
            estimate_text_width("\u{7}", 10.0),
            0.0,
            "control characters take no space"
        );
    }
}
