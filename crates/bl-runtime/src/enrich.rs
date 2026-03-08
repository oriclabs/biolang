use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

pub fn enrich_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("read_gmt", Arity::Exact(1)),
        ("enrich", Arity::Exact(3)),
        ("ora", Arity::Exact(3)),
        ("gsea", Arity::Exact(2)),
    ]
}

pub fn is_enrich_builtin(name: &str) -> bool {
    matches!(name, "read_gmt" | "enrich" | "ora" | "gsea")
}

pub fn call_enrich_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "read_gmt" => builtin_read_gmt(args),
        "enrich" | "ora" => builtin_enrich(args),
        "gsea" => builtin_gsea(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown enrichment builtin '{name}'"),
            None,
        )),
    }
}

/// Parse GMT format: tab-separated, each line = set_name \t description \t gene1 \t gene2 ...
fn builtin_read_gmt(args: Vec<Value>) -> Result<Value> {
    let path = match &args[0] {
        Value::Str(s) => s.clone(),
        other => {
            return Err(BioLangError::type_error(
                format!("read_gmt() requires Str (path), got {}", other.type_of()),
                None,
            ))
        }
    };

    let content = std::fs::read_to_string(&path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_gmt() failed to read '{}': {}", path, e),
            None,
        )
    })?;

    let mut map = HashMap::new();
    for line in content.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 3 {
            continue;
        }
        let name = fields[0].to_string();
        let genes: Vec<Value> = fields[2..]
            .iter()
            .filter(|s| !s.is_empty())
            .map(|s| Value::Str(s.to_string()))
            .collect();
        map.insert(name, Value::List(genes));
    }

    Ok(Value::Map(map))
}

