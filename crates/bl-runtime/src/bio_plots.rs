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

fn manhattan_plot_spec_value(table: &Table, opts: &HashMap<String, Value>) -> Result<Value> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let position_column = get_opt_str(opts, "pos", "pos");
    let p_column = get_opt_str(opts, "p", "pvalue");
    let label_column = opts.get("label").and_then(Value::as_str);
    let highlight_column = opts.get("highlight").and_then(Value::as_str);
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let positions = extract_table_col(table, position_column)?;
    let p_values = extract_table_col(table, p_column)?;
    let threshold = get_opt_f64(opts, "threshold", 5e-8);
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) || threshold == 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "manhattan() threshold must be finite and within (0, 1]",
            None,
        ));
    }
    if p_values
        .iter()
        .any(|p| !p.is_finite() || *p <= 0.0 || *p > 1.0)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "manhattan() p-values must be finite and within (0, 1]",
            None,
        ));
    }
    let (genome_positions, spans) = checked_genome_layout(&chromosomes, &positions, "manhattan")?;
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span.index))
        .collect::<HashMap<_, _>>();
    let label_index = label_column.and_then(|name| table.col_index(name));
    if label_column.is_some() && label_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", label_column.unwrap()),
            None,
        ));
    }
    let highlight_index = highlight_column.and_then(|name| table.col_index(name));
    if highlight_column.is_some() && highlight_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", highlight_column.unwrap()),
            None,
        ));
    }
    let negative_log_p = p_values
        .iter()
        .map(|value| -value.log10())
        .collect::<Vec<_>>();
    let raster = raster_choice(opts, "manhattan", p_values.len())?;
    let thin = thin_requested(opts, "manhattan")?;
    let rows = (0..p_values.len())
        .map(|index| {
            vec![
                Value::Int(index as i64),
                Value::Int(span_lookup[chromosomes[index].as_str()] as i64),
                Value::Str(chromosomes[index].clone()),
                Value::Float(positions[index]),
                Value::Float(genome_positions[index]),
                Value::Float(p_values[index]),
                Value::Float(negative_log_p[index]),
                Value::Bool(p_values[index] <= threshold),
                Value::Bool(
                    highlight_index.is_some_and(|column| table.rows[index][column].is_truthy()),
                ),
                label_index
                    .map(|column| Value::Str(format!("{}", table.rows[index][column])))
                    .unwrap_or(Value::Nil),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Manhattan Plot");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("manhattan".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "position",
                        "genome_position",
                        "p_value",
                        "neg_log10_p",
                        "significant",
                        "highlighted",
                        "label",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 1200.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 400.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Chromosome").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "-log10(p)").into()),
                        ),
                        ("threshold".into(), Value::Float(threshold)),
                        ("threshold_y".into(), Value::Float(-threshold.log10())),
                        ("raster".into(), Value::Bool(raster.enabled)),
                        ("raster_scale".into(), Value::Float(raster.scale)),
                        ("thin".into(), Value::Bool(thin)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("manhattan".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "chromosome_order".into(),
                            Value::Str("first_observed".into()),
                        ),
                        ("chromosome_gap_fraction".into(), Value::Float(0.02)),
                        ("p_transform".into(), Value::Str("-log10".into())),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("position_column".into(), Value::Str(position_column.into())),
                        ("p_column".into(), Value::Str(p_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

fn chromosome_spans_from_spec(
    map: &HashMap<String, Value>,
    family: &str,
) -> Result<Vec<ChromosomeSpan>> {
    let table = match map.get("chromosomes") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} specification field 'chromosomes' must be Table"),
                None,
            ))
        }
    };
    for required in ["chromosome_index", "chromosome", "offset", "length"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} chromosomes are missing '{required}'"),
                None,
            ));
        }
    }
    let index_column = table.col_index("chromosome_index").unwrap();
    let name_column = table.col_index("chromosome").unwrap();
    let offset_column = table.col_index("offset").unwrap();
    let length_column = table.col_index("length").unwrap();
    let mut spans: Vec<ChromosomeSpan> = Vec::with_capacity(table.num_rows());
    for (expected, row) in table.rows.iter().enumerate() {
        let index = frozen_nonnegative_integer(&row[index_column], family, "chromosome_index")?;
        let name = row[name_column].as_str().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} chromosome must be Str"),
                None,
            )
        })?;
        let offset = row[offset_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        if index != expected
            || name.is_empty()
            || !offset.is_finite()
            || !length.is_finite()
            || length <= 0.0
            || (expected == 0 && offset.abs() > 1e-10)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} chromosome layout is malformed"),
                None,
            ));
        }
        if let Some(previous) = spans.last() {
            let expected_offset = previous.offset + previous.length * 1.02;
            if (offset - expected_offset).abs() > 1e-8 * expected_offset.abs().max(1.0) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() {family} chromosome spans must use the frozen 2% gap"),
                    None,
                ));
            }
        }
        for (column, expected_value) in [
            ("start", offset),
            ("end", offset + length),
            ("midpoint", offset + length / 2.0),
        ] {
            if let Some(column_index) = table.col_index(column) {
                let observed = row[column_index].as_float().unwrap_or(f64::NAN);
                if !observed.is_finite()
                    || (observed - expected_value).abs() > 1e-8 * expected_value.abs().max(1.0)
                {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "render_plot() {family} chromosome field '{column}' is inconsistent"
                        ),
                        None,
                    ));
                }
            }
        }
        spans.push(ChromosomeSpan {
            index,
            name: name.into(),
            offset,
            length,
        });
    }
    if spans.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() {family} needs chromosomes"),
            None,
        ));
    }
    Ok(spans)
}

fn render_manhattan_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let genome_positions = extract_table_col(table, "genome_position")?;
    let negative_log_p = extract_table_col(table, "neg_log10_p")?;
    let chromosome_indices = extract_table_col(table, "chromosome_index")?;
    let highlighted_column = table.col_index("highlighted");
    let width = get_opt_f64(opts, "width", 1200.0);
    let height = get_opt_f64(opts, "height", 400.0);
    let title = get_opt_str(opts, "title", "Manhattan Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Chromosome");
    let y_label = get_opt_str(opts, "ylabel", "-log10(p)");
    let threshold_y = get_opt_f64(opts, "threshold_y", -5e-8f64.log10());
    let genome_end = spans.last().unwrap().end();
    let x_padding = (genome_end * 0.01).max(1e-9);
    let x_domain = (-x_padding, genome_end + x_padding);
    let y_max = negative_log_p
        .iter()
        .copied()
        .fold(threshold_y.max(1.0), f64::max)
        * 1.05;
    let y_domain = (0.0, y_max);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[0.0, y_max],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(threshold_y),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(threshold_y),
        "#d62728",
        1.1,
    );
    let raster = crate::plot::RasterChoice {
        enabled: opts.get("raster").is_some_and(Value::is_truthy),
        scale: get_opt_f64(opts, "raster_scale", 2.0),
    };
    let points = (0..negative_log_p.len())
        .map(|index| {
            let chromosome_index = chromosome_indices[index] as usize;
            (
                x_scale.map(genome_positions[index]),
                y_scale.map(negative_log_p[index]),
                PALETTE[chromosome_index % PALETTE.len()],
            )
        })
        .collect::<Vec<_>>();
    let area = canvas.point_area();
    let thin = opts.get("thin").is_some_and(Value::is_truthy);
    let drawn = if thin {
        let coordinates = points.iter().map(|&(x, y, _)| (x, y)).collect::<Vec<_>>();
        let grid = if raster.enabled { raster.scale } else { 1.0 };
        let kept = thin_to_pixel_grid(&coordinates, area, grid, &negative_log_p);
        let survivors = kept.iter().map(|&index| points[index]).collect::<Vec<_>>();
        canvas.add_scatter(&survivors, 2.5, area, raster);
        survivors.len()
    } else {
        canvas.add_scatter(&points, 2.5, area, raster);
        points.len()
    };
    if let Some(column) = highlighted_column {
        for (index, row) in table.rows.iter().enumerate() {
            if row[column].is_truthy() {
                canvas.add_circle(points[index].0, points[index].1, 4.5, "#d62728");
            }
        }
    }
    draw_genome_axis(&mut canvas, spans, x_domain, x_label);
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let significant = negative_log_p
        .iter()
        .filter(|value| **value >= threshold_y)
        .count();
    if drawn < points.len() {
        canvas.set_accessible_description(format!(
            "Manhattan plot, thinned to one variant per pixel: {drawn} of {} variants drawn, the most significant in each pixel. Point density does not indicate variant count. {significant} variants meet the significance threshold.",
            points.len()
        ));
        canvas.add_text(
            canvas.margin.left,
            canvas.height - 6.0,
            &format!(
                "thinned: {drawn} of {} variants drawn (most significant per pixel)",
                points.len()
            ),
            "start",
            9.0,
        );
    } else {
        canvas.set_accessible_description(format!(
            "Manhattan plot with {} variants across {} chromosomes; {significant} meet the significance threshold.",
            points.len(), spans.len()
        ));
    }
    Ok(canvas.render())
}

pub(crate) fn is_manhattan_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "manhattan"))
}

pub(crate) fn render_manhattan_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_manhattan_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 Manhattan Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() Manhattan specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "chromosome_index",
        "chromosome",
        "position",
        "genome_position",
        "p_value",
        "neg_log10_p",
        "significant",
        "highlighted",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() Manhattan data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() Manhattan specification has no variants",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "Manhattan")?;
    let mut options = frozen_spec_options(map, render_options, "Manhattan")?;
    let threshold = get_opt_f64(&options, "threshold", f64::NAN);
    let threshold_y = get_opt_f64(&options, "threshold_y", f64::NAN);
    let chromosome_column = table.col_index("chromosome_index").unwrap();
    let chromosome_name_column = table.col_index("chromosome").unwrap();
    let position_column = table.col_index("position").unwrap();
    let genome_column = table.col_index("genome_position").unwrap();
    let p_column = table.col_index("p_value").unwrap();
    let transformed_column = table.col_index("neg_log10_p").unwrap();
    let significant_column = table.col_index("significant").unwrap();
    let highlighted_column = table.col_index("highlighted").unwrap();
    for (expected, row) in table.rows.iter().enumerate() {
        if frozen_nonnegative_integer(
            &row[table.col_index("source_row").unwrap()],
            "Manhattan",
            "source_row",
        )? != expected
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() Manhattan source rows must be contiguous",
                None,
            ));
        }
        let chromosome_index =
            frozen_nonnegative_integer(&row[chromosome_column], "Manhattan", "chromosome_index")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let genome_position = row[genome_column].as_float().unwrap_or(f64::NAN);
        let p_value = row[p_column].as_float().unwrap_or(f64::NAN);
        let transformed = row[transformed_column].as_float().unwrap_or(f64::NAN);
        if chromosome_index >= spans.len()
            || !position.is_finite()
            || position < 0.0
            || position > spans[chromosome_index].length
            || (genome_position - (spans[chromosome_index].offset + position)).abs() > 1e-8
            || !p_value.is_finite()
            || p_value <= 0.0
            || p_value > 1.0
            || (transformed + p_value.log10()).abs() > 1e-10
            || !matches!(row[significant_column], Value::Bool(_))
            || row[significant_column].is_truthy() != (p_value <= threshold)
            || !matches!(row[highlighted_column], Value::Bool(_))
            || row[chromosome_name_column].as_str() != Some(spans[chromosome_index].name.as_str())
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() Manhattan frozen geometry is inconsistent",
                None,
            ));
        }
    }
    if !threshold.is_finite()
        || !threshold_y.is_finite()
        || (threshold_y + threshold.log10()).abs() > 1e-10
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() Manhattan threshold metadata is inconsistent",
            None,
        ));
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    if let Some(width) = render_options.get("width") {
        options.insert("width".into(), width.clone());
    }
    if let Some(height) = render_options.get("height") {
        options.insert("height".into(), height.clone());
    }
    let svg = render_manhattan_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Manhattan Plot");
    finish_frozen_bio_plot(value, render_options, title, "Manhattan", svg)
}

fn builtin_manhattan(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "manhattan")?;
    let opts = parse_options(&args);
    let specification = manhattan_plot_spec_value(table, &opts)?;
    render_manhattan_plot_spec_value(&specification, &opts)
}

// ── 2. qq_plot ──────────────────────────────────────────────────

fn genetic_qq_plot_spec_value(values: Vec<f64>, opts: &HashMap<String, Value>) -> Result<Value> {
    let input_count = values.len();
    let mut p_values = values
        .into_iter()
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= 1.0)
        .collect::<Vec<_>>();
    p_values.sort_by(|left, right| left.total_cmp(right));
    if p_values.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "qq_plot() needs p-values within (0, 1]",
            None,
        ));
    }
    let confidence = get_opt_f64(opts, "confidence", 0.95);
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "qq_plot() confidence must be within (0, 1)",
            None,
        ));
    }
    // Exact beta-order-statistic intervals are intentionally opt-in. At
    // genome-wide scale, evaluating two beta quantiles per test can dominate
    // the entire plot; the p-value positions and lambda GC remain O(n log n).
    let envelope = bio_plot_bool_option(opts, "envelope", false, "qq_plot")?;
    let count = p_values.len();
    let tail = (1.0 - confidence) / 2.0;
    let mut rows = Vec::with_capacity(count);
    for (index, &p_value) in p_values.iter().enumerate() {
        let rank = index + 1;
        let expected_p = (rank as f64 - 0.5) / count as f64;
        let (envelope_lower, envelope_upper) = if envelope {
            let beta_alpha = rank as f64;
            let beta_beta = (count - rank + 1) as f64;
            let lower_p = beta_quantile(tail, beta_alpha, beta_beta).max(f64::MIN_POSITIVE);
            let upper_p = beta_quantile(1.0 - tail, beta_alpha, beta_beta).max(f64::MIN_POSITIVE);
            (-upper_p.log10(), -lower_p.log10())
        } else {
            let expected = -expected_p.log10();
            (expected, expected)
        };
        rows.push(vec![
            Value::Int(rank as i64),
            Value::Float(p_value),
            Value::Float(expected_p),
            Value::Float(-expected_p.log10()),
            Value::Float(-p_value.log10()),
            Value::Float(envelope_lower),
            Value::Float(envelope_upper),
        ]);
    }
    let mut chi_square = p_values
        .iter()
        .map(|p_value| {
            let z = bl_core::bio_core::stats_ops::normal_quantile(p_value / 2.0);
            z * z
        })
        .collect::<Vec<_>>();
    chi_square.sort_by(f64::total_cmp);
    let lambda_gc = quantile_type7(&chi_square, 0.5) / 0.454_936_423_119_572_7;
    let raster = raster_choice(opts, "qq_plot", count)?;
    let title = get_opt_str(opts, "title", "Genetic Q-Q Plot");
    let dropped = input_count - count;
    let warnings = if dropped == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{dropped} values outside finite (0, 1] were excluded"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("genetic_qq".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "rank",
                        "p_value",
                        "expected_p",
                        "expected_neg_log10_p",
                        "observed_neg_log10_p",
                        "envelope_lower",
                        "envelope_upper",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 600.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 600.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Expected -log10(p)").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "Observed -log10(p)").into()),
                        ),
                        ("confidence".into(), Value::Float(confidence)),
                        ("envelope".into(), Value::Bool(envelope)),
                        ("lambda_gc".into(), Value::Float(lambda_gc)),
                        ("raster".into(), Value::Bool(raster.enabled)),
                        ("raster_scale".into(), Value::Float(raster.scale)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("qq_plot".into())),
                        ("input_values".into(), Value::Int(input_count as i64)),
                        ("retained_values".into(), Value::Int(count as i64)),
                        ("dropped_values".into(), Value::Int(dropped as i64)),
                        (
                            "expected_positions".into(),
                            Value::Str("(rank - 0.5) / n".into()),
                        ),
                        (
                            "envelope_distribution".into(),
                            Value::Str("beta_order_statistic".into()),
                        ),
                        (
                            "lambda_gc_denominator".into(),
                            Value::Float(0.454_936_423_119_572_7),
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

fn render_genetic_qq_svg(table: &Table, opts: &HashMap<String, Value>) -> Result<String> {
    let expected = extract_table_col(table, "expected_neg_log10_p")?;
    let observed = extract_table_col(table, "observed_neg_log10_p")?;
    let envelope_lower = extract_table_col(table, "envelope_lower")?;
    let envelope_upper = extract_table_col(table, "envelope_upper")?;
    let width = get_opt_f64(opts, "width", 600.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let title = get_opt_str(opts, "title", "Genetic Q-Q Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Expected -log10(p)");
    let y_label = get_opt_str(opts, "ylabel", "Observed -log10(p)");
    let max_value = expected
        .iter()
        .chain(observed.iter())
        .chain(envelope_upper.iter())
        .copied()
        .fold(1.0, f64::max)
        * 1.05;
    let domain = (0.0, max_value);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.fit_cartesian_layout(
        &[0.0, max_value],
        &[0.0, max_value],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    if opts.get("envelope").is_some_and(Value::is_truthy) {
        let mut polygon = expected
            .iter()
            .zip(envelope_upper.iter())
            .map(|(x, y)| format!("{:.2},{:.2}", x_scale.map(*x), y_scale.map(*y)))
            .collect::<Vec<_>>();
        polygon.extend(
            expected
                .iter()
                .zip(envelope_lower.iter())
                .rev()
                .map(|(x, y)| format!("{:.2},{:.2}", x_scale.map(*x), y_scale.map(*y))),
        );
        canvas.elements.push(format!(
            r##"<polygon points="{}" fill="#4e79a7" fill-opacity="0.14" stroke="none" />"##,
            polygon.join(" ")
        ));
    }
    canvas.add_line(
        x_scale.map(0.0),
        y_scale.map(0.0),
        x_scale.map(max_value),
        y_scale.map(max_value),
        "#6b7280",
        1.0,
    );
    let raster = crate::plot::RasterChoice {
        enabled: opts.get("raster").is_some_and(Value::is_truthy),
        scale: get_opt_f64(opts, "raster_scale", 2.0),
    };
    let points = expected
        .iter()
        .zip(observed.iter())
        .map(|(x, y)| (x_scale.map(*x), y_scale.map(*y), PALETTE[0]))
        .collect::<Vec<_>>();
    let area = canvas.point_area();
    canvas.add_scatter(&points, 3.0, area, raster);
    canvas.draw_x_axis(
        &Scale {
            domain,
            range: domain,
        },
        x_label,
    );
    canvas.draw_y_axis(
        &Scale {
            domain,
            range: domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let lambda = get_opt_f64(opts, "lambda_gc", f64::NAN);
    canvas.add_text(
        canvas.margin.left + canvas.plot_width() - 6.0,
        canvas.margin.top + 18.0,
        &format!("λGC = {lambda:.3}"),
        "end",
        canvas.theme.legend_size,
    );
    let envelope_description = if opts.get("envelope").is_some_and(Value::is_truthy) {
        "an exact beta order-statistic confidence envelope"
    } else {
        "no confidence envelope"
    };
    canvas.set_accessible_description(format!(
        "Genetic p-value Q-Q plot with {} tests, {envelope_description}, and genomic inflation factor lambda GC {lambda:.3}.",
        points.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_genetic_qq_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "genetic_qq"))
}

pub(crate) fn render_genetic_qq_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_genetic_qq_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 genetic Q-Q Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genetic Q-Q specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "rank",
        "p_value",
        "expected_p",
        "expected_neg_log10_p",
        "observed_neg_log10_p",
        "envelope_lower",
        "envelope_upper",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() genetic Q-Q data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genetic Q-Q specification has no p-values",
            None,
        ));
    }
    let rank_column = table.col_index("rank").unwrap();
    let p_column = table.col_index("p_value").unwrap();
    let expected_p_column = table.col_index("expected_p").unwrap();
    let expected_column = table.col_index("expected_neg_log10_p").unwrap();
    let observed_column = table.col_index("observed_neg_log10_p").unwrap();
    let lower_column = table.col_index("envelope_lower").unwrap();
    let upper_column = table.col_index("envelope_upper").unwrap();
    let count = table.num_rows();
    let options = frozen_spec_options(map, render_options, "genetic Q-Q")?;
    let confidence = get_opt_f64(&options, "confidence", f64::NAN);
    let envelope = options.get("envelope").is_some_and(Value::is_truthy);
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genetic Q-Q confidence must be within (0, 1)",
            None,
        ));
    }
    let tail = (1.0 - confidence) / 2.0;
    let mut previous_p = 0.0;
    let mut p_values = Vec::with_capacity(count);
    for (index, row) in table.rows.iter().enumerate() {
        let rank = frozen_nonnegative_integer(&row[rank_column], "genetic Q-Q", "rank")?;
        let p_value = row[p_column].as_float().unwrap_or(f64::NAN);
        let expected_p = row[expected_p_column].as_float().unwrap_or(f64::NAN);
        let expected = row[expected_column].as_float().unwrap_or(f64::NAN);
        let observed = row[observed_column].as_float().unwrap_or(f64::NAN);
        let lower = row[lower_column].as_float().unwrap_or(f64::NAN);
        let upper = row[upper_column].as_float().unwrap_or(f64::NAN);
        let frozen_expected_p = (index as f64 + 0.5) / count as f64;
        let (frozen_lower, frozen_upper) = if envelope {
            let rank = index + 1;
            let lower_p =
                beta_quantile(tail, rank as f64, (count - rank + 1) as f64).max(f64::MIN_POSITIVE);
            let upper_p = beta_quantile(1.0 - tail, rank as f64, (count - rank + 1) as f64)
                .max(f64::MIN_POSITIVE);
            (-upper_p.log10(), -lower_p.log10())
        } else {
            let expected = -frozen_expected_p.log10();
            (expected, expected)
        };
        if rank != index + 1
            || !p_value.is_finite()
            || p_value <= 0.0
            || p_value > 1.0
            || (index > 0 && p_value < previous_p)
            || (expected_p - frozen_expected_p).abs() > 1e-12
            || (expected + expected_p.log10()).abs() > 1e-10
            || (observed + p_value.log10()).abs() > 1e-10
            || !lower.is_finite()
            || !upper.is_finite()
            || lower > upper
            || (lower - frozen_lower).abs() > 2e-9
            || (upper - frozen_upper).abs() > 2e-9
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genetic Q-Q frozen geometry is inconsistent",
                None,
            ));
        }
        previous_p = p_value;
        p_values.push(p_value);
    }
    let lambda = get_opt_f64(&options, "lambda_gc", f64::NAN);
    let mut chi_square = p_values
        .iter()
        .map(|p_value| {
            let z = bl_core::bio_core::stats_ops::normal_quantile(p_value / 2.0);
            z * z
        })
        .collect::<Vec<_>>();
    chi_square.sort_by(f64::total_cmp);
    let frozen_lambda = quantile_type7(&chi_square, 0.5) / 0.454_936_423_119_572_7;
    if !lambda.is_finite() || lambda < 0.0 || (lambda - frozen_lambda).abs() > 2e-9 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genetic Q-Q lambda_gc must be finite and non-negative",
            None,
        ));
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_genetic_qq_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Genetic Q-Q Plot");
    finish_frozen_bio_plot(value, render_options, title, "genetic Q-Q", svg)
}

fn builtin_qq_plot(args: Vec<Value>) -> Result<Value> {
    let values = nums_from_value(&args[0], "qq_plot")?;
    let opts = parse_options(&args);
    let specification = genetic_qq_plot_spec_value(values, &opts)?;
    render_genetic_qq_plot_spec_value(&specification, &opts)
}

// ── 3. ideogram ─────────────────────────────────────────────────

fn cytoband_class(stain: &str) -> &'static str {
    let stain = stain.to_ascii_lowercase();
    if stain.contains("acen") {
        "acen"
    } else if stain.contains("gpos100") {
        "gpos100"
    } else if stain.contains("gpos75") {
        "gpos75"
    } else if stain.contains("gpos50") {
        "gpos50"
    } else if stain.contains("gpos25") {
        "gpos25"
    } else if stain.contains("gvar") {
        "gvar"
    } else if stain.contains("stalk") {
        "stalk"
    } else if stain.contains("gneg") {
        "gneg"
    } else {
        "unknown"
    }
}

fn cytoband_color(class: &str) -> &'static str {
    match class {
        "acen" => "#ef4444",
        "gpos100" => "#111827",
        "gpos75" => "#52525b",
        "gpos50" => "#9297a1",
        "gpos25" => "#d1d5db",
        "gvar" => "#c4b5fd",
        "stalk" => "#60a5fa",
        "gneg" => "#f8fafc",
        _ => "#e5e7eb",
    }
}

fn ideogram_plot_spec_value(table: &Table, opts: &HashMap<String, Value>) -> Result<Value> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let start_column = get_opt_str(opts, "start", "start");
    let end_column = get_opt_str(opts, "end", "end");
    let stain_column = if let Some(column) = opts.get("stain").and_then(Value::as_str) {
        Some(column)
    } else if table.col_index("stain").is_some() {
        Some("stain")
    } else if table.col_index("gieStain").is_some() {
        Some("gieStain")
    } else {
        None
    };
    let band_column = if let Some(column) = opts.get("band").and_then(Value::as_str) {
        Some(column)
    } else if table.col_index("band").is_some() {
        Some("band")
    } else if table.col_index("name").is_some() {
        Some("name")
    } else {
        None
    };
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let starts = extract_table_col(table, start_column)?;
    let ends = extract_table_col(table, end_column)?;
    if chromosomes.is_empty()
        || starts.len() != chromosomes.len()
        || ends.len() != chromosomes.len()
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ideogram() requires equally sized, non-empty chromosome/start/end columns",
            None,
        ));
    }
    if starts
        .iter()
        .zip(&ends)
        .any(|(&start, &end)| !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ideogram() intervals must be finite, non-negative and have end > start",
            None,
        ));
    }
    let (_, spans) = checked_genome_layout(&chromosomes, &ends, "ideogram")?;
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span.index))
        .collect::<HashMap<_, _>>();
    let stain_index = stain_column.and_then(|column| table.col_index(column));
    if stain_column.is_some() && stain_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", stain_column.unwrap()),
            None,
        ));
    }
    let band_index = band_column.and_then(|column| table.col_index(column));
    if band_column.is_some() && band_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", band_column.unwrap()),
            None,
        ));
    }
    if stain_index.is_some_and(|column| {
        table
            .rows
            .iter()
            .any(|row| !matches!(row[column], Value::Str(_)))
    }) || band_index.is_some_and(|column| {
        table
            .rows
            .iter()
            .any(|row| !matches!(row[column], Value::Str(_)))
    }) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ideogram() stain and band values must be Str",
            None,
        ));
    }
    let mut order = (0..table.num_rows()).collect::<Vec<_>>();
    order.sort_by(|&left, &right| {
        span_lookup[chromosomes[left].as_str()]
            .cmp(&span_lookup[chromosomes[right].as_str()])
            .then_with(|| starts[left].total_cmp(&starts[right]))
            .then_with(|| ends[left].total_cmp(&ends[right]))
            .then_with(|| left.cmp(&right))
    });
    let rows = order
        .into_iter()
        .map(|source_row| {
            let stain = stain_index
                .and_then(|column| table.rows[source_row][column].as_str())
                .unwrap_or("gneg");
            let band = band_index
                .and_then(|column| table.rows[source_row][column].as_str())
                .unwrap_or("");
            vec![
                Value::Int(source_row as i64),
                Value::Int(span_lookup[chromosomes[source_row].as_str()] as i64),
                Value::Str(chromosomes[source_row].clone()),
                Value::Float(starts[source_row]),
                Value::Float(ends[source_row]),
                Value::Float(ends[source_row] - starts[source_row]),
                Value::Str(band.into()),
                Value::Str(stain.into()),
                Value::Str(cytoband_class(stain).into()),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Chromosome Ideogram");
    let default_height = (spans.len() as f64 * 28.0 + 105.0).max(180.0);
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("ideogram".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "start",
                        "end",
                        "length",
                        "band",
                        "stain",
                        "stain_class",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 900.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", default_height)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Position").into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("ideogram".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "chromosome_order".into(),
                            Value::Str("first_observed".into()),
                        ),
                        (
                            "band_order".into(),
                            Value::Str("chromosome_start_end_source_row".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open".into()),
                        ),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("start_column".into(), Value::Str(start_column.into())),
                        ("end_column".into(), Value::Str(end_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

fn render_ideogram_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 900.0);
    let height = get_opt_f64(
        opts,
        "height",
        (spans.len() as f64 * 28.0 + 105.0).max(180.0),
    );
    let title = get_opt_str(opts, "title", "Chromosome Ideogram");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Position");
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 78.0;
    canvas.margin.right = 24.0;
    canvas.margin.top = if subtitle.is_empty() { 48.0 } else { 68.0 };
    canvas.margin.bottom = if caption.is_empty() { 43.0 } else { 62.0 };
    let local_maximum = spans.iter().map(|span| span.length).fold(1.0, f64::max);
    let x_scale = Scale {
        domain: (0.0, local_maximum),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let row_height = (canvas.plot_height() / spans.len() as f64)
        .min(24.0)
        .max(10.0);
    let band_height = (row_height * 0.62).clamp(8.0, 15.0);
    let chromosome_column = table.col_index("chromosome_index").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let class_column = table.col_index("stain_class").unwrap();
    let clip_rectangles = spans
        .iter()
        .map(|span| {
            let y = canvas.margin.top
                + span.index as f64 * row_height
                + (row_height - band_height) / 2.0;
            format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" />"#,
                x_scale.map(0.0),
                y,
                (x_scale.map(span.length) - x_scale.map(0.0)).max(1.0),
                band_height,
                band_height / 2.0
            )
        })
        .collect::<String>();
    canvas.elements.push(format!(
        r#"<defs><clipPath id="biolang-cytoband-clip">{clip_rectangles}</clipPath></defs>"#
    ));
    let mut paths = HashMap::<String, String>::new();
    for row in &table.rows {
        let chromosome_index = row[chromosome_column].as_float().unwrap_or(0.0) as usize;
        let start = row[start_column].as_float().unwrap_or(0.0);
        let end = row[end_column].as_float().unwrap_or(0.0);
        let class = row[class_column].as_str().unwrap_or("unknown");
        let y = canvas.margin.top
            + chromosome_index as f64 * row_height
            + (row_height - band_height) / 2.0;
        let x1 = x_scale.map(start);
        let x2 = x_scale.map(end);
        paths.entry(class.into()).or_default().push_str(&format!(
            "M{:.2},{:.2}H{:.2}V{:.2}H{:.2}Z",
            x1,
            y,
            x2,
            y + band_height,
            x1
        ));
    }
    for class in [
        "gneg", "gpos25", "gpos50", "gpos75", "gpos100", "acen", "gvar", "stalk", "unknown",
    ] {
        if let Some(path) = paths.get(class) {
            canvas.elements.push(format!(
                r##"<path d="{path}" fill="{}" stroke="none" clip-path="url(#biolang-cytoband-clip)" />"##,
                cytoband_color(class)
            ));
        }
    }
    for span in spans {
        let y =
            canvas.margin.top + span.index as f64 * row_height + (row_height - band_height) / 2.0;
        canvas.add_text(
            canvas.margin.left - 9.0,
            y + band_height / 2.0 + 4.0,
            &span.name,
            "end",
            canvas.theme.tick_size,
        );
        canvas.elements.push(format!(
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" fill="none" stroke="#475569" stroke-width="0.8" />"##,
            x_scale.map(0.0),
            y,
            (x_scale.map(span.length) - x_scale.map(0.0)).max(1.0),
            band_height,
            band_height / 2.0
        ));
    }
    let axis = Scale {
        domain: (0.0, local_maximum),
        range: (0.0, local_maximum),
    };
    canvas.draw_x_axis(&axis, x_label);
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Chromosome ideogram with {} cytobands across {} chromosomes; chromosome lengths share one coordinate scale.",
        table.num_rows(), spans.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_ideogram_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "ideogram"))
}

pub(crate) fn render_ideogram_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_ideogram_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 ideogram Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ideogram specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "chromosome_index",
        "chromosome",
        "start",
        "end",
        "length",
        "stain",
        "stain_class",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() ideogram data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() ideogram specification has no bands",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "ideogram")?;
    let source_column = table.col_index("source_row").unwrap();
    let index_column = table.col_index("chromosome_index").unwrap();
    let name_column = table.col_index("chromosome").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let length_column = table.col_index("length").unwrap();
    let stain_column = table.col_index("stain").unwrap();
    let class_column = table.col_index("stain_class").unwrap();
    let mut previous: Option<(usize, f64, f64, usize)> = None;
    for row in &table.rows {
        let source = frozen_nonnegative_integer(&row[source_column], "ideogram", "source_row")?;
        let index = frozen_nonnegative_integer(&row[index_column], "ideogram", "chromosome_index")?;
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        let name = row[name_column].as_str().unwrap_or("");
        let stain = row[stain_column].as_str().unwrap_or("");
        let class = row[class_column].as_str().unwrap_or("");
        let key = (index, start, end, source);
        if index >= spans.len()
            || name != spans[index].name
            || !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || end <= start
            || end > spans[index].length + 1e-8
            || (length - (end - start)).abs() > 1e-10
            || class != cytoband_class(stain)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ideogram frozen geometry is inconsistent",
                None,
            ));
        }
        previous = Some(key);
    }
    let options = frozen_spec_options(map, render_options, "ideogram")?;
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_ideogram_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Chromosome Ideogram");
    finish_frozen_bio_plot(value, render_options, title, "ideogram", svg)
}

fn builtin_ideogram(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "ideogram")?;
    let opts = parse_options(&args);
    let specification = ideogram_plot_spec_value(table, &opts)?;
    render_ideogram_plot_spec_value(&specification, &opts)
}

// ── 4. rainfall ─────────────────────────────────────────────────

fn rainfall_plot_spec_value(table: &Table, opts: &HashMap<String, Value>) -> Result<Option<Value>> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let position_column = get_opt_str(opts, "pos", "pos");
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let positions = extract_table_col(table, position_column)?;
    let (_, spans) = checked_genome_layout(&chromosomes, &positions, "rainfall")?;
    let duplicate_floor = get_opt_f64(opts, "duplicate_floor", 1.0);
    if !duplicate_floor.is_finite() || duplicate_floor <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "rainfall() duplicate_floor must be finite and positive",
            None,
        ));
    }
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span))
        .collect::<HashMap<_, _>>();
    let mut indices = (0..positions.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        span_lookup[chromosomes[*left].as_str()]
            .index
            .cmp(&span_lookup[chromosomes[*right].as_str()].index)
            .then(positions[*left].total_cmp(&positions[*right]))
            .then(left.cmp(right))
    });
    let mut rows = Vec::new();
    for pair in indices.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        if chromosomes[previous] != chromosomes[current] {
            continue;
        }
        let distance = positions[current] - positions[previous];
        let plotted_distance = distance.max(duplicate_floor);
        let span = span_lookup[chromosomes[current].as_str()];
        rows.push(vec![
            Value::Int(rows.len() as i64),
            Value::Int(current as i64),
            Value::Int(span.index as i64),
            Value::Str(chromosomes[current].clone()),
            Value::Float(positions[current]),
            Value::Float(positions[previous]),
            Value::Float(span.offset + positions[current]),
            Value::Float(distance),
            Value::Float(plotted_distance),
            Value::Float(plotted_distance.log10()),
            Value::Bool(distance == 0.0),
        ]);
    }
    if rows.is_empty() {
        return Ok(None);
    }
    let raster = raster_choice(opts, "rainfall", rows.len())?;
    let short_distance = get_opt_f64(opts, "short_distance", 1_000.0);
    let medium_distance = get_opt_f64(opts, "medium_distance", 100_000.0);
    if !short_distance.is_finite()
        || !medium_distance.is_finite()
        || short_distance <= 0.0
        || medium_distance <= short_distance
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "rainfall() distance colour boundaries must be finite, positive, and short_distance < medium_distance",
            None,
        ));
    }
    let title = get_opt_str(opts, "title", "Rainfall Plot");
    Ok(Some(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("rainfall".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "position",
                        "previous_position",
                        "genome_position",
                        "distance",
                        "plotted_distance",
                        "log10_distance",
                        "duplicate_position",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 1000.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 400.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Chromosome").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "log10(distance in bp)").into()),
                        ),
                        ("duplicate_floor".into(), Value::Float(duplicate_floor)),
                        ("short_distance".into(), Value::Float(short_distance)),
                        ("medium_distance".into(), Value::Float(medium_distance)),
                        ("raster".into(), Value::Bool(raster.enabled)),
                        ("raster_scale".into(), Value::Float(raster.scale)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("rainfall".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "distance_points".into(),
                            Value::Int((table.num_rows() - spans.len()) as i64),
                        ),
                        (
                            "sort".into(),
                            Value::Str(
                                "first_observed_chromosome_then_position_then_source_row".into(),
                            ),
                        ),
                        (
                            "distance_scope".into(),
                            Value::Str("within_chromosome".into()),
                        ),
                        ("distance_transform".into(), Value::Str("log10".into())),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("position_column".into(), Value::Str(position_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )))
}

fn render_rainfall_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let genome_positions = extract_table_col(table, "genome_position")?;
    let log_distance = extract_table_col(table, "log10_distance")?;
    let width = get_opt_f64(opts, "width", 1000.0);
    let height = get_opt_f64(opts, "height", 400.0);
    let title = get_opt_str(opts, "title", "Rainfall Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Chromosome");
    let y_label = get_opt_str(opts, "ylabel", "log10(distance in bp)");
    let genome_end = spans.last().unwrap().end();
    let x_padding = (genome_end * 0.01).max(1e-9);
    let x_domain = (-x_padding, genome_end + x_padding);
    let (mut y_min, mut y_max) = col_range(&log_distance);
    if (y_max - y_min).abs() < 1e-12 {
        y_min -= 0.5;
        y_max += 0.5;
    }
    let y_domain = (y_min, y_max);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[y_min, y_max],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    let short_log = get_opt_f64(opts, "short_distance", 1_000.0).log10();
    let medium_log = get_opt_f64(opts, "medium_distance", 100_000.0).log10();
    let points = genome_positions
        .iter()
        .zip(log_distance.iter())
        .map(|(x, y)| {
            let colour = if *y < short_log {
                "#d62728"
            } else if *y < medium_log {
                "#e69f00"
            } else {
                "#2a9d8f"
            };
            (x_scale.map(*x), y_scale.map(*y), colour)
        })
        .collect::<Vec<_>>();
    let raster = crate::plot::RasterChoice {
        enabled: opts.get("raster").is_some_and(Value::is_truthy),
        scale: get_opt_f64(opts, "raster_scale", 2.0),
    };
    let area = canvas.point_area();
    canvas.add_scatter(&points, 2.5, area, raster);
    draw_genome_axis(&mut canvas, spans, x_domain, x_label);
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let duplicate_column = table.col_index("duplicate_position").unwrap();
    let duplicates = table
        .rows
        .iter()
        .filter(|row| row[duplicate_column].is_truthy())
        .count();
    canvas.set_accessible_description(format!(
        "Rainfall plot with {} within-chromosome inter-variant distances across {} chromosomes; {duplicates} duplicate-position distances use the declared display floor.",
        points.len(), spans.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_rainfall_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "rainfall"))
}

pub(crate) fn render_rainfall_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_rainfall_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 rainfall Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() rainfall specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "chromosome_index",
        "chromosome",
        "position",
        "previous_position",
        "genome_position",
        "distance",
        "plotted_distance",
        "log10_distance",
        "duplicate_position",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() rainfall data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() rainfall specification has no distances",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "rainfall")?;
    let options = frozen_spec_options(map, render_options, "rainfall")?;
    let duplicate_floor = get_opt_f64(&options, "duplicate_floor", f64::NAN);
    let short_distance = get_opt_f64(&options, "short_distance", f64::NAN);
    let medium_distance = get_opt_f64(&options, "medium_distance", f64::NAN);
    if !duplicate_floor.is_finite()
        || duplicate_floor <= 0.0
        || !short_distance.is_finite()
        || !medium_distance.is_finite()
        || short_distance <= 0.0
        || medium_distance <= short_distance
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() rainfall distance floors and colour boundaries are invalid",
            None,
        ));
    }
    let point_column = table.col_index("point_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let chromosome_column = table.col_index("chromosome_index").unwrap();
    let chromosome_name_column = table.col_index("chromosome").unwrap();
    let position_column = table.col_index("position").unwrap();
    let previous_column = table.col_index("previous_position").unwrap();
    let genome_column = table.col_index("genome_position").unwrap();
    let distance_column = table.col_index("distance").unwrap();
    let plotted_column = table.col_index("plotted_distance").unwrap();
    let log_column = table.col_index("log10_distance").unwrap();
    let duplicate_column = table.col_index("duplicate_position").unwrap();
    let mut previous_key: Option<(usize, f64, usize)> = None;
    for (expected, row) in table.rows.iter().enumerate() {
        let chromosome_index =
            frozen_nonnegative_integer(&row[chromosome_column], "rainfall", "chromosome_index")?;
        let source_row = frozen_nonnegative_integer(&row[source_column], "rainfall", "source_row")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let previous = row[previous_column].as_float().unwrap_or(f64::NAN);
        let genome_position = row[genome_column].as_float().unwrap_or(f64::NAN);
        let distance = row[distance_column].as_float().unwrap_or(f64::NAN);
        let plotted = row[plotted_column].as_float().unwrap_or(f64::NAN);
        let log_distance = row[log_column].as_float().unwrap_or(f64::NAN);
        if frozen_nonnegative_integer(&row[point_column], "rainfall", "point_index")? != expected
            || chromosome_index >= spans.len()
            || !position.is_finite()
            || !previous.is_finite()
            || position < previous
            || (distance - (position - previous)).abs() > 1e-10
            || (plotted - distance.max(duplicate_floor)).abs() > 1e-10
            || (log_distance - plotted.log10()).abs() > 1e-10
            || !matches!(row[duplicate_column], Value::Bool(_))
            || row[duplicate_column].is_truthy() != (distance == 0.0)
            || (genome_position - (spans[chromosome_index].offset + position)).abs() > 1e-8
            || row[chromosome_name_column].as_str() != Some(spans[chromosome_index].name.as_str())
            || previous_key.is_some_and(
                |(previous_chromosome, previous_position, previous_source)| {
                    chromosome_index < previous_chromosome
                        || (chromosome_index == previous_chromosome
                            && (position < previous_position
                                || (position == previous_position && source_row < previous_source)))
                },
            )
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() rainfall frozen geometry is inconsistent",
                None,
            ));
        }
        previous_key = Some((chromosome_index, position, source_row));
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_rainfall_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Rainfall Plot");
    finish_frozen_bio_plot(value, render_options, title, "rainfall", svg)
}

fn builtin_rainfall(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "rainfall")?;
    let opts = parse_options(&args);
    let Some(specification) = rainfall_plot_spec_value(table, &opts)? else {
        write_output("  (insufficient data for rainfall plot)\n");
        return Ok(Value::Nil);
    };
    render_rainfall_plot_spec_value(&specification, &opts)
}

// ── 5. cnv_plot ─────────────────────────────────────────────────

fn cnv_state(ratio: f64, loss_threshold: f64, gain_threshold: f64) -> &'static str {
    if ratio > gain_threshold {
        "gain"
    } else if ratio < loss_threshold {
        "loss"
    } else {
        "neutral"
    }
}

fn cnv_plot_spec_value(table: &Table, opts: &HashMap<String, Value>) -> Result<Value> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let start_column = get_opt_str(opts, "start", "start");
    let end_column = get_opt_str(opts, "end", "end");
    let ratio_column = get_opt_str(opts, "ratio", "log2ratio");
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let starts = extract_table_col(table, start_column)?;
    let ends = extract_table_col(table, end_column)?;
    let ratios = extract_table_col(table, ratio_column)?;
    if chromosomes.is_empty()
        || starts.len() != chromosomes.len()
        || ends.len() != chromosomes.len()
        || ratios.len() != chromosomes.len()
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cnv_plot() requires equally sized, non-empty chromosome/start/end/ratio columns",
            None,
        ));
    }
    if starts
        .iter()
        .zip(&ends)
        .zip(&ratios)
        .any(|((&start, &end), &ratio)| {
            !start.is_finite()
                || !end.is_finite()
                || !ratio.is_finite()
                || start < 0.0
                || end <= start
        })
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cnv_plot() intervals and ratios must be finite, positions non-negative, and end > start",
            None,
        ));
    }
    let loss_threshold = get_opt_f64(opts, "loss_threshold", -0.2);
    let gain_threshold = get_opt_f64(opts, "gain_threshold", 0.2);
    if !loss_threshold.is_finite()
        || !gain_threshold.is_finite()
        || loss_threshold >= gain_threshold
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cnv_plot() requires finite loss_threshold < gain_threshold",
            None,
        ));
    }
    let (_, spans) = checked_genome_layout(&chromosomes, &ends, "cnv_plot")?;
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span))
        .collect::<HashMap<_, _>>();
    let mut order = (0..table.num_rows()).collect::<Vec<_>>();
    order.sort_by(|&left, &right| {
        span_lookup[chromosomes[left].as_str()]
            .index
            .cmp(&span_lookup[chromosomes[right].as_str()].index)
            .then_with(|| starts[left].total_cmp(&starts[right]))
            .then_with(|| ends[left].total_cmp(&ends[right]))
            .then_with(|| left.cmp(&right))
    });
    let rows = order
        .into_iter()
        .map(|source_row| {
            let span = span_lookup[chromosomes[source_row].as_str()];
            let genome_start = span.offset + starts[source_row];
            let genome_end = span.offset + ends[source_row];
            vec![
                Value::Int(source_row as i64),
                Value::Int(span.index as i64),
                Value::Str(chromosomes[source_row].clone()),
                Value::Float(starts[source_row]),
                Value::Float(ends[source_row]),
                Value::Float(ends[source_row] - starts[source_row]),
                Value::Float(genome_start),
                Value::Float(genome_end),
                Value::Float((genome_start + genome_end) / 2.0),
                Value::Float(ratios[source_row]),
                Value::Str(cnv_state(ratios[source_row], loss_threshold, gain_threshold).into()),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Copy-number Profile");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("cnv".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "start",
                        "end",
                        "length",
                        "genome_start",
                        "genome_end",
                        "genome_midpoint",
                        "log2ratio",
                        "state",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 1100.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 380.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Chromosome").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "log2 ratio").into()),
                        ),
                        ("loss_threshold".into(), Value::Float(loss_threshold)),
                        ("gain_threshold".into(), Value::Float(gain_threshold)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("cnv_plot".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "chromosome_order".into(),
                            Value::Str("first_observed".into()),
                        ),
                        ("chromosome_gap_fraction".into(), Value::Float(0.02)),
                        (
                            "segment_order".into(),
                            Value::Str("chromosome_start_end_source_row".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open".into()),
                        ),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("start_column".into(), Value::Str(start_column.into())),
                        ("end_column".into(), Value::Str(end_column.into())),
                        ("ratio_column".into(), Value::Str(ratio_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

fn render_cnv_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let genome_start = extract_table_col(table, "genome_start")?;
    let genome_end = extract_table_col(table, "genome_end")?;
    let ratios = extract_table_col(table, "log2ratio")?;
    let state_column = table.col_index("state").unwrap();
    let width = get_opt_f64(opts, "width", 1100.0);
    let height = get_opt_f64(opts, "height", 380.0);
    let title = get_opt_str(opts, "title", "Copy-number Profile");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Chromosome");
    let y_label = get_opt_str(opts, "ylabel", "log2 ratio");
    let maximum = ratios
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.5, f64::max)
        .max(get_opt_f64(opts, "loss_threshold", -0.2).abs())
        .max(get_opt_f64(opts, "gain_threshold", 0.2).abs());
    let y_domain = (-maximum * 1.12, maximum * 1.12);
    let final_end = spans.last().map(ChromosomeSpan::end).unwrap_or(1.0);
    let pad = final_end * 0.01;
    let x_domain = (-pad, final_end + pad);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[y_domain.0, 0.0, y_domain.1],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    for span in spans.iter().filter(|span| span.index % 2 == 1) {
        canvas.elements.push(format!(
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="#64748b" fill-opacity="0.035" />"##,
            x_scale.map(span.start()),
            canvas.margin.top,
            (x_scale.map(span.end()) - x_scale.map(span.start())).max(0.0),
            canvas.plot_height()
        ));
    }
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(0.0),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(0.0),
        "#475569",
        1.0,
    );
    let mut paths = HashMap::<String, String>::new();
    for (index, row) in table.rows.iter().enumerate() {
        let state = row[state_column].as_str().unwrap_or("neutral");
        paths.entry(state.into()).or_default().push_str(&format!(
            "M{:.2},{:.2}H{:.2}",
            x_scale.map(genome_start[index]),
            y_scale.map(ratios[index]),
            x_scale.map(genome_end[index])
        ));
    }
    for state in ["neutral", "loss", "gain"] {
        if let Some(path) = paths.get(state) {
            let color = match state {
                "gain" => "#d73027",
                "loss" => "#4575b4",
                _ => "#6b7280",
            };
            canvas.elements.push(format!(
                r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="2.2" stroke-linecap="butt" />"#
            ));
        }
    }
    draw_genome_axis(&mut canvas, spans, x_domain, x_label);
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let gain_count = table
        .rows
        .iter()
        .filter(|row| row[state_column].as_str() == Some("gain"))
        .count();
    let loss_count = table
        .rows
        .iter()
        .filter(|row| row[state_column].as_str() == Some("loss"))
        .count();
    canvas.set_accessible_description(format!(
        "Copy-number profile with {} segments across {} chromosomes: {gain_count} gain and {loss_count} loss segments.",
        table.num_rows(), spans.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_cnv_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "cnv"))
}

pub(crate) fn render_cnv_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_cnv_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 CNV Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() CNV specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "chromosome_index",
        "chromosome",
        "start",
        "end",
        "length",
        "genome_start",
        "genome_end",
        "genome_midpoint",
        "log2ratio",
        "state",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() CNV data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() CNV specification has no segments",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "CNV")?;
    let options = frozen_spec_options(map, render_options, "CNV")?;
    let loss_threshold = get_opt_f64(&options, "loss_threshold", f64::NAN);
    let gain_threshold = get_opt_f64(&options, "gain_threshold", f64::NAN);
    if !loss_threshold.is_finite()
        || !gain_threshold.is_finite()
        || loss_threshold >= gain_threshold
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() CNV thresholds are inconsistent",
            None,
        ));
    }
    let source_column = table.col_index("source_row").unwrap();
    let index_column = table.col_index("chromosome_index").unwrap();
    let name_column = table.col_index("chromosome").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let length_column = table.col_index("length").unwrap();
    let genome_start_column = table.col_index("genome_start").unwrap();
    let genome_end_column = table.col_index("genome_end").unwrap();
    let midpoint_column = table.col_index("genome_midpoint").unwrap();
    let ratio_column = table.col_index("log2ratio").unwrap();
    let state_column = table.col_index("state").unwrap();
    let mut previous: Option<(usize, f64, f64, usize)> = None;
    for row in &table.rows {
        let source = frozen_nonnegative_integer(&row[source_column], "CNV", "source_row")?;
        let index = frozen_nonnegative_integer(&row[index_column], "CNV", "chromosome_index")?;
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        let genome_start = row[genome_start_column].as_float().unwrap_or(f64::NAN);
        let genome_end = row[genome_end_column].as_float().unwrap_or(f64::NAN);
        let midpoint = row[midpoint_column].as_float().unwrap_or(f64::NAN);
        let ratio = row[ratio_column].as_float().unwrap_or(f64::NAN);
        let state = row[state_column].as_str().unwrap_or("");
        let name = row[name_column].as_str().unwrap_or("");
        let key = (index, start, end, source);
        if index >= spans.len()
            || name != spans[index].name
            || !start.is_finite()
            || !end.is_finite()
            || !ratio.is_finite()
            || start < 0.0
            || end <= start
            || end > spans[index].length + 1e-8
            || (length - (end - start)).abs() > 1e-10
            || (genome_start - (spans[index].offset + start)).abs() > 1e-8
            || (genome_end - (spans[index].offset + end)).abs() > 1e-8
            || (midpoint - (genome_start + genome_end) / 2.0).abs() > 1e-8
            || state != cnv_state(ratio, loss_threshold, gain_threshold)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() CNV frozen geometry is inconsistent",
                None,
            ));
        }
        previous = Some(key);
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_cnv_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Copy-number Profile");
    finish_frozen_bio_plot(value, render_options, title, "CNV", svg)
}

fn builtin_cnv_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "cnv_plot")?;
    let opts = parse_options(&args);
    let specification = cnv_plot_spec_value(table, &opts)?;
    render_cnv_plot_spec_value(&specification, &opts)
}

// ── 6. violin ───────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ViolinShape {
    name: String,
    sample_count: usize,
    bandwidth: f64,
    median: f64,
    input_min: f64,
    input_max: f64,
    points: Vec<(f64, f64)>,
}

fn violin_shape(name: String, values: &[f64], steps: usize) -> ViolinShape {
    let bandwidth = silverman_bandwidth(values);
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let (input_min, input_max) = col_range(values);
    ViolinShape {
        name,
        sample_count: values.len(),
        bandwidth,
        median: quantile_type7(&sorted, 0.5),
        input_min,
        input_max,
        points: gaussian_kde(values, bandwidth, steps),
    }
}

fn render_legacy_violin_svg(
    shapes: &[ViolinShape],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let w = get_opt_f64(opts, "width", 600.0);
    let h = get_opt_f64(opts, "height", 400.0);
    let mut c = themed_canvas(w, h, opts);
    c.margin.bottom = 60.0;
    let global_min = shapes
        .iter()
        .map(|shape| shape.input_min)
        .fold(f64::INFINITY, f64::min);
    let global_max = shapes
        .iter()
        .map(|shape| shape.input_max)
        .fold(f64::NEG_INFINITY, f64::max);
    let ys = Scale {
        domain: (global_min, global_max),
        range: (c.margin.top + c.plot_height(), c.margin.top),
    };
    let group_w = c.plot_width() / shapes.len() as f64;
    for (gi, shape) in shapes.iter().enumerate() {
        let max_d = shape
            .points
            .iter()
            .map(|point| point.1)
            .fold(0.0_f64, f64::max);
        let cx = c.margin.left + (gi as f64 + 0.5) * group_w;
        let half_w = group_w * 0.4;
        let mut points_l = String::new();
        let mut points_r = String::new();
        for &(value, density) in &shape.points {
            let y = ys.map(value);
            let dx = if max_d > 0.0 {
                density / max_d * half_w
            } else {
                0.0
            };
            points_l.push_str(&format!("{:.1},{:.1} ", cx - dx, y));
            points_r.push_str(&format!("{:.1},{:.1} ", cx + dx, y));
        }
        let all_points = format!(
            "{points_l}{}",
            points_r
                .split_whitespace()
                .rev()
                .collect::<Vec<_>>()
                .join(" ")
        );
        c.elements.push(format!(
            r#"<polygon points="{all_points}" fill="{}" opacity="0.6" />"#,
            PALETTE[gi % PALETTE.len()]
        ));
        c.add_text(
            cx,
            c.margin.top + c.plot_height() + 18.0,
            &shape.name,
            "middle",
            10.0,
        );
    }
    c.draw_y_axis(
        &Scale {
            domain: (global_min, global_max),
            range: (global_min, global_max),
        },
        get_opt_str(opts, "ylab", "Value"),
    );
    finish_themed_canvas(&mut c, opts, "Violin Plot");
    c.set_accessible_description(format!(
        "Violin plot with {} groups; each shape is a frozen Gaussian kernel-density grid.",
        shapes.len()
    ));
    Ok(c.render())
}

fn render_long_violin_svg(shapes: &[ViolinShape], opts: &HashMap<String, Value>) -> Result<String> {
    let theme = plot_theme(opts);
    let seurat_theme = get_opt_str(opts, "theme", "") == "seurat";
    let value_col = get_opt_str(opts, "value_label", "value").to_string();
    let title = get_opt_str(opts, "title", "Distribution").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let width = get_opt_f64(opts, "width", 640.0);
    let height = get_opt_f64(opts, "height", 420.0);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let lo = shapes
        .iter()
        .filter_map(|shape| shape.points.first().map(|point| point.0))
        .fold(f64::INFINITY, f64::min);
    let hi = shapes
        .iter()
        .filter_map(|shape| shape.points.last().map(|point| point.0))
        .fold(f64::NEG_INFINITY, f64::max);

    if theme.is_adaptive() {
        let provisional = Scale {
            domain: (lo, hi),
            range: (1.0, 0.0),
        };
        let widest_tick = provisional
            .nice_ticks(5)
            .iter()
            .map(|tick| estimate_text_width(&format!("{tick:.2}"), theme.tick_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_tick + 42.0).clamp(54.0, width * 0.30);
        canvas.margin.right = 18.0_f64.min(width * 0.08);
        canvas.margin.top = if subtitle.is_empty() { 48.0 } else { 66.0 };
        let provisional_slot =
            (width - canvas.margin.left - canvas.margin.right).max(1.0) / shapes.len() as f64;
        let widest_group = shapes
            .iter()
            .map(|shape| estimate_text_width(&shape.name, theme.tick_size))
            .fold(0.0, f64::max);
        let rotate_labels = widest_group > provisional_slot * 0.86;
        let label_reserve = if rotate_labels {
            (widest_group * 0.72 + 12.0).clamp(32.0, height * 0.28)
        } else {
            28.0
        };
        canvas.margin.bottom = label_reserve + if caption.is_empty() { 12.0 } else { 28.0 };
    }

    let y_scale = Scale {
        domain: (lo, hi),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let slot = canvas.plot_width() / shapes.len() as f64;
    if theme.is_adaptive() {
        let left = canvas.margin.left;
        let right = left + canvas.plot_width();
        let top = canvas.margin.top;
        let bottom = top + canvas.plot_height();
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for tick in y_scale.nice_ticks(5) {
            let y = y_scale.map(tick);
            canvas.add_line(left, y, right, y, theme.grid_colour, theme.grid_width);
        }
        canvas.add_line(
            left,
            bottom,
            right,
            bottom,
            theme.axis_colour,
            theme.axis_width,
        );
    }
    let widest_group = shapes
        .iter()
        .map(|shape| estimate_text_width(&shape.name, theme.tick_size))
        .fold(0.0, f64::max);
    let rotate_labels = theme.is_adaptive() && widest_group > slot * 0.86;

    for (gi, shape) in shapes.iter().enumerate() {
        let centre = canvas.margin.left + slot * (gi as f64 + 0.5);
        let peak = shape
            .points
            .iter()
            .map(|point| point.1)
            .fold(f64::MIN, f64::max)
            .max(1e-9);
        let half = slot * 0.42;
        let mut outline = Vec::with_capacity(shape.points.len() * 2);
        for &(value, density) in &shape.points {
            outline.push(format!(
                "{:.1},{:.1}",
                centre + density / peak * half,
                y_scale.map(value)
            ));
        }
        for &(value, density) in shape.points.iter().rev() {
            outline.push(format!(
                "{:.1},{:.1}",
                centre - density / peak * half,
                y_scale.map(value)
            ));
        }
        let colour = if seurat_theme {
            SEURAT_PALETTE[gi % SEURAT_PALETTE.len()]
        } else {
            PALETTE[gi % PALETTE.len()]
        };
        canvas.elements.push(format!(
            r#"<polygon points="{}" fill="{}" opacity="0.65" stroke="{}" stroke-width="1" />"#,
            outline.join(" "),
            colour,
            if seurat_theme { "#333333" } else { colour }
        ));
        let my = y_scale.map(shape.median);
        canvas.add_line(
            centre - half * 0.5,
            my,
            centre + half * 0.5,
            my,
            "#333333",
            2.0,
        );
        let label_y = canvas.margin.top + canvas.plot_height() + 16.0;
        if rotate_labels {
            canvas.add_text_rotated(
                centre,
                label_y - 3.0,
                &shape.name,
                40.0,
                "start",
                theme.tick_size,
            );
        } else {
            canvas.add_text(
                centre,
                label_y,
                &shape.name,
                "middle",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }
    canvas.draw_y_axis(&y_scale, &value_col);
    canvas.set_accessible_description(format!(
        "Violin plot of {value_col} for {} groups; each shape is a Gaussian kernel density and each horizontal mark is the group median.",
        shapes.len()
    ));
    if theme.is_adaptive() {
        canvas.draw_title(&title);
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    } else {
        canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    }
    Ok(canvas.render())
}

fn violin_plot_spec_value(
    shapes: &[ViolinShape],
    variant: &str,
    value_label: &str,
    opts: &HashMap<String, Value>,
) -> Value {
    let data_rows = shapes
        .iter()
        .enumerate()
        .flat_map(|(group_index, shape)| {
            shape
                .points
                .iter()
                .enumerate()
                .map(move |(grid_index, &(value, density))| {
                    vec![
                        Value::Int(group_index as i64),
                        Value::Str(shape.name.clone()),
                        Value::Int(grid_index as i64),
                        Value::Float(value),
                        Value::Float(density),
                    ]
                })
        })
        .collect();
    let group_rows = shapes
        .iter()
        .enumerate()
        .map(|(index, shape)| {
            vec![
                Value::Int(index as i64),
                Value::Str(shape.name.clone()),
                Value::Int(shape.sample_count as i64),
                Value::Float(shape.bandwidth),
                Value::Float(shape.median),
                Value::Float(shape.input_min),
                Value::Float(shape.input_max),
            ]
        })
        .collect();
    let (default_title, default_width, default_height) = if variant == "wide" {
        ("Violin Plot", 600.0, 400.0)
    } else {
        ("Distribution", 640.0, 420.0)
    };
    let options = HashMap::from([
        ("variant".into(), Value::Str(variant.into())),
        ("value_label".into(), Value::Str(value_label.into())),
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", default_title).into()),
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
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        (
            "ylab".into(),
            Value::Str(get_opt_str(opts, "ylab", "Value").into()),
        ),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", default_width)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", default_height)),
        ),
    ]);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("violin".into())),
            ("plot".into(), Value::Str(variant.into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", default_title).into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    ["group_index", "group", "grid_index", "value", "density"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    data_rows,
                )),
            ),
            (
                "groups".into(),
                Value::Table(Table::new(
                    [
                        "group_index",
                        "group",
                        "sample_count",
                        "bandwidth",
                        "median",
                        "input_min",
                        "input_max",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    group_rows,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "builtin".into(),
                            Value::Str(if variant == "wide" {
                                "violin".into()
                            } else {
                                "violin_plot".into()
                            }),
                        ),
                        ("groups".into(), Value::Int(shapes.len() as i64)),
                        (
                            "samples".into(),
                            Value::Int(
                                shapes.iter().map(|shape| shape.sample_count).sum::<usize>() as i64,
                            ),
                        ),
                        (
                            "grid_points".into(),
                            Value::Int(
                                shapes.iter().map(|shape| shape.points.len()).sum::<usize>() as i64,
                            ),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

pub(crate) fn is_violin_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "violin")
    )
}

pub(crate) fn render_violin_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_violin_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 violin Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin specification field 'data' must be Table",
                None,
            ))
        }
    };
    let summaries = match map.get("groups") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin specification field 'groups' must be Table",
                None,
            ))
        }
    };
    for required in ["group_index", "group", "grid_index", "value", "density"] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() violin data is missing '{required}'"),
                None,
            ));
        }
    }
    for required in [
        "group_index",
        "group",
        "sample_count",
        "bandwidth",
        "median",
        "input_min",
        "input_max",
    ] {
        if summaries.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() violin groups are missing '{required}'"),
                None,
            ));
        }
    }
    if summaries.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() violin specification has no groups",
            None,
        ));
    }
    let mut shapes = Vec::with_capacity(summaries.num_rows());
    let summary_index = |name: &str| summaries.col_index(name).unwrap();
    for (expected, row) in summaries.rows.iter().enumerate() {
        let index = row[summary_index("group_index")]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() violin group_index must be numeric",
                    None,
                )
            })?;
        if index != expected {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin group_index values must be contiguous and ordered",
                None,
            ));
        }
        let number = |name: &str| -> Result<f64> {
            row[summary_index(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() violin group field '{name}' must be numeric"),
                    None,
                )
            })
        };
        shapes.push(ViolinShape {
            name: format!("{}", row[summary_index("group")]),
            sample_count: number("sample_count")? as usize,
            bandwidth: number("bandwidth")?,
            median: number("median")?,
            input_min: number("input_min")?,
            input_max: number("input_max")?,
            points: Vec::new(),
        });
    }
    let group_index = data.col_index("group_index").unwrap();
    let grid_index = data.col_index("grid_index").unwrap();
    let value_index = data.col_index("value").unwrap();
    let density_index = data.col_index("density").unwrap();
    for row in &data.rows {
        let group = row[group_index]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() violin data group_index must be numeric",
                    None,
                )
            })?;
        let shape = shapes.get_mut(group).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin data references an unknown group_index",
                None,
            )
        })?;
        let expected_grid = shape.points.len();
        let actual_grid = row[grid_index]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() violin grid_index must be numeric",
                    None,
                )
            })?;
        if expected_grid != actual_grid {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin grid_index values must be contiguous within each group",
                None,
            ));
        }
        let grid_value = row[value_index].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin grid values must be numeric",
                None,
            )
        })?;
        let density = row[density_index].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin densities must be numeric",
                None,
            )
        })?;
        if !grid_value.is_finite() || !density.is_finite() || density < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin grid values must be finite and densities non-negative",
                None,
            ));
        }
        shape.points.push((grid_value, density));
    }
    if shapes.iter().any(|shape| shape.points.len() < 2) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() violin specification needs at least two grid points per group",
            None,
        ));
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin specification field 'options' must be Record",
                None,
            ))
        }
    };
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in ["width", "height"] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    let variant = get_opt_str(&options, "variant", "long");
    let svg = match variant {
        "wide" => render_legacy_violin_svg(&shapes, &options)?,
        "long" => render_long_violin_svg(&shapes, &options)?,
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() unknown violin variant '{other}'"),
                None,
            ))
        }
    };
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Violin Plot");
    match format.as_str() {
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
            "render_plot() terminal violin output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown violin format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_violin(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    // Collect named groups of numeric data
    let groups: Vec<(String, Vec<f64>)> = match &args[0] {
        Value::List(_) => {
            let vals = nums_from_value(&args[0], "violin")?;
            vec![("data".to_string(), vals)]
        }
        Value::Table(table) => table
            .columns
            .iter()
            .map(|col| {
                let vals = extract_table_col(table, col).unwrap_or_default();
                let finite: Vec<f64> = vals.into_iter().filter(|v| v.is_finite()).collect();
                (col.clone(), finite)
            })
            .filter(|(_, v)| !v.is_empty())
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "violin() requires List or Table",
                None,
            ))
        }
    };
    if groups.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin() no data",
            None,
        ));
    }

    let shapes = groups
        .iter()
        .map(|(name, values)| violin_shape(name.clone(), values, 50))
        .collect::<Vec<_>>();

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec =
            violin_plot_spec_value(&shapes, "wide", get_opt_str(&opts, "ylab", "Value"), &opts);
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_violin_plot_spec_value(&spec, &opts);
    }

    if fmt == "svg" {
        return render_legacy_violin_svg(&shapes, &opts).map(Value::Str);
    }

    // ASCII: horizontal violin per group
    let bar_w = get_opt_usize(&opts, "width", 40);
    let mut out = String::from("  Violin Plot\n");
    let max_label = groups.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    for (name, vals) in &groups {
        let bw = silverman_bw(vals);
        let (_, kde_d) = kde(vals, bw, bar_w);
        let max_d = kde_d.iter().cloned().fold(0.0f64, f64::max);
        let bars: String = kde_d
            .iter()
            .map(|&d| {
                let t = if max_d > 0.0 { d / max_d } else { 0.0 };
                let idx = (t * 7.0).round() as usize;
                ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][idx.min(7)]
            })
            .collect();
        out.push_str(&format!("  {:>w$}  {bars}\n", name, w = max_label));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 7. density ──────────────────────────────────────────────────

fn builtin_density(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let bw_opt = opts.get("bandwidth").and_then(|v| v.as_float());

    let groups: Vec<(String, Vec<f64>)> = match &args[0] {
        Value::List(_) => vec![("data".to_string(), nums_from_value(&args[0], "density")?)],
        Value::Table(table) => table
            .columns
            .iter()
            .filter_map(|col| {
                let vals: Vec<f64> = extract_table_col(table, col)
                    .ok()?
                    .into_iter()
                    .filter(|v| v.is_finite())
                    .collect();
                if vals.is_empty() {
                    None
                } else {
                    Some((col.clone(), vals))
                }
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "density() requires List or Table",
                None,
            ))
        }
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        let mut global_xs: Vec<f64> = Vec::new();
        let mut global_ys: Vec<f64> = Vec::new();
        let mut curves: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
        for (_, vals) in &groups {
            let bw = bw_opt.unwrap_or_else(|| silverman_bw(vals));
            let (kx, ky) = kde(vals, bw, 100);
            global_xs.extend(&kx);
            global_ys.extend(&ky);
            curves.push((kx, ky));
        }
        let xr = col_range(&global_xs);
        let yr = (0.0, col_range(&global_ys).1 * 1.1);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        for (gi, (kx, ky)) in curves.iter().enumerate() {
            let baseline = ys.map(0.0);
            let mut points = String::new();
            points.push_str(&format!("{:.1},{:.1} ", xs.map(kx[0]), baseline));
            for i in 0..kx.len() {
                points.push_str(&format!("{:.1},{:.1} ", xs.map(kx[i]), ys.map(ky[i])));
            }
            points.push_str(&format!(
                "{:.1},{:.1}",
                xs.map(*kx.last().unwrap()),
                baseline
            ));
            c.elements.push(format!(
                r#"<polygon points="{points}" fill="{}" opacity="0.4" />"#,
                PALETTE[gi % PALETTE.len()]
            ));
        }
        let dx = Scale {
            domain: xr,
            range: xr,
        };
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_x_axis(&dx, get_opt_str(&opts, "xlab", "Value"));
        c.draw_y_axis(&dy, get_opt_str(&opts, "ylab", "Density"));
        finish_themed_canvas(&mut c, &opts, "Density");
        return Ok(Value::Str(c.render()));
    }

    // ASCII: histogram-style density
    let width = get_opt_usize(&opts, "width", 60);
    let _height = get_opt_usize(&opts, "height", 16);
    let mut out = String::from("  Density\n");
    for (name, vals) in groups.iter() {
        let bw = bw_opt.unwrap_or_else(|| silverman_bw(vals));
        let (_kx, ky) = kde(vals, bw, width);
        let max_y = ky.iter().cloned().fold(0.0f64, f64::max);
        let bars: String = ky
            .iter()
            .map(|&y| {
                let t = if max_y > 0.0 { y / max_y } else { 0.0 };
                let idx = (t * 7.0).round() as usize;
                ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][idx.min(7)]
            })
            .collect();
        if groups.len() > 1 {
            out.push_str(&format!("  {name}:\n"));
        }
        out.push_str(&format!("  {bars}\n"));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 8. kaplan_meier ─────────────────────────────────────────────

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

#[derive(Clone, Debug)]
struct SurvivalStep {
    time: f64,
    n_risk: usize,
    n_event: usize,
    n_censor: usize,
    survival: f64,
    std_error: f64,
}

#[derive(Clone, Debug)]
struct SurvivalGroup {
    name: String,
    sample_count: usize,
    event_count: usize,
    censor_count: usize,
    median_survival: Option<f64>,
    steps: Vec<SurvivalStep>,
}

fn kaplan_meier_groups(table: &Table, opts: &HashMap<String, Value>) -> Result<Vec<SurvivalGroup>> {
    let times = extract_table_col(table, get_opt_str(opts, "time", "time"))?;
    let events = extract_table_col(table, get_opt_str(opts, "event", "event"))?;
    if times.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "kaplan_meier() requires at least one observation",
            None,
        ));
    }
    let labels = if let Some(Value::Str(column)) = opts.get("group") {
        extract_str_col(table, column)?
    } else {
        vec!["All".into(); times.len()]
    };
    let mut group_names = Vec::<String>::new();
    let mut grouped = Vec::<Vec<(f64, bool)>>::new();
    let mut lookup = HashMap::<String, usize>::new();
    for ((&time, &event), label) in times.iter().zip(&events).zip(labels) {
        if !time.is_finite() || time < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "kaplan_meier() times must be finite and non-negative",
                None,
            ));
        }
        if !event.is_finite() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "kaplan_meier() event values must be finite",
                None,
            ));
        }
        let next = group_names.len();
        let index = *lookup.entry(label.clone()).or_insert_with(|| {
            group_names.push(label);
            grouped.push(Vec::new());
            next
        });
        grouped[index].push((time, event >= 1.0));
    }
    let mut result = Vec::with_capacity(grouped.len());
    for (name, mut observations) in group_names.into_iter().zip(grouped) {
        observations.sort_by(|left, right| left.0.total_cmp(&right.0));
        let sample_count = observations.len();
        let mut at_risk = sample_count;
        let mut survival = 1.0;
        let mut greenwood_sum = 0.0;
        let mut steps = vec![SurvivalStep {
            time: 0.0,
            n_risk: sample_count,
            n_event: 0,
            n_censor: 0,
            survival,
            std_error: 0.0,
        }];
        let mut event_count = 0usize;
        let mut censor_count = 0usize;
        let mut median_survival = None;
        let mut index = 0usize;
        while index < observations.len() {
            let time = observations[index].0;
            let mut events_at_time = 0usize;
            let mut censored_at_time = 0usize;
            while index < observations.len() && observations[index].0 == time {
                if observations[index].1 {
                    events_at_time += 1;
                } else {
                    censored_at_time += 1;
                }
                index += 1;
            }
            if events_at_time > 0 {
                survival *= 1.0 - events_at_time as f64 / at_risk as f64;
                if at_risk > events_at_time {
                    greenwood_sum += events_at_time as f64
                        / (at_risk as f64 * (at_risk - events_at_time) as f64);
                }
                if median_survival.is_none() && survival <= 0.5 {
                    median_survival = Some(time);
                }
            }
            event_count += events_at_time;
            censor_count += censored_at_time;
            steps.push(SurvivalStep {
                time,
                n_risk: at_risk,
                n_event: events_at_time,
                n_censor: censored_at_time,
                survival,
                std_error: if survival > 0.0 {
                    survival * greenwood_sum.sqrt()
                } else {
                    0.0
                },
            });
            at_risk -= events_at_time + censored_at_time;
        }
        result.push(SurvivalGroup {
            name,
            sample_count,
            event_count,
            censor_count,
            median_survival,
            steps,
        });
    }
    Ok(result)
}

// ── 9. forest_plot ──────────────────────────────────────────────

fn survival_plot_spec_value(groups: &[SurvivalGroup], opts: &HashMap<String, Value>) -> Value {
    let mut rows = Vec::new();
    let mut summaries = Vec::new();
    for (group_index, group) in groups.iter().enumerate() {
        for (step_index, step) in group.steps.iter().enumerate() {
            rows.push(vec![
                Value::Int(group_index as i64),
                Value::Str(group.name.clone()),
                Value::Int(step_index as i64),
                Value::Float(step.time),
                Value::Int(step.n_risk as i64),
                Value::Int(step.n_event as i64),
                Value::Int(step.n_censor as i64),
                Value::Float(step.survival),
                Value::Float(step.std_error),
            ]);
        }
        summaries.push(vec![
            Value::Int(group_index as i64),
            Value::Str(group.name.clone()),
            Value::Int(group.sample_count as i64),
            Value::Int(group.event_count as i64),
            Value::Int(group.censor_count as i64),
            group
                .median_survival
                .map(Value::Float)
                .unwrap_or(Value::Nil),
        ]);
    }
    let title = get_opt_str(opts, "title", "Kaplan-Meier");
    let options = HashMap::from([
        ("title".into(), Value::Str(title.into())),
        (
            "subtitle".into(),
            Value::Str(get_opt_str(opts, "subtitle", "").into()),
        ),
        (
            "caption".into(),
            Value::Str(get_opt_str(opts, "caption", "").into()),
        ),
        (
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        (
            "xlabel".into(),
            Value::Str(get_opt_str(opts, "xlabel", "Time").into()),
        ),
        (
            "ylabel".into(),
            Value::Str(get_opt_str(opts, "ylabel", "Survival probability").into()),
        ),
        (
            "censor_marks".into(),
            Value::Bool(
                opts.get("censor_marks")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            ),
        ),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 640.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 440.0)),
        ),
    ]);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("survival".into())),
            ("plot".into(), Value::Str("kaplan_meier".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "group_index",
                        "group",
                        "step_index",
                        "time",
                        "n_risk",
                        "n_event",
                        "n_censor",
                        "survival",
                        "std_error",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "groups".into(),
                Value::Table(Table::new(
                    [
                        "group_index",
                        "group",
                        "sample_count",
                        "event_count",
                        "censor_count",
                        "median_survival",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    summaries,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("kaplan_meier".into())),
                        ("estimator".into(), Value::Str("product-limit".into())),
                        (
                            "tie_policy".into(),
                            Value::Str("events-and-censors-at-each-distinct-time".into()),
                        ),
                        ("standard_error".into(), Value::Str("Greenwood".into())),
                        (
                            "samples".into(),
                            Value::Int(
                                groups.iter().map(|group| group.sample_count).sum::<usize>() as i64,
                            ),
                        ),
                        ("groups".into(), Value::Int(groups.len() as i64)),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

fn render_survival_svg(groups: &[SurvivalGroup], opts: &HashMap<String, Value>) -> Result<String> {
    if groups.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() survival specification has no groups",
            None,
        ));
    }
    let width = get_opt_f64(opts, "width", 640.0);
    let height = get_opt_f64(opts, "height", 440.0);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    canvas.margin.left = 62.0_f64.min(width * 0.23);
    canvas.margin.right = if groups.len() > 1 {
        130.0_f64.min(width * 0.30)
    } else {
        20.0
    };
    canvas.margin.top = if subtitle.is_empty() { 52.0 } else { 70.0 };
    canvas.margin.bottom = if caption.is_empty() { 52.0 } else { 70.0 };
    let tmax = groups
        .iter()
        .flat_map(|group| group.steps.iter().map(|step| step.time))
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    let xs = Scale {
        domain: (0.0, tmax),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let ys = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let censor_marks = opts
        .get("censor_marks")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    for (group_index, group) in groups.iter().enumerate() {
        let colour = PALETTE[group_index % PALETTE.len()];
        let mut previous_survival = 1.0;
        let mut path = format!("M {:.2} {:.2}", xs.map(0.0), ys.map(previous_survival));
        let mut censor_path = String::new();
        for step in group.steps.iter().skip(1) {
            path.push_str(&format!(" H {:.2}", xs.map(step.time)));
            if step.survival != previous_survival {
                path.push_str(&format!(" V {:.2}", ys.map(step.survival)));
            }
            if censor_marks && step.n_censor > 0 {
                let x = xs.map(step.time);
                let y = ys.map(step.survival);
                censor_path.push_str(&format!(
                    " M {:.2} {:.2} H {:.2} M {:.2} {:.2} V {:.2}",
                    x - 4.0,
                    y,
                    x + 4.0,
                    x,
                    y - 4.0,
                    y + 4.0
                ));
            }
            previous_survival = step.survival;
        }
        path.push_str(&format!(" H {:.2}", xs.map(tmax)));
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{colour}" stroke-width="2" />"#
        ));
        if !censor_path.is_empty() {
            canvas.elements.push(format!(
                r#"<path d="{censor_path}" fill="none" stroke="{colour}" stroke-width="1.5" />"#
            ));
        }
        if groups.len() > 1 {
            let legend_x = canvas.margin.left + canvas.plot_width() + 12.0;
            let legend_y = canvas.margin.top + 16.0 + group_index as f64 * 20.0;
            canvas.add_line(legend_x, legend_y, legend_x + 18.0, legend_y, colour, 2.0);
            canvas.add_text(legend_x + 24.0, legend_y + 4.0, &group.name, "start", 10.0);
        }
    }
    canvas.draw_x_axis(
        &Scale {
            domain: (0.0, tmax),
            range: (0.0, tmax),
        },
        get_opt_str(opts, "xlabel", "Time"),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        },
        get_opt_str(opts, "ylabel", "Survival probability"),
    );
    canvas.draw_title(get_opt_str(opts, "title", "Kaplan-Meier"));
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Kaplan-Meier product-limit curves for {} group(s), including censor marks.",
        groups.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_survival_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "survival")
    )
}

pub(crate) fn render_survival_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_survival_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 survival Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "group_index",
        "group",
        "step_index",
        "time",
        "n_risk",
        "n_event",
        "n_censor",
        "survival",
        "std_error",
    ] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() survival data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| data.col_index(name).unwrap();
    let mut groups = Vec::<SurvivalGroup>::new();
    for row in &data.rows {
        let number = |name: &str| -> Result<f64> {
            row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() survival field '{name}' must be numeric"),
                    None,
                )
            })
        };
        let group_index =
            frozen_nonnegative_integer(&row[column("group_index")], "survival", "group_index")?;
        if group_index > groups.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival group_index values must be contiguous",
                None,
            ));
        }
        if group_index == groups.len() {
            groups.push(SurvivalGroup {
                name: format!("{}", row[column("group")]),
                sample_count: 0,
                event_count: 0,
                censor_count: 0,
                median_survival: None,
                steps: Vec::new(),
            });
        }
        let group = &mut groups[group_index];
        if frozen_nonnegative_integer(&row[column("step_index")], "survival", "step_index")?
            != group.steps.len()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival step_index values must be contiguous within each group",
                None,
            ));
        }
        let step = SurvivalStep {
            time: number("time")?,
            n_risk: frozen_nonnegative_integer(&row[column("n_risk")], "survival", "n_risk")?,
            n_event: frozen_nonnegative_integer(&row[column("n_event")], "survival", "n_event")?,
            n_censor: frozen_nonnegative_integer(&row[column("n_censor")], "survival", "n_censor")?,
            survival: number("survival")?,
            std_error: number("std_error")?,
        };
        if !step.time.is_finite()
            || step.time < 0.0
            || !step.survival.is_finite()
            || !(0.0..=1.0).contains(&step.survival)
            || !step.std_error.is_finite()
            || step.std_error < 0.0
            || group.steps.last().is_some_and(|previous| {
                step.time < previous.time || step.survival > previous.survival
            })
            || step.n_event + step.n_censor > step.n_risk
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival steps must have finite non-decreasing times and non-increasing probabilities in [0, 1]",
                None,
            ));
        }
        if let Some(previous) = group.steps.last() {
            let expected_risk = previous
                .n_risk
                .saturating_sub(previous.n_event + previous.n_censor);
            if step.n_risk != expected_risk {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() survival risk sets do not follow the preceding event/censor counts",
                    None,
                ));
            }
            let expected_survival = if step.n_risk == 0 {
                previous.survival
            } else {
                previous.survival * (1.0 - step.n_event as f64 / step.n_risk as f64)
            };
            if (step.survival - expected_survival).abs() > 1e-10 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() survival probability does not match its frozen risk/event counts",
                    None,
                ));
            }
        } else if step.time != 0.0
            || step.n_event != 0
            || step.n_censor != 0
            || step.survival != 1.0
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival curves must begin at time 0 with probability 1",
                None,
            ));
        }
        group.sample_count = group.sample_count.max(step.n_risk);
        group.event_count += step.n_event;
        group.censor_count += step.n_censor;
        if group.median_survival.is_none() && step.survival <= 0.5 {
            group.median_survival = Some(step.time);
        }
        group.steps.push(step);
    }
    if groups.is_empty() || groups.iter().any(|group| group.steps.is_empty()) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() survival specification has no complete curve",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "survival")?;
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_survival_svg(&groups, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Kaplan-Meier");
    finish_frozen_bio_plot(value, render_options, title, "survival", svg)
}

fn builtin_kaplan_meier(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "kaplan_meier")?;
    let opts = parse_options(&args);
    let groups = kaplan_meier_groups(table, &opts)?;
    let specification = survival_plot_spec_value(&groups, &opts);
    render_survival_plot_spec_value(&specification, &opts)
}

// ── 10. roc_curve ───────────────────────────────────────────────

#[derive(Clone, Debug)]
struct ForestInterval {
    source_row: usize,
    label: String,
    estimate: f64,
    lower: f64,
    upper: f64,
    weight: f64,
}

fn forest_intervals(table: &Table, opts: &HashMap<String, Value>) -> Result<Vec<ForestInterval>> {
    let labels = extract_str_col(table, get_opt_str(opts, "label", "label"))?;
    let estimates = extract_table_col(table, get_opt_str(opts, "estimate", "estimate"))?;
    let lowers = extract_table_col(table, get_opt_str(opts, "lower", "lower"))?;
    let uppers = extract_table_col(table, get_opt_str(opts, "upper", "upper"))?;
    let weights = if let Some(Value::Str(column)) = opts.get("weight") {
        extract_table_col(table, column)?
    } else {
        vec![1.0; labels.len()]
    };
    if labels.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "forest_plot() requires at least one interval",
            None,
        ));
    }
    let log_scale = get_opt_str(opts, "scale", "linear").eq_ignore_ascii_case("log");
    if !log_scale && !get_opt_str(opts, "scale", "linear").eq_ignore_ascii_case("linear") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "forest_plot() scale must be linear or log",
            None,
        ));
    }
    labels
        .into_iter()
        .zip(estimates)
        .zip(lowers)
        .zip(uppers)
        .zip(weights)
        .enumerate()
        .map(|(source_row, ((((label, estimate), lower), upper), weight))| {
            if !estimate.is_finite()
                || !lower.is_finite()
                || !upper.is_finite()
                || !weight.is_finite()
                || weight <= 0.0
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "forest_plot() estimates, interval bounds and weights must be finite; weights must be positive",
                    None,
                ));
            }
            if lower > estimate || estimate > upper {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "forest_plot() row {source_row} must satisfy lower <= estimate <= upper"
                    ),
                    None,
                ));
            }
            if log_scale && lower <= 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "forest_plot() log scale requires positive estimates and interval bounds",
                    None,
                ));
            }
            Ok(ForestInterval {
                source_row,
                label,
                estimate,
                lower,
                upper,
                weight,
            })
        })
        .collect()
}

fn forest_domain(
    intervals: &[ForestInterval],
    reference: f64,
    log_scale: bool,
) -> ((f64, f64), (f64, f64)) {
    let transform = |value: f64| if log_scale { value.ln() } else { value };
    let raw_min = intervals
        .iter()
        .map(|interval| interval.lower)
        .fold(reference, f64::min);
    let raw_max = intervals
        .iter()
        .map(|interval| interval.upper)
        .fold(reference, f64::max);
    let transformed_min = transform(raw_min);
    let transformed_max = transform(raw_max);
    let padding = ((transformed_max - transformed_min).abs() * 0.06).max(0.1);
    (
        (raw_min, raw_max),
        (transformed_min - padding, transformed_max + padding),
    )
}

fn forest_plot_spec_value(intervals: &[ForestInterval], opts: &HashMap<String, Value>) -> Value {
    let scale = get_opt_str(opts, "scale", "linear").to_ascii_lowercase();
    let reference_default = if scale == "log" { 1.0 } else { 0.0 };
    let reference = get_opt_f64(opts, "reference", reference_default);
    let (raw_domain, display_domain) = forest_domain(intervals, reference, scale.as_str() == "log");
    let title = get_opt_str(opts, "title", "Forest Plot");
    let rows = intervals
        .iter()
        .enumerate()
        .map(|(display_row, interval)| {
            vec![
                Value::Int(display_row as i64),
                Value::Int(interval.source_row as i64),
                Value::Str(interval.label.clone()),
                Value::Float(interval.estimate),
                Value::Float(interval.lower),
                Value::Float(interval.upper),
                Value::Float(interval.weight),
            ]
        })
        .collect();
    let height_default = (intervals.len() as f64 * 32.0 + 110.0).clamp(220.0, 1200.0);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("forest".into())),
            ("plot".into(), Value::Str("forest_plot".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "display_row",
                        "source_row",
                        "label",
                        "estimate",
                        "lower",
                        "upper",
                        "weight",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Effect size").into()),
                        ),
                        ("scale".into(), Value::Str(scale)),
                        ("reference".into(), Value::Float(reference)),
                        ("raw_min".into(), Value::Float(raw_domain.0)),
                        ("raw_max".into(), Value::Float(raw_domain.1)),
                        ("display_min".into(), Value::Float(display_domain.0)),
                        ("display_max".into(), Value::Float(display_domain.1)),
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 680.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", height_default)),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("forest_plot".into())),
                        ("intervals".into(), Value::Int(intervals.len() as i64)),
                        (
                            "marker_area".into(),
                            Value::Str("proportional-to-weight".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

fn render_forest_svg(
    intervals: &[ForestInterval],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    if intervals.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() forest specification has no intervals",
            None,
        ));
    }
    let width = get_opt_f64(opts, "width", 680.0);
    let height_default = (intervals.len() as f64 * 32.0 + 110.0).clamp(220.0, 1200.0);
    let height = get_opt_f64(opts, "height", height_default);
    let scale = get_opt_str(opts, "scale", "linear");
    let log_scale = scale == "log";
    let reference = get_opt_f64(opts, "reference", if log_scale { 1.0 } else { 0.0 });
    if !reference.is_finite() || (log_scale && reference <= 0.0) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() forest reference must be finite and positive on a log scale",
            None,
        ));
    }
    let transform = |value: f64| if log_scale { value.ln() } else { value };
    let display_domain = (
        get_opt_f64(opts, "display_min", f64::NAN),
        get_opt_f64(opts, "display_max", f64::NAN),
    );
    if !display_domain.0.is_finite()
        || !display_domain.1.is_finite()
        || display_domain.0 >= display_domain.1
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() forest display domain must be finite and increasing",
            None,
        ));
    }
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let widest_label = intervals
        .iter()
        .map(|interval| estimate_text_width(&interval.label, theme.tick_size))
        .fold(0.0, f64::max);
    canvas.margin.left = (widest_label + 18.0).clamp(82.0, width * 0.38);
    canvas.margin.right = 20.0;
    canvas.margin.top = if subtitle.is_empty() { 54.0 } else { 72.0 };
    canvas.margin.bottom = if caption.is_empty() { 54.0 } else { 72.0 };
    let xs = Scale {
        domain: display_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let row_height = canvas.plot_height() / intervals.len() as f64;
    canvas.add_line(
        xs.map(transform(reference)),
        canvas.margin.top,
        xs.map(transform(reference)),
        canvas.margin.top + canvas.plot_height(),
        theme.grid_colour,
        1.2,
    );
    let max_weight = intervals
        .iter()
        .map(|interval| interval.weight)
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    for (row, interval) in intervals.iter().enumerate() {
        let y = canvas.margin.top + (row as f64 + 0.5) * row_height;
        canvas.add_line(
            xs.map(transform(interval.lower)),
            y,
            xs.map(transform(interval.upper)),
            y,
            PALETTE[0],
            2.0,
        );
        canvas.add_line(
            xs.map(transform(interval.lower)),
            y - 4.0,
            xs.map(transform(interval.lower)),
            y + 4.0,
            PALETTE[0],
            1.2,
        );
        canvas.add_line(
            xs.map(transform(interval.upper)),
            y - 4.0,
            xs.map(transform(interval.upper)),
            y + 4.0,
            PALETTE[0],
            1.2,
        );
        let radius = 3.5 + 4.5 * (interval.weight / max_weight).sqrt();
        canvas.add_circle(xs.map(transform(interval.estimate)), y, radius, PALETTE[0]);
        canvas.add_text(
            canvas.margin.left - 8.0,
            y + theme.tick_size * 0.35,
            &interval.label,
            "end",
            theme.tick_size,
        );
    }
    if log_scale {
        let y = canvas.margin.top + canvas.plot_height();
        canvas.add_line(
            canvas.margin.left,
            y,
            canvas.margin.left + canvas.plot_width(),
            y,
            theme.axis_colour,
            1.0,
        );
        let divisions = if width < 400.0 { 2 } else { 4 };
        let mut ticks = (0..=divisions)
            .map(|index| {
                display_domain.0
                    + (display_domain.1 - display_domain.0) * index as f64 / divisions as f64
            })
            .collect::<Vec<_>>();
        ticks.push(transform(reference));
        ticks.sort_by(f64::total_cmp);
        ticks.dedup_by(|left, right| (*left - *right).abs() < 1e-8);
        if width < 400.0 {
            let reference_tick = transform(reference);
            let reference_x = xs.map(reference_tick);
            ticks.retain(|tick| {
                (*tick - reference_tick).abs() < 1e-8 || (xs.map(*tick) - reference_x).abs() >= 42.0
            });
        }
        for tick in ticks {
            let x = xs.map(tick);
            canvas.add_line(x, y, x, y + 5.0, theme.axis_colour, 1.0);
            canvas.add_text(
                x,
                y + 18.0,
                &format!("{:.2}", tick.exp()),
                "middle",
                theme.tick_size,
            );
        }
        canvas.add_text(
            canvas.margin.left + canvas.plot_width() / 2.0,
            height - 12.0,
            get_opt_str(opts, "xlabel", "Effect size"),
            "middle",
            theme.axis_title_size,
        );
    } else {
        canvas.draw_x_axis(
            &Scale {
                domain: display_domain,
                range: display_domain,
            },
            get_opt_str(opts, "xlabel", "Effect size"),
        );
    }
    canvas.draw_title(get_opt_str(opts, "title", "Forest Plot"));
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Forest plot of {} estimates and confidence intervals; marker area is proportional to weight and the reference is {reference}.",
        intervals.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_forest_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "forest")
    )
}

pub(crate) fn render_forest_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_forest_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 forest Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() forest specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "display_row",
        "source_row",
        "label",
        "estimate",
        "lower",
        "upper",
        "weight",
    ] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() forest data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| data.col_index(name).unwrap();
    let options = frozen_spec_options(map, render_options, "forest")?;
    let log_scale = get_opt_str(&options, "scale", "linear") == "log";
    let mut intervals = Vec::with_capacity(data.num_rows());
    for (expected_row, row) in data.rows.iter().enumerate() {
        let number = |name: &str| -> Result<f64> {
            row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() forest field '{name}' must be numeric"),
                    None,
                )
            })
        };
        if frozen_nonnegative_integer(&row[column("display_row")], "forest", "display_row")?
            != expected_row
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() forest display_row values must be contiguous and ordered",
                None,
            ));
        }
        let interval = ForestInterval {
            source_row: frozen_nonnegative_integer(
                &row[column("source_row")],
                "forest",
                "source_row",
            )?,
            label: format!("{}", row[column("label")]),
            estimate: number("estimate")?,
            lower: number("lower")?,
            upper: number("upper")?,
            weight: number("weight")?,
        };
        if !interval.estimate.is_finite()
            || !interval.lower.is_finite()
            || !interval.upper.is_finite()
            || !interval.weight.is_finite()
            || interval.weight <= 0.0
            || interval.lower > interval.estimate
            || interval.estimate > interval.upper
            || (log_scale && interval.lower <= 0.0)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() forest intervals must be finite, ordered, positive-weight, and positive on a log scale",
                None,
            ));
        }
        intervals.push(interval);
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_forest_svg(&intervals, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Forest Plot");
    finish_frozen_bio_plot(value, render_options, title, "forest", svg)
}

fn builtin_forest_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "forest_plot")?;
    let opts = parse_options(&args);
    let intervals = forest_intervals(table, &opts)?;
    let scale = get_opt_str(&opts, "scale", "linear");
    let reference = get_opt_f64(&opts, "reference", if scale == "log" { 1.0 } else { 0.0 });
    if !reference.is_finite() || (scale == "log" && reference <= 0.0) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "forest_plot() reference must be finite and positive on a log scale",
            None,
        ));
    }
    let specification = forest_plot_spec_value(&intervals, &opts);
    render_forest_plot_spec_value(&specification, &opts)
}

// ── 11. clustered_heatmap ───────────────────────────────────────

#[derive(Clone, Debug)]
struct RocPoint {
    threshold: Option<f64>,
    fpr: f64,
    tpr: f64,
    tp: Option<usize>,
    fp: Option<usize>,
    tn: Option<usize>,
    fn_count: Option<usize>,
}

fn roc_geometry(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<(Vec<RocPoint>, f64, String, usize)> {
    let precomputed = table.col_index("fpr").is_some() && table.col_index("tpr").is_some();
    let (points, observations) = if precomputed {
        let fprs = extract_table_col(table, "fpr")?;
        let tprs = extract_table_col(table, "tpr")?;
        if fprs.is_empty() || fprs.len() != tprs.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() precomputed fpr and tpr columns must be non-empty and equal length",
                None,
            ));
        }
        let mut points = Vec::with_capacity(fprs.len());
        for (index, (&fpr, &tpr)) in fprs.iter().zip(&tprs).enumerate() {
            if !fpr.is_finite()
                || !tpr.is_finite()
                || !(0.0..=1.0).contains(&fpr)
                || !(0.0..=1.0).contains(&tpr)
                || points
                    .last()
                    .is_some_and(|previous: &RocPoint| fpr < previous.fpr || tpr < previous.tpr)
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "roc_curve() precomputed point {index} must be finite, within [0, 1], and monotone in fpr/tpr"
                    ),
                    None,
                ));
            }
            points.push(RocPoint {
                threshold: None,
                fpr,
                tpr,
                tp: None,
                fp: None,
                tn: None,
                fn_count: None,
            });
        }
        (points, fprs.len())
    } else {
        let scores = extract_table_col(table, get_opt_str(opts, "score", "score"))?;
        let labels = extract_table_col(table, get_opt_str(opts, "label", "label"))?;
        if scores.is_empty() || scores.len() != labels.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() score and label columns must be non-empty and equal length",
                None,
            ));
        }
        if scores.iter().any(|value| !value.is_finite())
            || labels.iter().any(|value| !value.is_finite())
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() scores and labels must be finite",
                None,
            ));
        }
        let positives = labels.iter().filter(|&&label| label >= 1.0).count();
        let negatives = labels.len() - positives;
        if positives == 0 || negatives == 0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() requires at least one positive and one negative observation",
                None,
            ));
        }
        let mut order = (0..scores.len()).collect::<Vec<_>>();
        order.sort_by(|&left, &right| scores[right].total_cmp(&scores[left]));
        let mut tp = 0usize;
        let mut fp = 0usize;
        let mut points = vec![RocPoint {
            threshold: None,
            fpr: 0.0,
            tpr: 0.0,
            tp: Some(0),
            fp: Some(0),
            tn: Some(negatives),
            fn_count: Some(positives),
        }];
        let mut index = 0usize;
        while index < order.len() {
            let threshold = scores[order[index]];
            while index < order.len() && scores[order[index]] == threshold {
                if labels[order[index]] >= 1.0 {
                    tp += 1;
                } else {
                    fp += 1;
                }
                index += 1;
            }
            points.push(RocPoint {
                threshold: Some(threshold),
                fpr: fp as f64 / negatives as f64,
                tpr: tp as f64 / positives as f64,
                tp: Some(tp),
                fp: Some(fp),
                tn: Some(negatives - fp),
                fn_count: Some(positives - tp),
            });
        }
        (points, scores.len())
    };
    let fprs = points.iter().map(|point| point.fpr).collect::<Vec<_>>();
    let tprs = points.iter().map(|point| point.tpr).collect::<Vec<_>>();
    let auc_override = opts.get("auc").and_then(Value::as_float);
    let auc = auc_override.unwrap_or_else(|| trapz_auc(&fprs, &tprs));
    if !auc.is_finite() || !(0.0..=1.0).contains(&auc) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "roc_curve() auc must be finite and within [0, 1]",
            None,
        ));
    }
    Ok((
        points,
        auc,
        if auc_override.is_some() {
            "option".into()
        } else {
            "trapezoidal".into()
        },
        observations,
    ))
}

fn roc_plot_spec_value(
    points: &[RocPoint],
    auc: f64,
    auc_source: &str,
    observations: usize,
    opts: &HashMap<String, Value>,
) -> Value {
    let rows = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            vec![
                Value::Int(index as i64),
                point.threshold.map(Value::Float).unwrap_or(Value::Nil),
                Value::Float(point.fpr),
                Value::Float(point.tpr),
                point
                    .tp
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
                point
                    .fp
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
                point
                    .tn
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
                point
                    .fn_count
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "ROC Curve");
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("roc".into())),
            ("plot".into(), Value::Str("roc_curve".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "threshold",
                        "fpr",
                        "tpr",
                        "tp",
                        "fp",
                        "tn",
                        "fn",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "False positive rate").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "True positive rate").into()),
                        ),
                        ("auc".into(), Value::Float(auc)),
                        ("auc_source".into(), Value::Str(auc_source.into())),
                        (
                            "show_auc".into(),
                            Value::Bool(
                                opts.get("show_auc")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(true),
                            ),
                        ),
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 560.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 520.0)),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("roc_curve".into())),
                        (
                            "input".into(),
                            Value::Str(
                                if points.iter().all(|point| point.tp.is_none()) {
                                    "precomputed"
                                } else {
                                    "raw-scores"
                                }
                                .into(),
                            ),
                        ),
                        ("observations".into(), Value::Int(observations as i64)),
                        (
                            "tie_policy".into(),
                            Value::Str("simultaneous-at-distinct-score-threshold".into()),
                        ),
                        ("auc_method".into(), Value::Str(auc_source.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

fn render_roc_svg(points: &[RocPoint], opts: &HashMap<String, Value>) -> Result<String> {
    if points.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() ROC specification needs at least two points",
            None,
        ));
    }
    let width = get_opt_f64(opts, "width", 560.0);
    let height = get_opt_f64(opts, "height", 520.0);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    canvas.margin.left = 62.0_f64.min(width * 0.22);
    canvas.margin.right = 22.0;
    canvas.margin.top = if subtitle.is_empty() { 58.0 } else { 76.0 };
    canvas.margin.bottom = if caption.is_empty() { 56.0 } else { 72.0 };
    let xs = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let ys = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.add_line(
        xs.map(0.0),
        ys.map(0.0),
        xs.map(1.0),
        ys.map(1.0),
        theme.grid_colour,
        1.2,
    );
    let mut area = vec![format!("{:.2},{:.2}", xs.map(points[0].fpr), ys.map(0.0))];
    area.extend(
        points
            .iter()
            .map(|point| format!("{:.2},{:.2}", xs.map(point.fpr), ys.map(point.tpr))),
    );
    area.push(format!(
        "{:.2},{:.2}",
        xs.map(points.last().unwrap().fpr),
        ys.map(0.0)
    ));
    canvas.elements.push(format!(
        r#"<polygon points="{}" fill="{}" opacity="0.16" />"#,
        area.join(" "),
        PALETTE[0]
    ));
    let line = points
        .iter()
        .map(|point| format!("{:.2},{:.2}", xs.map(point.fpr), ys.map(point.tpr)))
        .collect::<Vec<_>>()
        .join(" ");
    canvas.elements.push(format!(
        r#"<polyline points="{line}" fill="none" stroke="{}" stroke-width="2.2" />"#,
        PALETTE[0]
    ));
    let axis = Scale {
        domain: (0.0, 1.0),
        range: (0.0, 1.0),
    };
    canvas.draw_x_axis(&axis, get_opt_str(opts, "xlabel", "False positive rate"));
    canvas.draw_y_axis(&axis, get_opt_str(opts, "ylabel", "True positive rate"));
    let auc = get_opt_f64(opts, "auc", f64::NAN);
    if !auc.is_finite() || !(0.0..=1.0).contains(&auc) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() ROC auc must be finite and within [0, 1]",
            None,
        ));
    }
    if opts
        .get("show_auc")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        canvas.add_text(
            canvas.margin.left + canvas.plot_width() - 6.0,
            canvas.margin.top + 18.0,
            &format!("AUC = {auc:.3}"),
            "end",
            theme.axis_title_size,
        );
    }
    canvas.draw_title(get_opt_str(opts, "title", "ROC Curve"));
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Receiver operating characteristic curve with {} frozen threshold points and trapezoidal area {auc:.4}.",
        points.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_roc_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "roc")
    )
}

pub(crate) fn render_roc_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_roc_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 ROC Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "threshold",
        "fpr",
        "tpr",
        "tp",
        "fp",
        "tn",
        "fn",
    ] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() ROC data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| data.col_index(name).unwrap();
    let optional_count = |row: &[Value], name: &str| -> Result<Option<usize>> {
        match &row[column(name)] {
            Value::Nil => Ok(None),
            value => frozen_nonnegative_integer(value, "ROC", name).map(Some),
        }
    };
    let mut points = Vec::with_capacity(data.num_rows());
    for (expected, row) in data.rows.iter().enumerate() {
        let number = |name: &str| -> Result<f64> {
            row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() ROC field '{name}' must be numeric"),
                    None,
                )
            })
        };
        if frozen_nonnegative_integer(&row[column("point_index")], "ROC", "point_index")?
            != expected
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC point_index values must be contiguous and ordered",
                None,
            ));
        }
        let threshold = match &row[column("threshold")] {
            Value::Nil => None,
            value => Some(value.as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() ROC threshold must be numeric or Nil",
                    None,
                )
            })?),
        };
        let point = RocPoint {
            threshold,
            fpr: number("fpr")?,
            tpr: number("tpr")?,
            tp: optional_count(row, "tp")?,
            fp: optional_count(row, "fp")?,
            tn: optional_count(row, "tn")?,
            fn_count: optional_count(row, "fn")?,
        };
        if !point.fpr.is_finite()
            || !point.tpr.is_finite()
            || !(0.0..=1.0).contains(&point.fpr)
            || !(0.0..=1.0).contains(&point.tpr)
            || point
                .threshold
                .is_some_and(|threshold| !threshold.is_finite())
            || points.last().is_some_and(|previous: &RocPoint| {
                point.fpr < previous.fpr || point.tpr < previous.tpr
            })
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC points must be finite, monotone, and within [0, 1]",
                None,
            ));
        }
        points.push(point);
    }
    let raw_counts = points.iter().any(|point| point.tp.is_some());
    if raw_counts {
        if points.iter().any(|point| {
            point.tp.is_none()
                || point.fp.is_none()
                || point.tn.is_none()
                || point.fn_count.is_none()
        }) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC confusion counts must be present for every raw-score point",
                None,
            ));
        }
        let positives = points[0].tp.unwrap() + points[0].fn_count.unwrap();
        let negatives = points[0].fp.unwrap() + points[0].tn.unwrap();
        if positives == 0 || negatives == 0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC confusion counts require both classes",
                None,
            ));
        }
        for point in &points {
            let tp = point.tp.unwrap();
            let fp = point.fp.unwrap();
            if tp + point.fn_count.unwrap() != positives
                || fp + point.tn.unwrap() != negatives
                || (point.tpr - tp as f64 / positives as f64).abs() > 1e-10
                || (point.fpr - fp as f64 / negatives as f64).abs() > 1e-10
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() ROC rates do not match their frozen confusion counts",
                    None,
                ));
            }
        }
    }
    let options = frozen_spec_options(map, render_options, "ROC")?;
    let auc = get_opt_f64(&options, "auc", f64::NAN);
    if get_opt_str(&options, "auc_source", "trapezoidal") == "trapezoidal" {
        let fprs = points.iter().map(|point| point.fpr).collect::<Vec<_>>();
        let tprs = points.iter().map(|point| point.tpr).collect::<Vec<_>>();
        if !auc.is_finite() || (auc - trapz_auc(&fprs, &tprs)).abs() > 1e-10 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC auc does not match its frozen trapezoidal curve",
                None,
            ));
        }
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_roc_svg(&points, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("ROC Curve");
    finish_frozen_bio_plot(value, render_options, title, "ROC", svg)
}

fn builtin_roc_curve(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "roc_curve")?;
    let opts = parse_options(&args);
    let (points, auc, auc_source, observations) = roc_geometry(table, &opts)?;
    let specification = roc_plot_spec_value(&points, auc, &auc_source, observations, &opts);
    render_roc_plot_spec_value(&specification, &opts)
}

#[derive(Clone, Copy, Debug)]
enum HeatmapLinkage {
    Complete,
    Average,
    Single,
    WardD2,
}

impl HeatmapLinkage {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "complete" => Ok(Self::Complete),
            "average" | "upgma" => Ok(Self::Average),
            "single" => Ok(Self::Single),
            "ward" | "ward.d2" | "ward_d2" => Ok(Self::WardD2),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "clustered_heatmap() linkage must be complete, average, single, or ward.D2; got '{value}'"
                ),
                None,
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Average => "average",
            Self::Single => "single",
            Self::WardD2 => "ward.D2",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum HeatmapDistance {
    Euclidean,
    Manhattan,
}

impl HeatmapDistance {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "euclidean" => Ok(Self::Euclidean),
            "manhattan" => Ok(Self::Manhattan),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "clustered_heatmap() distance must be euclidean or manhattan; got '{value}'"
                ),
                None,
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Euclidean => "euclidean",
            Self::Manhattan => "manhattan",
        }
    }
}

#[derive(Clone, Debug)]
struct HeatmapMerge {
    left: usize,
    right: usize,
    height: f64,
}

#[derive(Clone, Debug)]
struct HeatmapTree {
    merges: Vec<HeatmapMerge>,
    order: Vec<usize>,
}

fn heatmap_observation_distance(
    left: &[f64],
    right: &[f64],
    method: HeatmapDistance,
) -> Option<f64> {
    let dimensions = left.len().min(right.len());
    let mut compared = 0usize;
    let mut total = 0.0;
    for (&x, &y) in left.iter().zip(right.iter()) {
        if x.is_finite() && y.is_finite() {
            let delta = (x - y).abs();
            total += match method {
                HeatmapDistance::Euclidean => delta * delta,
                HeatmapDistance::Manhattan => delta,
            };
            compared += 1;
        }
    }
    if compared == 0 {
        return None;
    }
    // Match base R dist(): scale pairwise-complete distances when values are missing.
    let scaled = total * dimensions as f64 / compared as f64;
    Some(match method {
        HeatmapDistance::Euclidean => scaled.sqrt(),
        HeatmapDistance::Manhattan => scaled,
    })
}

fn hierarchical_heatmap_tree(
    data: &[Vec<f64>],
    distance_method: HeatmapDistance,
    linkage: HeatmapLinkage,
) -> Result<HeatmapTree> {
    let n = data.len();
    if n <= 1 {
        return Ok(HeatmapTree {
            merges: Vec::new(),
            order: (0..n).collect(),
        });
    }
    if matches!(linkage, HeatmapLinkage::WardD2)
        && !matches!(distance_method, HeatmapDistance::Euclidean)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "clustered_heatmap() ward.D2 linkage requires euclidean distance",
            None,
        ));
    }

    let capacity = 2 * n - 1;
    // Reuse one of the n original slots for each merged cluster. A 2n-by-2n
    // grid stores the same active distances but costs about four times more.
    let mut distances = vec![vec![f64::NAN; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = heatmap_observation_distance(&data[i], &data[j], distance_method)
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "clustered_heatmap() cannot compute a distance between observations {i} and {j}: no finite values overlap"
                        ),
                        None,
                    )
                })?;
            distances[i][j] = d;
            distances[j][i] = d;
        }
    }

    let mut active_slots: Vec<usize> = (0..n).collect();
    let mut node_for_slot: Vec<usize> = (0..n).collect();
    let mut sizes = vec![1usize; capacity];
    let mut last_height = vec![f64::NEG_INFINITY; capacity];
    let mut min_leaf: Vec<usize> = (0..capacity).collect();
    let mut merges = Vec::with_capacity(n - 1);

    for step in 0..(n - 1) {
        let mut best: Option<(usize, usize, f64, (usize, usize))> = None;
        for ai in 0..active_slots.len() {
            for bi in (ai + 1)..active_slots.len() {
                let a_slot = active_slots[ai];
                let b_slot = active_slots[bi];
                let d = distances[a_slot][b_slot];
                if !d.is_finite() {
                    continue;
                }
                let a_node = node_for_slot[a_slot];
                let b_node = node_for_slot[b_slot];
                let pair = (a_node.min(b_node), a_node.max(b_node));
                let replace = match best {
                    None => true,
                    Some((_, _, old_d, old_pair)) => d < old_d || (d == old_d && pair < old_pair),
                };
                if replace {
                    best = Some((a_slot, b_slot, d, pair));
                }
            }
        }
        let (a_slot, b_slot, height, _) = best.ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "clustered_heatmap() hierarchy contains no finite cluster distance",
                None,
            )
        })?;
        let a = node_for_slot[a_slot];
        let b = node_for_slot[b_slot];
        let merged = n + step;

        // Match hclust's visible branch rotation: tighter subtree left;
        // singleton tightest; input order resolves a tie.
        let (left, right) = if last_height[a] < last_height[b]
            || (last_height[a] == last_height[b] && min_leaf[a] <= min_leaf[b])
        {
            (a, b)
        } else {
            (b, a)
        };
        merges.push(HeatmapMerge {
            left,
            right,
            height,
        });
        sizes[merged] = sizes[a] + sizes[b];
        last_height[merged] = height;
        min_leaf[merged] = min_leaf[a].min(min_leaf[b]);

        for &other_slot in &active_slots {
            if other_slot == a_slot || other_slot == b_slot {
                continue;
            }
            let other = node_for_slot[other_slot];
            let da = distances[a_slot][other_slot];
            let db = distances[b_slot][other_slot];
            let updated = match linkage {
                HeatmapLinkage::Complete => da.max(db),
                HeatmapLinkage::Single => da.min(db),
                HeatmapLinkage::Average => {
                    (sizes[a] as f64 * da + sizes[b] as f64 * db) / (sizes[a] + sizes[b]) as f64
                }
                HeatmapLinkage::WardD2 => {
                    let sa = sizes[a] as f64;
                    let sb = sizes[b] as f64;
                    let so = sizes[other] as f64;
                    (((so + sa) * da * da + (so + sb) * db * db - so * height * height)
                        / (sa + sb + so))
                        .max(0.0)
                        .sqrt()
                }
            };
            distances[a_slot][other_slot] = updated;
            distances[other_slot][a_slot] = updated;
        }
        node_for_slot[a_slot] = merged;
        active_slots.retain(|&slot| slot != b_slot);
    }

    fn append_leaves(
        node: usize,
        leaf_count: usize,
        merges: &[HeatmapMerge],
        out: &mut Vec<usize>,
    ) {
        if node < leaf_count {
            out.push(node);
        } else {
            let merge = &merges[node - leaf_count];
            append_leaves(merge.left, leaf_count, merges, out);
            append_leaves(merge.right, leaf_count, merges, out);
        }
    }
    let mut order = Vec::with_capacity(n);
    append_leaves(node_for_slot[active_slots[0]], n, &merges, &mut order);
    Ok(HeatmapTree { merges, order })
}

fn draw_row_dendrogram(
    canvas: &mut SvgCanvas,
    tree: &HeatmapTree,
    heatmap_left: f64,
    heatmap_top: f64,
    cell_height: f64,
    dendrogram_width: f64,
) {
    let n = tree.order.len();
    if n < 2 || dendrogram_width <= 0.0 {
        return;
    }
    let mut x = vec![heatmap_left; 2 * n - 1];
    let mut y = vec![0.0; 2 * n - 1];
    for (position, &leaf) in tree.order.iter().enumerate() {
        y[leaf] = heatmap_top + (position as f64 + 0.5) * cell_height;
    }
    let max_height = tree
        .merges
        .iter()
        .map(|merge| merge.height)
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    for (index, merge) in tree.merges.iter().enumerate() {
        let node = n + index;
        x[node] = heatmap_left - dendrogram_width * merge.height / max_height;
        y[node] = 0.5 * (y[merge.left] + y[merge.right]);
        canvas.add_line(
            x[node],
            y[merge.left],
            x[node],
            y[merge.right],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[node],
            y[merge.left],
            x[merge.left],
            y[merge.left],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[node],
            y[merge.right],
            x[merge.right],
            y[merge.right],
            canvas.theme.axis_colour,
            1.0,
        );
    }
}

fn draw_column_dendrogram(
    canvas: &mut SvgCanvas,
    tree: &HeatmapTree,
    heatmap_left: f64,
    heatmap_top: f64,
    cell_width: f64,
    dendrogram_height: f64,
) {
    let n = tree.order.len();
    if n < 2 || dendrogram_height <= 0.0 {
        return;
    }
    let mut x = vec![0.0; 2 * n - 1];
    let mut y = vec![heatmap_top; 2 * n - 1];
    for (position, &leaf) in tree.order.iter().enumerate() {
        x[leaf] = heatmap_left + (position as f64 + 0.5) * cell_width;
    }
    let max_height = tree
        .merges
        .iter()
        .map(|merge| merge.height)
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    for (index, merge) in tree.merges.iter().enumerate() {
        let node = n + index;
        x[node] = 0.5 * (x[merge.left] + x[merge.right]);
        y[node] = heatmap_top - dendrogram_height * merge.height / max_height;
        canvas.add_line(
            x[merge.left],
            y[node],
            x[merge.right],
            y[node],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[merge.left],
            y[node],
            x[merge.left],
            y[merge.left],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[merge.right],
            y[node],
            x[merge.right],
            y[merge.right],
            canvas.theme.axis_colour,
            1.0,
        );
    }
}

fn hidden_heatmap_order(
    opts: &HashMap<String, Value>,
    key: &str,
    size: usize,
) -> Result<Option<Vec<usize>>> {
    let Some(value) = opts.get(key) else {
        return Ok(None);
    };
    let Value::List(items) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal option '{key}' must be a List"),
            None,
        ));
    };
    let mut order = Vec::with_capacity(items.len());
    for item in items.iter() {
        let index = item.as_float().map(|value| value as usize).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal option '{key}' must contain indices"),
                None,
            )
        })?;
        order.push(index);
    }
    let mut sorted = order.clone();
    sorted.sort_unstable();
    if order.len() != size || sorted != (0..size).collect::<Vec<_>>() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal option '{key}' is not a permutation"),
            None,
        ));
    }
    Ok(Some(order))
}

fn hidden_heatmap_tree(
    opts: &HashMap<String, Value>,
    key: &str,
    order: &[usize],
) -> Result<Option<HeatmapTree>> {
    let Some(value) = opts.get(key) else {
        return Ok(None);
    };
    let Value::Table(table) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal option '{key}' must be a Table"),
            None,
        ));
    };
    for required in ["left", "right", "height"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal tree '{key}' is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() != order.len().saturating_sub(1) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal tree '{key}' has the wrong merge count"),
            None,
        ));
    }
    let left = table.col_index("left").unwrap();
    let right = table.col_index("right").unwrap();
    let height = table.col_index("height").unwrap();
    let mut merges = Vec::with_capacity(table.num_rows());
    for (step, row) in table.rows.iter().enumerate() {
        let left_node = row[left]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "clustered_heatmap() internal tree '{key}' contains a non-numeric node"
                    ),
                    None,
                )
            })?;
        let right_node = row[right]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "clustered_heatmap() internal tree '{key}' contains a non-numeric node"
                    ),
                    None,
                )
            })?;
        let merge_height = row[height].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal tree '{key}' contains a non-numeric height"),
                None,
            )
        })?;
        let available_nodes = order.len() + step;
        if left_node >= available_nodes
            || right_node >= available_nodes
            || !merge_height.is_finite()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal tree '{key}' contains an invalid merge"),
                None,
            ));
        }
        merges.push(HeatmapMerge {
            left: left_node,
            right: right_node,
            height: merge_height,
        });
    }
    Ok(Some(HeatmapTree {
        merges,
        order: order.to_vec(),
    }))
}

fn heatmap_tree_table(tree: Option<&HeatmapTree>) -> Value {
    Value::Table(Table::new(
        ["left", "right", "height"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        tree.map(|tree| {
            tree.merges
                .iter()
                .map(|merge| {
                    vec![
                        Value::Int(merge.left as i64),
                        Value::Int(merge.right as i64),
                        Value::Float(merge.height),
                    ]
                })
                .collect()
        })
        .unwrap_or_default(),
    ))
}

#[allow(clippy::too_many_arguments)]
fn clustered_heatmap_spec_value(
    data: &[Vec<f64>],
    row_names: &[String],
    col_names: &[String],
    row_order: &[usize],
    col_order: &[usize],
    row_tree: Option<&HeatmapTree>,
    column_tree: Option<&HeatmapTree>,
    value_min: f64,
    value_max: f64,
    scale_min: f64,
    scale_max: f64,
    use_diverging: bool,
    opts: &HashMap<String, Value>,
) -> Value {
    let row_position = row_order
        .iter()
        .enumerate()
        .map(|(display, &source)| (source, display))
        .collect::<HashMap<_, _>>();
    let col_position = col_order
        .iter()
        .enumerate()
        .map(|(display, &source)| (source, display))
        .collect::<HashMap<_, _>>();
    let cells = data
        .iter()
        .enumerate()
        .flat_map(|(source_row, row)| {
            let row_position = &row_position;
            let col_position = &col_position;
            row.iter().enumerate().map(move |(source_col, &value)| {
                vec![
                    Value::Int(source_row as i64),
                    Value::Int(row_position[&source_row] as i64),
                    Value::Int(source_col as i64),
                    Value::Int(col_position[&source_col] as i64),
                    Value::Float(value),
                ]
            })
        })
        .collect();
    let rows = row_names
        .iter()
        .enumerate()
        .map(|(source, label)| {
            vec![
                Value::Int(source as i64),
                Value::Int(row_position[&source] as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let columns = col_names
        .iter()
        .enumerate()
        .map(|(source, label)| {
            vec![
                Value::Int(source as i64),
                Value::Int(col_position[&source] as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let non_finite = data
        .iter()
        .flat_map(|row| row.iter())
        .filter(|value| !value.is_finite())
        .count();
    let options = HashMap::from([
        ("plot".into(), Value::Str("clustered_heatmap".into())),
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", "Clustered Heatmap").into()),
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
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        (
            "order".into(),
            Value::Str(get_opt_str(opts, "order", "nearest").into()),
        ),
        (
            "linkage".into(),
            Value::Str(get_opt_str(opts, "linkage", "complete").into()),
        ),
        (
            "distance".into(),
            Value::Str(get_opt_str(opts, "distance", "euclidean").into()),
        ),
        (
            "dendrogram".into(),
            Value::Str(
                get_opt_str(
                    opts,
                    "dendrogram",
                    if row_tree.is_some() { "both" } else { "none" },
                )
                .into(),
            ),
        ),
        (
            "chars".into(),
            Value::Str(get_opt_str(opts, "chars", " ░▒▓█").into()),
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
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("heatmap".into())),
            ("plot".into(), Value::Str("clustered_heatmap".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "Clustered Heatmap").into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "display_row",
                        "source_col",
                        "display_col",
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
                    ["source_row", "display_row", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    rows,
                )),
            ),
            (
                "columns".into(),
                Value::Table(Table::new(
                    ["source_col", "display_col", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    columns,
                )),
            ),
            ("row_merges".into(), heatmap_tree_table(row_tree)),
            ("column_merges".into(), heatmap_tree_table(column_tree)),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("clustered_heatmap".into())),
                        ("input_rows".into(), Value::Int(data.len() as i64)),
                        (
                            "input_columns".into(),
                            Value::Int(data.first().map(Vec::len).unwrap_or(0) as i64),
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
                            "{non_finite} clustered-heatmap cells are non-finite"
                        ))]
                    }
                    .into(),
                ),
            ),
        ])
        .into(),
    )
}

pub(crate) fn is_clustered_heatmap_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "heatmap")
                && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "clustered_heatmap")
    )
}

pub(crate) fn render_clustered_heatmap_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_clustered_heatmap_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 clustered-heatmap Record",
                None,
            ))
        }
    };
    let table_field = |name: &str| -> Result<&Table> {
        match map.get(name) {
            Some(Value::Table(table)) => Ok(table),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "render_plot() clustered-heatmap specification field '{name}' must be Table"
                ),
                None,
            )),
        }
    };
    let cells = table_field("data")?;
    let rows = table_field("rows")?;
    let columns = table_field("columns")?;
    let row_merges = table_field("row_merges")?;
    let column_merges = table_field("column_merges")?;
    for required in [
        "source_row",
        "display_row",
        "source_col",
        "display_col",
        "value",
    ] {
        if cells.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() clustered-heatmap data is missing '{required}'"),
                None,
            ));
        }
    }
    if rows.num_rows() == 0 || columns.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() clustered-heatmap specification is empty",
            None,
        ));
    }
    for (table, source, display) in [
        (rows, "source_row", "display_row"),
        (columns, "source_col", "display_col"),
    ] {
        for required in [source, display, "label"] {
            if table.col_index(required).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() clustered-heatmap metadata is missing '{required}'"),
                    None,
                ));
            }
        }
    }
    let ordered_metadata =
        |table: &Table, source: &str, display: &str| -> Result<(Vec<String>, Vec<usize>)> {
            let source_index = table.col_index(source).unwrap();
            let display_index = table.col_index(display).unwrap();
            let label_index = table.col_index("label").unwrap();
            let mut labels = vec![String::new(); table.num_rows()];
            let mut order = vec![usize::MAX; table.num_rows()];
            for row in &table.rows {
                let source_value = row[source_index]
                    .as_float()
                    .map(|value| value as usize)
                    .ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "render_plot() clustered-heatmap source indices must be numeric",
                            None,
                        )
                    })?;
                let display_value = row[display_index]
                    .as_float()
                    .map(|value| value as usize)
                    .ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "render_plot() clustered-heatmap display indices must be numeric",
                            None,
                        )
                    })?;
                if source_value >= labels.len()
                    || display_value >= order.len()
                    || order[display_value] != usize::MAX
                {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "render_plot() clustered-heatmap metadata indices are invalid",
                        None,
                    ));
                }
                labels[source_value] = format!("{}", row[label_index]);
                order[display_value] = source_value;
            }
            if order.contains(&usize::MAX) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() clustered-heatmap metadata is incomplete",
                    None,
                ));
            }
            Ok((labels, order))
        };
    let (row_labels, row_order) = ordered_metadata(rows, "source_row", "display_row")?;
    let (col_labels, col_order) = ordered_metadata(columns, "source_col", "display_col")?;
    if cells.num_rows() != row_labels.len() * col_labels.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() clustered-heatmap data must contain one cell per row and column",
            None,
        ));
    }
    let sr = cells.col_index("source_row").unwrap();
    let sc = cells.col_index("source_col").unwrap();
    let vi = cells.col_index("value").unwrap();
    let mut matrix = vec![vec![f64::NAN; col_labels.len()]; row_labels.len()];
    for (expected, row) in cells.rows.iter().enumerate() {
        let source_row = row[sr]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() clustered-heatmap source_row must be numeric",
                    None,
                )
            })?;
        let source_col = row[sc]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() clustered-heatmap source_col must be numeric",
                    None,
                )
            })?;
        if source_row >= matrix.len()
            || source_col >= col_labels.len()
            || expected != source_row * col_labels.len() + source_col
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap cells must be ordered by source row and column",
                None,
            ));
        }
        matrix[source_row][source_col] = row[vi].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap values must be numeric",
                None,
            )
        })?;
    }
    let mut table_columns = Vec::with_capacity(col_labels.len() + 1);
    table_columns.push("gene".to_string());
    table_columns.extend(col_labels.iter().cloned());
    let table_rows = matrix
        .iter()
        .enumerate()
        .map(|(index, values)| {
            let mut row = Vec::with_capacity(values.len() + 1);
            row.push(Value::Str(row_labels[index].clone()));
            row.extend(values.iter().map(|value| Value::Float(*value)));
            row
        })
        .collect();
    let input = Value::Table(Table::new(table_columns, table_rows));
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap specification field 'options' must be Record",
                None,
            ))
        }
    };
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in ["width", "height"] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    options.insert("format".into(), Value::Str("svg".into()));
    options.insert(
        "__row_order".into(),
        Value::List(
            row_order
                .iter()
                .map(|index| Value::Int(*index as i64))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    options.insert(
        "__col_order".into(),
        Value::List(
            col_order
                .iter()
                .map(|index| Value::Int(*index as i64))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    options.insert("__row_tree".into(), Value::Table(row_merges.clone()));
    options.insert("__column_tree".into(), Value::Table(column_merges.clone()));
    for key in ["scale_min", "scale_max", "diverging"] {
        let value = options.get(key).cloned().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() clustered-heatmap options are missing '{key}'"),
                None,
            )
        })?;
        options.insert(format!("__{key}"), value);
    }
    let svg = match builtin_clustered_heatmap(vec![input, Value::Record(options.into())])? {
        Value::Str(svg) => svg,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap renderer did not return SVG",
                None,
            ))
        }
    };
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Clustered Heatmap");
    match format.as_str() {
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
            "render_plot() terminal clustered-heatmap output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown clustered-heatmap format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_clustered_heatmap(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let heat_chars: Vec<char> = get_opt_str(&opts, "chars", " ░▒▓█").chars().collect();
    let title = get_opt_str(&opts, "title", "Clustered Heatmap").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let legend_title = get_opt_str(&opts, "legend_title", "value").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let order_method = get_opt_str(&opts, "order", "nearest").to_ascii_lowercase();
    let hierarchical = match order_method.as_str() {
        "nearest" | "nn" => false,
        "hierarchical" | "hclust" => true,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                "clustered_heatmap() order must be nearest or hierarchical; got '{order_method}'"
            ),
                None,
            ))
        }
    };
    let linkage = HeatmapLinkage::parse(get_opt_str(&opts, "linkage", "complete"))?;
    let distance = HeatmapDistance::parse(get_opt_str(&opts, "distance", "euclidean"))?;
    let dendrogram_mode = get_opt_str(
        &opts,
        "dendrogram",
        if hierarchical { "both" } else { "none" },
    )
    .to_ascii_lowercase();
    let (draw_row_tree, draw_column_tree) = match dendrogram_mode.as_str() {
        "both" => (true, true),
        "row" | "rows" => (true, false),
        "column" | "columns" | "col" | "cols" => (false, true),
        "none" => (false, false),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "clustered_heatmap() dendrogram must be both, row, column, or none; got '{dendrogram_mode}'"
                ),
                None,
            ))
        }
    };
    if !hierarchical && (draw_row_tree || draw_column_tree) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "clustered_heatmap() dendrograms require order: \"hierarchical\"",
            None,
        ));
    }

    let (mut row_names, mut col_names, data) = match &args[0] {
        Value::Table(table) => {
            let mut numeric_names = Vec::new();
            let mut cols_data: Vec<Vec<f64>> = Vec::new();
            for col in &table.columns {
                if let Ok(values) = extract_table_col(table, col) {
                    // extract_table_col represents non-numeric strings as
                    // NaN. A gene annotation column therefore parses without
                    // an error but is not a numeric heatmap dimension.
                    if values.iter().any(|value| value.is_finite()) {
                        numeric_names.push(col.clone());
                        cols_data.push(values);
                    }
                }
            }
            if cols_data.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "clustered_heatmap() table contains no numeric columns",
                    None,
                ));
            }
            let (nrows, ncols) = (table.num_rows(), cols_data.len());
            let mut t = vec![vec![0.0; ncols]; nrows];
            for c in 0..ncols {
                for r in 0..nrows {
                    t[r][c] = cols_data[c][r];
                }
            }
            let label_column = ["gene", "feature", "name", "marker"]
                .iter()
                .find(|name| table.col_index(name).is_some());
            let rn = label_column
                .and_then(|name| extract_str_col(table, name).ok())
                .unwrap_or_else(|| (0..nrows).map(|i| format!("row{i}")).collect());
            (rn, numeric_names, t)
        }
        Value::Matrix(m) => {
            let rn = m
                .row_names
                .clone()
                .unwrap_or_else(|| (0..m.nrow).map(|i| format!("row{i}")).collect());
            let cn = m
                .col_names
                .clone()
                .unwrap_or_else(|| (0..m.ncol).map(|i| format!("col{i}")).collect());
            let mut data = vec![vec![0.0; m.ncol]; m.nrow];
            for r in 0..m.nrow {
                for c in 0..m.ncol {
                    data[r][c] = m.data[r * m.ncol + c];
                }
            }
            (rn, cn, data)
        }
        _ => {
            return Err(BioLangError::type_error(
                "clustered_heatmap() requires Table or Matrix",
                None,
            ))
        }
    };
    let option_labels = |key: &str| -> Option<Vec<String>> {
        match opts.get(key) {
            Some(Value::List(items)) => Some(items.iter().map(|item| format!("{item}")).collect()),
            _ => None,
        }
    };
    if let Some(labels) = option_labels("row_labels") {
        for (index, label) in labels.into_iter().enumerate().take(row_names.len()) {
            row_names[index] = label;
        }
    }
    if let Some(labels) = option_labels("col_labels") {
        for (index, label) in labels.into_iter().enumerate().take(col_names.len()) {
            col_names[index] = label;
        }
    }
    let nrows = data.len();
    let ncols = if nrows > 0 { data[0].len() } else { 0 };
    if nrows == 0 || ncols == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "clustered_heatmap() received empty data",
            None,
        ));
    }
    let col_data: Vec<Vec<f64>> = (0..ncols)
        .map(|c| (0..nrows).map(|r| data[r][c]).collect())
        .collect();
    let frozen_row_order = hidden_heatmap_order(&opts, "__row_order", nrows)?;
    let frozen_col_order = hidden_heatmap_order(&opts, "__col_order", ncols)?;
    let row_tree = if hierarchical {
        if let Some(order) = frozen_row_order.as_deref() {
            match hidden_heatmap_tree(&opts, "__row_tree", order)? {
                Some(tree) => Some(tree),
                None => Some(hierarchical_heatmap_tree(&data, distance, linkage)?),
            }
        } else {
            Some(hierarchical_heatmap_tree(&data, distance, linkage)?)
        }
    } else {
        None
    };
    let column_tree = if hierarchical {
        if let Some(order) = frozen_col_order.as_deref() {
            match hidden_heatmap_tree(&opts, "__column_tree", order)? {
                Some(tree) => Some(tree),
                None => Some(hierarchical_heatmap_tree(&col_data, distance, linkage)?),
            }
        } else {
            Some(hierarchical_heatmap_tree(&col_data, distance, linkage)?)
        }
    } else {
        None
    };
    let row_order = frozen_row_order.unwrap_or_else(|| {
        row_tree
            .as_ref()
            .map(|tree| tree.order.clone())
            .unwrap_or_else(|| nn_order(&data))
    });
    let col_order = frozen_col_order.unwrap_or_else(|| {
        column_tree
            .as_ref()
            .map(|tree| tree.order.clone())
            .unwrap_or_else(|| nn_order(&col_data))
    });
    let all: Vec<f64> = data
        .iter()
        .flat_map(|r| r.iter().copied())
        .filter(|v| v.is_finite())
        .collect();
    let (vmin, vmax) = if all.is_empty() {
        (0.0, 1.0)
    } else {
        col_range(&all)
    };
    let requested_centre = opts.get("center").and_then(Value::as_float);
    let use_diverging = opts
        .get("__diverging")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            publication_theme && (requested_centre.is_some() || (vmin < 0.0 && vmax > 0.0))
        });
    let frozen_scale = opts
        .get("__scale_min")
        .and_then(Value::as_float)
        .zip(opts.get("__scale_max").and_then(Value::as_float));
    let (scale_min, scale_max) = if let Some(domain) = frozen_scale {
        domain
    } else if use_diverging {
        let centre = requested_centre.unwrap_or(0.0);
        let radius = (vmin - centre)
            .abs()
            .max((vmax - centre).abs())
            .max(f64::EPSILON);
        (centre - radius, centre + radius)
    } else {
        (vmin, vmax)
    };

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = clustered_heatmap_spec_value(
            &data,
            &row_names,
            &col_names,
            &row_order,
            &col_order,
            row_tree.as_ref(),
            column_tree.as_ref(),
            vmin,
            vmax,
            scale_min,
            scale_max,
            use_diverging,
            &opts,
        );
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_clustered_heatmap_spec_value(&spec, &opts);
    }
    let colour = |t: f64| {
        if publication_theme {
            if use_diverging {
                publication_diverging_color(t)
            } else {
                publication_sequential_color(t)
            }
        } else {
            sequential_color(t)
        }
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::with_theme(w, h, theme);
        let row_dendrogram_width = if draw_row_tree {
            (w * 0.12).clamp(28.0, 80.0).min(w * 0.18)
        } else {
            0.0
        };
        let column_dendrogram_height = if draw_column_tree {
            (h * 0.12).clamp(28.0, 70.0).min(h * 0.18)
        } else {
            0.0
        };
        if theme.is_adaptive() {
            let widest_row = row_names
                .iter()
                .map(|label| estimate_text_width(label, theme.tick_size))
                .fold(0.0, f64::max);
            let widest_col = col_names
                .iter()
                .map(|label| estimate_text_width(label, theme.tick_size))
                .fold(0.0, f64::max);
            let legend_label = [scale_min, 0.5 * (scale_min + scale_max), scale_max]
                .iter()
                .map(|value| estimate_text_width(&format!("{value:.2}"), theme.legend_size))
                .fold(0.0, f64::max);
            let label_margin = (widest_row + 12.0).clamp(52.0, w * 0.27);
            c.margin.left =
                (label_margin + row_dendrogram_width + if draw_row_tree { 8.0 } else { 0.0 })
                    .min(w * 0.43);
            c.margin.right = (42.0
                + legend_label.max(estimate_text_width(&legend_title, theme.legend_size)))
            .clamp(78.0, w * 0.31);
            c.margin.top = if title.is_empty() {
                20.0
            } else if subtitle.is_empty() {
                48.0
            } else {
                66.0
            } + column_dendrogram_height
                + if draw_column_tree { 7.0 } else { 0.0 };
            c.margin.bottom = (widest_col * 0.72 + 18.0).clamp(48.0, h * 0.28)
                + if caption.is_empty() { 0.0 } else { 18.0 };
        } else {
            c.margin.left = 80.0 + row_dendrogram_width + if draw_row_tree { 8.0 } else { 0.0 };
            c.margin.top += column_dendrogram_height + if draw_column_tree { 7.0 } else { 0.0 };
            c.margin.bottom = 60.0;
        }
        let cw = c.plot_width() / ncols as f64;
        let ch = c.plot_height() / nrows as f64;
        for (ri, &row_i) in row_order.iter().enumerate() {
            for (ci, &col_i) in col_order.iter().enumerate() {
                let v = data[row_i][col_i];
                let t = if (scale_max - scale_min).abs() < f64::EPSILON {
                    0.5
                } else {
                    (v - scale_min) / (scale_max - scale_min)
                };
                let x = c.margin.left + ci as f64 * cw;
                let y = c.margin.top + ri as f64 * ch;
                c.add_rect(x, y, cw, ch, &colour(t));
                if theme.is_adaptive() && cw.min(ch) >= 4.0 {
                    c.elements.push(format!(
                        r#"<rect x="{x:.1}" y="{y:.1}" width="{cw:.1}" height="{ch:.1}" fill="none" stroke="{}" stroke-width="0.5" />"#,
                        theme.grid_colour
                    ));
                }
            }
        }
        if hierarchical {
            let row_heights = row_tree
                .as_ref()
                .map(|tree| {
                    tree.merges
                        .iter()
                        .map(|merge| format!("{:.12}", merge.height))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let column_heights = column_tree
                .as_ref()
                .map(|tree| {
                    tree.merges
                        .iter()
                        .map(|merge| format!("{:.12}", merge.height))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let row_order_metadata = row_order
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let column_order_metadata = col_order
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",");
            c.elements.push(format!(
                r#"<g data-biolang-clustering="hierarchical" data-distance="{}" data-linkage="{}" data-dendrogram="{}" data-row-order="{}" data-row-heights="{}" data-column-order="{}" data-column-heights="{}">"#,
                distance.name(),
                linkage.name(),
                dendrogram_mode,
                row_order_metadata,
                row_heights,
                column_order_metadata,
                column_heights
            ));
            let heatmap_left = c.margin.left;
            let heatmap_top = c.margin.top;
            if draw_row_tree {
                if let Some(tree) = &row_tree {
                    draw_row_dendrogram(
                        &mut c,
                        tree,
                        heatmap_left,
                        heatmap_top,
                        ch,
                        row_dendrogram_width,
                    );
                }
            }
            if draw_column_tree {
                if let Some(tree) = &column_tree {
                    draw_column_dendrogram(
                        &mut c,
                        tree,
                        heatmap_left,
                        heatmap_top,
                        cw,
                        column_dendrogram_height,
                    );
                }
            }
            c.elements.push("</g>".to_string());
        }
        let row_step = if theme.is_adaptive() {
            (10.0 / ch.max(1.0)).ceil().max(1.0) as usize
        } else {
            1
        };
        for (ri, &row_i) in row_order.iter().enumerate().step_by(row_step) {
            c.add_text(
                c.margin.left - row_dendrogram_width - if draw_row_tree { 7.0 } else { 3.0 },
                c.margin.top + (ri as f64 + 0.5) * ch + 4.0,
                &row_names[row_i],
                "end",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    9.0
                },
            );
        }
        if theme.is_adaptive() {
            let col_step = (10.0 / cw.max(1.0)).ceil().max(1.0) as usize;
            let label_y = c.margin.top + c.plot_height() + 10.0;
            for (ci, &col_i) in col_order.iter().enumerate().step_by(col_step) {
                c.add_text_rotated(
                    c.margin.left + (ci as f64 + 0.5) * cw,
                    label_y,
                    &col_names[col_i],
                    45.0,
                    "start",
                    theme.tick_size,
                );
            }

            let legend_x = c.margin.left + c.plot_width() + 14.0;
            let legend_top = c.margin.top;
            let legend_height = c.plot_height().min(180.0);
            c.add_text(
                legend_x,
                legend_top - 8.0,
                &legend_title,
                "start",
                theme.legend_size,
            );
            for step in 0..40 {
                let t = 1.0 - step as f64 / 39.0;
                c.add_rect(
                    legend_x,
                    legend_top + step as f64 * legend_height / 40.0,
                    12.0,
                    legend_height / 40.0 + 0.5,
                    &colour(t),
                );
            }
            for (value, y) in [
                (scale_max, legend_top + 4.0),
                (
                    (scale_min + scale_max) / 2.0,
                    legend_top + legend_height / 2.0 + 3.0,
                ),
                (scale_min, legend_top + legend_height + 3.0),
            ] {
                c.add_text(
                    legend_x + 17.0,
                    y,
                    &format!("{value:.2}"),
                    "start",
                    theme.legend_size,
                );
            }
        }
        c.set_accessible_description(if hierarchical {
            format!(
                "Heatmap with {nrows} rows and {ncols} columns, hierarchically ordered using {} distance and {} linkage; dendrogram display: {}.",
                distance.name(),
                linkage.name(),
                dendrogram_mode
            )
        } else {
            format!(
                "Heatmap with {nrows} rows and {ncols} columns, ordered by deterministic nearest-neighbour traversal from the first row and first column."
            )
        });
        c.draw_title(&title);
        if theme.is_adaptive() {
            c.draw_subtitle(&subtitle);
            c.draw_caption(&caption);
        }
        return Ok(Value::Str(c.render()));
    }

    let max_rl = row_names.iter().map(|s| s.len()).max().unwrap_or(0);
    let nlevels = heat_chars.len();
    let mut out = format!("  {title}\n");
    if hierarchical {
        out.push_str(&format!(
            "  order: hierarchical; distance: {}; linkage: {}\n",
            distance.name(),
            linkage.name()
        ));
    }
    out.push_str(&format!("  {:>w$}  ", "", w = max_rl));
    for &ci in &col_order {
        out.push_str(&format!(
            "{} ",
            &col_names[ci].chars().take(2).collect::<String>()
        ));
    }
    out.push('\n');
    for &ri in &row_order {
        out.push_str(&format!("  {:>w$}  ", row_names[ri], w = max_rl));
        for &ci in &col_order {
            let t = if (vmax - vmin).abs() < f64::EPSILON {
                0.5
            } else {
                (data[ri][ci] - vmin) / (vmax - vmin)
            };
            out.push(
                heat_chars[(t * (nlevels - 1) as f64)
                    .round()
                    .clamp(0.0, (nlevels - 1) as f64) as usize],
            );
            out.push_str("  ");
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

fn nn_order(data: &[Vec<f64>]) -> Vec<usize> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    let dist = |a: &[f64], b: &[f64]| -> f64 {
        let mut squared = 0.0;
        let mut compared = 0usize;
        for (&x, &y) in a.iter().zip(b.iter()) {
            if x.is_finite() && y.is_finite() {
                squared += (x - y).powi(2);
                compared += 1;
            }
        }
        if compared == 0 {
            f64::INFINITY
        } else {
            squared.sqrt()
        }
    };
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut cur = 0;
    visited[0] = true;
    order.push(0);
    for _ in 1..n {
        let mut best = None;
        let mut best_d = f64::INFINITY;
        for j in 0..n {
            if !visited[j] {
                let d = dist(&data[cur], &data[j]);
                if best.is_none() || d < best_d {
                    best_d = d;
                    best = Some(j);
                }
            }
        }
        let best = best.expect("an unvisited heatmap item must remain");
        visited[best] = true;
        order.push(best);
        cur = best;
    }
    order
}

// ── 12. pca_plot ────────────────────────────────────────────────

fn render_pca_scores_svg(
    pc1: &[f64],
    pc2: &[f64],
    labels: Option<&[String]>,
    row_names: Option<&[String]>,
    pct1: f64,
    pct2: f64,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let xr = col_range(pc1);
    let yr = col_range(pc2);
    let w = get_opt_f64(opts, "width", 600.0);
    let h = get_opt_f64(opts, "height", 400.0);
    let title = get_opt_str(opts, "title", "PCA Plot").to_string();
    let mut canvas = themed_canvas(w, h, opts);
    let x_scale = Scale {
        domain: xr,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: yr,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let finite_rows = (0..pc1.len().min(pc2.len()))
        .filter(|&index| pc1[index].is_finite() && pc2[index].is_finite())
        .collect::<Vec<_>>();
    let mut colour_map: HashMap<String, usize> = HashMap::new();
    let mut next_colour = 0;
    let mut points: Vec<(f64, f64, &str)> = Vec::with_capacity(finite_rows.len());
    for &index in &finite_rows {
        let colour_index = labels
            .map(|values| {
                let entry = colour_map.entry(values[index].clone()).or_insert_with(|| {
                    let value = next_colour;
                    next_colour += 1;
                    value
                });
                *entry
            })
            .unwrap_or(0);
        points.push((
            x_scale.map(pc1[index]),
            y_scale.map(pc2[index]),
            PALETTE[colour_index % PALETTE.len()],
        ));
    }
    let raster = raster_choice(opts, "pca_plot", finite_rows.len())?;
    let area = canvas.point_area();
    canvas.add_scatter(&points, 4.0, area, raster);
    if let Some(names) = row_names {
        for &index in &finite_rows {
            canvas.add_text(
                x_scale.map(pc1[index]) + 6.0,
                y_scale.map(pc2[index]) - 4.0,
                &names[index],
                "start",
                8.0,
            );
        }
    }
    if let Some(values) = labels {
        let mut seen: Vec<String> = Vec::new();
        for &index in &finite_rows {
            if !seen.contains(&values[index]) {
                seen.push(values[index].clone());
            }
        }
        for (index, name) in seen.iter().enumerate() {
            let lx = canvas.margin.left + canvas.plot_width() - 80.0;
            let ly = canvas.margin.top + 15.0 + index as f64 * 16.0;
            canvas.add_circle(lx, ly, 4.0, PALETTE[index % PALETTE.len()]);
            canvas.add_text(lx + 8.0, ly + 4.0, name, "start", 10.0);
        }
    }
    canvas.draw_x_axis(
        &Scale {
            domain: xr,
            range: xr,
        },
        &format!("PC1 ({pct1:.1}%)"),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: yr,
            range: yr,
        },
        &format!("PC2 ({pct2:.1}%)"),
    );
    canvas.draw_title(&title);
    canvas.draw_subtitle(get_opt_str(opts, "subtitle", ""));
    canvas.draw_caption(get_opt_str(opts, "caption", ""));
    canvas.set_accessible_description(format!(
        "PCA score plot with {} rendered of {} observations; PC1 explains {pct1:.1}% and PC2 explains {pct2:.1}% of total variance.",
        finite_rows.len(),
        pc1.len().min(pc2.len())
    ));
    Ok(canvas.render())
}

fn pca_plot_spec_value(
    pc1: &[f64],
    pc2: &[f64],
    labels: Option<&[String]>,
    row_names: Option<&[String]>,
    pct1: f64,
    pct2: f64,
    input_features: usize,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let point_count = pc1.len().min(pc2.len());
    let rendered_points = (0..point_count)
        .filter(|&index| pc1[index].is_finite() && pc2[index].is_finite())
        .count();
    let raster = raster_choice(opts, "pca_plot", rendered_points)?;
    let rows = (0..point_count)
        .map(|index| {
            vec![
                Value::Int(index as i64),
                Value::Float(pc1[index]),
                Value::Float(pc2[index]),
                labels
                    .and_then(|values| values.get(index))
                    .map(|value| Value::Str(value.clone()))
                    .unwrap_or(Value::Nil),
                row_names
                    .and_then(|values| values.get(index))
                    .map(|value| Value::Str(value.clone()))
                    .unwrap_or(Value::Nil),
            ]
        })
        .collect::<Vec<_>>();
    let data = Value::Table(Table::new(
        ["source_row", "pc1", "pc2", "group", "label"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        rows,
    ));
    let non_finite_coordinates = point_count - rendered_points;
    let warnings = if non_finite_coordinates == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{non_finite_coordinates} observations have non-finite PCA scores"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("pca".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "PCA Plot").into()),
            ),
            ("data".into(), data),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 600.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 400.0)),
                        ),
                        (
                            "title".into(),
                            Value::Str(get_opt_str(opts, "title", "PCA Plot").into()),
                        ),
                        ("pc1_variance_percent".into(), Value::Float(pct1)),
                        ("pc2_variance_percent".into(), Value::Float(pct2)),
                        ("has_groups".into(), Value::Bool(labels.is_some())),
                        ("has_labels".into(), Value::Bool(row_names.is_some())),
                        ("raster".into(), Value::Bool(raster.enabled)),
                        ("raster_scale".into(), Value::Float(raster.scale)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("pca_plot".into())),
                        ("input_rows".into(), Value::Int(point_count as i64)),
                        ("input_features".into(), Value::Int(input_features as i64)),
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

pub(crate) fn is_pca_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "pca")
    )
}

pub(crate) fn render_pca_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_pca_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 PCA Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in ["source_row", "pc1", "pc2", "group", "label"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() PCA data is missing '{required}'"),
                None,
            ));
        }
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA specification field 'options' must be Record",
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
    let pc1 = extract_table_col(table, "pc1")?;
    let pc2 = extract_table_col(table, "pc2")?;
    let groups = if options.get("has_groups").is_some_and(Value::is_truthy) {
        Some(extract_str_col(table, "group")?)
    } else {
        None
    };
    let labels = if options.get("has_labels").is_some_and(Value::is_truthy) {
        Some(extract_str_col(table, "label")?)
    } else {
        None
    };
    let pct1 = options
        .get("pc1_variance_percent")
        .and_then(Value::as_float)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA options are missing numeric 'pc1_variance_percent'",
                None,
            )
        })?;
    let pct2 = options
        .get("pc2_variance_percent")
        .and_then(Value::as_float)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA options are missing numeric 'pc2_variance_percent'",
                None,
            )
        })?;
    let svg = render_pca_scores_svg(
        &pc1,
        &pc2,
        groups.as_deref(),
        labels.as_deref(),
        pct1,
        pct2,
        &options,
    )?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("PCA Plot");
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(crate::plot::standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => {
            crate::plot::render_svg_terminal(&svg, 80, 24, crate::plot::TerminalPlotStyle::Ascii)
                .map(Value::Str)
                .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None))
        }
        #[cfg(feature = "native")]
        "unicode" | "braille" => {
            crate::plot::render_svg_terminal(&svg, 80, 24, crate::plot::TerminalPlotStyle::Braille)
                .map(Value::Str)
                .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None))
        }
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal PCA output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown PCA format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_pca_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    // umap_plot calls this `color_col`, this one calls it `group_col`, and
    // everyone types `color`. Accept all three rather than silently colouring
    // nothing when the caller guesses a sibling function's spelling.
    let group_col = ["group_col", "color_col", "color"]
        .iter()
        .map(|key| get_opt_str(&opts, key, ""))
        .find(|value| !value.is_empty())
        .unwrap_or("")
        .to_string();
    let show_labels = opts
        .get("labels")
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);

    // Extract numeric matrix and optional group labels
    let (data, nrow, ncol, labels, row_names) = match &args[0] {
        Value::Table(table) => {
            if table.num_rows() < 2 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "pca_plot() needs >= 2 rows",
                    None,
                ));
            }
            // Find numeric columns (exclude group_col)
            let mut num_cols: Vec<String> = Vec::new();
            for col in &table.columns {
                if col == &group_col {
                    continue;
                }
                let Some(index) = table.col_index(col) else {
                    continue;
                };
                let numeric = table.rows.iter().all(|row| match &row[index] {
                    Value::Int(_) | Value::Float(_) => true,
                    Value::Str(value) => value.parse::<f64>().is_ok(),
                    _ => false,
                });
                if numeric {
                    num_cols.push(col.clone());
                }
            }
            if num_cols.len() < 2 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "pca_plot() needs >= 2 numeric columns",
                    None,
                ));
            }
            let nrow = table.num_rows();
            let ncol = num_cols.len();
            let mut data = vec![0.0; nrow * ncol];
            for (ci, col) in num_cols.iter().enumerate() {
                let vals = extract_table_col(table, col)?;
                if vals.iter().any(|value| !value.is_finite()) {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("pca_plot() numeric column '{col}' contains non-finite values"),
                        None,
                    ));
                }
                for (ri, &v) in vals.iter().enumerate() {
                    data[ri * ncol + ci] = v;
                }
            }
            let lbls = if !group_col.is_empty() && table.col_index(&group_col).is_some() {
                extract_str_col(table, &group_col).ok()
            } else {
                None
            };
            // Use first column as row names if it's a string column
            let rn: Option<Vec<String>> = if show_labels {
                table.columns.iter().find_map(|c| {
                    if c == &group_col {
                        return None;
                    }
                    let idx = table.col_index(c)?;
                    if matches!(&table.rows[0][idx], Value::Str(_)) {
                        Some(
                            table
                                .rows
                                .iter()
                                .map(|r| match &r[idx] {
                                    Value::Str(s) => s.clone(),
                                    other => format!("{other}"),
                                })
                                .collect(),
                        )
                    } else {
                        None
                    }
                })
            } else {
                None
            };
            (data, nrow, ncol, lbls, rn)
        }
        Value::Matrix(m) => {
            if m.ncol < 2 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "pca_plot() needs >= 2 columns",
                    None,
                ));
            }
            (m.data.clone(), m.nrow, m.ncol, None, m.row_names.clone())
        }
        _ => {
            return Err(BioLangError::type_error(
                "pca_plot() requires Table or Matrix",
                None,
            ))
        }
    };
    if data.iter().any(|value| !value.is_finite()) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "pca_plot() requires finite numeric values",
            None,
        ));
    }
    if nrow < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "pca_plot() needs >= 2 rows",
            None,
        ));
    }

    // --- PCA via power iteration ---
    // 1. Center columns
    let mut centered = data.clone();
    for ci in 0..ncol {
        let mean = (0..nrow).map(|r| centered[r * ncol + ci]).sum::<f64>() / nrow as f64;
        for r in 0..nrow {
            centered[r * ncol + ci] -= mean;
        }
    }
    // 2. Compute covariance matrix (ncol x ncol)
    let mut cov = vec![0.0; ncol * ncol];
    for i in 0..ncol {
        for j in i..ncol {
            let val: f64 = (0..nrow)
                .map(|r| centered[r * ncol + i] * centered[r * ncol + j])
                .sum::<f64>()
                / (nrow - 1) as f64;
            cov[i * ncol + j] = val;
            cov[j * ncol + i] = val;
        }
    }
    // 3. Power iteration for top eigenvector
    let power_iter = |cov: &[f64], ncol: usize, deflate_vec: Option<&[f64]>| -> Vec<f64> {
        let mut v: Vec<f64> = (0..ncol)
            .map(|i| if i == 0 { 1.0 } else { 0.5 / (i as f64 + 1.0) })
            .collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        for x in &mut v {
            *x /= norm;
        }
        let mut work_cov = cov.to_vec();
        if let Some(dv) = deflate_vec {
            // Deflate: C' = C - lambda * v * v^T (approximate lambda via Rayleigh quotient)
            let mut av = vec![0.0; ncol];
            for i in 0..ncol {
                av[i] = (0..ncol).map(|j| cov[i * ncol + j] * dv[j]).sum::<f64>();
            }
            let lambda: f64 = (0..ncol).map(|i| dv[i] * av[i]).sum();
            for i in 0..ncol {
                for j in 0..ncol {
                    work_cov[i * ncol + j] -= lambda * dv[i] * dv[j];
                }
            }
        }
        for _ in 0..200 {
            let mut new_v = vec![0.0; ncol];
            for i in 0..ncol {
                new_v[i] = (0..ncol)
                    .map(|j| work_cov[i * ncol + j] * v[j])
                    .sum::<f64>();
            }
            let n = new_v.iter().map(|x| x * x).sum::<f64>().sqrt();
            if n < 1e-15 {
                break;
            }
            for x in &mut new_v {
                *x /= n;
            }
            v = new_v;
        }
        v
    };
    let ev1 = power_iter(&cov, ncol, None);
    let ev2 = power_iter(&cov, ncol, Some(&ev1));
    // 4. Project data onto PC1 and PC2
    let pc1: Vec<f64> = (0..nrow)
        .map(|r| (0..ncol).map(|c| centered[r * ncol + c] * ev1[c]).sum())
        .collect();
    let pc2: Vec<f64> = (0..nrow)
        .map(|r| (0..ncol).map(|c| centered[r * ncol + c] * ev2[c]).sum())
        .collect();

    // Compute variance explained
    let total_var: f64 = (0..ncol).map(|i| cov[i * ncol + i]).sum();
    let var1: f64 = (0..ncol)
        .map(|i| {
            let a: f64 = (0..ncol).map(|j| cov[i * ncol + j] * ev1[j]).sum();
            a * ev1[i]
        })
        .sum();
    let var2: f64 = (0..ncol)
        .map(|i| {
            let a: f64 = (0..ncol).map(|j| cov[i * ncol + j] * ev2[j]).sum();
            a * ev2[i]
        })
        .sum();
    let pct1 = if total_var > 0.0 {
        var1 / total_var * 100.0
    } else {
        0.0
    };
    let pct2 = if total_var > 0.0 {
        var2 / total_var * 100.0
    } else {
        0.0
    };

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = pca_plot_spec_value(
            &pc1,
            &pc2,
            labels.as_deref(),
            if show_labels {
                row_names.as_deref()
            } else {
                None
            },
            pct1,
            pct2,
            ncol,
            &opts,
        )?;
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_pca_plot_spec_value(&spec, &opts);
    }

    if fmt == "svg" {
        return render_pca_scores_svg(
            &pc1,
            &pc2,
            labels.as_deref(),
            if show_labels {
                row_names.as_deref()
            } else {
                None
            },
            pct1,
            pct2,
            &opts,
        )
        .map(Value::Str);
    }

    let xr = col_range(&pc1);
    let yr = col_range(&pc2);

    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for j in 0..pc1.len() {
        chart.put(pc1[j], pc2[j], xr, yr, '●');
    }
    write_output(&chart.render(&format!("PCA Plot (PC1: {pct1:.1}%, PC2: {pct2:.1}%)")));
    Ok(Value::Nil)
}

// ── 13. oncoprint ───────────────────────────────────────────────

fn builtin_oncoprint(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "oncoprint")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let samples = extract_str_col(table, get_opt_str(&opts, "sample", "sample"))?;
    let genes = extract_str_col(table, get_opt_str(&opts, "gene", "gene"))?;
    let mut_types =
        extract_str_col(table, "type").unwrap_or_else(|_| vec!["mutation".into(); samples.len()]);

    let sample_order: Vec<String> = {
        let mut s = Vec::new();
        for x in &samples {
            if !s.contains(x) {
                s.push(x.clone());
            }
        }
        s
    };
    let gene_order: Vec<String> = {
        let mut g = Vec::new();
        for x in &genes {
            if !g.contains(x) {
                g.push(x.clone());
            }
        }
        g
    };
    let mut grid: HashMap<(usize, usize), String> = HashMap::new();
    for j in 0..samples.len() {
        let si = sample_order.iter().position(|s| s == &samples[j]).unwrap();
        let gi = gene_order.iter().position(|g| g == &genes[j]).unwrap();
        grid.insert((gi, si), mut_types[j].clone());
    }

    let type_colors: HashMap<&str, &str> = [
        ("missense", "#e15759"),
        ("nonsense", "#333"),
        ("frameshift", "#4e79a7"),
        ("splice", "#76b7b2"),
        ("mutation", "#e15759"),
    ]
    .into();

    if fmt == "svg" {
        let cell = 12.0;
        let w = get_opt_f64(
            &opts,
            "width",
            (sample_order.len() as f64 * cell + 120.0).max(400.0),
        );
        let h = get_opt_f64(
            &opts,
            "height",
            (gene_order.len() as f64 * cell + 60.0).max(200.0),
        );
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 100.0;
        let cw = c.plot_width() / sample_order.len().max(1) as f64;
        let ch = c.plot_height() / gene_order.len().max(1) as f64;
        for gi in 0..gene_order.len() {
            let y = c.margin.top + gi as f64 * ch;
            c.add_text(
                c.margin.left - 3.0,
                y + ch / 2.0 + 4.0,
                &gene_order[gi],
                "end",
                10.0,
            );
            for si in 0..sample_order.len() {
                let x = c.margin.left + si as f64 * cw;
                c.add_rect(x, y, cw - 1.0, ch - 1.0, "#f0f0f0");
                if let Some(mt) = grid.get(&(gi, si)) {
                    c.add_rect(
                        x,
                        y + ch * 0.15,
                        cw - 1.0,
                        ch * 0.7,
                        type_colors.get(mt.as_str()).copied().unwrap_or("#e15759"),
                    );
                }
            }
        }
        finish_themed_canvas(&mut c, &opts, "OncoPrint");
        return Ok(Value::Str(c.render()));
    }

    let max_gl = gene_order.iter().map(|g| g.len()).max().unwrap_or(4);
    let mut out = String::from("  OncoPrint\n");
    for gi in 0..gene_order.len() {
        out.push_str(&format!("  {:>w$}  ", gene_order[gi], w = max_gl));
        for si in 0..sample_order.len() {
            out.push(if grid.contains_key(&(gi, si)) {
                '█'
            } else {
                '·'
            });
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 14. venn ────────────────────────────────────────────────────

fn builtin_venn(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let sets: Vec<(String, HashSet<String>)> = match &args[0] {
        Value::Record(map) => map
            .iter()
            .map(|(name, val)| {
                let items: HashSet<String> = match val {
                    Value::List(items) => items.iter().map(|v| format!("{v}")).collect(),
                    _ => HashSet::new(),
                };
                (name.clone(), items)
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "venn() requires Record of Lists",
                None,
            ))
        }
    };
    if sets.len() < 2 || sets.len() > 4 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "venn() needs 2-4 sets",
            None,
        ));
    }
    let names: Vec<&str> = sets.iter().map(|(n, _)| n.as_str()).collect();
    let set_refs: Vec<&HashSet<String>> = sets.iter().map(|(_, s)| s).collect();

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 500.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let r = w.min(h) * 0.25;
        let colors = ["#4e79a7", "#e15759", "#59a14f", "#edc948"];
        let offsets: Vec<(f64, f64)> = match sets.len() {
            2 => vec![(-r * 0.35, 0.0), (r * 0.35, 0.0)],
            3 => vec![(-r * 0.3, -r * 0.2), (r * 0.3, -r * 0.2), (0.0, r * 0.3)],
            _ => vec![
                (-r * 0.3, -r * 0.3),
                (r * 0.3, -r * 0.3),
                (-r * 0.3, r * 0.3),
                (r * 0.3, r * 0.3),
            ],
        };
        for (j, (dx, dy)) in offsets.iter().enumerate() {
            c.elements.push(format!(
                r#"<circle cx="{:.1}" cy="{:.1}" r="{r:.1}" fill="{}" opacity="0.25" stroke="{}" stroke-width="2" />"#,
                cx + dx, cy + dy, colors[j], colors[j]
            ));
            c.add_text(cx + dx * 2.5, cy + dy * 2.5, names[j], "middle", 12.0);
        }
        if sets.len() >= 2 {
            let inter: usize = set_refs[0].intersection(set_refs[1]).count();
            c.add_text(cx, cy, &inter.to_string(), "middle", 14.0);
        }
        finish_themed_canvas(&mut c, &opts, "Venn Diagram");
        return Ok(Value::Str(c.render()));
    }

    let mut out = String::from("  Venn Diagram\n");
    for (name, set) in &sets {
        out.push_str(&format!("  {name}: {} items\n", set.len()));
    }
    out.push('\n');
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            let inter = set_refs[i].intersection(set_refs[j]).count();
            out.push_str(&format!("  {} ∩ {} = {}\n", names[i], names[j], inter));
        }
    }
    let mut common: HashSet<String> = set_refs[0].clone();
    for s in &set_refs[1..] {
        common = common.intersection(s).cloned().collect();
    }
    out.push_str(&format!("  All: {} shared\n", common.len()));
    write_output(&out);
    Ok(Value::Nil)
}

// ── 15. upset ───────────────────────────────────────────────────

fn builtin_upset(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let sets: Vec<(String, HashSet<String>)> = match &args[0] {
        Value::Record(map) => map
            .iter()
            .map(|(n, v)| {
                let items: HashSet<String> = match v {
                    Value::List(l) => l.iter().map(|x| format!("{x}")).collect(),
                    _ => HashSet::new(),
                };
                (n.clone(), items)
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "upset() requires Record of Lists",
                None,
            ))
        }
    };
    let n = sets.len();
    if n < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "upset() needs >= 2 sets",
            None,
        ));
    }

    // Compute all intersection combinations
    let all_items: HashSet<String> = sets.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
    let mut combos: Vec<(Vec<bool>, usize)> = Vec::new();
    for mask in 1..(1u32 << n) {
        let membership: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
        let count = all_items
            .iter()
            .filter(|item| (0..n).all(|i| membership[i] == sets[i].1.contains(*item)))
            .count();
        if count > 0 {
            combos.push((membership, count));
        }
    }
    combos.sort_by(|a, b| b.1.cmp(&a.1));

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 100.0;
        c.margin.bottom = 80.0;
        let nc = combos.len().min(20);
        let bar_area_h = c.plot_height() * 0.6;
        let dot_area_h = c.plot_height() * 0.4;
        let bar_w = c.plot_width() / nc as f64;
        let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;
        for (ci, (membership, count)) in combos.iter().take(nc).enumerate() {
            let x = c.margin.left + ci as f64 * bar_w + bar_w * 0.15;
            let bw = bar_w * 0.7;
            let bh = (*count as f64 / max_count) * bar_area_h;
            c.add_rect(x, c.margin.top + bar_area_h - bh, bw, bh, PALETTE[0]);
            c.add_text(
                x + bw / 2.0,
                c.margin.top + bar_area_h - bh - 5.0,
                &count.to_string(),
                "middle",
                9.0,
            );
            // Dot matrix
            let dot_top = c.margin.top + bar_area_h + 10.0;
            for (si, &active) in membership.iter().enumerate() {
                let dy = dot_top + si as f64 * (dot_area_h / n as f64);
                let dx = x + bw / 2.0;
                c.add_circle(dx, dy + 5.0, 4.0, if active { "#333" } else { "#ddd" });
            }
        }
        // Set labels
        let dot_top = c.margin.top + bar_area_h + 10.0;
        for (si, (name, _)) in sets.iter().enumerate() {
            let y = dot_top + si as f64 * (dot_area_h / n as f64) + 9.0;
            c.add_text(c.margin.left - 5.0, y, name, "end", 10.0);
        }
        finish_themed_canvas(&mut c, &opts, "UpSet Plot");
        return Ok(Value::Str(c.render()));
    }

    let max_name = sets.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let nc = combos.len().min(15);
    let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1);
    let mut out = String::from("  UpSet Plot\n");
    // Bar row
    out.push_str(&format!("  {:>w$}  ", "count", w = max_name));
    for (_, count) in combos.iter().take(nc) {
        let _bar_len = (*count as f64 / max_count as f64 * 5.0).ceil() as usize;
        out.push_str(&format!("{:>3} ", count));
    }
    out.push('\n');
    // Dot matrix
    for (si, (name, _)) in sets.iter().enumerate() {
        out.push_str(&format!("  {:>w$}  ", name, w = max_name));
        for (membership, _) in combos.iter().take(nc) {
            out.push_str(if membership[si] { " ●  " } else { " ·  " });
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 16. sequence_logo ───────────────────────────────────────────

fn builtin_sequence_logo(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let seqs: Vec<String> = match &args[0] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => seq.data.clone(),
                _ => String::new(),
            })
            .filter(|s| !s.is_empty())
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "sequence_logo() requires List of sequences",
                None,
            ))
        }
    };
    if seqs.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sequence_logo() empty input",
            None,
        ));
    }
    let seq_len = seqs[0].len();
    let n = seqs.len() as f64;
    let is_dna = seqs[0].chars().all(|c| "ACGTUacgtu".contains(c));
    let alphabet_size: f64 = if is_dna { 4.0 } else { 20.0 };
    let max_bits = alphabet_size.log2();

    // Compute per-position information content
    let mut positions: Vec<Vec<(char, f64)>> = Vec::new(); // (char, height) per position
    for pos in 0..seq_len {
        let mut counts: HashMap<char, f64> = HashMap::new();
        for seq in &seqs {
            if let Some(ch) = seq.chars().nth(pos) {
                *counts.entry(ch.to_ascii_uppercase()).or_insert(0.0) += 1.0;
            }
        }
        let entropy: f64 = counts
            .values()
            .map(|&c| {
                let p = c / n;
                if p > 0.0 {
                    -p * p.log2()
                } else {
                    0.0
                }
            })
            .sum();
        let ic = max_bits - entropy;
        let mut chars: Vec<(char, f64)> =
            counts.iter().map(|(&ch, &c)| (ch, (c / n) * ic)).collect();
        chars.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        positions.push(chars);
    }

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", (seq_len as f64 * 30.0 + 80.0).min(1200.0));
        let h = get_opt_f64(&opts, "height", 200.0);
        let mut c = themed_canvas(w, h, &opts);
        let col_w = c.plot_width() / seq_len as f64;
        let y_scale = Scale {
            domain: (0.0, max_bits),
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        let char_colors: HashMap<char, &str> = [
            ('A', "#4caf50"),
            ('T', "#f44336"),
            ('U', "#f44336"),
            ('G', "#ff9800"),
            ('C', "#2196f3"),
        ]
        .into();
        for (pos, chars) in positions.iter().enumerate() {
            let x = c.margin.left + pos as f64 * col_w;
            let mut y_bottom = y_scale.map(0.0);
            for &(ch, height) in chars {
                let _y_top = y_scale.map(height);
                let letter_h = y_bottom - y_scale.map(height);
                if letter_h > 1.0 {
                    let color = char_colors.get(&ch).copied().unwrap_or("#333");
                    let font_size = (letter_h * 0.9).min(col_w * 0.9);
                    let escaped = format!("{ch}");
                    c.elements.push(format!(
                        r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{font_size:.0}" font-family="monospace" font-weight="bold" fill="{color}">{escaped}</text>"#,
                        x + col_w / 2.0, y_bottom
                    ));
                }
                y_bottom -= letter_h;
            }
        }
        let dy = Scale {
            domain: (0.0, max_bits),
            range: (0.0, max_bits),
        };
        c.draw_y_axis(&dy, "bits");
        finish_themed_canvas(&mut c, &opts, "Sequence Logo");
        return Ok(Value::Str(c.render()));
    }

    // ASCII logo: show top char per position with height indicator
    let mut out = String::from("  Sequence Logo\n  ");
    for chars in positions.iter() {
        if let Some(&(ch, _)) = chars.last() {
            out.push(ch);
        } else {
            out.push(' ');
        }
    }
    out.push_str("\n  ");
    for chars in positions.iter() {
        let total_ic: f64 = chars.iter().map(|(_, h)| h).sum();
        let bar = if total_ic > max_bits * 0.75 {
            '█'
        } else if total_ic > max_bits * 0.5 {
            '▄'
        } else if total_ic > max_bits * 0.25 {
            '▂'
        } else {
            '▁'
        };
        out.push(bar);
    }
    out.push_str(&format!("\n  (n={}, len={})\n", seqs.len(), seq_len));
    write_output(&out);
    Ok(Value::Nil)
}

// ── 17. phylo_tree ──────────────────────────────────────────────

#[derive(Clone)]
struct TreeNode {
    name: String,
    branch_len: f64,
    children: Vec<TreeNode>,
}

fn parse_newick(s: &str) -> Result<TreeNode> {
    let s = s.trim().trim_end_matches(';');
    let (node, _) = parse_newick_node(s.as_bytes(), 0)?;
    Ok(node)
}

fn parse_newick_node(data: &[u8], mut pos: usize) -> Result<(TreeNode, usize)> {
    let mut children = Vec::new();
    if pos < data.len() && data[pos] == b'(' {
        pos += 1; // skip '('
        loop {
            let (child, new_pos) = parse_newick_node(data, pos)?;
            children.push(child);
            pos = new_pos;
            if pos >= data.len() || data[pos] != b',' {
                break;
            }
            pos += 1; // skip ','
        }
        if pos < data.len() && data[pos] == b')' {
            pos += 1;
        }
    }
    // Parse name
    let mut name = String::new();
    while pos < data.len() && !b",):;".contains(&data[pos]) && data[pos] != b':' {
        name.push(data[pos] as char);
        pos += 1;
    }
    // Parse branch length
    let mut bl = 0.0;
    if pos < data.len() && data[pos] == b':' {
        pos += 1;
        let start = pos;
        while pos < data.len()
            && (data[pos].is_ascii_digit()
                || data[pos] == b'.'
                || data[pos] == b'-'
                || data[pos] == b'e'
                || data[pos] == b'E')
        {
            pos += 1;
        }
        if let Ok(v) = std::str::from_utf8(&data[start..pos])
            .unwrap_or("0")
            .parse::<f64>()
        {
            bl = v;
        }
    }
    Ok((
        TreeNode {
            name: name.trim().to_string(),
            branch_len: bl,
            children,
        },
        pos,
    ))
}

fn builtin_phylo_tree(args: Vec<Value>) -> Result<Value> {
    let newick = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "phylo_tree() requires Str (Newick format)",
                None,
            ))
        }
    };
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let root = parse_newick(&newick)?;

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 40.0;
        c.margin.right = 100.0;
        let leaves = count_leaves(&root);
        let max_depth = max_tree_depth(&root);
        let ml = c.margin.left;
        let mt = c.margin.top;
        let pw = c.plot_width();
        let ph = c.plot_height();
        draw_tree_svg(&mut c, &root, 0.0, max_depth, 0, leaves, ml, mt, pw, ph);
        finish_themed_canvas(&mut c, &opts, "Phylogenetic Tree");
        return Ok(Value::Str(c.render()));
    }

    let mut out = String::from("  Phylogenetic Tree\n");
    render_tree_ascii(&root, &mut out, "", true);
    write_output(&out);
    Ok(Value::Nil)
}

fn count_leaves(node: &TreeNode) -> usize {
    if node.children.is_empty() {
        1
    } else {
        node.children.iter().map(count_leaves).sum()
    }
}

fn max_tree_depth(node: &TreeNode) -> f64 {
    if node.children.is_empty() {
        node.branch_len
    } else {
        node.branch_len
            + node
                .children
                .iter()
                .map(max_tree_depth)
                .fold(0.0f64, f64::max)
    }
}

fn render_tree_ascii(node: &TreeNode, out: &mut String, prefix: &str, is_last: bool) {
    let connector = if prefix.is_empty() {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };
    let label = if node.name.is_empty() {
        String::new()
    } else {
        format!(" {}", node.name)
    };
    let bl = if node.branch_len > 0.0 {
        format!(":{:.4}", node.branch_len)
    } else {
        String::new()
    };
    out.push_str(&format!("  {prefix}{connector}{label}{bl}\n"));
    let child_prefix = if prefix.is_empty() {
        String::new()
    } else if is_last {
        format!("{prefix}    ")
    } else {
        format!("{prefix}│   ")
    };
    for (i, child) in node.children.iter().enumerate() {
        render_tree_ascii(child, out, &child_prefix, i == node.children.len() - 1);
    }
}

fn draw_tree_svg(
    c: &mut SvgCanvas,
    node: &TreeNode,
    x: f64,
    max_d: f64,
    leaf_idx: usize,
    total_leaves: usize,
    left: f64,
    top: f64,
    pw: f64,
    ph: f64,
) -> (f64, usize) {
    let x_pos = left + (x / max_d.max(0.001)) * pw;
    if node.children.is_empty() {
        let y_pos = top + (leaf_idx as f64 + 0.5) / total_leaves as f64 * ph;
        c.add_circle(x_pos, y_pos, 3.0, PALETTE[0]);
        if !node.name.is_empty() {
            c.add_text(x_pos + 8.0, y_pos + 4.0, &node.name, "start", 10.0);
        }
        return (y_pos, leaf_idx + 1);
    }
    let mut child_ys = Vec::new();
    let mut li = leaf_idx;
    for child in &node.children {
        let child_x = x + child.branch_len;
        let (cy, new_li) = draw_tree_svg(
            c,
            child,
            child_x,
            max_d,
            li,
            total_leaves,
            left,
            top,
            pw,
            ph,
        );
        let cx = left + (child_x / max_d.max(0.001)) * pw;
        c.add_line(x_pos, cy, cx, cy, "#333", 1.5);
        child_ys.push(cy);
        li = new_li;
    }
    if child_ys.len() >= 2 {
        let y_min = child_ys.iter().cloned().fold(f64::INFINITY, f64::min);
        let y_max = child_ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        c.add_line(x_pos, y_min, x_pos, y_max, "#333", 1.5);
    }
    let mid_y = child_ys.iter().sum::<f64>() / child_ys.len() as f64;
    (mid_y, li)
}

// ── 18. lollipop ────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct GenomeFeatureDatum {
    source_row: usize,
    chromosome: Option<String>,
    original_start: f64,
    original_end: f64,
    start: f64,
    end: f64,
    name: Option<String>,
    strand: String,
    lane: usize,
    label_drawn: bool,
}

fn optional_string_cell(
    row: &[Value],
    column: Option<usize>,
    family: &str,
) -> Result<Option<String>> {
    match column.map(|index| &row[index]) {
        None | Some(Value::Nil) => Ok(None),
        Some(Value::Str(value)) => Ok((!value.is_empty()).then(|| value.clone())),
        Some(_) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{family}() optional text columns must contain strings or nil"),
            None,
        )),
    }
}

fn assign_interval_lanes<T>(
    items: &mut [T],
    start: impl Fn(&T) -> f64,
    end: impl Fn(&T) -> f64,
    set_lane: impl Fn(&mut T, usize),
) -> usize {
    let mut lane_ends: Vec<f64> = Vec::new();
    for item in items {
        let item_start = start(item);
        let lane = lane_ends
            .iter()
            .position(|lane_end| *lane_end <= item_start)
            .unwrap_or_else(|| {
                lane_ends.push(f64::NEG_INFINITY);
                lane_ends.len() - 1
            });
        lane_ends[lane] = end(item);
        set_lane(item, lane);
    }
    lane_ends.len().max(1)
}

fn genome_track_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let table = require_table_bp(value, "genome_track")?;
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() empty data",
            None,
        ));
    }
    let start_column = table.col_index("start").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'start' not found", None)
    })?;
    let end_column = table.col_index("end").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'end' not found", None)
    })?;
    let chromosome_column = table
        .col_index("chrom")
        .or_else(|| table.col_index("chromosome"));
    let name_column = table.col_index(get_opt_str(opts, "label", "name"));
    let strand_column = table.col_index("strand");
    let mut features = Vec::with_capacity(table.num_rows());
    for (source_row, row) in table.rows.iter().enumerate() {
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "genome_track() start/end must be finite, non-negative, and end > start",
                None,
            ));
        }
        let chromosome = optional_string_cell(row, chromosome_column, "genome_track")?;
        let name = optional_string_cell(row, name_column, "genome_track")?;
        let strand = optional_string_cell(row, strand_column, "genome_track")?.unwrap_or_default();
        if !matches!(strand.as_str(), "" | "+" | "-" | ".") {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "genome_track() strand must be '+', '-', '.', or nil",
                None,
            ));
        }
        features.push(GenomeFeatureDatum {
            source_row,
            chromosome,
            original_start: start,
            original_end: end,
            start,
            end,
            name,
            strand,
            lane: 0,
            label_drawn: false,
        });
    }
    let input_rows = features.len();
    let requested_chromosome = opts.get("chromosome").and_then(Value::as_str);
    if let Some(chromosome) = requested_chromosome {
        if chromosome.trim().is_empty()
            || features.iter().all(|feature| feature.chromosome.is_none())
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "genome_track() chromosome filtering needs chromosome data",
                None,
            ));
        }
        features.retain(|feature| feature.chromosome.as_deref() == Some(chromosome));
    }
    let rows_with_chromosomes = features
        .iter()
        .filter(|feature| feature.chromosome.is_some())
        .count();
    let chromosomes = features
        .iter()
        .filter_map(|feature| feature.chromosome.as_deref())
        .collect::<HashSet<_>>();
    if rows_with_chromosomes != 0 && rows_with_chromosomes != features.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() chromosome values must be present for every row or omitted from every row",
            None,
        ));
    }
    if chromosomes.len() > 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() draws one region; select one chromosome with {chromosome: \"chr1\"}",
            None,
        ));
    }
    let selected_chromosome = chromosomes.into_iter().next().map(str::to_string);
    let region_start = opts.get("region_start").and_then(Value::as_float);
    let region_end = opts.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|number| !number.is_finite() || number < 0.0)
        || region_end.is_some_and(|number| !number.is_finite() || number < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() region_start/region_end must be finite, non-negative and increasing",
            None,
        ));
    }
    let mut clipped_rows = 0usize;
    features = features
        .into_iter()
        .filter_map(|mut feature| {
            if region_start.is_some_and(|start| feature.end <= start)
                || region_end.is_some_and(|end| feature.start >= end)
            {
                return None;
            }
            feature.start = region_start.map_or(feature.start, |start| feature.start.max(start));
            feature.end = region_end.map_or(feature.end, |end| feature.end.min(end));
            if feature.start != feature.original_start || feature.end != feature.original_end {
                clipped_rows += 1;
            }
            (feature.end > feature.start).then_some(feature)
        })
        .collect();
    if features.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() no features after chromosome/region filtering",
            None,
        ));
    }
    features.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let lane_count = assign_interval_lanes(
        &mut features,
        |feature| feature.start,
        |feature| feature.end,
        |feature, lane| feature.lane = lane,
    );
    let domain_start = region_start.unwrap_or_else(|| {
        features
            .iter()
            .map(|feature| feature.start)
            .fold(f64::INFINITY, f64::min)
    });
    let domain_end = region_end.unwrap_or_else(|| {
        features
            .iter()
            .map(|feature| feature.end)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let width = get_opt_f64(opts, "width", 1000.0);
    let show_labels = opts
        .get("show_labels")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let max_labels = get_opt_usize(opts, "max_labels", 100);
    if show_labels && max_labels > 0 {
        let span = domain_end - domain_start;
        let plot_width = (width - 80.0).max(1.0);
        let mut lane_right = vec![f64::NEG_INFINITY; lane_count];
        let mut drawn = 0usize;
        for feature in &mut features {
            let Some(name) = feature.name.as_deref() else {
                continue;
            };
            if drawn >= max_labels {
                break;
            }
            let x = (feature.start - domain_start) / span * plot_width;
            let right = x + estimate_text_width(name, 9.0) + 6.0;
            if x >= lane_right[feature.lane] {
                feature.label_drawn = true;
                lane_right[feature.lane] = right;
                drawn += 1;
            }
        }
    }
    let retained_rows = features.len();
    let rows = features
        .iter()
        .enumerate()
        .map(|(index, feature)| {
            vec![
                Value::Int(index as i64),
                Value::Int(feature.source_row as i64),
                feature
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(feature.original_start),
                Value::Float(feature.original_end),
                Value::Float(feature.start),
                Value::Float(feature.end),
                Value::Float(feature.end - feature.start),
                feature.name.clone().map(Value::Str).unwrap_or(Value::Nil),
                if feature.strand.is_empty() {
                    Value::Nil
                } else {
                    Value::Str(feature.strand.clone())
                },
                Value::Int(feature.lane as i64),
                Value::Str(PALETTE[feature.lane % PALETTE.len()].into()),
                Value::Bool(feature.label_drawn),
                Value::Bool(
                    feature.start != feature.original_start || feature.end != feature.original_end,
                ),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Genome Track");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("genome_track".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "feature_index",
                        "source_row",
                        "chromosome",
                        "original_start",
                        "original_end",
                        "start",
                        "end",
                        "length",
                        "name",
                        "strand",
                        "lane",
                        "color",
                        "label_drawn",
                        "clipped",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("width".into(), Value::Float(width)),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 300.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Genomic position").into()),
                        ),
                        (
                            "chromosome".into(),
                            selected_chromosome.map(Value::Str).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_start".into(),
                            region_start.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_end".into(),
                            region_end.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        ("domain_start".into(), Value::Float(domain_start)),
                        ("domain_end".into(), Value::Float(domain_end)),
                        ("lane_count".into(), Value::Int(lane_count as i64)),
                        ("show_labels".into(), Value::Bool(show_labels)),
                        ("max_labels".into(), Value::Int(max_labels as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("genome_track".into())),
                        ("input_rows".into(), Value::Int(input_rows as i64)),
                        ("retained_rows".into(), Value::Int(retained_rows as i64)),
                        ("clipped_rows".into(), Value::Int(clipped_rows as i64)),
                        (
                            "lane_rule".into(),
                            Value::Str("greedy_first_non_overlapping_lane".into()),
                        ),
                        (
                            "row_order".into(),
                            Value::Str("start_end_source_row".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "warnings".into(),
                Value::List(
                    if retained_rows == input_rows && clipped_rows == 0 {
                        Vec::new()
                    } else {
                        vec![Value::Str(format!(
                            "{retained_rows} features retained from {input_rows}; {clipped_rows} clipped to the requested region"
                        ))]
                    }
                    .into(),
                ),
            ),
        ])
        .into(),
    ))
}

fn render_genome_track_svg(table: &Table, opts: &HashMap<String, Value>) -> Result<String> {
    let width = get_opt_f64(opts, "width", 1000.0);
    let height = get_opt_f64(opts, "height", 300.0);
    let title = get_opt_str(opts, "title", "Genome Track");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let xlabel = get_opt_str(opts, "xlabel", "Genomic position");
    let domain = (
        get_opt_f64(opts, "domain_start", 0.0),
        get_opt_f64(opts, "domain_end", 1.0),
    );
    let lane_count = get_opt_usize(opts, "lane_count", 1).max(1);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[domain.0, domain.1],
        &[0.0, lane_count as f64],
        xlabel,
        "",
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let lane_step = (canvas.plot_height() / lane_count as f64).max(1.0);
    let feature_height = (lane_step * 0.48).clamp(4.0, 16.0);
    let backbone_y = canvas.margin.top + canvas.plot_height();
    canvas.add_line(
        canvas.margin.left,
        backbone_y,
        canvas.margin.left + canvas.plot_width(),
        backbone_y,
        "#b8bcc4",
        1.5,
    );
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let name_column = table.col_index("name").unwrap();
    let strand_column = table.col_index("strand").unwrap();
    let lane_column = table.col_index("lane").unwrap();
    let color_column = table.col_index("color").unwrap();
    let label_column = table.col_index("label_drawn").unwrap();
    let dense = table.num_rows() > 200;
    let mut dense_paths: BTreeMap<String, String> = BTreeMap::new();
    let mut arrow_path = String::new();
    for row in &table.rows {
        let start = row[start_column].as_float().unwrap();
        let end = row[end_column].as_float().unwrap();
        let lane = row[lane_column].as_float().unwrap() as usize;
        let color = row[color_column].as_str().unwrap();
        let x1 = x_scale.map(start);
        let x2 = x_scale.map(end);
        let y = canvas.margin.top + lane as f64 * lane_step + (lane_step - feature_height) / 2.0;
        let feature_width = (x2 - x1).max(1.5);
        if dense {
            dense_paths
                .entry(color.into())
                .or_default()
                .push_str(&format!(
                    "M{x1:.2},{y:.2}h{feature_width:.2}v{feature_height:.2}h-{feature_width:.2}Z"
                ));
        } else {
            canvas.add_rect(x1, y, feature_width, feature_height, color);
        }
        let strand = row[strand_column].as_str().unwrap_or("");
        if matches!(strand, "+" | "-") {
            let tip = if strand == "+" { x2 - 1.0 } else { x1 + 1.0 };
            let inward = if strand == "+" { -1.0 } else { 1.0 };
            let mid = y + feature_height / 2.0;
            arrow_path.push_str(&format!(
                "M{:.2},{:.2}L{tip:.2},{mid:.2}L{:.2},{:.2}",
                tip + inward * 5.0,
                mid - feature_height * 0.28,
                tip + inward * 5.0,
                mid + feature_height * 0.28
            ));
        }
        if row[label_column].as_bool().unwrap_or(false) {
            if let Some(name) = row[name_column].as_str() {
                canvas.add_text(x1, y - 3.0, name, "start", 9.0);
            }
        }
    }
    for (color, path) in dense_paths {
        canvas
            .elements
            .push(format!(r#"<path d="{path}" fill="{color}" />"#));
    }
    if !arrow_path.is_empty() {
        canvas.elements.push(format!(
            r##"<path d="{arrow_path}" fill="none" stroke="#ffffff" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />"##
        ));
    }
    canvas.draw_x_axis(
        &Scale {
            domain,
            range: domain,
        },
        xlabel,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Genome annotation track with {} features arranged across {lane_count} non-overlapping lanes.",
        table.num_rows()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_genome_track_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "genome_track"))
}

pub(crate) fn render_genome_track_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_genome_track_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 genome-track Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genome-track specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "feature_index",
        "source_row",
        "chromosome",
        "original_start",
        "original_end",
        "start",
        "end",
        "length",
        "name",
        "strand",
        "lane",
        "color",
        "label_drawn",
        "clipped",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() genome-track data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genome-track specification has no features",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "genome-track")?;
    let domain_start = options
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let domain_end = options
        .get("domain_end")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let lane_count = options
        .get("lane_count")
        .map(|value| frozen_nonnegative_integer(value, "genome-track", "lane_count"))
        .transpose()?
        .unwrap_or(0);
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_start < 0.0
        || domain_end <= domain_start
        || lane_count == 0
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genome-track frozen domain or lane count is malformed",
            None,
        ));
    }
    let index_column = table.col_index("feature_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let original_start_column = table.col_index("original_start").unwrap();
    let original_end_column = table.col_index("original_end").unwrap();
    let chromosome_column = table.col_index("chromosome").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let length_column = table.col_index("length").unwrap();
    let name_column = table.col_index("name").unwrap();
    let strand_column = table.col_index("strand").unwrap();
    let lane_column = table.col_index("lane").unwrap();
    let color_column = table.col_index("color").unwrap();
    let label_column = table.col_index("label_drawn").unwrap();
    let clipped_column = table.col_index("clipped").unwrap();
    let selected_chromosome = options.get("chromosome").and_then(Value::as_str);
    let region_start = options.get("region_start").and_then(Value::as_float);
    let region_end = options.get("region_end").and_then(Value::as_float);
    let mut previous: Option<(f64, f64, usize)> = None;
    let mut lane_ends = vec![f64::NEG_INFINITY; lane_count];
    let mut highest_lane = 0usize;
    for (expected, row) in table.rows.iter().enumerate() {
        let feature_index =
            frozen_nonnegative_integer(&row[index_column], "genome-track", "feature_index")?;
        let source = frozen_nonnegative_integer(&row[source_column], "genome-track", "source_row")?;
        let original_start = row[original_start_column].as_float().unwrap_or(f64::NAN);
        let original_end = row[original_end_column].as_float().unwrap_or(f64::NAN);
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        let lane = frozen_nonnegative_integer(&row[lane_column], "genome-track", "lane")?;
        let chromosome = match &row[chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() genome-track chromosome is malformed",
                    None,
                ))
            }
        };
        let name_ok = matches!(&row[name_column], Value::Nil | Value::Str(_));
        let expected_lane = lane_ends
            .iter()
            .position(|lane_end| *lane_end <= start)
            .unwrap_or(lane_count);
        let strand_ok = matches!(&row[strand_column], Value::Nil)
            || matches!(row[strand_column].as_str(), Some("+" | "-" | "."));
        let key = (start, end, source);
        let clipped = start != original_start || end != original_end;
        let expected_start = region_start.map_or(original_start, |lower| original_start.max(lower));
        let expected_end = region_end.map_or(original_end, |upper| original_end.min(upper));
        if feature_index != expected
            || !original_start.is_finite()
            || !original_end.is_finite()
            || !start.is_finite()
            || !end.is_finite()
            || original_start < 0.0
            || original_end <= original_start
            || start < domain_start
            || end > domain_end
            || end <= start
            || (start - expected_start).abs() > 1e-10
            || (end - expected_end).abs() > 1e-10
            || (length - (end - start)).abs() > 1e-10
            || lane >= lane_count
            || lane != expected_lane
            || !strand_ok
            || !name_ok
            || (row[label_column].is_truthy() && row[name_column].as_str().is_none())
            || selected_chromosome != chromosome
            || !row[color_column].as_str().is_some_and(valid_bio_hex_color)
            || !matches!(row[label_column], Value::Bool(_))
            || !matches!(row[clipped_column], Value::Bool(_))
            || row[clipped_column].is_truthy() != clipped
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genome-track frozen geometry is inconsistent",
                None,
            ));
        }
        lane_ends[lane] = end;
        highest_lane = highest_lane.max(lane);
        previous = Some(key);
    }
    if highest_lane + 1 != lane_count {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genome-track frozen lane count is inconsistent",
            None,
        ));
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_genome_track_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Genome Track");
    finish_frozen_bio_plot(value, render_options, title, "genome-track", svg)
}

pub(crate) fn builtin_genome_track(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = genome_track_spec_value(&args[0], &opts)?;
    render_genome_track_plot_spec_value(&specification, &opts)
}

#[cfg(any())]
fn builtin_lollipop_legacy(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "lollipop")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let positions = extract_table_col(table, get_opt_str(&opts, "pos", "position"))?;
    let labels = extract_str_col(table, get_opt_str(&opts, "label", "label")).ok();
    let heights = extract_table_col(table, "count")
        .or_else(|_| extract_table_col(table, "height"))
        .ok();

    let xr = col_range(&positions);
    let ys = heights.as_ref().map(|h| col_range(h)).unwrap_or((0.0, 1.0));
    let yr = (0.0, ys.1 * 1.1);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let y_scale = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        // Domain bar
        let bar_y = c.margin.top + c.plot_height();
        c.add_rect(c.margin.left, bar_y - 8.0, c.plot_width(), 16.0, "#eee");
        for i in 0..positions.len() {
            let x = xs.map(positions[i]);
            let height_val = heights.as_ref().map(|h| h[i]).unwrap_or(1.0);
            let y = y_scale.map(height_val);
            c.add_line(x, bar_y, x, y, "#333", 1.5);
            c.add_circle(x, y, 5.0, PALETTE[i % PALETTE.len()]);
            if let Some(ref lbls) = labels {
                c.add_text(x, y - 8.0, &lbls[i], "middle", 8.0);
            }
        }
        let dx = Scale {
            domain: xr,
            range: xr,
        };
        c.draw_x_axis(&dx, "Position");
        c.draw_title("Lollipop Plot");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 12);
    let mut chart = AsciiChart::new(width, height);
    for i in 0..positions.len() {
        let h = heights.as_ref().map(|hv| hv[i]).unwrap_or(1.0);
        chart.put(positions[i], h, xr, yr, '●');
        // Draw stem
        let (gx, gy) = chart.map(positions[i], h, xr, yr);
        let (_, base_y) = chart.map(positions[i], 0.0, xr, yr);
        for y in gy..base_y {
            chart.grid[y][gx] = '│';
        }
        chart.grid[gy][gx] = '●';
    }
    write_output(&chart.render("Lollipop Plot"));
    Ok(Value::Nil)
}

// ── 19. circos ──────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct LollipopDatum {
    source_row: usize,
    position: f64,
    height: f64,
    label: Option<String>,
    label_lane: usize,
    label_drawn: bool,
}

fn lollipop_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let table = require_table_bp(value, "lollipop")?;
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "lollipop() empty data",
            None,
        ));
    }
    let position_name = get_opt_str(opts, "pos", "position");
    let position_column = table.col_index(position_name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{position_name}' not found"),
            None,
        )
    })?;
    let height_column = table
        .col_index("count")
        .or_else(|| table.col_index("height"));
    let label_column = table.col_index(get_opt_str(opts, "label", "label"));
    let mut data = Vec::with_capacity(table.num_rows());
    for (source_row, row) in table.rows.iter().enumerate() {
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let height = height_column
            .and_then(|column| row[column].as_float())
            .unwrap_or(1.0);
        if !position.is_finite() || position < 0.0 || !height.is_finite() || height < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "lollipop() positions and heights must be finite and non-negative",
                None,
            ));
        }
        data.push(LollipopDatum {
            source_row,
            position,
            height,
            label: optional_string_cell(row, label_column, "lollipop")?,
            label_lane: 0,
            label_drawn: false,
        });
    }
    data.sort_by(|left, right| {
        left.position
            .total_cmp(&right.position)
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let observed_start = data.first().unwrap().position;
    let observed_end = data.last().unwrap().position;
    let mut domain_start = opts
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or_else(|| {
            if opts.contains_key("length") {
                0.0
            } else {
                observed_start
            }
        });
    let mut domain_end = opts
        .get("domain_end")
        .and_then(Value::as_float)
        .or_else(|| opts.get("length").and_then(Value::as_float))
        .unwrap_or(observed_end);
    if domain_end == domain_start {
        domain_start = (domain_start - 0.5).max(0.0);
        domain_end += 0.5;
    }
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_start < 0.0
        || domain_end <= domain_start
        || observed_start < domain_start
        || observed_end > domain_end
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "lollipop() domain must be finite, increasing, and contain every position",
            None,
        ));
    }
    let show_labels = opts
        .get("show_labels")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let max_labels = get_opt_usize(opts, "max_labels", 80);
    if show_labels {
        let width = get_opt_f64(opts, "width", 800.0);
        let plot_width = (width - 80.0).max(1.0);
        let span = domain_end - domain_start;
        let mut lane_right = [f64::NEG_INFINITY; 2];
        let mut drawn = 0usize;
        for datum in &mut data {
            let Some(label) = datum.label.as_deref() else {
                continue;
            };
            if drawn >= max_labels {
                break;
            }
            let x = (datum.position - domain_start) / span * plot_width;
            let half = estimate_text_width(label, 8.0) / 2.0 + 4.0;
            if let Some(lane) = lane_right.iter().position(|right| x - half >= *right) {
                datum.label_lane = lane;
                datum.label_drawn = true;
                lane_right[lane] = x + half;
                drawn += 1;
            }
        }
    }
    let y_max = data
        .iter()
        .map(|datum| datum.height)
        .fold(0.0, f64::max)
        .max(1.0);
    let rows = data
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                Value::Float(datum.position),
                Value::Float(datum.height),
                datum.label.clone().map(Value::Str).unwrap_or(Value::Nil),
                Value::Int(datum.label_lane as i64),
                Value::Bool(datum.label_drawn),
                Value::Str(PALETTE[index % PALETTE.len()].into()),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Lollipop Plot");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("lollipop".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "position",
                        "height",
                        "label",
                        "label_lane",
                        "label_drawn",
                        "color",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 800.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 320.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Position").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "Count / value").into()),
                        ),
                        ("domain_start".into(), Value::Float(domain_start)),
                        ("domain_end".into(), Value::Float(domain_end)),
                        ("y_max".into(), Value::Float(y_max)),
                        ("show_labels".into(), Value::Bool(show_labels)),
                        ("max_labels".into(), Value::Int(max_labels as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("lollipop".into())),
                        ("input_rows".into(), Value::Int(data.len() as i64)),
                        ("row_order".into(), Value::Str("position_source_row".into())),
                        (
                            "label_rule".into(),
                            Value::Str("two_lane_first_fit_with_limit".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::new().into())),
        ])
        .into(),
    ))
}

fn render_lollipop_svg(table: &Table, opts: &HashMap<String, Value>) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 320.0);
    let title = get_opt_str(opts, "title", "Lollipop Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let xlabel = get_opt_str(opts, "xlabel", "Position");
    let ylabel = get_opt_str(opts, "ylabel", "Count / value");
    let x_domain = (
        get_opt_f64(opts, "domain_start", 0.0),
        get_opt_f64(opts, "domain_end", 1.0),
    );
    let y_domain = (0.0, get_opt_f64(opts, "y_max", 1.0) * 1.12);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[0.0, y_domain.1],
        xlabel,
        ylabel,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    let baseline = y_scale.map(0.0);
    canvas.add_rect(
        canvas.margin.left,
        baseline - 5.0,
        canvas.plot_width(),
        10.0,
        "#e2e5e9",
    );
    let position_column = table.col_index("position").unwrap();
    let height_column = table.col_index("height").unwrap();
    let label_column = table.col_index("label").unwrap();
    let label_lane_column = table.col_index("label_lane").unwrap();
    let label_drawn_column = table.col_index("label_drawn").unwrap();
    let color_column = table.col_index("color").unwrap();
    let mut stems = String::new();
    let mut dense_points: BTreeMap<String, String> = BTreeMap::new();
    let dense = table.num_rows() > 200;
    for row in &table.rows {
        let x = x_scale.map(row[position_column].as_float().unwrap());
        let y = y_scale.map(row[height_column].as_float().unwrap());
        let color = row[color_column].as_str().unwrap();
        stems.push_str(&format!("M{x:.2},{baseline:.2}V{y:.2}"));
        if dense {
            dense_points
                .entry(color.into())
                .or_default()
                .push_str(&format!(
                    "M{:.2},{y:.2}a4,4 0 1,0 8,0a4,4 0 1,0 -8,0",
                    x - 4.0
                ));
        } else {
            canvas.add_circle(x, y, 5.0, color);
        }
        if row[label_drawn_column].as_bool().unwrap_or(false) {
            if let Some(label) = row[label_column].as_str() {
                let lane = row[label_lane_column].as_float().unwrap_or(0.0);
                canvas.add_text(x, y - 8.0 - lane * 11.0, label, "middle", 8.0);
            }
        }
    }
    canvas.elements.push(format!(
        r##"<path d="{stems}" fill="none" stroke="#343a40" stroke-width="1.4" />"##
    ));
    for (color, path) in dense_points {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="{color}" fill-opacity="0.78" />"#
        ));
    }
    canvas.draw_x_axis(
        &Scale {
            domain: x_domain,
            range: x_domain,
        },
        xlabel,
    );
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        ylabel,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Lollipop plot with {} observations across positions {:.3} to {:.3}.",
        table.num_rows(),
        x_domain.0,
        x_domain.1
    ));
    Ok(canvas.render())
}

pub(crate) fn is_lollipop_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "lollipop"))
}

pub(crate) fn render_lollipop_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_lollipop_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 lollipop Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() lollipop specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "position",
        "height",
        "label",
        "label_lane",
        "label_drawn",
        "color",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() lollipop data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() lollipop specification has no observations",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "lollipop")?;
    let domain_start = options
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let domain_end = options
        .get("domain_end")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let y_max = options
        .get("y_max")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_end <= domain_start
        || !y_max.is_finite()
        || y_max <= 0.0
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() lollipop frozen domains are malformed",
            None,
        ));
    }
    let index_column = table.col_index("point_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let position_column = table.col_index("position").unwrap();
    let height_column = table.col_index("height").unwrap();
    let label_lane_column = table.col_index("label_lane").unwrap();
    let label_drawn_column = table.col_index("label_drawn").unwrap();
    let color_column = table.col_index("color").unwrap();
    let label_value_column = table.col_index("label").unwrap();
    let mut previous: Option<(f64, usize)> = None;
    let mut observed_y_max = 1.0f64;
    for (expected, row) in table.rows.iter().enumerate() {
        let index = frozen_nonnegative_integer(&row[index_column], "lollipop", "point_index")?;
        let source = frozen_nonnegative_integer(&row[source_column], "lollipop", "source_row")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let height = row[height_column].as_float().unwrap_or(f64::NAN);
        let label_lane =
            frozen_nonnegative_integer(&row[label_lane_column], "lollipop", "label_lane")?;
        let key = (position, source);
        if index != expected
            || !position.is_finite()
            || position < domain_start
            || position > domain_end
            || !height.is_finite()
            || height < 0.0
            || height > y_max
            || label_lane > 1
            || !matches!(&row[label_value_column], Value::Nil | Value::Str(_))
            || (row[label_drawn_column].is_truthy() && row[label_value_column].as_str().is_none())
            || !matches!(row[label_drawn_column], Value::Bool(_))
            || !row[color_column].as_str().is_some_and(valid_bio_hex_color)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() lollipop frozen geometry is inconsistent",
                None,
            ));
        }
        observed_y_max = observed_y_max.max(height);
        previous = Some(key);
    }
    if (observed_y_max - y_max).abs() > 1e-10 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() lollipop frozen y maximum is inconsistent",
            None,
        ));
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_lollipop_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Lollipop Plot");
    finish_frozen_bio_plot(value, render_options, title, "lollipop", svg)
}

fn builtin_lollipop(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = lollipop_spec_value(&args[0], &opts)?;
    render_lollipop_plot_spec_value(&specification, &opts)
}

#[derive(Clone, Debug)]
struct CircosChromosome {
    source_row: usize,
    name: String,
    start: f64,
    end: f64,
    angle_start: f64,
    angle_end: f64,
    color: String,
    label_drawn: bool,
}

#[derive(Clone, Debug)]
struct CircosTrackMeta {
    index: usize,
    name: String,
    kind: String,
    radial_inner: f64,
    radial_outer: f64,
    value_min: f64,
    value_max: f64,
}

#[derive(Clone, Debug)]
struct CircosTrackMark {
    track_index: usize,
    point_index: usize,
    source_row: usize,
    chromosome_index: usize,
    chromosome: String,
    start: f64,
    end: f64,
    value: f64,
    angle_start: f64,
    angle_end: f64,
    radial_inner: f64,
    radial_outer: f64,
    color: String,
    label: Option<String>,
    label_drawn: bool,
    clipped: bool,
}

#[derive(Clone, Debug)]
struct CircosLink {
    link_index: usize,
    source_row: usize,
    source_chromosome_index: usize,
    source_chromosome: String,
    source_start: f64,
    source_end: f64,
    target_chromosome_index: usize,
    target_chromosome: String,
    target_start: f64,
    target_end: f64,
    source_angle_start: f64,
    source_angle_end: f64,
    target_angle_start: f64,
    target_angle_end: f64,
    weight: f64,
    stroke_width: f64,
    color: String,
    label: Option<String>,
    label_drawn: bool,
}

fn table_column_alias(table: &Table, names: &[&str]) -> Option<usize> {
    names.iter().find_map(|name| table.col_index(name))
}

fn finite_table_number(row: &[Value], column: usize, family: &str, field: &str) -> Result<f64> {
    let value = row[column].as_float().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{family}() column '{field}' must be numeric"),
            None,
        )
    })?;
    if !value.is_finite() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{family}() column '{field}' must be finite"),
            None,
        ));
    }
    Ok(value)
}

fn required_circos_column(table: &Table, names: &[&str], description: &str) -> Result<usize> {
    table_column_alias(table, names).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "circos() {description} is missing column '{}'",
                names.join("' or '")
            ),
            None,
        )
    })
}

fn circos_chromosome_angle(chromosome: &CircosChromosome, position: f64) -> f64 {
    chromosome.angle_start
        + (position - chromosome.start) / (chromosome.end - chromosome.start)
            * (chromosome.angle_end - chromosome.angle_start)
}

fn normalized_track_kind(kind: &str) -> Result<String> {
    let normalized = match kind.to_ascii_lowercase().as_str() {
        "bar" => "bar",
        "line" | "coverage" => "line",
        "point" | "points" | "variant" | "variants" => "point",
        "heatmap" | "tile" | "tiles" => "heatmap",
        "cnv" | "copy_number" | "copy-number" => "cnv",
        "gene" | "genes" | "peak" | "peaks" | "interval" | "intervals" => "interval",
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "circos() unknown track type '{other}', expected bar/line/point/heatmap/cnv/interval"
                ),
                None,
            ))
        }
    };
    Ok(normalized.to_string())
}

fn circos_track_records(value: Option<&Value>) -> Result<Vec<(String, String, Table)>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut tracks = Vec::new();
    let parse_track = |fallback_name: String, item: &Value| -> Result<(String, String, Table)> {
        let Value::Record(record) = item else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() each tracks entry must be a Record with data and type",
                None,
            ));
        };
        let data = match record.get("data") {
            Some(Value::Table(table)) => table.clone(),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() each track needs a data Table",
                    None,
                ))
            }
        };
        let name = record
            .get("name")
            .or_else(|| record.get("label"))
            .and_then(Value::as_str)
            .unwrap_or(&fallback_name)
            .to_string();
        let kind = normalized_track_kind(
            record
                .get("type")
                .or_else(|| record.get("track"))
                .and_then(Value::as_str)
                .unwrap_or("bar"),
        )?;
        Ok((name, kind, data))
    };
    match value {
        Value::List(items) => {
            for (index, item) in items.iter().enumerate() {
                tracks.push(parse_track(format!("track {}", index + 1), item)?);
            }
        }
        Value::Record(record) => {
            let mut names = record.keys().cloned().collect::<Vec<_>>();
            names.sort();
            for name in names {
                let item = record.get(&name).unwrap();
                match item {
                    Value::Table(table) => tracks.push((name, "bar".into(), table.clone())),
                    Value::Record(_) => tracks.push(parse_track(name, item)?),
                    _ => {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            "circos() named tracks must contain Tables or track Records",
                            None,
                        ))
                    }
                }
            }
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() tracks must be a List or Record",
                None,
            ))
        }
    }
    Ok(tracks)
}

fn circos_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let Value::Record(input) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() requires {segments: Table, links?: Table, tracks?: List}",
            None,
        ));
    };
    let segments = match input.get("segments").or_else(|| input.get("chromosomes")) {
        Some(Value::Table(table)) if !table.rows.is_empty() => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() requires a non-empty segments Table",
                None,
            ))
        }
    };
    let chrom_column =
        required_circos_column(segments, &["chrom", "chr", "chromosome"], "segments")?;
    let end_column = required_circos_column(segments, &["end", "length", "size"], "segments")?;
    let start_column = table_column_alias(segments, &["start"]);
    let color_column = table_column_alias(segments, &["color", "colour"]);
    let width = get_opt_f64(opts, "width", 700.0);
    let height = get_opt_f64(opts, "height", 700.0);
    if !width.is_finite() || !height.is_finite() || width < 160.0 || height < 160.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() width and height must be finite and at least 160",
            None,
        ));
    }
    let gap_degrees = get_opt_f64(opts, "gap_degrees", 2.0);
    let start_degrees = get_opt_f64(opts, "start_degrees", -90.0);
    if !gap_degrees.is_finite() || gap_degrees < 0.0 || !start_degrees.is_finite() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() gap_degrees must be non-negative and angles must be finite",
            None,
        ));
    }
    let mut chromosomes = Vec::<CircosChromosome>::new();
    let mut seen = HashSet::<String>::new();
    for (source_row, row) in segments.rows.iter().enumerate() {
        let name = row[chrom_column]
            .as_str()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() chromosome names must be non-empty strings",
                    None,
                )
            })?
            .to_string();
        if !seen.insert(name.clone()) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("circos() chromosome '{name}' appears more than once"),
                None,
            ));
        }
        let start = match start_column {
            Some(column) => finite_table_number(row, column, "circos", "start")?,
            None => 0.0,
        };
        let end = finite_table_number(row, end_column, "circos", "end")?;
        if start < 0.0 || end <= start {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() chromosome intervals require 0 <= start < end",
                None,
            ));
        }
        let color = color_column
            .and_then(|column| row[column].as_str())
            .unwrap_or(PALETTE[source_row % PALETTE.len()])
            .to_string();
        if !valid_bio_hex_color(&color) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() segment colors must be #rrggbb",
                None,
            ));
        }
        chromosomes.push(CircosChromosome {
            source_row,
            name,
            start,
            end,
            angle_start: 0.0,
            angle_end: 0.0,
            color,
            label_drawn: false,
        });
    }
    let gap = gap_degrees.to_radians();
    let available = std::f64::consts::TAU - gap * chromosomes.len() as f64;
    if available <= 0.05 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() chromosome gaps leave no drawable circle",
            None,
        ));
    }
    let total_size = chromosomes
        .iter()
        .map(|chromosome| chromosome.end - chromosome.start)
        .sum::<f64>();
    let mut angle = start_degrees.to_radians();
    let label_radius = width.min(height) * 0.39;
    let max_labels = get_opt_usize(opts, "max_labels", chromosomes.len());
    for (index, chromosome) in chromosomes.iter_mut().enumerate() {
        let sweep = (chromosome.end - chromosome.start) / total_size * available;
        chromosome.angle_start = angle;
        chromosome.angle_end = angle + sweep;
        chromosome.label_drawn = index < max_labels
            && sweep * label_radius
                >= estimate_text_width(&chromosome.name, plot_theme(opts).legend_size) + 4.0;
        angle += sweep + gap;
    }
    let lookup = chromosomes
        .iter()
        .enumerate()
        .map(|(index, chromosome)| (chromosome.name.clone(), index))
        .collect::<HashMap<_, _>>();

    let raw_tracks = circos_track_records(input.get("tracks"))?;
    let track_band = get_opt_f64(opts, "track_width", 0.075);
    let track_gap = get_opt_f64(opts, "track_gap", 0.018);
    if !track_band.is_finite()
        || !track_gap.is_finite()
        || track_band <= 0.0
        || track_gap < 0.0
        || 0.86 - raw_tracks.len() as f64 * (track_band + track_gap) < 0.18
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() track widths leave too little room for the chord layer",
            None,
        ));
    }
    let mut track_meta = Vec::<CircosTrackMeta>::new();
    let mut track_marks = Vec::<CircosTrackMark>::new();
    let mut label_budget = get_opt_usize(opts, "max_track_labels", 24);
    for (track_index, (name, kind, table)) in raw_tracks.iter().enumerate() {
        let track_chrom_column = required_circos_column(
            table,
            &["chrom", "chr", "chromosome"],
            &format!("track '{name}'"),
        )?;
        let point_column = table_column_alias(table, &["pos", "position"]);
        let mark_start_column = table_column_alias(table, &["start"])
            .or(point_column)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' needs start or position"),
                    None,
                )
            })?;
        let mark_end_column = table_column_alias(table, &["end"]);
        let value_column =
            table_column_alias(table, &["value", "score", "coverage", "depth", "log2ratio"]);
        let label_column = table_column_alias(table, &["label", "name", "gene"]);
        let mark_color_column = table_column_alias(table, &["color", "colour"]);
        let mut raw = Vec::<(
            usize,
            usize,
            f64,
            f64,
            f64,
            Option<String>,
            Option<String>,
            bool,
        )>::new();
        for (source_row, row) in table.rows.iter().enumerate() {
            let chromosome_name = row[track_chrom_column].as_str().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' chromosome must be a string"),
                    None,
                )
            })?;
            let chromosome_index = *lookup.get(chromosome_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "circos() track '{name}' references unknown chromosome '{chromosome_name}'"
                    ),
                    None,
                )
            })?;
            let chromosome = &chromosomes[chromosome_index];
            let original_start = finite_table_number(row, mark_start_column, "circos", "start")?;
            let original_end = match mark_end_column {
                Some(column) => finite_table_number(row, column, "circos", "end")?,
                None => original_start,
            };
            if original_start < 0.0 || original_end < original_start {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' requires 0 <= start <= end"),
                    None,
                ));
            }
            if original_end < chromosome.start || original_start > chromosome.end {
                continue;
            }
            let start = original_start.max(chromosome.start).min(chromosome.end);
            let end = original_end.max(chromosome.start).min(chromosome.end);
            let value = match value_column {
                Some(column) => finite_table_number(row, column, "circos", "value")?,
                None => 1.0,
            };
            let label = label_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            let color = mark_color_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            if color
                .as_deref()
                .is_some_and(|color| !valid_bio_hex_color(color))
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' colors must be #rrggbb"),
                    None,
                ));
            }
            raw.push((
                source_row,
                chromosome_index,
                start,
                end,
                value,
                label,
                color,
                start != original_start || end != original_end,
            ));
        }
        raw.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.3.total_cmp(&right.3))
                .then_with(|| left.0.cmp(&right.0))
        });
        let value_min = raw.iter().map(|row| row.4).fold(f64::INFINITY, f64::min);
        let value_max = raw
            .iter()
            .map(|row| row.4)
            .fold(f64::NEG_INFINITY, f64::max);
        let (value_min, value_max) = if raw.is_empty() {
            (0.0, 1.0)
        } else if (value_max - value_min).abs() < f64::EPSILON {
            (value_min.min(0.0), value_max.max(1.0))
        } else {
            (value_min, value_max)
        };
        let radial_outer = 0.86 - track_index as f64 * (track_band + track_gap);
        let radial_inner = radial_outer - track_band;
        track_meta.push(CircosTrackMeta {
            index: track_index,
            name: name.clone(),
            kind: kind.clone(),
            radial_inner,
            radial_outer,
            value_min,
            value_max,
        });
        for (
            point_index,
            (source_row, chromosome_index, start, end, value, label, explicit_color, clipped),
        ) in raw.into_iter().enumerate()
        {
            let chromosome = &chromosomes[chromosome_index];
            let normalized = ((value - value_min) / (value_max - value_min)).clamp(0.0, 1.0);
            let color = explicit_color.unwrap_or_else(|| match kind.as_str() {
                "cnv" => {
                    let magnitude = value_min.abs().max(value_max.abs()).max(f64::EPSILON);
                    publication_diverging_color((value / magnitude + 1.0) / 2.0)
                }
                "heatmap" | "bar" => publication_sequential_color(normalized),
                "line" => PALETTE[track_index % PALETTE.len()].to_string(),
                _ => PALETTE[track_index % PALETTE.len()].to_string(),
            });
            let radial_value = radial_inner + normalized * (radial_outer - radial_inner);
            let enough_arc = (circos_chromosome_angle(chromosome, end)
                - circos_chromosome_angle(chromosome, start))
            .abs()
                * label_radius
                >= label
                    .as_deref()
                    .map(|text| estimate_text_width(text, 8.0) + 3.0)
                    .unwrap_or(f64::INFINITY);
            let label_drawn =
                label.is_some() && label_budget > 0 && (kind == "point" || enough_arc);
            if label_drawn {
                label_budget -= 1;
            }
            track_marks.push(CircosTrackMark {
                track_index,
                point_index,
                source_row,
                chromosome_index,
                chromosome: chromosome.name.clone(),
                start,
                end,
                value,
                angle_start: circos_chromosome_angle(chromosome, start),
                angle_end: circos_chromosome_angle(chromosome, end),
                radial_inner,
                radial_outer: if kind == "bar" || kind == "line" || kind == "point" {
                    radial_value
                } else {
                    radial_outer
                },
                color,
                label,
                label_drawn,
                clipped,
            });
        }
    }

    let mut links = Vec::<CircosLink>::new();
    if let Some(link_value) = input.get("links") {
        let Value::Table(table) = link_value else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() links must be a Table",
                None,
            ));
        };
        let source_chrom_column = required_circos_column(
            table,
            &["source_chr", "source_chrom", "chrom1", "from_chr"],
            "links",
        )?;
        let target_chrom_column = required_circos_column(
            table,
            &["target_chr", "target_chrom", "chrom2", "to_chr"],
            "links",
        )?;
        let source_start_column = required_circos_column(
            table,
            &["source_start", "source_pos", "pos1", "from_pos"],
            "links",
        )?;
        let target_start_column = required_circos_column(
            table,
            &["target_start", "target_pos", "pos2", "to_pos"],
            "links",
        )?;
        let source_end_column = table_column_alias(table, &["source_end", "end1"]);
        let target_end_column = table_column_alias(table, &["target_end", "end2"]);
        let weight_column = table_column_alias(table, &["weight", "value", "count", "score"]);
        let link_color_column = table_column_alias(table, &["color", "colour"]);
        let link_label_column = table_column_alias(table, &["label", "name"]);
        let mut raw_links = Vec::<(
            usize,
            usize,
            String,
            f64,
            f64,
            usize,
            String,
            f64,
            f64,
            f64,
            Option<String>,
            Option<String>,
        )>::new();
        for (source_row, row) in table.rows.iter().enumerate() {
            let source_name = row[source_chrom_column].as_str().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link chromosome must be a string",
                    None,
                )
            })?;
            let target_name = row[target_chrom_column].as_str().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link chromosome must be a string",
                    None,
                )
            })?;
            let source_index = *lookup.get(source_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() link references unknown chromosome '{source_name}'"),
                    None,
                )
            })?;
            let target_index = *lookup.get(target_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() link references unknown chromosome '{target_name}'"),
                    None,
                )
            })?;
            let source_start =
                finite_table_number(row, source_start_column, "circos", "source_start")?;
            let target_start =
                finite_table_number(row, target_start_column, "circos", "target_start")?;
            let source_end = match source_end_column {
                Some(column) => finite_table_number(row, column, "circos", "source_end")?,
                None => source_start,
            };
            let target_end = match target_end_column {
                Some(column) => finite_table_number(row, column, "circos", "target_end")?,
                None => target_start,
            };
            let source_chromosome = &chromosomes[source_index];
            let target_chromosome = &chromosomes[target_index];
            if source_start < source_chromosome.start
                || source_end > source_chromosome.end
                || source_end < source_start
                || target_start < target_chromosome.start
                || target_end > target_chromosome.end
                || target_end < target_start
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link endpoints must lie inside their chromosome intervals",
                    None,
                ));
            }
            let weight = match weight_column {
                Some(column) => finite_table_number(row, column, "circos", "weight")?,
                None => 1.0,
            };
            if weight < 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link weights must be non-negative",
                    None,
                ));
            }
            let color = link_color_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            if color
                .as_deref()
                .is_some_and(|color| !valid_bio_hex_color(color))
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link colors must be #rrggbb",
                    None,
                ));
            }
            let label = link_label_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            raw_links.push((
                source_row,
                source_index,
                source_name.to_string(),
                source_start,
                source_end,
                target_index,
                target_name.to_string(),
                target_start,
                target_end,
                weight,
                color,
                label,
            ));
        }
        raw_links.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.3.total_cmp(&right.3))
                .then_with(|| left.5.cmp(&right.5))
                .then_with(|| left.7.total_cmp(&right.7))
                .then_with(|| left.0.cmp(&right.0))
        });
        let max_weight = raw_links
            .iter()
            .map(|row| row.9)
            .fold(0.0f64, f64::max)
            .max(1.0);
        let mut ranked = (0..raw_links.len()).collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            raw_links[*right]
                .9
                .total_cmp(&raw_links[*left].9)
                .then_with(|| raw_links[*left].0.cmp(&raw_links[*right].0))
        });
        let labelled = ranked
            .into_iter()
            .take(get_opt_usize(opts, "max_link_labels", 12))
            .collect::<HashSet<_>>();
        for (link_index, row) in raw_links.into_iter().enumerate() {
            let source_chromosome = &chromosomes[row.1];
            let target_chromosome = &chromosomes[row.5];
            let strength = (row.9 / max_weight).sqrt();
            links.push(CircosLink {
                link_index,
                source_row: row.0,
                source_chromosome_index: row.1,
                source_chromosome: row.2,
                source_start: row.3,
                source_end: row.4,
                target_chromosome_index: row.5,
                target_chromosome: row.6,
                target_start: row.7,
                target_end: row.8,
                source_angle_start: circos_chromosome_angle(source_chromosome, row.3),
                source_angle_end: circos_chromosome_angle(source_chromosome, row.4),
                target_angle_start: circos_chromosome_angle(target_chromosome, row.7),
                target_angle_end: circos_chromosome_angle(target_chromosome, row.8),
                weight: row.9,
                stroke_width: 0.75 + (strength * 3.0 * 4.0).round() / 4.0,
                color: row.10.unwrap_or_else(|| chromosomes[row.1].color.clone()),
                label_drawn: row.11.is_some() && labelled.contains(&link_index),
                label: row.11,
            });
        }
    }

    let segment_table = Value::Table(Table::new(
        vec![
            "chromosome_index".into(),
            "source_row".into(),
            "chromosome".into(),
            "start".into(),
            "end".into(),
            "size".into(),
            "angle_start".into(),
            "angle_end".into(),
            "color".into(),
            "label_drawn".into(),
        ],
        chromosomes
            .iter()
            .enumerate()
            .map(|(index, chromosome)| {
                vec![
                    Value::Int(index as i64),
                    Value::Int(chromosome.source_row as i64),
                    Value::Str(chromosome.name.clone().into()),
                    Value::Float(chromosome.start),
                    Value::Float(chromosome.end),
                    Value::Float(chromosome.end - chromosome.start),
                    Value::Float(chromosome.angle_start),
                    Value::Float(chromosome.angle_end),
                    Value::Str(chromosome.color.clone().into()),
                    Value::Bool(chromosome.label_drawn),
                ]
            })
            .collect(),
    ));
    let track_metadata = Value::Table(Table::new(
        vec![
            "track_index".into(),
            "name".into(),
            "type".into(),
            "radial_inner".into(),
            "radial_outer".into(),
            "value_min".into(),
            "value_max".into(),
        ],
        track_meta
            .iter()
            .map(|track| {
                vec![
                    Value::Int(track.index as i64),
                    Value::Str(track.name.clone().into()),
                    Value::Str(track.kind.clone().into()),
                    Value::Float(track.radial_inner),
                    Value::Float(track.radial_outer),
                    Value::Float(track.value_min),
                    Value::Float(track.value_max),
                ]
            })
            .collect(),
    ));
    let track_table = Value::Table(Table::new(
        vec![
            "track_index".into(),
            "point_index".into(),
            "source_row".into(),
            "chromosome_index".into(),
            "chromosome".into(),
            "start".into(),
            "end".into(),
            "value".into(),
            "angle_start".into(),
            "angle_end".into(),
            "radial_inner".into(),
            "radial_outer".into(),
            "color".into(),
            "label".into(),
            "label_drawn".into(),
            "clipped".into(),
        ],
        track_marks
            .iter()
            .map(|mark| {
                vec![
                    Value::Int(mark.track_index as i64),
                    Value::Int(mark.point_index as i64),
                    Value::Int(mark.source_row as i64),
                    Value::Int(mark.chromosome_index as i64),
                    Value::Str(mark.chromosome.clone().into()),
                    Value::Float(mark.start),
                    Value::Float(mark.end),
                    Value::Float(mark.value),
                    Value::Float(mark.angle_start),
                    Value::Float(mark.angle_end),
                    Value::Float(mark.radial_inner),
                    Value::Float(mark.radial_outer),
                    Value::Str(mark.color.clone().into()),
                    mark.label
                        .as_ref()
                        .map(|label| Value::Str(label.clone().into()))
                        .unwrap_or(Value::Nil),
                    Value::Bool(mark.label_drawn),
                    Value::Bool(mark.clipped),
                ]
            })
            .collect(),
    ));
    let link_table = Value::Table(Table::new(
        vec![
            "link_index".into(),
            "source_row".into(),
            "source_chromosome_index".into(),
            "source_chromosome".into(),
            "source_start".into(),
            "source_end".into(),
            "target_chromosome_index".into(),
            "target_chromosome".into(),
            "target_start".into(),
            "target_end".into(),
            "source_angle_start".into(),
            "source_angle_end".into(),
            "target_angle_start".into(),
            "target_angle_end".into(),
            "weight".into(),
            "stroke_width".into(),
            "color".into(),
            "label".into(),
            "label_drawn".into(),
        ],
        links
            .iter()
            .map(|link| {
                vec![
                    Value::Int(link.link_index as i64),
                    Value::Int(link.source_row as i64),
                    Value::Int(link.source_chromosome_index as i64),
                    Value::Str(link.source_chromosome.clone().into()),
                    Value::Float(link.source_start),
                    Value::Float(link.source_end),
                    Value::Int(link.target_chromosome_index as i64),
                    Value::Str(link.target_chromosome.clone().into()),
                    Value::Float(link.target_start),
                    Value::Float(link.target_end),
                    Value::Float(link.source_angle_start),
                    Value::Float(link.source_angle_end),
                    Value::Float(link.target_angle_start),
                    Value::Float(link.target_angle_end),
                    Value::Float(link.weight),
                    Value::Float(link.stroke_width),
                    Value::Str(link.color.clone().into()),
                    link.label
                        .as_ref()
                        .map(|label| Value::Str(label.clone().into()))
                        .unwrap_or(Value::Nil),
                    Value::Bool(link.label_drawn),
                ]
            })
            .collect(),
    ));
    let title = get_opt_str(opts, "title", "Circular genome");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    Ok(Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str("biolang.plot.spec/v1".into())),
            ("kind".into(), Value::Str("circular-genome".into())),
            ("plot".into(), Value::Str("circos".into())),
            ("title".into(), Value::Str(title.into())),
            ("subtitle".into(), Value::Str(subtitle.into())),
            ("caption".into(), Value::Str(caption.into())),
            ("segments".into(), segment_table),
            ("track_metadata".into(), track_metadata),
            ("tracks".into(), track_table),
            ("links".into(), link_table),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("width".into(), Value::Float(width)),
                        ("height".into(), Value::Float(height)),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        ("gap_radians".into(), Value::Float(gap)),
                        (
                            "start_angle".into(),
                            Value::Float(start_degrees.to_radians()),
                        ),
                        ("available_angle".into(), Value::Float(available)),
                        ("outer_radius".into(), Value::Float(1.0)),
                        ("chromosome_inner_radius".into(), Value::Float(0.89)),
                        (
                            "link_radius".into(),
                            Value::Float(
                                track_meta
                                    .last()
                                    .map(|track| track.radial_inner - 0.035)
                                    .unwrap_or(0.82),
                            ),
                        ),
                        ("dense_links".into(), Value::Bool(links.len() > 200)),
                        ("dense_tracks".into(), Value::Bool(track_marks.len() > 500)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "chromosome_order".into(),
                            Value::List(
                                chromosomes
                                    .iter()
                                    .map(|chromosome| Value::Str(chromosome.name.clone().into()))
                                    .collect::<Vec<_>>()
                                    .into(),
                            ),
                        ),
                        (
                            "segment_rows".into(),
                            Value::Int(segments.rows.len() as i64),
                        ),
                        ("track_count".into(), Value::Int(track_meta.len() as i64)),
                        ("track_marks".into(), Value::Int(track_marks.len() as i64)),
                        ("link_count".into(), Value::Int(links.len() as i64)),
                        (
                            "geometry".into(),
                            Value::Str("length_weighted_gapped_circle".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

fn polar_point(cx: f64, cy: f64, radius: f64, angle: f64) -> (f64, f64) {
    (cx + radius * angle.cos(), cy + radius * angle.sin())
}

fn annular_path(cx: f64, cy: f64, inner: f64, outer: f64, start: f64, end: f64) -> String {
    let (x1, y1) = polar_point(cx, cy, outer, start);
    let (x2, y2) = polar_point(cx, cy, outer, end);
    let (x3, y3) = polar_point(cx, cy, inner, end);
    let (x4, y4) = polar_point(cx, cy, inner, start);
    let large = usize::from((end - start).abs() > std::f64::consts::PI);
    format!(
        "M{x1:.2},{y1:.2}A{outer:.2},{outer:.2} 0 {large} 1 {x2:.2},{y2:.2}L{x3:.2},{y3:.2}A{inner:.2},{inner:.2} 0 {large} 0 {x4:.2},{y4:.2}Z"
    )
}

fn render_circos_svg(
    specification: &HashMap<String, Value>,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let segments = match specification.get("segments") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let track_metadata = match specification.get("track_metadata") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let tracks = match specification.get("tracks") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let links = match specification.get("links") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let options = frozen_spec_options(specification, opts, "circos")?;
    let width = get_opt_f64(&options, "width", 700.0);
    let height = get_opt_f64(&options, "height", 700.0);
    let title = specification
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Circular genome");
    let subtitle = specification
        .get("subtitle")
        .and_then(Value::as_str)
        .unwrap_or("");
    let caption = specification
        .get("caption")
        .and_then(Value::as_str)
        .unwrap_or("");
    let theme = plot_theme(&options);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.top = if title.is_empty() {
        20.0
    } else if subtitle.is_empty() {
        48.0
    } else {
        66.0
    };
    canvas.margin.bottom = if caption.is_empty() { 18.0 } else { 34.0 };
    canvas.margin.left = 20.0;
    canvas.margin.right = if track_metadata.rows.is_empty() {
        20.0
    } else {
        118.0_f64.min(width * 0.24)
    };
    let available_width = width - canvas.margin.left - canvas.margin.right;
    let available_height = height - canvas.margin.top - canvas.margin.bottom;
    let base_radius = available_width.min(available_height) * 0.46;
    let cx = canvas.margin.left + available_width / 2.0;
    let cy = canvas.margin.top + available_height / 2.0;
    let segment_columns = |name: &str| segments.col_index(name).unwrap();
    for row in &segments.rows {
        let start = row[segment_columns("angle_start")].as_float().unwrap();
        let end = row[segment_columns("angle_end")].as_float().unwrap();
        let color = row[segment_columns("color")].as_str().unwrap();
        let path = annular_path(cx, cy, base_radius * 0.89, base_radius, start, end);
        canvas.elements.push(format!(
            r##"<path d="{path}" fill="{color}" stroke="#ffffff" stroke-width="0.8" data-circos-layer="ideogram" />"##
        ));
        if row[segment_columns("label_drawn")].is_truthy() {
            let middle = (start + end) / 2.0;
            let (x, y) = polar_point(cx, cy, base_radius + 14.0, middle);
            canvas.add_text(
                x,
                y + 3.0,
                row[segment_columns("chromosome")].as_str().unwrap(),
                "middle",
                theme.legend_size,
            );
        }
    }
    let meta_type = track_metadata.col_index("type").unwrap();
    let meta_name = track_metadata.col_index("name").unwrap();
    let track_index_column = tracks.col_index("track_index").unwrap();
    let track_angle_start = tracks.col_index("angle_start").unwrap();
    let track_angle_end = tracks.col_index("angle_end").unwrap();
    let track_radial_inner = tracks.col_index("radial_inner").unwrap();
    let track_radial_outer = tracks.col_index("radial_outer").unwrap();
    let track_color = tracks.col_index("color").unwrap();
    let track_label = tracks.col_index("label").unwrap();
    let track_label_drawn = tracks.col_index("label_drawn").unwrap();
    let dense_tracks = get_opt_f64(&options, "dense_tracks", 0.0) > 0.0
        || matches!(options.get("dense_tracks"), Some(Value::Bool(true)));
    let mut dense_paths = BTreeMap::<(usize, String), String>::new();
    let mut line_paths = BTreeMap::<(usize, usize, String), String>::new();
    for row in &tracks.rows {
        let index = row[track_index_column].as_float().unwrap() as usize;
        let kind = track_metadata.rows[index][meta_type].as_str().unwrap();
        let start = row[track_angle_start].as_float().unwrap();
        let end = row[track_angle_end].as_float().unwrap();
        let inner = row[track_radial_inner].as_float().unwrap() * base_radius;
        let outer = row[track_radial_outer].as_float().unwrap() * base_radius;
        let color = row[track_color].as_str().unwrap();
        match kind {
            "point" => {
                let (x, y) = polar_point(cx, cy, outer, start);
                if dense_tracks {
                    dense_paths
                        .entry((index, color.to_string()))
                        .or_default()
                        .push_str(&format!("M{:.2},{:.2}h0.01", x, y));
                } else {
                    canvas.add_circle(x, y, 2.4, color);
                }
            }
            "line" => {
                let middle = (start + end) / 2.0;
                let (x, y) = polar_point(cx, cy, outer, middle);
                if !dense_tracks {
                    canvas.add_circle(x, y, 1.8, color);
                }
                let chromosome = row[tracks.col_index("chromosome_index").unwrap()]
                    .as_float()
                    .unwrap() as usize;
                let path = line_paths
                    .entry((index, chromosome, color.to_string()))
                    .or_default();
                path.push_str(if path.is_empty() { "M" } else { "L" });
                path.push_str(&format!("{x:.2},{y:.2}"));
            }
            _ => {
                let visible_end = if (end - start).abs() < 1e-8 {
                    start + 0.002
                } else {
                    end
                };
                let path = annular_path(cx, cy, inner, outer.max(inner + 1.0), start, visible_end);
                if dense_tracks {
                    dense_paths
                        .entry((index, color.to_string()))
                        .or_default()
                        .push_str(&path);
                } else {
                    canvas.elements.push(format!(
                        r#"<path d="{path}" fill="{color}" opacity="0.88" data-circos-layer="track" />"#
                    ));
                }
            }
        }
        if row[track_label_drawn].is_truthy() {
            if let Some(label) = row[track_label].as_str() {
                let (x, y) = polar_point(cx, cy, outer + 4.0, (start + end) / 2.0);
                canvas.add_text(x, y, label, "middle", 7.5);
            }
        }
    }
    for ((_, color), path) in dense_paths {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="{color}" stroke="{color}" stroke-width="3" opacity="0.82" data-circos-layer="dense-track" />"#
        ));
    }
    for ((_, _, color), path) in line_paths {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="1.5" opacity="0.9" data-circos-layer="line-track" />"#
        ));
    }
    let link_radius = get_opt_f64(&options, "link_radius", 0.82) * base_radius;
    let link_columns = |name: &str| links.col_index(name).unwrap();
    let dense_links = matches!(options.get("dense_links"), Some(Value::Bool(true)));
    let mut link_paths = BTreeMap::<(String, i64), String>::new();
    for row in &links.rows {
        let source_start = row[link_columns("source_angle_start")].as_float().unwrap();
        let source_end = row[link_columns("source_angle_end")].as_float().unwrap();
        let target_start = row[link_columns("target_angle_start")].as_float().unwrap();
        let target_end = row[link_columns("target_angle_end")].as_float().unwrap();
        let source = (source_start + source_end) / 2.0;
        let target = (target_start + target_end) / 2.0;
        let (sx, sy) = polar_point(cx, cy, link_radius, source);
        let (tx, ty) = polar_point(cx, cy, link_radius, target);
        let color = row[link_columns("color")].as_str().unwrap();
        let stroke = row[link_columns("stroke_width")].as_float().unwrap();
        let path = format!("M{sx:.2},{sy:.2}Q{cx:.2},{cy:.2} {tx:.2},{ty:.2}");
        if dense_links {
            link_paths
                .entry((color.to_string(), (stroke * 4.0).round() as i64))
                .or_default()
                .push_str(&path);
        } else if (source_end - source_start).abs() > 1e-8
            || (target_end - target_start).abs() > 1e-8
        {
            let (s2x, s2y) = polar_point(cx, cy, link_radius, source_end);
            let (t2x, t2y) = polar_point(cx, cy, link_radius, target_end);
            let ribbon = format!(
                "M{sx:.2},{sy:.2}Q{cx:.2},{cy:.2} {tx:.2},{ty:.2}L{t2x:.2},{t2y:.2}Q{cx:.2},{cy:.2} {s2x:.2},{s2y:.2}Z"
            );
            canvas.elements.push(format!(
                r#"<path d="{ribbon}" fill="{color}" opacity="0.25" stroke="{color}" stroke-width="0.5" data-circos-layer="ribbon" />"#
            ));
        } else {
            canvas.elements.push(format!(
                r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{stroke:.2}" opacity="0.38" data-circos-layer="link" />"#
            ));
        }
    }
    for ((color, bucket), path) in link_paths {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{:.2}" opacity="0.25" data-circos-layer="dense-links" />"#,
            bucket as f64 / 4.0
        ));
    }
    if !track_metadata.rows.is_empty() {
        let legend_x = width - canvas.margin.right + 10.0;
        let mut legend_y = canvas.margin.top + 6.0;
        for (index, row) in track_metadata.rows.iter().enumerate() {
            let color = PALETTE[index % PALETTE.len()];
            canvas.add_rect(legend_x, legend_y - 8.0, 9.0, 9.0, color);
            canvas.add_text(
                legend_x + 14.0,
                legend_y,
                row[meta_name].as_str().unwrap(),
                "start",
                theme.legend_size,
            );
            legend_y += 16.0;
        }
    }
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Circular genome plot with {} chromosomes, {} tracks, {} track marks and {} genomic links.",
        segments.rows.len(),
        track_metadata.rows.len(),
        tracks.rows.len(),
        links.rows.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_circos_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == "biolang.plot.spec/v1")
            && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "circos"))
}

pub(crate) fn render_circos_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let Value::Record(map) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a circos PlotSpec",
            None,
        ));
    };
    if !is_circos_plot_spec(value) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a biolang.plot.spec/v1 circos Record",
            None,
        ));
    }
    let table = |name: &str| -> Result<&Table> {
        match map.get(name) {
            Some(Value::Table(table)) => Ok(table),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() circos field '{name}' must be a Table"),
                None,
            )),
        }
    };
    let segments = table("segments")?;
    let track_metadata = table("track_metadata")?;
    let tracks = table("tracks")?;
    let links = table("links")?;
    let require_columns = |table: &Table, family: &str, columns: &[&str]| -> Result<()> {
        for column in columns {
            if table.col_index(column).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() circos {family} is missing '{column}'"),
                    None,
                ));
            }
        }
        Ok(())
    };
    require_columns(
        segments,
        "segments",
        &[
            "chromosome_index",
            "source_row",
            "chromosome",
            "start",
            "end",
            "size",
            "angle_start",
            "angle_end",
            "color",
            "label_drawn",
        ],
    )?;
    require_columns(
        track_metadata,
        "track metadata",
        &[
            "track_index",
            "name",
            "type",
            "radial_inner",
            "radial_outer",
            "value_min",
            "value_max",
        ],
    )?;
    require_columns(
        tracks,
        "tracks",
        &[
            "track_index",
            "point_index",
            "source_row",
            "chromosome_index",
            "chromosome",
            "start",
            "end",
            "value",
            "angle_start",
            "angle_end",
            "radial_inner",
            "radial_outer",
            "color",
            "label",
            "label_drawn",
            "clipped",
        ],
    )?;
    require_columns(
        links,
        "links",
        &[
            "link_index",
            "source_row",
            "source_chromosome_index",
            "source_chromosome",
            "source_start",
            "source_end",
            "target_chromosome_index",
            "target_chromosome",
            "target_start",
            "target_end",
            "source_angle_start",
            "source_angle_end",
            "target_angle_start",
            "target_angle_end",
            "weight",
            "stroke_width",
            "color",
            "label",
            "label_drawn",
        ],
    )?;
    let options = match map.get("options") {
        Some(Value::Record(options)) => options,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos options must be a Record",
                None,
            ))
        }
    };
    let gap = options
        .get("gap_radians")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos gap_radians is invalid",
                None,
            )
        })?;
    let start_angle = options
        .get("start_angle")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos start_angle is invalid",
                None,
            )
        })?;
    let mut chromosome_geometry = Vec::<CircosChromosome>::new();
    let mut expected_angle = start_angle;
    let mut total_size = 0.0;
    for row in &segments.rows {
        let size = row[segments.col_index("size").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        if !size.is_finite() || size <= 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos chromosome sizes are invalid",
                None,
            ));
        }
        total_size += size;
    }
    let available = std::f64::consts::TAU - gap * segments.rows.len() as f64;
    for (index, row) in segments.rows.iter().enumerate() {
        let frozen_index = frozen_nonnegative_integer(
            &row[segments.col_index("chromosome_index").unwrap()],
            "circos",
            "chromosome_index",
        )?;
        let source_row = frozen_nonnegative_integer(
            &row[segments.col_index("source_row").unwrap()],
            "circos",
            "source_row",
        )?;
        let name = row[segments.col_index("chromosome").unwrap()]
            .as_str()
            .unwrap_or("");
        let start = row[segments.col_index("start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let end = row[segments.col_index("end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let size = row[segments.col_index("size").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_start = row[segments.col_index("angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_end = row[segments.col_index("angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let expected_end = expected_angle + size / total_size * available;
        let color = row[segments.col_index("color").unwrap()]
            .as_str()
            .unwrap_or("");
        if frozen_index != index
            || name.is_empty()
            || !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || (end - start - size).abs() > 1e-8
            || (angle_start - expected_angle).abs() > 1e-10
            || (angle_end - expected_end).abs() > 1e-10
            || !valid_bio_hex_color(color)
            || !matches!(
                row[segments.col_index("label_drawn").unwrap()],
                Value::Bool(_)
            )
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos chromosome geometry is inconsistent",
                None,
            ));
        }
        chromosome_geometry.push(CircosChromosome {
            source_row,
            name: name.to_string(),
            start,
            end,
            angle_start,
            angle_end,
            color: color.to_string(),
            label_drawn: row[segments.col_index("label_drawn").unwrap()].is_truthy(),
        });
        expected_angle = expected_end + gap;
    }
    let mut frozen_track_metadata = Vec::<(String, f64, f64, f64, f64)>::new();
    for (index, row) in track_metadata.rows.iter().enumerate() {
        let frozen_index = frozen_nonnegative_integer(
            &row[track_metadata.col_index("track_index").unwrap()],
            "circos",
            "track_index",
        )?;
        let kind = row[track_metadata.col_index("type").unwrap()]
            .as_str()
            .unwrap_or("");
        let inner = row[track_metadata.col_index("radial_inner").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let outer = row[track_metadata.col_index("radial_outer").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let value_min = row[track_metadata.col_index("value_min").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let value_max = row[track_metadata.col_index("value_max").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        if frozen_index != index
            || !matches!(normalized_track_kind(kind), Ok(ref normalized) if normalized == kind)
            || !inner.is_finite()
            || !outer.is_finite()
            || !value_min.is_finite()
            || !value_max.is_finite()
            || inner < 0.0
            || outer <= inner
            || outer > 0.89
            || value_max <= value_min
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos track metadata is inconsistent",
                None,
            ));
        }
        frozen_track_metadata.push((kind.to_string(), inner, outer, value_min, value_max));
    }
    let mut point_counts = vec![0usize; track_metadata.rows.len()];
    for row in &tracks.rows {
        let track_index = frozen_nonnegative_integer(
            &row[tracks.col_index("track_index").unwrap()],
            "circos",
            "track_index",
        )?;
        let point_index = frozen_nonnegative_integer(
            &row[tracks.col_index("point_index").unwrap()],
            "circos",
            "point_index",
        )?;
        let chromosome_index = frozen_nonnegative_integer(
            &row[tracks.col_index("chromosome_index").unwrap()],
            "circos",
            "chromosome_index",
        )?;
        if track_index >= track_metadata.rows.len()
            || chromosome_index >= chromosome_geometry.len()
            || point_index != point_counts[track_index]
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos track indexes are inconsistent",
                None,
            ));
        }
        point_counts[track_index] += 1;
        let chromosome = &chromosome_geometry[chromosome_index];
        let start = row[tracks.col_index("start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let end = row[tracks.col_index("end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_start = row[tracks.col_index("angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_end = row[tracks.col_index("angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let value = row[tracks.col_index("value").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let radial_inner = row[tracks.col_index("radial_inner").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let radial_outer = row[tracks.col_index("radial_outer").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let (kind, metadata_inner, metadata_outer, value_min, value_max) =
            &frozen_track_metadata[track_index];
        let normalized = ((value - value_min) / (value_max - value_min)).clamp(0.0, 1.0);
        let expected_outer = if matches!(kind.as_str(), "bar" | "line" | "point") {
            metadata_inner + normalized * (metadata_outer - metadata_inner)
        } else {
            *metadata_outer
        };
        let color = row[tracks.col_index("color").unwrap()]
            .as_str()
            .unwrap_or("");
        if row[tracks.col_index("chromosome").unwrap()].as_str() != Some(chromosome.name.as_str())
            || start < chromosome.start
            || end > chromosome.end
            || end < start
            || !value.is_finite()
            || (angle_start - circos_chromosome_angle(chromosome, start)).abs() > 1e-10
            || (angle_end - circos_chromosome_angle(chromosome, end)).abs() > 1e-10
            || (radial_inner - metadata_inner).abs() > 1e-10
            || (radial_outer - expected_outer).abs() > 1e-10
            || !valid_bio_hex_color(color)
            || !matches!(
                row[tracks.col_index("label_drawn").unwrap()],
                Value::Bool(_)
            )
            || !matches!(row[tracks.col_index("clipped").unwrap()], Value::Bool(_))
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos track geometry is inconsistent",
                None,
            ));
        }
    }
    let max_link_weight = links
        .rows
        .iter()
        .filter_map(|row| row[links.col_index("weight").unwrap()].as_float())
        .fold(0.0f64, f64::max)
        .max(1.0);
    for (index, row) in links.rows.iter().enumerate() {
        let frozen_index = frozen_nonnegative_integer(
            &row[links.col_index("link_index").unwrap()],
            "circos",
            "link_index",
        )?;
        let source_index = frozen_nonnegative_integer(
            &row[links.col_index("source_chromosome_index").unwrap()],
            "circos",
            "source_chromosome_index",
        )?;
        let target_index = frozen_nonnegative_integer(
            &row[links.col_index("target_chromosome_index").unwrap()],
            "circos",
            "target_chromosome_index",
        )?;
        if frozen_index != index
            || source_index >= chromosome_geometry.len()
            || target_index >= chromosome_geometry.len()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos link indexes are inconsistent",
                None,
            ));
        }
        let source = &chromosome_geometry[source_index];
        let target = &chromosome_geometry[target_index];
        let source_start = row[links.col_index("source_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let source_end = row[links.col_index("source_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let target_start = row[links.col_index("target_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let target_end = row[links.col_index("target_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_source_angle = row[links.col_index("source_angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_source_angle_end = row[links.col_index("source_angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_target_angle = row[links.col_index("target_angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_target_angle_end = row[links.col_index("target_angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let weight = row[links.col_index("weight").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let stroke_width = row[links.col_index("stroke_width").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let expected_stroke_width =
            0.75 + ((weight / max_link_weight).sqrt() * 3.0 * 4.0).round() / 4.0;
        if source_start < source.start
            || source_end > source.end
            || source_end < source_start
            || target_start < target.start
            || target_end > target.end
            || target_end < target_start
            || (frozen_source_angle - circos_chromosome_angle(source, source_start)).abs() > 1e-10
            || (frozen_source_angle_end - circos_chromosome_angle(source, source_end)).abs() > 1e-10
            || (frozen_target_angle - circos_chromosome_angle(target, target_start)).abs() > 1e-10
            || (frozen_target_angle_end - circos_chromosome_angle(target, target_end)).abs() > 1e-10
            || !weight.is_finite()
            || weight < 0.0
            || !stroke_width.is_finite()
            || (stroke_width - expected_stroke_width).abs() > 1e-10
            || !row[links.col_index("color").unwrap()]
                .as_str()
                .is_some_and(valid_bio_hex_color)
            || !matches!(row[links.col_index("label_drawn").unwrap()], Value::Bool(_))
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos link geometry is inconsistent",
                None,
            ));
        }
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_circos_svg(map, render_options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Circular genome");
    finish_frozen_bio_plot(value, render_options, title, "circos", svg)
}

fn builtin_circos(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    // Keep the historical `circos(segments_table, options?)` form working.
    // The structured Record is the canonical multi-track input, but a bare
    // segment table remains a useful and documented shorthand for the outer
    // chromosome ring.
    let input = match &args[0] {
        Value::Table(_) => {
            Value::Record(HashMap::from([("segments".into(), args[0].clone())]).into())
        }
        value => value.clone(),
    };
    let specification = circos_spec_value(&input, &opts)?;
    render_circos_plot_spec_value(&specification, &opts)
}

#[cfg(any())]
fn builtin_circos_legacy(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let (segments, links) = match &args[0] {
        Value::Record(map) => {
            let seg = map.get("segments").cloned().unwrap_or(Value::Nil);
            let lnk = map.get("links").cloned().unwrap_or(Value::Nil);
            (seg, lnk)
        }
        Value::Table(_) => (args[0].clone(), Value::Nil),
        _ => {
            return Err(BioLangError::type_error(
                "circos() requires Record with 'segments' and 'links' Tables",
                None,
            ))
        }
    };

    let seg_table = match &segments {
        Value::Table(t) => t,
        _ => {
            return Err(BioLangError::type_error(
                "circos() 'segments' must be a Table",
                None,
            ))
        }
    };
    let chroms = extract_str_col(seg_table, "chrom")?;
    let ends = extract_table_col(seg_table, "end")?;

    // Compute chrom sizes
    let mut chrom_sizes: Vec<(String, f64)> = Vec::new();
    let mut seen: HashMap<String, f64> = HashMap::new();
    for (i, c) in chroms.iter().enumerate() {
        let e = seen.entry(c.clone()).or_insert(0.0);
        if ends[i] > *e {
            *e = ends[i];
        }
    }
    let mut chrom_order: Vec<String> = Vec::new();
    for c in &chroms {
        if !chrom_order.contains(c) {
            chrom_order.push(c.clone());
        }
    }
    for c in &chrom_order {
        chrom_sizes.push((c.clone(), *seen.get(c).unwrap_or(&1.0)));
    }
    let total_size: f64 = chrom_sizes.iter().map(|(_, s)| s).sum();

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 500.0);
        let h = get_opt_f64(&opts, "height", 500.0);
        let mut c = SvgCanvas::new(w, h);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let r = w.min(h) * 0.38;
        let r_inner = r * 0.85;
        let mut angle = 0.0f64;
        let mut chrom_angles: HashMap<String, (f64, f64)> = HashMap::new();
        for (name, size) in &chrom_sizes {
            let sweep = (*size / total_size) * 2.0 * std::f64::consts::PI;
            let a1 = angle;
            let a2 = angle + sweep;
            chrom_angles.insert(name.clone(), (a1, a2));
            // Draw arc segment
            let ci = chrom_order.iter().position(|c| c == name).unwrap_or(0);
            let color = PALETTE[ci % PALETTE.len()];
            let (x1, y1) = (cx + r * a1.cos(), cy + r * a1.sin());
            let (x2, y2) = (cx + r * a2.cos(), cy + r * a2.sin());
            let (x3, y3) = (cx + r_inner * a2.cos(), cy + r_inner * a2.sin());
            let (x4, y4) = (cx + r_inner * a1.cos(), cy + r_inner * a1.sin());
            let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
            c.elements.push(format!(
                r#"<path d="M {x1:.1},{y1:.1} A {r:.1},{r:.1} 0 {large} 1 {x2:.1},{y2:.1} L {x3:.1},{y3:.1} A {ri:.1},{ri:.1} 0 {large} 0 {x4:.1},{y4:.1} Z" fill="{color}" opacity="0.7" />"#,
                ri = r_inner
            ));
            let mid_a = (a1 + a2) / 2.0;
            let lx = cx + (r + 15.0) * mid_a.cos();
            let ly = cy + (r + 15.0) * mid_a.sin();
            c.add_text(lx, ly, name, "middle", 9.0);
            angle = a2 + 0.02; // small gap
        }
        // Draw links as bezier curves
        if let Value::Table(link_table) = &links {
            if let (Ok(c1), Ok(s1), Ok(c2), Ok(s2)) = (
                extract_str_col(link_table, "chrom1"),
                extract_table_col(link_table, "pos1"),
                extract_str_col(link_table, "chrom2"),
                extract_table_col(link_table, "pos2"),
            ) {
                for i in 0..c1.len() {
                    if let (Some(&(a1s, a1e)), Some(&(a2s, a2e))) =
                        (chrom_angles.get(&c1[i]), chrom_angles.get(&c2[i]))
                    {
                        let sz1 = seen.get(&c1[i]).copied().unwrap_or(1.0);
                        let sz2 = seen.get(&c2[i]).copied().unwrap_or(1.0);
                        let ang1 = a1s + (s1[i] / sz1) * (a1e - a1s);
                        let ang2 = a2s + (s2[i] / sz2) * (a2e - a2s);
                        let (px1, py1) = (cx + r_inner * ang1.cos(), cy + r_inner * ang1.sin());
                        let (px2, py2) = (cx + r_inner * ang2.cos(), cy + r_inner * ang2.sin());
                        c.elements.push(format!(
                            r#"<path d="M {px1:.1},{py1:.1} Q {cx:.1},{cy:.1} {px2:.1},{py2:.1}" fill="none" stroke="{}" opacity="0.4" stroke-width="1.5" />"#,
                            PALETTE[i % PALETTE.len()]
                        ));
                    }
                }
            }
        }
        c.draw_title("Circos");
        return Ok(Value::Str(c.render()));
    }

    // ASCII: linear summary
    let bar_w = get_opt_usize(&opts, "width", 50);
    let max_name = chrom_sizes.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let mut out = String::from("  Circos (linear view)\n");
    for (name, size) in &chrom_sizes {
        let len = (size / total_size * bar_w as f64).round() as usize;
        let bar: String = "█".repeat(len.max(1));
        out.push_str(&format!("  {:>w$}  {bar}\n", name, w = max_name));
    }
    if let Value::Table(link_table) = &links {
        if let (Ok(c1), Ok(s1), Ok(c2), Ok(s2)) = (
            extract_str_col(link_table, "chrom1"),
            extract_table_col(link_table, "pos1"),
            extract_str_col(link_table, "chrom2"),
            extract_table_col(link_table, "pos2"),
        ) {
            out.push_str(&format!("  Links ({}):\n", c1.len()));
            for i in 0..c1.len().min(10) {
                out.push_str(&format!(
                    "    {}:{:.0} → {}:{:.0}\n",
                    c1[i], s1[i], c2[i], s2[i]
                ));
            }
        }
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 20. hic_map ─────────────────────────────────────────────────

fn builtin_hic_map(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let heat_chars: Vec<char> = get_opt_str(&opts, "chars", " ░▒▓█").chars().collect();

    let (_names, data) = match &args[0] {
        Value::Matrix(m) => {
            let names = m
                .row_names
                .clone()
                .unwrap_or_else(|| (0..m.nrow).map(|i| format!("{i}")).collect());
            let mut data = vec![vec![0.0; m.ncol]; m.nrow];
            for r in 0..m.nrow {
                for c in 0..m.ncol {
                    data[r][c] = m.data[r * m.ncol + c];
                }
            }
            (names, data)
        }
        Value::Table(table) => {
            let mut cols_data: Vec<Vec<f64>> = Vec::new();
            for col in &table.columns {
                cols_data.push(extract_table_col(table, col)?);
            }
            let (nrows, ncols) = (table.num_rows(), table.num_cols());
            let mut t = vec![vec![0.0; ncols]; nrows];
            for c in 0..ncols {
                for r in 0..nrows {
                    t[r][c] = cols_data[c][r];
                }
            }
            let names: Vec<String> = (0..nrows).map(|i| format!("{i}")).collect();
            (names, t)
        }
        _ => {
            return Err(BioLangError::type_error(
                "hic_map() requires Matrix or Table",
                None,
            ))
        }
    };

    let n = data.len();
    let all: Vec<f64> = data
        .iter()
        .flat_map(|r| r.iter().copied())
        .filter(|v| v.is_finite() && *v > 0.0)
        .collect();
    let (vmin, vmax) = if all.is_empty() {
        (0.0, 1.0)
    } else {
        col_range(&all)
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 50.0;
        c.margin.bottom = 50.0;
        let cell = (c.plot_width() / n as f64).min(c.plot_height() / n as f64);
        for r in 0..n {
            for col in r..n {
                let v = data[r][col];
                let t = if (vmax - vmin).abs() < f64::EPSILON {
                    0.5
                } else {
                    ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
                };
                let color = sequential_color(t);
                c.add_rect(
                    c.margin.left + col as f64 * cell,
                    c.margin.top + r as f64 * cell,
                    cell,
                    cell,
                    &color,
                );
            }
        }
        finish_themed_canvas(&mut c, &opts, "Hi-C Contact Map");
        return Ok(Value::Str(c.render()));
    }

    let nlevels = heat_chars.len();
    let mut out = String::from("  Hi-C Contact Map\n");
    for r in 0..n {
        out.push_str("  ");
        for col in 0..n {
            if col < r {
                out.push_str("  ");
                continue;
            }
            let v = data[r][col];
            let t = if (vmax - vmin).abs() < f64::EPSILON {
                0.5
            } else {
                ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
            };
            out.push(heat_chars[(t * (nlevels - 1) as f64).round() as usize]);
            out.push(' ');
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 21. sashimi ─────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct SashimiCoverageDatum {
    source_row: usize,
    chromosome: Option<String>,
    position: f64,
    depth: f64,
}

#[derive(Clone, Debug)]
struct SashimiJunctionDatum {
    source_row: usize,
    chromosome: Option<String>,
    start: f64,
    end: f64,
    count: f64,
    strand: String,
    lane: usize,
    arc_fraction: f64,
    stroke_width: f64,
    label_drawn: bool,
}

fn sashimi_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let (coverage_table, junction_table) = match value {
        Value::Record(map) => {
            let coverage = match map.get("coverage") {
                None | Some(Value::Nil) => None,
                Some(Value::Table(table)) => Some(table),
                Some(_) => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "sashimi() coverage must be a Table",
                        None,
                    ))
                }
            };
            let junctions = match map.get("junctions") {
                Some(Value::Table(table)) => table,
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "sashimi() needs a junctions Table",
                        None,
                    ))
                }
            };
            (coverage, junctions)
        }
        Value::Table(table) => (None, table),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "sashimi() requires Record or Table, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    if junction_table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() junctions data is empty",
            None,
        ));
    }
    let start_column = junction_table.col_index("start").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'start' not found", None)
    })?;
    let end_column = junction_table.col_index("end").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'end' not found", None)
    })?;
    let count_column = junction_table.col_index("count");
    let junction_chromosome_column = junction_table
        .col_index("chrom")
        .or_else(|| junction_table.col_index("chromosome"));
    let strand_column = junction_table.col_index("strand");
    let mut junctions = Vec::with_capacity(junction_table.num_rows());
    for (source_row, row) in junction_table.rows.iter().enumerate() {
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let count = count_column
            .and_then(|column| row[column].as_float())
            .unwrap_or(1.0);
        if !start.is_finite()
            || !end.is_finite()
            || !count.is_finite()
            || start < 0.0
            || end <= start
            || count < 0.0
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sashimi() junction start/end/count must be finite and non-negative, with end > start",
                None,
            ));
        }
        let chromosome = optional_string_cell(row, junction_chromosome_column, "sashimi")?;
        let strand = optional_string_cell(row, strand_column, "sashimi")?.unwrap_or_default();
        if !matches!(strand.as_str(), "" | "+" | "-" | ".") {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sashimi() strand must be '+', '-', '.', or nil",
                None,
            ));
        }
        junctions.push(SashimiJunctionDatum {
            source_row,
            chromosome,
            start,
            end,
            count,
            strand,
            lane: 0,
            arc_fraction: 0.0,
            stroke_width: 0.0,
            label_drawn: false,
        });
    }
    let mut coverage = Vec::new();
    if let Some(table) = coverage_table {
        let position_column = table
            .col_index("pos")
            .or_else(|| table.col_index("position"))
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "sashimi() coverage needs a 'pos' or 'position' column",
                    None,
                )
            })?;
        let depth_column = table.col_index("depth").ok_or_else(|| {
            BioLangError::runtime(ErrorKind::TypeError, "column 'depth' not found", None)
        })?;
        let chromosome_column = table
            .col_index("chrom")
            .or_else(|| table.col_index("chromosome"));
        for (source_row, row) in table.rows.iter().enumerate() {
            let position = row[position_column].as_float().unwrap_or(f64::NAN);
            let depth = row[depth_column].as_float().unwrap_or(f64::NAN);
            if !position.is_finite() || position < 0.0 || !depth.is_finite() || depth < 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "sashimi() coverage positions and depths must be finite and non-negative",
                    None,
                ));
            }
            coverage.push(SashimiCoverageDatum {
                source_row,
                chromosome: optional_string_cell(row, chromosome_column, "sashimi")?,
                position,
                depth,
            });
        }
    }
    let input_junctions = junctions.len();
    let input_coverage = coverage.len();
    let requested_chromosome = opts.get("chromosome").and_then(Value::as_str);
    if let Some(chromosome) = requested_chromosome {
        if chromosome.trim().is_empty()
            || (junctions.iter().all(|datum| datum.chromosome.is_none())
                && coverage.iter().all(|datum| datum.chromosome.is_none()))
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sashimi() chromosome filtering needs chromosome data",
                None,
            ));
        }
        junctions.retain(|datum| {
            datum
                .chromosome
                .as_deref()
                .is_none_or(|value| value == chromosome)
        });
        coverage.retain(|datum| {
            datum
                .chromosome
                .as_deref()
                .is_none_or(|value| value == chromosome)
        });
    }
    for (present, total, family) in [
        (
            junctions
                .iter()
                .filter(|datum| datum.chromosome.is_some())
                .count(),
            junctions.len(),
            "junction",
        ),
        (
            coverage
                .iter()
                .filter(|datum| datum.chromosome.is_some())
                .count(),
            coverage.len(),
            "coverage",
        ),
    ] {
        if present != 0 && present != total {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("sashimi() {family} chromosome values must be complete or omitted"),
                None,
            ));
        }
    }
    let chromosomes = junctions
        .iter()
        .filter_map(|datum| datum.chromosome.as_deref())
        .chain(
            coverage
                .iter()
                .filter_map(|datum| datum.chromosome.as_deref()),
        )
        .collect::<HashSet<_>>();
    if chromosomes.len() > 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() draws one region; select one chromosome with {chromosome: \"chr1\"}",
            None,
        ));
    }
    let selected_chromosome = chromosomes.into_iter().next().map(str::to_string);
    let region_start = opts.get("region_start").and_then(Value::as_float);
    let region_end = opts.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|number| !number.is_finite() || number < 0.0)
        || region_end.is_some_and(|number| !number.is_finite() || number < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() region_start/region_end must be finite, non-negative and increasing",
            None,
        ));
    }
    junctions.retain(|datum| {
        region_start.is_none_or(|start| datum.start >= start)
            && region_end.is_none_or(|end| datum.end <= end)
    });
    coverage.retain(|datum| {
        region_start.is_none_or(|start| datum.position >= start)
            && region_end.is_none_or(|end| datum.position <= end)
    });
    if junctions.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() no complete junctions remain in the requested region",
            None,
        ));
    }
    junctions.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    coverage.sort_by(|left, right| {
        left.position
            .total_cmp(&right.position)
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let lane_count = assign_interval_lanes(
        &mut junctions,
        |datum| datum.start,
        |datum| datum.end,
        |datum, lane| datum.lane = lane,
    );
    let max_count = junctions
        .iter()
        .map(|datum| datum.count)
        .fold(0.0, f64::max)
        .max(1.0);
    for datum in &mut junctions {
        let strength = (datum.count / max_count).sqrt();
        datum.arc_fraction = 0.35 + 0.65 * strength;
        datum.stroke_width = 1.0 + (strength * 12.0).round() / 4.0;
    }
    let max_labels = get_opt_usize(opts, "max_labels", 60);
    let show_labels = opts
        .get("show_labels")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if show_labels && max_labels > 0 {
        let mut by_count: Vec<usize> = (0..junctions.len()).collect();
        by_count.sort_by(|left, right| {
            junctions[*right]
                .count
                .total_cmp(&junctions[*left].count)
                .then_with(|| {
                    junctions[*left]
                        .source_row
                        .cmp(&junctions[*right].source_row)
                })
        });
        for index in by_count.into_iter().take(max_labels) {
            junctions[index].label_drawn = true;
        }
    }
    let data_start = junctions
        .iter()
        .map(|datum| datum.start)
        .chain(coverage.iter().map(|datum| datum.position))
        .fold(f64::INFINITY, f64::min);
    let data_end = junctions
        .iter()
        .map(|datum| datum.end)
        .chain(coverage.iter().map(|datum| datum.position))
        .fold(f64::NEG_INFINITY, f64::max);
    let domain_start = region_start.unwrap_or(data_start);
    let domain_end = region_end.unwrap_or(data_end);
    let max_depth = coverage
        .iter()
        .map(|datum| datum.depth)
        .fold(0.0, f64::max)
        .max(1.0);
    let coverage_rows = coverage
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                datum
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(datum.position),
                Value::Float(datum.depth),
            ]
        })
        .collect();
    let junction_rows = junctions
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                datum
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(datum.start),
                Value::Float(datum.end),
                Value::Float(datum.end - datum.start),
                Value::Float(datum.count),
                if datum.strand.is_empty() {
                    Value::Nil
                } else {
                    Value::Str(datum.strand.clone())
                },
                Value::Int(datum.lane as i64),
                Value::Float(datum.arc_fraction),
                Value::Float(datum.stroke_width),
                Value::Str(PALETTE[datum.lane % PALETTE.len()].into()),
                Value::Bool(datum.label_drawn),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Sashimi Plot");
    let mut warnings = Vec::new();
    if junctions.len() != input_junctions || coverage.len() != input_coverage {
        warnings.push(Value::Str(format!(
            "{} of {input_junctions} complete junctions and {} of {input_coverage} coverage points retained",
            junctions.len(),
            coverage.len()
        )));
    }
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("sashimi".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "coverage".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "chromosome",
                        "position",
                        "depth",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    coverage_rows,
                )),
            ),
            (
                "junctions".into(),
                Value::Table(Table::new(
                    [
                        "junction_index",
                        "source_row",
                        "chromosome",
                        "start",
                        "end",
                        "span",
                        "count",
                        "strand",
                        "lane",
                        "arc_fraction",
                        "stroke_width",
                        "color",
                        "label_drawn",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    junction_rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 900.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 380.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Genomic position").into()),
                        ),
                        (
                            "chromosome".into(),
                            selected_chromosome.map(Value::Str).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_start".into(),
                            region_start.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_end".into(),
                            region_end.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        ("domain_start".into(), Value::Float(domain_start)),
                        ("domain_end".into(), Value::Float(domain_end)),
                        ("max_count".into(), Value::Float(max_count)),
                        ("max_depth".into(), Value::Float(max_depth)),
                        ("lane_count".into(), Value::Int(lane_count as i64)),
                        ("show_labels".into(), Value::Bool(show_labels)),
                        ("max_labels".into(), Value::Int(max_labels as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("sashimi".into())),
                        (
                            "junction_order".into(),
                            Value::Str("start_end_source_row".into()),
                        ),
                        (
                            "coverage_order".into(),
                            Value::Str("position_source_row".into()),
                        ),
                        (
                            "lane_rule".into(),
                            Value::Str("greedy_first_non_overlapping_lane".into()),
                        ),
                        (
                            "region_rule".into(),
                            Value::Str("complete_junctions_points_inclusive".into()),
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

fn render_sashimi_svg(
    coverage: &Table,
    junctions: &Table,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 900.0);
    let height = get_opt_f64(opts, "height", 380.0);
    let title = get_opt_str(opts, "title", "Sashimi Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let xlabel = get_opt_str(opts, "xlabel", "Genomic position");
    let domain = (
        get_opt_f64(opts, "domain_start", 0.0),
        get_opt_f64(opts, "domain_end", 1.0),
    );
    let max_depth = get_opt_f64(opts, "max_depth", 1.0);
    let lane_count = get_opt_usize(opts, "lane_count", 1).max(1);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[domain.0, domain.1],
        &[0.0, max_depth],
        xlabel,
        "",
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let panel_top = canvas.margin.top;
    let coverage_height = canvas.plot_height() * 0.43;
    let separator_y = panel_top + coverage_height + 12.0;
    let arc_base = canvas.margin.top + canvas.plot_height() - 6.0;
    let arc_height = (arc_base - separator_y - 8.0).max(10.0);
    canvas.add_line(
        canvas.margin.left,
        separator_y,
        canvas.margin.left + canvas.plot_width(),
        separator_y,
        "#c5c9d0",
        1.0,
    );
    if coverage.num_rows() > 0 {
        let position_column = coverage.col_index("position").unwrap();
        let depth_column = coverage.col_index("depth").unwrap();
        let mut area = String::new();
        let mut line = String::new();
        for (index, row) in coverage.rows.iter().enumerate() {
            let x = x_scale.map(row[position_column].as_float().unwrap());
            let depth = row[depth_column].as_float().unwrap();
            let y = panel_top + coverage_height - depth / max_depth * coverage_height;
            if index == 0 {
                area.push_str(&format!(
                    "M{x:.2},{:.2}L{x:.2},{y:.2}",
                    panel_top + coverage_height
                ));
                line.push_str(&format!("M{x:.2},{y:.2}"));
            } else {
                area.push_str(&format!("L{x:.2},{y:.2}"));
                line.push_str(&format!("L{x:.2},{y:.2}"));
            }
        }
        let last_x = x_scale.map(
            coverage.rows.last().unwrap()[position_column]
                .as_float()
                .unwrap(),
        );
        area.push_str(&format!("L{last_x:.2},{:.2}Z", panel_top + coverage_height));
        canvas.elements.push(format!(
            r##"<path d="{area}" fill="#9aa6b2" fill-opacity="0.35" stroke="none" />"##
        ));
        canvas.elements.push(format!(
            r##"<path d="{line}" fill="none" stroke="#5f6b76" stroke-width="1.3" stroke-linejoin="round" />"##
        ));
        canvas.add_text(
            canvas.margin.left + 4.0,
            panel_top + 12.0,
            &format!("Coverage (max {max_depth:.1})"),
            "start",
            9.0,
        );
    }
    let start_column = junctions.col_index("start").unwrap();
    let end_column = junctions.col_index("end").unwrap();
    let count_column = junctions.col_index("count").unwrap();
    let lane_column = junctions.col_index("lane").unwrap();
    let fraction_column = junctions.col_index("arc_fraction").unwrap();
    let stroke_column = junctions.col_index("stroke_width").unwrap();
    let color_column = junctions.col_index("color").unwrap();
    let label_column = junctions.col_index("label_drawn").unwrap();
    let mut paths: BTreeMap<(String, i64), String> = BTreeMap::new();
    for row in &junctions.rows {
        let x1 = x_scale.map(row[start_column].as_float().unwrap());
        let x2 = x_scale.map(row[end_column].as_float().unwrap());
        let lane = row[lane_column].as_float().unwrap() as usize;
        let fraction = row[fraction_column].as_float().unwrap();
        let stroke_width = row[stroke_column].as_float().unwrap();
        let color = row[color_column].as_str().unwrap();
        let lane_fraction = (lane + 1) as f64 / lane_count as f64;
        let apex_y = arc_base - arc_height * lane_fraction * fraction;
        let mid_x = (x1 + x2) / 2.0;
        paths
            .entry((color.into(), (stroke_width * 100.0).round() as i64))
            .or_default()
            .push_str(&format!(
                "M{x1:.2},{arc_base:.2}Q{mid_x:.2},{apex_y:.2} {x2:.2},{arc_base:.2}"
            ));
        if row[label_column].as_bool().unwrap_or(false) {
            canvas.add_text(
                mid_x,
                apex_y - 4.0,
                &format!("{:.0}", row[count_column].as_float().unwrap()),
                "middle",
                8.5,
            );
        }
    }
    for ((color, stroke_key), path) in paths {
        let stroke_width = stroke_key as f64 / 100.0;
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{stroke_width:.2}" stroke-opacity="0.82" />"#
        ));
    }
    canvas.add_text(
        canvas.margin.left + 4.0,
        separator_y + 13.0,
        "Splice junction reads",
        "start",
        9.0,
    );
    canvas.draw_x_axis(
        &Scale {
            domain,
            range: domain,
        },
        xlabel,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Sashimi plot with {} coverage points and {} splice junctions across {lane_count} non-overlapping arc lanes.",
        coverage.num_rows(),
        junctions.num_rows()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_sashimi_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "sashimi"))
}

pub(crate) fn render_sashimi_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_sashimi_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 sashimi Record",
                None,
            ))
        }
    };
    let coverage = match map.get("coverage") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi specification field 'coverage' must be Table",
                None,
            ))
        }
    };
    let junctions = match map.get("junctions") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi specification field 'junctions' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "chromosome",
        "position",
        "depth",
    ] {
        if coverage.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() sashimi coverage is missing '{required}'"),
                None,
            ));
        }
    }
    for required in [
        "junction_index",
        "source_row",
        "chromosome",
        "start",
        "end",
        "span",
        "count",
        "strand",
        "lane",
        "arc_fraction",
        "stroke_width",
        "color",
        "label_drawn",
    ] {
        if junctions.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() sashimi junctions are missing '{required}'"),
                None,
            ));
        }
    }
    if junctions.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() sashimi specification has no junctions",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "sashimi")?;
    let domain_start = options
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let domain_end = options
        .get("domain_end")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let max_count = options
        .get("max_count")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let max_depth = options
        .get("max_depth")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let lane_count = options
        .get("lane_count")
        .map(|value| frozen_nonnegative_integer(value, "sashimi", "lane_count"))
        .transpose()?
        .unwrap_or(0);
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_start < 0.0
        || domain_end <= domain_start
        || !max_count.is_finite()
        || max_count <= 0.0
        || !max_depth.is_finite()
        || max_depth <= 0.0
        || lane_count == 0
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() sashimi frozen domains are malformed",
            None,
        ));
    }
    let point_index_column = coverage.col_index("point_index").unwrap();
    let point_source_column = coverage.col_index("source_row").unwrap();
    let point_chromosome_column = coverage.col_index("chromosome").unwrap();
    let position_column = coverage.col_index("position").unwrap();
    let depth_column = coverage.col_index("depth").unwrap();
    let selected_chromosome = options.get("chromosome").and_then(Value::as_str);
    let mut previous_point: Option<(f64, usize)> = None;
    let mut observed_max_depth = 1.0f64;
    for (expected, row) in coverage.rows.iter().enumerate() {
        let index = frozen_nonnegative_integer(&row[point_index_column], "sashimi", "point_index")?;
        let source =
            frozen_nonnegative_integer(&row[point_source_column], "sashimi", "source_row")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let depth = row[depth_column].as_float().unwrap_or(f64::NAN);
        let chromosome = match &row[point_chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() sashimi coverage chromosome is malformed",
                    None,
                ))
            }
        };
        let key = (position, source);
        if index != expected
            || !position.is_finite()
            || position < domain_start
            || position > domain_end
            || !depth.is_finite()
            || depth < 0.0
            || depth > max_depth
            || chromosome.is_some_and(|name| selected_chromosome != Some(name))
            || previous_point.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi frozen coverage is inconsistent",
                None,
            ));
        }
        observed_max_depth = observed_max_depth.max(depth);
        previous_point = Some(key);
    }
    let junction_index_column = junctions.col_index("junction_index").unwrap();
    let source_column = junctions.col_index("source_row").unwrap();
    let chromosome_column = junctions.col_index("chromosome").unwrap();
    let start_column = junctions.col_index("start").unwrap();
    let end_column = junctions.col_index("end").unwrap();
    let span_column = junctions.col_index("span").unwrap();
    let count_column = junctions.col_index("count").unwrap();
    let strand_column = junctions.col_index("strand").unwrap();
    let lane_column = junctions.col_index("lane").unwrap();
    let fraction_column = junctions.col_index("arc_fraction").unwrap();
    let stroke_column = junctions.col_index("stroke_width").unwrap();
    let color_column = junctions.col_index("color").unwrap();
    let label_column = junctions.col_index("label_drawn").unwrap();
    let mut previous_junction: Option<(f64, f64, usize)> = None;
    let mut lane_ends = vec![f64::NEG_INFINITY; lane_count];
    let mut observed_max_count = 1.0f64;
    let mut highest_lane = 0usize;
    for (expected, row) in junctions.rows.iter().enumerate() {
        let index =
            frozen_nonnegative_integer(&row[junction_index_column], "sashimi", "junction_index")?;
        let source = frozen_nonnegative_integer(&row[source_column], "sashimi", "source_row")?;
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let span = row[span_column].as_float().unwrap_or(f64::NAN);
        let count = row[count_column].as_float().unwrap_or(f64::NAN);
        let lane = frozen_nonnegative_integer(&row[lane_column], "sashimi", "lane")?;
        let chromosome = match &row[chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() sashimi junction chromosome is malformed",
                    None,
                ))
            }
        };
        let strand_ok = matches!(&row[strand_column], Value::Nil)
            || matches!(row[strand_column].as_str(), Some("+" | "-" | "."));
        let fraction = row[fraction_column].as_float().unwrap_or(f64::NAN);
        let stroke = row[stroke_column].as_float().unwrap_or(f64::NAN);
        let expected_lane = lane_ends
            .iter()
            .position(|lane_end| *lane_end <= start)
            .unwrap_or(lane_count);
        let expected_strength = (count / max_count).sqrt();
        let expected_fraction = 0.35 + 0.65 * expected_strength;
        let expected_stroke = 1.0 + (expected_strength * 12.0).round() / 4.0;
        let key = (start, end, source);
        if index != expected
            || !start.is_finite()
            || !end.is_finite()
            || start < domain_start
            || end > domain_end
            || end <= start
            || (span - (end - start)).abs() > 1e-10
            || !count.is_finite()
            || count < 0.0
            || count > max_count
            || selected_chromosome != chromosome
            || !strand_ok
            || lane >= lane_count
            || lane != expected_lane
            || (fraction - expected_fraction).abs() > 1e-10
            || (stroke - expected_stroke).abs() > 1e-10
            || !row[color_column].as_str().is_some_and(valid_bio_hex_color)
            || !matches!(row[label_column], Value::Bool(_))
            || previous_junction.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi frozen junction geometry is inconsistent",
                None,
            ));
        }
        lane_ends[lane] = end;
        observed_max_count = observed_max_count.max(count);
        highest_lane = highest_lane.max(lane);
        previous_junction = Some(key);
    }
    if (observed_max_count - max_count).abs() > 1e-10
        || (observed_max_depth - max_depth).abs() > 1e-10
        || highest_lane + 1 != lane_count
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() sashimi frozen maxima or lane count are inconsistent",
            None,
        ));
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_sashimi_svg(coverage, junctions, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Sashimi Plot");
    finish_frozen_bio_plot(value, render_options, title, "sashimi", svg)
}

fn builtin_sashimi(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = sashimi_spec_value(&args[0], &opts)?;
    render_sashimi_plot_spec_value(&specification, &opts)
}

#[cfg(any())]
fn builtin_sashimi_legacy(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    // Input: Record{coverage: Table(pos, depth), junctions: Table(start, end, count)}
    // or just a Table of junctions
    let (cov_data, junctions) = match &args[0] {
        Value::Record(map) => {
            let cov = map.get("coverage").and_then(|v| {
                if let Value::Table(t) = v {
                    Some(t)
                } else {
                    None
                }
            });
            let junc = map.get("junctions").and_then(|v| {
                if let Value::Table(t) = v {
                    Some(t)
                } else {
                    None
                }
            });
            (cov.cloned(), junc.cloned())
        }
        Value::Table(t) => (None, Some(t.clone())),
        _ => {
            return Err(BioLangError::type_error(
                "sashimi() requires Record or Table",
                None,
            ))
        }
    };

    let junc_table = junctions.as_ref().ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "sashimi() needs junctions data", None)
    })?;
    let j_starts = extract_table_col(junc_table, "start")?;
    let j_ends = extract_table_col(junc_table, "end")?;
    let j_counts =
        extract_table_col(junc_table, "count").unwrap_or_else(|_| vec![1.0; j_starts.len()]);

    let mut all_pos: Vec<f64> = Vec::new();
    all_pos.extend(&j_starts);
    all_pos.extend(&j_ends);
    if let Some(ref ct) = cov_data {
        if let Ok(ps) = extract_table_col(ct, "pos") {
            all_pos.extend(&ps);
        }
    }
    let xr = col_range(&all_pos);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };

        // Coverage area
        if let Some(ref ct) = cov_data {
            if let (Ok(ps), Ok(ds)) = (extract_table_col(ct, "pos"), extract_table_col(ct, "depth"))
            {
                let max_d = ds.iter().cloned().fold(0.0f64, f64::max).max(1.0);
                let cov_h = c.plot_height() * 0.5;
                let base_y = c.margin.top + cov_h;
                let mut pts = format!("{:.1},{:.1} ", xs.map(ps[0]), base_y);
                for i in 0..ps.len() {
                    let y = base_y - (ds[i] / max_d) * cov_h;
                    pts.push_str(&format!("{:.1},{:.1} ", xs.map(ps[i]), y));
                }
                pts.push_str(&format!("{:.1},{:.1}", xs.map(*ps.last().unwrap()), base_y));
                c.elements.push(format!(
                    r##"<polygon points="{pts}" fill="#ccc" opacity="0.5" />"##
                ));
            }
        }

        // Junction arcs
        let max_count = j_counts.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        let arc_base = c.margin.top + c.plot_height() * 0.55;
        for i in 0..j_starts.len() {
            let x1 = xs.map(j_starts[i]);
            let x2 = xs.map(j_ends[i]);
            let mid_x = (x1 + x2) / 2.0;
            let arc_h = (j_counts[i] / max_count) * c.plot_height() * 0.35;
            c.elements.push(format!(
                r#"<path d="M {x1:.1},{arc_base:.1} Q {mid_x:.1},{:.1} {x2:.1},{arc_base:.1}" fill="none" stroke="{}" stroke-width="{:.1}" />"#,
                arc_base - arc_h, PALETTE[i % PALETTE.len()], (j_counts[i] / max_count * 3.0).max(1.0)
            ));
            c.add_text(
                mid_x,
                arc_base - arc_h - 5.0,
                &format!("{:.0}", j_counts[i]),
                "middle",
                9.0,
            );
        }

        let dx = Scale {
            domain: xr,
            range: xr,
        };
        c.draw_x_axis(&dx, "Position");
        c.draw_title("Sashimi Plot");
        return Ok(Value::Str(c.render()));
    }

    // ASCII
    let width = get_opt_usize(&opts, "width", 60);
    let mut out = String::from("  Sashimi Plot\n");

    // Coverage sparkline if available
    if let Some(ref ct) = cov_data {
        if let (Ok(ps), Ok(ds)) = (extract_table_col(ct, "pos"), extract_table_col(ct, "depth")) {
            let mut bins = vec![0.0; width];
            let mut counts = vec![0usize; width];
            let span = xr.1 - xr.0;
            for i in 0..ps.len() {
                let b = ((ps[i] - xr.0) / span * width as f64) as usize;
                let b = b.min(width - 1);
                bins[b] += ds[i];
                counts[b] += 1;
            }
            for i in 0..width {
                if counts[i] > 0 {
                    bins[i] /= counts[i] as f64;
                }
            }
            out.push_str(&format!("  Depth: {}\n", spark_str(&bins)));
        }
    }

    // Junction list
    out.push_str(&format!("  Junctions ({}):\n", j_starts.len()));
    for i in 0..j_starts.len().min(15) {
        out.push_str(&format!(
            "    {:.0}─{:.0} ({:.0} reads) ⌒\n",
            j_starts[i], j_ends[i], j_counts[i]
        ));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 22. volcano_plot ────────────────────────────────────────────

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

fn builtin_upset_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "UpSet Plot").to_string();
    let min_size = get_opt_f64(&opts, "min_size", 1.0) as usize;

    let sets: Vec<(String, HashSet<String>)> = match &args[0] {
        Value::Record(map) => map
            .iter()
            .map(|(n, v)| {
                let items: HashSet<String> = match v {
                    Value::List(l) => l.iter().map(|x| format!("{x}")).collect(),
                    _ => HashSet::new(),
                };
                (n.clone(), items)
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "upset_plot() requires Record of Lists",
                None,
            ))
        }
    };
    let n = sets.len();
    if n < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "upset_plot() needs >= 2 sets",
            None,
        ));
    }

    // Compute all intersection combinations (exclusive membership)
    let all_items: HashSet<String> = sets.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
    let mut combos: Vec<(Vec<bool>, usize)> = Vec::new();
    for mask in 1..(1u32 << n) {
        let membership: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
        let count = all_items
            .iter()
            .filter(|item| (0..n).all(|i| membership[i] == sets[i].1.contains(*item)))
            .count();
        if count >= min_size {
            combos.push((membership, count));
        }
    }
    combos.sort_by(|a, b| b.1.cmp(&a.1));

    // Set sizes
    let set_sizes: Vec<usize> = sets.iter().map(|(_, s)| s.len()).collect();
    let max_set_size = *set_sizes.iter().max().unwrap_or(&1);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 700.0);
        let h = get_opt_f64(&opts, "height", 500.0);
        let mut c = SvgCanvas::new(w, h);
        let left_bar_w = 100.0;
        c.margin.left = left_bar_w + 60.0;
        c.margin.bottom = 40.0;
        let nc = combos.len().min(25);
        let dot_area_h = n as f64 * 20.0 + 20.0;
        let bar_area_h = c.plot_height() - dot_area_h;
        let bar_w = if nc > 0 {
            c.plot_width() / nc as f64
        } else {
            c.plot_width()
        };
        let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1) as f64;
        let dot_top = c.margin.top + bar_area_h + 15.0;

        // Top: intersection size bars
        for (ci, (membership, count)) in combos.iter().take(nc).enumerate() {
            let x = c.margin.left + ci as f64 * bar_w + bar_w * 0.15;
            let bw = bar_w * 0.7;
            let bh = (*count as f64 / max_count) * bar_area_h * 0.9;
            c.add_rect(x, c.margin.top + bar_area_h - bh, bw, bh, "#333");
            c.add_text(
                x + bw / 2.0,
                c.margin.top + bar_area_h - bh - 5.0,
                &count.to_string(),
                "middle",
                9.0,
            );

            // Bottom: dot matrix
            let dx = x + bw / 2.0;
            let mut active_ys: Vec<f64> = Vec::new();
            for (si, &active) in membership.iter().enumerate() {
                let dy = dot_top + si as f64 * 20.0;
                c.add_circle(dx, dy, 5.0, if active { "#333" } else { "#ddd" });
                if active {
                    active_ys.push(dy);
                }
            }
            // Connect active dots with a line
            if active_ys.len() > 1 {
                let y_min = active_ys.iter().cloned().fold(f64::INFINITY, f64::min);
                let y_max = active_ys.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                c.add_line(dx, y_min, dx, y_max, "#333", 2.0);
            }
        }

        // Left: set size bars and labels
        for (si, (name, _)) in sets.iter().enumerate() {
            let y = dot_top + si as f64 * 20.0;
            c.add_text(c.margin.left - left_bar_w - 5.0, y + 4.0, name, "end", 10.0);
            let bar_len = (set_sizes[si] as f64 / max_set_size as f64) * left_bar_w * 0.9;
            c.add_rect(
                c.margin.left - bar_len - 2.0,
                y - 6.0,
                bar_len,
                12.0,
                PALETTE[si % PALETTE.len()],
            );
            c.add_text(
                c.margin.left - bar_len - 8.0,
                y + 4.0,
                &set_sizes[si].to_string(),
                "end",
                8.0,
            );
        }

        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let max_name = sets.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let nc = combos.len().min(15);
    let max_count = combos.iter().map(|(_, c)| *c).max().unwrap_or(1);
    let mut out = format!("  {title}\n");
    out.push_str(&format!("  {:>w$}  ", "count", w = max_name));
    for (_, count) in combos.iter().take(nc) {
        let _bar_len = (*count as f64 / max_count as f64 * 5.0).ceil() as usize;
        out.push_str(&format!("{:>3} ", count));
    }
    out.push('\n');
    for (si, (name, _)) in sets.iter().enumerate() {
        out.push_str(&format!("  {:>w$}  ", name, w = max_name));
        for (membership, _) in combos.iter().take(nc) {
            out.push_str(if membership[si] { " ●  " } else { " ·  " });
        }
        out.push_str(&format!("  ({})\n", set_sizes[si]));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 24. alignment_view (MSA) ────────────────────────────────────

fn builtin_alignment_view(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "Multiple Sequence Alignment").to_string();
    let color_by = get_opt_str(&opts, "color_by", "nucleotide").to_string();
    let start_pos = get_opt_f64(&opts, "start", 0.0) as usize;
    let end_pos_opt: Option<usize> = opts
        .get("end")
        .and_then(|v| v.as_float())
        .map(|f| f as usize);

    // Extract sequences: List of Records with {id, sequence}
    let (ids, sequences): (Vec<String>, Vec<String>) = match &args[0] {
        Value::List(items) => {
            let mut ids = Vec::new();
            let mut seqs = Vec::new();
            for item in items.iter() {
                match item {
                    Value::Record(map) => {
                        let id = map
                            .get("id")
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| format!("seq{}", ids.len() + 1));
                        let seq = map
                            .get("sequence")
                            .or(map.get("seq"))
                            .map(|v| match v {
                                Value::Str(s) => s.clone(),
                                Value::DNA(s) | Value::RNA(s) | Value::Protein(s) => s.data.clone(),
                                _ => format!("{v}"),
                            })
                            .unwrap_or_default();
                        ids.push(id);
                        seqs.push(seq);
                    }
                    Value::Str(s) => {
                        ids.push(format!("seq{}", ids.len() + 1));
                        seqs.push(s.clone());
                    }
                    Value::DNA(s) | Value::RNA(s) | Value::Protein(s) => {
                        ids.push(format!("seq{}", ids.len() + 1));
                        seqs.push(s.data.clone());
                    }
                    _ => {}
                }
            }
            (ids, seqs)
        }
        Value::Table(table) => {
            let id_col = table
                .col_index("id")
                .or(table.col_index("name"))
                .unwrap_or(0);
            let seq_col = table.col_index("sequence").or(table.col_index("seq"));
            if seq_col.is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "alignment_view() needs 'sequence' column",
                    None,
                ));
            }
            let seq_idx = seq_col.unwrap();
            let ids: Vec<String> = table
                .rows
                .iter()
                .map(|r| match &r[id_col] {
                    Value::Str(s) => s.clone(),
                    other => format!("{other}"),
                })
                .collect();
            let seqs: Vec<String> = table
                .rows
                .iter()
                .map(|r| match &r[seq_idx] {
                    Value::Str(s) => s.clone(),
                    Value::DNA(s) | Value::RNA(s) | Value::Protein(s) => s.data.clone(),
                    other => format!("{other}"),
                })
                .collect();
            (ids, seqs)
        }
        _ => {
            return Err(BioLangError::type_error(
                "alignment_view() requires List of Records or Table",
                None,
            ))
        }
    };
    if sequences.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "alignment_view() empty input",
            None,
        ));
    }

    let max_len = sequences.iter().map(|s| s.len()).max().unwrap_or(0);
    let end_pos = end_pos_opt.unwrap_or(max_len).min(max_len);
    let start = start_pos.min(end_pos);
    let display_len = (end_pos - start).min(100); // limit to 100 positions

    // Compute consensus
    let mut consensus = String::new();
    for pos in start..(start + display_len) {
        let mut counts: HashMap<char, usize> = HashMap::new();
        for seq in &sequences {
            if let Some(ch) = seq.chars().nth(pos) {
                *counts.entry(ch.to_ascii_uppercase()).or_insert(0) += 1;
            }
        }
        let top = counts
            .iter()
            .max_by_key(|(_, &c)| c)
            .map(|(&ch, _)| ch)
            .unwrap_or('-');
        consensus.push(top);
    }

    // Compute conservation scores (fraction matching consensus)
    let conservation: Vec<f64> = (0..display_len)
        .map(|di| {
            let pos = start + di;
            let cons_char = consensus.chars().nth(di).unwrap_or('-');
            let matches = sequences
                .iter()
                .filter(|s| s.chars().nth(pos).map(|c| c.to_ascii_uppercase()) == Some(cons_char))
                .count();
            matches as f64 / sequences.len() as f64
        })
        .collect();

    fn nuc_color(ch: char) -> &'static str {
        match ch.to_ascii_uppercase() {
            'A' => "#4caf50",
            'T' | 'U' => "#f44336",
            'C' => "#2196f3",
            'G' => "#ff9800",
            '-' => "#ffffff",
            _ => "#cccccc",
        }
    }

    fn conservation_color(score: f64) -> String {
        let t = score.clamp(0.0, 1.0);
        let r = (255.0 * (1.0 - t * 0.7)) as u8;
        let g = (255.0 * (1.0 - t * 0.4)) as u8;
        let b = (255.0 * (1.0 - t * 0.1)) as u8;
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    if fmt == "svg" {
        let cell_w = 12.0;
        let cell_h = 16.0;
        let label_w = ids.iter().map(|s| s.len()).max().unwrap_or(4) as f64 * 7.0 + 10.0;
        let pos_h = 20.0;
        let cons_h = 18.0;
        let w_auto = label_w + display_len as f64 * cell_w + 40.0;
        let h_auto = pos_h + (sequences.len() + 1) as f64 * cell_h + cons_h + 60.0;
        let w = get_opt_f64(&opts, "width", w_auto.min(1200.0));
        let h = get_opt_f64(&opts, "height", h_auto.min(800.0));
        let mut c = SvgCanvas::new(w, h);
        c.margin.left = label_w;
        c.margin.top = 50.0;

        let actual_cell_w = (c.plot_width() / display_len as f64).min(cell_w);

        // Position numbers at top (every 10th)
        for di in 0..display_len {
            let pos = start + di;
            if pos % 10 == 0 {
                let x = c.margin.left + di as f64 * actual_cell_w + actual_cell_w / 2.0;
                c.add_text(x, c.margin.top - 5.0, &pos.to_string(), "middle", 8.0);
            }
        }

        // Sequence rows
        for (si, seq) in sequences.iter().enumerate() {
            let y = c.margin.top + si as f64 * cell_h;
            c.add_text(c.margin.left - 5.0, y + cell_h * 0.7, &ids[si], "end", 9.0);
            for di in 0..display_len {
                let pos = start + di;
                let ch = seq.chars().nth(pos).unwrap_or('-');
                let x = c.margin.left + di as f64 * actual_cell_w;
                let fill = if color_by == "conservation" {
                    conservation_color(conservation[di])
                } else {
                    nuc_color(ch).to_string()
                };
                c.elements.push(format!(
                    r#"<rect x="{x:.1}" y="{y:.1}" width="{actual_cell_w:.1}" height="{:.1}" fill="{fill}" opacity="0.6" />"#,
                    cell_h - 1.0
                ));
                if actual_cell_w >= 8.0 {
                    c.elements.push(format!(
                        r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{:.0}" font-family="monospace" fill="#333">{}</text>"##,
                        x + actual_cell_w / 2.0, y + cell_h * 0.7, (actual_cell_w * 0.8).min(11.0), ch
                    ));
                }
            }
        }

        // Consensus row
        let cons_y = c.margin.top + sequences.len() as f64 * cell_h + 5.0;
        c.add_text(
            c.margin.left - 5.0,
            cons_y + cell_h * 0.7,
            "Consensus",
            "end",
            9.0,
        );
        for (di, cons_ch) in consensus.chars().enumerate() {
            let x = c.margin.left + di as f64 * actual_cell_w;
            let bg = if conservation[di] > 0.8 {
                "#333"
            } else if conservation[di] > 0.5 {
                "#888"
            } else {
                "#ccc"
            };
            c.elements.push(format!(
                r#"<rect x="{x:.1}" y="{cons_y:.1}" width="{actual_cell_w:.1}" height="{:.1}" fill="{bg}" />"#,
                cell_h - 1.0
            ));
            if actual_cell_w >= 8.0 {
                c.elements.push(format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{:.0}" font-family="monospace" fill="white" font-weight="bold">{}</text>"#,
                    x + actual_cell_w / 2.0, cons_y + cell_h * 0.7, (actual_cell_w * 0.8).min(11.0), cons_ch
                ));
            }
        }

        if end_pos - start > 100 {
            c.add_text(
                c.margin.left + c.plot_width() + 5.0,
                c.margin.top + 20.0,
                "...",
                "start",
                12.0,
            );
        }

        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let max_id = ids.iter().map(|s| s.len()).max().unwrap_or(4);
    let term_w = get_opt_usize(&opts, "width", 80);
    let show_len = display_len.min(term_w.saturating_sub(max_id + 3));
    let mut out = format!("  {title}\n");
    out.push_str(&format!("  {:>w$}  ", "", w = max_id));
    for di in 0..show_len {
        let pos = start + di;
        if pos % 10 == 0 {
            out.push('|');
        } else {
            out.push(' ');
        }
    }
    out.push('\n');
    for (si, seq) in sequences.iter().enumerate() {
        out.push_str(&format!("  {:>w$}  ", ids[si], w = max_id));
        for di in 0..show_len {
            let pos = start + di;
            out.push(seq.chars().nth(pos).unwrap_or('-'));
        }
        out.push('\n');
    }
    out.push_str(&format!("  {:>w$}  ", "Cons", w = max_id));
    for ch in consensus.chars().take(show_len) {
        out.push(ch);
    }
    out.push('\n');
    out.push_str(&format!("  {:>w$}  ", "", w = max_id));
    for &cv in conservation.iter().take(show_len) {
        out.push(if cv > 0.9 {
            '*'
        } else if cv > 0.5 {
            ':'
        } else if cv > 0.3 {
            '.'
        } else {
            ' '
        });
    }
    out.push('\n');
    write_output(&out);
    Ok(Value::Nil)
}

// ── 25. circos_plot ─────────────────────────────────────────────

fn builtin_circos_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "Circos Plot").to_string();
    let track_type = get_opt_str(&opts, "track", "bar").to_string();

    // Default human chromosome sizes (in bp)
    let default_chroms: Vec<(&str, f64)> = vec![
        ("chr1", 248956422.0),
        ("chr2", 242193529.0),
        ("chr3", 198295559.0),
        ("chr4", 190214555.0),
        ("chr5", 181538259.0),
        ("chr6", 170805979.0),
        ("chr7", 159345973.0),
        ("chr8", 145138636.0),
        ("chr9", 138394717.0),
        ("chr10", 133797422.0),
        ("chr11", 135086622.0),
        ("chr12", 133275309.0),
        ("chr13", 114364328.0),
        ("chr14", 107043718.0),
        ("chr15", 101991189.0),
        ("chr16", 90338345.0),
        ("chr17", 83257441.0),
        ("chr18", 80373285.0),
        ("chr19", 58617616.0),
        ("chr20", 64444167.0),
        ("chr21", 46709983.0),
        ("chr22", 50818468.0),
        ("chrX", 156040895.0),
        ("chrY", 57227415.0),
    ];

    // Extract data points: List of Records with {chrom, start, end, value}
    let data_points: Vec<(String, f64, f64, f64)> = match &args[0] {
        Value::List(items) => items
            .iter()
            .filter_map(|item| {
                if let Value::Record(map) = item {
                    let chrom = map
                        .get("chrom")
                        .or(map.get("chr"))
                        .map(|v| format!("{v}"))?;
                    let start = map
                        .get("start")
                        .or(map.get("pos"))
                        .and_then(|v| v.as_float())
                        .unwrap_or(0.0);
                    let end = map
                        .get("end")
                        .and_then(|v| v.as_float())
                        .unwrap_or(start + 1.0);
                    let value = map
                        .get("value")
                        .or(map.get("score"))
                        .and_then(|v| v.as_float())
                        .unwrap_or(1.0);
                    Some((chrom, start, end, value))
                } else {
                    None
                }
            })
            .collect(),
        Value::Table(table) => {
            let chroms = extract_str_col(
                table,
                if table.col_index("chrom").is_some() {
                    "chrom"
                } else {
                    "chr"
                },
            )?;
            let starts = extract_table_col(
                table,
                if table.col_index("start").is_some() {
                    "start"
                } else {
                    "pos"
                },
            )?;
            let ends = extract_table_col(table, "end")
                .unwrap_or_else(|_| starts.iter().map(|&s| s + 1.0).collect());
            let values = extract_table_col(table, "value")
                .or_else(|_| extract_table_col(table, "score"))
                .unwrap_or_else(|_| vec![1.0; chroms.len()]);
            chroms
                .into_iter()
                .zip(starts.into_iter().zip(ends.into_iter().zip(values)))
                .map(|(c, (s, (e, v)))| (c, s, e, v))
                .collect()
        }
        _ => {
            return Err(BioLangError::type_error(
                "circos_plot() requires List of Records or Table",
                None,
            ))
        }
    };

    // Determine chromosome set
    let mut chrom_order: Vec<String> = Vec::new();
    for (c, _, _, _) in &data_points {
        if !chrom_order.contains(c) {
            chrom_order.push(c.clone());
        }
    }
    let default_order: Vec<&str> = default_chroms.iter().map(|(n, _)| *n).collect();
    chrom_order.sort_by_key(|c| default_order.iter().position(|&d| d == c).unwrap_or(999));

    let chrom_sizes: Vec<(String, f64)> = chrom_order
        .iter()
        .map(|c| {
            let default_size = default_chroms
                .iter()
                .find(|(n, _)| *n == c.as_str())
                .map(|(_, s)| *s);
            let data_max = data_points
                .iter()
                .filter(|(dc, _, _, _)| dc == c)
                .map(|(_, _, e, _)| *e)
                .fold(0.0f64, f64::max);
            (c.clone(), default_size.unwrap_or(data_max * 1.1))
        })
        .collect();
    let total_size: f64 = chrom_sizes.iter().map(|(_, s)| s).sum();
    if total_size <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos_plot() no data to plot",
            None,
        ));
    }

    let val_range = {
        let vals: Vec<f64> = data_points.iter().map(|(_, _, _, v)| *v).collect();
        if vals.is_empty() {
            (0.0, 1.0)
        } else {
            col_range(&vals)
        }
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 500.0);
        let h = w;
        let mut c = SvgCanvas::new(w, h);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let r_outer = w.min(h) * 0.40;
        let r_inner = r_outer * 0.82;
        let r_track_outer = r_inner * 0.95;
        let r_track_inner = r_inner * 0.65;
        let gap = 0.01;
        let total_gap = gap * chrom_order.len() as f64;
        let usable_angle = 2.0 * std::f64::consts::PI - total_gap;

        let mut chrom_angles: HashMap<String, (f64, f64)> = HashMap::new();
        let mut angle = 0.0f64;
        for (ci, (name, size)) in chrom_sizes.iter().enumerate() {
            let sweep = (*size / total_size) * usable_angle;
            let a1 = angle;
            let a2 = angle + sweep;
            chrom_angles.insert(name.clone(), (a1, a2));

            let color = PALETTE[ci % PALETTE.len()];
            let (x1, y1) = (cx + r_outer * a1.cos(), cy + r_outer * a1.sin());
            let (x2, y2) = (cx + r_outer * a2.cos(), cy + r_outer * a2.sin());
            let (x3, y3) = (cx + r_inner * a2.cos(), cy + r_inner * a2.sin());
            let (x4, y4) = (cx + r_inner * a1.cos(), cy + r_inner * a1.sin());
            let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
            c.elements.push(format!(
                r#"<path d="M {x1:.1},{y1:.1} A {ro:.1},{ro:.1} 0 {large} 1 {x2:.1},{y2:.1} L {x3:.1},{y3:.1} A {ri:.1},{ri:.1} 0 {large} 0 {x4:.1},{y4:.1} Z" fill="{color}" opacity="0.5" stroke="white" stroke-width="0.5" />"#,
                ro = r_outer, ri = r_inner
            ));

            let mid_a = (a1 + a2) / 2.0;
            let lx = cx + (r_outer + 14.0) * mid_a.cos();
            let ly = cy + (r_outer + 14.0) * mid_a.sin();
            let label = name.strip_prefix("chr").unwrap_or(name);
            c.add_text(lx, ly + 4.0, label, "middle", 8.0);

            angle = a2 + gap;
        }

        // Draw data track
        for (chrom, start, end, value) in &data_points {
            if let Some(&(a1, a2)) = chrom_angles.get(chrom) {
                let chrom_size = chrom_sizes
                    .iter()
                    .find(|(n, _)| n == chrom)
                    .map(|(_, s)| *s)
                    .unwrap_or(1.0);
                let ang_start = a1 + (*start / chrom_size) * (a2 - a1);
                let ang_end = a1 + (*end / chrom_size) * (a2 - a1);
                let t = if (val_range.1 - val_range.0).abs() < f64::EPSILON {
                    0.5
                } else {
                    (value - val_range.0) / (val_range.1 - val_range.0)
                };
                let t = t.clamp(0.0, 1.0);

                match track_type.as_str() {
                    "scatter" => {
                        let r_pt = r_track_inner + t * (r_track_outer - r_track_inner);
                        let mid_ang = (ang_start + ang_end) / 2.0;
                        let px = cx + r_pt * mid_ang.cos();
                        let py = cy + r_pt * mid_ang.sin();
                        c.add_circle(px, py, 2.0, "#e15759");
                    }
                    "line" => {
                        let r_pt = r_track_inner + t * (r_track_outer - r_track_inner);
                        let mid_ang = (ang_start + ang_end) / 2.0;
                        let px = cx + r_pt * mid_ang.cos();
                        let py = cy + r_pt * mid_ang.sin();
                        let base_x = cx + r_track_inner * mid_ang.cos();
                        let base_y = cy + r_track_inner * mid_ang.sin();
                        c.add_line(base_x, base_y, px, py, "#4e79a7", 1.0);
                    }
                    _ => {
                        let r_bar = r_track_inner + t * (r_track_outer - r_track_inner);
                        let (bx1, by1) = (
                            cx + r_track_inner * ang_start.cos(),
                            cy + r_track_inner * ang_start.sin(),
                        );
                        let (bx2, by2) =
                            (cx + r_bar * ang_start.cos(), cy + r_bar * ang_start.sin());
                        let (bx3, by3) = (cx + r_bar * ang_end.cos(), cy + r_bar * ang_end.sin());
                        let (bx4, by4) = (
                            cx + r_track_inner * ang_end.cos(),
                            cy + r_track_inner * ang_end.sin(),
                        );
                        let large_bar = if (ang_end - ang_start) > std::f64::consts::PI {
                            1
                        } else {
                            0
                        };
                        let color = sequential_color(t);
                        c.elements.push(format!(
                            r#"<path d="M {bx1:.1},{by1:.1} L {bx2:.1},{by2:.1} A {r_bar:.1},{r_bar:.1} 0 {large_bar} 1 {bx3:.1},{by3:.1} L {bx4:.1},{by4:.1} A {ri:.1},{ri:.1} 0 {large_bar} 0 {bx1:.1},{by1:.1} Z" fill="{color}" opacity="0.8" />"#,
                            ri = r_track_inner
                        ));
                    }
                }
            }
        }

        c.elements.push(format!(
            r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" fill="none" stroke="#ddd" stroke-width="0.5" />"##,
            r_track_inner
        ));

        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let bar_w = get_opt_usize(&opts, "width", 50);
    let max_name = chrom_sizes.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let mut out = format!("  {title} (linear view)\n");
    for (name, size) in &chrom_sizes {
        let len = (size / total_size * bar_w as f64).round() as usize;
        let n_pts = data_points.iter().filter(|(c, _, _, _)| c == name).count();
        let bar: String = "█".repeat(len.max(1));
        out.push_str(&format!(
            "  {:>w$}  {bar}  ({n_pts} points)\n",
            name,
            w = max_name
        ));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── umap_plot ────────────────────────────────────────────────────

/// Scatter plot for UMAP/PCA/t-SNE embeddings.
/// data: Table with columns x, y, and optionally color/label/cluster
/// options: Record{title?, width?, height?, color_col?, label_col?, format?}
/// Scree / elbow plot: variance explained by each principal component.
///
/// The figure you read to choose how many components to keep - it flattens
/// where the components stop carrying structure. Accepts either the list of
/// ratios or the whole record `sc_pca` returns, because passing that record
/// straight through is what a reader will try first.
fn builtin_elbow_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);

    let values: Vec<f64> = match &args[0] {
        Value::List(items) => items.iter().filter_map(|v| v.as_float()).collect(),
        Value::Record(map) => map
            .get("explained_variance_ratio")
            .or_else(|| map.get("explained_variance"))
            .map(|v| match v {
                Value::List(items) => items.iter().filter_map(|x| x.as_float()).collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default(),
        _ => return Err(BioLangError::type_error(
            "elbow_plot() requires a List of variance ratios, or the Record returned by sc_pca()",
            None,
        )),
    };

    if values.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "elbow_plot() found no variance values",
            None,
        ));
    }

    let title = get_opt_str(&opts, "title", "Scree plot").to_string();
    let width = get_opt_f64(&opts, "width", 600.0);
    let height = get_opt_f64(&opts, "height", 400.0);
    let mut canvas = SvgCanvas::new(width, height);

    let highest = values.iter().cloned().fold(f64::MIN, f64::max);
    let x_scale = Scale {
        domain: (0.5, values.len() as f64 + 0.5),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        // Always anchored at zero: a scree plot read on a truncated axis
        // exaggerates the elbow, which is the one thing it exists to show.
        domain: (0.0, highest * 1.1 + 1e-9),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    let points: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{:.1},{:.1}", x_scale.map(i as f64 + 1.0), y_scale.map(*v)))
        .collect();
    canvas.elements.push(format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
        points.join(" "),
        PALETTE[0]
    ));
    for (i, v) in values.iter().enumerate() {
        canvas.add_circle(
            x_scale.map(i as f64 + 1.0),
            y_scale.map(*v),
            4.0,
            PALETTE[0],
        );
    }

    canvas.draw_x_axis(&x_scale, "component");
    canvas.draw_y_axis(&y_scale, "variance explained");
    canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);

    Ok(Value::Str(canvas.render()))
}

/// Violin plot: the shape of a distribution, per group.
///
/// The figure QC is read from - nFeature, nCount, percent.mt across samples.
/// A boxplot draws five numbers and so cannot show bimodality (Chapter 5 of the
/// MSMB companion measures exactly that blind spot); a violin draws the whole
/// density, which is the point.
///
/// Density is estimated with the same Gaussian KDE and `bw.nrd0` bandwidth
/// convention exposed by `violin_data()`. The long-form input contract remains
/// distinct from `violin()`, which treats numeric table columns as groups.
fn builtin_violin_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    let theme = plot_theme(&opts);
    let seurat_theme = get_opt_str(&opts, "theme", "") == "seurat";
    let value_col = ["value", "value_col", "y"]
        .iter()
        .map(|key| get_opt_str(&opts, key, ""))
        .find(|v| !v.is_empty())
        .unwrap_or("value")
        .to_string();
    let group_col = ["group", "group_col", "color", "x"]
        .iter()
        .map(|key| get_opt_str(&opts, key, ""))
        .find(|v| !v.is_empty())
        .unwrap_or("group")
        .to_string();

    // Collect values per group, preserving first-seen order so the axis is
    // stable across runs rather than following hash order.
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<f64>> = HashMap::new();
    let mut push = |group: String, value: f64| {
        if !value.is_finite() {
            return;
        }
        if !groups.contains_key(&group) {
            order.push(group.clone());
        }
        groups.entry(group).or_default().push(value);
    };

    match &args[0] {
        Value::List(items) => {
            for item in items.iter() {
                match item {
                    Value::Record(map) => {
                        if let Some(value) = map.get(&value_col).and_then(|v| v.as_float()) {
                            let group = map
                                .get(&group_col)
                                .map(|v| format!("{v}"))
                                .unwrap_or_else(|| "all".to_string());
                            push(group, value);
                        }
                    }
                    // A bare list of numbers is one unnamed group.
                    other => {
                        if let Some(value) = other.as_float() {
                            push("all".to_string(), value);
                        }
                    }
                }
            }
        }
        Value::Table(table) => {
            let values = extract_table_col(table, &value_col).unwrap_or_default();
            let labels = extract_str_col(table, &group_col)
                .unwrap_or_else(|_| vec!["all".to_string(); values.len()]);
            for (i, value) in values.iter().enumerate() {
                push(
                    labels.get(i).cloned().unwrap_or_else(|| "all".to_string()),
                    *value,
                );
            }
        }
        _ => {
            return Err(BioLangError::type_error(
                "violin_plot() requires a Table or List of Records",
                None,
            ))
        }
    }

    if order.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin_plot() found no values",
            None,
        ));
    }

    let shapes = order
        .iter()
        .map(|name| violin_shape(name.clone(), &groups[name], 128))
        .collect::<Vec<_>>();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let mut spec_options = opts.clone();
        spec_options.insert("value_label".into(), Value::Str(value_col.clone()));
        let spec = violin_plot_spec_value(&shapes, "long", &value_col, &spec_options);
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_violin_plot_spec_value(&spec, &opts);
    }
    if matches!(format.as_str(), "svg" | "raw") {
        let mut render_options = opts.clone();
        render_options.insert("value_label".into(), Value::Str(value_col.clone()));
        return render_long_violin_svg(&shapes, &render_options).map(Value::Str);
    }

    let title = get_opt_str(&opts, "title", "Distribution").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let width = get_opt_f64(&opts, "width", 640.0);
    let height = get_opt_f64(&opts, "height", 420.0);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);

    let lo = shapes
        .iter()
        .filter_map(|shape| shape.points.first().map(|point| point.0))
        .fold(f64::INFINITY, f64::min);
    let hi = shapes
        .iter()
        .filter_map(|shape| shape.points.last().map(|point| point.0))
        .fold(f64::NEG_INFINITY, f64::max);

    if theme.is_adaptive() {
        let provisional = Scale {
            domain: (lo, hi),
            range: (1.0, 0.0),
        };
        let widest_tick = provisional
            .nice_ticks(5)
            .iter()
            .map(|tick| estimate_text_width(&format!("{tick:.2}"), theme.tick_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_tick + 42.0).clamp(54.0, width * 0.30);
        canvas.margin.right = 18.0_f64.min(width * 0.08);
        canvas.margin.top = if subtitle.is_empty() { 48.0 } else { 66.0 };

        let provisional_slot =
            (width - canvas.margin.left - canvas.margin.right).max(1.0) / order.len() as f64;
        let widest_group = order
            .iter()
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let rotate_labels = widest_group > provisional_slot * 0.86;
        let label_reserve = if rotate_labels {
            (widest_group * 0.72 + 12.0).clamp(32.0, height * 0.28)
        } else {
            28.0
        };
        canvas.margin.bottom = label_reserve + if caption.is_empty() { 12.0 } else { 28.0 };
    }

    let y_scale = Scale {
        domain: (lo, hi),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let slot = canvas.plot_width() / order.len() as f64;

    if theme.is_adaptive() {
        let left = canvas.margin.left;
        let right = left + canvas.plot_width();
        let top = canvas.margin.top;
        let bottom = top + canvas.plot_height();
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for tick in y_scale.nice_ticks(5) {
            let y = y_scale.map(tick);
            canvas.add_line(left, y, right, y, theme.grid_colour, theme.grid_width);
        }
        canvas.add_line(
            left,
            bottom,
            right,
            bottom,
            theme.axis_colour,
            theme.axis_width,
        );
    }

    let widest_group = order
        .iter()
        .map(|label| estimate_text_width(label, theme.tick_size))
        .fold(0.0, f64::max);
    let rotate_labels = theme.is_adaptive() && widest_group > slot * 0.86;

    for (gi, name) in order.iter().enumerate() {
        let values = &groups[name];
        let centre = canvas.margin.left + slot * (gi as f64 + 0.5);
        let shape = &shapes[gi].points;
        let peak = shape
            .iter()
            .map(|(_, density)| *density)
            .fold(f64::MIN, f64::max)
            .max(1e-9);
        let half = slot * 0.42;

        // The same Gaussian KDE exposed by violin_data(), mirrored around the
        // group centre. Rendering changes width into pixels but never changes
        // the scientific grid or bandwidth.
        let mut outline: Vec<String> = Vec::new();
        for (value, density) in shape {
            outline.push(format!(
                "{:.1},{:.1}",
                centre + density / peak * half,
                y_scale.map(*value)
            ));
        }
        for (value, density) in shape.iter().rev() {
            outline.push(format!(
                "{:.1},{:.1}",
                centre - density / peak * half,
                y_scale.map(*value)
            ));
        }
        let colour = if seurat_theme {
            SEURAT_PALETTE[gi % SEURAT_PALETTE.len()]
        } else {
            PALETTE[gi % PALETTE.len()]
        };
        canvas.elements.push(format!(
            r#"<polygon points="{}" fill="{}" opacity="0.65" stroke="{}" stroke-width="1" />"#,
            outline.join(" "),
            colour,
            if seurat_theme { "#333333" } else { colour }
        ));

        // The median, so the violin still carries the summary a boxplot would.
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = quantile_type7(&sorted, 0.5);
        let my = y_scale.map(median);
        canvas.add_line(
            centre - half * 0.5,
            my,
            centre + half * 0.5,
            my,
            "#333333",
            2.0,
        );

        let label_y = canvas.margin.top + canvas.plot_height() + 16.0;
        if rotate_labels {
            canvas.add_text_rotated(centre, label_y - 3.0, name, 40.0, "start", theme.tick_size);
        } else {
            canvas.add_text(
                centre,
                label_y,
                name,
                "middle",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }

    canvas.draw_y_axis(&y_scale, &value_col);
    canvas.set_accessible_description(format!(
        "Violin plot of {value_col} for {} groups; each shape is a Gaussian kernel density and each horizontal mark is the group median.",
        order.len()
    ));
    if theme.is_adaptive() {
        canvas.draw_title(&title);
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    } else {
        canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    }
    Ok(Value::Str(canvas.render()))
}

/// One point per gene: mean expression against dispersion, with the genes
/// `highly_variable_genes` keeps drawn on top.
///
/// Seurat calls this VariableFeaturePlot and it is normally read as a sanity
/// check - "did feature selection pick the markers?" - but its real value is
/// showing *where* on the expression range the selection landed. The dispersion
/// of a count is mean-dependent, so a selection rule that ignores the trend
/// picks whatever is rarest. That bug was live in this runtime: ranking by
/// variance/mean^2 chose lncRNAs seen in two cells out of 2700 and dropped LYZ,
/// MS4A1 and GNLY entirely. On this figure it is unmistakable - every
/// highlighted point sits jammed against the left edge.
///
/// So the default y-axis is the raw dispersion against a log mean, which shows
/// the trend and the selection together. `y: "normalised"` gives the
/// bin-standardised value that is actually ranked, which is the flatter,
/// Seurat-shaped view.
fn builtin_variable_feature_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let use_normalised = get_opt_str(&opts, "y", "dispersion").starts_with("norm");
    let n_selected = get_opt_usize(&opts, "n", 2000);
    let n_labels = get_opt_usize(&opts, "label", 10);

    // Gene names are optional; without them the points are still positioned
    // correctly, they just cannot be labelled.
    let gene_names: Vec<String> = match opts.get("genes") {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                other => format!("{other}"),
            })
            .collect(),
        _ => Vec::new(),
    };

    // Either the matrix itself - in which case the same code that selects the
    // genes computes the coordinates - or a table of statistics computed
    // elsewhere.
    let is_records =
        matches!(&args[0], Value::List(items) if matches!(items.first(), Some(Value::Record(_))));

    struct Gene {
        name: String,
        mean: f64,
        dispersion: f64,
        selected: bool,
        rank_key: f64,
    }

    let genes: Vec<Gene> = if matches!(&args[0], Value::Table(_)) || is_records {
        let mean_col = get_opt_str(&opts, "mean_col", "mean").to_string();
        let disp_col = get_opt_str(&opts, "dispersion_col", "dispersion").to_string();
        let name_col = get_opt_str(&opts, "gene_col", "gene").to_string();

        let mut rows: Vec<Gene> = Vec::new();
        match &args[0] {
            Value::Table(table) => {
                let means = extract_table_col(table, &mean_col).unwrap_or_default();
                let disps = extract_table_col(table, &disp_col)
                    .or_else(|_| extract_table_col(table, "variance"))
                    .unwrap_or_default();
                let names = extract_str_col(table, &name_col)
                    .unwrap_or_else(|_| vec![String::new(); means.len()]);
                for i in 0..means.len().min(disps.len()) {
                    rows.push(Gene {
                        name: names.get(i).cloned().unwrap_or_default(),
                        mean: means[i],
                        dispersion: disps[i],
                        selected: false,
                        rank_key: disps[i],
                    });
                }
            }
            Value::List(items) => {
                for item in items.iter() {
                    if let Value::Record(map) = item {
                        let mean = map.get(&mean_col).and_then(|v| v.as_float());
                        let disp = map
                            .get(&disp_col)
                            .or_else(|| map.get("variance"))
                            .and_then(|v| v.as_float());
                        if let (Some(mean), Some(disp)) = (mean, disp) {
                            rows.push(Gene {
                                name: map
                                    .get(&name_col)
                                    .map(|v| format!("{v}"))
                                    .unwrap_or_default(),
                                mean,
                                dispersion: disp,
                                selected: map
                                    .get("variable")
                                    .map(|v| v.is_truthy())
                                    .unwrap_or(false),
                                rank_key: disp,
                            });
                        }
                    }
                }
            }
            _ => unreachable!("guarded by the match above"),
        }

        // An explicit highlight list wins over a `variable` column, and is how
        // you draw a selection that was made by something other than
        // highly_variable_genes.
        if let Some(Value::List(items)) = opts.get("highlight") {
            let wanted: HashSet<String> = items
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    other => format!("{other}"),
                })
                .collect();
            for gene in rows.iter_mut() {
                gene.selected = wanted.contains(&gene.name);
            }
        } else if !rows.iter().any(|g| g.selected) && opts.contains_key("n") {
            // A table with no `variable` column and no highlight list carries no
            // selection, so only an explicit `n` asks for one. Defaulting to the
            // top 2000 would have highlighted every row of any smaller table -
            // a figure claiming that every gene is variable.
            let mut order: Vec<usize> = (0..rows.len()).collect();
            order.sort_by(|&a, &b| {
                rows[b]
                    .rank_key
                    .partial_cmp(&rows[a].rank_key)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for &i in order.iter().take(n_selected) {
                rows[i].selected = true;
            }
        }
        rows
    } else {
        let stats = crate::singlecell::hvg_statistics(&args[0], "variable_feature_plot")?;
        let chosen: HashSet<usize> = stats.select(n_selected).into_iter().collect();
        stats
            .expressed
            .iter()
            .enumerate()
            .map(|(position, &gene)| Gene {
                name: gene_names
                    .get(gene)
                    .cloned()
                    .unwrap_or_else(|| format!("gene{gene}")),
                mean: stats.means[gene],
                dispersion: if use_normalised {
                    stats.normalised[position]
                } else {
                    stats.dispersions[position]
                },
                selected: chosen.contains(&gene),
                rank_key: stats.normalised[position],
            })
            .collect()
    };

    if genes.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "variable_feature_plot() found no genes with a mean and a dispersion",
            None,
        ));
    }

    let title = get_opt_str(&opts, "title", "Variable features").to_string();
    let width = get_opt_f64(&opts, "width", 640.0);
    let height = get_opt_f64(&opts, "height", 460.0);
    let mut canvas = SvgCanvas::new(width, height);

    // Mean expression spans orders of magnitude, so on a linear axis every gene
    // collapses onto the left edge and the figure shows nothing.
    let xs: Vec<f64> = genes.iter().map(|g| g.mean.max(1e-6).log10()).collect();
    let ys: Vec<f64> = genes.iter().map(|g| g.dispersion).collect();
    let (x_lo, x_hi) = col_range(&xs);
    let (y_lo, y_hi) = col_range(&ys);
    let x_pad = (x_hi - x_lo) * 0.05 + 1e-3;
    let y_pad = (y_hi - y_lo) * 0.05 + 1e-3;

    let x_scale = Scale {
        domain: (x_lo - x_pad, x_hi + x_pad),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (y_lo - y_pad, y_hi + y_pad),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // Unselected first, so the selection is never buried under the cloud. The
    // two layers raster separately because they are drawn at different radii,
    // which still leaves two elements instead of tens of thousands.
    let raster = raster_choice(&opts, "variable_feature_plot", genes.len())?;
    let area = canvas.point_area();
    let background: Vec<(f64, f64, &str)> = genes
        .iter()
        .enumerate()
        .filter(|(_, gene)| !gene.selected)
        .map(|(i, _)| (x_scale.map(xs[i]), y_scale.map(ys[i]), "#bbbbbb"))
        .collect();
    canvas.add_scatter(&background, 1.6, area, raster);
    // Red on grey, as Seurat draws it - the selection has to read at a glance
    // against a cloud of tens of thousands of points.
    let selected: Vec<(f64, f64, &str)> = genes
        .iter()
        .enumerate()
        .filter(|(_, gene)| gene.selected)
        .map(|(i, _)| (x_scale.map(xs[i]), y_scale.map(ys[i]), PALETTE[2]))
        .collect();
    let n_variable = selected.len();
    canvas.add_scatter(&selected, 2.4, area, raster);

    // Label the strongest few. Any more and the labels cover the cloud they are
    // meant to explain.
    let mut ranked: Vec<usize> = (0..genes.len()).filter(|&i| genes[i].selected).collect();
    ranked.sort_by(|&a, &b| {
        genes[b]
            .rank_key
            .partial_cmp(&genes[a].rank_key)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for &i in ranked.iter().take(n_labels) {
        if genes[i].name.is_empty() {
            continue;
        }
        canvas.add_text(
            x_scale.map(xs[i]) + 4.0,
            y_scale.map(ys[i]) - 4.0,
            &genes[i].name,
            "start",
            8.0,
        );
    }

    canvas.draw_x_axis(&x_scale, "log10 mean expression");
    canvas.draw_y_axis(
        &y_scale,
        if use_normalised {
            "standardised dispersion"
        } else {
            "dispersion (variance / mean)"
        },
    );
    canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    canvas.add_text(
        canvas.margin.left,
        36.0,
        &format!("{n_variable} variable of {} genes", genes.len()),
        "start",
        10.0,
    );

    Ok(Value::Str(canvas.render()))
}

/// Genes against clusters, where a dot's size is how many cells express the
/// gene and its colour is how strongly.
///
/// Seurat's DotPlot, and the figure that actually settles cell-type calls. A
/// feature plot shows one gene at a time and a heatmap of means hides how many
/// cells are behind each one - which matters, because a gene blazing in 5% of a
/// cluster and one steady across 90% give the same mean and mean opposite
/// things. Encoding both is the whole point, so both are drawn: area for
/// detection rate, colour for level.
///
/// Colour is the mean expression z-scored per gene across clusters, as Seurat
/// scales it. Without that a housekeeping gene at high absolute expression
/// washes out every marker on the plot; with it, each row says where that gene
/// is relatively highest, which is the question being asked.
fn render_dot_plot_geometry_svg(
    gene_names: &[String],
    cluster_names: &[String],
    detected: &[Vec<f64>],
    scaled: &[Vec<f64>],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    const CLIP: f64 = 2.5;
    let title = get_opt_str(opts, "title", "Marker expression").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let theme = plot_theme(opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let cell = get_opt_f64(opts, "cell", 26.0);
    let requested_width = get_opt_f64(opts, "width", 0.0);
    let requested_height = get_opt_f64(opts, "height", 0.0);
    let widest_gene = gene_names
        .iter()
        .map(|gene| estimate_text_width(gene, theme.tick_size))
        .fold(0.0, f64::max);
    let widest_cluster = cluster_names
        .iter()
        .map(|cluster| estimate_text_width(cluster, theme.tick_size))
        .fold(0.0, f64::max);
    let adaptive_left = (widest_gene + 14.0).clamp(64.0, 180.0);
    let adaptive_top =
        (54.0 + if subtitle.is_empty() { 0.0 } else { 18.0 } + widest_cluster * 0.68)
            .clamp(72.0, 132.0);
    let adaptive_right = 112.0;
    let adaptive_bottom = if caption.is_empty() { 16.0 } else { 32.0 };
    let width = if requested_width > 0.0 {
        requested_width
    } else if theme.is_adaptive() {
        adaptive_left + cell * cluster_names.len() as f64 + adaptive_right
    } else {
        180.0 + cell * cluster_names.len() as f64 + 120.0
    };
    let height = if requested_height > 0.0 {
        requested_height
    } else if theme.is_adaptive() {
        adaptive_top + cell * gene_names.len() as f64 + adaptive_bottom
    } else {
        90.0 + cell * gene_names.len() as f64
    };
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    if theme.is_adaptive() {
        canvas.margin.left = adaptive_left.min(width * 0.34);
        canvas.margin.right = adaptive_right.min(width * 0.36);
        canvas.margin.top = adaptive_top.min(height * 0.38);
        canvas.margin.bottom = adaptive_bottom.min(height * 0.15);
    }
    let left = if theme.is_adaptive() {
        canvas.margin.left
    } else {
        130.0
    };
    let top = if theme.is_adaptive() {
        canvas.margin.top
    } else {
        60.0
    };
    let cell_x = if theme.is_adaptive() {
        canvas.plot_width() / cluster_names.len().max(1) as f64
    } else {
        cell
    };
    let cell_y = if theme.is_adaptive() {
        canvas.plot_height() / gene_names.len().max(1) as f64
    } else {
        cell
    };
    let radius_max = if theme.is_adaptive() {
        (cell_x.min(cell_y) * 0.40).min(12.0)
    } else {
        cell * 0.42
    };

    if theme.is_adaptive() {
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for column in 0..=cluster_names.len() {
            let x = left + cell_x * column as f64;
            canvas.add_line(
                x,
                top,
                x,
                top + canvas.plot_height(),
                theme.grid_colour,
                theme.grid_width,
            );
        }
        for row in 0..=gene_names.len() {
            let y = top + cell_y * row as f64;
            canvas.add_line(
                left,
                y,
                left + canvas.plot_width(),
                y,
                theme.grid_colour,
                theme.grid_width,
            );
        }
    }
    for (column, cluster) in cluster_names.iter().enumerate() {
        let x = left + cell_x * (column as f64 + 0.5);
        canvas.add_text_rotated(
            x,
            top - 10.0,
            cluster,
            -45.0,
            "start",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
    }
    for (row, gene) in gene_names.iter().enumerate() {
        let y = top + cell_y * (row as f64 + 0.5);
        canvas.add_text(
            left - 8.0,
            y + 3.0,
            gene,
            "end",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
        for column in 0..cluster_names.len() {
            let fraction = detected[row][column];
            if fraction <= 0.0 {
                continue;
            }
            let x = left + cell_x * (column as f64 + 0.5);
            let radius = radius_max * fraction.sqrt();
            let t = (scaled[row][column] + CLIP) / (2.0 * CLIP);
            let colour = if publication_theme {
                publication_diverging_color(t)
            } else {
                sequential_color(t)
            };
            canvas.add_circle(x, y, radius, &colour);
        }
    }
    let legend_x =
        left + if theme.is_adaptive() {
            canvas.plot_width()
        } else {
            cell * cluster_names.len() as f64
        } + if theme.is_adaptive() { 16.0 } else { 24.0 };
    let legend_size = if theme.is_adaptive() {
        theme.legend_size
    } else {
        9.0
    };
    canvas.add_text(legend_x, top + 4.0, "% detected", "start", legend_size);
    for (i, fraction) in [0.25_f64, 0.5, 1.0].iter().enumerate() {
        let y = top + 22.0 + i as f64 * if theme.is_adaptive() { 25.0 } else { 18.0 };
        canvas.add_circle(legend_x + 8.0, y, radius_max * fraction.sqrt(), "#888888");
        canvas.add_text(
            legend_x + 22.0,
            y + 3.0,
            &format!("{:.0}%", fraction * 100.0),
            "start",
            8.0,
        );
    }
    let bar_top = top + if theme.is_adaptive() { 112.0 } else { 90.0 };
    canvas.add_text(legend_x, bar_top - 6.0, "z-score", "start", legend_size);
    for step in 0..24 {
        let t = 1.0 - step as f64 / 23.0;
        let colour = if publication_theme {
            publication_diverging_color(t)
        } else {
            sequential_color(t)
        };
        canvas.add_rect(legend_x, bar_top + step as f64 * 3.0, 10.0, 3.4, &colour);
    }
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 6.0,
        &format!("{CLIP:.1}"),
        "start",
        8.0,
    );
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 72.0,
        &format!("{:.1}", -CLIP),
        "start",
        8.0,
    );
    canvas.set_accessible_description(format!(
        "Single-cell dot plot for {} genes across {} clusters. Dot area encodes the percentage of detected cells and colour encodes per-gene z-scored mean expression.",
        gene_names.len(),
        cluster_names.len()
    ));
    if theme.is_adaptive() {
        canvas.draw_title(&title);
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    } else {
        canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    }
    Ok(canvas.render())
}

fn dot_plot_spec_value(
    gene_names: &[String],
    cluster_names: &[String],
    means: &[Vec<f64>],
    detected: &[Vec<f64>],
    scaled: &[Vec<f64>],
    input_cells: usize,
    opts: &HashMap<String, Value>,
) -> Value {
    let rows = gene_names
        .iter()
        .enumerate()
        .flat_map(|(gene_index, gene)| {
            cluster_names
                .iter()
                .enumerate()
                .map(move |(cluster_index, cluster)| {
                    vec![
                        Value::Int(gene_index as i64),
                        Value::Str(gene.clone()),
                        Value::Int(cluster_index as i64),
                        Value::Str(cluster.clone()),
                        Value::Float(means[gene_index][cluster_index]),
                        Value::Float(detected[gene_index][cluster_index]),
                        Value::Float(scaled[gene_index][cluster_index]),
                    ]
                })
        })
        .collect();
    let options = HashMap::from([
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", "Marker expression").into()),
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
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        ("cell".into(), Value::Float(get_opt_f64(opts, "cell", 26.0))),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 0.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 0.0)),
        ),
        ("z_score_clip".into(), Value::Float(2.5)),
    ]);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("dot_plot".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "Marker expression").into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "gene_index",
                        "gene",
                        "cluster_index",
                        "cluster",
                        "mean_expression",
                        "detected_fraction",
                        "scaled_expression",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("dot_plot".into())),
                        ("input_cells".into(), Value::Int(input_cells as i64)),
                        ("genes".into(), Value::Int(gene_names.len() as i64)),
                        ("clusters".into(), Value::Int(cluster_names.len() as i64)),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

pub(crate) fn is_dot_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "dot_plot")
    )
}

pub(crate) fn render_dot_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_dot_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 dot-plot Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "gene_index",
        "gene",
        "cluster_index",
        "cluster",
        "mean_expression",
        "detected_fraction",
        "scaled_expression",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() dot-plot data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| table.col_index(name).unwrap();
    let mut gene_names = Vec::<String>::new();
    let mut cluster_names = Vec::<String>::new();
    for row in &table.rows {
        let gene = row[column("gene_index")]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() dot-plot gene_index must be numeric",
                    None,
                )
            })?;
        let cluster = row[column("cluster_index")]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() dot-plot cluster_index must be numeric",
                    None,
                )
            })?;
        if gene == gene_names.len() {
            gene_names.push(format!("{}", row[column("gene")]));
        } else if gene > gene_names.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot gene indices must be contiguous",
                None,
            ));
        }
        if gene == 0 && cluster == cluster_names.len() {
            cluster_names.push(format!("{}", row[column("cluster")]));
        }
    }
    if gene_names.is_empty() || cluster_names.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() dot-plot specification is empty",
            None,
        ));
    }
    let expected = gene_names.len() * cluster_names.len();
    if table.num_rows() != expected {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() dot-plot data must contain one row per gene-cluster pair",
            None,
        ));
    }
    let mut means = vec![vec![0.0; cluster_names.len()]; gene_names.len()];
    let mut detected = means.clone();
    let mut scaled = means.clone();
    for (expected_row, row) in table.rows.iter().enumerate() {
        let gene = row[column("gene_index")].as_float().unwrap() as usize;
        let cluster = row[column("cluster_index")].as_float().unwrap() as usize;
        if expected_row != gene * cluster_names.len() + cluster
            || gene >= gene_names.len()
            || cluster >= cluster_names.len()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot rows must be ordered by gene and cluster index",
                None,
            ));
        }
        let number = |name: &str| -> Result<f64> {
            let value = row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() dot-plot field '{name}' must be numeric"),
                    None,
                )
            })?;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() dot-plot field '{name}' must be finite"),
                    None,
                ))
            }
        };
        means[gene][cluster] = number("mean_expression")?;
        detected[gene][cluster] = number("detected_fraction")?;
        scaled[gene][cluster] = number("scaled_expression")?;
        if !(0.0..=1.0).contains(&detected[gene][cluster]) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot detected_fraction must lie between zero and one",
                None,
            ));
        }
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot specification field 'options' must be Record",
                None,
            ))
        }
    };
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in ["width", "height"] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    let svg =
        render_dot_plot_geometry_svg(&gene_names, &cluster_names, &detected, &scaled, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Dot plot");
    match format.as_str() {
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
            "render_plot() terminal dot-plot output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown dot-plot format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_dot_plot(args: Vec<Value>) -> Result<Value> {
    let opts: HashMap<String, Value> = match args.get(2) {
        Some(Value::Record(map)) => map.as_ref().clone(),
        _ => HashMap::new(),
    };

    let (n_cells, n_genes, columns) = crate::singlecell::expression_columns(&args[0], "dot_plot")?;

    let labels: Vec<String> = match &args[1] {
        Value::List(items) => items.iter().map(|v| format!("{v}")).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "dot_plot() requires a List of cluster labels, one per cell",
                None,
            ))
        }
    };
    if labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "dot_plot(): {} cluster labels for {n_cells} cells",
                labels.len()
            ),
            None,
        ));
    }

    let gene_names: Vec<String> = match opts.get("genes") {
        Some(Value::List(items)) => items.iter().map(|v| format!("{v}")).collect(),
        _ => (0..n_genes).map(|g| format!("gene{g}")).collect(),
    };

    // Which genes to draw. Named features keep the caller's order, because a
    // dot plot is usually read as a story - lineage by lineage.
    let selected: Vec<usize> = match opts.get("features").or_else(|| opts.get("markers")) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Int(i) if (*i as usize) < n_genes => Some(*i as usize),
                other => {
                    let wanted = format!("{other}");
                    gene_names.iter().position(|name| *name == wanted)
                }
            })
            .collect(),
        _ => (0..n_genes).collect(),
    };
    if selected.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "dot_plot() found none of the requested features".to_string(),
            None,
        ));
    }

    let mut cluster_order: Vec<String> = Vec::new();
    let mut members: HashMap<String, Vec<usize>> = HashMap::new();
    for (cell, label) in labels.iter().enumerate() {
        if !members.contains_key(label) {
            cluster_order.push(label.clone());
        }
        members.entry(label.clone()).or_default().push(cell);
    }

    // mean expression and detection rate, per gene per cluster.
    let mut means = vec![vec![0.0_f64; cluster_order.len()]; selected.len()];
    let mut detected = vec![vec![0.0_f64; cluster_order.len()]; selected.len()];
    for (row, &gene) in selected.iter().enumerate() {
        let values = &columns[gene];
        for (column, cluster) in cluster_order.iter().enumerate() {
            let cells = &members[cluster];
            if cells.is_empty() {
                continue;
            }
            let mut total = 0.0;
            let mut expressing = 0usize;
            for &cell in cells {
                let value = values[cell];
                total += value;
                if value > 0.0 {
                    expressing += 1;
                }
            }
            means[row][column] = total / cells.len() as f64;
            detected[row][column] = expressing as f64 / cells.len() as f64;
        }
    }

    // z-score each gene across clusters, clipped as Seurat clips it so one
    // extreme cluster cannot flatten the rest of the row.
    const CLIP: f64 = 2.5;
    let scaled: Vec<Vec<f64>> = means
        .iter()
        .map(|row| {
            let n = row.len() as f64;
            let mean = row.iter().sum::<f64>() / n;
            let sd = (row.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
            row.iter()
                .map(|v| {
                    if sd > 1e-12 {
                        ((v - mean) / sd).clamp(-CLIP, CLIP)
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();

    let selected_gene_names = selected
        .iter()
        .map(|&gene| gene_names[gene].clone())
        .collect::<Vec<_>>();
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = dot_plot_spec_value(
            &selected_gene_names,
            &cluster_order,
            &means,
            &detected,
            &scaled,
            n_cells,
            &opts,
        );
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_dot_plot_spec_value(&spec, &opts);
    }
    if matches!(format.as_str(), "svg" | "raw") {
        return render_dot_plot_geometry_svg(
            &selected_gene_names,
            &cluster_order,
            &detected,
            &scaled,
            &opts,
        )
        .map(Value::Str);
    }

    let title = get_opt_str(&opts, "title", "Marker expression").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let cell = get_opt_f64(&opts, "cell", 26.0);
    let requested_width = get_opt_f64(&opts, "width", 0.0);
    let requested_height = get_opt_f64(&opts, "height", 0.0);
    let widest_gene = selected
        .iter()
        .map(|&gene| estimate_text_width(&gene_names[gene], theme.tick_size))
        .fold(0.0, f64::max);
    let widest_cluster = cluster_order
        .iter()
        .map(|cluster| estimate_text_width(cluster, theme.tick_size))
        .fold(0.0, f64::max);
    let adaptive_left = (widest_gene + 14.0).clamp(64.0, 180.0);
    let adaptive_top =
        (54.0 + if subtitle.is_empty() { 0.0 } else { 18.0 } + widest_cluster * 0.68)
            .clamp(72.0, 132.0);
    let adaptive_right = 112.0;
    let adaptive_bottom = if caption.is_empty() { 16.0 } else { 32.0 };
    // Sized to the grid unless told otherwise: a fixed canvas either crushes 30
    // genes together or strands 3 in a corner.
    let width = if requested_width > 0.0 {
        requested_width
    } else if theme.is_adaptive() {
        adaptive_left + cell * cluster_order.len() as f64 + adaptive_right
    } else {
        180.0 + cell * cluster_order.len() as f64 + 120.0
    };
    let height = if requested_height > 0.0 {
        requested_height
    } else if theme.is_adaptive() {
        adaptive_top + cell * selected.len() as f64 + adaptive_bottom
    } else {
        90.0 + cell * selected.len() as f64
    };
    let mut canvas = SvgCanvas::with_theme(width, height, theme);

    if theme.is_adaptive() {
        canvas.margin.left = adaptive_left.min(width * 0.34);
        canvas.margin.right = adaptive_right.min(width * 0.36);
        canvas.margin.top = adaptive_top.min(height * 0.38);
        canvas.margin.bottom = adaptive_bottom.min(height * 0.15);
    }

    let left = if theme.is_adaptive() {
        canvas.margin.left
    } else {
        130.0
    };
    let top = if theme.is_adaptive() {
        canvas.margin.top
    } else {
        60.0
    };
    let cell_x = if theme.is_adaptive() {
        canvas.plot_width() / cluster_order.len().max(1) as f64
    } else {
        cell
    };
    let cell_y = if theme.is_adaptive() {
        canvas.plot_height() / selected.len().max(1) as f64
    } else {
        cell
    };
    let radius_max = if theme.is_adaptive() {
        // Sparse panels can have very large cells. Dot area still represents
        // detection within each cell, but the maximum mark must not expand to
        // fill it: that overwhelms labels and makes the size key collide.
        (cell_x.min(cell_y) * 0.40).min(12.0)
    } else {
        cell * 0.42
    };

    if theme.is_adaptive() {
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for column in 0..=cluster_order.len() {
            let x = left + cell_x * column as f64;
            canvas.add_line(
                x,
                top,
                x,
                top + canvas.plot_height(),
                theme.grid_colour,
                theme.grid_width,
            );
        }
        for row in 0..=selected.len() {
            let y = top + cell_y * row as f64;
            canvas.add_line(
                left,
                y,
                left + canvas.plot_width(),
                y,
                theme.grid_colour,
                theme.grid_width,
            );
        }
    }

    for (column, cluster) in cluster_order.iter().enumerate() {
        let x = left + cell_x * (column as f64 + 0.5);
        canvas.add_text_rotated(
            x,
            top - 10.0,
            cluster,
            -45.0,
            "start",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
    }

    for (row, &gene) in selected.iter().enumerate() {
        let y = top + cell_y * (row as f64 + 0.5);
        canvas.add_text(
            left - 8.0,
            y + 3.0,
            &gene_names[gene],
            "end",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
        for column in 0..cluster_order.len() {
            let x = left + cell_x * (column as f64 + 0.5);
            let fraction = detected[row][column];
            if fraction <= 0.0 {
                continue;
            }
            // Area, not radius, tracks the fraction: a radius-linear dot at 50%
            // reads as a quarter of the ink, which under-sells every mid-range
            // gene on the plot.
            let radius = radius_max * fraction.sqrt();
            let t = (scaled[row][column] + CLIP) / (2.0 * CLIP);
            let colour = if publication_theme {
                publication_diverging_color(t)
            } else {
                sequential_color(t)
            };
            canvas.add_circle(x, y, radius, &colour);
        }
    }

    // Two legends, because the figure carries two encodings and a reader cannot
    // guess either.
    let legend_x =
        left + if theme.is_adaptive() {
            canvas.plot_width()
        } else {
            cell * cluster_order.len() as f64
        } + if theme.is_adaptive() { 16.0 } else { 24.0 };
    let legend_size = if theme.is_adaptive() {
        theme.legend_size
    } else {
        9.0
    };
    canvas.add_text(legend_x, top + 4.0, "% detected", "start", legend_size);
    for (i, fraction) in [0.25_f64, 0.5, 1.0].iter().enumerate() {
        let y = top + 22.0 + i as f64 * if theme.is_adaptive() { 25.0 } else { 18.0 };
        canvas.add_circle(legend_x + 8.0, y, radius_max * fraction.sqrt(), "#888888");
        canvas.add_text(
            legend_x + 22.0,
            y + 3.0,
            &format!("{:.0}%", fraction * 100.0),
            "start",
            8.0,
        );
    }
    let bar_top = top + if theme.is_adaptive() { 112.0 } else { 90.0 };
    canvas.add_text(legend_x, bar_top - 6.0, "z-score", "start", legend_size);
    for step in 0..24 {
        let t = 1.0 - step as f64 / 23.0;
        let colour = if publication_theme {
            publication_diverging_color(t)
        } else {
            sequential_color(t)
        };
        canvas.add_rect(legend_x, bar_top + step as f64 * 3.0, 10.0, 3.4, &colour);
    }
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 6.0,
        &format!("{CLIP:.1}"),
        "start",
        8.0,
    );
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 72.0,
        &format!("{:.1}", -CLIP),
        "start",
        8.0,
    );

    canvas.set_accessible_description(format!(
        "Single-cell dot plot for {} genes across {} clusters. Dot area encodes the percentage of detected cells and colour encodes per-gene z-scored mean expression.",
        selected.len(),
        cluster_order.len()
    ));
    if theme.is_adaptive() {
        canvas.draw_title(&title);
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    } else {
        canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    }
    Ok(Value::Str(canvas.render()))
}

fn expand_equal_aspect_domains(
    x: (f64, f64),
    y: (f64, f64),
    panel_width: f64,
    panel_height: f64,
) -> ((f64, f64), (f64, f64)) {
    let x_span = (x.1 - x.0).abs().max(1e-12);
    let y_span = (y.1 - y.0).abs().max(1e-12);
    let panel_ratio = panel_width.max(1.0) / panel_height.max(1.0);
    let domain_ratio = x_span / y_span;
    if domain_ratio < panel_ratio {
        let wanted = y_span * panel_ratio;
        let centre = (x.0 + x.1) / 2.0;
        ((centre - wanted / 2.0, centre + wanted / 2.0), y)
    } else {
        let wanted = x_span / panel_ratio;
        let centre = (y.0 + y.1) / 2.0;
        (x, (centre - wanted / 2.0, centre + wanted / 2.0))
    }
}

fn finite_quantile(values: &[f64], probability: f64) -> Option<f64> {
    let mut finite = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if finite.is_empty() {
        return None;
    }
    finite.sort_by(f64::total_cmp);
    Some(quantile_type7(&finite, probability.clamp(0.0, 1.0)))
}

/// FeaturePlot-compatible cutoff: a number, or `q05`/`q95` for a quantile.
fn feature_cutoff(
    opts: &HashMap<String, Value>,
    key: &str,
    values: &[f64],
    fallback: f64,
) -> Result<f64> {
    match opts.get(key) {
        None => Ok(fallback),
        Some(value) if value.as_float().is_some_and(f64::is_finite) => {
            Ok(value.as_float().unwrap())
        }
        Some(Value::Str(value)) => {
            let lower = value.to_ascii_lowercase();
            let Some(percent) = lower.strip_prefix('q') else {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "umap_plot() option '{key}' must be a number or quantile such as 'q05'"
                    ),
                    None,
                ));
            };
            let probability = percent.parse::<f64>().map_err(|_| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("umap_plot() option '{key}' has an invalid quantile '{value}'"),
                    None,
                )
            })? / 100.0;
            if !(0.0..=1.0).contains(&probability) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("umap_plot() option '{key}' quantile must be between q00 and q100"),
                    None,
                ));
            }
            finite_quantile(values, probability).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("umap_plot() cannot apply '{key}' to an empty feature"),
                    None,
                )
            })
        }
        Some(_) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("umap_plot() option '{key}' must be a number or quantile such as 'q05'"),
            None,
        )),
    }
}

fn group_legend_reserve(groups: &[String], height: f64, theme: PlotTheme) -> (usize, f64) {
    if groups.is_empty() || !theme.is_adaptive() {
        return (
            groups.len().max(1),
            if groups.is_empty() { 0.0 } else { 120.0 },
        );
    }
    let rows = (((height - 110.0).max(36.0) / 18.0).floor() as usize).max(1);
    let columns = groups.len().div_ceil(rows);
    let mut width = 0.0;
    for column in 0..columns {
        let start = column * rows;
        let end = (start + rows).min(groups.len());
        let widest = groups[start..end]
            .iter()
            .map(|label| estimate_text_width(label, theme.legend_size))
            .fold(0.0, f64::max);
        width += (widest + 34.0).clamp(76.0, 150.0);
    }
    (rows, width + 8.0)
}

fn embedding_plot_spec_value(
    opts: &HashMap<String, Value>,
    xs: &[f64],
    ys: &[f64],
    groups: &[String],
    labels: &[String],
    feature_values: &[f64],
    has_feature: bool,
    feature_label: &str,
    feature_range: (f64, f64),
    publication_theme: bool,
) -> Result<Value> {
    let point_count = xs.len().min(ys.len());
    let renderable_points = (0..point_count)
        .filter(|&index| xs[index].is_finite() && ys[index].is_finite())
        .count();
    let raster = raster_choice(opts, "umap_plot", renderable_points)?;
    let mut draw_order: Vec<usize> = (0..point_count)
        .filter(|&index| xs[index].is_finite() && ys[index].is_finite())
        .collect();
    if has_feature && publication_theme {
        draw_order.sort_by(|&a, &b| {
            match (feature_values[a].is_finite(), feature_values[b].is_finite()) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => feature_values[a].total_cmp(&feature_values[b]),
            }
        });
    }
    let mut draw_rank = vec![None; point_count];
    for (rank, &source_row) in draw_order.iter().enumerate() {
        draw_rank[source_row] = Some(rank);
    }

    let rows = (0..point_count)
        .map(|index| {
            vec![
                Value::Int(index as i64),
                draw_rank[index]
                    .map(|rank| Value::Int(rank as i64))
                    .unwrap_or(Value::Nil),
                Value::Float(xs[index]),
                Value::Float(ys[index]),
                Value::Str(groups.get(index).cloned().unwrap_or_default()),
                Value::Str(labels.get(index).cloned().unwrap_or_default()),
                if has_feature {
                    feature_values
                        .get(index)
                        .copied()
                        .filter(|value| value.is_finite())
                        .map(Value::Float)
                        .unwrap_or(Value::Nil)
                } else {
                    Value::Nil
                },
            ]
        })
        .collect::<Vec<_>>();
    let data = Value::Table(Table::new(
        [
            "source_row",
            "draw_rank",
            "x",
            "y",
            "group",
            "label",
            "feature",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        rows,
    ));

    let title = get_opt_str(opts, "title", "UMAP").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let theme_name = get_opt_str(opts, "theme", "biolang").to_string();
    let equal_aspect = match opts.get("aspect") {
        Some(Value::Str(value)) => value.eq_ignore_ascii_case("equal"),
        Some(Value::Bool(value)) => *value,
        _ => publication_theme,
    };
    let label_groups = opts
        .get("label_groups")
        .or_else(|| opts.get("label"))
        .is_some_and(Value::is_truthy);
    let mut replay_options = HashMap::from([
        ("title".into(), Value::Str(title.clone())),
        ("subtitle".into(), Value::Str(subtitle.clone())),
        ("caption".into(), Value::Str(caption.clone())),
        ("theme".into(), Value::Str(theme_name.clone())),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 600.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 450.0)),
        ),
        (
            "point_size".into(),
            Value::Float(get_opt_f64(opts, "point_size", 3.0).max(0.1)),
        ),
        (
            "aspect".into(),
            Value::Str(if equal_aspect { "equal" } else { "free" }.into()),
        ),
        ("label_groups".into(), Value::Bool(label_groups)),
        ("raster".into(), Value::Bool(raster.enabled)),
        ("raster_scale".into(), Value::Float(raster.scale)),
    ]);
    if groups.iter().any(|group| !group.is_empty()) {
        replay_options.insert("color_col".into(), Value::Str("group".into()));
    }
    if labels.iter().any(|label| !label.is_empty()) {
        replay_options.insert("label_col".into(), Value::Str("label".into()));
    }
    if has_feature {
        replay_options.insert("feature".into(), Value::Str("feature".into()));
        replay_options.insert(
            "feature_label".into(),
            Value::Str(feature_label.to_string()),
        );
        replay_options.insert("min_cutoff".into(), Value::Float(feature_range.0));
        replay_options.insert("max_cutoff".into(), Value::Float(feature_range.1));
    }
    for key in ["na_color", "xlab", "ylab"] {
        if let Some(value) = opts.get(key) {
            replay_options.insert(key.to_string(), value.clone());
        }
    }

    let non_finite_coordinates = xs
        .iter()
        .zip(ys.iter())
        .filter(|(x, y)| !x.is_finite() || !y.is_finite())
        .count();
    let provenance = Value::Record(
        HashMap::from([
            ("builtin".into(), Value::Str("umap_plot".into())),
            ("input_rows".into(), Value::Int(point_count as i64)),
            (
                "rendered_points".into(),
                Value::Int(renderable_points as i64),
            ),
            (
                "non_finite_coordinates".into(),
                Value::Int(non_finite_coordinates as i64),
            ),
            (
                "feature".into(),
                if has_feature {
                    Value::Str(feature_label.to_string())
                } else {
                    Value::Nil
                },
            ),
        ])
        .into(),
    );
    let warnings = if non_finite_coordinates == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{non_finite_coordinates} points have non-finite coordinates"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("embedding".into())),
            ("title".into(), Value::Str(title)),
            ("subtitle".into(), Value::Str(subtitle)),
            ("caption".into(), Value::Str(caption)),
            ("theme".into(), Value::Str(theme_name)),
            ("data".into(), data),
            ("options".into(), Value::Record(replay_options.into())),
            ("provenance".into(), provenance),
            ("warnings".into(), Value::List(warnings.into())),
        ])
        .into(),
    ))
}

pub(crate) fn is_embedding_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "embedding")
    )
}

pub(crate) fn render_embedding_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_embedding_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 embedding Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => Value::Table(table.clone()),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() embedding specification field 'data' must be Table",
                None,
            ))
        }
    };
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() embedding specification field 'options' must be Record",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "draw_rank",
        "x",
        "y",
        "group",
        "label",
        "feature",
    ] {
        let Value::Table(table) = &data else {
            unreachable!()
        };
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() embedding data is missing '{required}'"),
                None,
            ));
        }
    }

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
            options.insert(key.to_string(), override_value.clone());
        }
    }
    options.insert("format".into(), Value::Str("svg".into()));
    let svg = match builtin_umap_plot(vec![data, Value::Record(options.into())])? {
        Value::Str(svg) => svg,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() embedding renderer did not return SVG",
                None,
            ))
        }
    };
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Embedding");
    match format.as_str() {
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
            "render_plot() terminal embedding output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown embedding format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_umap_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "UMAP").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let seurat_theme = get_opt_str(&opts, "theme", "") == "seurat";
    let label_groups = opts
        .get("label_groups")
        .or_else(|| opts.get("label"))
        .is_some_and(Value::is_truthy);
    let point_radius = get_opt_f64(&opts, "point_size", 3.0).max(0.1);
    // `color` is what everyone reaches for; `color_col` is the documented name.
    // Accepting only the latter meant `umap_plot(pts, { color: "cluster" })` was
    // silently ignored and every point came out one colour.
    let color_col = {
        let explicit = get_opt_str(&opts, "color_col", "");
        if explicit.is_empty() {
            get_opt_str(&opts, "color", "").to_string()
        } else {
            explicit.to_string()
        }
    };

    const CLUSTER_COLUMNS: [&str; 5] = ["cluster", "group", "label", "color", "cell_type"];

    let color_col = if color_col.is_empty() {
        // Auto-detect a cluster column. This used to run for Table only, so the
        // same data passed as a List of Records - which the extraction below
        // handles perfectly well - rendered every cell in one colour with no
        // error. On PBMC3k that was 1 colour for 11 clusters, and nothing said
        // so.
        match &args[0] {
            Value::Table(t) => CLUSTER_COLUMNS
                .iter()
                .find(|&&c| t.col_index(c).is_some())
                .map(|&s| s.to_string())
                .unwrap_or_default(),
            Value::List(items) => items
                .iter()
                .find_map(|item| match item {
                    Value::Record(map) => CLUSTER_COLUMNS
                        .iter()
                        .find(|&&c| map.contains_key(c))
                        .map(|&s| s.to_string()),
                    _ => None,
                })
                .unwrap_or_default(),
            _ => String::new(),
        }
    } else {
        color_col
    };
    let label_col = get_opt_str(&opts, "label_col", "").to_string();

    // Extract x, y, color_labels, point_labels from Table or List<Record>
    let (xs, ys, color_labels, point_labels): (Vec<f64>, Vec<f64>, Vec<String>, Vec<String>) =
        match &args[0] {
            Value::Table(table) => {
                let x_col = ["x", "UMAP1", "umap1", "PC1", "pc1", "tSNE1", "tsne1"]
                    .iter()
                    .find(|&&c| table.col_index(c).is_some())
                    .copied()
                    .unwrap_or("x");
                let y_col = ["y", "UMAP2", "umap2", "PC2", "pc2", "tSNE2", "tsne2"]
                    .iter()
                    .find(|&&c| table.col_index(c).is_some())
                    .copied()
                    .unwrap_or("y");
                let xs = extract_table_col(table, x_col).unwrap_or_default();
                let ys = extract_table_col(table, y_col).unwrap_or_default();
                let cls = if !color_col.is_empty() {
                    extract_str_col(table, &color_col)
                        .unwrap_or_else(|_| vec![String::new(); xs.len()])
                } else {
                    vec![String::new(); xs.len()]
                };
                let lbls = if !label_col.is_empty() {
                    extract_str_col(table, &label_col)
                        .unwrap_or_else(|_| vec![String::new(); xs.len()])
                } else {
                    vec![String::new(); xs.len()]
                };
                (xs, ys, cls, lbls)
            }
            Value::List(items) => {
                let mut xs = Vec::new();
                let mut ys = Vec::new();
                let mut cls = Vec::new();
                let mut lbls = Vec::new();
                for item in items.iter() {
                    if let Value::Record(map) = item {
                        let x = map
                            .get("x")
                            .or(map.get("UMAP1"))
                            .or(map.get("umap1"))
                            .and_then(|v| v.as_float())
                            .unwrap_or(0.0);
                        let y = map
                            .get("y")
                            .or(map.get("UMAP2"))
                            .or(map.get("umap2"))
                            .and_then(|v| v.as_float())
                            .unwrap_or(0.0);
                        xs.push(x);
                        ys.push(y);
                        let cl = if !color_col.is_empty() {
                            map.get(&color_col)
                                .map(|v| format!("{v}"))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        cls.push(cl);
                        let lb = if !label_col.is_empty() {
                            map.get(&label_col)
                                .map(|v| format!("{v}"))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        lbls.push(lb);
                    }
                }
                (xs, ys, cls, lbls)
            }
            _ => {
                return Err(BioLangError::type_error(
                    "umap_plot() requires Table or List of Records",
                    None,
                ))
            }
        };

    if xs.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "umap_plot() empty data",
            None,
        ));
    }
    let rendered_point_count = xs
        .iter()
        .zip(ys.iter())
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .count();
    if rendered_point_count == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "umap_plot() has no finite coordinate pairs",
            None,
        ));
    }

    // Continuous colouring: one gene's expression across the same points.
    //
    // This is Seurat's FeaturePlot, and it is how marker-based annotation is
    // actually taught - colour the embedding by LYZ and the monocyte island
    // lights up. Handled here rather than in a separate builtin so it reuses the
    // extraction, scaling and canvas work above; feature_plot() is an alias.
    let feature_col = get_opt_str(&opts, "feature", "").to_string();
    // The colour key is labelled with the column name by default. `feature_label`
    // separates the two so a caller need not rename its data field just to get a
    // readable legend — building 737k records with a computed key to do that
    // cost more than drawing the plot.
    let feature_label = {
        let explicit = get_opt_str(&opts, "feature_label", "").to_string();
        if explicit.is_empty() {
            feature_col.clone()
        } else {
            explicit
        }
    };
    let feature_values: Vec<f64> = if feature_col.is_empty() {
        Vec::new()
    } else {
        match &args[0] {
            Value::Table(table) => extract_table_col(table, &feature_col).unwrap_or_default(),
            Value::List(items) => items
                .iter()
                .map(|item| match item {
                    Value::Record(map) => map
                        .get(&feature_col)
                        .and_then(|v| v.as_float())
                        .unwrap_or(f64::NAN),
                    _ => f64::NAN,
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    // A partial column would silently mis-colour points, so take it only when
    // there is a value for every point.
    let has_feature =
        feature_values.len() == xs.len() && feature_values.iter().any(|value| value.is_finite());
    let raw_feature_range = if has_feature {
        col_range(&feature_values)
    } else {
        (0.0, 1.0)
    };
    let feature_range = if has_feature {
        let lo = feature_cutoff(&opts, "min_cutoff", &feature_values, raw_feature_range.0)?;
        let hi = feature_cutoff(&opts, "max_cutoff", &feature_values, raw_feature_range.1)?;
        if lo > hi {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "umap_plot() min_cutoff must not exceed max_cutoff",
                None,
            ));
        }
        (lo, hi)
    } else {
        raw_feature_range
    };

    // Build group -> color index mapping
    let mut group_order: Vec<String> = Vec::new();
    let mut group_map: HashMap<String, usize> = HashMap::new();
    for (index, cl) in color_labels.iter().enumerate() {
        if !xs[index].is_finite() || !ys[index].is_finite() {
            continue;
        }
        if !cl.is_empty() && !group_map.contains_key(cl) {
            group_map.insert(cl.clone(), group_order.len());
            group_order.push(cl.clone());
        }
    }
    // A feature scale takes over the legend area, so the two never compete.
    let has_groups = !group_order.is_empty() && !has_feature;

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = embedding_plot_spec_value(
            &opts,
            &xs,
            &ys,
            &color_labels,
            &point_labels,
            &feature_values,
            has_feature,
            &feature_label,
            feature_range,
            publication_theme,
        )?;
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_embedding_plot_spec_value(&spec, &opts);
    }

    let xr = col_range(&xs);
    let yr = col_range(&ys);
    let xpad = (xr.1 - xr.0) * 0.05 + 0.1;
    let ypad = (yr.1 - yr.0) * 0.05 + 0.1;
    let xr = (xr.0 - xpad, xr.1 + xpad);
    let yr = (yr.0 - ypad, yr.1 + ypad);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 450.0);
        let mut c = SvgCanvas::with_theme(w, h, theme);
        let (legend_rows, group_legend_w) = group_legend_reserve(&group_order, h, theme);
        let feature_legend_w = if has_feature {
            if theme.is_adaptive() {
                (estimate_text_width(&feature_label, theme.legend_size) + 54.0).clamp(92.0, 170.0)
            } else {
                120.0
            }
        } else {
            0.0
        };
        let legend_w = if has_feature {
            feature_legend_w
        } else if has_groups {
            group_legend_w
        } else {
            0.0
        };
        c.fit_cartesian_layout(
            &Scale {
                domain: xr,
                range: xr,
            }
            .nice_ticks(5),
            &Scale {
                domain: yr,
                range: yr,
            }
            .nice_ticks(5),
            "UMAP 1",
            "UMAP 2",
            &title,
            &subtitle,
            &caption,
            legend_w,
        );
        // Legacy figures reserved a strip inside the default panel. Adaptive
        // themes reserve a true outer margin, so data and grid share one panel.
        let plot_right = if theme.is_adaptive() {
            c.margin.left + c.plot_width()
        } else {
            c.margin.left + c.plot_width() - legend_w
        };
        let equal_aspect = match opts.get("aspect") {
            Some(Value::Str(value)) => value.eq_ignore_ascii_case("equal"),
            Some(Value::Bool(value)) => *value,
            _ => publication_theme,
        };
        let (plot_xr, plot_yr) = if equal_aspect {
            expand_equal_aspect_domains(xr, yr, plot_right - c.margin.left, c.plot_height())
        } else {
            (xr, yr)
        };
        let domain_x = Scale {
            domain: plot_xr,
            range: plot_xr,
        };
        let domain_y = Scale {
            domain: plot_yr,
            range: plot_yr,
        };
        c.draw_cartesian_grid(&domain_x, &domain_y);
        c.set_accessible_description(format!(
            "Embedding with {} points, {} groups{}.",
            rendered_point_count,
            group_order.len(),
            if has_feature {
                format!(", coloured by {feature_label}")
            } else {
                String::new()
            }
        ));
        let x_scale = Scale {
            domain: plot_xr,
            range: (c.margin.left, plot_right),
        };
        let y_scale = Scale {
            domain: plot_yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };

        // One <circle> per cell, or one embedded raster for the lot. Vector
        // points are better when there are few: they hover, they select, they
        // scale to any zoom. The threshold and the reasoning behind it live
        // with `raster_choice`, so every scatter switches over at the same
        // size and explains itself the same way.
        let raster = raster_choice(&opts, "umap_plot", rendered_point_count)?;

        let point_color = |i: usize| -> String {
            if has_feature {
                let (lo, hi) = feature_range;
                if !feature_values[i].is_finite() {
                    return get_opt_str(
                        &opts,
                        "na_color",
                        if publication_theme {
                            "#d9dde3"
                        } else {
                            "#b8b8b8"
                        },
                    )
                    .to_string();
                }
                // A column with no spread would divide by zero; paint it mid-scale.
                let t = if (hi - lo).abs() < 1e-12 {
                    0.5
                } else {
                    (feature_values[i] - lo) / (hi - lo)
                };
                if seurat_theme {
                    seurat_feature_color(t)
                } else if publication_theme {
                    publication_sequential_color(t)
                } else {
                    sequential_color(t)
                }
            } else if has_groups {
                let ci = group_map.get(&color_labels[i]).copied().unwrap_or(0);
                if seurat_theme {
                    SEURAT_PALETTE[ci % SEURAT_PALETTE.len()].to_string()
                } else {
                    PALETTE[ci % PALETTE.len()].to_string()
                }
            } else {
                "#4e79a7".to_string()
            }
        };

        // Low values first, high values last: highly expressed cells remain
        // visible instead of being covered by input-order low-expression dots.
        let mut draw_order = (0..xs.len())
            .filter(|&index| xs[index].is_finite() && ys[index].is_finite())
            .collect::<Vec<_>>();
        if has_feature && publication_theme {
            draw_order.sort_by(|&a, &b| {
                match (feature_values[a].is_finite(), feature_values[b].is_finite()) {
                    (false, true) => std::cmp::Ordering::Less,
                    (true, false) => std::cmp::Ordering::Greater,
                    _ => feature_values[a].total_cmp(&feature_values[b]),
                }
            });
        }
        let points: Vec<(f64, f64, String)> = draw_order
            .iter()
            .map(|&i| (x_scale.map(xs[i]), y_scale.map(ys[i]), point_color(i)))
            .collect();
        // Not point_area(): the legend takes the right-hand strip, so the
        // points stop at plot_right rather than the full plot width.
        c.add_scatter(
            &points,
            point_radius,
            (
                c.margin.left,
                c.margin.top,
                plot_right - c.margin.left,
                c.plot_height(),
            ),
            raster,
        );

        // Point labels stay vector in both modes - they are text, and there are
        // never many of them.
        for i in 0..xs.len() {
            if xs[i].is_finite() && ys[i].is_finite() && !point_labels[i].is_empty() {
                c.add_text(
                    x_scale.map(xs[i]) + 4.0,
                    y_scale.map(ys[i]) - 4.0,
                    &point_labels[i],
                    "start",
                    7.0,
                );
            }
        }

        // Colour bar for a continuous feature: without a scale the reader can
        // see where expression is high but not how high.
        if has_feature {
            let lx = plot_right + 14.0;
            let bar_top = c.margin.top + 14.0;
            let bar_height = 120.0_f64;
            let steps = 40;
            for step in 0..steps {
                // Drawn top-down, so the top of the bar is the maximum.
                let t = 1.0 - (step as f64 / (steps - 1) as f64);
                let y = bar_top + (step as f64 / steps as f64) * bar_height;
                c.add_rect(
                    lx,
                    y,
                    12.0,
                    bar_height / steps as f64 + 0.6,
                    &if seurat_theme {
                        seurat_feature_color(t)
                    } else if publication_theme {
                        publication_sequential_color(t)
                    } else {
                        sequential_color(t)
                    },
                );
            }
            let (lo, hi) = feature_range;
            c.add_text(lx + 16.0, bar_top + 8.0, &format!("{hi:.2}"), "start", 9.0);
            c.add_text(
                lx + 16.0,
                bar_top + bar_height,
                &format!("{lo:.2}"),
                "start",
                9.0,
            );
            c.add_text(lx, bar_top - 6.0, &feature_label, "start", 10.0);
        }

        // Legend for groups
        if has_groups {
            let lx = plot_right + 12.0;
            let legend_draw_width = if theme.is_adaptive() {
                (c.margin.right - 20.0).max(40.0)
            } else {
                group_legend_w
            };
            for (gi, gname) in group_order.iter().enumerate() {
                let column = gi / legend_rows;
                let row = gi % legend_rows;
                let column_x = lx
                    + column as f64
                        * (legend_draw_width / group_order.len().div_ceil(legend_rows) as f64);
                let ly = c.margin.top + 10.0 + row as f64 * 18.0;
                let color = if seurat_theme {
                    SEURAT_PALETTE[gi % SEURAT_PALETTE.len()]
                } else {
                    PALETTE[gi % PALETTE.len()]
                };
                c.add_circle(column_x + 5.0, ly + 4.0, 4.0, color);
                c.add_text(column_x + 13.0, ly + 8.0, gname, "start", theme.legend_size);
            }
        }

        // Seurat's `label = TRUE`: place one readable label at the group
        // centre. Medians are less sensitive than means to stray UMAP points.
        if has_groups && label_groups {
            for group in &group_order {
                let mut group_x: Vec<f64> = color_labels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| {
                        (label == group && xs[i].is_finite() && ys[i].is_finite()).then_some(xs[i])
                    })
                    .collect();
                let mut group_y: Vec<f64> = color_labels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| {
                        (label == group && xs[i].is_finite() && ys[i].is_finite()).then_some(ys[i])
                    })
                    .collect();
                if group_x.is_empty() {
                    continue;
                }
                group_x.sort_by(f64::total_cmp);
                group_y.sort_by(f64::total_cmp);
                let x = x_scale.map(group_x[group_x.len() / 2]);
                let y = y_scale.map(group_y[group_y.len() / 2]);
                let box_width = 8.0 + estimate_text_width(group, theme.legend_size);
                c.add_rect(x - box_width / 2.0, y - 10.0, box_width, 16.0, "#ffffff");
                c.add_text(x, y + 2.0, group, "middle", theme.legend_size);
            }
        }

        // Axis labels
        let dx = Scale {
            domain: plot_xr,
            range: plot_xr,
        };
        let dy = Scale {
            domain: plot_yr,
            range: plot_yr,
        };
        // Explicit labels win; otherwise infer from the title. Inference alone
        // labelled a counts-vs-genes QC scatter "Dim 1"/"Dim 2", because this
        // builtin now draws any continuous-valued scatter, not just embeddings.
        let x_override = get_opt_str(&opts, "xlab", "").to_string();
        let y_override = get_opt_str(&opts, "ylab", "").to_string();
        let title_lc = title.to_lowercase();
        let x_label = if title_lc.contains("umap") {
            "UMAP 1"
        } else if title_lc.contains("pca") {
            "PC 1"
        } else if title_lc.contains("tsne") || title_lc.contains("t-sne") {
            "t-SNE 1"
        } else {
            "Dim 1"
        };
        let y_label = if title_lc.contains("umap") {
            "UMAP 2"
        } else if title_lc.contains("pca") {
            "PC 2"
        } else if title_lc.contains("tsne") || title_lc.contains("t-sne") {
            "t-SNE 2"
        } else {
            "Dim 2"
        };
        let x_label = if x_override.is_empty() {
            x_label
        } else {
            x_override.as_str()
        };
        let y_label = if y_override.is_empty() {
            y_label
        } else {
            y_override.as_str()
        };
        c.draw_x_axis(&dx, x_label);
        c.draw_y_axis(&dy, y_label);
        c.draw_title(&title);
        c.draw_subtitle(&subtitle);
        c.draw_caption(&caption);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for i in 0..xs.len() {
        if !xs[i].is_finite() || !ys[i].is_finite() {
            continue;
        }
        let ch = if has_groups {
            let ci = group_map.get(&color_labels[i]).copied().unwrap_or(0);
            char::from_digit((ci % 10) as u32, 10).unwrap_or('*')
        } else {
            '*'
        };
        chart.put(xs[i], ys[i], xr, yr, ch);
    }
    let n = rendered_point_count;
    write_output(&chart.render(&format!("{title}  ({n} points)")));
    Ok(Value::Nil)
}

// ── coverage_track ───────────────────────────────────────────────

#[derive(Clone, Debug)]
struct CoverageDatum {
    source_row: usize,
    chromosome: Option<String>,
    original_start: f64,
    original_end: f64,
    start: f64,
    end: f64,
    value: f64,
    geometry: &'static str,
}

fn coverage_value_from_record(map: &HashMap<String, Value>) -> Option<f64> {
    ["value", "coverage", "signal", "score"]
        .into_iter()
        .find_map(|key| map.get(key).and_then(Value::as_float))
}

fn coverage_data(value: &Value, opts: &HashMap<String, Value>) -> Result<Vec<CoverageDatum>> {
    match value {
        Value::Table(table) => {
            let position_column = opts
                .get("pos")
                .and_then(Value::as_str)
                .and_then(|name| table.col_index(name))
                .or_else(|| table.col_index("pos"))
                .or_else(|| table.col_index("position"));
            let start_column = table.col_index(get_opt_str(opts, "start", "start"));
            let end_column = table.col_index(get_opt_str(opts, "end", "end"));
            if position_column.is_none() && (start_column.is_none() || end_column.is_none()) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "coverage_track() Table needs pos/position or both start and end columns",
                    None,
                ));
            }
            let value_column = if let Some(name) = opts.get("value").and_then(Value::as_str) {
                table.col_index(name)
            } else {
                ["value", "coverage", "signal", "score"]
                    .into_iter()
                    .find_map(|name| table.col_index(name))
            }
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "coverage_track() Table needs value, coverage, signal or score",
                    None,
                )
            })?;
            let chromosome_column = table.col_index(get_opt_str(opts, "chrom", "chrom"));
            table
                .rows
                .iter()
                .enumerate()
                .map(|(source_row, row)| {
                    let chromosome = chromosome_column
                        .map(|column| {
                            row[column]
                                .as_str()
                                .filter(|name| !name.trim().is_empty())
                                .map(str::to_string)
                                .ok_or_else(|| {
                                    BioLangError::runtime(
                                        ErrorKind::TypeError,
                                        "coverage_track() chromosome values must be non-empty Str",
                                        None,
                                    )
                                })
                        })
                        .transpose()?;
                    let (start, end, geometry) = if let Some(column) = position_column {
                        let position = row[column].as_float().unwrap_or(f64::NAN);
                        (position, position, "point")
                    } else {
                        (
                            row[start_column.unwrap()].as_float().unwrap_or(f64::NAN),
                            row[end_column.unwrap()].as_float().unwrap_or(f64::NAN),
                            "interval",
                        )
                    };
                    let value = row[value_column].as_float().unwrap_or(f64::NAN);
                    Ok(CoverageDatum {
                        source_row,
                        chromosome,
                        original_start: start,
                        original_end: end,
                        start,
                        end,
                        value,
                        geometry,
                    })
                })
                .collect()
        }
        Value::List(items) => items
            .iter()
            .enumerate()
            .map(|(source_row, item)| {
                let map = match item {
                    Value::Record(map) => map,
                    _ => {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            "coverage_track() List items must be Records",
                            None,
                        ))
                    }
                };
                let chromosome = map
                    .get("chrom")
                    .or_else(|| map.get("chromosome"))
                    .map(|value| {
                        value
                            .as_str()
                            .filter(|name| !name.trim().is_empty())
                            .map(str::to_string)
                            .ok_or_else(|| {
                                BioLangError::runtime(
                                    ErrorKind::TypeError,
                                    "coverage_track() chromosome values must be non-empty Str",
                                    None,
                                )
                            })
                    })
                    .transpose()?;
                let (start, end, geometry) = if let Some(position) = map
                    .get("pos")
                    .or_else(|| map.get("position"))
                    .and_then(Value::as_float)
                {
                    (position, position, "point")
                } else {
                    let start = map.get("start").and_then(Value::as_float).ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "coverage_track() record needs pos/position or numeric start",
                            None,
                        )
                    })?;
                    let end = map.get("end").and_then(Value::as_float).ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "coverage_track() interval record needs numeric end",
                            None,
                        )
                    })?;
                    (start, end, "interval")
                };
                let value = coverage_value_from_record(map).ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "coverage_track() record needs numeric value, coverage, signal or score",
                        None,
                    )
                })?;
                Ok(CoverageDatum {
                    source_row,
                    chromosome,
                    original_start: start,
                    original_end: end,
                    start,
                    end,
                    value,
                    geometry,
                })
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            "coverage_track() requires Table or List of Records",
            None,
        )),
    }
}

fn valid_bio_hex_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn coverage_track_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let mut data = coverage_data(value, opts)?;
    let input_rows = data.len();
    if data.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() empty data",
            None,
        ));
    }
    if data.iter().any(|datum| {
        !datum.start.is_finite()
            || !datum.end.is_finite()
            || !datum.value.is_finite()
            || datum.start < 0.0
            || datum.end < datum.start
            || (datum.geometry == "interval" && datum.end <= datum.start)
    }) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() coordinates and values must be finite, positions non-negative, and interval end > start",
            None,
        ));
    }
    let requested_chromosome = opts.get("chromosome").and_then(Value::as_str);
    if let Some(chromosome) = requested_chromosome {
        if chromosome.trim().is_empty() || data.iter().all(|datum| datum.chromosome.is_none()) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "coverage_track() chromosome filtering needs a non-empty name and chromosome data",
                None,
            ));
        }
        data.retain(|datum| datum.chromosome.as_deref() == Some(chromosome));
    }
    let observed_chromosomes = data
        .iter()
        .filter_map(|datum| datum.chromosome.as_deref())
        .collect::<HashSet<_>>();
    let rows_with_chromosomes = data
        .iter()
        .filter(|datum| datum.chromosome.is_some())
        .count();
    if rows_with_chromosomes != 0 && rows_with_chromosomes != data.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() chromosome values must be present for every row or omitted from every row",
            None,
        ));
    }
    if observed_chromosomes.len() > 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() draws one genomic region; select one chromosome with {chromosome: \"chr1\"}",
            None,
        ));
    }
    let selected_chromosome = observed_chromosomes.into_iter().next().map(str::to_string);
    let region_start = opts.get("region_start").and_then(Value::as_float);
    let region_end = opts.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|value| !value.is_finite() || value < 0.0)
        || region_end.is_some_and(|value| !value.is_finite() || value < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() region_start/region_end must be finite, non-negative and increasing",
            None,
        ));
    }
    let mut clipped_rows = 0usize;
    data = data
        .into_iter()
        .filter_map(|mut datum| {
            if datum.geometry == "point" {
                let keep = region_start.is_none_or(|start| datum.start >= start)
                    && region_end.is_none_or(|end| datum.start <= end);
                keep.then_some(datum)
            } else {
                if region_start.is_some_and(|start| datum.end <= start)
                    || region_end.is_some_and(|end| datum.start >= end)
                {
                    return None;
                }
                let clipped_start =
                    region_start.map_or(datum.start, |start| datum.start.max(start));
                let clipped_end = region_end.map_or(datum.end, |end| datum.end.min(end));
                if clipped_start != datum.start || clipped_end != datum.end {
                    clipped_rows += 1;
                }
                datum.start = clipped_start;
                datum.end = clipped_end;
                (datum.end > datum.start).then_some(datum)
            }
        })
        .collect();
    if data.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() no data after chromosome/region filtering",
            None,
        ));
    }
    data.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let retained_rows = data.len();
    let rows = data
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                datum
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(datum.original_start),
                Value::Float(datum.original_end),
                Value::Float(datum.start),
                Value::Float(datum.end),
                Value::Float(if datum.geometry == "point" {
                    datum.start
                } else {
                    (datum.start + datum.end) / 2.0
                }),
                Value::Float(datum.value),
                Value::Str(datum.geometry.into()),
                Value::Bool(datum.start != datum.original_start || datum.end != datum.original_end),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Coverage Track");
    let color = get_opt_str(opts, "color", "#4e79a7");
    if !valid_bio_hex_color(color) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() color must be a #rrggbb value",
            None,
        ));
    }
    let warnings = if input_rows == retained_rows && clipped_rows == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{} rows retained from {input_rows}; {clipped_rows} overlapping intervals clipped to the requested region",
            retained_rows
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("coverage_track".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "chromosome",
                        "original_start",
                        "original_end",
                        "start",
                        "end",
                        "position",
                        "value",
                        "geometry",
                        "clipped",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 900.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 280.0)),
                        ),
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Genomic position").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "Coverage / signal").into()),
                        ),
                        ("color".into(), Value::Str(color.into())),
                        (
                            "chromosome".into(),
                            selected_chromosome
                                .clone()
                                .map(Value::Str)
                                .unwrap_or(Value::Nil),
                        ),
                        (
                            "region_start".into(),
                            region_start.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_end".into(),
                            region_end.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("coverage_track".into())),
                        ("input_rows".into(), Value::Int(input_rows as i64)),
                        ("retained_rows".into(), Value::Int(retained_rows as i64)),
                        ("clipped_rows".into(), Value::Int(clipped_rows as i64)),
                        (
                            "row_order".into(),
                            Value::Str("start_end_source_row".into()),
                        ),
                        (
                            "region_rule".into(),
                            Value::Str("points_inclusive_intervals_overlap_half_open".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open_for_intervals".into()),
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

fn render_coverage_track_svg(table: &Table, opts: &HashMap<String, Value>) -> Result<String> {
    let starts = extract_table_col(table, "start")?;
    let ends = extract_table_col(table, "end")?;
    let positions = extract_table_col(table, "position")?;
    let values = extract_table_col(table, "value")?;
    let geometry_column = table.col_index("geometry").unwrap();
    let width = get_opt_f64(opts, "width", 900.0);
    let height = get_opt_f64(opts, "height", 280.0);
    let title = get_opt_str(opts, "title", "Coverage Track");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Genomic position");
    let y_label = get_opt_str(opts, "ylabel", "Coverage / signal");
    let color = get_opt_str(opts, "color", "#4e79a7");
    let mut x_min = starts.iter().copied().fold(f64::INFINITY, f64::min);
    let mut x_max = ends.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if let Some(region_start) = opts.get("region_start").and_then(Value::as_float) {
        x_min = region_start;
    }
    if let Some(region_end) = opts.get("region_end").and_then(Value::as_float) {
        x_max = region_end;
    }
    if x_max <= x_min {
        x_min -= 0.5;
        x_max += 0.5;
    }
    let raw_min = values.iter().copied().fold(0.0, f64::min);
    let raw_max = values.iter().copied().fold(0.0, f64::max);
    let spread = (raw_max - raw_min).max(raw_max.abs() * 0.05).max(1.0);
    let y_domain = (
        if raw_min < 0.0 {
            raw_min - spread * 0.05
        } else {
            0.0
        },
        if raw_max > 0.0 {
            raw_max + spread * 0.05
        } else {
            0.0
        },
    );
    let y_domain = if y_domain.1 <= y_domain.0 {
        (y_domain.0 - 0.5, y_domain.1 + 0.5)
    } else {
        y_domain
    };
    let x_domain = (x_min, x_max);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[x_min, x_max],
        &[y_domain.0, 0.0, y_domain.1],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    let baseline = y_scale.map(0.0);
    let mut interval_fill = String::new();
    let mut interval_line = String::new();
    let mut point_indexes = Vec::new();
    for (index, row) in table.rows.iter().enumerate() {
        if row[geometry_column].as_str() == Some("interval") {
            let x1 = x_scale.map(starts[index]);
            let x2 = x_scale.map(ends[index]);
            let y = y_scale.map(values[index]);
            interval_fill.push_str(&format!(
                "M{:.2},{:.2}V{:.2}H{:.2}V{:.2}Z",
                x1, baseline, y, x2, baseline
            ));
            interval_line.push_str(&format!("M{:.2},{:.2}H{:.2}", x1, y, x2));
        } else {
            point_indexes.push(index);
        }
    }
    if !interval_fill.is_empty() {
        canvas.elements.push(format!(
            r#"<path d="{interval_fill}" fill="{color}" fill-opacity="0.34" stroke="none" />"#
        ));
        canvas.elements.push(format!(
            r#"<path d="{interval_line}" fill="none" stroke="{color}" stroke-width="1.35" />"#
        ));
    }
    if !point_indexes.is_empty() {
        let mut area = format!(
            "M{:.2},{:.2}",
            x_scale.map(positions[point_indexes[0]]),
            baseline
        );
        let mut line = String::new();
        for &index in &point_indexes {
            let x = x_scale.map(positions[index]);
            let y = y_scale.map(values[index]);
            area.push_str(&format!("L{:.2},{:.2}", x, y));
            if line.is_empty() {
                line.push_str(&format!("M{:.2},{:.2}", x, y));
            } else {
                line.push_str(&format!("L{:.2},{:.2}", x, y));
            }
        }
        area.push_str(&format!(
            "L{:.2},{:.2}Z",
            x_scale.map(positions[*point_indexes.last().unwrap()]),
            baseline
        ));
        canvas.elements.push(format!(
            r#"<path d="{area}" fill="{color}" fill-opacity="0.26" stroke="none" />"#
        ));
        canvas.elements.push(format!(
            r#"<path d="{line}" fill="none" stroke="{color}" stroke-width="1.5" stroke-linejoin="round" />"#
        ));
        if point_indexes.len() == 1 {
            let index = point_indexes[0];
            canvas.add_circle(
                x_scale.map(positions[index]),
                y_scale.map(values[index]),
                2.8,
                color,
            );
        }
    }
    canvas.draw_x_axis(
        &Scale {
            domain: x_domain,
            range: x_domain,
        },
        x_label,
    );
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let interval_count = table
        .rows
        .iter()
        .filter(|row| row[geometry_column].as_str() == Some("interval"))
        .count();
    canvas.set_accessible_description(format!(
        "Coverage track with {} observations: {interval_count} intervals and {} point samples.",
        table.num_rows(),
        table.num_rows() - interval_count
    ));
    Ok(canvas.render())
}

pub(crate) fn is_coverage_track_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "coverage_track"))
}

pub(crate) fn render_coverage_track_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_coverage_track_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 coverage-track Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() coverage-track specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "chromosome",
        "original_start",
        "original_end",
        "start",
        "end",
        "position",
        "value",
        "geometry",
        "clipped",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() coverage-track data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() coverage-track specification has no observations",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "coverage-track")?;
    let color = get_opt_str(&options, "color", "");
    if !valid_bio_hex_color(color) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() coverage-track color must be a #rrggbb value",
            None,
        ));
    }
    let point_column = table.col_index("point_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let chromosome_column = table.col_index("chromosome").unwrap();
    let original_start_column = table.col_index("original_start").unwrap();
    let original_end_column = table.col_index("original_end").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let position_column = table.col_index("position").unwrap();
    let value_column = table.col_index("value").unwrap();
    let geometry_column = table.col_index("geometry").unwrap();
    let clipped_column = table.col_index("clipped").unwrap();
    let selected_chromosome = options.get("chromosome").and_then(Value::as_str);
    let region_start = options.get("region_start").and_then(Value::as_float);
    let region_end = options.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|number| !number.is_finite() || number < 0.0)
        || region_end.is_some_and(|number| !number.is_finite() || number < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() coverage-track region is malformed",
            None,
        ));
    }
    let mut previous: Option<(f64, f64, usize)> = None;
    for (expected, row) in table.rows.iter().enumerate() {
        let point_index =
            frozen_nonnegative_integer(&row[point_column], "coverage-track", "point_index")?;
        let source =
            frozen_nonnegative_integer(&row[source_column], "coverage-track", "source_row")?;
        let original_start = row[original_start_column].as_float().unwrap_or(f64::NAN);
        let original_end = row[original_end_column].as_float().unwrap_or(f64::NAN);
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let datum = row[value_column].as_float().unwrap_or(f64::NAN);
        let geometry = row[geometry_column].as_str().unwrap_or("");
        let chromosome = match &row[chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() coverage-track chromosome is malformed",
                    None,
                ))
            }
        };
        let key = (start, end, source);
        let expected_position = if geometry == "point" {
            start
        } else {
            (start + end) / 2.0
        };
        let expected_start = if geometry == "point" {
            original_start
        } else {
            region_start.map_or(original_start, |lower| original_start.max(lower))
        };
        let expected_end = if geometry == "point" {
            original_end
        } else {
            region_end.map_or(original_end, |upper| original_end.min(upper))
        };
        let expected_clipped = start != original_start || end != original_end;
        if point_index != expected
            || !original_start.is_finite()
            || !original_end.is_finite()
            || !start.is_finite()
            || !end.is_finite()
            || !position.is_finite()
            || !datum.is_finite()
            || start < 0.0
            || end < start
            || !matches!(geometry, "point" | "interval")
            || (geometry == "point" && (end != start || original_end != original_start))
            || (geometry == "interval" && (end <= start || original_end <= original_start))
            || (start - expected_start).abs() > 1e-10
            || (end - expected_end).abs() > 1e-10
            || (position - expected_position).abs() > 1e-10
            || !matches!(row[clipped_column], Value::Bool(_))
            || row[clipped_column].is_truthy() != expected_clipped
            || selected_chromosome != chromosome
            || region_start.is_some_and(|lower| start < lower)
            || region_end.is_some_and(|upper| end > upper)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() coverage-track frozen geometry is inconsistent",
                None,
            ));
        }
        previous = Some(key);
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_coverage_track_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Coverage Track");
    finish_frozen_bio_plot(value, render_options, title, "coverage-track", svg)
}

/// Genome browser-style coverage track. Interval inputs retain and clip their
/// real spans; point inputs retain their sampled positions.
fn builtin_coverage_track(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = coverage_track_spec_value(&args[0], &opts)?;
    render_coverage_track_plot_spec_value(&specification, &opts)
}
