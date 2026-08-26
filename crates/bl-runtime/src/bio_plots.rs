use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::{HashMap, HashSet};

use crate::builtins::write_output;
use crate::plot::{
    col_range, extract_table_col, gaussian_kde, get_opt_f64, get_opt_str, parse_options,
    quantile_type7, raster_choice, sequential_color, seurat_feature_color, silverman_bandwidth,
    thin_requested, thin_to_pixel_grid, Scale, SvgCanvas, PALETTE, SEURAT_PALETTE,
};
use crate::viz::{get_opt_usize, nums_from_value, spark_str};

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

/// Assign genomic x-offsets: returns (genome_x_per_point, chrom_boundaries)
fn genome_x_layout(chroms: &[String], positions: &[f64]) -> (Vec<f64>, Vec<(String, f64, f64)>) {
    let mut chrom_order: Vec<String> = Vec::new();
    let mut chrom_max: HashMap<String, f64> = HashMap::new();
    for (i, c) in chroms.iter().enumerate() {
        chrom_max
            .entry(c.clone())
            .and_modify(|m| {
                if positions[i] > *m {
                    *m = positions[i];
                }
            })
            .or_insert(positions[i]);
        if (!chrom_max.contains_key(c) || !chrom_order.contains(c)) && !chrom_order.contains(c) {
            chrom_order.push(c.clone());
        }
    }
    let mut offsets: HashMap<String, f64> = HashMap::new();
    let mut boundaries = Vec::new();
    let mut cum = 0.0;
    for c in &chrom_order {
        offsets.insert(c.clone(), cum);
        let len = chrom_max.get(c).copied().unwrap_or(1.0);
        boundaries.push((c.clone(), cum, cum + len));
        cum += len + len * 0.02; // 2% gap
    }
    let xs: Vec<f64> = chroms
        .iter()
        .zip(positions.iter())
        .map(|(c, &p)| offsets.get(c).unwrap_or(&0.0) + p)
        .collect();
    (xs, boundaries)
}

// ── 1. manhattan ────────────────────────────────────────────────

fn builtin_manhattan(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "manhattan")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let threshold = get_opt_f64(&opts, "threshold", 5e-8);

    let chrom_col = get_opt_str(&opts, "chrom", "chrom").to_string();
    let pos_col = get_opt_str(&opts, "pos", "pos").to_string();
    let p_col = get_opt_str(&opts, "p", "pvalue").to_string();

    let chroms = extract_str_col(table, &chrom_col)?;
    let positions = extract_table_col(table, &pos_col)?;
    let pvalues = extract_table_col(table, &p_col)?;
    let nlp: Vec<f64> = pvalues
        .iter()
        .map(|&p| if p > 0.0 { -(p.log10()) } else { 0.0 })
        .collect();
    let nlp_thresh = if threshold > 0.0 {
        -(threshold.log10())
    } else {
        7.3
    };

    let (gx, boundaries) = genome_x_layout(&chroms, &positions);
    let xr = col_range(&gx);
    let (_, ymax) = col_range(&nlp);
    let yr = (0.0, ymax * 1.05);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 1200.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        // threshold line
        c.add_line(
            c.margin.left,
            ys.map(nlp_thresh),
            c.margin.left + c.plot_width(),
            ys.map(nlp_thresh),
            "#e15759",
            1.0,
        );
        // The worst case in the catalogue: a GWAS carries one point per variant,
        // and whole-genome studies run to millions. Vector circles cannot
        // represent that in a browser at all.
        let raster = raster_choice(&opts, "manhattan", nlp.len())?;
        let points: Vec<(f64, f64, &str)> = nlp
            .iter()
            .enumerate()
            .map(|(i, &y)| {
                let ci = boundaries
                    .iter()
                    .position(|b| b.0 == chroms[i])
                    .unwrap_or(0);
                (xs.map(gx[i]), ys.map(y), PALETTE[ci % PALETTE.len()])
            })
            .collect();
        let area = c.point_area();
        // A whole-genome study paints the same pixel hundreds of times over,
        // and that accumulated alpha is most of what the PNG has to encode.
        // Thinning is opt-in because it trades away density-as-shade; see
        // thin_to_pixel_grid for exactly what it drops.
        let thin = thin_requested(&opts, "manhattan")?;
        let drawn = if thin {
            let coords: Vec<(f64, f64)> = points.iter().map(|&(x, y, _)| (x, y)).collect();
            // Vector output has no device pixel of its own, so a thinned
            // vector plot thins at nominal size.
            let grid = if raster.enabled { raster.scale } else { 1.0 };
            let kept = thin_to_pixel_grid(&coords, area, grid, &nlp);
            let survivors: Vec<(f64, f64, &str)> = kept.iter().map(|&i| points[i]).collect();
            c.add_scatter(&survivors, 2.5, area, raster);
            survivors.len()
        } else {
            c.add_scatter(&points, 2.5, area, raster);
            points.len()
        };
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_y_axis(&dy, "-log10(p)");
        c.draw_title("Manhattan Plot");
        // The figure has to say so itself: someone reading the SVG later has no
        // other way to know it is not showing every variant.
        if drawn < points.len() {
            c.set_accessible_description(format!(
                "Manhattan plot, thinned to one variant per pixel: {drawn} of {} variants drawn,                  the most significant in each pixel. Point density does not indicate variant count.",
                points.len()
            ));
            c.add_text(
                c.margin.left,
                c.height - 6.0,
                &format!(
                    "thinned: {drawn} of {} variants drawn (most significant per pixel)",
                    points.len()
                ),
                "start",
                9.0,
            );
        }
        // chrom labels
        for (ci, (name, start, end)) in boundaries.iter().enumerate() {
            let mid = xs.map((start + end) / 2.0);
            if ci % 2 == 0 {
                c.add_text(
                    mid,
                    c.margin.top + c.plot_height() + 18.0,
                    name,
                    "middle",
                    9.0,
                );
            }
        }
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 80);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for (i, &y) in nlp.iter().enumerate() {
        let ch = if y >= nlp_thresh { '●' } else { '·' };
        chart.put(gx[i], y, xr, yr, ch);
    }
    chart.hline(nlp_thresh, yr, '╌');
    write_output(&chart.render("Manhattan Plot"));
    Ok(Value::Nil)
}

