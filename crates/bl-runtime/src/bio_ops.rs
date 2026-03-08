use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::matrix::Matrix;
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

/// Returns the list of (name, arity) for bio ops builtins.
pub fn bio_ops_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("de_bruijn_graph", Arity::Exact(2)),
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
        "de_bruijn_graph" | "neighbor_joining" | "umap" | "tsne" | "leiden" | "diff_expr"
    )
}

/// Execute a bio ops builtin by name.
pub fn call_bio_ops_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "de_bruijn_graph" => builtin_de_bruijn_graph(args),
        "neighbor_joining" => builtin_neighbor_joining(args),
        "umap" => builtin_umap(args),
        "tsne" => builtin_tsne(args),
        "leiden" => builtin_leiden(args),
        "diff_expr" => builtin_diff_expr(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            &format!("unknown bio_ops builtin: {name}"),
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
                    format!("de_bruijn_graph() requires List of DNA/Str, got {}", other.type_of()),
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
            Value::Record(rec)
        })
        .collect();

    let mut result = HashMap::new();
    result.insert("nodes".into(), Value::List(node_list));
    result.insert("edges".into(), Value::List(edge_list));
    Ok(Value::Record(result))
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
            for row_val in outer {
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
                format!("neighbor_joining() requires Matrix or List, got {}", other.type_of()),
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
                Value::List(n.children.iter().map(|&c| Value::Int(c as i64)).collect()),
            );
            Value::Record(rec)
        })
        .collect();

    Ok(Value::List(nodes))
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
            for row_val in outer {
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

    let embeddings = bl_core::bio_core::dimreduce_ops::umap(&data, n_components, n_neighbors, n_epochs, min_dist);

    let nrow = embeddings.len();
    let ncol = if nrow > 0 { embeddings[0].len() } else { 0 };
    let flat: Vec<f64> = embeddings.into_iter().flatten().collect();
    let m = Matrix::new(flat, nrow, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, &e, None))?;
    Ok(Value::Matrix(m))
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

    let embeddings = bl_core::bio_core::dimreduce_ops::tsne(&data, n_components, perplexity, n_iter, learning_rate);

    let nrow = embeddings.len();
    let ncol = if nrow > 0 { embeddings[0].len() } else { 0 };
    let flat: Vec<f64> = embeddings.into_iter().flatten().collect();
    let m = Matrix::new(flat, nrow, ncol)
        .map_err(|e| BioLangError::runtime(ErrorKind::TypeError, &e, None))?;
    Ok(Value::Matrix(m))
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

    let clusters = bl_core::bio_core::cluster_ops::louvain(&adj, resolution);
    Ok(Value::List(clusters.into_iter().map(|c| Value::Int(c as i64)).collect()))
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
                format!("diff_expr() requires Table or Matrix, got {}", other.type_of()),
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
                    format!("diff_expr() groups must be List[Int], got {}", other.type_of()),
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

    let results = bl_core::bio_core::diffexpr_ops::diff_expr(&counts, &groups, gene_names.as_deref());

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
