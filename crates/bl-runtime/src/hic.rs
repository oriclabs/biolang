//! Hi-C chromatin conformation analysis builtins.
//!
//! Functions: ice_normalize, insulation_score, tad_boundaries, distance_decay, expected_contacts.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ─────────────────────────────────────────────────────────

pub fn hic_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("ice_normalize", Arity::Exact(1)),
        ("insulation_score", Arity::Exact(2)),
        ("tad_boundaries", Arity::Exact(2)),
        ("distance_decay", Arity::Exact(1)),
        ("expected_contacts", Arity::Exact(1)),
    ]
}

pub fn is_hic_builtin(name: &str) -> bool {
    matches!(
        name,
        "ice_normalize"
            | "insulation_score"
            | "tad_boundaries"
            | "distance_decay"
            | "expected_contacts"
    )
}

pub fn call_hic_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "ice_normalize" => builtin_ice_normalize(args),
        "insulation_score" => builtin_insulation_score(args),
        "tad_boundaries" => builtin_tad_boundaries(args),
        "distance_decay" => builtin_distance_decay(args),
        "expected_contacts" => builtin_expected_contacts(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown hic builtin '{name}'"),
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

/// Extract the contact matrix as Vec<Vec<f64>> from a square Table.
fn table_to_matrix(table: &Table) -> Vec<Vec<f64>> {
    table
        .rows
        .iter()
        .map(|row| row.iter().map(|v| to_f64(v)).collect())
        .collect()
}

fn matrix_to_table(mat: Vec<Vec<f64>>, col_names: Vec<String>) -> Value {
    let rows: Vec<Vec<Value>> = mat
        .into_iter()
        .map(|row| row.into_iter().map(Value::Float).collect())
        .collect();
    Value::Table(Table::new(col_names, rows))
}

// ── ice_normalize ─────────────────────────────────────────────────────

fn builtin_ice_normalize(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "ice_normalize")?;
    let col_names = table.columns.clone();
    let n = table.rows.len();

    if n == 0 {
        return Ok(Value::Table(table.clone()));
    }

    let mut mat: Vec<Vec<f64>> = table_to_matrix(table);

    // Identify zero rows/cols (masked)
    let row_sum_initial: Vec<f64> = mat.iter().map(|r| r.iter().sum()).collect();
    let masked: Vec<bool> = row_sum_initial.iter().map(|&s| s == 0.0).collect();

    for _ in 0..30 {
        // Row normalisation
        for i in 0..n {
            if masked[i] {
                continue;
            }
            let row_sum: f64 = mat[i].iter().sum();
            if row_sum > 0.0 {
                for j in 0..mat[i].len().min(n) {
                    mat[i][j] /= row_sum;
                }
            }
        }
        // Column normalisation
        for j in 0..n {
            if masked[j] {
                continue;
            }
            let col_sum: f64 = mat
                .iter()
                .map(|r| if j < r.len() { r[j] } else { 0.0 })
                .sum();
            if col_sum > 0.0 {
                for i in 0..n {
                    if j < mat[i].len() {
                        mat[i][j] /= col_sum;
                    }
                }
            }
        }
    }

    Ok(matrix_to_table(mat, col_names))
}

// ── insulation_score ─────────────────────────────────────────────────

fn builtin_insulation_score(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "insulation_score")?;
    let window = match &args[1] {
        Value::Int(n) => *n as usize,
        Value::Float(f) => *f as usize,
        _ => {
            return Err(BioLangError::type_error(
                "insulation_score() window must be Int",
                None,
            ))
        }
    };

    let mat = table_to_matrix(table);
    let n = mat.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }

    let mut diamond_sums: Vec<f64> = vec![0.0; n];

    for i in 0..n {
        // Diamond region: rows [i-w..i], cols [i..i+w]
        let row_start = if i >= window { i - window } else { 0 };
        let col_end = (i + window).min(n);
        // Only compute for bins with a full diamond
        if i < window || i + window >= n {
            diamond_sums[i] = 0.0;
            continue;
        }
        let mut s = 0.0;
        for r in row_start..i {
            if r < mat.len() {
                for c in i..col_end {
                    if c < mat[r].len() {
                        s += mat[r][c];
                    }
                }
            }
        }
        diamond_sums[i] = s;
    }

    // Mean of non-zero diamond sums
    let non_zero: Vec<f64> = diamond_sums.iter().filter(|&&v| v > 0.0).cloned().collect();
    let mean_diamond = if non_zero.is_empty() {
        1.0
    } else {
        non_zero.iter().sum::<f64>() / non_zero.len() as f64
    };

    let scores: Vec<Value> = diamond_sums
        .iter()
        .map(|&s| {
            if s == 0.0 {
                Value::Float(0.0)
            } else {
                Value::Float((s / mean_diamond + 1.0).log2())
            }
        })
        .collect();

    Ok(Value::List((scores).into()))
}

// ── tad_boundaries ───────────────────────────────────────────────────

fn builtin_tad_boundaries(args: Vec<Value>) -> Result<Value> {
    let scores: Vec<f64> = match &args[0] {
        Value::List(l) => l.iter().map(|v| to_f64(v)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "tad_boundaries() scores must be List",
                None,
            ))
        }
    };
    let min_delta = to_f64(&args[1]);

    let n = scores.len();
    let mut boundaries: Vec<Value> = Vec::new();

    for i in 1..(n.saturating_sub(1)) {
        let prev = scores[i - 1];
        let curr = scores[i];
        let next = scores[i + 1];
        if curr < prev && curr < next {
            let delta = (prev - curr) + (next - curr);
            if delta >= 2.0 * min_delta {
                boundaries.push(Value::Int(i as i64));
            }
        }
    }

    Ok(Value::List((boundaries).into()))
}

// ── distance_decay ───────────────────────────────────────────────────

fn builtin_distance_decay(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "distance_decay")?;
    let mat = table_to_matrix(table);
    let n = mat.len();

    let col_names = vec!["distance".to_string(), "mean_contact".to_string()];
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for d in 0..n {
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for i in 0..(n - d) {
            let j = i + d;
            if j < mat[i].len() {
                sum += mat[i][j];
                count += 1;
            }
        }
        let mean_contact = if count > 0 { sum / count as f64 } else { 0.0 };
        rows.push(vec![Value::Int(d as i64), Value::Float(mean_contact)]);
    }

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── expected_contacts ────────────────────────────────────────────────

fn builtin_expected_contacts(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "expected_contacts")?;
    let col_names = table.columns.clone();
    let mat = table_to_matrix(table);
    let n = mat.len();

    // Compute distance decay
    let mut decay: Vec<f64> = vec![0.0; n];
    for d in 0..n {
        let mut sum = 0.0f64;
        let mut count = 0usize;
        for i in 0..(n - d) {
            let j = i + d;
            if j < mat[i].len() {
                sum += mat[i][j];
                count += 1;
            }
        }
        decay[d] = if count > 0 { sum / count as f64 } else { 0.0 };
    }

    // Build expected matrix
    let expected: Vec<Vec<f64>> = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| {
                    let d = if i <= j { j - i } else { i - j };
                    decay[d]
                })
                .collect()
        })
        .collect();

    Ok(matrix_to_table(expected, col_names))
}
