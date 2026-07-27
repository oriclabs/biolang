use bl_core::value::Value;
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::HashMap;

// ─── helpers ─────────────────────────────────────────────────────────────────

fn float_list(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|&v| Value::Float(v)).collect::<Vec<_>>().into())
}

fn int_list(vals: &[i64]) -> Value {
    Value::List(vals.iter().map(|&v| Value::Int(v)).collect::<Vec<_>>().into())
}

fn str_list(vals: &[&str]) -> Value {
    Value::List(vals.iter().map(|&v| Value::Str(v.to_string())).collect::<Vec<_>>().into())
}

fn matrix(rows: Vec<Vec<f64>>) -> Value {
    Value::List(rows.into_iter().map(float_list_from_vec).collect::<Vec<_>>().into())
}

fn float_list_from_vec(v: Vec<f64>) -> Value {
    Value::List(v.into_iter().map(Value::Float).collect::<Vec<_>>().into())
}

fn get_float(val: &Value, key: &str) -> f64 {
    match val {
        Value::Record(m) => match m.get(key).unwrap() {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            other => panic!("expected float for {key}, got {other:?}"),
        },
        other => panic!("expected Record, got {other:?}"),
    }
}

fn get_int(val: &Value, key: &str) -> i64 {
    match val {
        Value::Record(m) => match m.get(key).unwrap() {
            Value::Int(n) => *n,
            Value::Float(f) => *f as i64,
            other => panic!("expected int for {key}, got {other:?}"),
        },
        other => panic!("expected Record, got {other:?}"),
    }
}

fn get_str(val: &Value, key: &str) -> String {
    match val {
        Value::Record(m) => match m.get(key).unwrap() {
            Value::Str(s) => s.clone(),
            other => panic!("expected str for {key}, got {other:?}"),
        },
        other => panic!("expected Record, got {other:?}"),
    }
}

fn get_list(val: &Value, key: &str) -> Vec<Value> {
    match val {
        Value::Record(m) => match m.get(key).unwrap() {
            Value::List(items) => items.as_ref().clone(),
            other => panic!("expected List for {key}, got {other:?}"),
        },
        other => panic!("expected Record, got {other:?}"),
    }
}

fn as_float(val: &Value) -> f64 {
    match val {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        other => panic!("expected float, got {other:?}"),
    }
}

fn as_int(val: &Value) -> i64 {
    match val {
        Value::Int(n) => *n,
        Value::Float(f) => *f as i64,
        other => panic!("expected int, got {other:?}"),
    }
}

fn knn_edge(src: i64, tgt: i64, dist: f64) -> Value {
    let mut rec = HashMap::new();
    rec.insert("source".to_string(), Value::Int(src));
    rec.insert("target".to_string(), Value::Int(tgt));
    rec.insert("distance".to_string(), Value::Float(dist));
    Value::Record((rec).into())
}

// ─── read_10x ────────────────────────────────────────────────────────────────

#[test]
fn test_read_10x_missing_directory() {
    let result = call_singlecell_builtin(
        "read_10x",
        vec![Value::Str("/nonexistent/path/to/10x".to_string())],
    );
    assert!(result.is_err(), "should fail on missing directory");
    let msg = result.unwrap_err().message;
    assert!(
        msg.contains("barcodes.tsv"),
        "error should mention barcodes.tsv, got: {msg}"
    );
}

