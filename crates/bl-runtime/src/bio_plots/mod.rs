use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

use crate::builtins::write_output;
use crate::plot::{
    col_range, estimate_text_width, extract_table_col, gaussian_kde, get_opt_f64, get_opt_str,
    parse_options, plot_theme, publication_diverging_color, publication_sequential_color,
    quantile_type7, raster_choice, sequential_color, seurat_feature_color, silverman_bandwidth,
    thin_requested, thin_to_pixel_grid, PlotTheme, PlotThemeKind, Scale, SvgCanvas, PALETTE,
    SEURAT_PALETTE,
};
use crate::viz::{get_opt_usize, nums_from_value};

mod genomic;
pub(crate) use genomic::*;

mod clinical;
pub(crate) use clinical::*;

mod circos;
pub(crate) use circos::*;

mod tracks;
pub(crate) use tracks::*;

mod expression;
pub(crate) use expression::*;

mod distribution;
pub(crate) use distribution::*;

mod embedding;
pub(crate) use embedding::*;

mod sets;
pub(crate) use sets::*;

/// Start a legacy biological plot on the same presentation layer used by the
/// frozen plot specifications.  Several older renderers accepted `theme` but
/// constructed a legacy canvas, so the option was silently ignored.
fn themed_canvas(width: f64, height: f64, opts: &HashMap<String, Value>) -> SvgCanvas {
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    if canvas.theme.is_adaptive() {
        canvas.margin.top = if get_opt_str(opts, "subtitle", "").is_empty() {
            48.0
        } else {
            66.0
        };
        if !get_opt_str(opts, "caption", "").is_empty() {
            canvas.margin.bottom += 16.0;
        }
    }
    canvas
}

fn finish_themed_canvas(
    canvas: &mut SvgCanvas,
    opts: &HashMap<String, Value>,
    default_title: &str,
) {
    canvas.draw_title(get_opt_str(opts, "title", default_title));
    canvas.draw_subtitle(get_opt_str(opts, "subtitle", ""));
    canvas.draw_caption(get_opt_str(opts, "caption", ""));
}

// ── Registration ────────────────────────────────────────────────

pub fn bio_plots_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("manhattan", Arity::Range(1, 2)),
        ("qq_plot", Arity::Range(1, 2)),
        ("ideogram", Arity::Range(1, 2)),
        ("rainfall", Arity::Range(1, 2)),
        ("cnv_plot", Arity::Range(1, 2)),
        ("violin", Arity::Range(1, 2)),
        ("density", Arity::Range(1, 2)),
        ("kaplan_meier", Arity::Range(1, 2)),
        ("forest_plot", Arity::Range(1, 2)),
        ("roc_curve", Arity::Range(1, 2)),
        ("clustered_heatmap", Arity::Range(1, 2)),
        ("pca_plot", Arity::Range(1, 2)),
        ("oncoprint", Arity::Range(1, 2)),
        ("venn", Arity::Range(1, 2)),
        ("upset", Arity::Range(1, 2)),
        ("sequence_logo", Arity::Range(1, 2)),
        ("phylo_tree", Arity::Range(1, 2)),
        ("lollipop", Arity::Range(1, 2)),
        ("circos", Arity::Range(1, 2)),
        ("hic_map", Arity::Range(1, 2)),
        ("sashimi", Arity::Range(1, 2)),
        ("volcano_plot", Arity::Range(1, 2)),
        ("upset_plot", Arity::Range(1, 2)),
        ("alignment_view", Arity::Range(1, 2)),
        ("circos_plot", Arity::Range(1, 2)),
        ("umap_plot", Arity::Range(1, 2)),
        ("feature_plot", Arity::Range(1, 2)),
        ("elbow_plot", Arity::Range(1, 2)),
        ("violin_plot", Arity::Range(1, 2)),
        ("variable_feature_plot", Arity::Range(1, 2)),
        ("dot_plot", Arity::Range(2, 3)),
        ("coverage_track", Arity::Range(1, 2)),
    ]
}

