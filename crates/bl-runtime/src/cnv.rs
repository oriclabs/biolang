//! Copy number variation (CNV) builtins.
//!
//! Functions: log2_ratios, cbs_segment, cn_call, allele_specific_cn, cnv_summary.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ─────────────────────────────────────────────────────────

pub fn cnv_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("log2_ratios", Arity::Exact(2)),
        ("cbs_segment", Arity::Exact(1)),
        ("cn_call", Arity::Exact(2)),
        ("allele_specific_cn", Arity::Exact(2)),
        ("cnv_summary", Arity::Exact(1)),
    ]
}

pub fn is_cnv_builtin(name: &str) -> bool {
    matches!(
        name,
        "log2_ratios" | "cbs_segment" | "cn_call" | "allele_specific_cn" | "cnv_summary"
    )
}

pub fn call_cnv_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "log2_ratios" => builtin_log2_ratios(args),
        "cbs_segment" => builtin_cbs_segment(args),
        "cn_call" => builtin_cn_call(args),
        "allele_specific_cn" => builtin_allele_specific_cn(args),
        "cnv_summary" => builtin_cnv_summary(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown cnv builtin '{name}'"),
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

fn require_f64_list(val: &Value, func: &str) -> Result<Vec<f64>> {
    match val {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Float(f) => Ok(*f),
                Value::Int(n) => Ok(*n as f64),
                _ => Err(BioLangError::type_error(
                    format!("{func}() list must contain Float|Int"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List"),
            None,
        )),
    }
}

/// Student t CDF approximation using a rational approximation to erf.
/// Returns P(T ≤ t) for the two-tailed case via a normal approximation
/// when df is large, or a simple approximation otherwise.
fn t_cdf_two_tailed(t: f64, df: f64) -> f64 {
    if df <= 0.0 {
        return 1.0;
    }
    // Use normal approximation for df > 30, else use beta function approx
    let t_abs = t.abs();
    if df > 30.0 {
        // Normal approximation
        let z = t_abs * (1.0 - 1.0 / (4.0 * df)).powf(0.5);
        let p = erfc_approx(z / std::f64::consts::SQRT_2) / 2.0;
        (2.0 * p).min(1.0)
    } else {
        // Simplified: x = df / (df + t^2), regularized incomplete beta
        let x = df / (df + t_abs * t_abs);
        // Use a rough approximation via series expansion
        let p = regularized_beta(x, df / 2.0, 0.5);
        p.min(1.0)
    }
}

/// Approximation to erfc (complementary error function).
fn erfc_approx(x: f64) -> f64 {
    if x < 0.0 {
        return 2.0 - erfc_approx(-x);
    }
    // Abramowitz & Stegun 7.1.26
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let poly = t
        * (0.254829592
            + t * (-0.284496736 + t * (1.421413741 + t * (-1.453152027 + t * 1.061405429))));
    poly * (-x * x).exp()
}

/// Regularized incomplete beta function I_x(a, b) — rough approximation via
/// continued fraction (a few terms only, adequate for our p-value threshold).
fn regularized_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    // Use logarithm of beta function kernel then continued fraction
    let ln_beta = lgamma(a) + lgamma(b) - lgamma(a + b);
    let front = (x.powf(a) * (1.0 - x).powf(b)).ln() - ln_beta;
    // Modified Lentz continued fraction — 20 terms
    let mut f = 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < 1e-30 {
        d = 1e-30;
    }
    d = 1.0 / d;
    f = d;
    for m in 1..=20usize {
        let mf = m as f64;
        // Even step
        let num_e = mf * (b - mf) * x / ((a + 2.0 * mf - 1.0) * (a + 2.0 * mf));
        d = 1.0 + num_e * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + num_e / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        f *= d * c;
        // Odd step
        let num_o = -(a + mf) * (a + b + mf) * x / ((a + 2.0 * mf) * (a + 2.0 * mf + 1.0));
        d = 1.0 + num_o * d;
        if d.abs() < 1e-30 {
            d = 1e-30;
        }
        c = 1.0 + num_o / c;
        if c.abs() < 1e-30 {
            c = 1e-30;
        }
        d = 1.0 / d;
        let delta = d * c;
        f *= delta;
        if (delta - 1.0).abs() < 1e-8 {
            break;
        }
    }
    let bt = (front + f.ln()).exp();
    bt / a
}

/// Log gamma via Stirling approximation.
fn lgamma(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let mut xx = x;
    let mut res = 0.0;
    while xx < 7.0 {
        res -= xx.ln();
        xx += 1.0;
    }
    res += (2.0 * std::f64::consts::PI / xx).sqrt().ln() + xx * (xx.ln() - 1.0) + 1.0 / (12.0 * xx)
        - 1.0 / (360.0 * xx.powi(3));
    res
}

/// Welch t-test p-value for two samples.
fn welch_t_pvalue(a: &[f64], b: &[f64]) -> f64 {
    if a.len() < 2 || b.len() < 2 {
        return 1.0;
    }
    let n1 = a.len() as f64;
    let n2 = b.len() as f64;
    let mean1 = a.iter().sum::<f64>() / n1;
    let mean2 = b.iter().sum::<f64>() / n2;
    let var1 = a.iter().map(|&x| (x - mean1).powi(2)).sum::<f64>() / (n1 - 1.0);
    let var2 = b.iter().map(|&x| (x - mean2).powi(2)).sum::<f64>() / (n2 - 1.0);
    let se = (var1 / n1 + var2 / n2).sqrt();
    if se == 0.0 {
        return 1.0;
    }
    let t = (mean1 - mean2).abs() / se;
    // Welch-Satterthwaite df
    let df_num = (var1 / n1 + var2 / n2).powi(2);
    let df_den = (var1 / n1).powi(2) / (n1 - 1.0) + (var2 / n2).powi(2) / (n2 - 1.0);
    let df = if df_den == 0.0 { 1.0 } else { df_num / df_den };
    t_cdf_two_tailed(t, df)
}

fn mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f64>() / v.len() as f64
}

