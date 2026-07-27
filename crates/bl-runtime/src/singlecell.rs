//! Single-cell RNA-seq (Section 6) and CNV / tumour-purity (Section 7) builtins.
//!
//! Functions: normalize_total, log1p_transform, highly_variable_genes, cell_qc,
//! gene_qc, knn_graph, doublet_score, cnv_segment, loh_detect, tumor_purity,
//! vaf_to_ccf, mutational_signature,
//! read_10x, cell_cycle_score, module_score, sc_sctransform, sc_integrate,
//! diffusion_pseudotime, lr_score, lr_aggregate,
//! spatial_neighbors, spatial_moransi, reference_classify, pseudobulk_aggregate,
//! wnn_graph, velocity_estimate.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};
use std::io::{BufRead, BufReader};

// ── Registry ─────────────────────────────────────────────────────────

pub fn singlecell_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        // Section 6: Single-cell QC / normalisation
        ("normalize_total", Arity::Range(1, 2)),
        ("log1p_transform", Arity::Exact(1)),
        ("highly_variable_genes", Arity::Range(1, 2)),
        ("cell_qc", Arity::Range(1, 3)),
        ("gene_qc", Arity::Range(1, 2)),
        ("knn_graph", Arity::Range(1, 2)),
        ("leiden_cluster", Arity::Range(2, 3)),
        ("select_cols", Arity::Exact(2)),
        ("doublet_score", Arity::Range(1, 2)),
        // Section 6 extensions: Seurat-compatible single-cell ops
        ("read_10x", Arity::Range(1, 2)),
        ("cell_cycle_score", Arity::Exact(3)),
        ("module_score", Arity::Exact(2)),
        ("sc_sctransform", Arity::Exact(1)),
        ("sc_integrate", Arity::Exact(2)),
        ("diffusion_pseudotime", Arity::Exact(3)),
        // Cell-cell communication
        ("lr_score", Arity::Exact(3)),
        ("lr_aggregate", Arity::Exact(2)),
        // Spatial transcriptomics
        ("spatial_neighbors", Arity::Range(1, 2)),
        ("spatial_moransi", Arity::Exact(2)),
        // Cell type annotation
        ("reference_classify", Arity::Exact(3)),
        // Pseudobulk aggregation
        ("pseudobulk_aggregate", Arity::Exact(3)),
        // Multimodal integration
        ("wnn_graph", Arity::Exact(3)),
        // RNA velocity
        ("velocity_estimate", Arity::Exact(2)),
        // Section 7: CNV / tumour purity
        ("cnv_segment", Arity::Range(1, 2)),
        ("loh_detect", Arity::Exact(1)),
        ("tumor_purity", Arity::Exact(1)),
        ("vaf_to_ccf", Arity::Range(2, 4)),
        ("mutational_signature", Arity::Exact(1)),
    ]
}

pub fn is_singlecell_builtin(name: &str) -> bool {
    matches!(
        name,
        "normalize_total"
            | "log1p_transform"
            | "highly_variable_genes"
            | "cell_qc"
            | "gene_qc"
            | "knn_graph"
            | "leiden_cluster"
            | "select_cols"
            | "doublet_score"
            | "read_10x"
            | "cell_cycle_score"
            | "module_score"
            | "sc_sctransform"
            | "sc_integrate"
            | "diffusion_pseudotime"
            | "lr_score"
            | "lr_aggregate"
            | "spatial_neighbors"
            | "spatial_moransi"
            | "reference_classify"
            | "pseudobulk_aggregate"
            | "wnn_graph"
            | "velocity_estimate"
            | "cnv_segment"
            | "loh_detect"
            | "tumor_purity"
            | "vaf_to_ccf"
            | "mutational_signature"
    )
}

pub fn call_singlecell_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "normalize_total" => builtin_normalize_total(args),
        "log1p_transform" => builtin_log1p_transform(args),
        "highly_variable_genes" => builtin_highly_variable_genes(args),
        "cell_qc" => builtin_cell_qc(args),
        "gene_qc" => builtin_gene_qc(args),
        "knn_graph" => builtin_knn_graph(args),
        "leiden_cluster" => builtin_leiden_cluster(args),
        "select_cols" => builtin_select_cols(args),
        "doublet_score" => builtin_doublet_score(args),
        "read_10x" => builtin_read_10x(args),
        "cell_cycle_score" => builtin_cell_cycle_score(args),
        "module_score" => builtin_module_score(args),
        "sc_sctransform" => builtin_sc_sctransform(args),
        "sc_integrate" => builtin_sc_integrate(args),
        "diffusion_pseudotime" => builtin_diffusion_pseudotime(args),
        "lr_score" => builtin_lr_score(args),
        "lr_aggregate" => builtin_lr_aggregate(args),
        "spatial_neighbors" => builtin_spatial_neighbors(args),
        "spatial_moransi" => builtin_spatial_moransi(args),
        "reference_classify" => builtin_reference_classify(args),
        "pseudobulk_aggregate" => builtin_pseudobulk_aggregate(args),
        "wnn_graph" => builtin_wnn_graph(args),
        "velocity_estimate" => builtin_velocity_estimate(args),
        "cnv_segment" => builtin_cnv_segment(args),
        "loh_detect" => builtin_loh_detect(args),
        "tumor_purity" => builtin_tumor_purity(args),
        "vaf_to_ccf" => builtin_vaf_to_ccf(args),
        "mutational_signature" => builtin_mutational_signature(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown singlecell builtin: {name}"),
            None,
        )),
    }
}

// ── Helper functions ─────────────────────────────────────────────────

fn to_f64(v: &Value) -> Option<f64> {
    match v {
        Value::Float(f) => Some(*f),
        Value::Int(n) => Some(*n as f64),
        _ => None,
    }
}

fn require_matrix(val: &Value, func: &str) -> Result<Vec<Vec<f64>>> {
    match val {
        Value::List(rows) => rows
            .iter()
            .map(|row| match row {
                Value::List(cells) => cells
                    .iter()
                    .map(|v| {
                        to_f64(v).ok_or_else(|| {
                            BioLangError::type_error(
                                format!("{func}() matrix must contain numbers"),
                                None,
                            )
                        })
                    })
                    .collect(),
                _ => Err(BioLangError::type_error(
                    format!("{func}() matrix rows must be Lists"),
                    None,
                )),
            })
            .collect(),
        Value::Table(t) => Ok(t
            .rows
            .iter()
            .map(|row| row.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect())
            .collect()),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires List<List> or Table"),
            None,
        )),
    }
}

fn require_int(val: &Value, func: &str) -> Result<i64> {
    match val {
        Value::Int(n) => Ok(*n),
        _ => Err(BioLangError::type_error(
            format!("{func}() requires Int"),
            None,
        )),
    }
}

fn matrix_to_value(mat: Vec<Vec<f64>>) -> Value {
    Value::List(
        mat.into_iter()
            .map(|row| Value::List(row.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>().into(),
    )
}

// ── select_cols(matrix, indices) ─────────────────────────────────────
// Subset a matrix to the given column indices, in Rust. Replaces the
// interpreted `mat |> map(|row| idx |> map(|j| row[j]))` double-loop that
// dominates HVG selection on real-sized data (hundreds of thousands of
// interpreted closure calls).
fn builtin_select_cols(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "select_cols")?;
    let indices: Vec<usize> = match &args[1] {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                Value::Float(f) => Ok(*f as usize),
                other => Err(BioLangError::type_error(
                    format!("select_cols() indices must be Int, got {}", other.type_of()),
                    None,
                )),
            })
            .collect::<Result<Vec<usize>>>()?,
        other => {
            return Err(BioLangError::type_error(
                format!("select_cols() requires a List of indices, got {}", other.type_of()),
                None,
            ))
        }
    };
    let ncol = mat.first().map(|r| r.len()).unwrap_or(0);
    let out: Vec<Vec<f64>> = mat
        .iter()
        .map(|row| indices.iter().map(|&j| if j < ncol { row[j] } else { 0.0 }).collect())
        .collect();
    Ok(matrix_to_value(out))
}

// ── Section 6: Single-cell QC / normalisation ────────────────────────

// ── normalize_total(matrix, target=10000) ────────────────────────────

fn builtin_normalize_total(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "normalize_total")?;
    let target = if args.len() > 1 {
        match &args[1] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => {
                return Err(BioLangError::type_error(
                    "normalize_total() target must be a number",
                    None,
                ))
            }
        }
    } else {
        10_000.0
    };

    let normalized: Vec<Vec<f64>> = mat
        .into_iter()
        .map(|row| {
            let total: f64 = row.iter().sum();
            if total == 0.0 {
                row
            } else {
                row.into_iter().map(|v| v / total * target).collect()
            }
        })
        .collect();

    Ok(matrix_to_value(normalized))
}

// ── log1p_transform(matrix) ──────────────────────────────────────────

fn builtin_log1p_transform(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "log1p_transform")?;
    let transformed: Vec<Vec<f64>> = mat
        .into_iter()
        .map(|row| row.into_iter().map(|v| (v + 1.0).ln()).collect())
        .collect();
    Ok(matrix_to_value(transformed))
}