pub fn is_bio_plots_builtin(name: &str) -> bool {
    matches!(
        name,
        "manhattan"
            | "qq_plot"
            | "ideogram"
            | "rainfall"
            | "cnv_plot"
            | "violin"
            | "density"
            | "kaplan_meier"
            | "forest_plot"
            | "roc_curve"
            | "clustered_heatmap"
            | "pca_plot"
            | "oncoprint"
            | "venn"
            | "upset"
            | "sequence_logo"
            | "phylo_tree"
            | "lollipop"
            | "circos"
            | "hic_map"
            | "sashimi"
            | "volcano_plot"
            | "upset_plot"
            | "alignment_view"
            | "circos_plot"
            | "umap_plot"
            | "feature_plot"
            | "elbow_plot"
            | "violin_plot"
            | "variable_feature_plot"
            | "dot_plot"
            | "coverage_track"
    )
}

/// Normalize single-Record-with-`data` calling convention:
///   `func({data: table, title: "..."})` → `func(table, {title: "..."})`
/// This lets all bio plot functions accept both:
///   `manhattan(table, {title: "..."})` and `manhattan({data: table, title: "..."})`
fn normalize_data_args(args: Vec<Value>) -> Vec<Value> {
    if args.len() == 1 {
        if let Value::Record(ref map) = args[0] {
            if let Some(data) = map.get("data") {
                let mut opts = map.as_ref().clone();
                opts.remove("data");
                if opts.is_empty() {
                    return vec![data.clone()];
                }
                return vec![data.clone(), Value::Record(opts.into())];
            }
        }
    }
    args
}

pub fn call_bio_plots_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    let args = normalize_data_args(args);
    match name {
        "manhattan" => builtin_manhattan(args),
        "qq_plot" => builtin_qq_plot(args),
        "ideogram" => builtin_ideogram(args),
        "rainfall" => builtin_rainfall(args),
        "cnv_plot" => builtin_cnv_plot(args),
        "violin" => builtin_violin(args),
        "density" => builtin_density(args),
        "kaplan_meier" => builtin_kaplan_meier(args),
        "forest_plot" => builtin_forest_plot(args),
        "roc_curve" => builtin_roc_curve(args),
        "clustered_heatmap" => builtin_clustered_heatmap(args),
        "pca_plot" => builtin_pca_plot(args),
        "oncoprint" => builtin_oncoprint(args),
        "venn" => builtin_venn(args),
        "upset" => builtin_upset(args),
        "sequence_logo" => builtin_sequence_logo(args),
        "phylo_tree" => builtin_phylo_tree(args),
        "lollipop" => builtin_lollipop(args),
        "circos" => builtin_circos(args),
        "hic_map" => builtin_hic_map(args),
        "sashimi" => builtin_sashimi(args),
        "volcano_plot" => builtin_volcano_plot(args),
        "upset_plot" => builtin_upset_plot(args),
        "alignment_view" => builtin_alignment_view(args),
        "circos_plot" => builtin_circos_plot(args),
        // Same renderer: feature_plot is umap_plot with a continuous colour
        // scale, so they cannot drift apart.
        "umap_plot" | "feature_plot" => builtin_umap_plot(args),
        "elbow_plot" => builtin_elbow_plot(args),
        "violin_plot" => builtin_violin_plot(args),
        "variable_feature_plot" => builtin_variable_feature_plot(args),
        "dot_plot" => builtin_dot_plot(args),
        "coverage_track" => builtin_coverage_track(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown bio_plots builtin '{name}'"),
            None,
        )),
    }
}

// ── Shared Helpers ──────────────────────────────────────────────

struct AsciiChart {
    grid: Vec<Vec<char>>,
    w: usize,
    h: usize,
    ml: usize,
    mb: usize,
}

impl AsciiChart {
    fn new(w: usize, h: usize) -> Self {
        let ml = 8;
        let mb = 2;
        let mut grid = vec![vec![' '; w]; h];
        for y in 0..(h - mb) {
            grid[y][ml - 1] = '│';
        }
        for x in ml..w {
            grid[h - mb][x] = '─';
        }
        grid[h - mb][ml - 1] = '└';
        Self { grid, w, h, ml, mb }
    }

    fn pw(&self) -> usize {
        self.w - self.ml
    }
    fn ph(&self) -> usize {
        self.h - self.mb
    }

