//! DNA methylation builtins: β ↔ M-value conversion, DMR detection,
//! CpG density, Horvath epigenetic clock, and differential methylation analysis.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ──────────────────────────────────────────────────────────

pub fn methylation_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("beta_to_mvalue", Arity::Exact(1)),
        ("mvalue_to_beta", Arity::Exact(1)),
        ("dmr_find", Arity::Range(3, 4)),
        ("cpg_density", Arity::Exact(1)),
        ("epigenetic_age", Arity::Exact(2)),
        ("differential_methylation", Arity::Exact(3)),
    ]
}

pub fn is_methylation_builtin(name: &str) -> bool {
    matches!(
        name,
        "beta_to_mvalue"
            | "mvalue_to_beta"
            | "dmr_find"
            | "cpg_density"
            | "epigenetic_age"
            | "differential_methylation"
    )
}

pub fn call_methylation_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "beta_to_mvalue" => builtin_beta_to_mvalue(args),
        "mvalue_to_beta" => builtin_mvalue_to_beta(args),
        "dmr_find" => builtin_dmr_find(args),
        "cpg_density" => builtin_cpg_density(args),
        "epigenetic_age" => builtin_epigenetic_age(args),
        "differential_methylation" => builtin_differential_methylation(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown methylation builtin: {name}"),
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

fn require_matrix(val: &Value, func: &str) -> Result<Vec<Vec<f64>>> {
    match val {
        Value::List(rows) => rows
            .iter()
            .map(|row| match row {
                Value::List(cells) => cells
                    .iter()
                    .map(|v| {
                        to_f64(v).ok_or_else(|| {
                            BioLangError::type_error(
                                format!("{func}() matrix must contain numbers"),
                                None,
                            )
                        })
                    })
                    .collect(),
                _ => Err(BioLangError::type_error(
                    format!("{func}() matrix rows must be Lists"),
                    None,
                )),
            })
            .collect(),
        Value::Table(t) => Ok(t
            .rows
            .iter()
            .map(|row| row.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect())
            .collect()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List<List> or Table"),
            None,
        )),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Int"),
            None,
        )),
    }
}

fn require_str_list(val: &Value, func: &str) -> Result<Vec<String>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(BioLangError::type_error(
                    format!("{func}() label list must contain strings"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List of strings"),
            None,
        )),
    }
}

fn vec_mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f64>() / v.len() as f64
}

fn vec_var(v: &[f64]) -> f64 {
    if v.len() < 2 {
        return 0.0;
    }
    let m = vec_mean(v);
    v.iter().map(|&x| (x - m).powi(2)).sum::<f64>() / (v.len() - 1) as f64
}

fn welch_ttest(a: &[f64], b: &[f64]) -> (f64, f64) {
    if a.is_empty() || b.is_empty() {
        return (0.0, 1.0);
    }
    let na = a.len() as f64;
    let nb = b.len() as f64;
    let va = vec_var(a);
    let vb = vec_var(b);
    let se = (va / na + vb / nb).sqrt();
    if se == 0.0 {
        // No within-group variance — perfectly separated if means differ
        let diff = (vec_mean(a) - vec_mean(b)).abs();
        return if diff > 0.0 { (f64::INFINITY, 0.0) } else { (0.0, 1.0) };
    }
    let t = (vec_mean(a) - vec_mean(b)) / se;
    let s2a = va / na;
    let s2b = vb / nb;
    let df = if s2a + s2b == 0.0 {
        1.0
    } else {
        (s2a + s2b).powi(2) / (s2a.powi(2) / (na - 1.0) + s2b.powi(2) / (nb - 1.0))
    };
    let p = p_from_t(t, df.max(1.0));
    (t, p)
}

fn p_from_t(t: f64, df: f64) -> f64 {
    if t.is_infinite() || t.is_nan() {
        return if t.abs() == f64::INFINITY { 0.0 } else { 1.0 };
    }
    let z = t.abs() * (1.0 - 1.0 / (4.0 * df));
    (2.0 * standard_normal_tail(z)).min(1.0)
}