// ── highly_variable_genes(matrix, n=2000) ────────────────────────────

fn builtin_highly_variable_genes(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "highly_variable_genes")?;
    let n_hvg = if args.len() > 1 {
        require_int(&args[1], "highly_variable_genes")? as usize
    } else {
        2000
    };

    if mat.is_empty() {
        return Ok(Value::List((vec![]).into()));
    }
    let n_cells = mat.len() as f64;
    let n_genes = mat[0].len();

    // Per-gene mean
    let mut means = vec![0.0f64; n_genes];
    for row in &mat {
        for (j, &v) in row.iter().enumerate() {
            if j < n_genes {
                means[j] += v;
            }
        }
    }
    for m in &mut means {
        *m /= n_cells;
    }

    // Per-gene variance
    let mut variances = vec![0.0f64; n_genes];
    for row in &mat {
        for (j, &v) in row.iter().enumerate() {
            if j < n_genes {
                let d = v - means[j];
                variances[j] += d * d;
            }
        }
    }
    for v in &mut variances {
        *v /= n_cells;
    }

    // cv2 = variance / (mean^2 + 1e-10)
    let mut gene_cv2: Vec<(usize, f64)> = (0..n_genes)
        .map(|j| {
            let cv2 = variances[j] / (means[j] * means[j] + 1e-10);
            (j, cv2)
        })
        .collect();

    gene_cv2.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let top_n = n_hvg.min(n_genes);
    let indices: Vec<Value> = gene_cv2
        .into_iter()
        .take(top_n)
        .map(|(idx, _)| Value::Int(idx as i64))
        .collect();

    Ok(Value::List((indices).into()))
}

// ── cell_qc(matrix, gene_names?, mito_prefix="MT-") ─────────────────

fn builtin_cell_qc(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "cell_qc")?;

    let gene_names: Option<Vec<String>> = if args.len() > 1 {
        match &args[1] {
            Value::List(names) => Some(
                names
                    .iter()
                    .map(|v| match v {
                        Value::Str(s) => s.clone(),
                        other => format!("{other}"),
                    })
                    .collect(),
            ),
            Value::Nil => None,
            _ => None,
        }
    } else {
        None
    };

    let mito_prefix = if args.len() > 2 {
        match &args[2] {
            Value::Str(s) => s.clone(),
            _ => "MT-".to_string(),
        }
    } else {
        "MT-".to_string()
    };

    let mito_indices: Vec<usize> = if let Some(ref names) = gene_names {
        names
            .iter()
            .enumerate()
            .filter_map(|(i, n)| {
                if n.starts_with(&mito_prefix) {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    } else {
        vec![]
    };

    let columns = vec![
        "cell_idx".to_string(),
        "total_counts".to_string(),
        "n_genes".to_string(),
        "pct_mito".to_string(),
    ];

    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(mat.len());
    for (i, row) in mat.iter().enumerate() {
        let total: f64 = row.iter().sum();
        let n_genes_detected = row.iter().filter(|&&v| v > 0.0).count() as i64;
        let mito_counts: f64 = mito_indices
            .iter()
            .map(|&idx| row.get(idx).copied().unwrap_or(0.0))
            .sum();
        let pct_mito = if total > 0.0 && gene_names.is_some() {
            mito_counts / total * 100.0
        } else {
            0.0
        };
        rows.push(vec![
            Value::Int(i as i64),
            Value::Float(total),
            Value::Int(n_genes_detected),
            Value::Float(pct_mito),
        ]);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── gene_qc(matrix, gene_names?) ─────────────────────────────────────

fn builtin_gene_qc(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "gene_qc")?;

    let columns = vec![
        "gene_idx".to_string(),
        "n_cells".to_string(),
        "mean_expression".to_string(),
        "pct_dropout".to_string(),
    ];

    if mat.is_empty() {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let n_cells = mat.len() as f64;
    let n_genes = mat[0].len();

    let mut n_cells_expr = vec![0usize; n_genes];
    let mut sums = vec![0.0f64; n_genes];

    for row in &mat {
        for (j, &v) in row.iter().enumerate() {
            if j < n_genes {
                sums[j] += v;
                if v > 0.0 {
                    n_cells_expr[j] += 1;
                }
            }
        }
    }

    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_genes);
    for j in 0..n_genes {
        let mean_expr = sums[j] / n_cells;
        let pct_dropout = (n_cells - n_cells_expr[j] as f64) / n_cells * 100.0;
        rows.push(vec![
            Value::Int(j as i64),
            Value::Int(n_cells_expr[j] as i64),
            Value::Float(mean_expr),
            Value::Float(pct_dropout),
        ]);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── knn_graph(embeddings, k=15) ──────────────────────────────────────

fn builtin_knn_graph(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "knn_graph")?;
    let k = if args.len() > 1 {
        require_int(&args[1], "knn_graph")? as usize
    } else {
        15
    };

    let n = embeddings.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let k_actual = k.min(n.saturating_sub(1));

    // Euclidean distance
    let dist = |a: &[f64], b: &[f64]| -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
    };

    let mut edges: Vec<Value> = Vec::new();

    for i in 0..n {
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (j, dist(&embeddings[i], &embeddings[j])))
            .collect();

        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

        for (j, d) in dists.into_iter().take(k_actual) {
            let mut rec = HashMap::new();
            rec.insert("source".to_string(), Value::Int(i as i64));
            rec.insert("target".to_string(), Value::Int(j as i64));
            rec.insert("distance".to_string(), Value::Float(d));
            edges.push(Value::Record((rec).into()));
        }
    }

    Ok(Value::List((edges).into()))
}

// ── leiden_cluster(matrix, k, resolution=1.0) ────────────────────────
// Build a symmetric kNN graph on the embedding, then run real Leiden
// (local moving + refinement + aggregation, connected communities).

fn builtin_leiden_cluster(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "leiden_cluster")?;
    let k = require_int(&args[1], "leiden_cluster")? as usize;
    let resolution = if args.len() > 2 {
        match &args[2] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => 1.0,
        }
    } else {
        1.0
    };

    let n = embeddings.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let k_actual = k.min(n.saturating_sub(1));

    let dist = |a: &[f64], b: &[f64]| -> f64 {
        a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y) * (x - y))
            .sum::<f64>()
            .sqrt()
    };

    // Symmetric kNN adjacency: edge if j is a k-nearest neighbor of i or vice versa.
    let mut adj = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (j, dist(&embeddings[i], &embeddings[j])))
            .collect();
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (j, _) in dists.into_iter().take(k_actual) {
            adj[i][j] = 1.0;
            adj[j][i] = 1.0;
        }
    }

    let labels = bl_core::bio_core::cluster_ops::leiden(&adj, resolution);
    Ok(Value::List(
        labels.into_iter().map(|c| Value::Int(c as i64)).collect::<Vec<_>>().into(),
    ))
}

// ── doublet_score(matrix, n_simulated=500) ───────────────────────────

fn builtin_doublet_score(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "doublet_score")?;
    let n_simulated = if args.len() > 1 {
        require_int(&args[1], "doublet_score")? as usize
    } else {
        500
    };

    let n_cells = mat.len();
    if n_cells < 2 {
        return Ok(Value::List(mat.iter().map(|_| Value::Float(0.0)).collect::<Vec<_>>().into()));
    }
    let n_genes = mat[0].len();

    // Generate artificial doublets via deterministic LCG pairs
    let lcg_next = |s: u64| -> u64 {
        s.wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407)
    };
    let mut lcg_state: u64 = 12345;
    let mut artificial: Vec<Vec<f64>> = Vec::with_capacity(n_simulated);
    for _ in 0..n_simulated {
        lcg_state = lcg_next(lcg_state);
        let i = (lcg_state as usize) % n_cells;
        lcg_state = lcg_next(lcg_state);
        let j = (lcg_state as usize) % n_cells;
        let summed: Vec<f64> = (0..n_genes)
            .map(|g| mat[i].get(g).copied().unwrap_or(0.0) + mat[j].get(g).copied().unwrap_or(0.0))
            .collect();
        artificial.push(summed);
    }

    // Pearson correlation between two equal-length vectors
    let pearson = |a: &[f64], b: &[f64]| -> f64 {
        let n = a.len() as f64;
        if n == 0.0 {
            return 0.0;
        }
        let mean_a = a.iter().sum::<f64>() / n;
        let mean_b = b.iter().sum::<f64>() / n;
        let num: f64 = a
            .iter()
            .zip(b.iter())
            .map(|(&x, &y)| (x - mean_a) * (y - mean_b))
            .sum();
        let da: f64 = a
            .iter()
            .map(|&x| (x - mean_a) * (x - mean_a))
            .sum::<f64>()
            .sqrt();
        let db: f64 = b
            .iter()
            .map(|&y| (y - mean_b) * (y - mean_b))
            .sum::<f64>()
            .sqrt();
        if da == 0.0 || db == 0.0 {
            0.0
        } else {
            num / (da * db)
        }
    };

    // For each real cell find the nearest artificial doublet (max correlation)
    let mut raw_scores: Vec<f64> = mat
        .iter()
        .map(|cell| {
            artificial
                .iter()
                .map(|art| pearson(cell, art))
                .fold(f64::NEG_INFINITY, f64::max)
                .max(0.0)
        })
        .collect();

    // Normalise to [0, 1]
    let min_s = raw_scores.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_s = raw_scores.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max_s - min_s;
    if range > 0.0 {
        for s in &mut raw_scores {
            *s = (*s - min_s) / range;
        }
    }

    Ok(Value::List(
        raw_scores.into_iter().map(Value::Float).collect::<Vec<_>>().into(),
    ))
}

