//! Bulk RNA-seq analysis builtins.
//!
//! Functions: parse_salmon, parse_featurecounts, size_factors,
//! filter_low_counts, tpm_matrix, sample_correlation.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ─────────────────────────────────────────────────────────

pub fn rnaseq_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("parse_salmon", Arity::Exact(1)),
        ("parse_featurecounts", Arity::Exact(1)),
        ("size_factors", Arity::Exact(1)),
        ("filter_low_counts", Arity::Range(1, 3)),
        ("tpm_matrix", Arity::Exact(2)),
        ("sample_correlation", Arity::Exact(1)),
    ]
}

pub fn is_rnaseq_builtin(name: &str) -> bool {
    matches!(
        name,
        "parse_salmon"
            | "parse_featurecounts"
            | "size_factors"
            | "filter_low_counts"
            | "tpm_matrix"
            | "sample_correlation"
    )
}

pub fn call_rnaseq_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "parse_salmon" => builtin_parse_salmon(args),
        "parse_featurecounts" => builtin_parse_featurecounts(args),
        "size_factors" => builtin_size_factors(args),
        "filter_low_counts" => builtin_filter_low_counts(args),
        "tpm_matrix" => builtin_tpm_matrix(args),
        "sample_correlation" => builtin_sample_correlation(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown rnaseq builtin '{name}'"),
            None,
        )),
    }
}

// ── Helpers ──────────────────────────────────────────────────────────

fn require_str<'a>(val: &'a Value, func: &str) -> Result<&'a str> {
    match val {
        Value::Str(s) => Ok(s.as_str()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Str"),
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

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => 0.0,
    }
}

fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let n = a.len().min(b.len()) as f64;
    if n == 0.0 {
        return 0.0;
    }
    let mean_a = a.iter().sum::<f64>() / n;
    let mean_b = b.iter().sum::<f64>() / n;
    let num: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (x - mean_a) * (y - mean_b))
        .sum();
    let da: f64 = a.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>().sqrt();
    let db: f64 = b.iter().map(|y| (y - mean_b).powi(2)).sum::<f64>().sqrt();
    if da == 0.0 || db == 0.0 {
        0.0
    } else {
        num / (da * db)
    }
}

fn median(vals: &mut Vec<f64>) -> f64 {
    if vals.is_empty() {
        return 1.0;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = vals.len();
    if n.is_multiple_of(2) {
        (vals[n / 2 - 1] + vals[n / 2]) / 2.0
    } else {
        vals[n / 2]
    }
}

// ── parse_salmon ─────────────────────────────────────────────────────

fn builtin_parse_salmon(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "parse_salmon")?;
    let columns = ["name", "length", "eff_length", "tpm", "num_reads"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut rows = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("Name") || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.splitn(5, '\t').collect();
        if f.len() < 5 {
            continue;
        }
        rows.push(vec![
            Value::Str(f[0].to_string()),
            Value::Int(f[1].parse().unwrap_or(0)),
            Value::Float(f[2].parse().unwrap_or(0.0)),
            Value::Float(f[3].parse().unwrap_or(0.0)),
            Value::Float(f[4].parse().unwrap_or(0.0)),
        ]);
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

// ── parse_featurecounts ──────────────────────────────────────────────

fn builtin_parse_featurecounts(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "parse_featurecounts")?;
    let mut header_line: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if header_line.is_none() {
            // First non-comment line is header: Geneid Chr Start End Strand Length sample...
            header_line = Some(fields.iter().map(|s| s.to_string()).collect());
            continue;
        }
        if fields.len() < 7 {
            continue;
        }
        // gene_id is field[0], counts start at field[6]
        let mut row = vec![Value::Str(fields[0].to_string())];
        for &c in &fields[6..] {
            row.push(Value::Int(c.parse().unwrap_or(0)));
        }
        rows.push(row);
    }

    let columns = if let Some(ref h) = header_line {
        let mut cols = vec!["gene_id".to_string()];
        for s in h.iter().skip(6) {
            cols.push(s.clone());
        }
        cols
    } else {
        vec!["gene_id".to_string()]
    };

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── size_factors ─────────────────────────────────────────────────────

fn builtin_size_factors(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "size_factors")?;
    if table.rows.is_empty() || table.columns.is_empty() {
        return Ok(Value::List((vec![]).into()));
    }

    let n_genes = table.rows.len();
    let n_samples = table.columns.len();

    // Build genes × samples f64 matrix
    let mat: Vec<Vec<f64>> = table
        .rows
        .iter()
        .map(|row| row.iter().map(to_f64).collect())
        .collect();

    // For each gene, compute geometric mean across samples (skip if any zero)
    let geomeans: Vec<Option<f64>> = (0..n_genes)
        .map(|g| {
            let vals: Vec<f64> = (0..n_samples).map(|s| mat[g][s]).collect();
            if vals.iter().any(|&v| v <= 0.0) {
                None
            } else {
                let log_sum: f64 = vals.iter().map(|v| v.ln()).sum();
                Some((log_sum / n_samples as f64).exp())
            }
        })
        .collect();

    // For each sample, compute ratios then take median
    let factors: Vec<Value> = (0..n_samples)
        .map(|s| {
            let mut ratios: Vec<f64> = (0..n_genes)
                .filter_map(|g| geomeans[g].map(|gm| mat[g][s] / gm))
                .collect();
            Value::Float(median(&mut ratios))
        })
        .collect();

    Ok(Value::List((factors).into()))
}