// ── 2. qq_plot ──────────────────────────────────────────────────

fn builtin_qq_plot(args: Vec<Value>) -> Result<Value> {
    let vals = nums_from_value(&args[0], "qq_plot")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let mut pvals: Vec<f64> = vals
        .into_iter()
        .filter(|v| *v > 0.0 && v.is_finite())
        .collect();
    pvals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = pvals.len();
    if n == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "qq_plot() needs positive p-values",
            None,
        ));
    }

    let observed: Vec<f64> = pvals.iter().map(|p| -(p.log10())).collect();
    let expected: Vec<f64> = (0..n)
        .map(|i| -((i as f64 + 0.5) / n as f64).log10())
        .collect();
    let max_val = observed
        .last()
        .copied()
        .unwrap_or(1.0)
        .max(*expected.last().unwrap_or(&1.0));
    let range = (0.0, max_val * 1.05);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: range,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: range,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        c.add_line(
            xs.map(0.0),
            ys.map(0.0),
            xs.map(max_val),
            ys.map(max_val),
            "#ccc",
            1.0,
        );
        // A genome-wide Q-Q carries one point per test.
        let raster = raster_choice(&opts, "qq_plot", n)?;
        let points: Vec<(f64, f64, &str)> = (0..n)
            .map(|i| (xs.map(expected[i]), ys.map(observed[i]), PALETTE[0]))
            .collect();
        let area = c.point_area();
        c.add_scatter(&points, 3.0, area, raster);
        let d = Scale {
            domain: range,
            range,
        };
        c.draw_x_axis(&d, "Expected -log10(p)");
        c.draw_y_axis(&d, "Observed -log10(p)");
        c.draw_title("QQ Plot");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    // diagonal
    for i in 0..chart.pw() {
        let v = range.0 + (range.1 - range.0) * i as f64 / chart.pw() as f64;
        chart.put(v, v, range, range, '╱');
    }
    for i in 0..n {
        chart.put(expected[i], observed[i], range, range, '●');
    }
    write_output(&chart.render("QQ Plot"));
    Ok(Value::Nil)
}

// ── 3. ideogram ─────────────────────────────────────────────────

fn builtin_ideogram(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "ideogram")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let bar_width = get_opt_usize(&opts, "width", 60);

    let chroms = extract_str_col(table, "chrom")?;
    let starts = extract_table_col(table, "start")?;
    let ends = extract_table_col(table, "end")?;
    let stain_col = table.col_index("stain").or_else(|| table.col_index("band"));

    // Group bands by chrom
    let mut chrom_order: Vec<String> = Vec::new();
    let mut chrom_bands: HashMap<String, Vec<(f64, f64, String)>> = HashMap::new();
    for i in 0..chroms.len() {
        let stain = stain_col
            .map(|si| match &table.rows[i][si] {
                Value::Str(s) => s.clone(),
                _ => String::new(),
            })
            .unwrap_or_default();
        if !chrom_bands.contains_key(&chroms[i]) {
            chrom_order.push(chroms[i].clone());
        }
        chrom_bands
            .entry(chroms[i].clone())
            .or_default()
            .push((starts[i], ends[i], stain));
    }

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", chrom_order.len() as f64 * 25.0 + 60.0);
        let mut c = SvgCanvas::new(w, h);
        c.margin.left = 80.0;
        let row_h = 16.0;
        let gap = 8.0;
        for (ci, chrom) in chrom_order.iter().enumerate() {
            let bands = &chrom_bands[chrom];
            let max_end = bands.iter().map(|b| b.1).fold(0.0f64, f64::max);
            let y = c.margin.top + ci as f64 * (row_h + gap);
            c.add_text(
                c.margin.left - 5.0,
                y + row_h / 2.0 + 4.0,
                chrom,
                "end",
                11.0,
            );
            let pw = c.plot_width();
            for (s, e, stain) in bands {
                let x1 = c.margin.left + s / max_end * pw;
                let x2 = c.margin.left + e / max_end * pw;
                let color = match stain.as_str() {
                    s if s.contains("gpos100") || s.contains("acen") => "#333",
                    s if s.contains("gpos75") => "#666",
                    s if s.contains("gpos50") => "#999",
                    s if s.contains("gpos25") => "#ccc",
                    _ => "#eee",
                };
                c.add_rect(x1, y, (x2 - x1).max(1.0), row_h, color);
            }
        }
        c.draw_title("Ideogram");
        return Ok(Value::Str(c.render()));
    }

    let mut out = String::from("  Ideogram\n");
    let max_label = chrom_order.iter().map(|c| c.len()).max().unwrap_or(4);
    for chrom in &chrom_order {
        let bands = &chrom_bands[chrom];
        let max_end = bands.iter().map(|b| b.1).fold(0.0f64, f64::max);
        let mut bar = vec![' '; bar_width];
        for (s, e, stain) in bands {
            let i0 = (s / max_end * bar_width as f64) as usize;
            let i1 = ((e / max_end * bar_width as f64).ceil() as usize).min(bar_width);
            let ch = match stain.as_str() {
                s if s.contains("gpos100") || s.contains("acen") => '█',
                s if s.contains("gpos75") => '▓',
                s if s.contains("gpos50") => '▒',
                s if s.contains("gpos25") => '░',
                _ => '─',
            };
            for b in i0..i1 {
                bar[b] = ch;
            }
        }
        let bar_str: String = bar.into_iter().collect();
        out.push_str(&format!("  {:>w$}  {bar_str}\n", chrom, w = max_label));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 4. rainfall ─────────────────────────────────────────────────

fn builtin_rainfall(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "rainfall")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let chroms = extract_str_col(table, get_opt_str(&opts, "chrom", "chrom"))?;
    let positions = extract_table_col(table, get_opt_str(&opts, "pos", "pos"))?;

    // Sort by chrom + position and compute inter-mutation distances
    let mut indices: Vec<usize> = (0..chroms.len()).collect();
    indices.sort_by(|&a, &b| {
        chroms[a]
            .cmp(&chroms[b])
            .then(positions[a].partial_cmp(&positions[b]).unwrap())
    });

    let mut dists: Vec<f64> = Vec::new();
    let mut gxs: Vec<f64> = Vec::new();
    let mut chrom_labels: Vec<String> = Vec::new();
    for w in indices.windows(2) {
        let (i, j) = (w[0], w[1]);
        if chroms[i] == chroms[j] {
            let d = (positions[j] - positions[i]).max(1.0);
            dists.push(d.log10());
            gxs.push(positions[j]);
            chrom_labels.push(chroms[j].clone());
        }
    }

    if dists.is_empty() {
        write_output("  (insufficient data for rainfall plot)\n");
        return Ok(Value::Nil);
    }

    let (gx_all, _) = genome_x_layout(&chrom_labels, &gxs);
    let xr = col_range(&gx_all);
    let yr = col_range(&dists);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 1000.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        // One point per mutation across the genome; a somatic catalogue runs to
        // hundreds of thousands.
        let raster = raster_choice(&opts, "rainfall", dists.len())?;
        let points: Vec<(f64, f64, &str)> = (0..dists.len())
            .map(|i| {
                let color = if dists[i] < 3.0 {
                    "#e15759"
                } else if dists[i] < 5.0 {
                    "#f28e2b"
                } else {
                    "#76b7b2"
                };
                (xs.map(gx_all[i]), ys.map(dists[i]), color)
            })
            .collect();
        let area = c.point_area();
        c.add_scatter(&points, 2.5, area, raster);
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_y_axis(&dy, "log10(distance)");
        c.draw_title("Rainfall Plot");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 80);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for i in 0..dists.len() {
        let ch = if dists[i] < 3.0 { '●' } else { '·' };
        chart.put(gx_all[i], dists[i], xr, yr, ch);
    }
    write_output(&chart.render("Rainfall Plot"));
    Ok(Value::Nil)
}

