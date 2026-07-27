//! VCF / variant analysis builtins.
//!
//! Functions: vcf_parse, vcf_filter, titv_ratio, variant_summary, allele_freq.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ─────────────────────────────────────────────────────────

pub fn variants_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("vcf_parse", Arity::Exact(1)),
        ("vcf_filter", Arity::Range(1, 3)),
        ("titv_ratio", Arity::Exact(1)),
        ("variant_summary", Arity::Exact(1)),
        ("allele_freq", Arity::Range(1, 2)),
    ]
}

pub fn is_variants_builtin(name: &str) -> bool {
    matches!(
        name,
        "vcf_parse" | "vcf_filter" | "titv_ratio" | "variant_summary" | "allele_freq"
    )
}

pub fn call_variants_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "vcf_parse" => builtin_vcf_parse(args),
        "vcf_filter" => builtin_vcf_filter(args),
        "titv_ratio" => builtin_titv_ratio(args),
        "variant_summary" => builtin_variant_summary(args),
        "allele_freq" => builtin_allele_freq(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown variants builtin '{name}'"),
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

fn col_index(table: &Table, name: &str, func: &str) -> Result<usize> {
    table
        .columns
        .iter()
        .position(|c| c == name)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                format!("{func}(): column '{name}' not found"),
                None,
            )
        })
}

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => 0.0,
    }
}

fn is_transition(r: &str, a: &str) -> bool {
    matches!(
        (r, a),
        ("A", "G") | ("G", "A") | ("C", "T") | ("T", "C")
    )
}

fn is_transversion(r: &str, a: &str) -> bool {
    matches!(
        (r, a),
        ("A", "C") | ("C", "A") | ("A", "T") | ("T", "A") | ("G", "C") | ("C", "G") | ("G", "T") | ("T", "G")
    )
}

// ── vcf_parse ────────────────────────────────────────────────────────

fn builtin_vcf_parse(args: Vec<Value>) -> Result<Value> {
    let text = require_str(&args[0], "vcf_parse")?;
    let columns = ["chrom", "pos", "id", "ref_", "alt", "qual", "filter", "info"]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.splitn(9, '\t').collect();
        if fields.len() < 8 {
            continue;
        }
        let pos: i64 = fields[1].parse().unwrap_or(0);
        let qual: f64 = if fields[5] == "." {
            0.0
        } else {
            fields[5].parse().unwrap_or(0.0)
        };
        rows.push(vec![
            Value::Str(fields[0].to_string()),
            Value::Int(pos),
            Value::Str(fields[2].to_string()),
            Value::Str(fields[3].to_string()),
            Value::Str(fields[4].to_string()),
            Value::Float(qual),
            Value::Str(fields[6].to_string()),
            Value::Str(fields[7].to_string()),
        ]);
    }
    Ok(Value::Table(Table::new(columns, rows)))
}

// ── vcf_filter ───────────────────────────────────────────────────────

fn builtin_vcf_filter(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "vcf_filter")?.clone();
    let min_qual = if args.len() > 1 {
        match &args[1] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => 0.0,
        }
    } else {
        0.0
    };
    let require_pass = if args.len() > 2 {
        match &args[2] {
            Value::Bool(b) => *b,
            _ => false,
        }
    } else {
        false
    };

    if args.len() == 1 {
        return Ok(Value::Table(table));
    }

    let qual_col = col_index(&table, "qual", "vcf_filter")?;
    let filter_col = col_index(&table, "filter", "vcf_filter")?;

    let rows = table
        .rows
        .into_iter()
        .filter(|row| {
            let q = to_f64(&row[qual_col]);
            if q < min_qual {
                return false;
            }
            if require_pass {
                match &row[filter_col] {
                    Value::Str(s) => s == "PASS",
                    _ => false,
                }
            } else {
                true
            }
        })
        .collect();

    Ok(Value::Table(Table::new(table.columns, rows)))
}

// ── titv_ratio ───────────────────────────────────────────────────────

fn builtin_titv_ratio(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "titv_ratio")?;
    let ref_col = col_index(table, "ref_", "titv_ratio")?;
    let alt_col = col_index(table, "alt", "titv_ratio")?;

    let mut ti = 0u64;
    let mut tv = 0u64;

    for row in &table.rows {
        let r = match &row[ref_col] {
            Value::Str(s) => s.as_str(),
            _ => continue,
        };
        let a = match &row[alt_col] {
            Value::Str(s) => s.as_str(),
            _ => continue,
        };
        if r.len() != 1 || a.len() != 1 {
            continue;
        }
        let ru = r.to_uppercase();
        let au = a.to_uppercase();
        if is_transition(&ru, &au) {
            ti += 1;
        } else if is_transversion(&ru, &au) {
            tv += 1;
        }
    }

    Ok(Value::Float(if tv == 0 {
        0.0
    } else {
        ti as f64 / tv as f64
    }))
}

// ── variant_summary ──────────────────────────────────────────────────

fn builtin_variant_summary(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "variant_summary")?;
    let ref_col = col_index(table, "ref_", "variant_summary")?;
    let alt_col = col_index(table, "alt", "variant_summary")?;

    let mut counts = std::collections::HashMap::<&'static str, i64>::new();

    for row in &table.rows {
        let r = match &row[ref_col] {
            Value::Str(s) => s.as_str(),
            _ => continue,
        };
        let a = match &row[alt_col] {
            Value::Str(s) => s.as_str(),
            _ => continue,
        };
        let kind = if r.len() == 1 && a.len() == 1 {
            "SNP"
        } else if a.len() > r.len() {
            "insertion"
        } else if r.len() > a.len() {
            "deletion"
        } else {
            "MNV"
        };
        *counts.entry(kind).or_insert(0) += 1;
    }

    let mut rows = Vec::new();
    for kind in &["SNP", "insertion", "deletion", "MNV"] {
        if let Some(&cnt) = counts.get(kind) {
            rows.push(vec![Value::Str(kind.to_string()), Value::Int(cnt)]);
        }
    }

    Ok(Value::Table(Table::new(
        vec!["type_".to_string(), "count".to_string()],
        rows,
    )))
}

// ── allele_freq ──────────────────────────────────────────────────────

fn builtin_allele_freq(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "allele_freq")?;
    let field = if args.len() > 1 {
        match &args[1] {
            Value::Str(s) => s.clone(),
            _ => "AF".to_string(),
        }
    } else {
        "AF".to_string()
    };

    let info_col = col_index(table, "info", "allele_freq")?;
    let prefix = format!("{}=", field);

    let freqs = table
        .rows
        .iter()
        .map(|row| {
            let info = match &row[info_col] {
                Value::Str(s) => s.as_str(),
                _ => return Value::Float(0.0),
            };
            for part in info.split(';') {
                if part.starts_with(&prefix) {
                    let val_str = &part[prefix.len()..];
                    // Handle multi-allelic AF=0.1,0.2 — take first
                    let first = val_str.split(',').next().unwrap_or("0");
                    if let Ok(f) = first.parse::<f64>() {
                        return Value::Float(f);
                    }
                }
            }
            Value::Float(0.0)
        })
        .collect::<Vec<_>>().into();

    Ok(Value::List(freqs))
}
