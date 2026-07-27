//! Immune repertoire analysis builtins (TCR/BCR).
//!
//! Functions: parse_vdj, clonotype_diversity, clonal_expansion,
//! vj_usage, cdr3_length_dist, shared_clones.

use std::collections::HashMap;

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ──────────────────────────────────────────────────────────

pub fn immune_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("parse_vdj", Arity::Exact(1)),
        ("clonotype_diversity", Arity::Exact(1)),
        ("clonal_expansion", Arity::Range(1, 2)),
        ("vj_usage", Arity::Exact(1)),
        ("cdr3_length_dist", Arity::Exact(1)),
        ("shared_clones", Arity::Exact(1)),
    ]
}

pub fn is_immune_builtin(name: &str) -> bool {
    matches!(
        name,
        "parse_vdj"
            | "clonotype_diversity"
            | "clonal_expansion"
            | "vj_usage"
            | "cdr3_length_dist"
            | "shared_clones"
    )
}

pub fn call_immune_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "parse_vdj" => builtin_parse_vdj(args),
        "clonotype_diversity" => builtin_clonotype_diversity(args),
        "clonal_expansion" => builtin_clonal_expansion(args),
        "vj_usage" => builtin_vj_usage(args),
        "cdr3_length_dist" => builtin_cdr3_length_dist(args),
        "shared_clones" => builtin_shared_clones(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown immune builtin '{name}'"),
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

const KNOWN_COLS: &[&str] = &[
    "barcode",
    "raw_clonotype_id",
    "chain",
    "v_gene",
    "d_gene",
    "j_gene",
    "cdr3",
    "cdr3_nt",
    "reads",
    "umis",
];

// ── parse_vdj ─────────────────────────────────────────────────────────

fn builtin_parse_vdj(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "parse_vdj")?;

    let mut lines = text.lines();
    let header_line = match lines.next() {
        Some(l) => l,
        None => {
            return Ok(Value::Table(Table::new(
                vec!["error".to_string()],
                vec![vec![Value::Str("empty input".to_string())]],
            )))
        }
    };

    let sep = if header_line.contains('\t') { '\t' } else { ',' };
    let raw_headers: Vec<String> = header_line
        .split(sep)
        .map(|s| s.trim().to_string())
        .collect();

    // Find which known columns are present
    let keep_indices: Vec<(usize, String)> = raw_headers
        .iter()
        .enumerate()
        .filter(|(_, h)| KNOWN_COLS.contains(&h.as_str()))
        .map(|(i, h)| (i, h.clone()))
        .collect();

    if keep_indices.is_empty() {
        return Ok(Value::Table(Table::new(
            vec!["error".to_string()],
            vec![vec![Value::Str(
                "no recognized VDJ columns found".to_string(),
            )]],
        )));
    }

    let col_names: Vec<String> = keep_indices.iter().map(|(_, h)| h.clone()).collect();
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(sep).collect();
        let row: Vec<Value> = keep_indices
            .iter()
            .map(|(ci, _)| {
                let s = parts.get(*ci).copied().unwrap_or("").trim();
                // reads/umis as Int if numeric
                if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
                    Value::Int(s.parse::<i64>().unwrap_or(0))
                } else {
                    Value::Str(s.to_string())
                }
            })
            .collect();
        rows.push(row);
    }

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── clonotype_diversity ───────────────────────────────────────────────