// ── Section 6 extensions: Seurat-compatible single-cell ops ─────────

fn read_lines_from_path(path: &std::path::Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(path).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): cannot open {}: {e}", path.display()),
            None,
        )
    })?;
    let is_gz = path.extension().map_or(false, |e| e == "gz");
    if is_gz {
        #[cfg(feature = "native")]
        {
            use flate2::read::GzDecoder;
            BufReader::new(GzDecoder::new(file))
                .lines()
                .map(|l| {
                    l.map_err(|e| {
                        BioLangError::runtime(
                            ErrorKind::IOError,
                            format!("read_10x(): gz read error: {e}"),
                            None,
                        )
                    })
                })
                .collect()
        }
        #[cfg(not(feature = "native"))]
        Err(BioLangError::runtime(
            ErrorKind::IOError,
            "read_10x(): .gz support requires native feature",
            None,
        ))
    } else {
        BufReader::new(file)
            .lines()
            .map(|l| {
                l.map_err(|e| {
                    BioLangError::runtime(
                        ErrorKind::IOError,
                        format!("read_10x(): read error: {e}"),
                        None,
                    )
                })
            })
            .collect()
    }
}

fn parse_mtx_lines(lines: Vec<String>) -> Result<(usize, usize, Vec<(usize, usize, f64)>)> {
    let mut iter = lines.into_iter();
    iter.next(); // skip %%MatrixMarket header
    let size_line = loop {
        match iter.next() {
            None => {
                return Err(BioLangError::runtime(
                    ErrorKind::IOError,
                    "read_10x(): malformed matrix.mtx (no size line)",
                    None,
                ))
            }
            Some(l) if l.starts_with('%') => continue,
            Some(l) => break l,
        }
    };
    let parts: Vec<&str> = size_line.split_whitespace().collect();
    let n_rows: usize = parts.first().and_then(|s| s.parse().ok()).unwrap_or(0);
    let n_cols: usize = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut entries = Vec::new();
    for line in iter {
        let t = line.trim();
        if t.is_empty() || t.starts_with('%') {
            continue;
        }
        let p: Vec<&str> = t.split_whitespace().collect();
        if p.len() >= 3 {
            let row: usize = p[0].parse().unwrap_or(0);
            let col: usize = p[1].parse().unwrap_or(0);
            let val: f64 = p[2].parse().unwrap_or(0.0);
            entries.push((row, col, val));
        }
    }
    Ok((n_rows, n_cols, entries))
}

// ── read_10x(path) ───────────────────────────────────────────────────

fn builtin_read_10x(args: Vec<Value>) -> Result<Value> {
    use std::path::Path;
    let dir_str = match &args[0] {
        Value::Str(s) => s.clone(),
        _ => {
            return Err(BioLangError::type_error(
                "read_10x() requires a directory path string",
                None,
            ))
        }
    };
    let dir = Path::new(&dir_str);

    let find_file = |names: &[&str]| -> Option<std::path::PathBuf> {
        names.iter().find_map(|n| {
            let p = dir.join(n);
            if p.exists() { Some(p) } else { None }
        })
    };

    let barcodes_path = find_file(&["barcodes.tsv.gz", "barcodes.tsv"]).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): barcodes.tsv not found in {dir_str}"),
            None,
        )
    })?;
    let features_path =
        find_file(&["features.tsv.gz", "features.tsv", "genes.tsv.gz", "genes.tsv"])
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::IOError,
                    format!("read_10x(): features.tsv not found in {dir_str}"),
                    None,
                )
            })?;
    let matrix_path = find_file(&["matrix.mtx.gz", "matrix.mtx"]).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("read_10x(): matrix.mtx not found in {dir_str}"),
            None,
        )
    })?;

    let barcodes: Vec<String> = read_lines_from_path(&barcodes_path)?
        .into_iter()
        .filter(|l| !l.is_empty())
        .collect();

    // features.tsv is `gene_id \t gene_symbol \t feature_type`. Default to the
    // symbol, matching Seurat's Read10X(gene.column = 2) and scanpy's
    // read_10x_mtx(var_names="gene_symbols"). Downstream steps match on symbols
    // — the "MT-" prefix for percent-mito, marker panels, DE output — so
    // reading the Ensembl ID here makes percent-mito silently zero.
    // Pass gene_column = 1 to get Ensembl IDs instead.
    let gene_column = if args.len() > 1 {
        match &args[1] {
            Value::Int(n) if *n == 1 || *n == 2 => *n as usize,
            Value::Int(n) => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("read_10x(): gene_column must be 1 (ID) or 2 (symbol), got {n}"),
                    None,
                ))
            }
            _ => {
                return Err(BioLangError::type_error(
                    "read_10x(): gene_column must be Int",
                    None,
                ))
            }
        }
    } else {
        2
    };

    let genes: Vec<String> = read_lines_from_path(&features_path)?
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| {
            let cols: Vec<&str> = l.split('\t').collect();
            let pick = cols.get(gene_column - 1).map(|s| s.trim()).unwrap_or("");
            if pick.is_empty() {
                // old-style genes.tsv, or a single-column features file
                cols.first().map(|s| s.trim()).unwrap_or("").to_string()
            } else {
                pick.to_string()
            }
        })
        .collect();

    let (n_genes_mtx, n_cells_mtx, entries) = parse_mtx_lines(read_lines_from_path(&matrix_path)?)?;

    let n_g = genes.len().max(n_genes_mtx);
    let n_c = barcodes.len().max(n_cells_mtx);

    let mut matrix = vec![vec![0.0f64; n_g]; n_c];
    for (gene_1, cell_1, val) in entries {
        let g = gene_1.saturating_sub(1);
        let c = cell_1.saturating_sub(1);
        if g < n_g && c < n_c {
            matrix[c][g] = val;
        }
    }

    let mut rec = HashMap::new();
    rec.insert("matrix".to_string(), matrix_to_value(matrix));
    rec.insert(
        "genes".to_string(),
        Value::List(genes.into_iter().map(Value::Str).collect::<Vec<_>>().into()),
    );
    rec.insert(
        "barcodes".to_string(),
        Value::List(barcodes.into_iter().map(Value::Str).collect::<Vec<_>>().into()),
    );
    rec.insert("n_cells".to_string(), Value::Int(n_c as i64));
    rec.insert("n_genes".to_string(), Value::Int(n_g as i64));
    Ok(Value::Record((rec).into()))
}

fn gene_indices_from_value(val: &Value, func: &str) -> Result<Vec<usize>> {
    match val {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Int(n) => Ok(*n as usize),
                _ => Err(BioLangError::type_error(
                    format!("{func}() gene indices must be Int"),
                    None,
                )),
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            format!("{func}() gene indices must be List<Int>"),
            None,
        )),
    }
}

fn cell_mean_expression(row: &[f64], indices: &[usize]) -> f64 {
    if indices.is_empty() {
        return 0.0;
    }
    indices.iter().map(|&i| row.get(i).copied().unwrap_or(0.0)).sum::<f64>()
        / indices.len() as f64
}

// ── cell_cycle_score(matrix, s_gene_indices, g2m_gene_indices) ────────

fn builtin_cell_cycle_score(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "cell_cycle_score")?;
    let s_idx = gene_indices_from_value(&args[1], "cell_cycle_score")?;
    let g2m_idx = gene_indices_from_value(&args[2], "cell_cycle_score")?;

    let scores: Vec<Value> = mat
        .iter()
        .map(|row| {
            let s_score = cell_mean_expression(row, &s_idx);
            let g2m_score = cell_mean_expression(row, &g2m_idx);
            let phase = if s_score > g2m_score && s_score > 0.1 {
                "S"
            } else if g2m_score >= s_score && g2m_score > 0.1 {
                "G2M"
            } else {
                "G1"
            };
            let mut rec = HashMap::new();
            rec.insert("s_score".to_string(), Value::Float(s_score));
            rec.insert("g2m_score".to_string(), Value::Float(g2m_score));
            rec.insert("phase".to_string(), Value::Str(phase.to_string()));
            Value::Record((rec).into())
        })
        .collect();

    Ok(Value::List((scores).into()))
}

// ── module_score(matrix, gene_indices) ───────────────────────────────

fn builtin_module_score(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "module_score")?;
    let indices = gene_indices_from_value(&args[1], "module_score")?;

    let scores: Vec<Value> = mat
        .iter()
        .map(|row| Value::Float(cell_mean_expression(row, &indices)))
        .collect();

    Ok(Value::List((scores).into()))
}

