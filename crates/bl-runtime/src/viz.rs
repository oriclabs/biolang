use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Value};
use std::collections::HashMap;

use crate::builtins::write_output;
use crate::plot::{
    col_range, extract_table_col, get_opt_f64, get_opt_str, parse_options, Scale, SvgCanvas,
};

// ── Registration ────────────────────────────────────────────────

pub fn viz_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("sparkline", Arity::Exact(1)),
        ("bar_chart", Arity::Range(1, 2)),
        ("boxplot", Arity::Range(1, 2)),
        ("heatmap_ascii", Arity::Range(1, 2)),
        ("coverage", Arity::Range(1, 4)),
        ("dotplot", Arity::Range(2, 3)),
        ("alignment_view", Arity::Range(1, 2)),
        ("quality_plot", Arity::Range(1, 2)),
    ]
}

pub fn is_viz_builtin(name: &str) -> bool {
    matches!(
        name,
        "sparkline"
            | "bar_chart"
            | "boxplot"
            | "heatmap_ascii"
            | "coverage"
            | "dotplot"
            | "alignment_view"
            | "quality_plot"
    )
}

pub fn call_viz_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "sparkline" => builtin_sparkline(args),
        "bar_chart" => builtin_bar_chart(args),
        "boxplot" => builtin_boxplot(args),
        "heatmap_ascii" => builtin_heatmap_ascii(args),
        "coverage" => builtin_coverage(args),
        "dotplot" => builtin_dotplot(args),
        "alignment_view" => builtin_alignment_view(args),
        "quality_plot" => builtin_quality_plot(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown viz builtin '{name}'"),
            None,
        )),
    }
}

// ── Shared helpers ──────────────────────────────────────────────

const SPARK_CHARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub(crate) fn nums_from_value(val: &Value, func: &str) -> Result<Vec<f64>> {
    match val {
        Value::List(items) => {
            let mut v = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Int(n) => v.push(*n as f64),
                    Value::Float(f) => v.push(*f),
                    _ => {
                        return Err(BioLangError::type_error(
                            format!("{func}() list must contain numbers"),
                            None,
                        ))
                    }
                }
            }
            Ok(v)
        }
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List of numbers"),
            None,
        )),
    }
}

pub(crate) fn spark_str(vals: &[f64]) -> String {
    if vals.is_empty() {
        return String::new();
    }
    let (lo, hi) = col_range(vals);
    let span = hi - lo;
    vals.iter()
        .map(|&v| {
            if span < f64::EPSILON {
                SPARK_CHARS[3] // middle block for all-equal
            } else {
                let idx = (((v - lo) / span) * 7.0).round() as usize;
                SPARK_CHARS[idx.min(7)]
            }
        })
        .collect()
}

pub(crate) fn get_opt_usize(opts: &HashMap<String, Value>, key: &str, default: usize) -> usize {
    opts.get(key)
        .and_then(|v| match v {
            Value::Int(n) => Some(*n as usize),
            Value::Float(f) => Some(*f as usize),
            _ => None,
        })
        .unwrap_or(default)
}

pub(crate) fn get_opt_bool(opts: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    opts.get(key)
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(default)
}

fn extract_seq_str(val: &Value, func: &str) -> Result<String> {
    match val {
        Value::DNA(seq) | Value::RNA(seq) | Value::Protein(seq) => Ok(seq.data.clone()),
        Value::Str(s) => Ok(s.clone()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires DNA/RNA/Protein/Str sequence"),
            None,
        )),
    }
}

// ── 1. sparkline ────────────────────────────────────────────────

fn builtin_sparkline(args: Vec<Value>) -> Result<Value> {
    let vals = nums_from_value(&args[0], "sparkline")?;
    Ok(Value::Str(spark_str(&vals)))
}

// ── 2. bar_chart ────────────────────────────────────────────────

