//! LC-MS metabolomics builtins.
//!
//! Functions: mz_match, isotope_correct, feature_group, log_transform,
//! pathway_enrichment, normalize_samples.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::{HashMap, HashSet};

// ── Registry ─────────────────────────────────────────────────────────

pub fn metabolomics_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("mz_match", Arity::Range(2, 3)),
        ("isotope_correct", Arity::Exact(2)),
        ("feature_group", Arity::Range(1, 3)),
        ("log_transform", Arity::Exact(1)),
        ("pathway_enrichment", Arity::Exact(2)),
        ("normalize_samples", Arity::Range(1, 2)),
    ]
}

pub fn is_metabolomics_builtin(name: &str) -> bool {
    matches!(
        name,
        "mz_match"
            | "isotope_correct"
            | "feature_group"
            | "log_transform"
            | "pathway_enrichment"
            | "normalize_samples"
    )
}

pub fn call_metabolomics_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "mz_match" => builtin_mz_match(args),
        "isotope_correct" => builtin_isotope_correct(args),
        "feature_group" => builtin_feature_group(args),
        "log_transform" => builtin_log_transform(args),
        "pathway_enrichment" => builtin_pathway_enrichment(args),
        "normalize_samples" => builtin_normalize_samples(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown metabolomics builtin '{name}'"),
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

fn is_numeric(v: &Value) -> bool {
    matches!(v, Value::Float(_) | Value::Int(_))
}

fn col_index(table: &Table, name: &str, func: &str) -> Result<usize> {
    table.columns.iter().position(|c| c == name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::NameError,
            format!("{func}(): column '{name}' not found"),
            None,
        )
    })
}

/// Log-factorial for Fisher exact p-value.
fn log_factorial(n: usize) -> f64 {
    (1..=n).map(|i| (i as f64).ln()).sum()
}

fn fisher_log_prob(a: usize, b: usize, c: usize, d: usize) -> f64 {
    let n = a + b + c + d;
    log_factorial(a + b) + log_factorial(c + d) + log_factorial(a + c) + log_factorial(b + d)
        - log_factorial(n)
        - log_factorial(a)
        - log_factorial(b)
        - log_factorial(c)
        - log_factorial(d)
}

fn fisher_exact_pvalue(a: usize, b: usize, c: usize, d: usize) -> f64 {
    let observed_lp = fisher_log_prob(a, b, c, d);
    let n1 = a + b;
    let n2 = c + d;
    let k = a + c;
    let n = a + b + c + d;
    let lo = k.saturating_sub(n2);
    let hi = k.min(n1);
    let mut p = 0.0f64;
    for x in lo..=hi {
        let lp = fisher_log_prob(x, n1 - x, k - x, n2 - (k - x));
        if lp <= observed_lp + 1e-10 {
            p += lp.exp();
        }
    }
    p.min(1.0)
}

// ── mz_match ─────────────────────────────────────────────────────────

fn builtin_mz_match(args: Vec<Value>) -> Result<Value> {
    let observed_mz = match &args[0] {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => {
            return Err(BioLangError::type_error(
                "mz_match() observed_mz must be Float",
                None,
            ))
        }
    };
    let db = require_table(&args[1], "mz_match")?;
    let ppm_tol = if args.len() > 2 {
        to_f64(&args[2])
    } else {
        5.0
    };

    let mz_col = col_index(db, "exact_mz", "mz_match")?;

    let mut out_columns = db.columns.clone();
    out_columns.push("ppm_error".to_string());

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for row in &db.rows {
        let ref_mz = to_f64(&row[mz_col]);
        if ref_mz == 0.0 {
            continue;
        }
        let ppm = (observed_mz - ref_mz).abs() / ref_mz * 1e6;
        if ppm <= ppm_tol {
            let mut new_row = row.clone();
            new_row.push(Value::Float(ppm));
            out_rows.push(new_row);
        }
    }

    Ok(Value::Table(Table::new(out_columns, out_rows)))
}

// ── isotope_correct ───────────────────────────────────────────────────

fn builtin_isotope_correct(args: Vec<Value>) -> Result<Value> {
    let intensities: Vec<f64> = match &args[0] {
        Value::List(l) => l.iter().map(|v| to_f64(v)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "isotope_correct() intensities must be List",
                None,
            ))
        }
    };
    let n_carbons = match &args[1] {
        Value::Int(n) => *n as usize,
        Value::Float(f) => *f as usize,
        _ => {
            return Err(BioLangError::type_error(
                "isotope_correct() n_carbons must be Int",
                None,
            ))
        }
    };

    let m = intensities.len();
    if m == 0 {
        return Ok(Value::List((Vec::new()).into()));
    }

    // Natural abundance of ¹³C
    let p13 = 0.01109f64;
    let p12 = 1.0 - p13;

    // Build lower-triangular correction matrix C[i][j] for i >= j
    // C[i][j] = C(n-j, i-j) * p13^(i-j) * p12^(n-i)
    let binom = |n: usize, k: usize| -> f64 {
        if k > n {
            return 0.0;
        }
        let mut result = 1.0f64;
        for i in 0..k {
            result *= (n - i) as f64 / (i + 1) as f64;
        }
        result
    };

    let n = n_carbons;
    let mut c = vec![vec![0.0f64; m]; m];
    for i in 0..m {
        for j in 0..=i {
            let diff = i - j;
            if diff <= n && n >= i {
                c[i][j] = binom(n - j, diff) * p13.powi(diff as i32) * p12.powi((n - i) as i32);
            }
        }
    }

    // Solve lower-triangular system C * x = b via forward substitution
    let mut corrected = vec![0.0f64; m];
    for i in 0..m {
        let mut s = intensities[i];
        for j in 0..i {
            s -= c[i][j] * corrected[j];
        }
        let diag = c[i][i];
        corrected[i] = if diag.abs() > 1e-12 { s / diag } else { 0.0 };
        corrected[i] = corrected[i].max(0.0);
    }

    Ok(Value::List(
        corrected
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    ))
}