// ── sc_sctransform(matrix) ────────────────────────────────────────────
// Computes Pearson residuals under a simplified negative-binomial model.

fn builtin_sc_sctransform(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "sc_sctransform")?;
    let n_cells = mat.len();
    if n_cells == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let n_genes = mat[0].len();
    let clip_val = (n_cells as f64).sqrt();
    let theta = 100.0_f64; // fixed overdispersion parameter

    let lib_sizes: Vec<f64> = mat.iter().map(|row| row.iter().sum::<f64>()).collect();

    let mut gene_sums = vec![0.0f64; n_genes];
    for row in &mat {
        for (j, &v) in row.iter().enumerate() {
            if j < n_genes {
                gene_sums[j] += v;
            }
        }
    }
    let total = gene_sums.iter().sum::<f64>().max(1e-10);

    let mut residuals = vec![vec![0.0f64; n_genes]; n_cells];
    for (i, row) in mat.iter().enumerate() {
        for j in 0..n_genes {
            let x = row.get(j).copied().unwrap_or(0.0);
            let mu = lib_sizes[i] * gene_sums[j] / total;
            let variance = mu + mu * mu / theta;
            let residual = if variance > 1e-12 {
                (x - mu) / variance.sqrt()
            } else {
                0.0
            };
            residuals[i][j] = residual.clamp(-clip_val, clip_val);
        }
    }

    Ok(matrix_to_value(residuals))
}

// ── sc_integrate(matrix, batch_ids) ──────────────────────────────────
// Simplified batch correction: subtract per-batch per-gene mean.

fn builtin_sc_integrate(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "sc_integrate")?;
    let batch_ids: Vec<i64> = match &args[1] {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Int(n) => *n,
                Value::Float(f) => *f as i64,
                _ => 0,
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "sc_integrate() batch_ids must be List<Int>",
                None,
            ))
        }
    };

    let n_cells = mat.len();
    if n_cells == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let n_genes = mat[0].len();

    if batch_ids.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "sc_integrate(): batch_ids length {} != n_cells {n_cells}",
                batch_ids.len()
            ),
            None,
        ));
    }

    let mut unique_batches: Vec<i64> = batch_ids.clone();
    unique_batches.sort_unstable();
    unique_batches.dedup();

    let mut batch_means: HashMap<i64, Vec<f64>> = HashMap::new();
    for &b in &unique_batches {
        let mut sums = vec![0.0f64; n_genes];
        let mut count = 0usize;
        for (i, row) in mat.iter().enumerate() {
            if batch_ids[i] == b {
                for (j, &v) in row.iter().enumerate() {
                    if j < n_genes {
                        sums[j] += v;
                    }
                }
                count += 1;
            }
        }
        if count > 0 {
            for s in &mut sums {
                *s /= count as f64;
            }
        }
        batch_means.insert(b, sums);
    }

    let corrected: Vec<Vec<f64>> = mat
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let means = batch_means.get(&batch_ids[i]).map(|v| v.as_slice()).unwrap_or(&[]);
            row.iter()
                .enumerate()
                .map(|(j, &v)| v - means.get(j).copied().unwrap_or(0.0))
                .collect()
        })
        .collect();

    Ok(matrix_to_value(corrected))
}

// ── diffusion_pseudotime(embeddings, knn_edges, start_cell) ──────────
// Dijkstra shortest path on the KNN graph from start_cell.

fn builtin_diffusion_pseudotime(args: Vec<Value>) -> Result<Value> {
    let embeddings = require_matrix(&args[0], "diffusion_pseudotime")?;
    let n_cells = embeddings.len();

    let edges: Vec<(usize, usize, f64)> = match &args[1] {
        Value::List(list) => list
            .iter()
            .filter_map(|v| {
                let rec = match v {
                    Value::Record(r) => r,
                    _ => return None,
                };
                let src = match rec.get("source")? {
                    Value::Int(n) => *n as usize,
                    _ => return None,
                };
                let tgt = match rec.get("target")? {
                    Value::Int(n) => *n as usize,
                    _ => return None,
                };
                let dist = match rec.get("distance")? {
                    Value::Float(f) => *f,
                    Value::Int(n) => *n as f64,
                    _ => return None,
                };
                Some((src, tgt, dist))
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "diffusion_pseudotime() knn_edges must be List<Record>",
                None,
            ))
        }
    };

    let start_cell = require_int(&args[2], "diffusion_pseudotime")? as usize;

    if n_cells == 0 {
        return Ok(Value::List((vec![]).into()));
    }
    let start = start_cell.min(n_cells - 1);

    let mut adj: Vec<Vec<(usize, f64)>> = vec![vec![]; n_cells];
    for (src, tgt, d) in edges {
        let w = d.max(1e-10);
        if src < n_cells && tgt < n_cells {
            adj[src].push((tgt, w));
            adj[tgt].push((src, w));
        }
    }

    // Dijkstra with f64 bits as heap key (valid for non-negative distances)
    let mut dist_vec = vec![f64::INFINITY; n_cells];
    dist_vec[start] = 0.0;
    let mut heap: BinaryHeap<(Reverse<u64>, usize)> = BinaryHeap::new();
    heap.push((Reverse(0_f64.to_bits()), start));

    while let Some((Reverse(d_bits), u)) = heap.pop() {
        let d = f64::from_bits(d_bits);
        if d > dist_vec[u] {
            continue;
        }
        for &(v, w) in &adj[u] {
            let nd = d + w;
            if nd < dist_vec[v] {
                dist_vec[v] = nd;
                heap.push((Reverse(nd.to_bits()), v));
            }
        }
    }

    Ok(Value::List(
        dist_vec
            .into_iter()
            .map(|d| Value::Float(if d.is_infinite() { -1.0 } else { d }))
            .collect::<Vec<_>>().into(),
    ))
}

// ── Cell-cell communication ──────────────────────────────────────────

// ── lr_score(matrix, cell_labels, lr_pairs) ──────────────────────────
// matrix: cells × genes (consistent with other builtins)
// cell_labels: one cluster label string per cell
// lr_pairs: List<[ligand_gene_idx, receptor_gene_idx]>
// Returns Table: sender, receiver, ligand_idx, receptor_idx, score (sorted descending)