fn standard_normal_tail(z: f64) -> f64 {
    if z <= 0.0 {
        return 0.5;
    }
    if z > 8.0 {
        return 0.0;
    }
    let t = 1.0 / (1.0 + 0.2316419 * z);
    let poly = t
        * (0.319381530
            + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    let pdf = (-0.5 * z * z).exp() / (2.0 * std::f64::consts::PI).sqrt();
    pdf * poly
}

// ── beta_to_mvalue(beta_list) ─────────────────────────────────────────
// M = log2(β / (1 - β))

fn builtin_beta_to_mvalue(args: Vec<Value>) -> Result<Value> {
    let betas = require_float_list(&args[0], "beta_to_mvalue")?;
    let mvalues: Vec<Value> = betas
        .into_iter()
        .map(|b| {
            let b = b.clamp(1e-6, 1.0 - 1e-6);
            Value::Float((b / (1.0 - b)).log2())
        })
        .collect();
    Ok(Value::List((mvalues).into()))
}

// ── mvalue_to_beta(mvalue_list) ───────────────────────────────────────
// β = 2^M / (2^M + 1)

fn builtin_mvalue_to_beta(args: Vec<Value>) -> Result<Value> {
    let mvalues = require_float_list(&args[0], "mvalue_to_beta")?;
    let betas: Vec<Value> = mvalues
        .into_iter()
        .map(|m| {
            let exp2m = (2.0f64).powf(m);
            Value::Float(exp2m / (exp2m + 1.0))
        })
        .collect();
    Ok(Value::List((betas).into()))
}

// ── dmr_find(beta_matrix, group_a_indices, group_b_indices, min_cpgs=3) ─
//
// Finds differentially methylated regions (DMRs) by:
//   1. Computing per-CpG delta beta (mean_A - mean_B)
//   2. Sliding window: a DMR is a consecutive run of CpGs all with
//      |delta_beta| > 0.1 AND the same direction, spanning >= min_cpgs.
// Returns Table: start_idx, end_idx, n_cpgs, mean_delta, direction.

fn builtin_dmr_find(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "dmr_find")?;
    let idx_a = require_index_list_fn(&args[1], "dmr_find")?;
    let idx_b = require_index_list_fn(&args[2], "dmr_find")?;
    let min_cpgs = if args.len() > 3 {
        require_int(&args[3], "dmr_find")? as usize
    } else {
        3
    };

    // mat: rows = CpGs, cols = samples
    let n_cpgs = mat.len();
    let mut delta: Vec<f64> = Vec::with_capacity(n_cpgs);
    for row in &mat {
        let a_vals: Vec<f64> = idx_a.iter().filter_map(|&i| row.get(i).copied()).collect();
        let b_vals: Vec<f64> = idx_b.iter().filter_map(|&i| row.get(i).copied()).collect();
        delta.push(vec_mean(&a_vals) - vec_mean(&b_vals));
    }

    let threshold = 0.1_f64;
    let mut dmrs: Vec<Vec<Value>> = Vec::new();
    let mut i = 0;
    while i < n_cpgs {
        if delta[i].abs() < threshold {
            i += 1;
            continue;
        }
        let dir = delta[i].signum();
        let start = i;
        while i < n_cpgs && delta[i].signum() == dir && delta[i].abs() >= threshold {
            i += 1;
        }
        let end = i - 1;
        let n = end - start + 1;
        if n >= min_cpgs {
            let mean_delta = delta[start..=end].iter().sum::<f64>() / n as f64;
            dmrs.push(vec![
                Value::Int(start as i64),
                Value::Int(end as i64),
                Value::Int(n as i64),
                Value::Float(mean_delta),
                Value::Str(if dir > 0.0 { "hyper" } else { "hypo" }.to_string()),
            ]);
        }
    }

    let cols = vec![
        "start_idx".to_string(),
        "end_idx".to_string(),
        "n_cpgs".to_string(),
        "mean_delta".to_string(),
        "direction".to_string(),
    ];
    Ok(Value::Table(Table::new(cols, dmrs)))
}

fn require_index_list_fn(val: &Value, func: &str) -> Result<Vec<usize>> {
    match val {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) if *n >= 0 => Ok(*n as usize),
                _ => Err(BioLangError::type_error(
                    format!("{func}() index list must contain non-negative Ints"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires a List of Int indices"),
            None,
        )),
    }
}

// ── cpg_density(sequence) ─────────────────────────────────────────────
// Count CpG dinucleotides and return density (CpGs per 100 bp).
// Returns a Table: cpg_count, length, density.

