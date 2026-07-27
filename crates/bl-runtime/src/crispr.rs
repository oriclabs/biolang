//! CRISPR screen analysis builtins.
//!
//! Functions: guide_counts, lfc_guides, mageck_score, essential_genes,
//! guide_gc, crispr_qc.

use std::collections::HashMap;

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ──────────────────────────────────────────────────────────

pub fn crispr_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("guide_counts", Arity::Exact(1)),
        ("lfc_guides", Arity::Exact(3)),
        ("mageck_score", Arity::Exact(3)),
        ("essential_genes", Arity::Range(1, 2)),
        ("guide_gc", Arity::Exact(1)),
        ("crispr_qc", Arity::Exact(1)),
    ]
}

pub fn is_crispr_builtin(name: &str) -> bool {
    matches!(
        name,
        "guide_counts"
            | "lfc_guides"
            | "mageck_score"
            | "essential_genes"
            | "guide_gc"
            | "crispr_qc"
    )
}

pub fn call_crispr_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "guide_counts" => builtin_guide_counts(args),
        "lfc_guides" => builtin_lfc_guides(args),
        "mageck_score" => builtin_mageck_score(args),
        "essential_genes" => builtin_essential_genes(args),
        "guide_gc" => builtin_guide_gc(args),
        "crispr_qc" => builtin_crispr_qc(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown crispr builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table"),
            None,
        )),
    }
}

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Str"),
            None,
        )),
    }
}

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => 0.0,
    }
}

fn require_int_list(val: &Value, func: &str) -> Result<Vec<usize>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                _ => Err(BioLangError::type_error(
                    format!("{func}() column indices must be List<Int>"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List of column indices"),
            None,
        )),
    }
}

fn col_means(table: &Table, col_indices: &[usize]) -> Result<Vec<f64>> {
    let n_sample_cols = col_indices.len();
    if n_sample_cols == 0 {
        return Err(BioLangError::type_error(
            "col_means: empty column list",
            None,
        ));
    }
    let mut means = vec![0.0f64; table.rows.len()];
    for (row_idx, row) in table.rows.iter().enumerate() {
        let sum: f64 = col_indices
            .iter()
            .map(|&ci| if ci < row.len() { to_f64(&row[ci]) } else { 0.0 })
            .sum();
        means[row_idx] = sum / n_sample_cols as f64;
    }
    Ok(means)
}

// ── guide_counts ──────────────────────────────────────────────────────

fn builtin_guide_counts(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "guide_counts")?;

    let mut lines = text.lines().filter(|l| !l.starts_with('#'));
    let header_line = lines.next().ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "guide_counts(): empty input".to_string(), None)
    })?;

    let sep = if header_line.contains('\t') { '\t' } else { ',' };
    let headers: Vec<String> = header_line.split(sep).map(|s| s.trim().to_string()).collect();

    if headers.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "guide_counts(): need at least guide + gene columns".to_string(),
            None,
        ));
    }

    // Column names: guide, gene, then sample names from header
    let mut col_names = vec!["guide".to_string(), "gene".to_string()];
    for s in headers.iter().skip(2) {
        col_names.push(s.clone());
    }

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split(sep).collect();
        if parts.len() < 2 {
            continue;
        }
        let mut row = vec![
            Value::Str(parts[0].trim().to_string()),
            Value::Str(parts[1].trim().to_string()),
        ];
        for p in parts.iter().skip(2) {
            let count: i64 = p.trim().parse().unwrap_or(0);
            row.push(Value::Int(count));
        }
        // pad missing sample cols
        while row.len() < col_names.len() {
            row.push(Value::Int(0));
        }
        rows.push(row);
    }

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── lfc_guides ────────────────────────────────────────────────────────