// ── feature_group ─────────────────────────────────────────────────────

fn builtin_feature_group(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "feature_group")?;
    let mz_tol_ppm = if args.len() > 1 {
        to_f64(&args[1])
    } else {
        5.0
    };
    let rt_tol = if args.len() > 2 {
        to_f64(&args[2])
    } else {
        0.1
    };

    let mz_col = col_index(table, "mz", "feature_group")?;
    let rt_col = col_index(table, "rt", "feature_group")?;

    let n = table.rows.len();
    let mzs: Vec<f64> = table.rows.iter().map(|r| to_f64(&r[mz_col])).collect();
    let rts: Vec<f64> = table.rows.iter().map(|r| to_f64(&r[rt_col])).collect();

    // Union-Find
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(parent: &mut Vec<usize>, i: usize) -> usize {
        if parent[i] != i {
            parent[i] = find(parent, parent[i]);
        }
        parent[i]
    }

    for i in 0..n {
        for j in (i + 1)..n {
            let mz_ok = if mzs[i] > 0.0 {
                (mzs[i] - mzs[j]).abs() / mzs[i] * 1e6 <= mz_tol_ppm
            } else {
                false
            };
            let rt_ok = (rts[i] - rts[j]).abs() <= rt_tol;
            if mz_ok && rt_ok {
                let pi = find(&mut parent, i);
                let pj = find(&mut parent, j);
                if pi != pj {
                    parent[pi] = pj;
                }
            }
        }
    }

    // Compress roots to sequential group IDs
    let mut root_to_id: HashMap<usize, i64> = HashMap::new();
    let mut next_id = 0i64;
    let group_ids: Vec<i64> = (0..n)
        .map(|i| {
            let root = find(&mut parent, i);
            *root_to_id.entry(root).or_insert_with(|| {
                let id = next_id;
                next_id += 1;
                id
            })
        })
        .collect();

    let mut out_columns = table.columns.clone();
    out_columns.push("group_id".to_string());

    let out_rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .zip(group_ids.iter())
        .map(|(row, &gid)| {
            let mut new_row = row.clone();
            new_row.push(Value::Int(gid));
            new_row
        })
        .collect();

    Ok(Value::Table(Table::new(out_columns, out_rows)))
}

// ── log_transform ─────────────────────────────────────────────────────

fn builtin_log_transform(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "log_transform")?.clone();

    let out_rows: Vec<Vec<Value>> = table
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| {
                    if is_numeric(v) {
                        let x = to_f64(v);
                        Value::Float(if x <= 0.0 { 0.0 } else { (x + 1.0).log2() })
                    } else {
                        v.clone()
                    }
                })
                .collect()
        })
        .collect();

    Ok(Value::Table(Table::new(table.columns, out_rows)))
}

// ── pathway_enrichment ────────────────────────────────────────────────