// ── CBS segmentation ─────────────────────────────────────────────────

struct Segment {
    start: usize,
    end: usize, // exclusive
}

fn cbs_recursive(ratios: &[f64], offset: usize, segments: &mut Vec<Segment>) {
    let n = ratios.len();
    if n < 5 {
        segments.push(Segment {
            start: offset,
            end: offset + n,
        });
        return;
    }

    // Find split point minimising within-segment variance
    let mut best_split = None;
    let mut best_score = f64::INFINITY;

    for split in 2..(n - 2) {
        let left = &ratios[..split];
        let right = &ratios[split..];
        let n_l = left.len() as f64;
        let n_r = right.len() as f64;
        let var_l: f64 = {
            let m = mean(left);
            left.iter().map(|&x| (x - m).powi(2)).sum::<f64>() / n_l
        };
        let var_r: f64 = {
            let m = mean(right);
            right.iter().map(|&x| (x - m).powi(2)).sum::<f64>() / n_r
        };
        let score = var_l * n_l + var_r * n_r;
        if score < best_score {
            best_score = score;
            best_split = Some(split);
        }
    }

    if let Some(split) = best_split {
        let left = &ratios[..split];
        let right = &ratios[split..];
        let p = welch_t_pvalue(left, right);
        if p < 0.05 {
            cbs_recursive(left, offset, segments);
            cbs_recursive(right, offset + split, segments);
            return;
        }
    }

    segments.push(Segment {
        start: offset,
        end: offset + n,
    });
}

// ── log2_ratios ──────────────────────────────────────────────────────

fn builtin_log2_ratios(args: Vec<Value>) -> Result<Value> {
    let tumor = require_f64_list(&args[0], "log2_ratios")?;
    let normal = require_f64_list(&args[1], "log2_ratios")?;
    if tumor.len() != normal.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "log2_ratios(): tumor length {} != normal length {}",
                tumor.len(),
                normal.len()
            ),
            None,
        ));
    }
    let result: Vec<Value> = tumor
        .iter()
        .zip(normal.iter())
        .map(|(&t, &n)| Value::Float(((t + 1.0) / (n + 1.0)).log2()))
        .collect();
    Ok(Value::List((result).into()))
}

// ── cbs_segment ──────────────────────────────────────────────────────