fn builtin_clonotype_diversity(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "clonotype_diversity")?;

    let cdr3_col = table
        .columns
        .iter()
        .position(|c| c == "cdr3")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "clonotype_diversity(): column 'cdr3' not found".to_string(),
                None,
            )
        })?;

    let mut counts: HashMap<String, usize> = HashMap::new();
    for row in &table.rows {
        let cdr3 = match row.get(cdr3_col) {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => continue,
        };
        *counts.entry(cdr3).or_insert(0) += 1;
    }

    let total: usize = counts.values().sum();
    let richness = counts.len();

    if total == 0 {
        let mut rec = HashMap::new();
        rec.insert("shannon".to_string(), Value::Float(0.0));
        rec.insert("simpson".to_string(), Value::Float(0.0));
        rec.insert("chao1".to_string(), Value::Float(0.0));
        rec.insert("richness".to_string(), Value::Int(0));
        rec.insert("total_cells".to_string(), Value::Int(0));
        return Ok(Value::Record((rec).into()));
    }

    let n = total as f64;
    let shannon: f64 = counts
        .values()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.ln()
        })
        .sum();

    let simpson: f64 = 1.0
        - counts
            .values()
            .map(|&c| {
                let p = c as f64 / n;
                p * p
            })
            .sum::<f64>();

    let f1 = counts.values().filter(|&&c| c == 1).count() as f64;
    let f2 = counts.values().filter(|&&c| c == 2).count() as f64;
    let chao1 = if f2 == 0.0 {
        richness as f64 + f1 * (f1 - 1.0) / 2.0
    } else {
        richness as f64 + (f1 * f1) / (2.0 * f2)
    };

    let mut rec = HashMap::new();
    rec.insert("shannon".to_string(), Value::Float(shannon));
    rec.insert("simpson".to_string(), Value::Float(simpson));
    rec.insert("chao1".to_string(), Value::Float(chao1));
    rec.insert("richness".to_string(), Value::Int(richness as i64));
    rec.insert("total_cells".to_string(), Value::Int(total as i64));

    Ok(Value::Record((rec).into()))
}

// ── clonal_expansion ─────────────────────────────────────────────────

fn builtin_clonal_expansion(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "clonal_expansion")?;
    let threshold = if args.len() > 1 {
        to_f64(&args[1])
    } else {
        0.01
    };

    let cdr3_col = table
        .columns
        .iter()
        .position(|c| c == "cdr3")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "clonal_expansion(): column 'cdr3' not found".to_string(),
                None,
            )
        })?;

    let mut counts: HashMap<String, usize> = HashMap::new();
    for row in &table.rows {
        let cdr3 = match row.get(cdr3_col) {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => continue,
        };
        *counts.entry(cdr3).or_insert(0) += 1;
    }

    let total: usize = counts.values().sum();
    let n_total_clones = counts.len() as i64;

    if total == 0 {
        let mut rec = HashMap::new();
        rec.insert("n_expanded".to_string(), Value::Int(0));
        rec.insert("n_total_clones".to_string(), Value::Int(0));
        rec.insert("expanded_fraction".to_string(), Value::Float(0.0));
        rec.insert("top_clone_fraction".to_string(), Value::Float(0.0));
        return Ok(Value::Record((rec).into()));
    }

    let n = total as f64;
    let n_expanded = counts
        .values()
        .filter(|&&c| c as f64 / n > threshold)
        .count() as i64;

    let top_count = counts.values().max().copied().unwrap_or(0);
    let top_clone_fraction = top_count as f64 / n;
    let expanded_fraction = n_expanded as f64 / n_total_clones as f64;

    let mut rec = HashMap::new();
    rec.insert("n_expanded".to_string(), Value::Int(n_expanded));
    rec.insert("n_total_clones".to_string(), Value::Int(n_total_clones));
    rec.insert("expanded_fraction".to_string(), Value::Float(expanded_fraction));
    rec.insert("top_clone_fraction".to_string(), Value::Float(top_clone_fraction));

    Ok(Value::Record((rec).into()))
}

// ── vj_usage ─────────────────────────────────────────────────────────

