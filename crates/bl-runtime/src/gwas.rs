//! GWAS summary-statistics builtins.
//!
//! Functions: parse_sumstats, manhattan_data, qq_data, clump, top_loci, lambda_gc.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

// ── Registry ─────────────────────────────────────────────────────────

pub fn gwas_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("parse_sumstats", Arity::Exact(1)),
        ("manhattan_data", Arity::Exact(1)),
        ("qq_data", Arity::Exact(1)),
        ("clump", Arity::Range(1, 3)),
        ("top_loci", Arity::Range(1, 2)),
        ("lambda_gc", Arity::Exact(1)),
    ]
}

pub fn is_gwas_builtin(name: &str) -> bool {
    matches!(
        name,
        "parse_sumstats" | "manhattan_data" | "qq_data" | "clump" | "top_loci" | "lambda_gc"
    )
}

pub fn call_gwas_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "parse_sumstats" => builtin_parse_sumstats(args),
        "manhattan_data" => builtin_manhattan_data(args),
        "qq_data" => builtin_qq_data(args),
        "clump" => builtin_clump(args),
        "top_loci" => builtin_top_loci(args),
        "lambda_gc" => builtin_lambda_gc(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown gwas builtin '{name}'"),
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

fn require_list<'a>(val: &'a Value, func: &str) -> Result<&'a Vec<Value>> {
    match val {
        Value::List(l) => Ok(l),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List"),
            None,
        )),
    }
}

fn to_f64(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => f64::NAN,
    }
}

fn col_idx(t: &Table, name: &str) -> Option<usize> {
    t.columns.iter().position(|c| c == name)
}

fn require_col(t: &Table, name: &str, func: &str) -> Result<usize> {
    col_idx(t, name)
        .ok_or_else(|| BioLangError::type_error(format!("{func}() requires column '{name}'"), None))
}

/// Inverse normal CDF approximation (Beasley-Springer-Moro rational approximation).
fn inv_normal(p: f64) -> f64 {
    let p = p.clamp(1e-300, 1.0 - 1e-15);
    let q = if p < 0.5 { p } else { 1.0 - p };
    let r = (-2.0 * q.ln()).sqrt();
    let c0 = 2.515517;
    let c1 = 0.802853;
    let c2 = 0.010328;
    let d1 = 1.432788;
    let d2 = 0.189269;
    let d3 = 0.001308;
    let num = c0 + c1 * r + c2 * r * r;
    let den = 1.0 + d1 * r + d2 * r * r + d3 * r * r * r;
    let z = r - num / den;
    if p < 0.5 {
        -z
    } else {
        z
    }
}

fn median_f64(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n.is_multiple_of(2) {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    } else {
        v[n / 2]
    }
}

fn chrom_order(chrom: &str) -> u32 {
    let s = chrom.trim_start_matches("chr");
    if let Ok(n) = s.parse::<u32>() {
        return n;
    }
    match s.to_uppercase().as_str() {
        "X" => 23,
        "Y" => 24,
        "MT" | "M" => 25,
        _ => 26,
    }
}

// ── parse_sumstats ────────────────────────────────────────────────────