// ── 5. cnv_plot ─────────────────────────────────────────────────

fn builtin_cnv_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "cnv_plot")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let chroms = extract_str_col(table, get_opt_str(&opts, "chrom", "chrom"))?;
    let starts = extract_table_col(table, get_opt_str(&opts, "start", "start"))?;
    let ends = extract_table_col(table, get_opt_str(&opts, "end", "end"))?;
    let ratios = extract_table_col(table, get_opt_str(&opts, "ratio", "log2ratio"))?;

    let midpoints: Vec<f64> = starts
        .iter()
        .zip(ends.iter())
        .map(|(s, e)| (s + e) / 2.0)
        .collect();
    let (gx, _boundaries) = genome_x_layout(&chroms, &midpoints);
    let xr = col_range(&gx);
    let (ylo, yhi) = col_range(&ratios);
    let yabs = ylo.abs().max(yhi.abs()).max(0.5);
    let yr = (-yabs, yabs);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 1000.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        c.add_line(
            c.margin.left,
            ys.map(0.0),
            c.margin.left + c.plot_width(),
            ys.map(0.0),
            "#ccc",
            1.0,
        );
        for i in 0..ratios.len() {
            let x1 = xs.map(gx[i] - (ends[i] - starts[i]) / 2.0);
            let x2 = xs.map(gx[i] + (ends[i] - starts[i]) / 2.0);
            let y = ys.map(ratios[i]);
            let y0 = ys.map(0.0);
            let color = if ratios[i] > 0.2 {
                "#e15759"
            } else if ratios[i] < -0.2 {
                "#4e79a7"
            } else {
                "#999"
            };
            c.add_rect(x1, y.min(y0), (x2 - x1).max(1.0), (y - y0).abs(), color);
        }
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_y_axis(&dy, "log2 ratio");
        c.draw_title("Copy Number");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 80);
    let height = get_opt_usize(&opts, "height", 16);
    let mut chart = AsciiChart::new(width, height);
    chart.hline(0.0, yr, '╌');
    for i in 0..ratios.len() {
        let ch = if ratios[i] > 0.2 {
            '▲'
        } else if ratios[i] < -0.2 {
            '▼'
        } else {
            '·'
        };
        chart.put(gx[i], ratios[i], xr, yr, ch);
    }
    write_output(&chart.render("Copy Number"));
    Ok(Value::Nil)
}

