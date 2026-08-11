use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::matrix::Matrix;
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

/// Returns the list of (name, arity) for bio ops builtins.
pub fn bio_ops_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("de_bruijn_graph", Arity::Exact(2)),
        ("reversal_distance", Arity::Exact(2)),
        ("sorting_reversals", Arity::Exact(2)),
        ("neighbor_joining", Arity::Exact(1)),
        ("umap", Arity::Range(2, 3)),
        ("tsne", Arity::Range(2, 3)),
        ("leiden", Arity::Range(1, 2)),
        ("diff_expr", Arity::Exact(2)),
    ]
}

/// Check if a name is a known bio_ops builtin.
pub fn is_bio_ops_builtin(name: &str) -> bool {
    matches!(
        name,
        "de_bruijn_graph"
            | "reversal_distance"
            | "sorting_reversals"
            | "neighbor_joining"
            | "umap"
            | "tsne"
            | "leiden"
            | "diff_expr"
    )
}

/// Execute a bio ops builtin by name.
pub fn call_bio_ops_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "de_bruijn_graph" => builtin_de_bruijn_graph(args),
        "reversal_distance" => builtin_reversal_distance(args),
        "sorting_reversals" => builtin_sorting_reversals(args),
        "neighbor_joining" => builtin_neighbor_joining(args),
        "umap" => builtin_umap(args),
        "tsne" => builtin_tsne(args),
        "leiden" => builtin_leiden(args),
        "diff_expr" => builtin_diff_expr(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown bio_ops builtin: {name}"),
            None,
        )),
    }
}

fn builtin_de_bruijn_graph(args: Vec<Value>) -> Result<Value> {
    let sequences: Vec<String> = match &args[0] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::DNA(s) | Value::RNA(s) => Ok(s.data.clone()),
                Value::Str(s) => Ok(s.clone()),
                other => Err(BioLangError::type_error(
                    format!(
                        "de_bruijn_graph() requires List of DNA/Str, got {}",
                        other.type_of()
                    ),
                    None,
                )),
            })
            .collect::<Result<_>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("de_bruijn_graph() requires List, got {}", other.type_of()),
                None,
            ))
        }
    };
    let k = match &args[1] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("de_bruijn_graph() k must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };

    let seq_refs: Vec<&str> = sequences.iter().map(|s| s.as_str()).collect();
    let (nodes, edges) = bl_core::bio_core::graph_ops::de_bruijn_graph(&seq_refs, k);

    let node_list: Vec<Value> = nodes
        .iter()
        .map(|n| Value::Str(n.sequence.clone()))
        .collect();

    let edge_list: Vec<Value> = edges
        .iter()
        .map(|e| {
            let mut rec = HashMap::new();
            rec.insert("from".into(), Value::Str(e.from.clone()));
            rec.insert("to".into(), Value::Str(e.to.clone()));
            rec.insert("label".into(), Value::Str(e.label.clone()));
            Value::Record((rec).into())
        })
        .collect();

    let mut result = HashMap::new();
    result.insert("nodes".into(), Value::List((node_list).into()));
    result.insert("edges".into(), Value::List((edge_list).into()));
    Ok(Value::Record((result).into()))
}

fn builtin_neighbor_joining(args: Vec<Value>) -> Result<Value> {
    let (distances, names) = match &args[0] {
        Value::Matrix(m) => {
            let mut dists = Vec::with_capacity(m.nrow);
            for i in 0..m.nrow {
                dists.push(m.row(i));
            }
            (dists, m.row_names.clone())
        }
        Value::List(outer) => {
            let mut dists = Vec::new();
            for row_val in outer.iter() {
                match row_val {
                    Value::List(inner) => {
                        let row: Vec<f64> = inner
                            .iter()
                            .map(|v| match v {
                                Value::Float(f) => *f,
                                Value::Int(n) => *n as f64,
                                _ => 0.0,
                            })
                            .collect();
                        dists.push(row);
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            "neighbor_joining() requires Matrix or List[List[Float]]",
                            None,
                        ))
                    }
                }
            }
            (dists, None)
        }
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "neighbor_joining() requires Matrix or List, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };

    let tree = bl_core::bio_core::phylo_ops::neighbor_joining(&distances, names.as_deref());

    let nodes: Vec<Value> = tree
        .iter()
        .map(|n| {
            let mut rec = HashMap::new();
            rec.insert("name".into(), Value::Str(n.name.clone()));
            rec.insert("distance".into(), Value::Float(n.distance));
            rec.insert(
                "children".into(),
                Value::List(
                    n.children
                        .iter()
                        .map(|&c| Value::Int(c as i64))
                        .collect::<Vec<_>>()
                        .into(),
                ),
            );
            Value::Record((rec).into())
        })
        .collect();

    Ok(Value::List((nodes).into()))
}

