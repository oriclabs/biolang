use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::{HashMap, HashSet};

use crate::builtins::write_output;
use crate::plot::{
    col_range, extract_table_col, get_opt_f64, get_opt_str, parse_options, sequential_color,
    Scale, SvgCanvas, PALETTE,
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
                let mut opts = map.clone();
                opts.remove("data");
                if opts.is_empty() {
                    return vec![data.clone()];
                }
                return vec![data.clone(), Value::Record(opts)];
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

    fn pw(&self) -> usize { self.w - self.ml }
    fn ph(&self) -> usize { self.h - self.mb }

    fn map(&self, x: f64, y: f64, xr: (f64, f64), yr: (f64, f64)) -> (usize, usize) {
        let tx = if (xr.1 - xr.0).abs() < f64::EPSILON { 0.5 } else { (x - xr.0) / (xr.1 - xr.0) };
        let ty = if (yr.1 - yr.0).abs() < f64::EPSILON { 0.5 } else { (y - yr.0) / (yr.1 - yr.0) };
        let gx = self.ml + (tx * (self.pw() - 1) as f64).round().clamp(0.0, (self.pw() - 1) as f64) as usize;
        let gy = (self.ph() - 1) - (ty * (self.ph() - 1) as f64).round().clamp(0.0, (self.ph() - 1) as f64) as usize;
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
        BioLangError::runtime(ErrorKind::TypeError, format!("column '{col}' not found"), None)
    })?;
    Ok(table.rows.iter().map(|row| match &row[idx] {
        Value::Str(s) => s.clone(),
        other => format!("{other}"),
    }).collect())
}

fn require_table_bp<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table, got {}", other.type_of()), None,
        )),
    }
}

fn kde(data: &[f64], bw: f64, n: usize) -> (Vec<f64>, Vec<f64>) {
    let (lo, hi) = col_range(data);
    let m = bw * 3.0;
    let step = (hi - lo + 2.0 * m) / (n - 1).max(1) as f64;
    let xs: Vec<f64> = (0..n).map(|i| (lo - m) + step * i as f64).collect();
    let norm = 1.0 / (data.len() as f64 * bw * (2.0 * std::f64::consts::PI).sqrt());
    let ys: Vec<f64> = xs.iter().map(|&x| {
        data.iter().map(|&d| { let z = (x - d) / bw; (-0.5 * z * z).exp() }).sum::<f64>() * norm
    }).collect();
    (xs, ys)
}

fn silverman_bw(data: &[f64]) -> f64 {
    let n = data.len() as f64;
    if n < 2.0 { return 1.0; }
    let mean = data.iter().sum::<f64>() / n;
    let var = data.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let sd = var.sqrt().max(0.01);
    1.06 * sd * n.powf(-0.2)
}

fn trapz_auc(xs: &[f64], ys: &[f64]) -> f64 {
    xs.windows(2).zip(ys.windows(2)).map(|(xw, yw)| {
        (xw[1] - xw[0]).abs() * (yw[0] + yw[1]) / 2.0
    }).sum()
}