fn builtin_bar_chart(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_usize(&opts, "width", 40);
    let sort = get_opt_bool(&opts, "sort", false);
    let limit = get_opt_usize(&opts, "limit", 20);

    let mut entries: Vec<(String, f64)> = match &args[0] {
        Value::Record(map) => map
            .iter()
            .map(|(k, v)| (k.clone(), v.as_float().unwrap_or(0.0)))
            .collect(),
        Value::Table(table) => {
            let label_col = get_opt_str(&opts, "label", &table.columns[0]).to_string();
            let value_col = if table.num_cols() > 1 {
                get_opt_str(&opts, "value", &table.columns[1]).to_string()
            } else {
                get_opt_str(&opts, "value", &table.columns[0]).to_string()
            };
            let li = table.col_index(&label_col).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("bar_chart(): column '{label_col}' not found"),
                    None,
                )
            })?;
            let vi = table.col_index(&value_col).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("bar_chart(): column '{value_col}' not found"),
                    None,
                )
            })?;
            table
                .rows
                .iter()
                .map(|row| {
                    let label = match &row[li] {
                        Value::Str(s) => s.clone(),
                        other => format!("{other}"),
                    };
                    let val = row[vi].as_float().unwrap_or(0.0);
                    (label, val)
                })
                .collect()
        }
        Value::List(items) => {
            // List of Records with label/value-like fields (e.g., from head() on a kmer stream)
            items.iter().filter_map(|item| {
                if let Value::Record(rec) = item {
                    // Try common label/value field patterns
                    let label = rec.get("kmer").or_else(|| rec.get("name")).or_else(|| rec.get("label"))
                        .or_else(|| rec.get("key")).or_else(|| rec.get("id"));
                    let value = rec.get("count").or_else(|| rec.get("value")).or_else(|| rec.get("score"))
                        .or_else(|| rec.get("n"));
                    match (label, value) {
                        (Some(l), Some(v)) => Some((format!("{l}"), v.as_float().unwrap_or(0.0))),
                        _ => None,
                    }
                } else {
                    None
                }
            }).collect()
        }
        Value::Stream(s) => {
            // Consume up to `limit` items from the stream
            let mut entries = Vec::new();
            for _ in 0..limit {
                match s.next() {
                    Some(Value::Record(rec)) => {
                        let label = rec.get("kmer").or_else(|| rec.get("name")).or_else(|| rec.get("label"))
                            .or_else(|| rec.get("key")).or_else(|| rec.get("id"));
                        let value = rec.get("count").or_else(|| rec.get("value")).or_else(|| rec.get("score"))
                            .or_else(|| rec.get("n"));
                        if let (Some(l), Some(v)) = (label, value) {
                            entries.push((format!("{l}"), v.as_float().unwrap_or(0.0)));
                        }
                    }
                    Some(_) => {}
                    None => break,
                }
            }
            entries
        }
        _ => {
            return Err(BioLangError::type_error(
                "bar_chart() requires Record, Table, List, or Stream",
                None,
            ))
        }
    };

    if sort {
        entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    }
    if entries.len() > limit {
        entries.truncate(limit);
    }

    let max_val = entries
        .iter()
        .map(|(_, v)| *v)
        .fold(f64::NEG_INFINITY, f64::max);
    let max_label = entries.iter().map(|(l, _)| l.len()).max().unwrap_or(0);

    let mut out = String::new();
    for (label, val) in &entries {
        let bar_len = if max_val > 0.0 {
            ((*val / max_val) * width as f64).round() as usize
        } else {
            0
        };
        let bar: String = "█".repeat(bar_len);
        out.push_str(&format!(
            "  {:>w$} |{} {}\n",
            label,
            bar,
            format_num(*val),
            w = max_label
        ));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 3. boxplot ──────────────────────────────────────────────────

fn builtin_boxplot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_usize(&opts, "width", 60);

    match &args[0] {
        Value::List(_) => {
            let vals = nums_from_value(&args[0], "boxplot")?;
            if vals.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "boxplot() empty list",
                    None,
                ));
            }
            render_boxplot("values", &vals, width);
        }
        Value::Table(table) => {
            for col in &table.columns {
                let vals = extract_table_col(table, col)?;
                let finite: Vec<f64> = vals.into_iter().filter(|v| v.is_finite()).collect();
                if finite.is_empty() {
                    continue;
                }
                render_boxplot(col, &finite, width);
            }
        }
        _ => {
            return Err(BioLangError::type_error(
                "boxplot() requires List or Table",
                None,
            ))
        }
    }

    Ok(Value::Nil)
}

fn render_boxplot(name: &str, vals: &[f64], width: usize) {
    let mut sorted: Vec<f64> = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted.len();
    let lo = sorted[0];
    let hi = sorted[n - 1];
    let q1 = sorted[n / 4];
    let med = sorted[n / 2];
    let q3 = sorted[3 * n / 4];
    let mean = vals.iter().sum::<f64>() / n as f64;
    let span = hi - lo;

    let mut out = String::new();
    out.push_str(&format!("  {name}\n"));

    if span < f64::EPSILON {
        // All values the same
        let mid = width / 2;
        let line: String = " ".repeat(mid) + "|";
        out.push_str(&format!("  {line}\n"));
    } else {
        // Build the box-whisker line
        let map = |v: f64| -> usize { ((v - lo) / span * (width - 1) as f64).round() as usize };
        let ilo = map(lo);
        let iq1 = map(q1);
        let imed = map(med);
        let iq3 = map(q3);
        let ihi = map(hi);

        let mut chars: Vec<char> = vec![' '; width];
        // Whiskers
        for i in ilo..=iq1 {
            chars[i] = '─';
        }
        for i in iq3..=ihi {
            chars[i] = '─';
        }
        chars[ilo] = '├';
        chars[ihi] = '┤';
        // Box
        chars[iq1] = '[';
        chars[iq3] = ']';
        // Fill box interior
        for i in (iq1 + 1)..iq3 {
            chars[i] = '─';
        }
        // Median
        chars[imed] = '|';

        let line: String = chars.into_iter().collect();
        out.push_str(&format!("  {line}\n"));
    }

    out.push_str(&format!(
        "  {:.2}{:>w$}\n",
        lo,
        format!("{:.2}", hi),
        w = width - format!("{:.2}", lo).len()
    ));
    out.push_str(&format!(
        "  n={}  mean={:.2}  median={:.2}  Q1={:.2}  Q3={:.2}\n",
        n, mean, med, q1, q3
    ));
    write_output(&out);
}

// ── 4. heatmap_ascii ────────────────────────────────────────────