#[test]
fn test_read_10x_wrong_arg_type() {
    let result = call_singlecell_builtin("read_10x", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_read_10x_from_temp_dir() {
    use std::io::Write;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path();

    // Write barcodes.tsv
    let mut f = std::fs::File::create(path.join("barcodes.tsv")).unwrap();
    writeln!(f, "AAACCTGAGAAACCAT-1").unwrap();
    writeln!(f, "AAACCTGAGAAACCGC-1").unwrap();
    writeln!(f, "AAACCTGAGAAACCTA-1").unwrap();

    // Write features.tsv (gene_id \t gene_name \t type)
    let mut f = std::fs::File::create(path.join("features.tsv")).unwrap();
    writeln!(f, "ENSG00000243485\tGENE1\tGene Expression").unwrap();
    writeln!(f, "ENSG00000238009\tGENE2\tGene Expression").unwrap();

    // Write matrix.mtx (2 genes × 3 cells, sparse)
    // Format: %%MatrixMarket matrix coordinate integer general
    //         n_rows n_cols nnz
    //         row col value
    let mut f = std::fs::File::create(path.join("matrix.mtx")).unwrap();
    writeln!(f, "%%MatrixMarket matrix coordinate integer general").unwrap();
    writeln!(f, "2 3 4").unwrap(); // 2 genes, 3 cells, 4 nnz entries
    writeln!(f, "1 1 5").unwrap(); // gene1, cell1 = 5
    writeln!(f, "1 2 3").unwrap(); // gene1, cell2 = 3
    writeln!(f, "2 1 1").unwrap(); // gene2, cell1 = 1
    writeln!(f, "2 3 7").unwrap(); // gene2, cell3 = 7

    let result = call_singlecell_builtin(
        "read_10x",
        vec![Value::Str(path.to_str().unwrap().to_string())],
    );
    assert!(result.is_ok(), "read_10x failed: {:?}", result.err());
    let val = result.unwrap();

    assert_eq!(get_int(&val, "n_cells"), 3);
    assert_eq!(get_int(&val, "n_genes"), 2);

    let barcodes = get_list(&val, "barcodes");
    assert_eq!(barcodes.len(), 3);

    // Default is the gene SYMBOL (features.tsv column 2), matching Seurat's
    // Read10X(gene.column = 2) and scanpy's read_10x_mtx. Everything
    // downstream matches on symbols — notably the "MT-" prefix used for
    // percent-mito — so returning Ensembl IDs here made that silently zero.
    let genes = get_list(&val, "genes");
    assert_eq!(genes.len(), 2);
    match &genes[0] {
        Value::Str(s) => assert_eq!(s, "GENE1"),
        other => panic!("expected gene symbol string, got {other:?}"),
    }

    // gene_column = 1 still gives the Ensembl IDs.
    let by_id = call_singlecell_builtin(
        "read_10x",
        vec![
            Value::Str(path.to_str().unwrap().to_string()),
            Value::Int(1),
        ],
    )
    .expect("read_10x with gene_column = 1");
    match &get_list(&by_id, "genes")[0] {
        Value::Str(s) => assert_eq!(s, "ENSG00000243485"),
        other => panic!("expected gene ID string, got {other:?}"),
    }

    // an out-of-range column is rejected rather than silently falling back
    assert!(call_singlecell_builtin(
        "read_10x",
        vec![
            Value::Str(path.to_str().unwrap().to_string()),
            Value::Int(7),
        ],
    )
    .is_err());

    // Check sparse matrix encoding: cell0 (col1), gene0 (row1) = 5
    let mat = get_list(&val, "matrix");
    let cell0 = match &mat[0] {
        Value::List(row) => row.clone(),
        other => panic!("expected row list, got {other:?}"),
    };
    // cell 0: gene0 = 5, gene1 = 1
    assert!((as_float(&cell0[0]) - 5.0).abs() < 1e-9);
    assert!((as_float(&cell0[1]) - 1.0).abs() < 1e-9);

    // cell 1: gene0 = 3, gene1 = 0
    let cell1 = match &mat[1] {
        Value::List(row) => row.clone(),
        other => panic!("{other:?}"),
    };
    assert!((as_float(&cell1[0]) - 3.0).abs() < 1e-9);
    assert!((as_float(&cell1[1]) - 0.0).abs() < 1e-9);

    // cell 2: gene0 = 0, gene1 = 7
    let cell2 = match &mat[2] {
        Value::List(row) => row.clone(),
        other => panic!("{other:?}"),
    };
    assert!((as_float(&cell2[0]) - 0.0).abs() < 1e-9);
    assert!((as_float(&cell2[1]) - 7.0).abs() < 1e-9);
}

// ─── cell_cycle_score ────────────────────────────────────────────────────────

#[test]
fn test_cell_cycle_score_s_phase() {
    // Gene 0 = S-phase marker (high in cell 0), gene 1 = G2M marker
    let mat = matrix(vec![
        vec![5.0, 0.0, 0.0], // cell 0: high S
        vec![0.0, 5.0, 0.0], // cell 1: high G2M
        vec![0.0, 0.0, 0.1], // cell 2: G1 (low expression)
    ]);
    let s_genes   = int_list(&[0]);    // gene 0
    let g2m_genes = int_list(&[1]);    // gene 1

    let result = call_singlecell_builtin("cell_cycle_score", vec![mat, s_genes, g2m_genes]);
    assert!(result.is_ok());
    let scores = match result.unwrap() {
        Value::List(s) => s,
        other => panic!("expected List, got {other:?}"),
    };
    assert_eq!(scores.len(), 3);

    // Cell 0: S phase
    assert_eq!(get_str(&scores[0], "phase"), "S");
    // Cell 1: G2M phase
    assert_eq!(get_str(&scores[1], "phase"), "G2M");
    // Cell 2: G1 (both scores ≤ 0.1)
    assert_eq!(get_str(&scores[2], "phase"), "G1");
}

#[test]
fn test_cell_cycle_score_empty_gene_sets() {
    let mat = matrix(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let s_genes   = int_list(&[]);
    let g2m_genes = int_list(&[]);

    let result = call_singlecell_builtin("cell_cycle_score", vec![mat, s_genes, g2m_genes]);
    assert!(result.is_ok());
    let scores = match result.unwrap() {
        Value::List(s) => s,
        other => panic!("{other:?}"),
    };
    // Both scores = 0.0 → G1 for all cells
    for s in scores.iter() {
        assert_eq!(get_str(s, "phase"), "G1");
        assert!((get_float(s, "s_score") - 0.0).abs() < 1e-9);
    }
}

#[test]
fn test_cell_cycle_score_fields_present() {
    let mat = matrix(vec![vec![1.0, 0.5]]);
    let s_genes   = int_list(&[0]);
    let g2m_genes = int_list(&[1]);

    let result = call_singlecell_builtin("cell_cycle_score", vec![mat, s_genes, g2m_genes]).unwrap();
    let scores = match result {
        Value::List(s) => s,
        other => panic!("{other:?}"),
    };
    let row = &scores[0];
    // All three fields must be present
    assert!(get_float(row, "s_score") >= 0.0);
    assert!(get_float(row, "g2m_score") >= 0.0);
    let phase = get_str(row, "phase");
    assert!(["S", "G2M", "G1"].contains(&phase.as_str()));
}

// ─── module_score ─────────────────────────────────────────────────────────────

#[test]
fn test_module_score_basic() {
    let mat = matrix(vec![
        vec![2.0, 0.0, 4.0],
        vec![0.0, 0.0, 0.0],
        vec![3.0, 3.0, 3.0],
    ]);
    // Score genes 0 and 2
    let indices = int_list(&[0, 2]);
    let result = call_singlecell_builtin("module_score", vec![mat, indices]).unwrap();
    let scores = match result {
        Value::List(s) => s,
        other => panic!("{other:?}"),
    };
    assert_eq!(scores.len(), 3);
    // Cell 0: mean(2.0, 4.0) = 3.0
    assert!((as_float(&scores[0]) - 3.0).abs() < 1e-9);
    // Cell 1: mean(0.0, 0.0) = 0.0
    assert!((as_float(&scores[1]) - 0.0).abs() < 1e-9);
    // Cell 2: mean(3.0, 3.0) = 3.0
    assert!((as_float(&scores[2]) - 3.0).abs() < 1e-9);
}

#[test]
fn test_module_score_single_gene() {
    let mat = matrix(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let indices = int_list(&[1]); // gene 1 only
    let result = call_singlecell_builtin("module_score", vec![mat, indices]).unwrap();
    let scores = match result {
        Value::List(s) => s,
        other => panic!("{other:?}"),
    };
    assert!((as_float(&scores[0]) - 2.0).abs() < 1e-9);
    assert!((as_float(&scores[1]) - 4.0).abs() < 1e-9);
}

#[test]
fn test_module_score_empty_indices() {
    let mat = matrix(vec![vec![1.0, 2.0]]);
    let indices = int_list(&[]);
    let result = call_singlecell_builtin("module_score", vec![mat, indices]).unwrap();
    let scores = match result {
        Value::List(s) => s,
        other => panic!("{other:?}"),
    };
    assert!((as_float(&scores[0]) - 0.0).abs() < 1e-9);
}

// ─── sc_sctransform ──────────────────────────────────────────────────────────

#[test]
fn test_sc_sctransform_shape_preserved() {
    let mat = matrix(vec![
        vec![10.0, 0.0, 5.0],
        vec![0.0, 20.0, 3.0],
        vec![8.0, 8.0, 8.0],
        vec![2.0, 2.0, 2.0],
    ]);
    let result = call_singlecell_builtin("sc_sctransform", vec![mat]).unwrap();
    let rows = match result {
        Value::List(r) => r,
        other => panic!("{other:?}"),
    };
    assert_eq!(rows.len(), 4, "same number of cells");
    for row in rows.iter() {
        let cols = match row {
            Value::List(c) => c,
            other => panic!("{other:?}"),
        };
        assert_eq!(cols.len(), 3, "same number of genes");
    }
}

#[test]
fn test_sc_sctransform_residuals_clipped() {
    // 100 cells, 1 gene — one cell has enormous count
    let mut rows: Vec<Vec<f64>> = vec![vec![1.0]; 99];
    rows.push(vec![100_000.0]); // last cell huge
    let mat = matrix(rows);
    let n_cells = 100;

    let result = call_singlecell_builtin("sc_sctransform", vec![mat]).unwrap();
    let vals = match result {
        Value::List(r) => r,
        other => panic!("{other:?}"),
    };
    let clip = (n_cells as f64).sqrt();
    for v in vals.iter() {
        let f = match v {
            Value::List(row) => as_float(&row[0]),
            other => panic!("{other:?}"),
        };
        assert!(
            f <= clip + 1e-6 && f >= -clip - 1e-6,
            "residual {f} outside clip range ±{clip}"
        );
    }
}

#[test]
fn test_sc_sctransform_empty() {
    let result = call_singlecell_builtin("sc_sctransform", vec![matrix(vec![])]).unwrap();
    assert_eq!(result, Value::List((vec![]).into()));
}

#[test]
fn test_sc_sctransform_zero_matrix() {
    // All-zero input → all-zero residuals (mu = 0, residual = 0)
    let mat = matrix(vec![vec![0.0, 0.0], vec![0.0, 0.0]]);
    let result = call_singlecell_builtin("sc_sctransform", vec![mat]).unwrap();
    let rows = match result {
        Value::List(r) => r,
        other => panic!("{other:?}"),
    };
    for row in rows.iter() {
        for v in match row {
            Value::List(c) => c.iter(),
            _ => panic!(),
        } {
            assert!((as_float(v) - 0.0).abs() < 1e-9);
        }
    }
}

// ─── sc_integrate ─────────────────────────────────────────────────────────────

#[test]
fn test_sc_integrate_removes_batch_mean() {
    // Batch 0: mean gene0 = 2.0, mean gene1 = 4.0
    // Batch 1: mean gene0 = 10.0, mean gene1 = 20.0
    let mat = matrix(vec![
        vec![1.0, 3.0],  // batch 0, cell 0
        vec![3.0, 5.0],  // batch 0, cell 1
        vec![8.0, 18.0], // batch 1, cell 2
        vec![12.0, 22.0],// batch 1, cell 3
    ]);
    let batch_ids = int_list(&[0, 0, 1, 1]);

    let result = call_singlecell_builtin("sc_integrate", vec![mat, batch_ids]).unwrap();
    let rows = match result {
        Value::List(r) => r,
        other => panic!("{other:?}"),
    };

    // Batch 0 mean: gene0 = 2.0, gene1 = 4.0
    // Cell 0 corrected: [1-2, 3-4] = [-1, -1]
    let cell0 = match &rows[0] {
        Value::List(c) => c.clone(),
        other => panic!("{other:?}"),
    };
    assert!((as_float(&cell0[0]) - (-1.0)).abs() < 1e-9);
    assert!((as_float(&cell0[1]) - (-1.0)).abs() < 1e-9);

    // Batch 1 mean: gene0 = 10.0, gene1 = 20.0
    // Cell 2 corrected: [8-10, 18-20] = [-2, -2]
    let cell2 = match &rows[2] {
        Value::List(c) => c.clone(),
        other => panic!("{other:?}"),
    };
    assert!((as_float(&cell2[0]) - (-2.0)).abs() < 1e-9);
    assert!((as_float(&cell2[1]) - (-2.0)).abs() < 1e-9);
}

#[test]
fn test_sc_integrate_single_batch() {
    // Single batch → subtract mean → centered output
    let mat = matrix(vec![
        vec![2.0, 4.0],
        vec![4.0, 8.0],
    ]);
    let batch_ids = int_list(&[0, 0]);

    let result = call_singlecell_builtin("sc_integrate", vec![mat, batch_ids]).unwrap();
    let rows = match result {
        Value::List(r) => r,
        other => panic!("{other:?}"),
    };
    // Mean: gene0 = 3.0, gene1 = 6.0
    // Cell 0: [2-3, 4-6] = [-1, -2]
    let cell0 = match &rows[0] {
        Value::List(c) => c.clone(),
        _ => panic!(),
    };
    assert!((as_float(&cell0[0]) - (-1.0)).abs() < 1e-9);
    assert!((as_float(&cell0[1]) - (-2.0)).abs() < 1e-9);
}

#[test]
fn test_sc_integrate_batch_length_mismatch() {
    let mat = matrix(vec![vec![1.0], vec![2.0]]);
    let batch_ids = int_list(&[0]); // only 1 label for 2 cells
    let result = call_singlecell_builtin("sc_integrate", vec![mat, batch_ids]);
    assert!(result.is_err(), "should fail on length mismatch");
}

#[test]
fn test_sc_integrate_empty() {
    let result =
        call_singlecell_builtin("sc_integrate", vec![matrix(vec![]), int_list(&[])]).unwrap();
    assert_eq!(result, Value::List((vec![]).into()));
}

// ─── diffusion_pseudotime ─────────────────────────────────────────────────────

fn make_edges(pairs: &[(i64, i64, f64)]) -> Value {
    Value::List(pairs.iter().map(|&(s, t, d)| knn_edge(s, t, d)).collect::<Vec<_>>().into())
}

#[test]
fn test_diffusion_pseudotime_linear_chain() {
    // 5 cells in a chain: 0—1—2—3—4 with unit weights
    let embeddings = matrix(vec![
        vec![0.0], vec![1.0], vec![2.0], vec![3.0], vec![4.0],
    ]);
    let edges = make_edges(&[
        (0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0),
    ]);
    let start_cell = Value::Int(0);

    let result =
        call_singlecell_builtin("diffusion_pseudotime", vec![embeddings, edges, start_cell])
            .unwrap();

    let pt = match result {
        Value::List(v) => v.iter().map(as_float).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert_eq!(pt.len(), 5);
    assert!((pt[0] - 0.0).abs() < 1e-9, "start cell = 0");
    assert!((pt[1] - 1.0).abs() < 1e-9, "cell 1 = 1.0");
    assert!((pt[2] - 2.0).abs() < 1e-9, "cell 2 = 2.0");
    assert!((pt[3] - 3.0).abs() < 1e-9, "cell 3 = 3.0");
    assert!((pt[4] - 4.0).abs() < 1e-9, "cell 4 = 4.0");
}

#[test]
fn test_diffusion_pseudotime_shortest_path() {
    // 4-cell diamond: 0→1 (cost 10), 0→2 (cost 1), 2→3 (cost 1), 1→3 (cost 1)
    // Shortest from 0 to 3: 0→2→3 (cost 2)
    let embeddings = matrix(vec![
        vec![0.0, 0.0], vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0],
    ]);
    let edges = make_edges(&[
        (0, 1, 10.0), (0, 2, 1.0), (2, 3, 1.0), (1, 3, 1.0),
    ]);
    let result = call_singlecell_builtin(
        "diffusion_pseudotime",
        vec![embeddings, edges, Value::Int(0)],
    )
    .unwrap();

    let pt = match result {
        Value::List(v) => v.iter().map(as_float).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert!((pt[0] - 0.0).abs() < 1e-9, "start = 0");
    assert!((pt[3] - 2.0).abs() < 1e-9, "cell 3 via 0→2→3 = 2.0");
    assert!(pt[1] < pt[3] + 10.0 + 1e-6, "cell 1 ≤ 10+1 from start");
}

#[test]
fn test_diffusion_pseudotime_unreachable_cell() {
    // Cells 0, 1 connected; cells 2, 3 are isolated
    let embeddings = matrix(vec![
        vec![0.0], vec![1.0], vec![5.0], vec![6.0],
    ]);
    let edges = make_edges(&[(0, 1, 1.0)]);
    let result = call_singlecell_builtin(
        "diffusion_pseudotime",
        vec![embeddings, edges, Value::Int(0)],
    )
    .unwrap();

    let pt = match result {
        Value::List(v) => v.iter().map(as_float).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert!(pt[2] < 0.0, "cell 2 unreachable → -1");
    assert!(pt[3] < 0.0, "cell 3 unreachable → -1");
    assert!((pt[0] - 0.0).abs() < 1e-9);
    assert!((pt[1] - 1.0).abs() < 1e-9);
}

#[test]
fn test_diffusion_pseudotime_empty_cells() {
    let embeddings = matrix(vec![]);
    let edges = make_edges(&[]);
    let result =
        call_singlecell_builtin("diffusion_pseudotime", vec![embeddings, edges, Value::Int(0)])
            .unwrap();
    assert_eq!(result, Value::List((vec![]).into()));
}

#[test]
fn test_diffusion_pseudotime_start_clamped_to_last() {
    // 3 cells, start_cell = 99 → clamped to cell 2
    let embeddings = matrix(vec![vec![0.0], vec![1.0], vec![2.0]]);
    let edges = make_edges(&[(0, 1, 1.0), (1, 2, 1.0)]);
    let result = call_singlecell_builtin(
        "diffusion_pseudotime",
        vec![embeddings, edges, Value::Int(99)],
    )
    .unwrap();
    let pt = match result {
        Value::List(v) => v.iter().map(as_float).collect::<Vec<_>>(),
        other => panic!("{other:?}"),
    };
    assert!((pt[2] - 0.0).abs() < 1e-9, "start (clamped) = cell 2");
    assert!((pt[1] - 1.0).abs() < 1e-9);
    assert!((pt[0] - 2.0).abs() < 1e-9);
}

// ─── lr_score ────────────────────────────────────────────────────────────────

fn lr_pair(li: i64, ri: i64) -> Value {
    Value::List((vec![Value::Int(li), Value::Int(ri)]).into())
}

#[test]
fn test_lr_score_basic() {
    // 3 genes × 4 cells, 2 clusters A (cells 0,1) and B (cells 2,3)
    // mat[cell][gene]: cells × genes
    // Cluster A: gene0 mean=2.0, gene2 mean=3.0
    // Cluster B: gene0 mean=0.5, gene2 mean=4.0
    let mat = matrix(vec![
        vec![1.0, 0.0, 2.0], // cell 0, cluster A
        vec![3.0, 0.0, 4.0], // cell 1, cluster A
        vec![0.0, 1.0, 3.0], // cell 2, cluster B
        vec![1.0, 0.0, 5.0], // cell 3, cluster B
    ]);
    let labels = str_list(&["A", "A", "B", "B"]);
    // LR pair: gene 0 (ligand) → gene 2 (receptor)
    let lr_pairs = Value::List((vec![lr_pair(0, 2)]).into());

    let result = call_singlecell_builtin("lr_score", vec![mat, labels, lr_pairs]);
    assert!(result.is_ok(), "lr_score failed: {:?}", result.err());

    let table = match result.unwrap() {
        Value::Table(t) => t,
        other => panic!("expected Table, got {other:?}"),
    };

    // Must have rows with score > 0
    assert!(!table.rows.is_empty(), "expected scored interactions");

    // Check column names
    assert!(table.col_index("sender").is_some());
    assert!(table.col_index("receiver").is_some());
    assert!(table.col_index("ligand_idx").is_some());
    assert!(table.col_index("receptor_idx").is_some());
    assert!(table.col_index("score").is_some());

    let score_col = table.col_index("score").unwrap();
    // All scores should be positive (zero-score rows are filtered)
    for row in &table.rows {
        let s = match &row[score_col] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            other => panic!("score not a number: {other:?}"),
        };
        assert!(s > 0.0, "expected positive score, got {s}");
    }

    // Rows should be sorted descending by score
    let scores: Vec<f64> = table.rows.iter().map(|r| match &r[score_col] {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => 0.0,
    }).collect();
    for window in scores.windows(2) {
        assert!(window[0] >= window[1], "rows not sorted descending: {:?}", scores);
    }
}

#[test]
fn test_lr_score_all_zero_returns_empty() {
    let mat = matrix(vec![
        vec![0.0, 0.0],
        vec![0.0, 0.0],
    ]);
    let labels = str_list(&["A", "A"]);
    let lr_pairs = Value::List((vec![lr_pair(0, 1)]).into());

    let result = call_singlecell_builtin("lr_score", vec![mat, labels, lr_pairs]).unwrap();
    let table = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    // All-zero matrix → score = 0*0 = 0 → filtered out → empty table
    assert_eq!(table.rows.len(), 0, "expected empty table for zero matrix");
}

#[test]
fn test_lr_score_label_mismatch_errors() {
    let mat = matrix(vec![vec![1.0], vec![2.0]]);
    let labels = str_list(&["A"]); // only 1 label for 2 cells
    let lr_pairs = Value::List((vec![lr_pair(0, 0)]).into());
    let result = call_singlecell_builtin("lr_score", vec![mat, labels, lr_pairs]);
    assert!(result.is_err(), "should fail on label/cell count mismatch");
}

#[test]
fn test_lr_score_empty_pairs() {
    let mat = matrix(vec![vec![1.0, 2.0], vec![3.0, 4.0]]);
    let labels = str_list(&["A", "B"]);
    let lr_pairs = Value::List((vec![]).into()); // no pairs

    let result = call_singlecell_builtin("lr_score", vec![mat, labels, lr_pairs]).unwrap();
    let table = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    assert_eq!(table.rows.len(), 0);
}

// ─── lr_aggregate ────────────────────────────────────────────────────────────

fn make_score_table() -> Value {
    use bl_core::value::Table;
    // Table: sender, receiver, ligand_idx, receptor_idx, score
    let columns = vec![
        "sender".to_string(), "receiver".to_string(),
        "ligand_idx".to_string(), "receptor_idx".to_string(),
        "score".to_string(),
    ];
    let rows = vec![
        vec![Value::Str("A".into()), Value::Str("B".into()), Value::Int(0), Value::Int(1), Value::Float(3.0)],
        vec![Value::Str("A".into()), Value::Str("B".into()), Value::Int(2), Value::Int(3), Value::Float(1.5)],
        vec![Value::Str("B".into()), Value::Str("A".into()), Value::Int(0), Value::Int(1), Value::Float(2.0)],
    ];
    Value::Table(Table::new(columns, rows))
}

fn make_pathway_table() -> Value {
    use bl_core::value::Table;
    // pathway_map: ligand_idx, receptor_idx, pathway
    let columns = vec![
        "ligand_idx".to_string(), "receptor_idx".to_string(), "pathway".to_string(),
    ];
    let rows = vec![
        vec![Value::Int(0), Value::Int(1), Value::Str("VEGF".into())],
        vec![Value::Int(2), Value::Int(3), Value::Str("EGF".into())],
    ];
    Value::Table(Table::new(columns, rows))
}

#[test]
fn test_lr_aggregate_basic() {
    let scores = make_score_table();
    let pathway_map = make_pathway_table();

    let result = call_singlecell_builtin("lr_aggregate", vec![scores, pathway_map]);
    assert!(result.is_ok(), "lr_aggregate failed: {:?}", result.err());

    let table = match result.unwrap() {
        Value::Table(t) => t,
        other => panic!("expected Table, got {other:?}"),
    };

    assert!(table.col_index("sender").is_some());
    assert!(table.col_index("receiver").is_some());
    assert!(table.col_index("pathway").is_some());
    assert!(table.col_index("total_score").is_some());
    assert!(table.col_index("n_pairs").is_some());

    // A→B: VEGF score=3.0, EGF score=1.5; B→A: VEGF score=2.0
    // Should have 3 output rows
    assert_eq!(table.rows.len(), 3, "expected 3 aggregated rows");

    // First row (highest total_score) should be A→B via VEGF (score 3.0)
    let ts_col = table.col_index("total_score").unwrap();
    let top_score = match &table.rows[0][ts_col] {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        other => panic!("{other:?}"),
    };
    assert!((top_score - 3.0).abs() < 1e-9, "top score should be 3.0, got {top_score}");
}

#[test]
fn test_lr_aggregate_no_pathway_match() {
    // Score table references pair (5, 6) which is not in pathway_map → empty output
    use bl_core::value::Table;
    let score_cols = vec![
        "sender".to_string(), "receiver".to_string(),
        "ligand_idx".to_string(), "receptor_idx".to_string(), "score".to_string(),
    ];
    let score_rows = vec![
        vec![Value::Str("A".into()), Value::Str("B".into()), Value::Int(5), Value::Int(6), Value::Float(2.0)],
    ];
    let scores = Value::Table(Table::new(score_cols, score_rows));
    let pathway_map = make_pathway_table(); // only knows about (0,1) and (2,3)

    let result = call_singlecell_builtin("lr_aggregate", vec![scores, pathway_map]).unwrap();
    let table = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    assert_eq!(table.rows.len(), 0, "expected empty table when no pathway matches");
}

// ─── integration: existing builtins still work ────────────────────────────────

#[test]
fn test_normalize_total_unchanged() {
    let mat = matrix(vec![vec![1.0, 1.0], vec![2.0, 2.0]]);
    let result = call_singlecell_builtin("normalize_total", vec![mat]).unwrap();
    let rows = match result {
        Value::List(r) => r,
        other => panic!("{other:?}"),
    };
    // Row 0: total=2 → each value = 10000/2 = 5000
    let row0 = match &rows[0] {
        Value::List(c) => c.clone(),
        _ => panic!(),
    };
    assert!((as_float(&row0[0]) - 5000.0).abs() < 1e-6);
}

#[test]
fn test_module_score_wrong_arg() {
    let result = call_singlecell_builtin(
        "module_score",
        vec![
            matrix(vec![vec![1.0]]),
            Value::Str("bad".to_string()),
        ],
    );
    assert!(result.is_err());
}

#[test]
fn test_cell_cycle_score_wrong_matrix_arg() {
    let result = call_singlecell_builtin(
        "cell_cycle_score",
        vec![
            Value::Str("not_a_matrix".to_string()),
            int_list(&[0]),
            int_list(&[1]),
        ],
    );
    assert!(result.is_err());
}

// ─── spatial_neighbors ───────────────────────────────────────────────────────

#[test]
fn test_spatial_neighbors_basic() {
    use bl_core::value::Table;
    // 3 spots in a line: 0,0  1,0  3,0
    let cols = vec!["x".to_string(), "y".to_string()];
    let rows = vec![
        vec![Value::Float(0.0), Value::Float(0.0)],
        vec![Value::Float(1.0), Value::Float(0.0)],
        vec![Value::Float(3.0), Value::Float(0.0)],
    ];
    let coords = Value::Table(Table::new(cols, rows));
    let result = call_singlecell_builtin("spatial_neighbors", vec![coords, Value::Int(2)]).unwrap();
    let t = match result { Value::Table(t) => t, other => panic!("{other:?}") };
    // Each spot should have 2 neighbors (k=2, n-1=2)
    assert_eq!(t.rows.len(), 6, "3 spots × 2 neighbors = 6 rows");
    // Spot 0's nearest neighbor should be spot 1 (distance 1.0)
    let spot0: Vec<&Vec<Value>> = t.rows.iter().filter(|r| r[0] == Value::Int(0)).collect();
    assert_eq!(spot0.len(), 2);
    let neighbor_of_0: i64 = match spot0[0][1] { Value::Int(n) => n, _ => panic!() };
    assert_eq!(neighbor_of_0, 1, "spot 0's nearest neighbor should be spot 1");
}

#[test]
fn test_spatial_neighbors_empty_coords() {
    use bl_core::value::Table;
    let coords = Value::Table(Table::new(vec!["x".to_string(), "y".to_string()], vec![]));
    let result = call_singlecell_builtin("spatial_neighbors", vec![coords]).unwrap();
    let t = match result { Value::Table(t) => t, other => panic!("{other:?}") };
    assert_eq!(t.rows.len(), 0);
}

// ─── spatial_moransi ─────────────────────────────────────────────────────────

#[test]
fn test_spatial_moransi_perfectly_correlated() {
    use bl_core::value::Table;
    // 4 spots in a row with linearly increasing expression — high positive Moran's I
    let cols = vec!["x".to_string(), "y".to_string()];
    let spot_rows = vec![
        vec![Value::Float(0.0), Value::Float(0.0)],
        vec![Value::Float(1.0), Value::Float(0.0)],
        vec![Value::Float(2.0), Value::Float(0.0)],
        vec![Value::Float(3.0), Value::Float(0.0)],
    ];
    let coords = Value::Table(Table::new(cols, spot_rows));
    let neighbors =
        call_singlecell_builtin("spatial_neighbors", vec![coords, Value::Int(1)]).unwrap();
    // Expression perfectly matches spatial position
    let expr = float_list(&[1.0, 2.0, 3.0, 4.0]);
    let result = call_singlecell_builtin("spatial_moransi", vec![expr, neighbors]).unwrap();
    let i = match result { Value::Float(f) => f, other => panic!("{other:?}") };
    assert!(i > 0.3, "expected positive Moran's I for spatially correlated expression, got {i}");
}

#[test]
fn test_spatial_moransi_constant_expression_returns_zero() {
    use bl_core::value::Table;
    let cols = vec!["x".to_string(), "y".to_string()];
    let spot_rows = vec![
        vec![Value::Float(0.0), Value::Float(0.0)],
        vec![Value::Float(1.0), Value::Float(0.0)],
    ];
    let coords = Value::Table(Table::new(cols, spot_rows));
    let neighbors =
        call_singlecell_builtin("spatial_neighbors", vec![coords, Value::Int(1)]).unwrap();
    let expr = float_list(&[5.0, 5.0]); // constant → denom = 0
    let result = call_singlecell_builtin("spatial_moransi", vec![expr, neighbors]).unwrap();
    let i = match result { Value::Float(f) => f, other => panic!("{other:?}") };
    assert_eq!(i, 0.0);
}

// ─── reference_classify ──────────────────────────────────────────────────────

#[test]
fn test_reference_classify_basic() {
    // 2 genes, 3 ref cells (labels A, A, B), 1 query cell identical to ref[0]
    // query = ref[0] → should be classified as A with confidence 1.0 (top-5 capped to n_ref=3, 2/3 > 1/3)
    let ref_mat = matrix(vec![
        vec![1.0, 2.0, 0.0], // gene 0: ref cells 0,1,2
        vec![0.0, 0.0, 1.0], // gene 1
    ]);
    let query_mat = matrix(vec![
        vec![1.0], // gene 0, query cell 0 = ref cell 0
        vec![0.0], // gene 1
    ]);
    let labels = str_list(&["A", "A", "B"]);
    let result = call_singlecell_builtin("reference_classify", vec![query_mat, ref_mat, labels]).unwrap();
    let t = match result { Value::Table(t) => t, other => panic!("{other:?}") };
    assert_eq!(t.rows.len(), 1);
    let label = match &t.rows[0][1] { Value::Str(s) => s.clone(), other => panic!("{other:?}") };
    assert_eq!(label, "A");
    let conf = match &t.rows[0][2] { Value::Float(f) => *f, other => panic!("{other:?}") };
    assert!(conf > 0.0 && conf <= 1.0);
}

// ─── pseudobulk_aggregate ────────────────────────────────────────────────────

#[test]
fn test_pseudobulk_aggregate_basic() {
    // 2 genes, 4 cells: 2 from cluster A sample S1, 1 from A/S2, 1 from B/S1
    let mat = matrix(vec![
        vec![1.0, 3.0, 5.0, 0.0], // gene 0
        vec![2.0, 4.0, 0.0, 6.0], // gene 1
    ]);
    let cell_labels = str_list(&["A", "A", "A", "B"]);
    let sample_labels = str_list(&["S1", "S1", "S2", "S1"]);
    let result = call_singlecell_builtin(
        "pseudobulk_aggregate",
        vec![mat, cell_labels, sample_labels],
    ).unwrap();
    let t = match result { Value::Table(t) => t, other => panic!("{other:?}") };
    // Columns should be: A__S1, A__S2, B__S1 (sorted)
    assert_eq!(t.columns, vec!["A__S1", "A__S2", "B__S1"]);
    assert_eq!(t.rows.len(), 2, "one row per gene");
    // gene 0: A__S1 = 1+3=4, A__S2 = 5, B__S1 = 0
    let a_s1_idx = t.col_index("A__S1").unwrap();
    let sum_g0_a_s1 = match &t.rows[0][a_s1_idx] { Value::Float(f) => *f, other => panic!("{other:?}") };
    assert!((sum_g0_a_s1 - 4.0).abs() < 1e-9, "A__S1 sum for gene0 should be 4, got {sum_g0_a_s1}");
}

// ─── wnn_graph ───────────────────────────────────────────────────────────────

#[test]
fn test_wnn_graph_basic() {
    // 3 cells × 2 dims each; modality A perfectly separates all cells; modality B is noise
    let mat_a = matrix(vec![
        vec![0.0, 0.0],
        vec![10.0, 0.0],
        vec![20.0, 0.0],
    ]);
    let mat_b = matrix(vec![
        vec![1.0, 0.0],
        vec![1.1, 0.0],
        vec![1.2, 0.0],
    ]);
    let result = call_singlecell_builtin("wnn_graph", vec![mat_a, mat_b, Value::Int(2)]).unwrap();
    let t = match result { Value::Table(t) => t, other => panic!("{other:?}") };
    // 3 cells × up to 2 neighbors = up to 6 edges
    assert!(t.rows.len() > 0, "expected edges in WNN graph");
    // All weights should be in (0, 1]
    for row in &t.rows {
        let w = match &row[2] { Value::Float(f) => *f, other => panic!("{other:?}") };
        assert!(w > 0.0 && w <= 1.0, "edge weight {w} out of range (0, 1]");
    }
}

// ─── velocity_estimate ───────────────────────────────────────────────────────

#[test]
fn test_velocity_estimate_basic() {
    // 2 genes, 2 cells; beta_g = mean_spliced / (mean_unspliced + eps)
    // velocity[g][c] = unspliced[g][c] * beta_g - spliced[g][c]
    let spliced = matrix(vec![
        vec![2.0, 4.0], // gene 0: mean_s=3.0
        vec![1.0, 1.0], // gene 1: mean_s=1.0
    ]);
    let unspliced = matrix(vec![
        vec![1.0, 1.0], // gene 0: mean_u=1.0 → beta=3.0
        vec![2.0, 2.0], // gene 1: mean_u=2.0 → beta=0.5
    ]);
    let result = call_singlecell_builtin("velocity_estimate", vec![spliced, unspliced]).unwrap();
    // gene 0 cell 0: 1.0 * 3.0 - 2.0 = 1.0
    // gene 0 cell 1: 1.0 * 3.0 - 4.0 = -1.0
    let rows = match result {
        Value::List(r) => r,
        other => panic!("{other:?}"),
    };
    assert_eq!(rows.len(), 2, "should have 2 gene rows");
    let g0 = match &rows[0] {
        Value::List(cells) => cells.clone(),
        other => panic!("{other:?}"),
    };
    let v00 = match &g0[0] { Value::Float(f) => *f, other => panic!("{other:?}") };
    let v01 = match &g0[1] { Value::Float(f) => *f, other => panic!("{other:?}") };
    assert!((v00 - 1.0).abs() < 1e-4, "velocity[0][0] should be ~1.0, got {v00}");
    assert!((v01 - (-1.0)).abs() < 1e-4, "velocity[0][1] should be ~-1.0, got {v01}");
}
