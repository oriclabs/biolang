//! qPCR analysis builtins: ΔCt, ΔΔCt fold change, PCR efficiency, reference normalization,
//! and geNorm stability scoring.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ──────────────────────────────────────────────────────────

pub fn qpcr_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("delta_ct", Arity::Exact(2)),
        ("delta_delta_ct", Arity::Exact(2)),
        ("pcr_efficiency", Arity::Exact(2)),
        ("reference_normalize", Arity::Exact(2)),
        ("genorm_stability", Arity::Exact(2)),
    ]
}

pub fn is_qpcr_builtin(name: &str) -> bool {
    matches!(
        name,
        "delta_ct"
            | "delta_delta_ct"
            | "pcr_efficiency"
            | "reference_normalize"
            | "genorm_stability"
    )
}

pub fn call_qpcr_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "delta_ct" => builtin_delta_ct(args),
        "delta_delta_ct" => builtin_delta_delta_ct(args),
        "pcr_efficiency" => builtin_pcr_efficiency(args),
        "reference_normalize" => builtin_reference_normalize(args),
        "genorm_stability" => builtin_genorm_stability(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown qpcr builtin: {name}"),
            None,
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────

fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

fn require_f64(val: &Value, func: &str) -> Result<f64> {
    to_f64(val).ok_or_else(|| {
        BioLangError::type_error(format!("{func}() requires a number"), None)
    })
}

fn require_float_list(val: &Value, func: &str) -> Result<Vec<f64>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| {
                to_f64(v).ok_or_else(|| {
                    BioLangError::type_error(format!("{func}() list must contain numbers"), None)
                })
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List of numbers"),
            None,
        )),
    }
}

fn require_int_list(val: &Value, func: &str) -> Result<Vec<usize>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                _ => Err(BioLangError::type_error(
                    format!("{func}() index list must contain Int values"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() index list must be a List"),
            None,
        )),
    }
}

fn require_table_clone(val: &Value, func: &str) -> Result<Table> {
    match val {
        Value::Table(t) => Ok(t.clone()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a Table"),
            None,
        )),
    }
}

fn vec_mean(v: &[f64]) -> f64 {
    if v.is_empty() { return 0.0; }
    v.iter().sum::<f64>() / v.len() as f64
}

fn vec_stdev(v: &[f64]) -> f64 {
    if v.len() < 2 { return 0.0; }
    let m = vec_mean(v);
    let var = v.iter().map(|&x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64;
    var.sqrt()
}

/// Linear regression slope for (x, y) pairs.
fn linreg_slope(x: &[f64], y: &[f64]) -> Option<f64> {
    let n = x.len().min(y.len()) as f64;
    if n < 2.0 { return None; }
    let mx = x.iter().sum::<f64>() / n;
    let my = y.iter().sum::<f64>() / n;
    let num: f64 = x.iter().zip(y).map(|(&xi, &yi)| (xi - mx) * (yi - my)).sum();
    let den: f64 = x.iter().map(|&xi| (xi - mx).powi(2)).sum();
    if den.abs() < 1e-15 { return None; }
    Some(num / den)
}

// ── delta_ct(sample_ct, reference_ct) ────────────────────────────────
// Works on scalars or paired lists.

fn builtin_delta_ct(args: Vec<Value>) -> Result<Value> {
    match (&args[0], &args[1]) {
        // Both scalars
        _ if to_f64(&args[0]).is_some() && to_f64(&args[1]).is_some() => {
            let s = require_f64(&args[0], "delta_ct")?;
            let r = require_f64(&args[1], "delta_ct")?;
            Ok(Value::Float(s - r))
        }
        // Both lists
        (Value::List(_), Value::List(_)) => {
            let s = require_float_list(&args[0], "delta_ct")?;
            let r = require_float_list(&args[1], "delta_ct")?;
            if s.len() != r.len() {
                return Err(BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!(
                        "delta_ct(): sample_ct length {} != reference_ct length {}",
                        s.len(),
                        r.len()
                    ),
                    None,
                ));
            }
            Ok(Value::List(
                s.iter().zip(r.iter()).map(|(&a, &b)| Value::Float(a - b)).collect::<Vec<_>>().into(),
            ))
        }
        _ => Err(BioLangError::type_error(
            "delta_ct() requires two numbers or two equal-length lists",
            None,
        )),
    }
}