fn builtin_lr_score(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "lr_score")?;
    let n_cells = mat.len();

    let cell_labels: Vec<String> = match &args[1] {
        Value::List(list) => list
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                other => format!("{other}"),
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "lr_score() cell_labels must be List<Str>",
                None,
            ))
        }
    };

    if cell_labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!(
                "lr_score(): cell_labels length {} != n_cells {n_cells}",
                cell_labels.len()
            ),
            None,
        ));
    }

    let lr_pairs: Vec<(usize, usize)> = match &args[2] {
        Value::List(list) => list
            .iter()
            .enumerate()
            .map(|(i, pair)| match pair {
                Value::List(p) if p.len() >= 2 => {
                    let li = match &p[0] {
                        Value::Int(n) => *n as usize,
                        _ => {
                            return Err(BioLangError::type_error(
                                format!("lr_score() lr_pairs[{i}][0] must be Int"),
                                None,
                            ))
                        }
                    };
                    let ri = match &p[1] {
                        Value::Int(n) => *n as usize,
                        _ => {
                            return Err(BioLangError::type_error(
                                format!("lr_score() lr_pairs[{i}][1] must be Int"),
                                None,
                            ))
                        }
                    };
                    Ok((li, ri))
                }
                _ => Err(BioLangError::type_error(
                    format!("lr_score() lr_pairs[{i}] must be List<Int> of length >= 2"),
                    None,
                )),
            })
            .collect::<Result<Vec<_>>>()?,
        _ => {
            return Err(BioLangError::type_error(
                "lr_score() lr_pairs must be List<List<Int>>",
                None,
            ))
        }
    };

    let columns: Vec<String> = ["sender", "receiver", "ligand_idx", "receptor_idx", "score"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    if n_cells == 0 || lr_pairs.is_empty() {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let n_genes = mat[0].len();

    // Group cell indices by cluster label
    let mut cluster_cells: HashMap<String, Vec<usize>> = HashMap::new();
    for (cell_idx, label) in cell_labels.iter().enumerate() {
        cluster_cells.entry(label.clone()).or_default().push(cell_idx);
    }

    // Compute mean expression per gene for each cluster
    let mut cluster_means: HashMap<String, Vec<f64>> = HashMap::new();
    for (cluster, cells) in &cluster_cells {
        let mut means = vec![0.0f64; n_genes];
        let n = cells.len() as f64;
        for &c in cells {
            for (g, &v) in mat[c].iter().enumerate() {
                if g < n_genes {
                    means[g] += v;
                }
            }
        }
        for m in &mut means {
            *m /= n;
        }
        cluster_means.insert(cluster.clone(), means);
    }

    // Deterministic ordering
    let mut clusters: Vec<String> = cluster_means.keys().cloned().collect();
    clusters.sort();

    // Score every sender × receiver × LR pair with score > 0
    let mut scored: Vec<(f64, Vec<Value>)> = Vec::new();
    for sender in &clusters {
        for receiver in &clusters {
            let s_means = &cluster_means[sender];
            let r_means = &cluster_means[receiver];
            for &(li, ri) in &lr_pairs {
                let l_expr = s_means.get(li).copied().unwrap_or(0.0);
                let r_expr = r_means.get(ri).copied().unwrap_or(0.0);
                let score = l_expr * r_expr;
                if score > 0.0 {
                    scored.push((score, vec![
                        Value::Str(sender.clone()),
                        Value::Str(receiver.clone()),
                        Value::Int(li as i64),
                        Value::Int(ri as i64),
                        Value::Float(score),
                    ]));
                }
            }
        }
    }

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let rows: Vec<Vec<Value>> = scored.into_iter().map(|(_, r)| r).collect();
    Ok(Value::Table(Table::new(columns, rows)))
}

// ── lr_aggregate(lr_scores, pathway_map) ─────────────────────────────
// lr_scores: Table from lr_score() — sender, receiver, ligand_idx, receptor_idx, score
// pathway_map: Table — ligand_idx, receptor_idx, pathway
// Returns Table: sender, receiver, pathway, total_score, n_pairs (sorted descending)

fn builtin_lr_aggregate(args: Vec<Value>) -> Result<Value> {
    let (score_rows, s_col, r_col, li_col, ri_col, sc_col) = match &args[0] {
        Value::Table(t) => {
            let s_col = t.col_index("sender").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'sender'", None)
            })?;
            let r_col = t.col_index("receiver").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'receiver'", None)
            })?;
            let li_col = t.col_index("ligand_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'ligand_idx'", None)
            })?;
            let ri_col = t.col_index("receptor_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'receptor_idx'", None)
            })?;
            let sc_col = t.col_index("score").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() lr_scores missing 'score'", None)
            })?;
            (t.rows.clone(), s_col, r_col, li_col, ri_col, sc_col)
        }
        _ => {
            return Err(BioLangError::type_error(
                "lr_aggregate() first arg must be a Table from lr_score()",
                None,
            ))
        }
    };

    let (pm_rows, pm_li, pm_ri, pm_path) = match &args[1] {
        Value::Table(t) => {
            let pm_li = t.col_index("ligand_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() pathway_map missing 'ligand_idx'", None)
            })?;
            let pm_ri = t.col_index("receptor_idx").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() pathway_map missing 'receptor_idx'", None)
            })?;
            let pm_path = t.col_index("pathway").ok_or_else(|| {
                BioLangError::type_error("lr_aggregate() pathway_map missing 'pathway'", None)
            })?;
            (t.rows.clone(), pm_li, pm_ri, pm_path)
        }
        _ => {
            return Err(BioLangError::type_error(
                "lr_aggregate() second arg must be a Table with ligand_idx, receptor_idx, pathway",
                None,
            ))
        }
    };

    // Build (ligand_idx, receptor_idx) → pathway lookup
    let mut pathway_lookup: HashMap<(i64, i64), String> = HashMap::new();
    for row in &pm_rows {
        let li = match row.get(pm_li) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let ri = match row.get(pm_ri) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let pathway = match row.get(pm_path) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        pathway_lookup.insert((li, ri), pathway);
    }

    // Aggregate (sender, receiver, pathway) → (total_score, n_pairs)
    let mut agg: HashMap<(String, String, String), (f64, usize)> = HashMap::new();
    for row in &score_rows {
        let sender = match row.get(s_col) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        let receiver = match row.get(r_col) {
            Some(Value::Str(s)) => s.clone(),
            _ => continue,
        };
        let li = match row.get(li_col) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let ri = match row.get(ri_col) {
            Some(Value::Int(n)) => *n,
            Some(Value::Float(f)) => *f as i64,
            _ => continue,
        };
        let score = to_f64(row.get(sc_col).unwrap_or(&Value::Float(0.0))).unwrap_or(0.0);
        if let Some(pathway) = pathway_lookup.get(&(li, ri)) {
            let e = agg.entry((sender, receiver, pathway.clone())).or_insert((0.0, 0));
            e.0 += score;
            e.1 += 1;
        }
    }

    let out_columns: Vec<String> = ["sender", "receiver", "pathway", "total_score", "n_pairs"]
        .iter()
        .map(|s| s.to_string())
        .collect();

    let mut result: Vec<(f64, Vec<Value>)> = agg
        .into_iter()
        .map(|((sender, receiver, pathway), (total, n))| {
            (total, vec![
                Value::Str(sender),
                Value::Str(receiver),
                Value::Str(pathway),
                Value::Float(total),
                Value::Int(n as i64),
            ])
        })
        .collect();
    result.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    let rows: Vec<Vec<Value>> = result.into_iter().map(|(_, r)| r).collect();
    Ok(Value::Table(Table::new(out_columns, rows)))
}

// ── Spatial transcriptomics ──────────────────────────────────────────

// ── spatial_neighbors(coords, k=6) ───────────────────────────────────
// coords: Table with columns x, y (one row per spot)
// Returns Table: cell, neighbor, distance (Euclidean), sorted by cell then distance

