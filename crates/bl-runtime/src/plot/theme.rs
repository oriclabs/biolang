//! Theme for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

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