fn builtin_vj_usage(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "vj_usage")?;

    // Try v_gene first, fall back to j_gene
    let gene_col = table
        .columns
        .iter()
        .position(|c| c == "v_gene")
        .or_else(|| table.columns.iter().position(|c| c == "j_gene"))
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "vj_usage(): neither 'v_gene' nor 'j_gene' column found".to_string(),
                None,
            )
        })?;

    let mut counts: HashMap<String, usize> = HashMap::new();
    for row in &table.rows {
        let gene = match row.get(gene_col) {
            Some(Value::Str(s)) if !s.is_empty() => s.clone(),
            _ => continue,
        };
        *counts.entry(gene).or_insert(0) += 1;
    }

    let total: usize = counts.values().sum();
    let n = total as f64;

    let col_names = vec![
        "gene".to_string(),
        "count".to_string(),
        "fraction".to_string(),
    ];

    let mut gene_rows: Vec<Vec<Value>> = counts
        .iter()
        .map(|(g, &c)| {
            vec![
                Value::Str(g.clone()),
                Value::Int(c as i64),
                Value::Float(if n > 0.0 { c as f64 / n } else { 0.0 }),
            ]
        })
        .collect();

    gene_rows.sort_by(|a, b| {
        let ca = match &a[1] { Value::Int(n) => *n, _ => 0 };
        let cb = match &b[1] { Value::Int(n) => *n, _ => 0 };
        cb.cmp(&ca)
    });

    Ok(Value::Table(Table::new(col_names, gene_rows)))
}

// ── cdr3_length_dist ─────────────────────────────────────────────────

fn builtin_cdr3_length_dist(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "cdr3_length_dist")?;

    let cdr3_col = table
        .columns
        .iter()
        .position(|c| c == "cdr3")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "cdr3_length_dist(): column 'cdr3' not found".to_string(),
                None,
            )
        })?;

    let mut counts: HashMap<usize, usize> = HashMap::new();
    for row in &table.rows {
        let len = match row.get(cdr3_col) {
            Some(Value::Str(s)) if !s.is_empty() => s.chars().count(),
            _ => continue,
        };
        *counts.entry(len).or_insert(0) += 1;
    }

    let total: usize = counts.values().sum();
    let n = total as f64;

    let mut lengths: Vec<usize> = counts.keys().copied().collect();
    lengths.sort_unstable();

    let col_names = vec![
        "length".to_string(),
        "count".to_string(),
        "fraction".to_string(),
    ];

    let rows: Vec<Vec<Value>> = lengths
        .iter()
        .map(|&l| {
            let c = counts[&l];
            vec![
                Value::Int(l as i64),
                Value::Int(c as i64),
                Value::Float(if n > 0.0 { c as f64 / n } else { 0.0 }),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(col_names, rows)))
}

// ── shared_clones ─────────────────────────────────────────────────────

fn builtin_shared_clones(args: Vec<Value>) -> Result<Value> {
    let tables = match &args[0] {
        Value::List(l) => l.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "shared_clones() requires List[Table]",
                None,
            ))
        }
    };

    let mut cdr3_sample_count: HashMap<String, usize> = HashMap::new();

    for val in tables.iter() {
        let table = require_table(val, "shared_clones")?;
        let cdr3_col = match table.columns.iter().position(|c| c == "cdr3") {
            Some(idx) => idx,
            None => continue,
        };

        // Collect unique CDR3s in this table
        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        for row in &table.rows {
            if let Some(Value::Str(s)) = row.get(cdr3_col) {
                if !s.is_empty() {
                    seen.insert(s.clone());
                }
            }
        }
        for cdr3 in seen {
            *cdr3_sample_count.entry(cdr3).or_insert(0) += 1;
        }
    }

    let col_names = vec!["cdr3".to_string(), "n_samples".to_string()];

    let mut rows: Vec<Vec<Value>> = cdr3_sample_count
        .iter()
        .filter(|(_, &n)| n >= 2)
        .map(|(cdr3, &n)| vec![Value::Str(cdr3.clone()), Value::Int(n as i64)])
        .collect();

    rows.sort_by(|a, b| {
        let na = match &a[1] { Value::Int(n) => *n, _ => 0 };
        let nb = match &b[1] { Value::Int(n) => *n, _ => 0 };
        nb.cmp(&na)
    });

    Ok(Value::Table(Table::new(col_names, rows)))
}
