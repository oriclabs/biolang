//! Microbiome / amplicon / shotgun metagenomics builtins.
//!
//! Functions: alpha_diversity, beta_diversity, rarefaction,
//! relative_abundance, taxonomic_collapse.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};

// ── Registry ─────────────────────────────────────────────────────────

pub fn microbiome_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("alpha_diversity", Arity::Range(1, 2)),
        ("beta_diversity", Arity::Range(1, 2)),
        ("rarefaction", Arity::Exact(2)),
        ("relative_abundance", Arity::Exact(1)),
        ("taxonomic_collapse", Arity::Exact(2)),
    ]
}

pub fn is_microbiome_builtin(name: &str) -> bool {
    matches!(
        name,
        "alpha_diversity"
            | "beta_diversity"
            | "rarefaction"
            | "relative_abundance"
            | "taxonomic_collapse"
    )
}

pub fn call_microbiome_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "alpha_diversity" => builtin_alpha_diversity(args),
        "beta_diversity" => builtin_beta_diversity(args),
        "rarefaction" => builtin_rarefaction(args),
        "relative_abundance" => builtin_relative_abundance(args),
        "taxonomic_collapse" => builtin_taxonomic_collapse(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown microbiome builtin '{name}'"),
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

fn counts_from_list(list: &[Value], func: &str) -> Result<Vec<f64>> {
    list.iter()
        .map(|v| match v {
            Value::Int(n) => Ok(*n as f64),
            Value::Float(f) => Ok(*f),
            _ => Err(BioLangError::type_error(
                format!("{func}() counts must be List<Int|Float>"),
                None,
            )),
        })
        .collect()
}

/// LCG pseudo-random for reproducible rarefaction (seed 42).
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0
    }
    fn next_usize(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

// ── alpha_diversity ──────────────────────────────────────────────────

fn builtin_alpha_diversity(args: Vec<Value>) -> Result<Value> {
    let counts = match &args[0] {
        Value::List(l) => counts_from_list(l, "alpha_diversity")?,
        _ => {
            return Err(BioLangError::type_error(
                "alpha_diversity() counts must be List",
                None,
            ))
        }
    };

    let method = if args.len() > 1 {
        require_str(&args[1], "alpha_diversity")?.to_string()
    } else {
        "shannon".to_string()
    };

    let total: f64 = counts.iter().sum();
    if total == 0.0 {
        return Ok(Value::Float(0.0));
    }

    let result = match method.as_str() {
        "shannon" => {
            -counts
                .iter()
                .filter(|&&c| c > 0.0)
                .map(|&c| {
                    let p = c / total;
                    p * p.ln()
                })
                .sum::<f64>()
        }
        "simpson" => {
            1.0 - counts
                .iter()
                .map(|&c| {
                    let p = c / total;
                    p * p
                })
                .sum::<f64>()
        }
        "chao1" => {
            let observed = counts.iter().filter(|&&c| c > 0.0).count() as f64;
            let n1 = counts.iter().filter(|&&c| c == 1.0).count() as f64;
            let n2 = counts.iter().filter(|&&c| c == 2.0).count() as f64;
            if n2 == 0.0 {
                observed + n1 * (n1 - 1.0) / 2.0
            } else {
                observed + (n1 * n1) / (2.0 * n2)
            }
        }
        "observed" => counts.iter().filter(|&&c| c > 0.0).count() as f64,
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("alpha_diversity(): unknown method '{other}'; use shannon|simpson|chao1|observed"),
                None,
            ))
        }
    };

    Ok(Value::Float(result))
}

// ── beta_diversity ───────────────────────────────────────────────────

fn builtin_beta_diversity(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "beta_diversity")?;
    let method = if args.len() > 1 {
        require_str(&args[1], "beta_diversity")?.to_string()
    } else {
        "bray_curtis".to_string()
    };

    let n_samples = table.rows.len();
    let mat: Vec<Vec<f64>> = table
        .rows
        .iter()
        .map(|row| row.iter().map(|v| to_f64(v)).collect::<Vec<f64>>())
        .collect();

    let col_names: Vec<String> = (0..n_samples).map(|i| format!("sample_{i}")).collect();
    let mut out_rows: Vec<Vec<Value>> = Vec::new();

    for i in 0..n_samples {
        let mut row = Vec::new();
        for j in 0..n_samples {
            let d = match method.as_str() {
                "bray_curtis" => {
                    let num: f64 = mat[i].iter().zip(mat[j].iter()).map(|(a, b)| (a - b).abs()).sum::<f64>();
                    let denom: f64 = mat[i].iter().zip(mat[j].iter()).map(|(a, b)| a + b).sum::<f64>();
                    if denom == 0.0 { 0.0 } else { num / denom }
                }
                "jaccard" => {
                    let n_otu = mat[i].len().min(mat[j].len());
                    let mut both = 0usize;
                    let mut either = 0usize;
                    for k in 0..n_otu {
                        let a = mat[i][k] > 0.0;
                        let b = mat[j][k] > 0.0;
                        if a && b { both += 1; }
                        if a || b { either += 1; }
                    }
                    if either == 0 { 0.0 } else { 1.0 - both as f64 / either as f64 }
                }
                other => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("beta_diversity(): unknown method '{other}'; use bray_curtis|jaccard"),
                        None,
                    ))
                }
            };
            row.push(Value::Float(d));
        }
        out_rows.push(row);
    }

    Ok(Value::Table(Table::new(col_names, out_rows)))
}