// ── 6. violin ───────────────────────────────────────────────────

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

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = SvgCanvas::new(w, h);
        c.margin.bottom = 60.0;
        let ng = groups.len();
        let mut global_min = f64::INFINITY;
        let mut global_max = f64::NEG_INFINITY;
        for (_, vals) in &groups {
            let (lo, hi) = col_range(vals);
            global_min = global_min.min(lo);
            global_max = global_max.max(hi);
        }
        let ys = Scale {
            domain: (global_min, global_max),
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        let group_w = c.plot_width() / ng as f64;
        for (gi, (name, vals)) in groups.iter().enumerate() {
            let bw = silverman_bw(vals);
            let (kde_y, kde_d) = kde(vals, bw, 50);
            let max_d = kde_d.iter().cloned().fold(0.0f64, f64::max);
            let cx = c.margin.left + (gi as f64 + 0.5) * group_w;
            let half_w = group_w * 0.4;
            let mut points_l = String::new();
            let mut points_r = String::new();
            for i in 0..kde_y.len() {
                let y = ys.map(kde_y[i]);
                let dx = if max_d > 0.0 {
                    kde_d[i] / max_d * half_w
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
                name,
                "middle",
                10.0,
            );
        }
        let dy = Scale {
            domain: (global_min, global_max),
            range: (global_min, global_max),
        };
        // Both were hardcoded, so a caller passing {title: gene} got a figure
        // headed "Violin Plot" — the option was accepted and discarded, which
        // is worse than not offering it.
        c.draw_y_axis(&dy, get_opt_str(&opts, "ylab", "Value"));
        c.draw_title(get_opt_str(&opts, "title", "Violin Plot"));
        return Ok(Value::Str(c.render()));
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
        let mut c = SvgCanvas::new(w, h);
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
        c.draw_x_axis(&dx, "Value");
        c.draw_y_axis(&dy, "Density");
        c.draw_title("Density");
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

fn builtin_kaplan_meier(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "kaplan_meier")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let times = extract_table_col(table, get_opt_str(&opts, "time", "time"))?;
    let events = extract_table_col(table, get_opt_str(&opts, "event", "event"))?;

    let mut pairs: Vec<(f64, bool)> = times
        .iter()
        .zip(events.iter())
        .map(|(&t, &e)| (t, e >= 1.0))
        .collect();
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let n = pairs.len();
    let mut surv = 1.0f64;
    let mut curve_t: Vec<f64> = vec![0.0];
    let mut curve_s: Vec<f64> = vec![1.0];
    let mut at_risk = n;
    let mut i = 0;
    while i < n {
        let t = pairs[i].0;
        let mut d = 0usize;
        let mut cc = 0usize;
        while i < n && (pairs[i].0 - t).abs() < f64::EPSILON {
            if pairs[i].1 {
                d += 1;
            } else {
                cc += 1;
            }
            i += 1;
        }
        if d > 0 {
            surv *= 1.0 - d as f64 / at_risk as f64;
            curve_t.push(t);
            curve_s.push(surv);
        }
        at_risk -= d + cc;
    }
    let tmax = pairs.last().map(|p| p.0).unwrap_or(1.0);
    curve_t.push(tmax);
    curve_s.push(surv);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: (0.0, tmax),
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: (0.0, 1.0),
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        for j in 0..curve_t.len() - 1 {
            let x1 = xs.map(curve_t[j]);
            let x2 = xs.map(curve_t[j + 1]);
            let y = ys.map(curve_s[j]);
            c.add_line(x1, y, x2, y, PALETTE[0], 2.0);
            if j + 1 < curve_s.len() {
                c.add_line(x2, y, x2, ys.map(curve_s[j + 1]), PALETTE[0], 2.0);
            }
        }
        let dx = Scale {
            domain: (0.0, tmax),
            range: (0.0, tmax),
        };
        let dy = Scale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        };
        c.draw_x_axis(&dx, "Time");
        c.draw_y_axis(&dy, "Survival");
        c.draw_title("Kaplan-Meier");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 16);
    let mut chart = AsciiChart::new(width, height);
    let xr = (0.0, tmax);
    let yr = (0.0, 1.0);
    for j in 0..curve_t.len() - 1 {
        let steps = ((curve_t[j + 1] - curve_t[j]) / tmax * chart.pw() as f64)
            .ceil()
            .max(1.0) as usize;
        for s in 0..=steps {
            let t = curve_t[j] + (curve_t[j + 1] - curve_t[j]) * s as f64 / steps.max(1) as f64;
            chart.put(t, curve_s[j], xr, yr, '─');
        }
    }
    write_output(&chart.render("Kaplan-Meier"));
    Ok(Value::Nil)
}

// ── 9. forest_plot ──────────────────────────────────────────────

fn builtin_forest_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "forest_plot")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let labels = extract_str_col(table, get_opt_str(&opts, "label", "label"))?;
    let estimates = extract_table_col(table, get_opt_str(&opts, "estimate", "estimate"))?;
    let lowers = extract_table_col(table, get_opt_str(&opts, "lower", "lower"))?;
    let uppers = extract_table_col(table, get_opt_str(&opts, "upper", "upper"))?;
    let n = labels.len();

    let mut all_vals: Vec<f64> = Vec::new();
    all_vals.extend(&lowers);
    all_vals.extend(&uppers);
    let xr0 = col_range(&all_vals);
    let xr = (xr0.0.min(0.0) - 0.1, xr0.1.max(0.0) + 0.1);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", (n as f64 * 30.0 + 80.0).min(800.0));
        let mut c = SvgCanvas::new(w, h);
        c.margin.left = 120.0;
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let row_h = c.plot_height() / n as f64;
        c.add_line(
            xs.map(0.0),
            c.margin.top,
            xs.map(0.0),
            c.margin.top + c.plot_height(),
            "#ccc",
            1.0,
        );
        for j in 0..n {
            let y = c.margin.top + (j as f64 + 0.5) * row_h;
            c.add_line(xs.map(lowers[j]), y, xs.map(uppers[j]), y, PALETTE[0], 2.0);
            c.add_circle(xs.map(estimates[j]), y, 5.0, PALETTE[0]);
            c.add_text(c.margin.left - 5.0, y + 4.0, &labels[j], "end", 10.0);
        }
        let dx = Scale {
            domain: xr,
            range: xr,
        };
        c.draw_x_axis(&dx, "Effect Size");
        c.draw_title("Forest Plot");
        return Ok(Value::Str(c.render()));
    }

    let bar_w = get_opt_usize(&opts, "width", 40);
    let max_label = labels.iter().map(|l| l.len()).max().unwrap_or(4);
    let mut out = String::from("  Forest Plot\n");
    for j in 0..n {
        let mut line = vec![' '; bar_w];
        let map_x = |v: f64| -> usize {
            ((v - xr.0) / (xr.1 - xr.0) * (bar_w - 1) as f64)
                .round()
                .clamp(0.0, (bar_w - 1) as f64) as usize
        };
        line[map_x(0.0)] = '│';
        for x in map_x(lowers[j])..=map_x(uppers[j]) {
            if line[x] == ' ' {
                line[x] = '─';
            }
        }
        line[map_x(estimates[j])] = '◆';
        let s: String = line.into_iter().collect();
        out.push_str(&format!("  {:>w$}  {s}\n", labels[j], w = max_label));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 10. roc_curve ───────────────────────────────────────────────

fn builtin_roc_curve(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "roc_curve")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    // Support precomputed FPR/TPR columns or raw score/label columns
    let has_fpr_tpr = table.col_index("fpr").is_some() && table.col_index("tpr").is_some();

    let (fprs, tprs) = if has_fpr_tpr {
        (
            extract_table_col(table, "fpr")?,
            extract_table_col(table, "tpr")?,
        )
    } else {
        let scores = extract_table_col(table, get_opt_str(&opts, "score", "score"))?;
        let labels = extract_table_col(table, get_opt_str(&opts, "label", "label"))?;
        let mut indices: Vec<usize> = (0..scores.len()).collect();
        indices.sort_by(|&a, &b| scores[b].partial_cmp(&scores[a]).unwrap());
        let total_pos = labels.iter().filter(|&&l| l >= 1.0).count() as f64;
        let total_neg = labels.len() as f64 - total_pos;
        let mut fp_v: Vec<f64> = vec![0.0];
        let mut tp_v: Vec<f64> = vec![0.0];
        let (mut tp, mut fp) = (0.0, 0.0);
        for &idx in &indices {
            if labels[idx] >= 1.0 {
                tp += 1.0;
            } else {
                fp += 1.0;
            }
            tp_v.push(if total_pos > 0.0 { tp / total_pos } else { 0.0 });
            fp_v.push(if total_neg > 0.0 { fp / total_neg } else { 0.0 });
        }
        (fp_v, tp_v)
    };

    let auc_opt = get_opt_f64(&opts, "auc", -1.0);
    let auc = if auc_opt >= 0.0 {
        auc_opt
    } else {
        trapz_auc(&fprs, &tprs)
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: (0.0, 1.0),
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: (0.0, 1.0),
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        c.add_line(
            xs.map(0.0),
            ys.map(0.0),
            xs.map(1.0),
            ys.map(1.0),
            "#ccc",
            1.0,
        );
        let mut pts = String::new();
        for j in 0..fprs.len() {
            pts.push_str(&format!("{:.1},{:.1} ", xs.map(fprs[j]), ys.map(tprs[j])));
        }
        pts.push_str(&format!("{:.1},{:.1}", xs.map(1.0), ys.map(0.0)));
        c.elements.push(format!(
            r#"<polygon points="{pts}" fill="{}" opacity="0.2" />"#,
            PALETTE[0]
        ));
        let lp: String = fprs
            .iter()
            .zip(tprs.iter())
            .map(|(&x, &y)| format!("{:.1},{:.1}", xs.map(x), ys.map(y)))
            .collect::<Vec<_>>()
            .join(" ");
        c.elements.push(format!(
            r#"<polyline points="{lp}" fill="none" stroke="{}" stroke-width="2" />"#,
            PALETTE[0]
        ));
        let d = Scale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        };
        c.draw_x_axis(&d, "FPR");
        c.draw_y_axis(&d, "TPR");
        c.draw_title(&format!("ROC Curve (AUC = {auc:.3})"));
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 40);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    let r = (0.0, 1.0);
    for j in 0..chart.pw() {
        let v = j as f64 / chart.pw() as f64;
        chart.put(v, v, r, r, '╱');
    }
    for j in 0..fprs.len() {
        chart.put(fprs[j], tprs[j], r, r, '●');
    }
    write_output(&chart.render(&format!("ROC Curve (AUC = {auc:.3})")));
    Ok(Value::Nil)
}