// ── delta_delta_ct(sample_delta_ct, control_delta_ct) ─────────────────
// fold_change = 2^(-(sample_delta_ct - control_delta_ct))

fn builtin_delta_delta_ct(args: Vec<Value>) -> Result<Value> {
    match (&args[0], &args[1]) {
        _ if to_f64(&args[0]).is_some() && to_f64(&args[1]).is_some() => {
            let s = require_f64(&args[0], "delta_delta_ct")?;
            let c = require_f64(&args[1], "delta_delta_ct")?;
            let fold = 2f64.powf(-(s - c));
            Ok(Value::Float(fold))
        }
        (Value::List(_), Value::List(_)) => {
            let s = require_float_list(&args[0], "delta_delta_ct")?;
            let c = require_float_list(&args[1], "delta_delta_ct")?;
            if s.len() != c.len() {
                return Err(BioLangError::runtime(
                    ErrorKind::IndexOutOfBounds,
                    format!(
                        "delta_delta_ct(): sample length {} != control length {}",
                        s.len(),
                        c.len()
                    ),
                    None,
                ));
            }
            Ok(Value::List(
                s.iter()
                    .zip(c.iter())
                    .map(|(&si, &ci)| Value::Float(2f64.powf(-(si - ci))))
                    .collect::<Vec<_>>().into(),
            ))
        }
        _ => Err(BioLangError::type_error(
            "delta_delta_ct() requires two numbers or two equal-length lists",
            None,
        )),
    }
}

// ── pcr_efficiency(cts, log_dilutions) ────────────────────────────────
// Slope from linear regression of Ct ~ log10(dilution);
// efficiency = 10^(-1/slope) - 1, clamped to [0.0, 1.5].

fn builtin_pcr_efficiency(args: Vec<Value>) -> Result<Value> {
    let cts  = require_float_list(&args[0], "pcr_efficiency")?;
    let ldil = require_float_list(&args[1], "pcr_efficiency")?;
    if cts.len() < 2 || cts.len() != ldil.len() {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!(
                "pcr_efficiency(): need ≥2 matched points, got {} cts and {} dilutions",
                cts.len(),
                ldil.len()
            ),
            None,
        ));
    }

    let slope = linreg_slope(&ldil, &cts).ok_or_else(|| {
        BioLangError::runtime(ErrorKind::DivisionByZero, "pcr_efficiency(): zero variance in dilutions", None)
    })?;

    let efficiency = (10f64.powf(-1.0 / slope) - 1.0).clamp(0.0, 1.5);
    let columns = vec!["efficiency".to_string(), "slope".to_string()];
    let rows = vec![vec![Value::Float(efficiency), Value::Float(slope)]];
    Ok(Value::Table(Table::new(columns, rows)))
}

// ── reference_normalize(ct_table, ref_gene_indices) ──────────────────
// ct_table: rows = genes, columns = samples.
// ref_gene_indices: List[Int] — zero-based row indices of reference genes.
// Per sample: reference mean = mean(ct_table[ref_rows][col]);
// normalized[row][col] = ct_table[row][col] - reference_mean[col].