fn matrix_from_value(val: &Value) -> Result<Vec<Vec<f64>>> {
    match val {
        Value::Matrix(m) => {
            let mut rows = Vec::with_capacity(m.nrow);
            for i in 0..m.nrow {
                rows.push(m.row(i));
            }
            Ok(rows)
        }
        Value::List(outer) => {
            let mut rows = Vec::new();
            for row_val in outer.iter() {
                match row_val {
                    Value::List(inner) => {
                        let row: Vec<f64> = inner
                            .iter()
                            .map(|v| match v {
                                Value::Float(f) => *f,
                                Value::Int(n) => *n as f64,
                                _ => 0.0,
                            })
                            .collect();
                        rows.push(row);
                    }
                    _ => {
                        return Err(BioLangError::type_error(
                            "Expected Matrix or List[List[Float]]",
                            None,
                        ))
                    }
                }
            }
            Ok(rows)
        }
        other => Err(BioLangError::type_error(
            format!("Expected Matrix or List, got {}", other.type_of()),
            None,
        )),
    }
}

fn extract_record_float(args: &[Value], idx: usize, key: &str, default: f64) -> f64 {
    if args.len() > idx {
        if let Value::Record(map) = &args[idx] {
            if let Some(v) = map.get(key) {
                return match v {
                    Value::Float(f) => *f,
                    Value::Int(n) => *n as f64,
                    _ => default,
                };
            }
        }
    }
    default
}

fn extract_record_usize(args: &[Value], idx: usize, key: &str, default: usize) -> usize {
    if args.len() > idx {
        if let Value::Record(map) = &args[idx] {
            if let Some(Value::Int(n)) = map.get(key) {
                return *n as usize;
            }
        }
    }
    default
}

fn extract_record_string(args: &[Value], idx: usize, key: &str, default: &str) -> String {
    if let Some(Value::Record(map)) = args.get(idx) {
        if let Some(Value::Str(value)) = map.get(key) {
            return value.clone();
        }
    }
    default.to_string()
}

fn builtin_umap(args: Vec<Value>) -> Result<Value> {
    let data = matrix_from_value(&args[0])?;
    let n_components = match &args[1] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("umap() n_components must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    let n_neighbors = extract_record_usize(&args, 2, "n_neighbors", 15);
    let n_epochs = extract_record_usize(&args, 2, "n_epochs", 200);
    let min_dist = extract_record_float(&args, 2, "min_dist", 0.1);
    let spread = extract_record_float(&args, 2, "spread", 1.0);
    let seed = extract_record_usize(&args, 2, "seed", 42) as u64;
    let negative_sample_rate = extract_record_usize(&args, 2, "negative_sample_rate", 5);
    let metric = extract_record_string(&args, 2, "metric", "euclidean");

    let embeddings = bl_core::bio_core::dimreduce_ops::umap_configured(
        &data,
        n_components,
        n_neighbors,
        n_epochs,
        min_dist,
        spread,
        &metric,
        seed,
        negative_sample_rate,
    );

    let nrow = embeddings.len();
    let ncol = if nrow > 0 { embeddings[0].len() } else { 0 };
    let flat: Vec<f64> = embeddings.into_iter().flatten().collect();
    let m = Matrix::new(flat, nrow, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, &e, None))?;
    Ok(Value::Matrix(m.into()))
}

fn builtin_tsne(args: Vec<Value>) -> Result<Value> {
    let data = matrix_from_value(&args[0])?;
    let n_components = match &args[1] {
        Value::Int(n) => *n as usize,
        other => {
            return Err(BioLangError::type_error(
                format!("tsne() n_components must be Int, got {}", other.type_of()),
                None,
            ))
        }
    };
    let perplexity = extract_record_float(&args, 2, "perplexity", 30.0);
    let n_iter = extract_record_usize(&args, 2, "n_iter", 1000);
    let learning_rate = extract_record_float(&args, 2, "learning_rate", 200.0);

    let embeddings = bl_core::bio_core::dimreduce_ops::tsne(
        &data,
        n_components,
        perplexity,
        n_iter,
        learning_rate,
    );

    let nrow = embeddings.len();
    let ncol = if nrow > 0 { embeddings[0].len() } else { 0 };
    let flat: Vec<f64> = embeddings.into_iter().flatten().collect();
    let m = Matrix::new(flat, nrow, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, &e, None))?;
    Ok(Value::Matrix(m.into()))
}