fn builtin_lfc_guides(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "lfc_guides")?.clone();
    let ctrl_cols = require_int_list(&args[1], "lfc_guides")?;
    let trt_cols = require_int_list(&args[2], "lfc_guides")?;

    let mean_ctrl = col_means(&table, &ctrl_cols)?;
    let mean_trt = col_means(&table, &trt_cols)?;

    // guide and gene are columns 0 and 1
    let col_names = vec![
        "guide".to_string(),
        "gene".to_string(),
        "mean_ctrl".to_string(),
        "mean_trt".to_string(),
        "lfc".to_string(),
    ];

    let rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mc = mean_ctrl[i];
            let mt = mean_trt[i];
            let lfc = ((mt + 1.0) / (mc + 1.0)).log2();
            vec![
                row.first().cloned().unwrap_or(Value::Str(String::new())),
                row.get(1).cloned().unwrap_or(Value::Str(String::new())),
                Value::Float(mc),
                Value::Float(mt),
                Value::Float(lfc),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── mageck_score ──────────────────────────────────────────────────────

fn builtin_mageck_score(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "mageck_score")?.clone();
    let ctrl_cols = require_int_list(&args[1], "mageck_score")?;
    let trt_cols = require_int_list(&args[2], "mageck_score")?;

    let mean_ctrl = col_means(&table, &ctrl_cols)?;
    let mean_trt = col_means(&table, &trt_cols)?;

    // Compute LFC per guide
    let mut lfc_vec: Vec<f64> = mean_ctrl
        .iter()
        .zip(mean_trt.iter())
        .map(|(&mc, &mt)| ((mt + 1.0) / (mc + 1.0)).log2())
        .collect();

    let n = lfc_vec.len();

    // Rank LFC ascending (index of sorted order)
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| lfc_vec[a].partial_cmp(&lfc_vec[b]).unwrap_or(std::cmp::Ordering::Equal));
    let mut ranks = vec![0usize; n];
    for (rank, &idx) in order.iter().enumerate() {
        ranks[idx] = rank + 1; // 1-based
    }

    // Per guide: rank fraction
    let rank_frac: Vec<f64> = ranks.iter().map(|&r| r as f64 / n as f64).collect();

    // Group by gene
    let mut gene_guides: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, row) in table.rows.iter().enumerate() {
        let gene = match row.get(1) {
            Some(Value::Str(s)) => s.clone(),
            _ => format!("gene_{i}"),
        };
        gene_guides.entry(gene).or_default().push(i);
    }

    let col_names = vec![
        "gene".to_string(),
        "n_guides".to_string(),
        "rra_score".to_string(),
        "mean_lfc".to_string(),
    ];

    let mut gene_rows: Vec<Vec<Value>> = gene_guides
        .iter()
        .map(|(gene, indices)| {
            let n_g = indices.len();
            // Geometric mean of rank fractions
            let log_sum: f64 = indices.iter().map(|&i| rank_frac[i].ln()).sum();
            let rra = (log_sum / n_g as f64).exp();
            let mean_lfc: f64 = indices.iter().map(|&i| lfc_vec[i]).sum::<f64>() / n_g as f64;
            vec![
                Value::Str(gene.clone()),
                Value::Int(n_g as i64),
                Value::Float(rra),
                Value::Float(mean_lfc),
            ]
        })
        .collect();

    // Sort by rra_score ascending
    gene_rows.sort_by(|a, b| {
        let ra = match &a[2] { Value::Float(f) => *f, _ => f64::MAX };
        let rb = match &b[2] { Value::Float(f) => *f, _ => f64::MAX };
        ra.partial_cmp(&rb).unwrap_or(std::cmp::Ordering::Equal)
    });

    // suppress unused warning on lfc_vec
    let _ = lfc_vec.as_mut_slice();

    Ok(Value::Table(Table::new(col_names, gene_rows)))
}

// ── essential_genes ───────────────────────────────────────────────────

fn builtin_essential_genes(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "essential_genes")?.clone();
    let top_n = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => *n as usize,
            _ => {
                return Err(BioLangError::type_error(
                    "essential_genes() top_n must be Int",
                    None,
                ))
            }
        }
    } else {
        20
    };

    let take = top_n.min(table.rows.len());
    Ok(Value::Table(Table::new(
        table.columns.clone(),
        table.rows[..take].to_vec(),
    )))
}

// ── guide_gc ──────────────────────────────────────────────────────────

fn builtin_guide_gc(args: Vec<Value>) -> Result<Value> {
    let seq = require_str(&args[0], "guide_gc")?;
    if seq.is_empty() {
        return Ok(Value::Float(0.0));
    }
    let gc = seq
        .chars()
        .filter(|&c| matches!(c, 'G' | 'C' | 'g' | 'c'))
        .count() as f64;
    Ok(Value::Float(gc / seq.len() as f64))
}

// ── crispr_qc ─────────────────────────────────────────────────────────

fn builtin_crispr_qc(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "crispr_qc")?;

    let n_guides = table.rows.len() as i64;

    // Unique genes (column 1)
    let mut genes: std::collections::HashSet<String> = std::collections::HashSet::new();
    for row in &table.rows {
        if let Some(Value::Str(g)) = row.get(1) {
            genes.insert(g.clone());
        }
    }
    let n_genes = genes.len() as i64;

    // Total counts per guide (columns 2+)
    let sample_cols: Vec<usize> = (2..table.columns.len()).collect();
    let totals: Vec<f64> = table
        .rows
        .iter()
        .map(|row| {
            sample_cols
                .iter()
                .map(|&ci| if ci < row.len() { to_f64(&row[ci]) } else { 0.0 })
                .sum()
        })
        .collect();

    let zero_count_guides = totals.iter().filter(|&&t| t == 0.0).count() as i64;

    // Gini coefficient
    let mut sorted_totals = totals.clone();
    sorted_totals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = sorted_totals.len() as f64;
    let sum_x: f64 = sorted_totals.iter().sum();
    let gini = if sum_x == 0.0 || n == 0.0 {
        0.0
    } else {
        let weighted: f64 = sorted_totals
            .iter()
            .enumerate()
            .map(|(i, &x)| (2.0 * (i as f64 + 1.0) - n - 1.0) * x)
            .sum();
        weighted / (n * sum_x)
    };

    let mut rec = HashMap::new();
    rec.insert("n_guides".to_string(), Value::Int(n_guides));
    rec.insert("n_genes".to_string(), Value::Int(n_genes));
    rec.insert("zero_count_guides".to_string(), Value::Int(zero_count_guides));
    rec.insert("gini_coefficient".to_string(), Value::Float(gini));

    Ok(Value::Record((rec).into()))
}