fn builtin_heatmap_ascii(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let heat_chars: Vec<char> = get_opt_str(&opts, "chars", " ░▒▓█").chars().collect();

    let (row_names, col_names, data) = match &args[0] {
        Value::Table(table) => {
            let mut data: Vec<Vec<f64>> = Vec::new();
            let row_names: Vec<String> = (0..table.num_rows()).map(|i| format!("row{i}")).collect();
            let col_names: Vec<String> = table.columns.clone();
            for col in &table.columns {
                data.push(extract_table_col(table, col)?);
            }
            // Transpose so data[row][col]
            let nrows = table.num_rows();
            let ncols = table.num_cols();
            let mut transposed = vec![vec![0.0; ncols]; nrows];
            for c in 0..ncols {
                for r in 0..nrows {
                    transposed[r][c] = data[c][r];
                }
            }
            (row_names, col_names, transposed)
        }
        Value::Matrix(m) => {
            let row_names: Vec<String> = m
                .row_names
                .clone()
                .unwrap_or_else(|| (0..m.nrow).map(|i| format!("row{i}")).collect());
            let col_names: Vec<String> = m
                .col_names
                .clone()
                .unwrap_or_else(|| (0..m.ncol).map(|i| format!("col{i}")).collect());
            let mut data = vec![vec![0.0; m.ncol]; m.nrow];
            for r in 0..m.nrow {
                for c in 0..m.ncol {
                    data[r][c] = m.data[r * m.ncol + c];
                }
            }
            (row_names, col_names, data)
        }
        Value::List(rows) => {
            let mut data: Vec<Vec<f64>> = Vec::new();
            for (i, row_val) in rows.iter().enumerate() {
                match row_val {
                    Value::List(cols) => {
                        let row: Vec<f64> = cols.iter().map(|v| match v {
                            Value::Int(n) => *n as f64,
                            Value::Float(f) => *f,
                            _ => f64::NAN,
                        }).collect();
                        data.push(row);
                    }
                    _ => return Err(BioLangError::type_error(
                        &format!("heatmap_ascii() row {i} is not a list"),
                        None,
                    )),
                }
            }
            let nrows = data.len();
            let ncols = if nrows > 0 { data[0].len() } else { 0 };
            let row_names: Vec<String> = (0..nrows).map(|i| format!("row{i}")).collect();
            let col_names: Vec<String> = (0..ncols).map(|i| format!("col{i}")).collect();
            (row_names, col_names, data)
        }
        _ => {
            return Err(BioLangError::type_error(
                "heatmap_ascii() requires Table, Matrix, or nested List",
                None,
            ))
        }
    };

    // Find global range
    let mut all: Vec<f64> = Vec::new();
    for row in &data {
        for &v in row {
            if v.is_finite() {
                all.push(v);
            }
        }
    }
    let (vmin, vmax) = col_range(&all);
    let nlevels = heat_chars.len();

    // Compute max label width
    let max_row_label = row_names.iter().map(|s| s.len()).max().unwrap_or(0);

    let mut out = String::new();
    // Column header
    out.push_str(&format!(
        "  {:>w$}  ",
        "",
        w = max_row_label
    ));
    for col in &col_names {
        // Show first 2 chars of each column
        let label: String = col.chars().take(2).collect();
        out.push_str(&format!("{label} "));
    }
    out.push('\n');

    for (ri, row) in data.iter().enumerate() {
        out.push_str(&format!("  {:>w$}  ", row_names[ri], w = max_row_label));
        for &v in row {
            let t = if (vmax - vmin).abs() < f64::EPSILON {
                0.5
            } else {
                (v - vmin) / (vmax - vmin)
            };
            let idx = (t * (nlevels - 1) as f64).round() as usize;
            let ch = heat_chars[idx.min(nlevels - 1)];
            out.push_str(&format!("{ch}  "));
        }
        out.push('\n');
    }

    write_output(&out);
    Ok(Value::Nil)
}

// ── 5. coverage ─────────────────────────────────────────────────