/// Over-Representation Analysis using hypergeometric test.
/// enrich(genes: List[Str], gene_sets: Map{name → List[Str]}, bg_size: Int) → Table
fn builtin_enrich(args: Vec<Value>) -> Result<Value> {
    let genes: Vec<String> = match &args[0] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => Ok(s.clone()),
                _ => Err(BioLangError::type_error(
                    "enrich() genes must be List of Str",
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "enrich() first arg must be List of gene names",
                None,
            ))
        }
    };

    let gene_sets: HashMap<String, Vec<String>> = match &args[1] {
        Value::Map(m) => {
            let mut sets = HashMap::new();
            for (name, val) in m {
                match val {
                    Value::List(items) => {
                        let gs: Vec<String> = items
                            .iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect();
                        sets.insert(name.clone(), gs);
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            "enrich() gene_sets values must be Lists",
                            None,
                        ))
                    }
                }
            }
            sets
        }
        _ => {
            return Err(BioLangError::type_error(
                "enrich() second arg must be Map of gene sets",
                None,
            ))
        }
    };

    let bg_size = match &args[2] {
        Value::Int(n) => *n as u64,
        _ => {
            return Err(BioLangError::type_error(
                "enrich() third arg must be Int (background size)",
                None,
            ))
        }
    };

    let gene_set_ref: std::collections::HashSet<&str> =
        genes.iter().map(|s| s.as_str()).collect();
    let n = genes.len() as u64; // number of drawn (query genes)

    let mut results: Vec<(String, usize, f64, Vec<String>)> = Vec::new();

    for (set_name, set_genes) in &gene_sets {
        let big_k = set_genes.len() as u64; // total in this set
        let overlap: Vec<String> = set_genes
            .iter()
            .filter(|g| gene_set_ref.contains(g.as_str()))
            .cloned()
            .collect();
        let k = overlap.len() as u64; // overlap count

        if k == 0 {
            results.push((set_name.clone(), 0, 1.0, vec![]));
            continue;
        }

        // Hypergeometric tail probability: P(X >= k)
        let big_n = bg_size; // population size
        let big_n2 = big_n - big_k; // non-successes in population
        let upper = n.min(big_k);
        let mut p_value = 0.0;
        for x in k..=upper {
            p_value +=
                bl_core::bio_core::stats_ops::hypergeometric_prob(x, big_k, big_n2, n);
        }
        p_value = p_value.min(1.0);

        results.push((set_name.clone(), k as usize, p_value, overlap));
    }

    // Sort by p_value
    results.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));

    // BH correction
    let raw_p: Vec<f64> = results.iter().map(|r| r.2).collect();
    let adjusted =
        bl_core::bio_core::stats_ops::benjamini_hochberg_correction(&raw_p, 0.05);

    // Build output table
    let columns = vec![
        "term".to_string(),
        "overlap".to_string(),
        "p_value".to_string(),
        "fdr".to_string(),
        "genes".to_string(),
    ];
    let mut rows = Vec::new();
    for (i, (name, overlap, p, genes_overlap)) in results.iter().enumerate() {
        rows.push(vec![
            Value::Str(name.clone()),
            Value::Int(*overlap as i64),
            Value::Float(*p),
            Value::Float(adjusted.adjusted_p_values[i]),
            Value::Str(genes_overlap.join(",")),
        ]);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

/// Gene Set Enrichment Analysis (Subramanian et al. 2005).
/// gsea(ranked_table: Table{gene, score}, gene_sets: Map{name → List[Str]}) → Table
fn builtin_gsea(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Table(t) => t,
        other => {
            return Err(BioLangError::type_error(
                format!("gsea() requires Table, got {}", other.type_of()),
                None,
            ))
        }
    };

    let gene_sets: HashMap<String, Vec<String>> = match &args[1] {
        Value::Map(m) => {
            let mut sets = HashMap::new();
            for (name, val) in m {
                if let Value::List(items) = val {
                    let gs: Vec<String> = items
                        .iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();
                    sets.insert(name.clone(), gs);
                }
            }
            sets
        }
        _ => {
            return Err(BioLangError::type_error(
                "gsea() second arg must be Map of gene sets",
                None,
            ))
        }
    };

    // Get gene and score columns
    let gene_idx = table.col_index("gene").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "gsea() table needs 'gene' column", None)
    })?;
    let score_idx = table.col_index("score").ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "gsea() table needs 'score' column",
            None,
        )
    })?;

    // Build ranked list (sorted by score descending)
    let mut ranked: Vec<(String, f64)> = Vec::new();
    for row in &table.rows {
        let gene = match &row[gene_idx] {
            Value::Str(s) => s.clone(),
            _ => continue,
        };
        let score = match &row[score_idx] {
            Value::Int(n) => *n as f64,
            Value::Float(f) => *f,
            _ => continue,
        };
        ranked.push((gene, score));
    }
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let _n_genes = ranked.len();

    // xorshift PRNG
    let mut rng = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    if rng == 0 {
        rng = 42;
    }

    let n_perms = 1000usize;

    let mut gsea_results: Vec<(String, f64, f64, f64, f64, String)> = Vec::new();

    for (set_name, set_genes) in &gene_sets {
        let gene_set: std::collections::HashSet<&str> =
            set_genes.iter().map(|s| s.as_str()).collect();

        // Compute enrichment score
        let es = compute_es(&ranked, &gene_set);

        // Permutation null
        let mut null_es = Vec::with_capacity(n_perms);
        let mut perm_ranked = ranked.clone();
        for _ in 0..n_perms {
            // Shuffle scores
            for i in (1..perm_ranked.len()).rev() {
                rng ^= rng << 13;
                rng ^= rng >> 7;
                rng ^= rng << 17;
                let j = (rng as usize) % (i + 1);
                let tmp = perm_ranked[i].1;
                perm_ranked[i].1 = perm_ranked[j].1;
                perm_ranked[j].1 = tmp;
            }
            perm_ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            null_es.push(compute_es(&perm_ranked, &gene_set));
        }

        let mean_null_abs = null_es.iter().map(|e| e.abs()).sum::<f64>() / n_perms as f64;
        let nes = if mean_null_abs > 0.0 {
            es / mean_null_abs
        } else {
            0.0
        };

        let p_value = if es >= 0.0 {
            null_es.iter().filter(|&&e| e >= es).count() as f64 / n_perms as f64
        } else {
            null_es.iter().filter(|&&e| e <= es).count() as f64 / n_perms as f64
        };

        // Leading edge genes
        let leading: Vec<String> = ranked
            .iter()
            .filter(|(g, _)| gene_set.contains(g.as_str()))
            .take(set_genes.len().min(10))
            .map(|(g, _)| g.clone())
            .collect();

        gsea_results.push((
            set_name.clone(),
            es,
            nes,
            p_value,
            0.0, // FDR placeholder
            leading.join(","),
        ));
    }

    // BH correction on p-values
    let raw_p: Vec<f64> = gsea_results.iter().map(|r| r.3).collect();
    let adjusted = bl_core::bio_core::stats_ops::benjamini_hochberg_correction(&raw_p, 0.05);
    for (i, r) in gsea_results.iter_mut().enumerate() {
        r.4 = adjusted.adjusted_p_values[i];
    }

    // Sort by absolute NES descending
    gsea_results.sort_by(|a, b| {
        b.2.abs()
            .partial_cmp(&a.2.abs())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let columns = vec![
        "term".to_string(),
        "es".to_string(),
        "nes".to_string(),
        "p_value".to_string(),
        "fdr".to_string(),
        "leading_edge".to_string(),
    ];
    let rows: Vec<Vec<Value>> = gsea_results
        .into_iter()
        .map(|(name, es, nes, p, fdr, leading)| {
            vec![
                Value::Str(name),
                Value::Float(es),
                Value::Float(nes),
                Value::Float(p),
                Value::Float(fdr),
                Value::Str(leading),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(columns, rows)))
}

/// Compute enrichment score for a ranked gene list and a gene set.
fn compute_es(ranked: &[(String, f64)], gene_set: &std::collections::HashSet<&str>) -> f64 {
    let n = ranked.len() as f64;
    let n_hit: f64 = ranked
        .iter()
        .filter(|(g, _)| gene_set.contains(g.as_str()))
        .map(|(_, s)| s.abs())
        .sum();
    let n_miss = n - ranked.iter().filter(|(g, _)| gene_set.contains(g.as_str())).count() as f64;

    if n_hit == 0.0 || n_miss == 0.0 {
        return 0.0;
    }

    let mut running_sum = 0.0f64;
    let mut max_dev = 0.0f64;
    let miss_penalty = 1.0 / n_miss;

    for (gene, score) in ranked {
        if gene_set.contains(gene.as_str()) {
            running_sum += score.abs() / n_hit;
        } else {
            running_sum -= miss_penalty;
        }
        if running_sum.abs() > max_dev.abs() {
            max_dev = running_sum;
        }
    }

    max_dev
}

