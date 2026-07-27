//! Drug response and pharmacogenomics builtins.
//!
//! Functions: fit_ic50, dose_response_curve, auc_response,
//! bliss_synergy, loewe_synergy, drug_rank.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

// ── Registry ─────────────────────────────────────────────────────────

pub fn drug_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("fit_ic50", Arity::Exact(2)),
        ("dose_response_curve", Arity::Exact(5)),
        ("auc_response", Arity::Exact(2)),
        ("bliss_synergy", Arity::Exact(3)),
        ("loewe_synergy", Arity::Exact(5)),
        ("drug_rank", Arity::Range(1, 2)),
    ]
}

pub fn is_drug_builtin(name: &str) -> bool {
    matches!(
        name,
        "fit_ic50"
            | "dose_response_curve"
            | "auc_response"
            | "bliss_synergy"
            | "loewe_synergy"
            | "drug_rank"
    )
}

pub fn call_drug_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "fit_ic50" => builtin_fit_ic50(args),
        "dose_response_curve" => builtin_dose_response_curve(args),
        "auc_response" => builtin_auc_response(args),
        "bliss_synergy" => builtin_bliss_synergy(args),
        "loewe_synergy" => builtin_loewe_synergy(args),
        "drug_rank" => builtin_drug_rank(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown drug builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

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
                    format!("{func}() requires List<Float|Int>"),
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

fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Table"),
            None,
        )),
    }
}

fn median_f64(vals: &[f64]) -> f64 {
    if vals.is_empty() {
        return 0.0;
    }
    let mut s = vals.to_vec();
    s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = s.len();
    if n % 2 == 0 {
        (s[n / 2 - 1] + s[n / 2]) / 2.0
    } else {
        s[n / 2]
    }
}

/// Evaluate 4PL model at a single concentration.
fn four_pl(conc: f64, ic50: f64, slope: f64, top: f64, bottom: f64) -> f64 {
    if conc <= 0.0 {
        return top;
    }
    let ratio = ic50 / conc;
    bottom + (top - bottom) / (1.0 + ratio.powf(slope))
}

// ── fit_ic50 ─────────────────────────────────────────────────────────

fn builtin_fit_ic50(args: Vec<Value>) -> Result<Value> {
    let concentrations = require_f64_list(&args[0], "fit_ic50")?;
    let viabilities = require_f64_list(&args[1], "fit_ic50")?;

    if concentrations.len() != viabilities.len() || concentrations.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "fit_ic50(): concentrations and viabilities must be non-empty and same length".to_string(),
            None,
        ));
    }

    let min_conc = concentrations.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_conc = concentrations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let ic50_min = min_conc * 0.01;
    let ic50_max = max_conc * 100.0;

    // Initialize parameters
    let mut ic50 = median_f64(&concentrations);
    let mut slope = 1.0f64;
    let mut top = 100.0f64;
    let mut bottom = 0.0f64;

    let lr = 1e-5;
    let n = concentrations.len() as f64;

    for _ in 0..3000 {
        let mut d_ic50 = 0.0f64;
        let mut d_slope = 0.0f64;
        let mut d_top = 0.0f64;
        let mut d_bottom = 0.0f64;

        for (&c, &y) in concentrations.iter().zip(viabilities.iter()) {
            if c <= 0.0 {
                continue;
            }
            let pred = four_pl(c, ic50, slope, top, bottom);
            let residual = pred - y; // dL/dpred = 2*residual (dropping the 2)

            let ratio = ic50 / c;
            let ratio_s = ratio.powf(slope);
            let denom = 1.0 + ratio_s;
            let denom2 = denom * denom;
            let range = top - bottom;

            // dpred/dtop = 1/(1+ratio^slope)
            d_top += residual / denom;

            // dpred/dbottom = 1 - 1/(1+ratio^slope)
            d_bottom += residual * (1.0 - 1.0 / denom);

            // dpred/dic50 = range * (-slope * ratio^(slope-1) / c) / denom^2
            //             = -range * slope * ratio^slope / (ic50 * denom^2)
            if ic50 > 0.0 {
                d_ic50 += residual * (-range * slope * ratio_s) / (ic50 * denom2);
            }

            // dpred/dslope = -range * ratio^slope * ln(ratio) / denom^2
            if ratio > 0.0 {
                d_slope += residual * (-range * ratio_s * ratio.ln()) / denom2;
            }
        }

        // Update with gradient (divide by n for mean)
        ic50 -= lr * d_ic50 / n;
        slope -= lr * d_slope / n;
        top -= lr * d_top / n;
        bottom -= lr * d_bottom / n;

        // Clip
        ic50 = ic50.clamp(ic50_min, ic50_max);
        slope = slope.clamp(0.1, 10.0);
    }

    // Compute R²
    let mean_y = viabilities.iter().sum::<f64>() / n;
    let ss_tot: f64 = viabilities.iter().map(|&y| (y - mean_y).powi(2)).sum();
    let ss_res: f64 = concentrations
        .iter()
        .zip(viabilities.iter())
        .map(|(&c, &y)| {
            let pred = four_pl(c, ic50, slope, top, bottom);
            (y - pred).powi(2)
        })
        .sum();
    let r2 = if ss_tot == 0.0 {
        1.0
    } else {
        1.0 - ss_res / ss_tot
    };

    let mut rec = HashMap::new();
    rec.insert("ic50".to_string(), Value::Float(ic50));
    rec.insert("slope".to_string(), Value::Float(slope));
    rec.insert("top".to_string(), Value::Float(top));
    rec.insert("bottom".to_string(), Value::Float(bottom));
    rec.insert("r2".to_string(), Value::Float(r2));

    Ok(Value::Record((rec).into()))
}