// ── filter_low_counts ────────────────────────────────────────────────

fn builtin_filter_low_counts(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "filter_low_counts")?.clone();
    let min_count = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            _ => 10,
        }
    } else {
        10
    };
    let min_samples = if args.len() > 2 {
        match &args[2] {
            Value::Int(n) => *n as usize,
            _ => 2,
        }
    } else {
        2
    };

    let rows = table
        .rows
        .into_iter()
        .filter(|row| {
            let passing = row.iter().filter(|v| to_f64(v) >= min_count as f64).count();
            passing >= min_samples
        })
        .collect();

    Ok(Value::Table(Table::new(table.columns, rows)))
}

// ── tpm_matrix ───────────────────────────────────────────────────────

fn builtin_tpm_matrix(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "tpm_matrix")?;
    let lengths = match &args[1] {
        Value::List(l) => l.iter().map(|v| to_f64(v).max(1.0)).collect::<Vec<_>>(),
        _ => {
            return Err(BioLangError::type_error(
                "tpm_matrix() lengths must be List",
                None,
            ))
        }
    };

    if table.rows.len() != lengths.len() {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "tpm_matrix(): {} genes but {} lengths",
                table.rows.len(),
                lengths.len()
            ),
            None,
        ));
    }

    let n_samples = table.columns.len();
    let n_genes = table.rows.len();

    // Build RPK matrix: genes × samples
    let rpk: Vec<Vec<f64>> = (0..n_genes)
        .map(|g| {
            let len_kb = lengths[g] / 1000.0;
            (0..n_samples)
                .map(|s| to_f64(&table.rows[g][s]) / len_kb)
                .collect()
        })
        .collect();

    // Per sample sum of RPK, then scale to TPM
    let tpm_rows: Vec<Vec<Value>> = {
        let col_sums: Vec<f64> = (0..n_samples)
            .map(|s| (0..n_genes).map(|g| rpk[g][s]).sum::<f64>())
            .collect();

        (0..n_genes)
            .map(|g| {
                (0..n_samples)
                    .map(|s| {
                        Value::Float(if col_sums[s] == 0.0 {
                            0.0
                        } else {
                            rpk[g][s] / col_sums[s] * 1_000_000.0
                        })
                    })
                    .collect()
            })
            .collect()
    };

    Ok(Value::Table(Table::new(table.columns.clone(), tpm_rows)))
}

// ── sample_correlation ───────────────────────────────────────────────

fn builtin_sample_correlation(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "sample_correlation")?;
    let n_samples = table.columns.len();
    let n_genes = table.rows.len();

    // Build columns: genes per sample, log1p-transformed
    let cols: Vec<Vec<f64>> = (0..n_samples)
        .map(|s| {
            (0..n_genes)
                .map(|g| (to_f64(&table.rows[g][s]) + 1.0).ln())
                .collect()
        })
        .collect();

    let col_names: Vec<String> = (0..n_samples).map(|i| format!("sample_{i}")).collect();
    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..n_samples {
        let row: Vec<Value> = (0..n_samples)
            .map(|j| Value::Float(pearson(&cols[i], &cols[j])))
            .collect();
        out_rows.push(row);
    }

    Ok(Value::Table(Table::new(col_names, out_rows)))
}