fn builtin_coverage(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_usize(&opts, "width", 80);

    match &args[0] {
        Value::List(items) if !items.is_empty() && matches!(&items[0], Value::List(_)) => {
            // List<List<Int>> — interval pairs [start, end]
            let mut intervals: Vec<(i64, i64)> = Vec::new();
            for item in items {
                if let Value::List(pair) = item {
                    if pair.len() >= 2 {
                        let s = pair[0].as_float().unwrap_or(0.0) as i64;
                        let e = pair[1].as_float().unwrap_or(0.0) as i64;
                        intervals.push((s, e));
                    }
                }
            }

            // Determine region from args or data
            let chrom_label = if args.len() > 1 {
                if let Value::Str(s) = &args[1] { s.clone() } else { "region".to_string() }
            } else { "region".to_string() };
            let region_start = if args.len() > 2 { args[2].as_float().unwrap_or(0.0) as i64 }
                else { intervals.iter().map(|i| i.0).min().unwrap_or(0) };
            let region_end = if args.len() > 3 { args[3].as_float().unwrap_or(0.0) as i64 }
                else { intervals.iter().map(|i| i.1).max().unwrap_or(1) };

            let span = (region_end - region_start).max(1) as usize;
            let mut depth = vec![0i64; span];
            for (s, e) in &intervals {
                let lo = ((*s - region_start).max(0)) as usize;
                let hi = ((*e - region_start).min(span as i64)) as usize;
                for pos in lo..hi.min(span) {
                    depth[pos] += 1;
                }
            }

            // Bin to display width
            let bin_size = (span as f64 / width as f64).max(1.0);
            let mut binned: Vec<f64> = Vec::new();
            let mut i = 0.0;
            while (i as usize) < span {
                let lo = i as usize;
                let hi = ((i + bin_size) as usize).min(span);
                if lo < hi {
                    let sum: i64 = depth[lo..hi].iter().sum();
                    binned.push(sum as f64 / (hi - lo) as f64);
                }
                i += bin_size;
            }

            let line = spark_str(&binned);
            let max_depth = depth.iter().max().unwrap_or(&0);
            let mean_depth = if span > 0 { depth.iter().sum::<i64>() as f64 / span as f64 } else { 0.0 };
            let mut out = String::new();
            out.push_str(&format!("  {chrom_label}:{region_start}-{region_end}\n"));
            out.push_str(&format!("  {line}\n"));
            out.push_str(&format!("  max_depth={max_depth}  mean_depth={mean_depth:.1}  intervals={}\n", intervals.len()));
            write_output(&out);
        }
        Value::List(_) => {
            let vals = nums_from_value(&args[0], "coverage")?;
            let binned = bin_values(&vals, width);
            let line = spark_str(&binned);
            write_output(&format!("  {line}\n"));
        }
        Value::Table(table) => {
            // bedGraph format: chrom, start, end, value
            let chrom_filter = get_opt_str(&opts, "chrom", "").to_string();
            let chrom_idx = table.col_index("chrom").or_else(|| table.col_index("chr"));
            let start_idx = table.col_index("start");
            let end_idx = table.col_index("end");
            let val_idx = table.col_index("value").or_else(|| table.col_index("score"));

            if chrom_idx.is_none() || start_idx.is_none() || end_idx.is_none() || val_idx.is_none()
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "coverage() table needs chrom/start/end/value columns",
                    None,
                ));
            }

            let ci = chrom_idx.unwrap();
            let si = start_idx.unwrap();
            let ei = end_idx.unwrap();
            let vi = val_idx.unwrap();

            // Group by chrom
            let mut chroms: Vec<String> = Vec::new();
            let mut chrom_data: HashMap<String, Vec<(f64, f64, f64)>> = HashMap::new();

            for row in &table.rows {
                let chrom = match &row[ci] {
                    Value::Str(s) => s.clone(),
                    _ => continue,
                };
                if !chrom_filter.is_empty() && chrom != chrom_filter {
                    continue;
                }
                let start = row[si].as_float().unwrap_or(0.0);
                let end = row[ei].as_float().unwrap_or(0.0);
                let val = row[vi].as_float().unwrap_or(0.0);

                if !chrom_data.contains_key(&chrom) {
                    chroms.push(chrom.clone());
                }
                chrom_data.entry(chrom).or_default().push((start, end, val));
            }

            let max_chrom_len = chroms.iter().map(|c| c.len()).max().unwrap_or(4);

            for chrom in &chroms {
                if let Some(regions) = chrom_data.get(chrom) {
                    let global_start = regions.iter().map(|r| r.0).fold(f64::INFINITY, f64::min);
                    let global_end = regions
                        .iter()
                        .map(|r| r.1)
                        .fold(f64::NEG_INFINITY, f64::max);
                    let span = global_end - global_start;
                    if span <= 0.0 {
                        continue;
                    }

                    let mut bins = vec![0.0f64; width];
                    for &(start, end, val) in regions {
                        let b0 = ((start - global_start) / span * width as f64) as usize;
                        let b1 = ((end - global_start) / span * width as f64).ceil() as usize;
                        for b in b0..b1.min(width) {
                            bins[b] += val;
                        }
                    }

                    let line = spark_str(&bins);
                    write_output(&format!("  {:>w$}  {line}\n", chrom, w = max_chrom_len));
                }
            }
        }
        _ => {
            return Err(BioLangError::type_error(
                "coverage() requires List or bedGraph Table",
                None,
            ))
        }
    }

    Ok(Value::Nil)
}

fn bin_values(vals: &[f64], nbins: usize) -> Vec<f64> {
    if vals.is_empty() || nbins == 0 {
        return vec![0.0; nbins];
    }
    let mut bins = vec![0.0; nbins];
    let mut counts = vec![0usize; nbins];
    let chunk = vals.len() as f64 / nbins as f64;
    for (i, &v) in vals.iter().enumerate() {
        let b = (i as f64 / chunk) as usize;
        let b = b.min(nbins - 1);
        bins[b] += v;
        counts[b] += 1;
    }
    for i in 0..nbins {
        if counts[i] > 0 {
            bins[i] /= counts[i] as f64;
        }
    }
    bins
}

// ── 6. dotplot ──────────────────────────────────────────────────

