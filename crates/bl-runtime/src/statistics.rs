//! Statistical builtins: multiple testing correction, bootstrap CI, genomic
//! inflation factor, Pearson correlation.
//!
//! `fisher_exact`, `chi_square` and `permutation_test` used to be here as well
//! as in `stats.rs`. Dispatch reaches `stats.rs` first, so this module's copies
//! were unreachable -- but both registries feed one arity table, and this one
//! was appended second, so its `Exact(3)` won the arity check for
//! `permutation_test` while `stats.rs` provided the implementation. The
//! documented fourth argument was therefore rejected before any code that could
//! have used it ran. They are gone; `stats.rs` owns all three.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ──────────────────────────────────────────────────────────

pub fn statistics_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("bh_adjust", Arity::Exact(1)),
        ("bonferroni_adjust", Arity::Exact(1)),
        ("bootstrap_ci", Arity::Range(1, 3)),
        ("genomic_inflation", Arity::Exact(1)),
        ("pearson_correlation", Arity::Exact(2)),
    ]
}

pub fn is_statistics_builtin(name: &str) -> bool {
    matches!(
        name,
        "bh_adjust"
            | "bonferroni_adjust"
            | "bootstrap_ci"
            | "genomic_inflation"
            | "pearson_correlation"
    )
}

pub fn call_statistics_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "bh_adjust" => builtin_bh_adjust(args),
        "bonferroni_adjust" => builtin_bonferroni_adjust(args),
        "bootstrap_ci" => builtin_bootstrap_ci(args),
        "genomic_inflation" => builtin_genomic_inflation(args),
        "pearson_correlation" => builtin_pearson_correlation(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown statistics builtin: {name}"),
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

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Int"),
            None,
        )),
    }
}

fn require_float(val: &Value, func: &str) -> Result<f64> {
    to_f64(val).ok_or_else(|| BioLangError::type_error(format!("{func}() requires a number"), None))
}

/// Inverse normal CDF via Beasley-Springer-Moro algorithm.
fn probit(p: f64) -> f64 {
    if p <= 0.0 {
        return -8.0;
    }
    if p >= 1.0 {
        return 8.0;
    }
    // rational approximation; splits at p = 0.5
    let (sign, q) = if p < 0.5 { (-1.0, p) } else { (1.0, 1.0 - p) };
    let r = (-2.0 * q.ln()).sqrt();
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;
    let num = c0 + c1 * r + c2 * r * r;
    let den = 1.0 + d1 * r + d2 * r * r + d3 * r * r * r;
    sign * (r - num / den)
}

/// LCG pseudo-random number generator (deterministic seed).
fn lcg_next(state: u64) -> u64 {
    state.wrapping_mul(1664525).wrapping_add(1013904223) & 0xFFFF_FFFF
}

fn vec_mean(v: &[f64]) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.iter().sum::<f64>() / v.len() as f64
}

fn vec_median(v: &mut Vec<f64>) -> f64 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    }
}

// ── bh_adjust(p_values) ───────────────────────────────────────────────

fn builtin_bh_adjust(args: Vec<Value>) -> Result<Value> {
    let ps = require_float_list(&args[0], "bh_adjust")?;
    let n = ps.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }

    // Pair (p-value, original index), sort ascending
    let mut indexed: Vec<(f64, usize)> = ps.iter().copied().zip(0..).collect();
    indexed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // BH: adjusted[i] = p[i] * n / (rank+1), then enforce monotonicity from largest rank down
    let mut adjusted = vec![0.0f64; n];
    for (rank, &(p, _orig)) in indexed.iter().enumerate() {
        adjusted[rank] = (p * n as f64 / (rank + 1) as f64).min(1.0);
    }
    // Enforce monotonicity: scan from largest rank downward
    for i in (0..n - 1).rev() {
        if adjusted[i] > adjusted[i + 1] {
            adjusted[i] = adjusted[i + 1];
        }
    }

    // Map back to original order
    let mut result = vec![0.0f64; n];
    for (rank, &(_p, orig)) in indexed.iter().enumerate() {
        result[orig] = adjusted[rank];
    }

    Ok(Value::List(
        result
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    ))
}

// ── bonferroni_adjust(p_values) ───────────────────────────────────────

fn builtin_bonferroni_adjust(args: Vec<Value>) -> Result<Value> {
    let ps = require_float_list(&args[0], "bonferroni_adjust")?;
    let n = ps.len() as f64;
    let adjusted: Vec<Value> = ps.iter().map(|&p| Value::Float((p * n).min(1.0))).collect();
    Ok(Value::List((adjusted).into()))
}