fn builtin_leiden(args: Vec<Value>) -> Result<Value> {
    let adj = matrix_from_value(&args[0])?;
    let resolution = if args.len() > 1 {
        match &args[1] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => 1.0,
        }
    } else {
        1.0
    };

    let clusters = bl_core::bio_core::cluster_ops::leiden(&adj, resolution);
    Ok(Value::List(
        clusters
            .into_iter()
            .map(|c| Value::Int(c as i64))
            .collect::<Vec<_>>()
            .into(),
    ))
}

fn builtin_diff_expr(args: Vec<Value>) -> Result<Value> {
    // counts: Table or Matrix
    let (counts, gene_names) = match &args[0] {
        Value::Table(t) => {
            // Each row is a gene, columns after the first are samples
            let mut matrix = Vec::new();
            let mut names = Vec::new();
            for row in &t.rows {
                let gene_name = row.first().map(|v| format!("{v}")).unwrap_or_default();
                names.push(gene_name);
                let vals: Vec<f64> = row[1..]
                    .iter()
                    .map(|v| match v {
                        Value::Float(f) => *f,
                        Value::Int(n) => *n as f64,
                        _ => 0.0,
                    })
                    .collect();
                matrix.push(vals);
            }
            (matrix, Some(names))
        }
        Value::Matrix(m) => {
            let mut rows = Vec::with_capacity(m.nrow);
            for i in 0..m.nrow {
                rows.push(m.row(i));
            }
            (rows, m.row_names.clone())
        }
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "diff_expr() requires Table or Matrix, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };

    let groups: Vec<usize> = match &args[1] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                other => Err(BioLangError::type_error(
                    format!(
                        "diff_expr() groups must be List[Int], got {}",
                        other.type_of()
                    ),
                    None,
                )),
            })
            .collect::<Result<_>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("diff_expr() groups must be List, got {}", other.type_of()),
                None,
            ))
        }
    };

    let results =
        bl_core::bio_core::diffexpr_ops::diff_expr(&counts, &groups, gene_names.as_deref());

    let columns = vec![
        "gene".into(),
        "log2fc".into(),
        "pvalue".into(),
        "padj".into(),
        "mean_a".into(),
        "mean_b".into(),
    ];
    let rows: Vec<Vec<Value>> = results
        .into_iter()
        .map(|r| {
            vec![
                Value::Str(r.gene),
                Value::Float(r.log2fc),
                Value::Float(r.pvalue),
                Value::Float(r.padj),
                Value::Float(r.mean_a),
                Value::Float(r.mean_b),
            ]
        })
        .collect();

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── Reversal distance ────────────────────────────────────────────────
//
// Chromosomes rearrange by reversal, so the fewest reversals separating two
// gene orders measures how far two genomes have drifted. Unlike the 2-break
// distance there is no useful closed form at this size, so it is searched for —
// which is why the search lives in Rust: ten elements give 45 reversals and 3.6
// million reachable orders.

fn read_permutation(value: &Value, func: &str) -> Result<Vec<u8>> {
    let items = match value {
        Value::List(items) => items,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "{func}() requires a list of positive integers, got {}",
                    other.type_of()
                ),
                None,
            ));
        }
    };
    let values: Vec<u8> = items
        .iter()
        .map(|item| match item {
            Value::Int(n) if *n >= 1 && *n <= 255 => Ok(*n as u8),
            Value::Int(n) => Err(BioLangError::type_error(
                format!("{func}(): {n} is outside the range this supports (1..255)"),
                None,
            )),
            other => Err(BioLangError::type_error(
                format!(
                    "{func}() permutations hold integers, got {}",
                    other.type_of()
                ),
                None,
            )),
        })
        .collect::<Result<_>>()?;

    // A permutation, not merely a list: a repeated or missing value would make
    // the relabelling below meaningless rather than merely wrong.
    let mut seen = values.clone();
    seen.sort_unstable();
    seen.dedup();
    if seen.len() != values.len() {
        return Err(BioLangError::type_error(
            format!("{func}() needs a permutation — a value is repeated"),
            None,
        ));
    }
    Ok(values)
}