fn builtin_dotplot(args: Vec<Value>) -> Result<Value> {
    let seq1 = extract_seq_str(&args[0], "dotplot")?;
    let seq2 = extract_seq_str(&args[1], "dotplot")?;
    let opts = if args.len() > 2 {
        if let Value::Record(map) = &args[2] {
            map.clone()
        } else {
            HashMap::new()
        }
    } else {
        HashMap::new()
    };

    let window = get_opt_usize(&opts, "window", 5);
    let threshold = get_opt_usize(&opts, "threshold", 3);
    let format = get_opt_str(&opts, "format", "ascii").to_string();

    let s1: Vec<char> = seq1.chars().collect();
    let s2: Vec<char> = seq2.chars().collect();

    if s1.len() < window || s2.len() < window {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "dotplot() sequences must be >= window size ({window})"
            ),
            None,
        ));
    }

    // Compute k-mer match positions
    let matches = compute_dotplot_matches(&s1, &s2, window, threshold);

    if format == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 800.0);
        return dotplot_svg(&s1, &s2, &matches, w, h);
    }

    // ASCII mode
    let grid_w = 60.min(s1.len() - window + 1);
    let grid_h = 40.min(s2.len() - window + 1);
    let step_x = (s1.len() - window + 1) as f64 / grid_w as f64;
    let step_y = (s2.len() - window + 1) as f64 / grid_h as f64;

    // Build subsampled grid
    let mut grid = vec![vec![false; grid_w]; grid_h];
    for &(x, y) in &matches {
        let gx = (x as f64 / step_x) as usize;
        let gy = (y as f64 / step_y) as usize;
        if gx < grid_w && gy < grid_h {
            grid[gy][gx] = true;
        }
    }

    let mut out = String::new();
    out.push_str("  Dotplot\n");
    out.push_str(&format!("  seq1: {} bp  seq2: {} bp  window={window}\n", s1.len(), s2.len()));
    for row in &grid {
        out.push_str("  ");
        for &cell in row {
            out.push(if cell { '●' } else { '·' });
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

fn compute_dotplot_matches(
    s1: &[char],
    s2: &[char],
    window: usize,
    threshold: usize,
) -> Vec<(usize, usize)> {
    let mut matches = Vec::new();
    let n1 = s1.len().saturating_sub(window - 1);
    let n2 = s2.len().saturating_sub(window - 1);

    for i in 0..n1 {
        for j in 0..n2 {
            let mut count = 0;
            for k in 0..window {
                if s1[i + k].eq_ignore_ascii_case(&s2[j + k]) {
                    count += 1;
                }
            }
            if count >= threshold {
                matches.push((i, j));
            }
        }
    }
    matches
}

fn dotplot_svg(
    s1: &[char],
    s2: &[char],
    matches: &[(usize, usize)],
    width: f64,
    height: f64,
) -> Result<Value> {
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 60.0;
    canvas.margin.bottom = 60.0;

    let n1 = s1.len() as f64;
    let n2 = s2.len() as f64;
    let x_scale = Scale {
        domain: (0.0, n1),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, n2),
        range: (canvas.margin.top, canvas.margin.top + canvas.plot_height()),
    };

    let r = (canvas.plot_width() / n1).max(1.0).min(3.0);
    for &(x, y) in matches {
        canvas.add_circle(x_scale.map(x as f64), y_scale.map(y as f64), r, "#4e79a7");
    }

    let d_x = Scale { domain: (0.0, n1), range: (0.0, n1) };
    let d_y = Scale { domain: (0.0, n2), range: (0.0, n2) };
    canvas.draw_x_axis(&d_x, "Sequence 1");
    canvas.draw_y_axis(&d_y, "Sequence 2");
    canvas.draw_title("Dot Plot");

    Ok(Value::Str(canvas.render()))
}

// ── 7. alignment_view ───────────────────────────────────────────

fn builtin_alignment_view(args: Vec<Value>) -> Result<Value> {
    // List<Str> → multiple sequence alignment display
    if let Value::List(items) = &args[0] {
        if !items.is_empty() && matches!(&items[0], Value::Str(_)) {
            return alignment_view_msa(items);
        }
    }
    let table = match &args[0] {
        Value::Table(t) => t,
        _ => {
            return Err(BioLangError::type_error(
                "alignment_view() requires Table (SAM data) or List<Str> (aligned sequences)",
                None,
            ))
        }
    };
    let opts = parse_options(&args);
    let display_width = get_opt_usize(&opts, "width", 80);
    let max_lanes = get_opt_usize(&opts, "max_lanes", 30);
    let format = get_opt_str(&opts, "format", "ascii").to_string();
    let region_str = get_opt_str(&opts, "region", "").to_string();

    // Parse reads from table
    let pos_idx = table.col_index("pos").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "alignment_view() needs 'pos' column", None)
    })?;
    let cigar_idx = table.col_index("cigar").ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "alignment_view() needs 'cigar' column",
            None,
        )
    })?;
    let flag_idx = table.col_index("flag");
    let qname_idx = table.col_index("qname");

    let mut reads: Vec<AlignRead> = Vec::new();
    for row in &table.rows {
        let pos = row[pos_idx].as_float().unwrap_or(0.0) as i64;
        let cigar = match &row[cigar_idx] {
            Value::Str(s) => s.clone(),
            _ => continue,
        };
        let ref_len = reference_length(&cigar) as i64;
        let flag = flag_idx.and_then(|i| row[i].as_float()).unwrap_or(0.0) as u16;
        let name = qname_idx
            .and_then(|i| row[i].as_str().map(|s| s.to_string()))
            .unwrap_or_default();

        reads.push(AlignRead {
            _name: name,
            pos,
            end: pos + ref_len,
            flag,
            _cigar: cigar,
        });
    }

    if reads.is_empty() {
        write_output("  (no reads)\n");
        return Ok(Value::Nil);
    }

    // Filter by region if specified
    if !region_str.is_empty() {
        if let Some((_, start, end)) = parse_region(&region_str) {
            reads.retain(|r| r.end > start && r.pos < end);
        }
    }

    reads.sort_by_key(|r| r.pos);

    // Determine display range
    let view_start = reads.iter().map(|r| r.pos).min().unwrap_or(0);
    let view_end = reads.iter().map(|r| r.end).max().unwrap_or(view_start + 1);

    if format == "svg" {
        let w = get_opt_f64(&opts, "width", 1000.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        return alignment_svg(&reads, view_start, view_end, w, h, max_lanes);
    }

    // ASCII alignment view
    let span = (view_end - view_start).max(1) as f64;

    // Coverage depth
    let mut depth = vec![0u32; display_width];
    for read in &reads {
        let b0 = ((read.pos - view_start) as f64 / span * display_width as f64) as usize;
        let b1 =
            ((read.end - view_start) as f64 / span * display_width as f64).ceil() as usize;
        for b in b0..b1.min(display_width) {
            depth[b] += 1;
        }
    }
    let depth_f: Vec<f64> = depth.iter().map(|&d| d as f64).collect();
    let cov_line = spark_str(&depth_f);

    // Greedy lane packing
    let lanes = pack_lanes(&reads, view_start, span, display_width, max_lanes);

    let mut out = String::new();
    out.push_str(&format!(
        "  Region: {}–{} ({} reads)\n",
        view_start,
        view_end,
        reads.len()
    ));
    out.push_str(&format!("  Depth: {cov_line}\n"));

    for lane in &lanes {
        let mut line = vec![' '; display_width];
        for &ri in lane {
            let read = &reads[ri];
            let b0 =
                ((read.pos - view_start) as f64 / span * display_width as f64).round() as usize;
            let b1 =
                ((read.end - view_start) as f64 / span * display_width as f64).round() as usize;
            let is_reverse = read.flag & 0x10 != 0;
            let ch = if is_reverse { '<' } else { '>' };
            for b in b0..b1.min(display_width) {
                line[b] = ch;
            }
        }
        let s: String = line.into_iter().collect();
        out.push_str(&format!("  {s}\n"));
    }

    write_output(&out);
    Ok(Value::Nil)
}