// ── dose_response_curve ───────────────────────────────────────────────

fn builtin_dose_response_curve(args: Vec<Value>) -> Result<Value> {
    let concentrations = require_f64_list(&args[0], "dose_response_curve")?;
    let ic50 = to_f64(&args[1]);
    let slope = to_f64(&args[2]);
    let top = to_f64(&args[3]);
    let bottom = to_f64(&args[4]);

    let result: Vec<Value> = concentrations
        .iter()
        .map(|&c| Value::Float(four_pl(c, ic50, slope, top, bottom)))
        .collect();

    Ok(Value::List((result).into()))
}

// ── auc_response ─────────────────────────────────────────────────────

fn builtin_auc_response(args: Vec<Value>) -> Result<Value> {
    let concentrations = require_f64_list(&args[0], "auc_response")?;
    let viabilities = require_f64_list(&args[1], "auc_response")?;

    if concentrations.len() != viabilities.len() || concentrations.len() < 2 {
        return Ok(Value::Float(0.0));
    }

    // Sort by concentration
    let mut pairs: Vec<(f64, f64)> = concentrations
        .iter()
        .zip(viabilities.iter())
        .map(|(&c, &v)| (c, v))
        .collect();
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let min_c = pairs[0].0;
    let max_c = pairs[pairs.len() - 1].0;
    let log_range = if min_c > 0.0 && max_c > min_c {
        (max_c / min_c).log10()
    } else {
        1.0
    };

    // Trapezoidal integration over log10(concentration)
    let mut auc = 0.0f64;
    for i in 1..pairs.len() {
        let lc0 = if pairs[i - 1].0 > 0.0 {
            pairs[i - 1].0.log10()
        } else {
            continue
        };
        let lc1 = if pairs[i].0 > 0.0 {
            pairs[i].0.log10()
        } else {
            continue
        };
        let dlc = lc1 - lc0;
        auc += (pairs[i - 1].1 + pairs[i].1) * 0.5 * dlc;
    }

    // Normalize by log range
    let normalized = if log_range > 0.0 { auc / log_range } else { 0.0 };
    Ok(Value::Float(normalized))
}

// ── bliss_synergy ─────────────────────────────────────────────────────

fn builtin_bliss_synergy(args: Vec<Value>) -> Result<Value> {
    let viab_a = to_f64(&args[0]);
    let viab_b = to_f64(&args[1]);
    let viab_combo = to_f64(&args[2]);

    // Bliss expected: fraction_a * fraction_b, converted back to %
    let expected = (viab_a / 100.0) * (viab_b / 100.0) * 100.0;
    let synergy = viab_combo - expected;

    Ok(Value::Float(synergy))
}

// ── loewe_synergy ────────────────────────────────────────────────────

fn builtin_loewe_synergy(args: Vec<Value>) -> Result<Value> {
    let ic50_a = to_f64(&args[0]);
    let ic50_b = to_f64(&args[1]);
    let conc_a = to_f64(&args[2]);
    let conc_b = to_f64(&args[3]);
    // args[4] is observed_effect — included for signature compat, not used in CI formula
    let _observed = to_f64(&args[4]);

    if ic50_a == 0.0 || ic50_b == 0.0 {
        return Ok(Value::Float(0.0));
    }

    let ci = conc_a / ic50_a + conc_b / ic50_b;
    let synergy = 1.0 - ci;

    Ok(Value::Float(synergy))
}

// ── drug_rank ────────────────────────────────────────────────────────

fn builtin_drug_rank(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "drug_rank")?.clone();
    let ascending = if args.len() > 1 {
        match &args[1] {
            Value::Bool(b) => *b,
            _ => true,
        }
    } else {
        true
    };

    let ic50_col = table
        .columns
        .iter()
        .position(|c| c == "ic50")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "drug_rank(): column 'ic50' not found".to_string(),
                None,
            )
        })?;

    // Collect (row_index, ic50_value) and sort
    let mut indexed: Vec<(usize, f64)> = table
        .rows
        .iter()
        .enumerate()
        .map(|(i, row)| (i, to_f64(&row[ic50_col])))
        .collect();

    if ascending {
        indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
    } else {
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    }

    let mut out_columns = table.columns.clone();
    out_columns.push("rank".to_string());

    let rows: Vec<Vec<Value>> = indexed
        .iter()
        .enumerate()
        .map(|(rank, (orig_idx, _))| {
            let mut row = table.rows[*orig_idx].clone();
            row.push(Value::Int((rank + 1) as i64));
            row
        })
        .collect();

    Ok(Value::Table(Table::new(out_columns, rows)))
}