// ── 11. clustered_heatmap ───────────────────────────────────────

fn builtin_clustered_heatmap(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let heat_chars: Vec<char> = get_opt_str(&opts, "chars", " ░▒▓█").chars().collect();

    let (row_names, col_names, data) = match &args[0] {
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
            let rn: Vec<String> = (0..nrows).map(|i| format!("row{i}")).collect();
            (rn, table.columns.clone(), t)
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
    let nrows = data.len();
    let ncols = if nrows > 0 { data[0].len() } else { 0 };
    let row_order = nn_order(&data);
    let col_data: Vec<Vec<f64>> = (0..ncols)
        .map(|c| (0..nrows).map(|r| data[r][c]).collect())
        .collect();
    let col_order = nn_order(&col_data);
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

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::new(w, h);
        c.margin.left = 80.0;
        c.margin.bottom = 60.0;
        let cw = c.plot_width() / ncols as f64;
        let ch = c.plot_height() / nrows as f64;
        for (ri, &row_i) in row_order.iter().enumerate() {
            for (ci, &col_i) in col_order.iter().enumerate() {
                let v = data[row_i][col_i];
                let t = if (vmax - vmin).abs() < f64::EPSILON {
                    0.5
                } else {
                    (v - vmin) / (vmax - vmin)
                };
                c.add_rect(
                    c.margin.left + ci as f64 * cw,
                    c.margin.top + ri as f64 * ch,
                    cw,
                    ch,
                    &sequential_color(t),
                );
            }
        }
        for (ri, &row_i) in row_order.iter().enumerate() {
            c.add_text(
                c.margin.left - 3.0,
                c.margin.top + (ri as f64 + 0.5) * ch + 4.0,
                &row_names[row_i],
                "end",
                9.0,
            );
        }
        c.draw_title("Clustered Heatmap");
        return Ok(Value::Str(c.render()));
    }

    let max_rl = row_names.iter().map(|s| s.len()).max().unwrap_or(0);
    let nlevels = heat_chars.len();
    let mut out = String::from("  Clustered Heatmap\n");
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
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut cur = 0;
    visited[0] = true;
    order.push(0);
    for _ in 1..n {
        let mut best = 0;
        let mut best_d = f64::INFINITY;
        for j in 0..n {
            if !visited[j] {
                let d = dist(&data[cur], &data[j]);
                if d < best_d {
                    best_d = d;
                    best = j;
                }
            }
        }
        visited[best] = true;
        order.push(best);
        cur = best;
    }
    order
}