struct AlignRead {
    _name: String,
    pos: i64,
    end: i64,
    flag: u16,
    _cigar: String,
}

fn parse_cigar(cigar: &str) -> Vec<(u32, char)> {
    let mut ops = Vec::new();
    let mut num = 0u32;
    for ch in cigar.chars() {
        if ch.is_ascii_digit() {
            num = num * 10 + (ch as u32 - '0' as u32);
        } else {
            ops.push((num, ch));
            num = 0;
        }
    }
    ops
}

fn reference_length(cigar: &str) -> u32 {
    parse_cigar(cigar)
        .iter()
        .filter(|(_, op)| matches!(op, 'M' | 'D' | 'N' | 'X' | '='))
        .map(|(n, _)| n)
        .sum()
}

fn parse_region(region: &str) -> Option<(String, i64, i64)> {
    // Format: chr1:10000-20000
    let parts: Vec<&str> = region.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }
    let chrom = parts[0].to_string();
    let range_parts: Vec<&str> = parts[1].split('-').collect();
    if range_parts.len() != 2 {
        return None;
    }
    let start = range_parts[0].replace(',', "").parse::<i64>().ok()?;
    let end = range_parts[1].replace(',', "").parse::<i64>().ok()?;
    Some((chrom, start, end))
}

fn pack_lanes(
    reads: &[AlignRead],
    view_start: i64,
    span: f64,
    display_width: usize,
    max_lanes: usize,
) -> Vec<Vec<usize>> {
    let mut lanes: Vec<Vec<usize>> = Vec::new();
    let mut lane_ends: Vec<usize> = Vec::new(); // rightmost occupied column per lane

    for (ri, read) in reads.iter().enumerate() {
        let b0 =
            ((read.pos - view_start) as f64 / span * display_width as f64).round() as usize;
        let b1 =
            ((read.end - view_start) as f64 / span * display_width as f64).round() as usize;
        let b1 = b1.min(display_width);

        // Find first lane where this read fits
        let mut placed = false;
        for (li, end) in lane_ends.iter_mut().enumerate() {
            if b0 > *end {
                lanes[li].push(ri);
                *end = b1;
                placed = true;
                break;
            }
        }
        if !placed && lanes.len() < max_lanes {
            lanes.push(vec![ri]);
            lane_ends.push(b1);
        }
    }
    lanes
}