fn builtin_parse_sumstats(args: Vec<Value>) -> Result<Value> {
    let text = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "parse_sumstats() requires Str",
                None,
            ))
        }
    };

    let chrom_names = ["chr", "chrom", "chromosome"];
    let pos_names = ["bp", "pos", "position"];
    let snp_names = ["snp", "rsid", "id", "markername"];
    let pval_names = ["p", "pval", "p_value", "p-value"];
    let beta_names = ["beta", "effect", "b"];
    let se_names = ["se", "stderr"];
    let a1_names = ["a1", "alt", "ea"];
    let a2_names = ["a2", "ref", "nea"];
    let maf_names = ["maf", "eaf", "freq"];

    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .collect();
    if lines.is_empty() {
        return Ok(Value::Table(Table::new(vec![], vec![])));
    }

    // Auto-detect delimiter
    let delim = if lines[0].contains('\t') { '\t' } else { ' ' };

    let header: Vec<String> = lines[0]
        .split(delim)
        .map(|s| s.trim().to_lowercase())
        .collect();

    // Map canonical names to header indices
    let find_col = |candidates: &[&str]| -> Option<usize> {
        candidates
            .iter()
            .find_map(|c| header.iter().position(|h| h == c))
    };

    let idx_chrom = find_col(&chrom_names);
    let idx_pos = find_col(&pos_names);
    let idx_snp = find_col(&snp_names);
    let idx_pval = find_col(&pval_names);
    let idx_beta = find_col(&beta_names);
    let idx_se = find_col(&se_names);
    let idx_a1 = find_col(&a1_names);
    let idx_a2 = find_col(&a2_names);
    let idx_maf = find_col(&maf_names);

    let mut out_cols: Vec<String> = vec![];
    let mut col_indices: Vec<(usize, bool)> = vec![]; // (header_idx, is_numeric)

    let push = |name: &str,
                idx: Option<usize>,
                numeric: bool,
                out_cols: &mut Vec<String>,
                col_indices: &mut Vec<(usize, bool)>| {
        if let Some(i) = idx {
            out_cols.push(name.to_string());
            col_indices.push((i, numeric));
        }
    };

    push("chrom", idx_chrom, false, &mut out_cols, &mut col_indices);
    push("pos", idx_pos, true, &mut out_cols, &mut col_indices);
    push("snp", idx_snp, false, &mut out_cols, &mut col_indices);
    push("pval", idx_pval, true, &mut out_cols, &mut col_indices);
    push("beta", idx_beta, true, &mut out_cols, &mut col_indices);
    push("se", idx_se, true, &mut out_cols, &mut col_indices);
    push("a1", idx_a1, false, &mut out_cols, &mut col_indices);
    push("a2", idx_a2, false, &mut out_cols, &mut col_indices);
    push("maf", idx_maf, true, &mut out_cols, &mut col_indices);

    let mut rows: Vec<Vec<Value>> = vec![];
    for line in &lines[1..] {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let fields: Vec<&str> = line.split(delim).collect();
        let row: Vec<Value> = col_indices
            .iter()
            .map(|(idx, numeric)| {
                let raw = fields.get(*idx).copied().unwrap_or("").trim();
                if *numeric {
                    raw.parse::<f64>()
                        .map(Value::Float)
                        .unwrap_or(Value::Str(raw.to_string()))
                } else {
                    Value::Str(raw.to_string())
                }
            })
            .collect();
        rows.push(row);
    }

    Ok(Value::Table(Table::new(out_cols, rows)))
}

// ── manhattan_data ────────────────────────────────────────────────────

fn builtin_manhattan_data(args: Vec<Value>) -> Result<Value> {
    let t = require_table(&args[0], "manhattan_data")?;
    let ci_chrom = require_col(t, "chrom", "manhattan_data")?;
    let ci_pos = require_col(t, "pos", "manhattan_data")?;
    let ci_pval = require_col(t, "pval", "manhattan_data")?;
    let ci_snp = col_idx(t, "snp");

    // Collect rows with chrom order
    let mut data: Vec<(u32, i64, f64, String)> = t
        .rows
        .iter()
        .map(|row| {
            let chrom_str = match &row[ci_chrom] {
                Value::Str(s) => s.clone(),
                _ => String::new(),
            };
            let pos = to_f64(&row[ci_pos]) as i64;
            let pval = to_f64(&row[ci_pval]);
            let snp = ci_snp.map_or(String::new(), |i| match &row[i] {
                Value::Str(s) => s.clone(),
                _ => String::new(),
            });
            (chrom_order(&chrom_str), pos, pval, snp)
        })
        .collect();

    data.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    // Compute cumulative positions
    let mut chrom_max: HashMap<u32, i64> = HashMap::new();
    for (ord, pos, _, _) in &data {
        let e = chrom_max.entry(*ord).or_insert(0);
        if *pos > *e {
            *e = *pos;
        }
    }

    let mut chroms: Vec<u32> = chrom_max.keys().copied().collect();
    chroms.sort();
    let mut offsets: HashMap<u32, i64> = HashMap::new();
    let mut running = 0i64;
    for c in &chroms {
        offsets.insert(*c, running);
        running += chrom_max[c] + 1_000_000;
    }

    let has_snp = ci_snp.is_some();
    let mut out_cols = vec![
        "chrom".to_string(),
        "pos".to_string(),
        "cumulative_pos".to_string(),
        "neg_log10_p".to_string(),
        "pval".to_string(),
    ];
    if has_snp {
        out_cols.insert(0, "snp".to_string());
    }

    let rows: Vec<Vec<Value>> = data
        .iter()
        .map(|(ord, pos, pval, snp)| {
            let cum = offsets[ord] + pos;
            let nlp = -pval.max(1e-300).log10();
            let mut row = vec![];
            if has_snp {
                row.push(Value::Str(snp.clone()));
            }
            row.push(Value::Int(*ord as i64));
            row.push(Value::Int(*pos));
            row.push(Value::Int(cum));
            row.push(Value::Float(nlp));
            row.push(Value::Float(*pval));
            row
        })
        .collect();

    Ok(Value::Table(Table::new(out_cols, rows)))
}