// ── bootstrap_ci(values, n_bootstrap=1000, confidence=0.95) ──────────

fn builtin_bootstrap_ci(args: Vec<Value>) -> Result<Value> {
    let vals = require_float_list(&args[0], "bootstrap_ci")?;
    let n_boot = if args.len() > 1 {
        (require_int(&args[1], "bootstrap_ci")? as usize).min(10_000)
    } else {
        1000
    };
    let confidence = if args.len() > 2 {
        require_float(&args[2], "bootstrap_ci")?
    } else {
        0.95
    };

    if vals.is_empty() {
        let columns = vec![
            "mean".to_string(),
            "lower".to_string(),
            "upper".to_string(),
            "std_err".to_string(),
        ];
        let rows = vec![vec![
            Value::Float(0.0),
            Value::Float(0.0),
            Value::Float(0.0),
            Value::Float(0.0),
        ]];
        return Ok(Value::Table(Table::new(columns, rows)));
    }

    let n = vals.len();
    let observed_mean = vec_mean(&vals);

    let mut boot_means: Vec<f64> = Vec::with_capacity(n_boot);
    let mut state = 42u64;
    for _ in 0..n_boot {
        let mut sample = Vec::with_capacity(n);
        for _ in 0..n {
            state = lcg_next(state);
            sample.push(vals[(state as usize) % n]);
        }
        boot_means.push(vec_mean(&sample));
    }

    boot_means.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let alpha = (1.0 - confidence) / 2.0;
    let lo_idx = ((alpha * n_boot as f64) as usize).min(n_boot.saturating_sub(1));
    let hi_idx = (((1.0 - alpha) * n_boot as f64) as usize).min(n_boot.saturating_sub(1));
    let lower = boot_means[lo_idx];
    let upper = boot_means[hi_idx];

    let variance = boot_means
        .iter()
        .map(|&m| (m - observed_mean).powi(2))
        .sum::<f64>()
        / n_boot as f64;
    let std_err = variance.sqrt();

    let columns = vec![
        "mean".to_string(),
        "lower".to_string(),
        "upper".to_string(),
        "std_err".to_string(),
    ];
    let rows = vec![vec![
        Value::Float(observed_mean),
        Value::Float(lower),
        Value::Float(upper),
        Value::Float(std_err),
    ]];
    Ok(Value::Table(Table::new(columns, rows)))
}

// ── genomic_inflation(p_values) ───────────────────────────────────────

fn builtin_genomic_inflation(args: Vec<Value>) -> Result<Value> {
    let ps = require_float_list(&args[0], "genomic_inflation")?;
    if ps.is_empty() {
        return Ok(Value::Float(1.0));
    }

    // Convert p → chi2(1) via probit: z = Φ⁻¹(1-p), chi2 = z²
    let mut chi2: Vec<f64> = ps
        .iter()
        .map(|&p| {
            let z = probit(1.0 - p.clamp(1e-300, 1.0));
            z * z
        })
        .collect();

    let median_obs = vec_median(&mut chi2);
    // Expected median of chi2(1) = 0.4549
    let lambda = median_obs / 0.4549;
    Ok(Value::Float(lambda.max(0.0)))
}

// ── pearson_correlation(x, y) ─────────────────────────────────────────

fn builtin_pearson_correlation(args: Vec<Value>) -> Result<Value> {
    let x = require_float_list(&args[0], "pearson_correlation")?;
    let y = require_float_list(&args[1], "pearson_correlation")?;
    if x.len() != y.len() {
        return Err(BioLangError::runtime(
            ErrorKind::IndexOutOfBounds,
            format!(
                "pearson_correlation(): x length {} != y length {}",
                x.len(),
                y.len()
            ),
            None,
        ));
    }
    if x.is_empty() {
        return Ok(Value::Float(0.0));
    }

    let mx = vec_mean(&x);
    let my = vec_mean(&y);

    let num: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| (xi - mx) * (yi - my))
        .sum();
    let denom_x: f64 = x.iter().map(|&xi| (xi - mx).powi(2)).sum::<f64>().sqrt();
    let denom_y: f64 = y.iter().map(|&yi| (yi - my).powi(2)).sum::<f64>().sqrt();

    if denom_x == 0.0 || denom_y == 0.0 {
        return Ok(Value::Float(0.0));
    }

    Ok(Value::Float((num / (denom_x * denom_y)).clamp(-1.0, 1.0)))
}