/// Render a multiple sequence alignment from a list of aligned strings
fn alignment_view_msa(seqs: &[Value]) -> Result<Value> {
    let strings: Vec<&str> = seqs
        .iter()
        .filter_map(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .collect();

    if strings.is_empty() {
        write_output("  (no sequences)\n");
        return Ok(Value::Nil);
    }

    let max_len = strings.iter().map(|s| s.len()).max().unwrap_or(0);
    let reference = strings[0]; // first sequence is the reference

    let mut out = String::new();
    let label_w = format!("seq{}", strings.len()).len().max(3);

    // Build consensus line showing mismatches and gaps
    for (i, seq) in strings.iter().enumerate() {
        out.push_str(&format!("  {:>width$}  ", format!("seq{}", i + 1), width = label_w));
        for (j, ch) in seq.chars().enumerate() {
            if i == 0 {
                // Reference row: show as-is
                out.push(ch);
            } else {
                let ref_ch = reference.chars().nth(j);
                match ref_ch {
                    Some(r) if r == ch => out.push('.'),  // match → dot
                    _ if ch == '-' => out.push('-'),       // gap
                    _ => out.push(ch),                     // mismatch → show base
                }
            }
        }
        out.push('\n');
    }

    // Consensus line
    out.push_str(&format!("  {:>width$}  ", "", width = label_w));
    for j in 0..max_len {
        let mut bases: Vec<char> = Vec::new();
        for seq in &strings {
            if let Some(ch) = seq.chars().nth(j) {
                if ch != '-' {
                    bases.push(ch);
                }
            }
        }
        if bases.is_empty() {
            out.push(' ');
        } else {
            let all_same = bases.iter().all(|&b| b == bases[0]);
            if all_same { out.push('*'); } else { out.push(' '); }
        }
    }
    out.push('\n');

    write_output(&out);
    Ok(Value::Nil)
}

fn alignment_svg(
    reads: &[AlignRead],
    view_start: i64,
    view_end: i64,
    width: f64,
    height: f64,
    max_lanes: usize,
) -> Result<Value> {
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.top = 60.0;
    canvas.margin.bottom = 40.0;

    let span = (view_end - view_start).max(1) as f64;
    let x_scale = Scale {
        domain: (view_start as f64, view_end as f64),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };

    // Coverage depth (pixel resolution)
    let px_width = canvas.plot_width() as usize;
    let mut depth = vec![0u32; px_width];
    for read in reads {
        let b0 = ((read.pos - view_start) as f64 / span * px_width as f64) as usize;
        let b1 = ((read.end - view_start) as f64 / span * px_width as f64).ceil() as usize;
        for b in b0..b1.min(px_width) {
            depth[b] += 1;
        }
    }
    let max_depth = *depth.iter().max().unwrap_or(&1) as f64;

    // Draw coverage area (top 40px)
    let cov_top = canvas.margin.top;
    let cov_height = 35.0;
    for (i, &d) in depth.iter().enumerate() {
        let h = (d as f64 / max_depth) * cov_height;
        let x = canvas.margin.left + i as f64;
        canvas.add_rect(x, cov_top + cov_height - h, 1.0, h, "#cccccc");
    }

    // Lane packing
    let lanes = pack_lanes(reads, view_start, span, px_width, max_lanes);
    let lane_h = 8.0;
    let lane_gap = 2.0;
    let reads_top = cov_top + cov_height + 10.0;

    for (li, lane) in lanes.iter().enumerate() {
        let y = reads_top + li as f64 * (lane_h + lane_gap);
        for &ri in lane {
            let read = &reads[ri];
            let x1 = x_scale.map(read.pos as f64);
            let x2 = x_scale.map(read.end as f64);
            let is_reverse = read.flag & 0x10 != 0;
            let color = if is_reverse { "#e15759" } else { "#4e79a7" };
            canvas.add_rect(x1, y, (x2 - x1).max(1.0), lane_h, color);
        }
    }

    let d_x = Scale {
        domain: (view_start as f64, view_end as f64),
        range: (view_start as f64, view_end as f64),
    };
    canvas.draw_x_axis(&d_x, "Position");
    canvas.draw_title("Alignment View");

    Ok(Value::Str(canvas.render()))
}

// ── 8. quality_plot ─────────────────────────────────────────────

fn builtin_quality_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let format = get_opt_str(&opts, "format", "ascii").to_string();

    // Convert PHRED+33 quality string to list of quality scores
    let resolved_arg = match &args[0] {
        Value::Str(s) => {
            let quals: Vec<Value> = s.bytes().map(|b| Value::Int((b.saturating_sub(33)) as i64)).collect();
            Value::List(quals)
        }
        other => other.clone(),
    };

    match &resolved_arg {
        Value::List(items) if !items.is_empty() => {
            // Check if it's List<Int> (single read) or List<List<Int>> (multi read)
            match &items[0] {
                Value::Int(_) | Value::Float(_) => {
                    // Single read quality
                    let quals: Vec<f64> = items
                        .iter()
                        .map(|v| v.as_float().unwrap_or(0.0))
                        .collect();

                    if format == "svg" {
                        let w = get_opt_f64(&opts, "width", 800.0);
                        let h = get_opt_f64(&opts, "height", 400.0);
                        return quality_svg_single(&quals, w, h);
                    }

                    // ASCII: per-base sparkline with quality zones
                    let normalized: Vec<f64> =
                        quals.iter().map(|&q| (q / 42.0).clamp(0.0, 1.0) * 42.0).collect();
                    let line = spark_str(&normalized);
                    let mut out = String::new();
                    out.push_str("  Quality per base position\n");
                    out.push_str(&format!("  {line}\n"));
                    out.push_str(&format!(
                        "  n={} min={:.0} max={:.0} mean={:.1}\n",
                        quals.len(),
                        quals.iter().cloned().fold(f64::INFINITY, f64::min),
                        quals.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                        quals.iter().sum::<f64>() / quals.len() as f64
                    ));
                    write_output(&out);
                    Ok(Value::Nil)
                }
                Value::List(_) => {
                    // Multi-read: compute per-position medians
                    let read_quals: Vec<Vec<f64>> = items
                        .iter()
                        .filter_map(|v| {
                            if let Value::List(inner) = v {
                                Some(
                                    inner
                                        .iter()
                                        .map(|q| q.as_float().unwrap_or(0.0))
                                        .collect(),
                                )
                            } else {
                                None
                            }
                        })
                        .collect();

                    let max_len = read_quals.iter().map(|r| r.len()).max().unwrap_or(0);
                    let mut medians = Vec::with_capacity(max_len);

                    for pos in 0..max_len {
                        let mut col_vals: Vec<f64> = read_quals
                            .iter()
                            .filter_map(|r| r.get(pos).copied())
                            .collect();
                        col_vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        let med = if col_vals.is_empty() {
                            0.0
                        } else {
                            col_vals[col_vals.len() / 2]
                        };
                        medians.push(med);
                    }

                    if format == "svg" {
                        let w = get_opt_f64(&opts, "width", 800.0);
                        let h = get_opt_f64(&opts, "height", 400.0);
                        return quality_svg_multi(&read_quals, &medians, w, h);
                    }

                    // ASCII: zone-based display
                    let mut out = String::new();
                    out.push_str(&format!(
                        "  Per-position median quality ({} reads, {} positions)\n",
                        read_quals.len(),
                        max_len
                    ));
                    let zone_line: String = medians
                        .iter()
                        .map(|&q| {
                            if q >= 30.0 {
                                '█'
                            } else if q >= 20.0 {
                                '▄'
                            } else {
                                '▁'
                            }
                        })
                        .collect();
                    out.push_str(&format!("  {zone_line}\n"));
                    out.push_str("  █ ≥30 (good)  ▄ 20-29 (ok)  ▁ <20 (poor)\n");
                    write_output(&out);
                    Ok(Value::Nil)
                }
                _ => Err(BioLangError::type_error(
                    "quality_plot() requires List<Int> or List<List<Int>>",
                    None,
                )),
            }
        }
        _ => Err(BioLangError::type_error(
            "quality_plot() requires non-empty List",
            None,
        )),
    }
}