fn builtin_spatial_neighbors(args: Vec<Value>) -> Result<Value> {
    let (xs, ys) = match &args[0] {
        Value::Table(t) => {
            let x_col = t.col_index("x").ok_or_else(|| {
                BioLangError::type_error("spatial_neighbors() coords must have column 'x'", None)
            })?;
            let y_col = t.col_index("y").ok_or_else(|| {
                BioLangError::type_error("spatial_neighbors() coords must have column 'y'", None)
            })?;
            let xs: Vec<f64> = t.rows.iter().map(|r| to_f64(r.get(x_col).unwrap_or(&Value::Float(0.0))).unwrap_or(0.0)).collect();
            let ys: Vec<f64> = t.rows.iter().map(|r| to_f64(r.get(y_col).unwrap_or(&Value::Float(0.0))).unwrap_or(0.0)).collect();
            (xs, ys)
        }
        _ => return Err(BioLangError::type_error("spatial_neighbors() coords must be Table with x,y columns", None)),
    };

    let k = if args.len() > 1 {
        require_int(&args[1], "spatial_neighbors")? as usize
    } else {
        6
    };

    let n = xs.len();
    let k_actual = k.min(n.saturating_sub(1));
    let columns: Vec<String> = ["cell", "neighbor", "distance"].iter().map(|s| s.to_string()).collect();

    if n == 0 {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let mut rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..n {
        let mut dists: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| {
                let dx = xs[i] - xs[j];
                let dy = ys[i] - ys[j];
                (j, (dx * dx + dy * dy).sqrt())
            })
            .collect();
        dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        for (j, d) in dists.into_iter().take(k_actual) {
            rows.push(vec![Value::Int(i as i64), Value::Int(j as i64), Value::Float(d)]);
        }
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── spatial_moransi(expr_vec, spatial_adj) ───────────────────────────
// expr_vec: List<Float> — gene expression across spots
// spatial_adj: Table with columns cell, neighbor (from spatial_neighbors)
// Returns Float in [-1, 1]: Moran's I spatial autocorrelation

fn builtin_spatial_moransi(args: Vec<Value>) -> Result<Value> {
    let expr: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => return Err(BioLangError::type_error("spatial_moransi() expr_vec must be List<Float>", None)),
    };
    let n = expr.len();

    let mut adj: Vec<Vec<usize>> = vec![vec![]; n];
    let mut total_w: usize = 0;
    match &args[1] {
        Value::Table(t) => {
            let c_col = t.col_index("cell").ok_or_else(|| {
                BioLangError::type_error("spatial_moransi() spatial_adj missing 'cell' column", None)
            })?;
            let nb_col = t.col_index("neighbor").ok_or_else(|| {
                BioLangError::type_error("spatial_moransi() spatial_adj missing 'neighbor' column", None)
            })?;
            for row in &t.rows {
                let c = match row.get(c_col) {
                    Some(Value::Int(i)) => *i as usize,
                    Some(Value::Float(f)) => *f as usize,
                    _ => continue,
                };
                let nb = match row.get(nb_col) {
                    Some(Value::Int(i)) => *i as usize,
                    Some(Value::Float(f)) => *f as usize,
                    _ => continue,
                };
                if c < n && nb < n {
                    adj[c].push(nb);
                    total_w += 1;
                }
            }
        }
        _ => return Err(BioLangError::type_error("spatial_moransi() spatial_adj must be Table from spatial_neighbors()", None)),
    }

    if n == 0 || total_w == 0 {
        return Ok(Value::Float(0.0));
    }

    let mean = expr.iter().sum::<f64>() / n as f64;
    let denom: f64 = expr.iter().map(|&x| (x - mean) * (x - mean)).sum();
    if denom < 1e-12 {
        return Ok(Value::Float(0.0));
    }

    // Binary weights: w_ij = 1 if j in adj[i]; W = total_w
    let mut numer = 0.0f64;
    for i in 0..n {
        let xi = expr[i];
        for &j in &adj[i] {
            numer += (xi - mean) * (expr[j] - mean);
        }
    }

    let i_moran = (n as f64 / total_w as f64) * numer / denom;
    Ok(Value::Float(i_moran.clamp(-1.0, 1.0)))
}

// ── Cell type annotation ─────────────────────────────────────────────

// ── reference_classify(query_matrix, ref_matrix, ref_labels) ─────────
// query_matrix: Table genes × query_cells; ref_matrix: genes × ref_cells
// Returns Table: cell, label (majority-vote over top-5 cosine neighbours), confidence

fn builtin_reference_classify(args: Vec<Value>) -> Result<Value> {
    let q_mat = require_matrix(&args[0], "reference_classify")?; // mat[gene][query_cell]
    let r_mat = require_matrix(&args[1], "reference_classify")?; // mat[gene][ref_cell]
    let ref_labels: Vec<String> = match &args[2] {
        Value::List(list) => list.iter().map(|v| match v {
            Value::Str(s) => s.clone(),
            other => format!("{other}"),
        }).collect(),
        _ => return Err(BioLangError::type_error("reference_classify() ref_labels must be List<Str>", None)),
    };

    let n_genes_q = q_mat.len();
    let n_genes_r = r_mat.len();
    if n_genes_q != n_genes_r {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("reference_classify(): query has {n_genes_q} genes but ref has {n_genes_r}"),
            None,
        ));
    }
    let n_genes = n_genes_q;
    let n_query = if n_genes > 0 { q_mat[0].len() } else { 0 };
    let n_ref = if n_genes > 0 { r_mat[0].len() } else { 0 };
    if ref_labels.len() != n_ref {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!("reference_classify(): ref_labels length {} != n_ref_cells {n_ref}", ref_labels.len()),
            None,
        ));
    }

    let k = 5usize.min(n_ref);
    let columns: Vec<String> = ["cell", "label", "confidence"].iter().map(|s| s.to_string()).collect();
    let mut rows: Vec<Vec<Value>> = Vec::new();

    for qc in 0..n_query {
        let qvec: Vec<f64> = (0..n_genes).map(|g| q_mat[g].get(qc).copied().unwrap_or(0.0)).collect();
        let q_norm: f64 = qvec.iter().map(|&v| v * v).sum::<f64>().sqrt();

        let mut sims: Vec<(usize, f64)> = (0..n_ref).map(|rc| {
            let dot: f64 = (0..n_genes).map(|g| qvec[g] * r_mat[g].get(rc).copied().unwrap_or(0.0)).sum();
            let r_norm: f64 = (0..n_genes).map(|g| {
                let v = r_mat[g].get(rc).copied().unwrap_or(0.0);
                v * v
            }).sum::<f64>().sqrt();
            let sim = if q_norm > 1e-10 && r_norm > 1e-10 { dot / (q_norm * r_norm) } else { 0.0 };
            (rc, sim)
        }).collect();
        sims.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let mut counts: HashMap<String, usize> = HashMap::new();
        for &(rc, _) in sims.iter().take(k) {
            *counts.entry(ref_labels[rc].clone()).or_insert(0) += 1;
        }
        let (best_label, best_count) = counts.into_iter()
            .max_by_key(|(_, c)| *c)
            .unwrap_or_else(|| ("unknown".to_string(), 0));
        let confidence = if k > 0 { best_count as f64 / k as f64 } else { 0.0 };

        rows.push(vec![
            Value::Int(qc as i64),
            Value::Str(best_label),
            Value::Float(confidence),
        ]);
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── Pseudobulk aggregation ───────────────────────────────────────────

// ── pseudobulk_aggregate(matrix, cell_labels, sample_labels) ─────────
// matrix: Table genes × cells; sums counts per (cluster, sample) group
// Returns Table: columns = "cluster__sample", rows = genes

fn builtin_pseudobulk_aggregate(args: Vec<Value>) -> Result<Value> {
    let mat = require_matrix(&args[0], "pseudobulk_aggregate")?; // mat[gene][cell]
    let n_genes = mat.len();
    let n_cells = if n_genes > 0 { mat[0].len() } else { 0 };

    let parse_str_list = |v: &Value, name: &str| -> Result<Vec<String>> {
        match v {
            Value::List(list) => Ok(list.iter().map(|v| match v {
                Value::Str(s) => s.clone(),
                other => format!("{other}"),
            }).collect()),
            _ => Err(BioLangError::type_error(format!("pseudobulk_aggregate() {name} must be List<Str>"), None)),
        }
    };
    let cell_labels = parse_str_list(&args[1], "cell_labels")?;
    let sample_labels = parse_str_list(&args[2], "sample_labels")?;

    if cell_labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!("pseudobulk_aggregate(): cell_labels length {} != n_cells {n_cells}", cell_labels.len()),
            None,
        ));
    }
    if sample_labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::ArityError,
            format!("pseudobulk_aggregate(): sample_labels length {} != n_cells {n_cells}", sample_labels.len()),
            None,
        ));
    }

    // Collect unique groups in deterministic sorted order
    let mut group_keys: Vec<String> = (0..n_cells)
        .map(|c| format!("{}__{}", cell_labels[c], sample_labels[c]))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    group_keys.sort();
    let n_groups = group_keys.len();
    let group_to_col: HashMap<String, usize> = group_keys.iter().enumerate().map(|(i, k)| (k.clone(), i)).collect();

    // Group → cell indices
    let mut group_cells: Vec<Vec<usize>> = vec![vec![]; n_groups];
    for c in 0..n_cells {
        let key = format!("{}__{}", cell_labels[c], sample_labels[c]);
        if let Some(&col) = group_to_col.get(&key) {
            group_cells[col].push(c);
        }
    }

    // Sum counts: one row per gene, one column per group
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_genes);
    for g in 0..n_genes {
        let row: Vec<Value> = (0..n_groups).map(|col| {
            let sum: f64 = group_cells[col].iter()
                .map(|&c| mat[g].get(c).copied().unwrap_or(0.0))
                .sum();
            Value::Float(sum)
        }).collect();
        rows.push(row);
    }

    Ok(Value::Table(Table::new(group_keys, rows)))
}

// ── Multimodal integration ───────────────────────────────────────────

// ── wnn_graph(matrix_a, matrix_b, k) ─────────────────────────────────
// matrix_a, matrix_b: Table cells × dims (rows=cells) for two modalities
// Returns Table: cell, neighbor, weight (α weighted by modality quality)

fn builtin_wnn_graph(args: Vec<Value>) -> Result<Value> {
    let mat_a = require_matrix(&args[0], "wnn_graph")?; // mat[cell][dim]
    let mat_b = require_matrix(&args[1], "wnn_graph")?;
    let k = require_int(&args[2], "wnn_graph")? as usize;

    let n_a = mat_a.len();
    let n_b = mat_b.len();
    if n_a != n_b {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("wnn_graph(): matrix_a has {n_a} cells but matrix_b has {n_b}"),
            None,
        ));
    }
    let n = n_a;
    let k_actual = k.min(n.saturating_sub(1));
    let columns: Vec<String> = ["cell", "neighbor", "weight"].iter().map(|s| s.to_string()).collect();

    if n == 0 || k_actual == 0 {
        return Ok(Value::Table(Table::new(columns, vec![])));
    }

    let euclid = |a: &[f64], b: &[f64]| -> f64 {
        a.iter().zip(b.iter()).map(|(x, y)| (x - y) * (x - y)).sum::<f64>().sqrt()
    };

    // k-NN for each modality
    let knn = |mat: &[Vec<f64>]| -> Vec<Vec<(usize, f64)>> {
        (0..n).map(|i| {
            let mut dists: Vec<(usize, f64)> = (0..n).filter(|&j| j != i)
                .map(|j| (j, euclid(&mat[i], &mat[j])))
                .collect();
            dists.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
            dists.into_iter().take(k_actual).collect()
        }).collect()
    };

    let knn_a = knn(&mat_a);
    let knn_b = knn(&mat_b);

    // Per-cell modality weight α_i
    let alpha: Vec<f64> = (0..n).map(|i| {
        let mean_a = if knn_a[i].is_empty() { 0.0 } else {
            knn_a[i].iter().map(|(_, d)| d).sum::<f64>() / knn_a[i].len() as f64
        };
        let mean_b = if knn_b[i].is_empty() { 0.0 } else {
            knn_b[i].iter().map(|(_, d)| d).sum::<f64>() / knn_b[i].len() as f64
        };
        let ea = (-mean_a).exp();
        let eb = (-mean_b).exp();
        let denom = ea + eb;
        if denom > 1e-12 { ea / denom } else { 0.5 }
    }).collect();

    // Merge edges from both modalities, keep top-k by weight
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for i in 0..n {
        let ai = alpha[i];
        let mut edges: HashMap<usize, f64> = HashMap::new();
        for &(j, _) in &knn_a[i] {
            edges.entry(j).or_insert(ai);
        }
        for &(j, _) in &knn_b[i] {
            edges.entry(j).or_insert(1.0 - ai);
        }
        let mut sorted: Vec<(usize, f64)> = edges.into_iter().collect();
        sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (j, w) in sorted.into_iter().take(k_actual) {
            rows.push(vec![Value::Int(i as i64), Value::Int(j as i64), Value::Float(w)]);
        }
    }

    Ok(Value::Table(Table::new(columns, rows)))
}