fn builtin_cbs_segment(args: Vec<Value>) -> Result<Value> {
    let ratios = require_f64_list(&args[0], "cbs_segment")?;

    let mut raw_segments: Vec<Segment> = Vec::new();
    cbs_recursive(&ratios, 0, &mut raw_segments);
    raw_segments.sort_by_key(|s| s.start);

    // Merge adjacent segments whose means differ by < 0.1
    let mut merged: Vec<(usize, usize, f64)> = Vec::new(); // (start, end, mean)
    for seg in &raw_segments {
        let seg_mean = mean(&ratios[seg.start..seg.end]);
        if let Some(last) = merged.last_mut() {
            if (last.2 - seg_mean).abs() < 0.1 {
                last.1 = seg.end;
                let slice = &ratios[last.0..last.1];
                last.2 = mean(slice);
                continue;
            }
        }
        merged.push((seg.start, seg.end, seg_mean));
    }

    let col_names = vec![
        "start".to_string(),
        "end".to_string(),
        "mean_ratio".to_string(),
        "n_bins".to_string(),
    ];
    let rows: Vec<Vec<Value>> = merged
        .into_iter()
        .map(|(s, e, m)| {
            vec![
                Value::Int(s as i64),
                Value::Int(e as i64),
                Value::Float(m),
                Value::Int((e - s) as i64),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── cn_call ──────────────────────────────────────────────────────────

fn builtin_cn_call(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "cn_call")?.clone();
    let ploidy = match &args[1] {
        Value::Int(n) => *n as f64,
        Value::Float(f) => *f,
        _ => {
            return Err(BioLangError::type_error(
                "cn_call() ploidy must be Int|Float",
                None,
            ))
        }
    };

    let mean_ratio_col = table
        .columns
        .iter()
        .position(|c| c == "mean_ratio")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "cn_call(): column 'mean_ratio' not found".to_string(),
                None,
            )
        })?;

    let mut out_columns = table.columns.clone();
    out_columns.push("copy_number".to_string());

    let rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .map(|row| {
            let ratio = to_f64(&row[mean_ratio_col]);
            let cn = (ploidy * 2f64.powf(ratio)).round() as i64;
            let cn = cn.clamp(0, 8);
            let mut new_row = row.clone();
            new_row.push(Value::Int(cn));
            new_row
        })
        .collect();

    Ok(Value::Table(Table::new(out_columns, rows)))
}

// ── allele_specific_cn ───────────────────────────────────────────────

fn builtin_allele_specific_cn(args: Vec<Value>) -> Result<Value> {
    let baf = require_f64_list(&args[0], "allele_specific_cn")?;
    let ratio = require_f64_list(&args[1], "allele_specific_cn")?;
    if baf.len() != ratio.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "allele_specific_cn(): baf length {} != ratio length {}",
                baf.len(),
                ratio.len()
            ),
            None,
        ));
    }

    let col_names = vec![
        "total_cn".to_string(),
        "major_cn".to_string(),
        "minor_cn".to_string(),
        "baf".to_string(),
        "log2_ratio".to_string(),
    ];

    let rows: Vec<Vec<Value>> = baf
        .iter()
        .zip(ratio.iter())
        .map(|(&b, &r)| {
            let total_cn = ((2.0 * 2f64.powf(r)).round() as i64).clamp(0, 8);
            let minor_allele_frac = b.min(1.0 - b);
            let minor_cn =
                ((total_cn as f64 * minor_allele_frac).round() as i64).clamp(0, total_cn);
            let major_cn = total_cn - minor_cn;
            vec![
                Value::Int(total_cn),
                Value::Int(major_cn),
                Value::Int(minor_cn),
                Value::Float(b),
                Value::Float(r),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── cnv_summary ──────────────────────────────────────────────────────

fn builtin_cnv_summary(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "cnv_summary")?;

    let start_col = table
        .columns
        .iter()
        .position(|c| c == "start")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "cnv_summary(): column 'start' not found".to_string(),
                None,
            )
        })?;
    let end_col = table
        .columns
        .iter()
        .position(|c| c == "end")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "cnv_summary(): column 'end' not found".to_string(),
                None,
            )
        })?;
    let ratio_col = table
        .columns
        .iter()
        .position(|c| c == "mean_ratio")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "cnv_summary(): column 'mean_ratio' not found".to_string(),
                None,
            )
        })?;

    let n_segments = table.rows.len() as i64;
    let mut total_bins = 0i64;
    let mut amplified = 0i64;
    let mut deleted = 0i64;
    let mut ratio_sum = 0.0f64;

    for row in &table.rows {
        let start = to_f64(&row[start_col]) as i64;
        let end = to_f64(&row[end_col]) as i64;
        let ratio = to_f64(&row[ratio_col]);
        let n_bins = (end - start).max(0);
        total_bins += n_bins;
        ratio_sum += ratio * n_bins as f64;
        if ratio > 0.58 {
            amplified += n_bins;
        }
        if ratio < -1.0 {
            deleted += n_bins;
        }
    }

    let mean_ratio = if total_bins > 0 {
        ratio_sum / total_bins as f64
    } else {
        0.0
    };
    let fraction_altered = if total_bins > 0 {
        (amplified + deleted) as f64 / total_bins as f64
    } else {
        0.0
    };

    let mut fields = std::collections::HashMap::new();
    fields.insert("n_segments".to_string(), Value::Int(n_segments));
    fields.insert("n_bins_amplified".to_string(), Value::Int(amplified));
    fields.insert("n_bins_deleted".to_string(), Value::Int(deleted));
    fields.insert("mean_ratio".to_string(), Value::Float(mean_ratio));
    fields.insert(
        "fraction_altered".to_string(),
        Value::Float(fraction_altered),
    );
    Ok(Value::Record((fields).into()))
}
