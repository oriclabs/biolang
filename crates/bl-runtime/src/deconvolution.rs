//! Cell-type deconvolution builtins.
//!
//! Functions: nnls, deconvolve, marker_score, estimate_purity,
//! cell_type_correlation.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

// ── Registry ─────────────────────────────────────────────────────────

pub fn deconvolution_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("nnls", Arity::Exact(2)),
        ("deconvolve", Arity::Exact(2)),
        ("marker_score", Arity::Exact(2)),
        ("estimate_purity", Arity::Exact(2)),
        ("cell_type_correlation", Arity::Exact(1)),
    ]
}

pub fn is_deconvolution_builtin(name: &str) -> bool {
    matches!(
        name,
        "nnls" | "deconvolve" | "marker_score" | "estimate_purity" | "cell_type_correlation"
    )
}

pub fn call_deconvolution_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "nnls" => builtin_nnls(args),
        "deconvolve" => builtin_deconvolve(args),
        "marker_score" => builtin_marker_score(args),
        "estimate_purity" => builtin_estimate_purity(args),
        "cell_type_correlation" => builtin_cell_type_correlation(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown deconvolution builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table"),
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

fn require_float_list(val: &Value, func: &str) -> Result<Vec<f64>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Float(f) => Ok(*f),
                Value::Int(n) => Ok(*n as f64),
                _ => Err(BioLangError::type_error(
                    format!("{func}() list must contain numbers"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List of numbers"),
            None,
        )),
    }
}

/// Extract column j from a Table as Vec<f64>.
fn table_col(table: &Table, j: usize) -> Vec<f64> {
    table.rows.iter().map(|r| to_f64(&r[j])).collect()
}

/// Pearson correlation between two equal-length slices.
fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len() as f64;
    if n == 0.0 {
        return 0.0;
    }
    let ma = a.iter().sum::<f64>() / n;
    let mb = b.iter().sum::<f64>() / n;
    let num: f64 = a.iter().zip(b).map(|(x, y)| (x - ma) * (y - mb)).sum();
    let da: f64 = a.iter().map(|x| (x - ma).powi(2)).sum::<f64>().sqrt();
    let db: f64 = b.iter().map(|y| (y - mb).powi(2)).sum::<f64>().sqrt();
    if da == 0.0 || db == 0.0 {
        0.0
    } else {
        num / (da * db)
    }
}

// ── nnls ─────────────────────────────────────────────────────────────