    fn map(&self, x: f64, y: f64, xr: (f64, f64), yr: (f64, f64)) -> (usize, usize) {
        let tx = if (xr.1 - xr.0).abs() < f64::EPSILON {
            0.5
        } else {
            (x - xr.0) / (xr.1 - xr.0)
        };
        let ty = if (yr.1 - yr.0).abs() < f64::EPSILON {
            0.5
        } else {
            (y - yr.0) / (yr.1 - yr.0)
        };
        let gx = self.ml
            + (tx * (self.pw() - 1) as f64)
                .round()
                .clamp(0.0, (self.pw() - 1) as f64) as usize;
        let gy = (self.ph() - 1)
            - (ty * (self.ph() - 1) as f64)
                .round()
                .clamp(0.0, (self.ph() - 1) as f64) as usize;
        (gx, gy)
    }

    fn put(&mut self, x: f64, y: f64, xr: (f64, f64), yr: (f64, f64), ch: char) {
        let (gx, gy) = self.map(x, y, xr, yr);
        if gx < self.w && gy < self.h {
            self.grid[gy][gx] = ch;
        }
    }

    fn hline(&mut self, y: f64, yr: (f64, f64), ch: char) {
        let (_, gy) = self.map(0.0, y, (0.0, 1.0), yr);
        for x in self.ml..self.w {
            if self.grid[gy][x] == ' ' || self.grid[gy][x] == '─' {
                self.grid[gy][x] = ch;
            }
        }
    }

    fn render(&self, title: &str) -> String {
        let mut out = format!("  {title}\n");
        for row in &self.grid {
            out.push_str("  ");
            out.push_str(&row.iter().collect::<String>());
            out.push('\n');
        }
        out
    }
}

fn extract_str_col(table: &Table, col: &str) -> Result<Vec<String>> {
    let idx = table.col_index(col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{col}' not found"),
            None,
        )
    })?;
    Ok(table
        .rows
        .iter()
        .map(|row| match &row[idx] {
            Value::Str(s) => s.clone(),
            other => format!("{other}"),
        })
        .collect())
}

fn require_table_bp<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table, got {}", other.type_of()),
            None,
        )),
    }
}

fn kde(data: &[f64], bw: f64, n: usize) -> (Vec<f64>, Vec<f64>) {
    gaussian_kde(data, bw, n).into_iter().unzip()
}

fn silverman_bw(data: &[f64]) -> f64 {
    silverman_bandwidth(data)
}

fn trapz_auc(xs: &[f64], ys: &[f64]) -> f64 {
    xs.windows(2)
        .zip(ys.windows(2))
        .map(|(xw, yw)| (xw[1] - xw[0]).abs() * (yw[0] + yw[1]) / 2.0)
        .sum()
}

#[derive(Clone, Debug)]
struct ChromosomeSpan {
    index: usize,
    name: String,
    offset: f64,
    length: f64,
}

impl ChromosomeSpan {
    fn start(&self) -> f64 {
        self.offset
    }

    fn end(&self) -> f64 {
        self.offset + self.length
    }

    fn midpoint(&self) -> f64 {
        self.offset + self.length / 2.0
    }
}

/// Build a deterministic first-observed chromosome layout.
///
/// Lexical sorting places chromosome 10 before chromosome 2 and makes figures
/// depend on naming conventions. First-observed order works for human, model
/// organism and contig inputs alike, while the frozen chromosome table records
/// that decision for exact replay.
fn checked_genome_layout(
    chroms: &[String],
    positions: &[f64],
    builtin: &str,
) -> Result<(Vec<f64>, Vec<ChromosomeSpan>)> {
    if chroms.is_empty() || chroms.len() != positions.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{builtin}() requires equally sized, non-empty chromosome and position columns"
            ),
            None,
        ));
    }
    let mut order = Vec::<String>::new();
    let mut maximum = HashMap::<String, f64>::new();
    for (chromosome, &position) in chroms.iter().zip(positions) {
        if chromosome.trim().is_empty() || !position.is_finite() || position < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{builtin}() chromosomes must be non-empty and positions finite and non-negative"),
                None,
            ));
        }
        if !maximum.contains_key(chromosome) {
            order.push(chromosome.clone());
        }
        maximum
            .entry(chromosome.clone())
            .and_modify(|value| *value = value.max(position))
            .or_insert(position);
    }
    let mut spans = Vec::with_capacity(order.len());
    let mut offset = 0.0;
    for (index, chromosome) in order.into_iter().enumerate() {
        let length = maximum[&chromosome].max(1.0);
        spans.push(ChromosomeSpan {
            index,
            name: chromosome,
            offset,
            length,
        });
        offset += length * 1.02;
    }
    let lookup = spans
        .iter()
        .map(|span| (span.name.clone(), span))
        .collect::<HashMap<_, _>>();
    let genome_positions = chroms
        .iter()
        .zip(positions)
        .map(|(chromosome, position)| lookup[chromosome].offset + position)
        .collect();
    Ok((genome_positions, spans))
}