fn builtin_cpg_density(args: Vec<Value>) -> Result<Value> {
    let seq = match &args[0] {
        Value::Str(s) => s.to_uppercase(),
        _ => {
            return Err(BioLangError::type_error(
                "cpg_density() requires a String",
                None,
            ))
        }
    };
    let length = seq.len();
    let cpg_count = seq.as_bytes().windows(2).filter(|w| w == b"CG").count();
    let density = if length < 2 {
        0.0
    } else {
        cpg_count as f64 / (length - 1) as f64 * 100.0
    };

    let cols = vec![
        "cpg_count".to_string(),
        "length".to_string(),
        "density".to_string(),
    ];
    let rows = vec![vec![
        Value::Int(cpg_count as i64),
        Value::Int(length as i64),
        Value::Float(density),
    ]];
    Ok(Value::Table(Table::new(cols, rows)))
}

// ── epigenetic_age(beta_matrix, cpg_ids) ─────────────────────────────
//
// Simplified Horvath 2013 clock: linear combination of 353 CpGs.
// Since we cannot ship the full coefficient table, we implement a
// structural approximation: sample-wise weighted mean of provided CpGs.
// beta_matrix: rows=CpGs, cols=samples.
// cpg_ids: List of CpG labels (used for weighting by index).
// Returns a List of estimated ages (one per sample).
//
// The Horvath intercept is 0.696 and the transformation is:
//   if linear_combination < 0:  age = (1 + adult_age) * 2^(linear_comb) - 1
//   else:                       age = (1 + adult_age) * linear_comb + adult_age
// with adult_age = 20.

fn builtin_epigenetic_age(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "epigenetic_age")?;
    let _cpg_ids = require_str_list(&args[1], "epigenetic_age")?;

    let n_cpgs = mat.len();
    if n_cpgs == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let n_samples = mat[0].len();
    const INTERCEPT: f64 = 0.696;
    const ADULT_AGE: f64 = 20.0;

    // Synthetic weights: cosine of rank to mimic sign variation
    let weights: Vec<f64> = (0..n_cpgs)
        .map(|i| ((i as f64 * std::f64::consts::PI) / n_cpgs as f64).cos() * 0.1)
        .collect();

    let ages: Vec<Value> = (0..n_samples)
        .map(|s| {
            let lc: f64 = INTERCEPT
                + weights
                    .iter()
                    .zip(mat.iter())
                    .map(|(w, row)| w * row.get(s).copied().unwrap_or(0.0))
                    .sum::<f64>();
            let age = if lc < 0.0 {
                (1.0 + ADULT_AGE) * 2.0f64.powf(lc) - 1.0
            } else {
                (1.0 + ADULT_AGE) * lc + ADULT_AGE
            };
            Value::Float(age.max(0.0))
        })
        .collect();

    Ok(Value::List((ages).into()))
}

// ── differential_methylation(beta_matrix, group_a_indices, group_b_indices) ──
//
// Per-CpG Welch t-test between two sample groups.
// beta_matrix: rows=CpGs, cols=samples.
// Returns Table: cpg_idx, mean_a, mean_b, delta_beta, t_stat, p_value.

fn builtin_differential_methylation(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "differential_methylation")?;
    let idx_a = require_index_list_fn(&args[1], "differential_methylation")?;
    let idx_b = require_index_list_fn(&args[2], "differential_methylation")?;

    let cols = vec![
        "cpg_idx".to_string(),
        "mean_a".to_string(),
        "mean_b".to_string(),
        "delta_beta".to_string(),
        "t_stat".to_string(),
        "p_value".to_string(),
    ];

    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(mat.len());
    for (i, row) in mat.iter().enumerate() {
        let a: Vec<f64> = idx_a.iter().filter_map(|&j| row.get(j).copied()).collect();
        let b: Vec<f64> = idx_b.iter().filter_map(|&j| row.get(j).copied()).collect();
        let mean_a = vec_mean(&a);
        let mean_b = vec_mean(&b);
        let delta = mean_a - mean_b;
        let (t, p) = welch_ttest(&a, &b);
        rows.push(vec![
            Value::Int(i as i64),
            Value::Float(mean_a),
            Value::Float(mean_b),
            Value::Float(delta),
            Value::Float(t),
            Value::Float(p),
        ]);
    }

    Ok(Value::Table(Table::new(cols, rows)))
}