/// Assign genomic x-offsets: returns (genome_x_per_point, chrom_boundaries)
fn genome_x_layout(chroms: &[String], positions: &[f64]) -> (Vec<f64>, Vec<(String, f64, f64)>) {
    let mut chrom_order: Vec<String> = Vec::new();
    let mut chrom_max: HashMap<String, f64> = HashMap::new();
    for (i, c) in chroms.iter().enumerate() {
        chrom_max.entry(c.clone()).and_modify(|m| { if positions[i] > *m { *m = positions[i]; } }).or_insert(positions[i]);
        if !chrom_max.contains_key(c) || !chrom_order.contains(c) {
            if !chrom_order.contains(c) { chrom_order.push(c.clone()); }
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
    let xs: Vec<f64> = chroms.iter().zip(positions.iter())
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
    let nlp: Vec<f64> = pvalues.iter().map(|&p| if p > 0.0 { -(p.log10()) } else { 0.0 }).collect();
    let nlp_thresh = if threshold > 0.0 { -(threshold.log10()) } else { 7.3 };

    let (gx, boundaries) = genome_x_layout(&chroms, &positions);
    let xr = col_range(&gx);
    let (_, ymax) = col_range(&nlp);
    let yr = (0.0, ymax * 1.05);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 1200.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let ys = Scale { domain: yr, range: (c.margin.top + c.plot_height(), c.margin.top) };
        // threshold line
        c.add_line(c.margin.left, ys.map(nlp_thresh), c.margin.left + c.plot_width(), ys.map(nlp_thresh), "#e15759", 1.0);
        for (i, &y) in nlp.iter().enumerate() {
            let ci = boundaries.iter().position(|b| b.0 == chroms[i]).unwrap_or(0);
            let color = PALETTE[ci % PALETTE.len()];
            c.add_circle(xs.map(gx[i]), ys.map(y), 2.5, color);
        }
        let dy = Scale { domain: yr, range: yr };
        c.draw_y_axis(&dy, "-log10(p)");
        c.draw_title("Manhattan Plot");
        // chrom labels
        for (ci, (name, start, end)) in boundaries.iter().enumerate() {
            let mid = xs.map((start + end) / 2.0);
            if ci % 2 == 0 {
                c.add_text(mid, c.margin.top + c.plot_height() + 18.0, name, "middle", 9.0);
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

    let mut pvals: Vec<f64> = vals.into_iter().filter(|v| *v > 0.0 && v.is_finite()).collect();
    pvals.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = pvals.len();
    if n == 0 {
        return Err(BioLangError::runtime(ErrorKind::TypeError, "qq_plot() needs positive p-values", None));
    }

    let observed: Vec<f64> = pvals.iter().map(|p| -(p.log10())).collect();
    let expected: Vec<f64> = (0..n).map(|i| -((i as f64 + 0.5) / n as f64).log10()).collect();
    let max_val = observed.last().copied().unwrap_or(1.0).max(*expected.last().unwrap_or(&1.0));
    let range = (0.0, max_val * 1.05);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale { domain: range, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let ys = Scale { domain: range, range: (c.margin.top + c.plot_height(), c.margin.top) };
        c.add_line(xs.map(0.0), ys.map(0.0), xs.map(max_val), ys.map(max_val), "#ccc", 1.0);
        for i in 0..n {
            c.add_circle(xs.map(expected[i]), ys.map(observed[i]), 3.0, PALETTE[0]);
        }
        let d = Scale { domain: range, range: range };
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
        let stain = stain_col.map(|si| match &table.rows[i][si] {
            Value::Str(s) => s.clone(), _ => String::new(),
        }).unwrap_or_default();
        if !chrom_bands.contains_key(&chroms[i]) { chrom_order.push(chroms[i].clone()); }
        chrom_bands.entry(chroms[i].clone()).or_default().push((starts[i], ends[i], stain));
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
            c.add_text(c.margin.left - 5.0, y + row_h / 2.0 + 4.0, chrom, "end", 11.0);
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
            for b in i0..i1 { bar[b] = ch; }
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
    indices.sort_by(|&a, &b| chroms[a].cmp(&chroms[b]).then(positions[a].partial_cmp(&positions[b]).unwrap()));

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
        let xs = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let ys = Scale { domain: yr, range: (c.margin.top + c.plot_height(), c.margin.top) };
        for i in 0..dists.len() {
            let color = if dists[i] < 3.0 { "#e15759" } else if dists[i] < 5.0 { "#f28e2b" } else { "#76b7b2" };
            c.add_circle(xs.map(gx_all[i]), ys.map(dists[i]), 2.5, color);
        }
        let dy = Scale { domain: yr, range: yr };
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

    let midpoints: Vec<f64> = starts.iter().zip(ends.iter()).map(|(s, e)| (s + e) / 2.0).collect();
    let (gx, _boundaries) = genome_x_layout(&chroms, &midpoints);
    let xr = col_range(&gx);
    let (ylo, yhi) = col_range(&ratios);
    let yabs = ylo.abs().max(yhi.abs()).max(0.5);
    let yr = (-yabs, yabs);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 1000.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let ys = Scale { domain: yr, range: (c.margin.top + c.plot_height(), c.margin.top) };
        c.add_line(c.margin.left, ys.map(0.0), c.margin.left + c.plot_width(), ys.map(0.0), "#ccc", 1.0);
        for i in 0..ratios.len() {
            let x1 = xs.map(gx[i] - (ends[i] - starts[i]) / 2.0);
            let x2 = xs.map(gx[i] + (ends[i] - starts[i]) / 2.0);
            let y = ys.map(ratios[i]);
            let y0 = ys.map(0.0);
            let color = if ratios[i] > 0.2 { "#e15759" } else if ratios[i] < -0.2 { "#4e79a7" } else { "#999" };
            c.add_rect(x1, y.min(y0), (x2 - x1).max(1.0), (y - y0).abs(), color);
        }
        let dy = Scale { domain: yr, range: yr };
        c.draw_y_axis(&dy, "log2 ratio");
        c.draw_title("Copy Number");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 80);
    let height = get_opt_usize(&opts, "height", 16);
    let mut chart = AsciiChart::new(width, height);
    chart.hline(0.0, yr, '╌');
    for i in 0..ratios.len() {
        let ch = if ratios[i] > 0.2 { '▲' } else if ratios[i] < -0.2 { '▼' } else { '·' };
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
        Value::Table(table) => {
            table.columns.iter().map(|col| {
                let vals = extract_table_col(table, col).unwrap_or_default();
                let finite: Vec<f64> = vals.into_iter().filter(|v| v.is_finite()).collect();
                (col.clone(), finite)
            }).filter(|(_, v)| !v.is_empty()).collect()
        }
        _ => return Err(BioLangError::type_error("violin() requires List or Table", None)),
    };
    if groups.is_empty() {
        return Err(BioLangError::runtime(ErrorKind::TypeError, "violin() no data", None));
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
        let ys = Scale { domain: (global_min, global_max), range: (c.margin.top + c.plot_height(), c.margin.top) };
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
                let dx = if max_d > 0.0 { kde_d[i] / max_d * half_w } else { 0.0 };
                points_l.push_str(&format!("{:.1},{:.1} ", cx - dx, y));
                points_r.push_str(&format!("{:.1},{:.1} ", cx + dx, y));
            }
            let all_points = format!("{points_l}{}", points_r.split_whitespace().rev().collect::<Vec<_>>().join(" "));
            c.elements.push(format!(r#"<polygon points="{all_points}" fill="{}" opacity="0.6" />"#, PALETTE[gi % PALETTE.len()]));
            c.add_text(cx, c.margin.top + c.plot_height() + 18.0, name, "middle", 10.0);
        }
        let dy = Scale { domain: (global_min, global_max), range: (global_min, global_max) };
        c.draw_y_axis(&dy, "Value");
        c.draw_title("Violin Plot");
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
        let bars: String = kde_d.iter().map(|&d| {
            let t = if max_d > 0.0 { d / max_d } else { 0.0 };
            let idx = (t * 7.0).round() as usize;
            ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][idx.min(7)]
        }).collect();
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
        Value::Table(table) => {
            table.columns.iter().filter_map(|col| {
                let vals: Vec<f64> = extract_table_col(table, col).ok()?.into_iter().filter(|v| v.is_finite()).collect();
                if vals.is_empty() { None } else { Some((col.clone(), vals)) }
            }).collect()
        }
        _ => return Err(BioLangError::type_error("density() requires List or Table", None)),
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
        let xs = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let ys = Scale { domain: yr, range: (c.margin.top + c.plot_height(), c.margin.top) };
        for (gi, (kx, ky)) in curves.iter().enumerate() {
            let baseline = ys.map(0.0);
            let mut points = String::new();
            points.push_str(&format!("{:.1},{:.1} ", xs.map(kx[0]), baseline));
            for i in 0..kx.len() {
                points.push_str(&format!("{:.1},{:.1} ", xs.map(kx[i]), ys.map(ky[i])));
            }
            points.push_str(&format!("{:.1},{:.1}", xs.map(*kx.last().unwrap()), baseline));
            c.elements.push(format!(r#"<polygon points="{points}" fill="{}" opacity="0.4" />"#, PALETTE[gi % PALETTE.len()]));
        }
        let dx = Scale { domain: xr, range: xr };
        let dy = Scale { domain: yr, range: yr };
        c.draw_x_axis(&dx, "Value");
        c.draw_y_axis(&dy, "Density");
        c.draw_title("Density");
        return Ok(Value::Str(c.render()));
    }

    // ASCII: histogram-style density
    let width = get_opt_usize(&opts, "width", 60);
    let _height = get_opt_usize(&opts, "height", 16);
    let mut out = String::from("  Density\n");
    for (_gi, (name, vals)) in groups.iter().enumerate() {
        let bw = bw_opt.unwrap_or_else(|| silverman_bw(vals));
        let (_kx, ky) = kde(vals, bw, width);
        let max_y = ky.iter().cloned().fold(0.0f64, f64::max);
        let bars: String = ky.iter().map(|&y| {
            let t = if max_y > 0.0 { y / max_y } else { 0.0 };
            let idx = (t * 7.0).round() as usize;
            ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][idx.min(7)]
        }).collect();
        if groups.len() > 1 { out.push_str(&format!("  {name}:\n")); }
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

    let mut pairs: Vec<(f64, bool)> = times.iter().zip(events.iter())
        .map(|(&t, &e)| (t, e >= 1.0)).collect();
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
            if pairs[i].1 { d += 1; } else { cc += 1; }
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
        let xs = Scale { domain: (0.0, tmax), range: (c.margin.left, c.margin.left + c.plot_width()) };
        let ys = Scale { domain: (0.0, 1.0), range: (c.margin.top + c.plot_height(), c.margin.top) };
        for j in 0..curve_t.len() - 1 {
            let x1 = xs.map(curve_t[j]);
            let x2 = xs.map(curve_t[j + 1]);
            let y = ys.map(curve_s[j]);
            c.add_line(x1, y, x2, y, PALETTE[0], 2.0);
            if j + 1 < curve_s.len() {
                c.add_line(x2, y, x2, ys.map(curve_s[j + 1]), PALETTE[0], 2.0);
            }
        }
        let dx = Scale { domain: (0.0, tmax), range: (0.0, tmax) };
        let dy = Scale { domain: (0.0, 1.0), range: (0.0, 1.0) };
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
        let steps = ((curve_t[j + 1] - curve_t[j]) / tmax * chart.pw() as f64).ceil().max(1.0) as usize;
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
        let xs = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let row_h = c.plot_height() / n as f64;
        c.add_line(xs.map(0.0), c.margin.top, xs.map(0.0), c.margin.top + c.plot_height(), "#ccc", 1.0);
        for j in 0..n {
            let y = c.margin.top + (j as f64 + 0.5) * row_h;
            c.add_line(xs.map(lowers[j]), y, xs.map(uppers[j]), y, PALETTE[0], 2.0);
            c.add_circle(xs.map(estimates[j]), y, 5.0, PALETTE[0]);
            c.add_text(c.margin.left - 5.0, y + 4.0, &labels[j], "end", 10.0);
        }
        let dx = Scale { domain: xr, range: xr };
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
            ((v - xr.0) / (xr.1 - xr.0) * (bar_w - 1) as f64).round().clamp(0.0, (bar_w - 1) as f64) as usize
        };
        line[map_x(0.0)] = '│';
        for x in map_x(lowers[j])..=map_x(uppers[j]) { if line[x] == ' ' { line[x] = '─'; } }
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
        (extract_table_col(table, "fpr")?, extract_table_col(table, "tpr")?)
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
            if labels[idx] >= 1.0 { tp += 1.0; } else { fp += 1.0; }
            tp_v.push(if total_pos > 0.0 { tp / total_pos } else { 0.0 });
            fp_v.push(if total_neg > 0.0 { fp / total_neg } else { 0.0 });
        }
        (fp_v, tp_v)
    };

    let auc_opt = get_opt_f64(&opts, "auc", -1.0);
    let auc = if auc_opt >= 0.0 { auc_opt } else { trapz_auc(&fprs, &tprs) };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale { domain: (0.0, 1.0), range: (c.margin.left, c.margin.left + c.plot_width()) };
        let ys = Scale { domain: (0.0, 1.0), range: (c.margin.top + c.plot_height(), c.margin.top) };
        c.add_line(xs.map(0.0), ys.map(0.0), xs.map(1.0), ys.map(1.0), "#ccc", 1.0);
        let mut pts = String::new();
        for j in 0..fprs.len() { pts.push_str(&format!("{:.1},{:.1} ", xs.map(fprs[j]), ys.map(tprs[j]))); }
        pts.push_str(&format!("{:.1},{:.1}", xs.map(1.0), ys.map(0.0)));
        c.elements.push(format!(r#"<polygon points="{pts}" fill="{}" opacity="0.2" />"#, PALETTE[0]));
        let lp: String = fprs.iter().zip(tprs.iter())
            .map(|(&x, &y)| format!("{:.1},{:.1}", xs.map(x), ys.map(y))).collect::<Vec<_>>().join(" ");
        c.elements.push(format!(r#"<polyline points="{lp}" fill="none" stroke="{}" stroke-width="2" />"#, PALETTE[0]));
        let d = Scale { domain: (0.0, 1.0), range: (0.0, 1.0) };
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
    for j in 0..fprs.len() { chart.put(fprs[j], tprs[j], r, r, '●'); }
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
            for col in &table.columns { cols_data.push(extract_table_col(table, col)?); }
            let (nrows, ncols) = (table.num_rows(), table.num_cols());
            let mut t = vec![vec![0.0; ncols]; nrows];
            for c in 0..ncols { for r in 0..nrows { t[r][c] = cols_data[c][r]; } }
            let rn: Vec<String> = (0..nrows).map(|i| format!("row{i}")).collect();
            (rn, table.columns.clone(), t)
        }
        Value::Matrix(m) => {
            let rn = m.row_names.clone().unwrap_or_else(|| (0..m.nrow).map(|i| format!("row{i}")).collect());
            let cn = m.col_names.clone().unwrap_or_else(|| (0..m.ncol).map(|i| format!("col{i}")).collect());
            let mut data = vec![vec![0.0; m.ncol]; m.nrow];
            for r in 0..m.nrow { for c in 0..m.ncol { data[r][c] = m.data[r * m.ncol + c]; } }
            (rn, cn, data)
        }
        _ => return Err(BioLangError::type_error("clustered_heatmap() requires Table or Matrix", None)),
    };
    let nrows = data.len();
    let ncols = if nrows > 0 { data[0].len() } else { 0 };
    let row_order = nn_order(&data);
    let col_data: Vec<Vec<f64>> = (0..ncols).map(|c| (0..nrows).map(|r| data[r][c]).collect()).collect();
    let col_order = nn_order(&col_data);
    let all: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).filter(|v| v.is_finite()).collect();
    let (vmin, vmax) = if all.is_empty() { (0.0, 1.0) } else { col_range(&all) };

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
                let t = if (vmax - vmin).abs() < f64::EPSILON { 0.5 } else { (v - vmin) / (vmax - vmin) };
                c.add_rect(c.margin.left + ci as f64 * cw, c.margin.top + ri as f64 * ch, cw, ch, &sequential_color(t));
            }
        }
        for (ri, &row_i) in row_order.iter().enumerate() {
            c.add_text(c.margin.left - 3.0, c.margin.top + (ri as f64 + 0.5) * ch + 4.0, &row_names[row_i], "end", 9.0);
        }
        c.draw_title("Clustered Heatmap");
        return Ok(Value::Str(c.render()));
    }

    let max_rl = row_names.iter().map(|s| s.len()).max().unwrap_or(0);
    let nlevels = heat_chars.len();
    let mut out = String::from("  Clustered Heatmap\n");
    out.push_str(&format!("  {:>w$}  ", "", w = max_rl));
    for &ci in &col_order { out.push_str(&format!("{} ", &col_names[ci].chars().take(2).collect::<String>())); }
    out.push('\n');
    for &ri in &row_order {
        out.push_str(&format!("  {:>w$}  ", row_names[ri], w = max_rl));
        for &ci in &col_order {
            let t = if (vmax - vmin).abs() < f64::EPSILON { 0.5 } else { (data[ri][ci] - vmin) / (vmax - vmin) };
            out.push(heat_chars[(t * (nlevels - 1) as f64).round().clamp(0.0, (nlevels - 1) as f64) as usize]);
            out.push_str("  ");
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

fn nn_order(data: &[Vec<f64>]) -> Vec<usize> {
    let n = data.len();
    if n == 0 { return vec![]; }
    let dist = |a: &[f64], b: &[f64]| -> f64 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
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
                if d < best_d { best_d = d; best = j; }
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
    let color_col = get_opt_str(&opts, "color", "").to_string();

    let (xs, ys, labels) = match &args[0] {
        Value::Table(table) => {
            if table.num_cols() < 2 {
                return Err(BioLangError::runtime(ErrorKind::TypeError, "pca_plot() needs >= 2 columns", None));
            }
            let xc = get_opt_str(&opts, "x", &table.columns[0]).to_string();
            let yc = get_opt_str(&opts, "y", &table.columns[1]).to_string();
            let xs = extract_table_col(table, &xc)?;
            let ys = extract_table_col(table, &yc)?;
            let lbls = if !color_col.is_empty() && table.col_index(&color_col).is_some() {
                extract_str_col(table, &color_col).ok()
            } else { None };
            (xs, ys, lbls)
        }
        Value::Matrix(m) => {
            if m.ncol < 2 { return Err(BioLangError::runtime(ErrorKind::TypeError, "pca_plot() needs >= 2 columns", None)); }
            let xs: Vec<f64> = (0..m.nrow).map(|r| m.data[r * m.ncol]).collect();
            let ys: Vec<f64> = (0..m.nrow).map(|r| m.data[r * m.ncol + 1]).collect();
            (xs, ys, None)
        }
        _ => return Err(BioLangError::type_error("pca_plot() requires Table or Matrix", None)),
    };
    let xr = col_range(&xs);
    let yr = col_range(&ys);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::new(w, h);
        let x_scale = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let y_scale = Scale { domain: yr, range: (c.margin.top + c.plot_height(), c.margin.top) };
        let mut cm: HashMap<String, usize> = HashMap::new();
        let mut next_ci = 0;
        for j in 0..xs.len() {
            let ci = labels.as_ref().and_then(|l| {
                let e = cm.entry(l[j].clone()).or_insert_with(|| { let v = next_ci; next_ci += 1; v });
                Some(*e)
            }).unwrap_or(0);
            c.add_circle(x_scale.map(xs[j]), y_scale.map(ys[j]), 4.0, PALETTE[ci % PALETTE.len()]);
        }
        let dx = Scale { domain: xr, range: xr };
        let dy = Scale { domain: yr, range: yr };
        c.draw_x_axis(&dx, "PC1");
        c.draw_y_axis(&dy, "PC2");
        c.draw_title("PCA Plot");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for j in 0..xs.len() { chart.put(xs[j], ys[j], xr, yr, '●'); }
    write_output(&chart.render("PCA Plot"));
    Ok(Value::Nil)
}

// ── 13. oncoprint ───────────────────────────────────────────────

fn builtin_oncoprint(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "oncoprint")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let samples = extract_str_col(table, get_opt_str(&opts, "sample", "sample"))?;
    let genes = extract_str_col(table, get_opt_str(&opts, "gene", "gene"))?;
    let mut_types = extract_str_col(table, "type").unwrap_or_else(|_| vec!["mutation".into(); samples.len()]);

    let sample_order: Vec<String> = { let mut s = Vec::new(); for x in &samples { if !s.contains(x) { s.push(x.clone()); } } s };
    let gene_order: Vec<String> = { let mut g = Vec::new(); for x in &genes { if !g.contains(x) { g.push(x.clone()); } } g };
    let mut grid: HashMap<(usize, usize), String> = HashMap::new();
    for j in 0..samples.len() {
        let si = sample_order.iter().position(|s| s == &samples[j]).unwrap();
        let gi = gene_order.iter().position(|g| g == &genes[j]).unwrap();
        grid.insert((gi, si), mut_types[j].clone());
    }

    let type_colors: HashMap<&str, &str> = [("missense", "#e15759"), ("nonsense", "#333"), ("frameshift", "#4e79a7"), ("splice", "#76b7b2"), ("mutation", "#e15759")].into();

    if fmt == "svg" {
        let cell = 12.0;
        let w = get_opt_f64(&opts, "width", (sample_order.len() as f64 * cell + 120.0).max(400.0));
        let h = get_opt_f64(&opts, "height", (gene_order.len() as f64 * cell + 60.0).max(200.0));
        let mut c = SvgCanvas::new(w, h);
        c.margin.left = 100.0;
        let cw = c.plot_width() / sample_order.len().max(1) as f64;
        let ch = c.plot_height() / gene_order.len().max(1) as f64;
        for gi in 0..gene_order.len() {
            let y = c.margin.top + gi as f64 * ch;
            c.add_text(c.margin.left - 3.0, y + ch / 2.0 + 4.0, &gene_order[gi], "end", 10.0);
            for si in 0..sample_order.len() {
                let x = c.margin.left + si as f64 * cw;
                c.add_rect(x, y, cw - 1.0, ch - 1.0, "#f0f0f0");
                if let Some(mt) = grid.get(&(gi, si)) {
                    c.add_rect(x, y + ch * 0.15, cw - 1.0, ch * 0.7, type_colors.get(mt.as_str()).copied().unwrap_or("#e15759"));
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
            out.push(if grid.contains_key(&(gi, si)) { '█' } else { '·' });
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
        Value::Record(map) => {
            map.iter().map(|(name, val)| {
                let items: HashSet<String> = match val {
                    Value::List(items) => items.iter().map(|v| format!("{v}")).collect(),
                    _ => HashSet::new(),
                };
                (name.clone(), items)
            }).collect()
        }
        _ => return Err(BioLangError::type_error("venn() requires Record of Lists", None)),
    };
    if sets.len() < 2 || sets.len() > 4 {
        return Err(BioLangError::runtime(ErrorKind::TypeError, "venn() needs 2-4 sets", None));
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
            _ => vec![(-r * 0.3, -r * 0.3), (r * 0.3, -r * 0.3), (-r * 0.3, r * 0.3), (r * 0.3, r * 0.3)],
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
    for (name, set) in &sets { out.push_str(&format!("  {name}: {} items\n", set.len())); }
    out.push('\n');
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            let inter = set_refs[i].intersection(set_refs[j]).count();
            out.push_str(&format!("  {} ∩ {} = {}\n", names[i], names[j], inter));
        }
    }
    let mut common: HashSet<String> = set_refs[0].clone();
    for s in &set_refs[1..] { common = common.intersection(s).cloned().collect(); }
    out.push_str(&format!("  All: {} shared\n", common.len()));
    write_output(&out);
    Ok(Value::Nil)
}

// ── 15. upset ───────────────────────────────────────────────────

fn builtin_upset(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let sets: Vec<(String, HashSet<String>)> = match &args[0] {
        Value::Record(map) => map.iter().map(|(n, v)| {
            let items: HashSet<String> = match v {
                Value::List(l) => l.iter().map(|x| format!("{x}")).collect(),
                _ => HashSet::new(),
            };
            (n.clone(), items)
        }).collect(),
        _ => return Err(BioLangError::type_error("upset() requires Record of Lists", None)),
    };
    let n = sets.len();
    if n < 2 { return Err(BioLangError::runtime(ErrorKind::TypeError, "upset() needs >= 2 sets", None)); }

    // Compute all intersection combinations
    let all_items: HashSet<String> = sets.iter().flat_map(|(_, s)| s.iter().cloned()).collect();
    let mut combos: Vec<(Vec<bool>, usize)> = Vec::new();
    for mask in 1..(1u32 << n) {
        let membership: Vec<bool> = (0..n).map(|i| mask & (1 << i) != 0).collect();
        let count = all_items.iter().filter(|item| {
            (0..n).all(|i| membership[i] == sets[i].1.contains(*item))
        }).count();
        if count > 0 { combos.push((membership, count)); }
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
            c.add_text(x + bw / 2.0, c.margin.top + bar_area_h - bh - 5.0, &count.to_string(), "middle", 9.0);
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
        Value::List(items) => items.iter().map(|v| match v {
            Value::Str(s) => s.clone(),
            Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => seq.data.clone(),
            _ => String::new(),
        }).filter(|s| !s.is_empty()).collect(),
        _ => return Err(BioLangError::type_error("sequence_logo() requires List of sequences", None)),
    };
    if seqs.is_empty() { return Err(BioLangError::runtime(ErrorKind::TypeError, "sequence_logo() empty input", None)); }
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
        let entropy: f64 = counts.values().map(|&c| {
            let p = c / n;
            if p > 0.0 { -p * p.log2() } else { 0.0 }
        }).sum();
        let ic = max_bits - entropy;
        let mut chars: Vec<(char, f64)> = counts.iter()
            .map(|(&ch, &c)| (ch, (c / n) * ic)).collect();
        chars.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        positions.push(chars);
    }

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", (seq_len as f64 * 30.0 + 80.0).min(1200.0));
        let h = get_opt_f64(&opts, "height", 200.0);
        let mut c = SvgCanvas::new(w, h);
        let col_w = c.plot_width() / seq_len as f64;
        let y_scale = Scale { domain: (0.0, max_bits), range: (c.margin.top + c.plot_height(), c.margin.top) };
        let char_colors: HashMap<char, &str> = [('A', "#4caf50"), ('T', "#f44336"), ('U', "#f44336"), ('G', "#ff9800"), ('C', "#2196f3")].into();
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
        let dy = Scale { domain: (0.0, max_bits), range: (0.0, max_bits) };
        c.draw_y_axis(&dy, "bits");
        c.draw_title("Sequence Logo");
        return Ok(Value::Str(c.render()));
    }

    // ASCII logo: show top char per position with height indicator
    let mut out = String::from("  Sequence Logo\n  ");
    for (_pos, chars) in positions.iter().enumerate() {
        if let Some(&(ch, _)) = chars.last() { out.push(ch); } else { out.push(' '); }
    }
    out.push_str("\n  ");
    for chars in positions.iter() {
        let total_ic: f64 = chars.iter().map(|(_, h)| h).sum();
        let bar = if total_ic > max_bits * 0.75 { '█' } else if total_ic > max_bits * 0.5 { '▄' } else if total_ic > max_bits * 0.25 { '▂' } else { '▁' };
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
            if pos >= data.len() || data[pos] != b',' { break; }
            pos += 1; // skip ','
        }
        if pos < data.len() && data[pos] == b')' { pos += 1; }
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
        while pos < data.len() && (data[pos].is_ascii_digit() || data[pos] == b'.' || data[pos] == b'-' || data[pos] == b'e' || data[pos] == b'E') {
            pos += 1;
        }
        if let Ok(v) = std::str::from_utf8(&data[start..pos]).unwrap_or("0").parse::<f64>() { bl = v; }
    }
    Ok((TreeNode { name: name.trim().to_string(), branch_len: bl, children }, pos))
}

fn builtin_phylo_tree(args: Vec<Value>) -> Result<Value> {
    let newick = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => return Err(BioLangError::type_error("phylo_tree() requires Str (Newick format)", None)),
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
    if node.children.is_empty() { 1 } else { node.children.iter().map(count_leaves).sum() }
}

fn max_tree_depth(node: &TreeNode) -> f64 {
    if node.children.is_empty() { node.branch_len }
    else { node.branch_len + node.children.iter().map(max_tree_depth).fold(0.0f64, f64::max) }
}

fn render_tree_ascii(node: &TreeNode, out: &mut String, prefix: &str, is_last: bool) {
    let connector = if prefix.is_empty() { "" } else if is_last { "└── " } else { "├── " };
    let label = if node.name.is_empty() { String::new() } else { format!(" {}", node.name) };
    let bl = if node.branch_len > 0.0 { format!(":{:.4}", node.branch_len) } else { String::new() };
    out.push_str(&format!("  {prefix}{connector}{label}{bl}\n"));
    let child_prefix = if prefix.is_empty() { String::new() } else if is_last { format!("{prefix}    ") } else { format!("{prefix}│   ") };
    for (i, child) in node.children.iter().enumerate() {
        render_tree_ascii(child, out, &child_prefix, i == node.children.len() - 1);
    }
}

fn draw_tree_svg(c: &mut SvgCanvas, node: &TreeNode, x: f64, max_d: f64, leaf_idx: usize, total_leaves: usize, left: f64, top: f64, pw: f64, ph: f64) -> (f64, usize) {
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
        let (cy, new_li) = draw_tree_svg(c, child, child_x, max_d, li, total_leaves, left, top, pw, ph);
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
    let heights = extract_table_col(table, "count").or_else(|_| extract_table_col(table, "height")).ok();

    let xr = col_range(&positions);
    let ys = heights.as_ref().map(|h| col_range(h)).unwrap_or((0.0, 1.0));
    let yr = (0.0, ys.1 * 1.1);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };
        let y_scale = Scale { domain: yr, range: (c.margin.top + c.plot_height(), c.margin.top) };
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
        let dx = Scale { domain: xr, range: xr };
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
        for y in gy..base_y { chart.grid[y][gx] = '│'; }
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
        _ => return Err(BioLangError::type_error("circos() requires Record with 'segments' and 'links' Tables", None)),
    };

    let seg_table = match &segments {
        Value::Table(t) => t,
        _ => return Err(BioLangError::type_error("circos() 'segments' must be a Table", None)),
    };
    let chroms = extract_str_col(seg_table, "chrom")?;
    let ends = extract_table_col(seg_table, "end")?;

    // Compute chrom sizes
    let mut chrom_sizes: Vec<(String, f64)> = Vec::new();
    let mut seen: HashMap<String, f64> = HashMap::new();
    for (i, c) in chroms.iter().enumerate() {
        let e = seen.entry(c.clone()).or_insert(0.0);
        if ends[i] > *e { *e = ends[i]; }
    }
    let mut chrom_order: Vec<String> = Vec::new();
    for c in &chroms { if !chrom_order.contains(c) { chrom_order.push(c.clone()); } }
    for c in &chrom_order { chrom_sizes.push((c.clone(), *seen.get(c).unwrap_or(&1.0))); }
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
                    if let (Some(&(a1s, a1e)), Some(&(a2s, a2e))) = (chrom_angles.get(&c1[i]), chrom_angles.get(&c2[i])) {
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
                out.push_str(&format!("    {}:{:.0} → {}:{:.0}\n", c1[i], s1[i], c2[i], s2[i]));
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
            let names = m.row_names.clone().unwrap_or_else(|| (0..m.nrow).map(|i| format!("{i}")).collect());
            let mut data = vec![vec![0.0; m.ncol]; m.nrow];
            for r in 0..m.nrow { for c in 0..m.ncol { data[r][c] = m.data[r * m.ncol + c]; } }
            (names, data)
        }
        Value::Table(table) => {
            let mut cols_data: Vec<Vec<f64>> = Vec::new();
            for col in &table.columns { cols_data.push(extract_table_col(table, col)?); }
            let (nrows, ncols) = (table.num_rows(), table.num_cols());
            let mut t = vec![vec![0.0; ncols]; nrows];
            for c in 0..ncols { for r in 0..nrows { t[r][c] = cols_data[c][r]; } }
            let names: Vec<String> = (0..nrows).map(|i| format!("{i}")).collect();
            (names, t)
        }
        _ => return Err(BioLangError::type_error("hic_map() requires Matrix or Table", None)),
    };

    let n = data.len();
    let all: Vec<f64> = data.iter().flat_map(|r| r.iter().copied()).filter(|v| v.is_finite() && *v > 0.0).collect();
    let (vmin, vmax) = if all.is_empty() { (0.0, 1.0) } else { col_range(&all) };

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
                let t = if (vmax - vmin).abs() < f64::EPSILON { 0.5 } else { ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0) };
                let color = sequential_color(t);
                c.add_rect(c.margin.left + col as f64 * cell, c.margin.top + r as f64 * cell, cell, cell, &color);
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
            if col < r { out.push_str("  "); continue; }
            let v = data[r][col];
            let t = if (vmax - vmin).abs() < f64::EPSILON { 0.5 } else { ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0) };
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
            let cov = map.get("coverage").and_then(|v| if let Value::Table(t) = v { Some(t) } else { None });
            let junc = map.get("junctions").and_then(|v| if let Value::Table(t) = v { Some(t) } else { None });
            (cov.cloned(), junc.cloned())
        }
        Value::Table(t) => (None, Some(t.clone())),
        _ => return Err(BioLangError::type_error("sashimi() requires Record or Table", None)),
    };

    let junc_table = junctions.as_ref().ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "sashimi() needs junctions data", None)
    })?;
    let j_starts = extract_table_col(junc_table, "start")?;
    let j_ends = extract_table_col(junc_table, "end")?;
    let j_counts = extract_table_col(junc_table, "count").unwrap_or_else(|_| vec![1.0; j_starts.len()]);

    let mut all_pos: Vec<f64> = Vec::new();
    all_pos.extend(&j_starts);
    all_pos.extend(&j_ends);
    if let Some(ref ct) = cov_data {
        if let Ok(ps) = extract_table_col(ct, "pos") { all_pos.extend(&ps); }
    }
    let xr = col_range(&all_pos);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale { domain: xr, range: (c.margin.left, c.margin.left + c.plot_width()) };

        // Coverage area
        if let Some(ref ct) = cov_data {
            if let (Ok(ps), Ok(ds)) = (extract_table_col(ct, "pos"), extract_table_col(ct, "depth")) {
                let max_d = ds.iter().cloned().fold(0.0f64, f64::max).max(1.0);
                let cov_h = c.plot_height() * 0.5;
                let base_y = c.margin.top + cov_h;
                let mut pts = format!("{:.1},{:.1} ", xs.map(ps[0]), base_y);
                for i in 0..ps.len() {
                    let y = base_y - (ds[i] / max_d) * cov_h;
                    pts.push_str(&format!("{:.1},{:.1} ", xs.map(ps[i]), y));
                }
                pts.push_str(&format!("{:.1},{:.1}", xs.map(*ps.last().unwrap()), base_y));
                c.elements.push(format!(r##"<polygon points="{pts}" fill="#ccc" opacity="0.5" />"##));
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
            c.add_text(mid_x, arc_base - arc_h - 5.0, &format!("{:.0}", j_counts[i]), "middle", 9.0);
        }

        let dx = Scale { domain: xr, range: xr };
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
            for i in 0..width { if counts[i] > 0 { bins[i] /= counts[i] as f64; } }
            out.push_str(&format!("  Depth: {}\n", spark_str(&bins)));
        }
    }

    // Junction list
    out.push_str(&format!("  Junctions ({}):\n", j_starts.len()));
    for i in 0..j_starts.len().min(15) {
        out.push_str(&format!("    {:.0}─{:.0} ({:.0} reads) ⌒\n", j_starts[i], j_ends[i], j_counts[i]));
    }
    write_output(&out);
    Ok(Value::Nil)
}