fn builtin_pathway_enrichment(args: Vec<Value>) -> Result<Value> {
    let metabolite_list: HashSet<String> = match &args[0] {
        Value::List(l) => l
            .iter()
            .filter_map(|v| {
                if let Value::Str(s) = v {
                    Some(s.clone())
                } else {
                    None
                }
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "pathway_enrichment() metabolite_list must be List",
                None,
            ))
        }
    };

    let pathway_db = require_table(&args[1], "pathway_enrichment")?;
    let pw_col = col_index(pathway_db, "pathway", "pathway_enrichment")?;
    let met_col = col_index(pathway_db, "metabolite", "pathway_enrichment")?;

    // Build pathway → set of metabolites
    let mut pw_map: HashMap<String, HashSet<String>> = HashMap::new();
    let mut all_metabolites: HashSet<String> = HashSet::new();
    for row in &pathway_db.rows {
        let pw = match &row[pw_col] {
            Value::Str(s) => s.clone(),
            _ => continue,
        };
        let met = match &row[met_col] {
            Value::Str(s) => s.clone(),
            _ => continue,
        };
        all_metabolites.insert(met.clone());
        pw_map.entry(pw).or_default().insert(met);
    }

    let total_bg = all_metabolites.len();
    let total_hits = metabolite_list.len();

    let mut result_rows: Vec<(String, usize, usize, f64)> = pw_map
        .iter()
        .map(|(pw, pw_mets)| {
            let a = pw_mets.intersection(&metabolite_list).count(); // hits in pathway
            let b = total_hits - a; // hits not in pathway
            let c = pw_mets.len() - a; // non-hits in pathway
            let d = total_bg - pw_mets.len() - b; // non-hits not in pathway
            let p = fisher_exact_pvalue(a, b, c, d);
            (pw.clone(), pw_mets.len(), a, p)
        })
        .collect();

    result_rows.sort_by(|x, y| x.3.partial_cmp(&y.3).unwrap_or(std::cmp::Ordering::Equal));

    let out_columns = vec![
        "pathway".to_string(),
        "n_pathway".to_string(),
        "n_hits".to_string(),
        "p_value".to_string(),
    ];
    let out_rows: Vec<Vec<Value>> = result_rows
        .into_iter()
        .map(|(pw, n_pw, n_hits, p)| {
            vec![
                Value::Str(pw),
                Value::Int(n_pw as i64),
                Value::Int(n_hits as i64),
                Value::Float(p),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(out_columns, out_rows)))
}

// ── normalize_samples ─────────────────────────────────────────────────

fn builtin_normalize_samples(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "normalize_samples")?.clone();
    let method = if args.len() > 1 {
        match &args[1] {
            Value::Str(s) => s.clone(),
            _ => "median".to_string(),
        }
    } else {
        "median".to_string()
    };

    let n_rows = table.rows.len();
    let n_cols = table.columns.len();

    match method.as_str() {
        "median" | "sum" => {
            // Operate per column
            let mut factors = vec![1.0f64; n_cols];
            for col in 0..n_cols {
                let vals: Vec<f64> = (0..n_rows)
                    .filter_map(|r| {
                        let v = to_f64(&table.rows[r][col]);
                        if v > 0.0 {
                            Some(v)
                        } else {
                            None
                        }
                    })
                    .collect();
                if vals.is_empty() {
                    factors[col] = 1.0;
                } else if method == "sum" {
                    factors[col] = vals.iter().sum::<f64>();
                } else {
                    // median
                    let mut sorted = vals.clone();
                    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                    let mid = sorted.len() / 2;
                    factors[col] = if sorted.len() % 2 == 0 {
                        (sorted[mid - 1] + sorted[mid]) / 2.0
                    } else {
                        sorted[mid]
                    };
                }
                if factors[col] == 0.0 {
                    factors[col] = 1.0;
                }
            }

            let out_rows: Vec<Vec<Value>> = table
                .rows
                .iter()
                .map(|row| {
                    row.iter()
                        .enumerate()
                        .map(|(c, v)| {
                            if is_numeric(v) {
                                Value::Float(to_f64(v) / factors[c])
                            } else {
                                v.clone()
                            }
                        })
                        .collect()
                })
                .collect();

            Ok(Value::Table(Table::new(table.columns, out_rows)))
        }
        "quantile" => {
            // Quantile normalization: rank-based per-column normalization
            // 1. For each column, rank values and sort
            // 2. Compute per-rank mean across columns
            // 3. Replace each value with the mean for its rank

            if n_rows == 0 || n_cols == 0 {
                return Ok(Value::Table(table));
            }

            // Extract numeric columns only (all columns assumed numeric here)
            let mut col_data: Vec<Vec<f64>> = (0..n_cols)
                .map(|c| (0..n_rows).map(|r| to_f64(&table.rows[r][c])).collect())
                .collect();

            // For each column: argsort
            let argsorts: Vec<Vec<usize>> = col_data
                .iter()
                .map(|col| {
                    let mut idx: Vec<usize> = (0..n_rows).collect();
                    idx.sort_by(|&a, &b| col[a].partial_cmp(&col[b]).unwrap());
                    idx
                })
                .collect();

            // Compute per-rank means
            let rank_means: Vec<f64> = (0..n_rows)
                .map(|rank| {
                    let sum: f64 = (0..n_cols).map(|c| col_data[c][argsorts[c][rank]]).sum();
                    sum / n_cols as f64
                })
                .collect();

            // Re-assign values
            for (c, argsort) in argsorts.iter().enumerate() {
                // rank_of[original_idx] = rank
                let mut rank_of = vec![0usize; n_rows];
                for (rank, &orig) in argsort.iter().enumerate() {
                    rank_of[orig] = rank;
                }
                for r in 0..n_rows {
                    col_data[c][r] = rank_means[rank_of[r]];
                }
            }

            let out_rows: Vec<Vec<Value>> = (0..n_rows)
                .map(|r| (0..n_cols).map(|c| Value::Float(col_data[c][r])).collect())
                .collect();

            Ok(Value::Table(Table::new(table.columns, out_rows)))
        }
        other => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("normalize_samples(): unknown method '{other}'; use median|sum|quantile"),
            None,
        )),
    }
}