// ── 12. pca_plot ────────────────────────────────────────────────

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
            // Find numeric columns (exclude group_col)
            let mut num_cols: Vec<String> = Vec::new();
            for col in &table.columns {
                if col == &group_col {
                    continue;
                }
                if extract_table_col(table, col).is_ok() {
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

    let xr = col_range(&pc1);
    let yr = col_range(&pc2);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let title = get_opt_str(&opts, "title", "PCA Plot").to_string();
        let mut c = SvgCanvas::new(w, h);
        let x_scale = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let y_scale = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        let mut cm: HashMap<String, usize> = HashMap::new();
        let mut next_ci = 0;
        let mut points: Vec<(f64, f64, &str)> = Vec::with_capacity(pc1.len());
        for j in 0..pc1.len() {
            let ci = labels
                .as_ref()
                .map(|l| {
                    let e = cm.entry(l[j].clone()).or_insert_with(|| {
                        let v = next_ci;
                        next_ci += 1;
                        v
                    });
                    *e
                })
                .unwrap_or(0);
            points.push((
                x_scale.map(pc1[j]),
                y_scale.map(pc2[j]),
                PALETTE[ci % PALETTE.len()],
            ));
        }
        // Points first, then every label, rather than alternating: a later
        // point used to be able to bury an earlier label, and one raster cannot
        // interleave with text at all.
        let raster = raster_choice(&opts, "pca_plot", pc1.len())?;
        let area = c.point_area();
        c.add_scatter(&points, 4.0, area, raster);
        if show_labels {
            if let Some(ref rn) = row_names {
                for j in 0..pc1.len() {
                    c.add_text(
                        x_scale.map(pc1[j]) + 6.0,
                        y_scale.map(pc2[j]) - 4.0,
                        &rn[j],
                        "start",
                        8.0,
                    );
                }
            }
        }
        // Legend for groups
        if let Some(ref lbls) = labels {
            let mut seen: Vec<String> = Vec::new();
            for l in lbls {
                if !seen.contains(l) {
                    seen.push(l.clone());
                }
            }
            for (i, name) in seen.iter().enumerate() {
                let lx = c.margin.left + c.plot_width() - 80.0;
                let ly = c.margin.top + 15.0 + i as f64 * 16.0;
                c.add_circle(lx, ly, 4.0, PALETTE[i % PALETTE.len()]);
                c.add_text(lx + 8.0, ly + 4.0, name, "start", 10.0);
            }
        }
        let dx = Scale {
            domain: xr,
            range: xr,
        };
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_x_axis(&dx, &format!("PC1 ({pct1:.1}%)"));
        c.draw_y_axis(&dy, &format!("PC2 ({pct2:.1}%)"));
        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

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
        let mut c = SvgCanvas::new(w, h);
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
        c.draw_title("OncoPrint");
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
        let mut c = SvgCanvas::new(w, h);
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
        c.draw_title("Venn Diagram");
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
        let mut c = SvgCanvas::new(w, h);
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
        c.draw_title("UpSet Plot");
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
        let mut c = SvgCanvas::new(w, h);
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
        c.draw_title("Sequence Logo");
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
        let mut c = SvgCanvas::new(w, h);
        c.margin.left = 40.0;
        c.margin.right = 100.0;
        let leaves = count_leaves(&root);
        let max_depth = max_tree_depth(&root);
        let ml = c.margin.left;
        let mt = c.margin.top;
        let pw = c.plot_width();
        let ph = c.plot_height();
        draw_tree_svg(&mut c, &root, 0.0, max_depth, 0, leaves, ml, mt, pw, ph);
        c.draw_title("Phylogenetic Tree");
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

fn builtin_lollipop(args: Vec<Value>) -> Result<Value> {
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

fn builtin_circos(args: Vec<Value>) -> Result<Value> {
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
        let mut c = SvgCanvas::new(w, h);
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
        c.draw_title("Hi-C Contact Map");
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

fn builtin_sashimi(args: Vec<Value>) -> Result<Value> {
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

    let title = get_opt_str(&opts, "title", "Distribution").to_string();
    let width = get_opt_f64(&opts, "width", 640.0);
    let height = get_opt_f64(&opts, "height", 420.0);
    let mut canvas = SvgCanvas::new(width, height);

    let shapes = order
        .iter()
        .map(|name| {
            let values = &groups[name];
            gaussian_kde(values, silverman_bandwidth(values), 128)
        })
        .collect::<Vec<_>>();
    let lo = shapes
        .iter()
        .filter_map(|shape| shape.first().map(|point| point.0))
        .fold(f64::INFINITY, f64::min);
    let hi = shapes
        .iter()
        .filter_map(|shape| shape.last().map(|point| point.0))
        .fold(f64::NEG_INFINITY, f64::max);

    let y_scale = Scale {
        domain: (lo, hi),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let slot = canvas.plot_width() / order.len() as f64;

    for (gi, name) in order.iter().enumerate() {
        let values = &groups[name];
        let centre = canvas.margin.left + slot * (gi as f64 + 0.5);
        let shape = &shapes[gi];
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

        canvas.add_text(
            centre,
            canvas.margin.top + canvas.plot_height() + 16.0,
            name,
            "middle",
            10.0,
        );
    }

    canvas.draw_y_axis(&y_scale, &value_col);
    canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
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

    let title = get_opt_str(&opts, "title", "Marker expression").to_string();
    let cell = get_opt_f64(&opts, "cell", 26.0);
    let width = get_opt_f64(&opts, "width", 0.0);
    let height = get_opt_f64(&opts, "height", 0.0);
    // Sized to the grid unless told otherwise: a fixed canvas either crushes 30
    // genes together or strands 3 in a corner.
    let width = if width > 0.0 {
        width
    } else {
        180.0 + cell * cluster_order.len() as f64 + 120.0
    };
    let height = if height > 0.0 {
        height
    } else {
        90.0 + cell * selected.len() as f64
    };
    let mut canvas = SvgCanvas::new(width, height);

    let left = 130.0;
    let top = 60.0;
    let radius_max = cell * 0.42;

    for (column, cluster) in cluster_order.iter().enumerate() {
        let x = left + cell * (column as f64 + 0.5);
        canvas.add_text_rotated(x, top - 10.0, cluster, -45.0, "start", 9.0);
    }

    for (row, &gene) in selected.iter().enumerate() {
        let y = top + cell * (row as f64 + 0.5);
        canvas.add_text(left - 8.0, y + 3.0, &gene_names[gene], "end", 9.0);
        for column in 0..cluster_order.len() {
            let x = left + cell * (column as f64 + 0.5);
            let fraction = detected[row][column];
            if fraction <= 0.0 {
                continue;
            }
            // Area, not radius, tracks the fraction: a radius-linear dot at 50%
            // reads as a quarter of the ink, which under-sells every mid-range
            // gene on the plot.
            let radius = radius_max * fraction.sqrt();
            let t = (scaled[row][column] + CLIP) / (2.0 * CLIP);
            canvas.add_circle(x, y, radius, &sequential_color(t));
        }
    }

    // Two legends, because the figure carries two encodings and a reader cannot
    // guess either.
    let legend_x = left + cell * cluster_order.len() as f64 + 24.0;
    canvas.add_text(legend_x, top + 4.0, "% detected", "start", 9.0);
    for (i, fraction) in [0.25_f64, 0.5, 1.0].iter().enumerate() {
        let y = top + 20.0 + i as f64 * 18.0;
        canvas.add_circle(legend_x + 8.0, y, radius_max * fraction.sqrt(), "#888888");
        canvas.add_text(
            legend_x + 22.0,
            y + 3.0,
            &format!("{:.0}%", fraction * 100.0),
            "start",
            8.0,
        );
    }
    let bar_top = top + 90.0;
    canvas.add_text(legend_x, bar_top - 6.0, "z-score", "start", 9.0);
    for step in 0..24 {
        let t = 1.0 - step as f64 / 23.0;
        canvas.add_rect(
            legend_x,
            bar_top + step as f64 * 3.0,
            10.0,
            3.4,
            &sequential_color(t),
        );
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

    canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    Ok(Value::Str(canvas.render()))
}

fn builtin_umap_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "UMAP").to_string();
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
                .filter_map(|item| match item {
                    Value::Record(map) => map.get(&feature_col).and_then(|v| v.as_float()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    // A partial column would silently mis-colour points, so take it only when
    // there is a value for every point.
    let has_feature = !feature_values.is_empty() && feature_values.len() == xs.len();
    let feature_range = if has_feature {
        col_range(&feature_values)
    } else {
        (0.0, 1.0)
    };

    // Build group -> color index mapping
    let mut group_order: Vec<String> = Vec::new();
    let mut group_map: HashMap<String, usize> = HashMap::new();
    for cl in &color_labels {
        if !cl.is_empty() && !group_map.contains_key(cl) {
            group_map.insert(cl.clone(), group_order.len());
            group_order.push(cl.clone());
        }
    }
    // A feature scale takes over the legend area, so the two never compete.
    let has_groups = !group_order.is_empty() && !has_feature;

    let xr = col_range(&xs);
    let yr = col_range(&ys);
    let xpad = (xr.1 - xr.0) * 0.05 + 0.1;
    let ypad = (yr.1 - yr.0) * 0.05 + 0.1;
    let xr = (xr.0 - xpad, xr.1 + xpad);
    let yr = (yr.0 - ypad, yr.1 + ypad);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 450.0);
        let mut c = SvgCanvas::new(w, h);
        // Reserve right margin space for legend
        let legend_w = if has_groups || has_feature {
            120.0
        } else {
            0.0
        };
        let plot_right = c.margin.left + c.plot_width() - legend_w;
        let x_scale = Scale {
            domain: xr,
            range: (c.margin.left, plot_right),
        };
        let y_scale = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };

        // One <circle> per cell, or one embedded raster for the lot. Vector
        // points are better when there are few: they hover, they select, they
        // scale to any zoom. The threshold and the reasoning behind it live
        // with `raster_choice`, so every scatter switches over at the same
        // size and explains itself the same way.
        let raster = raster_choice(&opts, "umap_plot", xs.len())?;

        let point_color = |i: usize| -> String {
            if has_feature {
                let (lo, hi) = feature_range;
                // A column with no spread would divide by zero; paint it mid-scale.
                let t = if (hi - lo).abs() < 1e-12 {
                    0.5
                } else {
                    (feature_values[i] - lo) / (hi - lo)
                };
                if seurat_theme {
                    seurat_feature_color(t)
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

        let points: Vec<(f64, f64, String)> = (0..xs.len())
            .map(|i| (x_scale.map(xs[i]), y_scale.map(ys[i]), point_color(i)))
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
            if !point_labels[i].is_empty() {
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
            let lx = plot_right + 10.0;
            let mut ly = c.margin.top + 10.0;
            for (gi, gname) in group_order.iter().enumerate() {
                let color = if seurat_theme {
                    SEURAT_PALETTE[gi % SEURAT_PALETTE.len()]
                } else {
                    PALETTE[gi % PALETTE.len()]
                };
                c.add_circle(lx + 5.0, ly + 4.0, 4.0, color);
                c.add_text(lx + 13.0, ly + 8.0, gname, "start", 9.0);
                ly += 16.0;
            }
        }

        // Seurat's `label = TRUE`: place one readable label at the group
        // centre. Medians are less sensitive than means to stray UMAP points.
        if has_groups && label_groups {
            for group in &group_order {
                let mut group_x: Vec<f64> = color_labels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| (label == group).then_some(xs[i]))
                    .collect();
                let mut group_y: Vec<f64> = color_labels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| (label == group).then_some(ys[i]))
                    .collect();
                if group_x.is_empty() {
                    continue;
                }
                group_x.sort_by(f64::total_cmp);
                group_y.sort_by(f64::total_cmp);
                let x = x_scale.map(group_x[group_x.len() / 2]);
                let y = y_scale.map(group_y[group_y.len() / 2]);
                let box_width = 8.0 + group.chars().count() as f64 * 6.2;
                c.add_rect(x - box_width / 2.0, y - 10.0, box_width, 16.0, "#ffffff");
                c.add_text(x, y + 2.0, group, "middle", 10.0);
            }
        }

        // Axis labels
        let dx = Scale {
            domain: xr,
            range: xr,
        };
        let dy = Scale {
            domain: yr,
            range: yr,
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
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for i in 0..xs.len() {
        let ch = if has_groups {
            let ci = group_map.get(&color_labels[i]).copied().unwrap_or(0);
            char::from_digit((ci % 10) as u32, 10).unwrap_or('*')
        } else {
            '*'
        };
        chart.put(xs[i], ys[i], xr, yr, ch);
    }
    let n = xs.len();
    write_output(&chart.render(&format!("{title}  ({n} points)")));
    Ok(Value::Nil)
}

// ── coverage_track ───────────────────────────────────────────────

/// Genome browser-style coverage track (filled area chart).
/// data: Table with columns chrom, start, end, value OR List<Record{pos, value}>
/// options: Record{title?, width?, height?, region_start?, region_end?, color?, format?}
fn builtin_coverage_track(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "Coverage Track").to_string();
    let color = get_opt_str(&opts, "color", "#4e79a7").to_string();

    // Extract (position, value) pairs from data.
    // For interval data (chrom/start/end/value), use midpoint of interval as position.
    let (mut positions, mut values): (Vec<f64>, Vec<f64>) = match &args[0] {
        Value::Table(table) => {
            if table.col_index("pos").is_some() || table.col_index("position").is_some() {
                // pos/value format
                let pos_col = if table.col_index("pos").is_some() {
                    "pos"
                } else {
                    "position"
                };
                let ps = extract_table_col(table, pos_col).unwrap_or_default();
                let vs = extract_table_col(table, "value")
                    .or_else(|_| extract_table_col(table, "coverage"))
                    .or_else(|_| extract_table_col(table, "signal"))
                    .or_else(|_| extract_table_col(table, "score"))
                    .unwrap_or_default();
                (ps, vs)
            } else {
                // chrom/start/end/value interval format — use midpoints
                let starts = extract_table_col(table, "start").unwrap_or_default();
                let ends = extract_table_col(table, "end").unwrap_or_default();
                let vs = extract_table_col(table, "value")
                    .or_else(|_| extract_table_col(table, "coverage"))
                    .or_else(|_| extract_table_col(table, "signal"))
                    .or_else(|_| extract_table_col(table, "score"))
                    .unwrap_or_default();
                let mids: Vec<f64> = starts
                    .iter()
                    .zip(ends.iter())
                    .map(|(s, e)| (s + e) / 2.0)
                    .collect();
                (mids, vs)
            }
        }
        Value::List(items) => {
            let mut ps = Vec::new();
            let mut vs = Vec::new();
            for item in items.iter() {
                if let Value::Record(map) = item {
                    let p = map
                        .get("pos")
                        .or(map.get("position"))
                        .and_then(|v| v.as_float())
                        .or_else(|| {
                            let s = map.get("start").and_then(|v| v.as_float()).unwrap_or(0.0);
                            let e = map.get("end").and_then(|v| v.as_float()).unwrap_or(s);
                            Some((s + e) / 2.0)
                        })
                        .unwrap_or(0.0);
                    let v = map
                        .get("value")
                        .or(map.get("coverage"))
                        .or(map.get("signal"))
                        .or(map.get("score"))
                        .and_then(|v| v.as_float())
                        .unwrap_or(0.0);
                    ps.push(p);
                    vs.push(v);
                }
            }
            (ps, vs)
        }
        _ => {
            return Err(BioLangError::type_error(
                "coverage_track() requires Table or List of Records",
                None,
            ))
        }
    };

    if positions.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() empty data",
            None,
        ));
    }

    // Sort by position
    let mut order: Vec<usize> = (0..positions.len()).collect();
    order.sort_by(|&a, &b| {
        positions[a]
            .partial_cmp(&positions[b])
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    positions = order.iter().map(|&i| positions[i]).collect();
    values = order.iter().map(|&i| values[i]).collect();

    // Apply optional region clipping
    let region_start = opts.get("region_start").and_then(|v| v.as_float());
    let region_end = opts.get("region_end").and_then(|v| v.as_float());
    if let (Some(rs), Some(re)) = (region_start, region_end) {
        let (pos_filtered, val_filtered): (Vec<f64>, Vec<f64>) = positions
            .iter()
            .zip(values.iter())
            .filter_map(|(&p, &v)| {
                if p >= rs && p <= re {
                    Some((p, v))
                } else {
                    None
                }
            })
            .unzip();
        positions = pos_filtered;
        values = val_filtered;
    }

    if positions.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() no data after region filter",
            None,
        ));
    }

    let xr = (*positions.first().unwrap(), *positions.last().unwrap());
    let (_, y_max) = col_range(&values);
    let yr = (0.0, y_max * 1.05 + 1.0);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 700.0);
        let h = get_opt_f64(&opts, "height", 200.0);
        let mut c = SvgCanvas::new(w, h);
        let x_scale = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let y_scale = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        let baseline = y_scale.map(0.0);

        // Build SVG polygon path for filled area
        let mut path_pts: Vec<String> = Vec::new();
        path_pts.push(format!("{:.1},{:.1}", x_scale.map(positions[0]), baseline));
        for i in 0..positions.len() {
            path_pts.push(format!(
                "{:.1},{:.1}",
                x_scale.map(positions[i]),
                y_scale.map(values[i])
            ));
        }
        path_pts.push(format!(
            "{:.1},{:.1}",
            x_scale.map(*positions.last().unwrap()),
            baseline
        ));

        // Fill area with semi-transparent color derived from the base color
        let fill_color = if color.starts_with('#') && color.len() == 7 {
            format!("{}88", &color)
        } else {
            color.to_string()
        };
        c.elements.push(format!(
            r##"<polygon points="{}" fill="{}" stroke="none" />"##,
            path_pts.join(" "),
            fill_color
        ));
        // Draw top line in solid color
        let line_pts: Vec<String> = positions
            .iter()
            .zip(values.iter())
            .map(|(&p, &v)| format!("{:.1},{:.1}", x_scale.map(p), y_scale.map(v)))
            .collect();
        c.elements.push(format!(
            r##"<polyline points="{}" fill="none" stroke="{}" stroke-width="1.5" />"##,
            line_pts.join(" "),
            color
        ));

        let dx = Scale {
            domain: xr,
            range: xr,
        };
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_x_axis(&dx, "Genomic Position");
        c.draw_y_axis(&dy, "Coverage");
        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback — sparse bar chart
    let width = get_opt_usize(&opts, "width", 70);
    let height = get_opt_usize(&opts, "height", 12);
    let mut chart = AsciiChart::new(width, height);
    for (&p, &v) in positions.iter().zip(values.iter()) {
        chart.put(p, v, xr, yr, '▓');
    }
    write_output(&chart.render(&format!("{title}  (max={y_max:.1})")));
    Ok(Value::Nil)
}