fn builtin_reference_normalize(args: Vec<Value>) -> Result<Value> {
    let table = require_table_clone(&args[0], "reference_normalize")?;
    let ref_idxs = require_int_list(&args[1], "reference_normalize")?;

    let rows = &table.rows;
    let cols = &table.columns;
    let ngenes = rows.len();
    let nsamples = cols.len();

    if ref_idxs.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            "reference_normalize(): ref_gene_indices must not be empty",
            None,
        ));
    }
    for &idx in &ref_idxs {
        if idx >= ngenes {
            return Err(BioLangError::runtime(
                ErrorKind::IndexOutOfBounds,
                format!("reference_normalize(): ref index {idx} out of range (table has {ngenes} rows)"),
                None,
            ));
        }
    }

    // Extract numeric matrix: matrix[gene][sample]
    let mut matrix: Vec<Vec<f64>> = Vec::with_capacity(ngenes);
    for row in rows {
        let gene_vals: Vec<f64> = row
            .iter()
            .map(|v| {
                to_f64(v).ok_or_else(|| {
                    BioLangError::type_error("reference_normalize(): table must contain numbers", None)
                })
            })
            .collect::<Result<_>>()?;
        matrix.push(gene_vals);
    }

    // Reference mean per sample column
    let ref_means: Vec<f64> = (0..nsamples)
        .map(|col| {
            let ref_vals: Vec<f64> = ref_idxs.iter().map(|&r| matrix[r][col]).collect();
            vec_mean(&ref_vals)
        })
        .collect();

    // Subtract reference mean
    let norm_rows: Vec<Vec<Value>> = matrix
        .iter()
        .map(|gene_row| {
            gene_row
                .iter()
                .enumerate()
                .map(|(col, &ct)| Value::Float(ct - ref_means[col]))
                .collect()
        })
        .collect();

    Ok(Value::Table(Table::new(cols.to_vec(), norm_rows)))
}

// ── genorm_stability(ct_table, ref_gene_indices) ──────────────────────
// geNorm M-score: for each reference gene i, compute pairwise log2(Ct_i/Ct_j)
// across all samples, then stdev. M_i = mean of those stdevs.
// Returns Table(gene_idx, m_score) sorted by m_score ascending (most stable first).

fn builtin_genorm_stability(args: Vec<Value>) -> Result<Value> {
    let table = require_table_clone(&args[0], "genorm_stability")?;
    let ref_idxs = require_int_list(&args[1], "genorm_stability")?;

    let rows = &table.rows;
    let ngenes = rows.len();
    let nsamples = if rows.is_empty() { 0 } else { rows[0].len() };

    if ref_idxs.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            "genorm_stability(): need at least 2 reference genes",
            None,
        ));
    }
    for &idx in &ref_idxs {
        if idx >= ngenes {
            return Err(BioLangError::runtime(
                ErrorKind::IndexOutOfBounds,
                format!("genorm_stability(): ref index {idx} out of range (table has {ngenes} rows)"),
                None,
            ));
        }
    }

    // Extract ct values for reference genes only: ref_matrix[local_idx][sample]
    let ref_matrix: Vec<Vec<f64>> = ref_idxs
        .iter()
        .map(|&gi| {
            rows[gi]
                .iter()
                .map(|v| {
                    to_f64(v).ok_or_else(|| {
                        BioLangError::type_error("genorm_stability(): table must contain numbers", None)
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<_>>()?;

    let nref = ref_idxs.len();
    let mut m_scores: Vec<(usize, f64)> = Vec::with_capacity(nref);

    for i in 0..nref {
        let mut pairwise_stdevs: Vec<f64> = Vec::new();
        for j in 0..nref {
            if i == j { continue; }
            // log2(Ct_i / Ct_j) per sample; skip if either is 0
            let ratios: Vec<f64> = (0..nsamples)
                .filter_map(|s| {
                    let ci = ref_matrix[i][s];
                    let cj = ref_matrix[j][s];
                    if ci > 0.0 && cj > 0.0 {
                        Some((ci / cj).log2())
                    } else {
                        None
                    }
                })
                .collect();
            pairwise_stdevs.push(vec_stdev(&ratios));
        }
        let m = vec_mean(&pairwise_stdevs);
        m_scores.push((ref_idxs[i], m));
    }

    // Sort ascending (lowest M = most stable)
    m_scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let columns = vec!["gene_idx".to_string(), "m_score".to_string()];
    let result_rows: Vec<Vec<Value>> = m_scores
        .into_iter()
        .map(|(idx, m)| vec![Value::Int(idx as i64), Value::Float(m)])
        .collect();

    Ok(Value::Table(Table::new(columns, result_rows)))
}