fn quality_svg_single(quals: &[f64], width: f64, height: f64) -> Result<Value> {
    let mut canvas = SvgCanvas::new(width, height);

    let n = quals.len() as f64;
    let x_scale = Scale {
        domain: (0.0, n),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, 42.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // Background zone bands
    let green_y = y_scale.map(42.0);
    let yellow_y = y_scale.map(30.0);
    let red_y = y_scale.map(20.0);
    let bottom_y = y_scale.map(0.0);

    canvas.add_rect(
        canvas.margin.left, green_y,
        canvas.plot_width(), yellow_y - green_y,
        "#e8f5e9",
    );
    canvas.add_rect(
        canvas.margin.left, yellow_y,
        canvas.plot_width(), red_y - yellow_y,
        "#fff8e1",
    );
    canvas.add_rect(
        canvas.margin.left, red_y,
        canvas.plot_width(), bottom_y - red_y,
        "#ffebee",
    );

    // Bars per position
    let bar_w = (canvas.plot_width() / n).max(1.0);
    for (i, &q) in quals.iter().enumerate() {
        let color = if q >= 30.0 {
            "#4caf50"
        } else if q >= 20.0 {
            "#ff9800"
        } else {
            "#f44336"
        };
        let x = x_scale.map(i as f64);
        let y = y_scale.map(q);
        let h = y_scale.map(0.0) - y;
        canvas.add_rect(x, y, bar_w * 0.8, h, color);
    }

    let d_x = Scale { domain: (0.0, n), range: (0.0, n) };
    let d_y = Scale { domain: (0.0, 42.0), range: (0.0, 42.0) };
    canvas.draw_x_axis(&d_x, "Position");
    canvas.draw_y_axis(&d_y, "Phred Quality");
    canvas.draw_title("Quality Scores");

    Ok(Value::Str(canvas.render()))
}

fn quality_svg_multi(
    _read_quals: &[Vec<f64>],
    medians: &[f64],
    width: f64,
    height: f64,
) -> Result<Value> {
    let mut canvas = SvgCanvas::new(width, height);

    let n = medians.len() as f64;
    let x_scale = Scale {
        domain: (0.0, n),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, 42.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // Background zone bands
    let green_y = y_scale.map(42.0);
    let yellow_y = y_scale.map(30.0);
    let red_y = y_scale.map(20.0);
    let bottom_y = y_scale.map(0.0);

    canvas.add_rect(
        canvas.margin.left, green_y,
        canvas.plot_width(), yellow_y - green_y,
        "#e8f5e9",
    );
    canvas.add_rect(
        canvas.margin.left, yellow_y,
        canvas.plot_width(), red_y - yellow_y,
        "#fff8e1",
    );
    canvas.add_rect(
        canvas.margin.left, red_y,
        canvas.plot_width(), bottom_y - red_y,
        "#ffebee",
    );

    // Median bars
    let bar_w = (canvas.plot_width() / n).max(1.0);
    for (i, &q) in medians.iter().enumerate() {
        let color = if q >= 30.0 {
            "#4caf50"
        } else if q >= 20.0 {
            "#ff9800"
        } else {
            "#f44336"
        };
        let x = x_scale.map(i as f64);
        let y = y_scale.map(q);
        let h = y_scale.map(0.0) - y;
        canvas.add_rect(x, y, bar_w * 0.8, h, color);
    }

    let d_x = Scale { domain: (0.0, n), range: (0.0, n) };
    let d_y = Scale { domain: (0.0, 42.0), range: (0.0, 42.0) };
    canvas.draw_x_axis(&d_x, "Position");
    canvas.draw_y_axis(&d_y, "Phred Quality (Median)");
    canvas.draw_title("Per-Position Quality");

    Ok(Value::Str(canvas.render()))
}

// ── Helpers ─────────────────────────────────────────────────────

fn format_num(v: f64) -> String {
    if v == v.floor() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v:.2}")
    }
}