fn both_permutations(args: &[Value], func: &str) -> Result<(Vec<u8>, Vec<u8>)> {
    let source = read_permutation(&args[0], func)?;
    let target = read_permutation(&args[1], func)?;
    if source.len() != target.len() {
        return Err(BioLangError::type_error(
            format!(
                "{func}(): the permutations are {} and {} long — they have to match",
                source.len(),
                target.len()
            ),
            None,
        ));
    }
    Ok((source, target))
}

/// `reversal_distance(source, target)` — the fewest reversals taking one
/// permutation to the other.
fn builtin_reversal_distance(args: Vec<Value>) -> Result<Value> {
    let (source, target) = both_permutations(&args, "reversal_distance")?;
    bl_core::bio_core::reversal::reversal_distance(&source, &target)
        .map(|d| Value::Int(d as i64))
        .ok_or_else(|| {
            BioLangError::type_error(
                "reversal_distance(): the two must be permutations of the same values",
                None,
            )
        })
}

/// `sorting_reversals(source, target)` — one shortest sequence of reversals, as
/// 1-based inclusive `[from, to]` pairs.
///
/// 1-based because that is how the intervals are written in the literature and
/// in the problems that ask for them; the core works 0-based.
fn builtin_sorting_reversals(args: Vec<Value>) -> Result<Value> {
    let (source, target) = both_permutations(&args, "sorting_reversals")?;
    let steps =
        bl_core::bio_core::reversal::sorting_reversals(&source, &target).ok_or_else(|| {
            BioLangError::type_error(
                "sorting_reversals(): the two must be permutations of the same values",
                None,
            )
        })?;
    let listed: Vec<Value> = steps
        .into_iter()
        .map(|(from, to)| {
            Value::List(vec![Value::Int(from as i64 + 1), Value::Int(to as i64 + 1)].into())
        })
        .collect();
    Ok(Value::List(listed.into()))
}

#[cfg(test)]
mod reversal_tests {
    use super::*;

    fn permutation(values: &[i64]) -> Value {
        Value::List(
            values
                .iter()
                .map(|v| Value::Int(*v))
                .collect::<Vec<_>>()
                .into(),
        )
    }

    #[test]
    fn the_published_rear_distances() {
        let cases: [(&[i64], &[i64], i64); 3] = [
            (
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                &[3, 1, 5, 2, 7, 4, 9, 6, 10, 8],
                9,
            ),
            (
                &[3, 10, 8, 2, 5, 4, 7, 1, 6, 9],
                &[5, 2, 3, 1, 7, 4, 10, 8, 6, 9],
                4,
            ),
            (
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                0,
            ),
        ];
        for (source, target, expected) in cases {
            let got =
                builtin_reversal_distance(vec![permutation(source), permutation(target)]).unwrap();
            assert_eq!(got, Value::Int(expected), "{source:?} -> {target:?}");
        }
    }

    #[test]
    fn the_reversals_returned_actually_sort() {
        let source = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let target = [1, 8, 9, 3, 2, 7, 6, 5, 4, 10];
        let steps =
            builtin_sorting_reversals(vec![permutation(&source), permutation(&target)]).unwrap();
        let mut current: Vec<i64> = source.to_vec();
        match steps {
            Value::List(items) => {
                assert_eq!(items.len(), 2, "the sample needs two reversals");
                for step in items.iter() {
                    match step {
                        Value::List(pair) => match (&pair[0], &pair[1]) {
                            (Value::Int(from), Value::Int(to)) => {
                                // 1-based inclusive.
                                current[(*from as usize - 1)..*to as usize].reverse();
                            }
                            other => panic!("expected two ints, got {other:?}"),
                        },
                        other => panic!("expected a pair, got {other:?}"),
                    }
                }
            }
            other => panic!("expected a list, got {other:?}"),
        }
        assert_eq!(current, target.to_vec());
    }

    #[test]
    fn a_repeated_value_is_not_a_permutation() {
        let error =
            builtin_reversal_distance(vec![permutation(&[1, 1, 2]), permutation(&[1, 2, 3])])
                .expect_err("not a permutation");
        assert!(error.to_string().contains("repeated"), "{error}");
    }

    #[test]
    fn mismatched_lengths_are_reported() {
        let error = builtin_reversal_distance(vec![permutation(&[1, 2, 3]), permutation(&[1, 2])])
            .expect_err("different lengths");
        assert!(error.to_string().contains("have to match"), "{error}");
    }
}