fn chromosome_table(spans: &[ChromosomeSpan]) -> Value {
    Value::Table(Table::new(
        [
            "chromosome_index",
            "chromosome",
            "offset",
            "length",
            "start",
            "end",
            "midpoint",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        spans
            .iter()
            .map(|span| {
                vec![
                    Value::Int(span.index as i64),
                    Value::Str(span.name.clone()),
                    Value::Float(span.offset),
                    Value::Float(span.length),
                    Value::Float(span.start()),
                    Value::Float(span.end()),
                    Value::Float(span.midpoint()),
                ]
            })
            .collect(),
    ))
}

fn draw_genome_axis(
    canvas: &mut SvgCanvas,
    spans: &[ChromosomeSpan],
    domain: (f64, f64),
    label: &str,
) {
    let y = canvas.margin.top + canvas.plot_height();
    canvas.add_line(
        canvas.margin.left,
        y,
        canvas.margin.left + canvas.plot_width(),
        y,
        canvas.theme.axis_colour,
        canvas.theme.axis_width,
    );
    let scale = Scale {
        domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let available = canvas.plot_width().max(1.0);
    let step = (spans.len() as f64 * 38.0 / available).ceil().max(1.0) as usize;
    for span in spans.iter().step_by(step) {
        let x = scale.map(span.midpoint());
        canvas.add_text(x, y + 18.0, &span.name, "middle", canvas.theme.tick_size);
    }
    if !label.is_empty() {
        canvas.add_text(
            canvas.margin.left + canvas.plot_width() / 2.0,
            if canvas.theme.is_adaptive() {
                y + 36.0
            } else {
                canvas.height - 5.0
            },
            label,
            "middle",
            canvas.theme.axis_title_size,
        );
    }
}

fn beta_quantile(probability: f64, alpha: f64, beta: f64) -> f64 {
    if probability <= 0.0 {
        return 0.0;
    }
    if probability >= 1.0 {
        return 1.0;
    }
    let mut lower = 0.0;
    let mut upper = 1.0;
    for _ in 0..80 {
        let midpoint = (lower + upper) / 2.0;
        let cdf = bl_core::bio_core::stats_ops::regularized_incomplete_beta(midpoint, alpha, beta);
        if cdf < probability {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    (lower + upper) / 2.0
}

fn bio_plot_bool_option(
    opts: &HashMap<String, Value>,
    key: &str,
    default: bool,
    builtin: &str,
) -> Result<bool> {
    match opts.get(key) {
        None => Ok(default),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{builtin}() option '{key}' must be Bool"),
            None,
        )),
    }
}

// ── 1. manhattan ────────────────────────────────────────────────

fn finish_frozen_bio_plot(
    specification: &Value,
    render_options: &HashMap<String, Value>,
    title: &str,
    family: &str,
    svg: String,
) -> Result<Value> {
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    match format.as_str() {
        "spec" | "data" => Ok(specification.clone()),
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(crate::plot::standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => crate::plot::render_svg_terminal(
            &svg,
            80,
            24,
            crate::plot::TerminalPlotStyle::Ascii,
        )
        .map(Value::Str)
        .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => crate::plot::render_svg_terminal(
            &svg,
            80,
            24,
            crate::plot::TerminalPlotStyle::Braille,
        )
        .map(Value::Str)
        .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() terminal {family} output needs the native build"),
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown {family} format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn frozen_spec_options(
    specification: &HashMap<String, Value>,
    render_options: &HashMap<String, Value>,
    family: &str,
) -> Result<HashMap<String, Value>> {
    let mut options = match specification.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} specification field 'options' must be Record"),
                None,
            ))
        }
    };
    for key in ["width", "height"] {
        if let Some(value) = render_options.get(key) {
            options.insert(key.into(), value.clone());
        }
    }
    Ok(options)
}