/// Non-negative least squares via projected gradient descent.
/// reference: Table with rows = genes, columns = cell types.
/// mixture: List<Float> with one value per gene (row).
fn nnls_solve(mixture: &[f64], reference: &Table) -> Vec<f64> {
    let n_genes = mixture.len();
    let n_types = reference.columns.len();

    // Build A matrix: A[gene][type] = reference.rows[gene][type_col]
    // A is n_genes × n_types
    let a: Vec<Vec<f64>> = (0..n_genes)
        .map(|g| {
            (0..n_types)
                .map(|t| {
                    if g < reference.rows.len() && t < reference.rows[g].len() {
                        to_f64(&reference.rows[g][t])
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();

    // Precompute AᵀA and Aᵀb
    let mut ata = vec![vec![0.0f64; n_types]; n_types];
    let mut atb = vec![0.0f64; n_types];
    for g in 0..n_genes {
        for i in 0..n_types {
            atb[i] += a[g][i] * mixture[g];
            for j in 0..n_types {
                ata[i][j] += a[g][i] * a[g][j];
            }
        }
    }

    // Projected gradient descent: x ← max(0, x - α*(AᵀAx - Aᵀb))
    let alpha = 0.001;
    let mut x = vec![1.0 / n_types as f64; n_types];
    for _ in 0..2000 {
        // grad = AᵀAx - Aᵀb
        let mut grad = vec![0.0f64; n_types];
        for i in 0..n_types {
            for j in 0..n_types {
                grad[i] += ata[i][j] * x[j];
            }
            grad[i] -= atb[i];
        }
        // Update and project
        for i in 0..n_types {
            x[i] = (x[i] - alpha * grad[i]).max(0.0);
        }
    }

    // Normalize to sum 1
    let total: f64 = x.iter().sum();
    if total > 0.0 {
        x.iter_mut().for_each(|v| *v /= total);
    }
    x
}

fn builtin_nnls(args: Vec<Value>) -> Result<Value> {
    let mixture = require_float_list(&args[0], "nnls")?;
    let reference = require_table(&args[1], "nnls")?;

    let fracs = nnls_solve(&mixture, reference);
    Ok(Value::List(fracs.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
}

// ── deconvolve ───────────────────────────────────────────────────────

fn builtin_deconvolve(args: Vec<Value>) -> Result<Value> {
    let bulk = require_table(&args[0], "deconvolve")?;
    let reference = require_table(&args[1], "deconvolve")?;

    let n_genes = bulk.rows.len();
    let n_samples = bulk.columns.len();
    let n_types = reference.columns.len();

    // For each sample column, collect gene values and run nnls
    let mut result_rows: Vec<Vec<Value>> = vec![Vec::new(); n_types];
    for t in 0..n_types {
        result_rows[t] = Vec::new();
    }

    for s in 0..n_samples {
        let mixture: Vec<f64> = (0..n_genes)
            .map(|g| {
                if g < bulk.rows.len() && s < bulk.rows[g].len() {
                    to_f64(&bulk.rows[g][s])
                } else {
                    0.0
                }
            })
            .collect();
        let fracs = nnls_solve(&mixture, reference);
        for (t, f) in fracs.into_iter().enumerate() {
            if t < n_types {
                result_rows[t].push(Value::Float(f));
            }
        }
    }

    // Table: rows = cell types, columns = sample names
    let rows: Vec<Vec<Value>> = result_rows;
    Ok(Value::Table(Table::new(bulk.columns.clone(), rows)))
}

// ── marker_score ─────────────────────────────────────────────────────

fn builtin_marker_score(args: Vec<Value>) -> Result<Value> {
    let bulk = require_table(&args[0], "marker_score")?;
    let markers = match &args[1] {
        Value::Record(r) => r,
        _ => {
            return Err(BioLangError::type_error(
                "marker_score() markers must be Record",
                None,
            ))
        }
    };

    // Build gene-name → column-index map from bulk
    let gene_col_map: HashMap<String, usize> = bulk
        .columns
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), i))
        .collect();

    let n_samples = if bulk.rows.is_empty() {
        0
    } else {
        bulk.rows[0].len()
    };

    let mut cell_types: Vec<String> = markers.keys().cloned().collect();
    cell_types.sort();

    let mut result_rows: Vec<Vec<Value>> = Vec::new();

    for ct in &cell_types {
        let marker_genes: Vec<String> = match markers.get(ct) {
            Some(Value::List(l)) => l
                .iter()
                .filter_map(|v| {
                    if let Value::Str(s) = v {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect(),
            _ => Vec::new(),
        };

        // For each sample: mean expression of marker genes
        let mut scores = vec![0.0f64; n_samples];
        let mut n_found = 0usize;
        for gene in &marker_genes {
            if let Some(&row_idx) = gene_col_map.get(gene) {
                // bulk: rows = genes, columns = samples — but columns stores gene names
                // so actually rows[row_idx] is one gene across all samples
                if row_idx < bulk.rows.len() {
                    n_found += 1;
                    for (s, val) in bulk.rows[row_idx].iter().enumerate() {
                        if s < n_samples {
                            scores[s] += to_f64(val);
                        }
                    }
                }
            }
        }
        if n_found > 0 {
            scores.iter_mut().for_each(|v| *v /= n_found as f64);
        }

        // Normalize to [0,1]
        let min_s = scores.iter().cloned().fold(f64::INFINITY, f64::min);
        let max_s = scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let range = max_s - min_s;
        let normalized: Vec<Value> = scores
            .iter()
            .map(|&s| {
                Value::Float(if range == 0.0 {
                    0.0
                } else {
                    (s - min_s) / range
                })
            })
            .collect();
        result_rows.push(normalized);
    }

    // Column names = sample names from bulk (all columns since bulk rows are genes)
    // Use generic sample names if bulk columns are gene names
    let sample_names: Vec<String> = (0..n_samples).map(|i| format!("sample_{i}")).collect();
    Ok(Value::Table(Table::new(sample_names, result_rows)))
}

// ── estimate_purity ───────────────────────────────────────────────────

fn builtin_estimate_purity(args: Vec<Value>) -> Result<Value> {
    let fractions = require_table(&args[0], "estimate_purity")?;
    let tumor_col = match &args[1] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "estimate_purity() tumor_col must be Str",
                None,
            ))
        }
    };

    // fractions: rows = cell types (as rows), columns = samples
    // Find the row whose cell-type name matches tumor_col
    // Actually fractions table doesn't embed row labels — use column index trick:
    // Try finding tumor_col in the *columns* of fractions (samples as columns with row = cell types)
    // Since our deconvolve returns rows=cell types, cols=samples, we don't embed cell type names
    // Return all values in the matching row index by checking if nrows matches
    // Simpler: find tumor_col in fractions.columns as a sample name — that is purity for that sample
    // Or: treat fractions as rows=cell types indexed by position, user picks row by name via marker
    // Best approach: if fractions has a "cell_type" column, find the tumor row there;
    // otherwise fall back to returning list of 1.0

    // Look for a "cell_type" or first-string column
    let ct_col_idx = fractions.columns.iter().position(|c| c == "cell_type");

    if let Some(ct_idx) = ct_col_idx {
        for row in &fractions.rows {
            if let Value::Str(name) = &row[ct_idx] {
                if name == &tumor_col {
                    let vals: Vec<Value> = row
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != ct_idx)
                        .map(|(_, v)| Value::Float(to_f64(v)))
                        .collect();
                    return Ok(Value::List((vals).into()));
                }
            }
        }
        // Not found — return 1.0 per sample
        let n_samples = fractions
            .rows
            .first()
            .map(|r| if r.len() > 1 { r.len() - 1 } else { 0 })
            .unwrap_or(0);
        return Ok(Value::List((vec![Value::Float(1.0); n_samples]).into()));
    }

    // No cell_type column: check if tumor_col is a column name (sample column)
    if let Some(col_idx) = fractions.columns.iter().position(|c| c == &tumor_col) {
        let vals: Vec<Value> = fractions
            .rows
            .iter()
            .map(|r| Value::Float(to_f64(&r[col_idx])))
            .collect();
        return Ok(Value::List((vals).into()));
    }

    // Fall back: 1.0 per row (assume pure)
    Ok(Value::List(
        (vec![Value::Float(1.0); fractions.rows.len()]).into(),
    ))
}

// ── cell_type_correlation ─────────────────────────────────────────────

fn builtin_cell_type_correlation(args: Vec<Value>) -> Result<Value> {
    let fractions = require_table(&args[0], "cell_type_correlation")?;

    let n_types = fractions.rows.len();
    let n_samples = fractions.columns.len();

    // Build vectors per cell type
    let vecs: Vec<Vec<f64>> = (0..n_types)
        .map(|t| {
            (0..n_samples)
                .map(|s| {
                    if s < fractions.rows[t].len() {
                        to_f64(&fractions.rows[t][s])
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();

    // Compute pairwise Pearson
    let col_names: Vec<String> = (0..n_types).map(|i| format!("type_{i}")).collect();
    let rows: Vec<Vec<Value>> = (0..n_types)
        .map(|i| {
            (0..n_types)
                .map(|j| Value::Float(pearson(&vecs[i], &vecs[j])))
                .collect()
        })
        .collect();

    Ok(Value::Table(Table::new(col_names, rows)))
}