// ── RNA velocity ─────────────────────────────────────────────────────

// ── velocity_estimate(spliced, unspliced) ────────────────────────────
// Deterministic model: β_g = mean_spliced_g / (mean_unspliced_g + ε)
// velocity[g][c] = unspliced[g][c] * β_g - spliced[g][c]
// Returns Table same shape as inputs (genes × cells)

fn builtin_velocity_estimate(args: Vec<Value>) -> Result<Value> {
    let spliced = require_matrix(&args[0], "velocity_estimate")?; // mat[gene][cell]
    let unspliced = require_matrix(&args[1], "velocity_estimate")?;

    let n_genes = spliced.len();
    if n_genes != unspliced.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("velocity_estimate(): spliced has {n_genes} genes but unspliced has {}", unspliced.len()),
            None,
        ));
    }
    if n_genes == 0 {
        return Ok(matrix_to_value(vec![]));
    }
    let n_cells = spliced[0].len();
    if n_cells != unspliced[0].len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("velocity_estimate(): spliced has {n_cells} cells but unspliced has {}", unspliced[0].len()),
            None,
        ));
    }

    const EPS: f64 = 1e-6;
    let result: Vec<Vec<f64>> = (0..n_genes).map(|g| {
        let s_row = &spliced[g];
        let u_row = &unspliced[g];
        let mean_s = s_row.iter().sum::<f64>() / n_cells.max(1) as f64;
        let mean_u = u_row.iter().sum::<f64>() / n_cells.max(1) as f64;
        let beta = mean_s / (mean_u + EPS);
        (0..n_cells).map(|c| {
            u_row.get(c).copied().unwrap_or(0.0) * beta
                - s_row.get(c).copied().unwrap_or(0.0)
        }).collect()
    }).collect();

    Ok(matrix_to_value(result))
}

// ── Section 7: CNV / tumour purity ───────────────────────────────────

// ── cnv_segment(log_ratios, min_segment=5) ───────────────────────────

fn builtin_cnv_segment(args: Vec<Value>) -> Result<Value> {
    let log_ratios: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "cnv_segment() requires List<Float>",
                None,
            ))
        }
    };
    let min_segment = if args.len() > 1 {
        require_int(&args[1], "cnv_segment")? as usize
    } else {
        5
    };

    let n = log_ratios.len();
    if n == 0 {
        return Ok(Value::List((vec![]).into()));
    }

    // CBS approximation: sliding window t-test to detect change points
    let window = min_segment.max(3);
    let mut change_points: Vec<usize> = vec![0];

    if n >= 2 * window {
        for i in window..n.saturating_sub(window) {
            let left = &log_ratios[i.saturating_sub(window)..i];
            let right = &log_ratios[i..i + window];

            let mean_l = left.iter().sum::<f64>() / left.len() as f64;
            let mean_r = right.iter().sum::<f64>() / right.len() as f64;

            let var_l = left
                .iter()
                .map(|&x| (x - mean_l) * (x - mean_l))
                .sum::<f64>()
                / (left.len() as f64 + 1e-10);
            let var_r = right
                .iter()
                .map(|&x| (x - mean_r) * (x - mean_r))
                .sum::<f64>()
                / (right.len() as f64 + 1e-10);
            let pooled_se = ((var_l + var_r) / window as f64).sqrt() + 1e-10;
            let t = (mean_r - mean_l).abs() / pooled_se;

            // t ≈ 2.0 ~ p < 0.05 for moderate degrees of freedom
            if t > 2.0 {
                if i - *change_points.last().unwrap() >= min_segment {
                    change_points.push(i);
                }
            }
        }
    }
    change_points.push(n);

    // Build raw segments
    let mut segments: Vec<Value> = Vec::new();
    let mut segment_id: i64 = 0;
    for win in change_points.windows(2) {
        let seg_start = win[0];
        let seg_end = win[1];
        let seg_len = seg_end - seg_start;
        let seg_vals = &log_ratios[seg_start..seg_end];
        let mean_ratio = seg_vals.iter().sum::<f64>() / seg_vals.len().max(1) as f64;

        let mut rec = HashMap::new();
        rec.insert("start".to_string(), Value::Int(seg_start as i64));
        rec.insert("end".to_string(), Value::Int(seg_end as i64));
        rec.insert("n_probes".to_string(), Value::Int(seg_len as i64));
        rec.insert("mean_ratio".to_string(), Value::Float(mean_ratio));
        rec.insert("segment_id".to_string(), Value::Int(segment_id));
        segments.push(Value::Record((rec).into()));
        segment_id += 1;
    }

    // Merge adjacent segments whose means differ by < 0.1 log2
    let mut merged: Vec<HashMap<String, Value>> = Vec::new();
    for seg in segments {
        if let Value::Record(fields) = seg {
            if let Some(last) = merged.last_mut() {
                let last_mean = match last.get("mean_ratio") {
                    Some(Value::Float(f)) => *f,
                    _ => f64::INFINITY,
                };
                let this_mean = match fields.get("mean_ratio") {
                    Some(Value::Float(f)) => *f,
                    _ => f64::INFINITY,
                };
                if (last_mean - this_mean).abs() < 0.1 {
                    let last_start = match last.get("start") {
                        Some(Value::Int(n)) => *n as usize,
                        _ => 0,
                    };
                    let this_end = match fields.get("end") {
                        Some(Value::Int(n)) => *n as usize,
                        _ => 0,
                    };
                    let merged_vals = &log_ratios[last_start..this_end];
                    let merged_mean =
                        merged_vals.iter().sum::<f64>() / merged_vals.len().max(1) as f64;
                    last.insert("end".to_string(), Value::Int(this_end as i64));
                    last.insert("n_probes".to_string(), Value::Int(merged_vals.len() as i64));
                    last.insert("mean_ratio".to_string(), Value::Float(merged_mean));
                    continue;
                }
            }
            merged.push((fields).as_ref().clone());
        }
    }

    // Re-number segment ids
    let result: Vec<Value> = merged
        .into_iter()
        .enumerate()
        .map(|(i, mut fields)| {
            fields.insert("segment_id".to_string(), Value::Int(i as i64));
            Value::Record((fields).into())
        })
        .collect();

    Ok(Value::List((result).into()))
}

// ── loh_detect(het_snp_vafs) ─────────────────────────────────────────

fn builtin_loh_detect(args: Vec<Value>) -> Result<Value> {
    let vafs: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.5)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "loh_detect() requires List<Float>",
                None,
            ))
        }
    };

    let n_snps = vafs.len();
    let empty_rec = || {
        let mut rec = HashMap::new();
        rec.insert("n_snps".to_string(), Value::Int(0));
        rec.insert("n_loh_snps".to_string(), Value::Int(0));
        rec.insert("loh_fraction".to_string(), Value::Float(0.0));
        rec.insert("mean_vaf".to_string(), Value::Float(0.0));
        rec.insert("median_deviation".to_string(), Value::Float(0.0));
        Value::Record((rec).into())
    };

    if n_snps == 0 {
        return Ok(empty_rec());
    }

    let loh_threshold = 0.2; // |VAF - 0.5| > 0.2
    let n_loh = vafs
        .iter()
        .filter(|&&v| (v - 0.5).abs() > loh_threshold)
        .count();
    let loh_fraction = n_loh as f64 / n_snps as f64;
    let mean_vaf = vafs.iter().sum::<f64>() / n_snps as f64;

    let mut deviations: Vec<f64> = vafs.iter().map(|&v| (v - 0.5).abs()).collect();
    deviations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_deviation = if n_snps % 2 == 0 {
        (deviations[n_snps / 2 - 1] + deviations[n_snps / 2]) / 2.0
    } else {
        deviations[n_snps / 2]
    };

    let mut rec = HashMap::new();
    rec.insert("n_snps".to_string(), Value::Int(n_snps as i64));
    rec.insert("n_loh_snps".to_string(), Value::Int(n_loh as i64));
    rec.insert("loh_fraction".to_string(), Value::Float(loh_fraction));
    rec.insert("mean_vaf".to_string(), Value::Float(mean_vaf));
    rec.insert(
        "median_deviation".to_string(),
        Value::Float(median_deviation),
    );
    Ok(Value::Record((rec).into()))
}

// ── tumor_purity(vafs) ───────────────────────────────────────────────