fn frozen_nonnegative_integer(value: &Value, family: &str, field: &str) -> Result<usize> {
    let number = value.as_float().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() {family} field '{field}' must be numeric"),
            None,
        )
    })?;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 || number > usize::MAX as f64 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() {family} field '{field}' must be a non-negative integer"),
            None,
        ));
    }
    Ok(number as usize)
}

fn builtin_volcano_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let fc_cutoff = get_opt_f64(&opts, "fc_cutoff", 1.0);
    let p_cutoff = get_opt_f64(&opts, "p_cutoff", 0.05);
    let title = get_opt_str(&opts, "title", "Volcano Plot").to_string();

    // Extract data from Table or List of Records
    let (fcs, pvals, gene_names) = match &args[0] {
        Value::Table(table) => {
            let fc_col = if table.col_index("log2fc").is_some() {
                "log2fc"
            } else if table.col_index("log2FoldChange").is_some() {
                "log2FoldChange"
            } else {
                "log2fc"
            };
            let p_col = if table.col_index("pvalue").is_some() {
                "pvalue"
            } else if table.col_index("padj").is_some() {
                "padj"
            } else {
                "pvalue"
            };
            let fcs = extract_table_col(table, fc_col)?;
            let pvals = extract_table_col(table, p_col)?;
            let names = table
                .col_index("gene")
                .or(table.col_index("name"))
                .map(|idx| {
                    table
                        .rows
                        .iter()
                        .map(|r| match &r[idx] {
                            Value::Str(s) => s.clone(),
                            other => format!("{other}"),
                        })
                        .collect::<Vec<_>>()
                });
            (fcs, pvals, names)
        }
        Value::List(items) => {
            let mut fcs = Vec::new();
            let mut pvals = Vec::new();
            let mut names: Vec<String> = Vec::new();
            for item in items.iter() {
                if let Value::Record(map) = item {
                    let fc = map
                        .get("log2fc")
                        .or(map.get("log2FoldChange"))
                        .and_then(|v| v.as_float())
                        .unwrap_or(0.0);
                    let pv = map
                        .get("pvalue")
                        .or(map.get("padj"))
                        .and_then(|v| v.as_float())
                        .unwrap_or(1.0);
                    fcs.push(fc);
                    pvals.push(pv);
                    if let Some(n) = map.get("gene").or(map.get("name")) {
                        names.push(format!("{n}"));
                    } else {
                        names.push(String::new());
                    }
                }
            }
            let names = if names.iter().any(|n| !n.is_empty()) {
                Some(names)
            } else {
                None
            };
            (fcs, pvals, names)
        }
        _ => {
            return Err(BioLangError::type_error(
                "volcano_plot() requires Table or List of Records",
                None,
            ))
        }
    };
    if fcs.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "volcano_plot() empty data",
            None,
        ));
    }

    let neg_log_p: Vec<f64> = pvals
        .iter()
        .map(|&p| if p > 0.0 { -(p.log10()) } else { 0.0 })
        .collect();
    let neg_log_p_cutoff = -(p_cutoff.log10());

    let (x_min, x_max) = col_range(&fcs);
    let x_abs = x_min.abs().max(x_max.abs()) * 1.1;
    let (_, y_max) = col_range(&neg_log_p);
    let yr = (0.0, y_max * 1.1);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = SvgCanvas::new(w, h);
        let x_scale = Scale {
            domain: (-x_abs, x_abs),
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let y_scale = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };

        // Dashed significance lines
        let p_y = y_scale.map(neg_log_p_cutoff);
        c.elements.push(format!(
            r##"<line x1="{:.1}" y1="{p_y:.1}" x2="{:.1}" y2="{p_y:.1}" stroke="#999" stroke-width="1" stroke-dasharray="5,3" />"##,
            c.margin.left, c.margin.left + c.plot_width()
        ));
        let fc_neg_x = x_scale.map(-fc_cutoff);
        let fc_pos_x = x_scale.map(fc_cutoff);
        c.elements.push(format!(
            r##"<line x1="{fc_neg_x:.1}" y1="{:.1}" x2="{fc_neg_x:.1}" y2="{:.1}" stroke="#999" stroke-width="1" stroke-dasharray="5,3" />"##,
            c.margin.top, c.margin.top + c.plot_height()
        ));
        c.elements.push(format!(
            r##"<line x1="{fc_pos_x:.1}" y1="{:.1}" x2="{fc_pos_x:.1}" y2="{:.1}" stroke="#999" stroke-width="1" stroke-dasharray="5,3" />"##,
            c.margin.top, c.margin.top + c.plot_height()
        ));

        // Collect top hits for labeling
        let mut top_hits: Vec<(usize, f64)> = Vec::new();
        let mut points: Vec<(f64, f64, &str)> = Vec::with_capacity(fcs.len());
        for i in 0..fcs.len() {
            let color = if neg_log_p[i] > neg_log_p_cutoff && fcs[i] > fc_cutoff {
                "#e15759"
            } else if neg_log_p[i] > neg_log_p_cutoff && fcs[i] < -fc_cutoff {
                "#4e79a7"
            } else {
                "#999"
            };
            points.push((x_scale.map(fcs[i]), y_scale.map(neg_log_p[i]), color));
            if neg_log_p[i] > neg_log_p_cutoff && fcs[i].abs() > fc_cutoff {
                top_hits.push((i, neg_log_p[i]));
            }
        }
        // One point per gene, so a whole-transcriptome result is tens of
        // thousands. The labels below stay vector; there are only ten.
        let raster = raster_choice(&opts, "volcano_plot", fcs.len())?;
        let area = c.point_area();
        c.add_scatter(&points, 3.0, area, raster);

        // Label top 10 most significant hits
        if let Some(ref names) = gene_names {
            top_hits.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            for &(idx, _) in top_hits.iter().take(10) {
                if !names[idx].is_empty() {
                    c.add_text(
                        x_scale.map(fcs[idx]) + 5.0,
                        y_scale.map(neg_log_p[idx]) - 5.0,
                        &names[idx],
                        "start",
                        8.0,
                    );
                }
            }
        }

        // Legend
        let lx = c.margin.left + c.plot_width() - 90.0;
        c.add_circle(lx, c.margin.top + 10.0, 4.0, "#e15759");
        c.add_text(lx + 8.0, c.margin.top + 14.0, "Up", "start", 9.0);
        c.add_circle(lx, c.margin.top + 24.0, 4.0, "#4e79a7");
        c.add_text(lx + 8.0, c.margin.top + 28.0, "Down", "start", 9.0);
        c.add_circle(lx, c.margin.top + 38.0, 4.0, "#999");
        c.add_text(lx + 8.0, c.margin.top + 42.0, "NS", "start", 9.0);

        let dx = Scale {
            domain: (-x_abs, x_abs),
            range: (-x_abs, x_abs),
        };
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_x_axis(&dx, "log2(Fold Change)");
        c.draw_y_axis(&dy, "-log10(p-value)");
        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    chart.hline(neg_log_p_cutoff, yr, '┄');
    for i in 0..fcs.len() {
        let ch = if neg_log_p[i] > neg_log_p_cutoff && fcs[i].abs() > fc_cutoff {
            if fcs[i] > 0.0 {
                '▲'
            } else {
                '▼'
            }
        } else {
            '·'
        };
        chart.put(fcs[i], neg_log_p[i], (-x_abs, x_abs), yr, ch);
    }
    let n_up = fcs
        .iter()
        .zip(neg_log_p.iter())
        .filter(|(&f, &p)| p > neg_log_p_cutoff && f > fc_cutoff)
        .count();
    let n_down = fcs
        .iter()
        .zip(neg_log_p.iter())
        .filter(|(&f, &p)| p > neg_log_p_cutoff && f < -fc_cutoff)
        .count();
    write_output(&chart.render(&format!("{title}  (up={n_up}, down={n_down})")));
    Ok(Value::Nil)
}

// ── 23. upset_plot ──────────────────────────────────────────────