// ── qq_data ───────────────────────────────────────────────────────────

fn builtin_qq_data(args: Vec<Value>) -> Result<Value> {
    let list = require_list(&args[0], "qq_data")?;
    let mut pvals: Vec<f64> = list.iter().map(to_f64).collect();
    pvals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = pvals.len();
    let cols = vec!["expected".to_string(), "observed".to_string()];
    let rows: Vec<Vec<Value>> = pvals
        .iter()
        .enumerate()
        .map(|(i, &p)| {
            let expected = -((i as f64 + 1.0) / (n as f64 + 1.0)).log10();
            let observed = -p.max(1e-300).log10();
            vec![Value::Float(expected), Value::Float(observed)]
        })
        .collect();
    Ok(Value::Table(Table::new(cols, rows)))
}

// ── clump ─────────────────────────────────────────────────────────────

fn builtin_clump(args: Vec<Value>) -> Result<Value> {
    let t = require_table(&args[0], "clump")?;
    let p_threshold = if args.len() > 1 {
        to_f64(&args[1])
    } else {
        5e-8
    };
    let window_kb = if args.len() > 2 {
        to_f64(&args[2]) as i64
    } else {
        250
    };
    let window_bp = window_kb * 1000;

    let ci_chrom = require_col(t, "chrom", "clump")?;
    let ci_pos = require_col(t, "pos", "clump")?;
    let ci_pval = require_col(t, "pval", "clump")?;

    // Sort rows by pval ascending
    let mut indices: Vec<usize> = (0..t.rows.len()).collect();
    indices.sort_by(|&a, &b| {
        let pa = to_f64(&t.rows[a][ci_pval]);
        let pb = to_f64(&t.rows[b][ci_pval]);
        pa.partial_cmp(&pb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut excluded = vec![false; t.rows.len()];
    let mut index_snps: Vec<usize> = vec![];

    for &i in &indices {
        if excluded[i] {
            continue;
        }
        let p = to_f64(&t.rows[i][ci_pval]);
        if p > p_threshold {
            break;
        }
        index_snps.push(i);
        let chrom_i = match &t.rows[i][ci_chrom] {
            Value::Str(s) => s.clone(),
            _ => String::new(),
        };
        let pos_i = to_f64(&t.rows[i][ci_pos]) as i64;
        // Exclude nearby SNPs on same chrom
        for &j in &indices {
            if excluded[j] || i == j {
                continue;
            }
            let chrom_j = match &t.rows[j][ci_chrom] {
                Value::Str(s) => s.as_str() == chrom_i.as_str(),
                _ => false,
            };
            if !chrom_j {
                continue;
            }
            let pos_j = to_f64(&t.rows[j][ci_pos]) as i64;
            if (pos_j - pos_i).abs() <= window_bp {
                excluded[j] = true;
            }
        }
    }

    let rows: Vec<Vec<Value>> = index_snps.iter().map(|&i| t.rows[i].clone()).collect();
    Ok(Value::Table(Table::new(t.columns.clone(), rows)))
}

// ── top_loci ──────────────────────────────────────────────────────────

fn builtin_top_loci(args: Vec<Value>) -> Result<Value> {
    let t = require_table(&args[0], "top_loci")?;
    let p_threshold = if args.len() > 1 {
        to_f64(&args[1])
    } else {
        5e-8
    };
    let ci_pval = require_col(t, "pval", "top_loci")?;

    let mut rows: Vec<Vec<Value>> = t
        .rows
        .iter()
        .filter(|row| to_f64(&row[ci_pval]) <= p_threshold)
        .cloned()
        .collect();
    rows.sort_by(|a, b| {
        to_f64(&a[ci_pval])
            .partial_cmp(&to_f64(&b[ci_pval]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(Value::Table(Table::new(t.columns.clone(), rows)))
}

// ── lambda_gc ─────────────────────────────────────────────────────────

fn builtin_lambda_gc(args: Vec<Value>) -> Result<Value> {
    let list = require_list(&args[0], "lambda_gc")?;
    let pvals: Vec<f64> = list.iter().map(to_f64).collect();
    if pvals.is_empty() {
        return Ok(Value::Float(1.0));
    }
    // chi2 = (inv_normal(p/2))^2
    let chi2_vals: Vec<f64> = pvals
        .iter()
        .map(|&p| {
            let z = inv_normal(p / 2.0);
            z * z
        })
        .collect();
    let med = median_f64(chi2_vals);
    Ok(Value::Float(med / 0.4549))
}