fn builtin_tumor_purity(args: Vec<Value>) -> Result<Value> {
    let vafs: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "tumor_purity() requires List<Float>",
                None,
            ))
        }
    };

    let n_variants = vafs.len();
    if n_variants == 0 {
        let mut rec = HashMap::new();
        rec.insert("estimated_purity".to_string(), Value::Float(0.0));
        rec.insert("dominant_peak_vaf".to_string(), Value::Float(0.0));
        rec.insert("n_variants".to_string(), Value::Int(0));
        return Ok(Value::Record((rec).into()));
    }

    // Histogram with 0.05-width bins over [0, 1]
    let n_bins = 20usize;
    let bin_width = 1.0 / n_bins as f64;
    let mut hist = vec![0usize; n_bins];
    for &v in &vafs {
        let bin = ((v / bin_width) as usize).min(n_bins - 1);
        hist[bin] += 1;
    }

    let (mode_bin, _) = hist
        .iter()
        .enumerate()
        .max_by_key(|(_, &cnt)| cnt)
        .unwrap_or((0, &0));

    let dominant_peak_vaf = (mode_bin as f64 + 0.5) * bin_width;
    // purity ≈ 2 * modal_vaf (diploid, clonal heterozygous variants)
    let estimated_purity = (2.0 * dominant_peak_vaf).clamp(0.0, 1.0);

    let mut rec = HashMap::new();
    rec.insert(
        "estimated_purity".to_string(),
        Value::Float(estimated_purity),
    );
    rec.insert(
        "dominant_peak_vaf".to_string(),
        Value::Float(dominant_peak_vaf),
    );
    rec.insert("n_variants".to_string(), Value::Int(n_variants as i64));
    Ok(Value::Record((rec).into()))
}

// ── vaf_to_ccf(vaf, purity, cn_total=2, cn_minor=0) ─────────────────

fn builtin_vaf_to_ccf(args: Vec<Value>) -> Result<Value> {
    let vaf = to_f64(&args[0])
        .ok_or_else(|| BioLangError::type_error("vaf_to_ccf() vaf must be a number", None))?;
    let purity = to_f64(&args[1])
        .ok_or_else(|| BioLangError::type_error("vaf_to_ccf() purity must be a number", None))?;
    let cn_total = if args.len() > 2 {
        to_f64(&args[2]).unwrap_or(2.0)
    } else {
        2.0
    };
    // cn_minor_of_variant — included for API completeness
    let _cn_minor = if args.len() > 3 {
        to_f64(&args[3]).unwrap_or(0.0)
    } else {
        0.0
    };

    if purity <= 0.0 {
        return Ok(Value::Float(0.0));
    }

    // CCF = vaf * (2*(1-purity) + purity*cn_total) / purity
    let ccf = vaf * (2.0 * (1.0 - purity) + purity * cn_total) / purity;
    Ok(Value::Float(ccf.clamp(0.0, 1.0)))
}

// ── mutational_signature(mut_counts_96) ──────────────────────────────
//
// Simplified 10-signature × 96-context COSMIC SBS matrix.
// NOTE: Values below are APPROXIMATE placeholders for structural correctness.
// Replace columns with official values from cosmic-signatures.cancer.sanger.ac.uk
// (COSMIC v3.3 SBS reference signatures TSV).
//
// Column order: SBS1, SBS2, SBS3, SBS4, SBS5, SBS13, SBS17a, SBS17b, SBS18, SBS40

static COSMIC_SIGS: [[f64; 10]; 96] = [
    [
        0.011, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.009, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.008, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.006, 0.001, 0.003, 0.001, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.014, 0.012, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.011, 0.010, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.013, 0.012, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.009, 0.008, 0.003, 0.001, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.008, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.007, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.007, 0.001, 0.003, 0.001, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.005, 0.001, 0.003, 0.001, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.011, 0.001, 0.003, 0.003, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.009, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.008, 0.001, 0.003, 0.002, 0.008, 0.001, 0.001, 0.001, 0.002, 0.008,
    ],
    [
        0.006, 0.001, 0.003, 0.002, 0.007, 0.001, 0.001, 0.001, 0.002, 0.007,
    ],
    [
        0.001, 0.001, 0.008, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.006, 0.002, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.008, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.005, 0.001, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.010, 0.004, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.008, 0.003, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.010, 0.003, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.006, 0.002, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.006, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.005, 0.002, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.006, 0.001, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.004, 0.001, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.009, 0.005, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.007, 0.004, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.001, 0.009, 0.004, 0.010, 0.001, 0.001, 0.001, 0.003, 0.010,
    ],
    [
        0.001, 0.001, 0.006, 0.003, 0.009, 0.001, 0.001, 0.001, 0.003, 0.009,
    ],
    [
        0.001, 0.050, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.040, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.050, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.035, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.060, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.048, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.060, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.042, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.035, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.028, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.035, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.025, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.042, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.034, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.042, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.030, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.040, 0.050, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.032, 0.040, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.040, 0.050, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.028, 0.035, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.048, 0.060, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.038, 0.048, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.048, 0.060, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.034, 0.042, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.028, 0.035, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.022, 0.028, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.028, 0.035, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.020, 0.025, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.034, 0.042, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.027, 0.034, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.034, 0.042, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.024, 0.030, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.003, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
    [
        0.001, 0.001, 0.003, 0.002, 0.006, 0.001, 0.001, 0.001, 0.002, 0.006,
    ],
    [
        0.001, 0.001, 0.003, 0.001, 0.005, 0.001, 0.001, 0.001, 0.002, 0.005,
    ],
];

const SIG_NAMES: [&str; 10] = [
    "SBS1", "SBS2", "SBS3", "SBS4", "SBS5", "SBS13", "SBS17a", "SBS17b", "SBS18", "SBS40",
];

fn builtin_mutational_signature(args: Vec<Value>) -> Result<Value> {
    let counts: Vec<f64> = match &args[0] {
        Value::List(list) => list.iter().map(|v| to_f64(v).unwrap_or(0.0)).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "mutational_signature() requires List<Int|Float> of 96 counts",
                None,
            ))
        }
    };

    if counts.len() != 96 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "mutational_signature() requires exactly 96 SBS counts, got {}",
                counts.len()
            ),
            None,
        ));
    }

    let total_mutations: f64 = counts.iter().sum();

    let zero_contributions = || -> Vec<Value> {
        SIG_NAMES
            .iter()
            .map(|&name| {
                let mut rec = HashMap::new();
                rec.insert("signature".to_string(), Value::Str(name.to_string()));
                rec.insert("weight".to_string(), Value::Float(0.0));
                Value::Record((rec).into())
            })
            .collect()
    };

    if total_mutations == 0.0 {
        let mut result = HashMap::new();
        result.insert(
            "contributions".to_string(),
            Value::List((zero_contributions()).into()),
        );
        result.insert("r_squared".to_string(), Value::Float(0.0));
        result.insert("total_mutations".to_string(), Value::Float(0.0));
        return Ok(Value::Record((result).into()));
    }

    // Normalize observed counts to proportions
    let obs: Vec<f64> = counts.iter().map(|&c| c / total_mutations).collect();

    const N_SIGS: usize = 10;
    // Non-negative least squares via projected gradient descent
    let mut weights = vec![1.0 / N_SIGS as f64; N_SIGS];
    let learning_rate = 0.01;
    let n_iter = 1000;

    for _ in 0..n_iter {
        // fitted_i = sum_j COSMIC_SIGS[i][j] * weights[j]
        let mut fitted = vec![0.0f64; 96];
        for (j, &w) in weights.iter().enumerate() {
            for i in 0..96 {
                fitted[i] += COSMIC_SIGS[i][j] * w;
            }
        }

        // gradient_j = 2 * sum_i (fitted_i - obs_i) * COSMIC_SIGS[i][j]
        let mut grad = vec![0.0f64; N_SIGS];
        for i in 0..96 {
            let residual = fitted[i] - obs[i];
            for j in 0..N_SIGS {
                grad[j] += 2.0 * residual * COSMIC_SIGS[i][j];
            }
        }

        // Gradient step + project onto non-negative orthant
        for j in 0..N_SIGS {
            weights[j] = (weights[j] - learning_rate * grad[j]).max(0.0);
        }

        // Re-normalise to sum = 1
        let wsum: f64 = weights.iter().sum();
        if wsum > 0.0 {
            for w in &mut weights {
                *w /= wsum;
            }
        }
    }

    // Compute R²
    let mut fitted_final = vec![0.0f64; 96];
    for (j, &w) in weights.iter().enumerate() {
        for i in 0..96 {
            fitted_final[i] += COSMIC_SIGS[i][j] * w;
        }
    }
    let obs_mean = obs.iter().sum::<f64>() / 96.0;
    let ss_tot: f64 = obs.iter().map(|&o| (o - obs_mean) * (o - obs_mean)).sum();
    let ss_res: f64 = obs
        .iter()
        .zip(fitted_final.iter())
        .map(|(&o, &f)| (o - f) * (o - f))
        .sum();
    let r_squared = if ss_tot > 0.0 {
        (1.0 - ss_res / ss_tot).clamp(0.0, 1.0)
    } else {
        1.0
    };

    let contributions: Vec<Value> = SIG_NAMES
        .iter()
        .zip(weights.iter())
        .map(|(&name, &w)| {
            let mut rec = HashMap::new();
            rec.insert("signature".to_string(), Value::Str(name.to_string()));
            rec.insert("weight".to_string(), Value::Float(w));
            Value::Record((rec).into())
        })
        .collect();

    let mut result = HashMap::new();
    result.insert("contributions".to_string(), Value::List((contributions).into()));
    result.insert("r_squared".to_string(), Value::Float(r_squared));
    result.insert("total_mutations".to_string(), Value::Float(total_mutations));
    Ok(Value::Record((result).into()))
}