// ── rarefaction ──────────────────────────────────────────────────────

fn builtin_rarefaction(args: Vec<Value>) -> Result<Value> {
    let counts: Vec<i64> = match &args[0] {
        Value::List(l) => l
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n),
                Value::Float(f) => Ok(*f as i64),
                _ => Err(BioLangError::type_error(
                    "rarefaction() counts must be List<Int>",
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "rarefaction() counts must be List",
                None,
            ))
        }
    };

    let depth = match &args[1] {
        Value::Int(n) => *n,
        _ => {
            return Err(BioLangError::type_error(
                "rarefaction() depth must be Int",
                None,
            ))
        }
    };

    let total: i64 = counts.iter().sum();
    if depth > total {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("rarefaction(): depth {depth} > total reads {total}"),
            None,
        ));
    }

    // Expand counts to a pool of OTU indices
    let mut pool: Vec<usize> = counts
        .iter()
        .enumerate()
        .flat_map(|(otu, &cnt)| std::iter::repeat(otu).take(cnt.max(0) as usize))
        .collect();

    // Fisher-Yates shuffle (deterministic, seed=42) then take first `depth`
    let mut rng = Lcg(42);
    let n = pool.len();
    for i in (1..n).rev() {
        let j = rng.next_usize(i + 1);
        pool.swap(i, j);
    }

    let mut rarefied = vec![0i64; counts.len()];
    for &otu in pool.iter().take(depth as usize) {
        rarefied[otu] += 1;
    }

    Ok(Value::List(rarefied.into_iter().map(Value::Int).collect::<Vec<_>>().into()))
}

// ── relative_abundance ───────────────────────────────────────────────

fn builtin_relative_abundance(args: Vec<Value>) -> Result<Value> {
    let counts = match &args[0] {
        Value::List(l) => l.iter().map(|v| to_f64(v)).collect::<Vec<_>>(),
        _ => {
            return Err(BioLangError::type_error(
                "relative_abundance() counts must be List",
                None,
            ))
        }
    };

    let total: f64 = counts.iter().sum();
    let result: Vec<Value> = if total == 0.0 {
        counts.iter().map(|_| Value::Float(0.0)).collect()
    } else {
        counts.iter().map(|&c| Value::Float(c / total)).collect()
    };

    Ok(Value::List((result).into()))
}

// ── taxonomic_collapse ───────────────────────────────────────────────

fn builtin_taxonomic_collapse(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "taxonomic_collapse")?.clone();
    let level = require_str(&args[1], "taxonomic_collapse")?;

    // Determine rank prefix from level name
    let rank_prefix = match level.to_ascii_lowercase().as_str() {
        "phylum" | "p" => "p__",
        "class" | "c" => "c__",
        "order" | "o" => "o__",
        "family" | "f" => "f__",
        "genus" | "g" => "g__",
        "species" | "s" => "s__",
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "taxonomic_collapse(): unknown level '{other}'; use phylum|class|order|family|genus|species"
                ),
                None,
            ))
        }
    };

    // Find taxonomy column
    let tax_col = table
        .columns
        .iter()
        .position(|c| c == "taxonomy")
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::NameError,
                "taxonomic_collapse(): column 'taxonomy' not found".to_string(),
                None,
            )
        })?;

    // Sample columns = everything except taxonomy
    let sample_cols: Vec<(usize, String)> = table
        .columns
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != tax_col)
        .map(|(i, s): (usize, &String)| (i, s.clone()))
        .collect();

    // Parse taxon at the requested level from a taxonomy string
    let extract_taxon = |tax: &str| -> String {
        for part in tax.split(';') {
            let part = part.trim();
            if part.starts_with(rank_prefix) {
                let name = &part[rank_prefix.len()..];
                if !name.is_empty() {
                    return name.to_string();
                }
            }
        }
        "Unclassified".to_string()
    };

    // Aggregate counts by taxon
    let mut agg: std::collections::HashMap<String, Vec<f64>> = std::collections::HashMap::new();

    for row in &table.rows {
        let tax = match &row[tax_col] {
            Value::Str(s) => s.as_str(),
            _ => "",
        };
        let taxon = extract_taxon(tax);
        let entry = agg.entry(taxon).or_insert_with(|| vec![0.0; sample_cols.len()]);
        for (out_idx, (src_col, _)) in sample_cols.iter().enumerate() {
            entry[out_idx] += to_f64(&row[*src_col]);
        }
    }

    // Build output table
    let mut out_columns = vec!["taxon".to_string()];
    out_columns.extend(sample_cols.iter().map(|(_, name): &(usize, String)| name.clone()));

    let mut taxa: Vec<String> = agg.keys().cloned().collect();
    taxa.sort();

    let rows: Vec<Vec<Value>> = taxa
        .into_iter()
        .map(|taxon| {
            let counts = &agg[&taxon];
            let mut row = vec![Value::Str(taxon)];
            row.extend(counts.iter().map(|&c| Value::Float(c)));
            row
        })
        .collect();

    Ok(Value::Table(Table::new(out_columns, rows)))
}
