use bl_core::sparse_matrix::SparseMatrix;
use bl_core::value::{Table, Value};
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::HashMap;

// ─── helpers ─────────────────────────────────────────────────────────────────

fn float_list(vals: &[f64]) -> Value {
    Value::List(
        vals.iter()
            .map(|&v| Value::Float(v))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn int_list(vals: &[i64]) -> Value {
    Value::List(
        vals.iter()
            .map(|&v| Value::Int(v))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn str_list(vals: &[&str]) -> Value {
    Value::List(
        vals.iter()
            .map(|&v| Value::Str(v.to_string()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn matrix(rows: Vec<Vec<f64>>) -> Value {
    Value::List(
        rows.into_iter()
            .map(float_list_from_vec)
            .collect::<Vec<_>>()
            .into(),
    )
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

fn sparse_matrix(rows: Vec<Vec<f64>>) -> Value {
    Value::SparseMatrix(std::sync::Arc::new(SparseMatrix::from_dense(&rows)))
}

fn sparse_single_cell_object(counts: Vec<Vec<f64>>, genes: &[&str], barcodes: &[&str]) -> Value {
    let matrix = sparse_matrix(counts);
    let obs = Value::Table(Table::new(
        vec!["barcode".to_string()],
        barcodes
            .iter()
            .map(|barcode| vec![Value::Str((*barcode).to_string())])
            .collect(),
    ));
    let var = Value::Table(Table::new(
        vec!["gene".to_string()],
        genes
            .iter()
            .map(|gene| vec![Value::Str((*gene).to_string())])
            .collect(),
    ));
    let mut layers = HashMap::new();
    layers.insert("counts".to_string(), matrix.clone());
    let mut object = HashMap::new();
    object.insert("matrix".to_string(), matrix);
    object.insert("genes".to_string(), str_list(genes));
    object.insert("barcodes".to_string(), str_list(barcodes));
    object.insert("obs".to_string(), obs);
    object.insert("var".to_string(), var);
    object.insert("layers".to_string(), Value::Record(layers.into()));
    object.insert("n_cells".to_string(), Value::Int(barcodes.len() as i64));
    object.insert("n_genes".to_string(), Value::Int(genes.len() as i64));
    Value::Record(object.into())
}

#[test]
fn sparse_object_merge_keeps_layers_annotations_and_batches_in_sync() {
    let left = sparse_single_cell_object(
        vec![vec![2.0, 0.0], vec![0.0, 3.0]],
        &["A", "B"],
        &["L1", "L2"],
    );
    let right = sparse_single_cell_object(vec![vec![4.0, 1.0]], &["A", "B"], &["R1"]);

    let merged = call_singlecell_builtin(
        "sc_merge_objects",
        vec![
            left,
            right,
            Value::Str("sample-a".into()),
            Value::Str("sample-b".into()),
        ],
    )
    .unwrap();

    assert_eq!(get_int(&merged, "n_cells"), 3);
    assert_eq!(get_list(&merged, "barcodes").len(), 3);
    assert_eq!(get_list(&merged, "batch_ids").len(), 3);
    match &merged {
        Value::Record(object) => {
            assert!(matches!(
                object.get("matrix"),
                Some(Value::SparseMatrix(matrix)) if matrix.nrow == 3 && matrix.ncol == 2
            ));
            assert!(matches!(
                object.get("obs"),
                Some(Value::Table(table)) if table.rows.len() == 3
            ));
            assert!(matches!(
                object.get("layers"),
                Some(Value::Record(layers))
                    if matches!(
                        layers.get("counts"),
                        Some(Value::SparseMatrix(matrix)) if matrix.nrow == 3
                    )
            ));
        }
        other => panic!("expected Record, got {other:?}"),
    }
}

#[test]
fn sparse_preprocessing_preserves_sparsity() {
    let counts = sparse_matrix(vec![
        vec![1.0, 0.0, 3.0],
        vec![0.0, 2.0, 0.0],
        vec![4.0, 0.0, 1.0],
    ]);
    let normalized =
        call_singlecell_builtin("normalize_total", vec![counts.clone(), Value::Float(10.0)])
            .unwrap();
    let logged = call_singlecell_builtin("log1p_transform", vec![normalized.clone()]).unwrap();

    match normalized {
        Value::SparseMatrix(matrix) => {
            assert_eq!(matrix.nnz(), 5);
            assert_eq!(matrix.row_sums(), vec![10.0, 10.0, 10.0]);
        }
        other => panic!("expected sparse normalized matrix, got {other:?}"),
    }
    assert!(matches!(logged, Value::SparseMatrix(_)));

    let rows =
        call_singlecell_builtin("select_rows", vec![counts.clone(), int_list(&[2, 0])]).unwrap();
    let columns = call_singlecell_builtin("select_cols", vec![rows, int_list(&[2, 0])]).unwrap();
    match columns {
        Value::SparseMatrix(matrix) => {
            assert_eq!(matrix.to_dense(), vec![vec![1.0, 4.0], vec![3.0, 1.0]]);
        }
        other => panic!("expected sparse subset, got {other:?}"),
    }
}

#[test]
fn sparse_qc_and_hvg_match_expected_dimensions() {
    let counts = sparse_matrix(vec![
        vec![5.0, 0.0, 1.0],
        vec![3.0, 2.0, 0.0],
        vec![0.0, 4.0, 0.0],
    ]);
    let cell_qc = call_singlecell_builtin(
        "cell_qc",
        vec![counts.clone(), str_list(&["MT-A", "B", "C"])],
    )
    .unwrap();
    match cell_qc {
        Value::Table(table) => {
            assert_eq!(table.rows.len(), 3);
            assert_eq!(as_float(&table.rows[0][1]), 6.0);
            assert_eq!(as_int(&table.rows[0][2]), 2);
            assert!((as_float(&table.rows[0][3]) - 83.333_333).abs() < 1e-4);
        }
        other => panic!("expected QC table, got {other:?}"),
    }

    let gene_qc = call_singlecell_builtin("gene_qc", vec![counts.clone()]).unwrap();
    match gene_qc {
        Value::Table(table) => {
            assert_eq!(table.rows.len(), 3);
            assert_eq!(as_int(&table.rows[0][1]), 2);
        }
        other => panic!("expected gene QC table, got {other:?}"),
    }
    let hvg =
        call_singlecell_builtin("highly_variable_genes", vec![counts, Value::Int(2)]).unwrap();
    assert!(matches!(hvg, Value::List(ref values) if values.len() == 2));
}

#[test]
fn sparse_pca_returns_compact_scores_and_loadings() {
    let counts = sparse_matrix(vec![
        vec![8.0, 7.0, 0.0, 0.0],
        vec![7.0, 8.0, 0.0, 0.0],
        vec![0.0, 0.0, 8.0, 7.0],
        vec![0.0, 0.0, 7.0, 8.0],
    ]);
    let pca = call_singlecell_builtin("sc_pca", vec![counts, Value::Int(2)]).unwrap();
    let scores = get_list(&pca, "scores");
    let loadings = get_list(&pca, "loadings");
    assert_eq!(scores.len(), 4);
    assert!(scores
        .iter()
        .all(|row| matches!(row, Value::List(values) if values.len() == 2)));
    assert_eq!(loadings.len(), 4);
    assert_eq!(get_int(&pca, "n_components"), 2);
    let explained = get_list(&pca, "explained_variance_ratio");
    assert!(explained.iter().map(as_float).sum::<f64>() <= 1.0 + 1e-9);
}

// ─── sc_pca: the properties that make a result principal components ─────────

/// A matrix whose covariance spectrum is deliberately *flat* in the tail.
///
/// This is the shape that broke the old implementation, so it is the shape the
/// tests have to use. Twelve orthogonal directions carry variances 40 and 20 —
/// well separated — then ten more within a few percent of each other. Deflated
/// power iteration separates two directions at a rate set by the ratio of their
/// variances, so a near-flat tail is where it stalls, and once one component is
/// wrong every later one inherits the error.
/// The gene count has to exceed the block width or there is nothing to test.
/// With `k` components the implementation iterates on `k + 10` vectors, so a
/// twelve-gene fixture would hand it the entire space and turn the subspace
/// iteration into an exact dense solve — which passes every check while
/// exercising none of the convergence machinery. Sixty genes against twelve
/// requested components keeps it an actual subspace.
const SPECTRUM_GENES: usize = 60;

fn clustered_spectrum_matrix() -> Vec<Vec<f64>> {
    let n_cells = 200;
    (0..n_cells)
        .map(|cell| {
            (0..SPECTRUM_GENES)
                .map(|gene| {
                    // Two dominant directions, then a deliberately near-flat
                    // tail: the regime where power iteration stalls.
                    let variance = match gene {
                        0 => 40.0,
                        1 => 20.0,
                        g => 3.0 - 0.03 * (g as f64),
                    };
                    // A deterministic, mean-zero basis: distinct frequencies
                    // over the cell index.
                    let phase = (cell as f64 + 1.0) * (gene as f64 + 1.0) * 0.7;
                    variance.max(0.1).sqrt() * phase.sin()
                })
                .collect()
        })
        .collect()
}

fn pca_of(rows: Vec<Vec<f64>>, k: i64) -> Value {
    call_singlecell_builtin("sc_pca", vec![matrix(rows), Value::Int(k)]).unwrap()
}

/// The defining property: principal components are ordered.
///
/// The old implementation returned 8 inversions out of 40 components on real
/// data — explained variance rising from one component to the next, which means
/// they were not principal components. Nothing downstream can be trusted when
/// this fails, and nothing was checking it.
#[test]
fn pca_explained_variance_never_increases() {
    let pca = pca_of(clustered_spectrum_matrix(), 12);
    let explained: Vec<f64> = get_list(&pca, "explained_variance")
        .iter()
        .map(as_float)
        .collect();
    assert_eq!(explained.len(), 12);
    for window in explained.windows(2) {
        assert!(
            window[0] >= window[1] - 1e-9,
            "explained variance rose from {} to {} — components are out of order: {explained:?}",
            window[0],
            window[1]
        );
    }
}

/// Loadings must be orthonormal. A single Gram-Schmidt pass drifts exactly
/// where the spectrum is flat, so this is the same failure seen from the other
/// side: components that are supposed to describe independent directions and
/// quietly do not.
#[test]
fn pca_loadings_are_orthonormal() {
    let pca = pca_of(clustered_spectrum_matrix(), 12);
    let loadings = get_list(&pca, "loadings");
    let rows: Vec<Vec<f64>> = loadings
        .iter()
        .map(|row| match row {
            Value::List(values) => values.iter().map(as_float).collect(),
            other => panic!("expected a row, got {other:?}"),
        })
        .collect();
    let k = rows[0].len();
    let column = |c: usize| -> Vec<f64> { rows.iter().map(|row| row[c]).collect() };

    for i in 0..k {
        let a = column(i);
        let norm: f64 = a.iter().map(|v| v * v).sum::<f64>().sqrt();
        assert!(
            (norm - 1.0).abs() < 1e-6,
            "component {i} has norm {norm}, expected 1"
        );
        for j in (i + 1)..k {
            let b = column(j);
            let dot: f64 = a.iter().zip(&b).map(|(x, y)| x * y).sum();
            assert!(
                dot.abs() < 1e-6,
                "components {i} and {j} are not orthogonal (dot = {dot})"
            );
        }
    }
}

/// Scores must be the data projected onto the loadings. If the two disagree the
/// embedding everything clusters on is not the one the loadings describe.
#[test]
fn pca_scores_are_the_projection_the_loadings_describe() {
    let rows = clustered_spectrum_matrix();
    let pca = pca_of(rows.clone(), 6);
    let means: Vec<f64> = get_list(&pca, "mean").iter().map(as_float).collect();
    let loadings: Vec<Vec<f64>> = get_list(&pca, "loadings")
        .iter()
        .map(|row| match row {
            Value::List(values) => values.iter().map(as_float).collect(),
            other => panic!("expected a row, got {other:?}"),
        })
        .collect();
    let scores: Vec<Vec<f64>> = get_list(&pca, "scores")
        .iter()
        .map(|row| match row {
            Value::List(values) => values.iter().map(as_float).collect(),
            other => panic!("expected a row, got {other:?}"),
        })
        .collect();

    for (cell, row) in rows.iter().enumerate() {
        for component in 0..scores[cell].len() {
            let expected: f64 = row
                .iter()
                .zip(&means)
                .enumerate()
                .map(|(gene, (value, mean))| (value - mean) * loadings[gene][component])
                .sum();
            assert!(
                (scores[cell][component] - expected).abs() < 1e-6,
                "cell {cell} component {component}: stored {} vs projected {expected}",
                scores[cell][component]
            );
        }
    }
}

/// Ask for every component and the variance must all be accounted for.
///
/// The eigenvalues of a covariance matrix sum to its trace, which is the total
/// per-gene variance, so a full decomposition has to add back up to 100%.
#[test]
fn pca_with_every_component_accounts_for_all_the_variance() {
    let rows = clustered_spectrum_matrix();
    let n_genes = rows[0].len() as i64;
    let pca = pca_of(rows, n_genes);
    let ratios: Vec<f64> = get_list(&pca, "explained_variance_ratio")
        .iter()
        .map(as_float)
        .collect();
    let total: f64 = ratios.iter().sum();
    assert!(
        (total - 1.0).abs() < 1e-6,
        "a full decomposition explained {:.6} of the variance, not all of it: {ratios:?}",
        total
    );
}

/// The test with teeth: a partial decomposition must agree with a full one.
///
/// Ordering is guaranteed by the Rayleigh-Ritz step whether or not the subspace
/// converged, so monotonicity alone cannot detect a stalled iterate — a
/// deliberately crippled sweep limit still passes it. What a stalled iterate
/// *cannot* do is find the true leading directions, so its eigenvalues come out
/// too small. Asking for twelve components on sixty genes is a genuine subspace
/// iteration; asking for all sixty is an exact dense solve. They must agree.
#[test]
fn pca_subspace_iteration_finds_the_true_leading_components() {
    let rows = clustered_spectrum_matrix();
    let exact: Vec<f64> = get_list(
        &pca_of(rows.clone(), SPECTRUM_GENES as i64),
        "explained_variance",
    )
    .iter()
    .map(as_float)
    .collect();
    let partial: Vec<f64> = get_list(&pca_of(rows, 12), "explained_variance")
        .iter()
        .map(as_float)
        .collect();

    assert_eq!(partial.len(), 12);
    for (index, (got, want)) in partial.iter().zip(&exact).enumerate() {
        let relative = (got - want).abs() / want.abs().max(1e-12);
        assert!(
            relative < 1e-6,
            "component {index}: subspace run gave {got}, exact decomposition {want} \
             (relative error {relative:.2e}) — the iteration had not converged"
        );
    }
}

/// Two runs of the same input must agree exactly. Sign is arbitrary in any
/// eigendecomposition, so it is pinned by a rule rather than left to whichever
/// way the arithmetic fell.
#[test]
fn pca_is_reproducible_including_component_signs() {
    let rows = clustered_spectrum_matrix();
    let first = get_list(&pca_of(rows.clone(), 8), "loadings");
    let second = get_list(&pca_of(rows, 8), "loadings");
    assert_eq!(first, second, "identical input gave different loadings");
}

#[test]
fn leiden_graph_clusters_two_connected_groups() {
    let edges = vec![
        knn_edge(0, 1, 0.1),
        knn_edge(1, 2, 0.1),
        knn_edge(0, 2, 0.1),
        knn_edge(3, 4, 0.1),
        knn_edge(4, 5, 0.1),
        knn_edge(3, 5, 0.1),
        knn_edge(2, 3, 10.0),
    ];
    let labels = call_singlecell_builtin(
        "leiden_graph",
        vec![Value::List(edges.into()), Value::Int(6), Value::Float(1.0)],
    )
    .unwrap();
    match labels {
        Value::List(labels) => {
            assert_eq!(labels.len(), 6);
            assert_eq!(as_int(&labels[0]), as_int(&labels[1]));
            assert_eq!(as_int(&labels[1]), as_int(&labels[2]));
            assert_eq!(as_int(&labels[3]), as_int(&labels[4]));
            assert_ne!(as_int(&labels[0]), as_int(&labels[3]));
        }
        other => panic!("expected labels, got {other:?}"),
    }
}

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

    let sparse = call_singlecell_builtin(
        "read_10x_sparse",
        vec![Value::Str(path.to_str().unwrap().to_string())],
    )
    .expect("read_10x_sparse");
    match &sparse {
        Value::Record(record) => {
            match record.get("matrix").unwrap() {
                Value::SparseMatrix(matrix) => {
                    assert_eq!((matrix.nrow, matrix.ncol, matrix.nnz()), (3, 2, 4));
                    assert_eq!(matrix.get(0, 0), 5.0);
                    assert_eq!(matrix.get(0, 1), 1.0);
                    assert_eq!(matrix.row_names.as_ref().unwrap()[0], "AAACCTGAGAAACCAT-1");
                    assert_eq!(matrix.col_names.as_ref().unwrap()[0], "GENE1");
                }
                other => panic!("expected sparse matrix, got {other:?}"),
            }
            assert!(matches!(record.get("obs"), Some(Value::Table(_))));
            assert!(matches!(record.get("var"), Some(Value::Table(_))));
            assert!(matches!(record.get("layers"), Some(Value::Record(_))));
        }
        other => panic!("expected single-cell record, got {other:?}"),
    }
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
    let s_genes = int_list(&[0]); // gene 0
    let g2m_genes = int_list(&[1]); // gene 1

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
    let s_genes = int_list(&[]);
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
    let s_genes = int_list(&[0]);
    let g2m_genes = int_list(&[1]);

    let result =
        call_singlecell_builtin("cell_cycle_score", vec![mat, s_genes, g2m_genes]).unwrap();
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
//
// The builtin always returns `{matrix, genes}`. Genes detected in fewer than
// five cells are dropped, so which original-axis columns survived is never
// optional information — the previous shape, a bare Matrix assumed to line up
// with the input gene axis, could not express that.

fn sct_result(value: &Value) -> (Vec<Vec<f64>>, Vec<usize>) {
    let record = match value {
        Value::Record(record) => record,
        other => panic!("expected a Record, got {other:?}"),
    };
    let rows = match record.get("matrix").expect("matrix field") {
        Value::Matrix(m) => (0..m.nrow)
            .map(|i| (0..m.ncol).map(|j| m.data[i * m.ncol + j]).collect())
            .collect(),
        other => panic!("expected a Matrix, got {other:?}"),
    };
    let genes = match record.get("genes").expect("genes field") {
        Value::List(values) => values
            .iter()
            .map(|v| match v {
                Value::Int(n) => *n as usize,
                other => panic!("expected Int, got {other:?}"),
            })
            .collect(),
        other => panic!("expected a List, got {other:?}"),
    };
    (rows, genes)
}

/// Enough cells to actually fit an overdispersion — the method needs a gene
/// detected in at least a handful of cells before it will model it at all, so
/// four-cell fixtures return nothing and test nothing.
///
/// Genes 0 and 1 are switched on in the first half of the cells only. Genes 2
/// to 7 track sequencing depth, which is what the model expects of a gene with
/// no biology in it.
fn sct_fixture() -> Vec<Vec<f64>> {
    let n_cells = 120;
    (0..n_cells)
        .map(|cell| {
            // Depth varies threefold across cells, so "scales with depth" and
            // "is constant" are genuinely different behaviours here.
            let depth = 1.0 + 2.0 * (cell as f64 / n_cells as f64);
            (0..8)
                .map(|gene| {
                    let base = if gene < 2 && cell < n_cells / 2 {
                        30.0
                    } else {
                        4.0
                    };
                    ((base * depth) + ((cell * (gene + 1)) % 3) as f64).round()
                })
                .collect()
        })
        .collect()
}

#[test]
fn test_sc_sctransform_returns_a_residual_per_cell_per_surviving_gene() {
    let result = call_singlecell_builtin("sc_sctransform", vec![matrix(sct_fixture())]).unwrap();
    let stage = match &result {
        Value::Record(record) => record.get("residual_variance_stage"),
        _ => None,
    };
    assert_eq!(
        stage,
        Some(&Value::Str(
            "pearson_before_centering_and_covariate_regression".into()
        ))
    );
    let (rows, genes) = sct_result(&result);
    assert_eq!(rows.len(), 120, "one row per cell");
    assert!(!genes.is_empty(), "every gene was filtered out");
    for row in &rows {
        assert_eq!(row.len(), genes.len(), "row width must match the gene list");
    }
    assert!(
        genes.windows(2).all(|w| w[0] < w[1]),
        "indices ascend: {genes:?}"
    );
    assert!(
        rows.iter().flatten().any(|&r| r != 0.0),
        "every residual was zero"
    );
}

#[test]
fn test_sc_sctransform_sparse_and_dense_paths_match() {
    let rows = sct_fixture();
    let dense =
        call_singlecell_builtin("sc_sctransform", vec![matrix(rows.clone()), Value::Int(5)])
            .unwrap();
    let sparse =
        call_singlecell_builtin("sc_sctransform", vec![sparse_matrix(rows), Value::Int(5)])
            .unwrap();
    let (dense_residuals, dense_genes) = sct_result(&dense);
    let (sparse_residuals, sparse_genes) = sct_result(&sparse);
    assert_eq!(sparse_genes, dense_genes);
    assert_eq!(sparse_residuals, dense_residuals);
}

#[test]
fn test_sc_sctransform_drops_genes_below_min_cells_and_keeps_boundary() {
    let rows: Vec<Vec<f64>> = (0..120)
        .map(|cell| {
            vec![
                if cell < 4 { 7.0 } else { 0.0 },
                if cell < 5 { 2.0 } else { 0.0 },
                3.0 + (cell % 3) as f64,
            ]
        })
        .collect();
    let result = call_singlecell_builtin("sc_sctransform", vec![matrix(rows)]).unwrap();
    let (_, genes) = sct_result(&result);
    assert_eq!(genes, vec![1, 2]);
}

/// Pearson residuals are first clipped at sqrt(n_cells / 30), the published
/// default, and then centered just as Seurat's SCTransform calls ScaleData.
/// Centering a clipped column can move a value just outside the raw clip; it
/// cannot move it outside twice that bound.
#[test]
fn test_sc_sctransform_residuals_follow_seurats_clip_then_center_order() {
    let mut rows = sct_fixture();
    // One cell with an absurd count, to push against the ceiling.
    rows[0][0] = 500_000.0;
    let n_cells = rows.len();
    let result = call_singlecell_builtin("sc_sctransform", vec![matrix(rows)]).unwrap();
    let (residuals, _) = sct_result(&result);

    let clip = (n_cells as f64 / 30.0).sqrt();
    for gene in 0..residuals[0].len() {
        let mean = residuals.iter().map(|row| row[gene]).sum::<f64>() / n_cells as f64;
        assert!(mean.abs() < 1e-10, "gene {gene} mean was {mean}");
        for row in &residuals {
            let value = row[gene];
            assert!(
                value.abs() <= 2.0 * clip + 1e-9,
                "centered residual {value} escaped twice the raw clip of {clip}"
            );
        }
    }
    assert!(
        residuals.iter().flatten().any(|&r| r.abs() > clip - 1e-6),
        "the outlier never approached the raw clip, so this proves nothing"
    );
}

/// Capping picks by residual variance and must not disturb the values.
#[test]
fn test_sc_sctransform_cap_selects_without_changing_residuals() {
    let rows = sct_fixture();
    let (full, full_genes) =
        sct_result(&call_singlecell_builtin("sc_sctransform", vec![matrix(rows.clone())]).unwrap());
    let (capped, capped_genes) = sct_result(
        &call_singlecell_builtin("sc_sctransform", vec![matrix(rows), Value::Int(3)]).unwrap(),
    );

    assert_eq!(capped_genes.len(), 3, "asked for three genes");
    for (out, gene) in capped_genes.iter().enumerate() {
        let source = full_genes
            .iter()
            .position(|g| g == gene)
            .expect("a capped gene that the full run did not return");
        for cell in 0..capped.len() {
            assert!(
                (capped[cell][out] - full[cell][source]).abs() < 1e-12,
                "cell {cell} gene {gene}: capped {} vs full {}",
                capped[cell][out],
                full[cell][source]
            );
        }
    }
}

/// Asking for more features than survive filtering is not an error, and must
/// not silently return fewer than the unrestricted run.
#[test]
fn test_sc_sctransform_cap_wider_than_the_data_keeps_everything() {
    let rows = sct_fixture();
    let (_, all) =
        sct_result(&call_singlecell_builtin("sc_sctransform", vec![matrix(rows.clone())]).unwrap());
    let (_, asked) = sct_result(
        &call_singlecell_builtin("sc_sctransform", vec![matrix(rows), Value::Int(999)]).unwrap(),
    );
    assert_eq!(all, asked);
}

#[test]
fn test_sc_sctransform_empty() {
    let result = call_singlecell_builtin("sc_sctransform", vec![matrix(vec![])]).unwrap();
    assert_eq!(result, Value::List((vec![]).into()));
}

/// An all-zero matrix has no gene worth modelling, so the answer is an empty
/// gene list rather than a wall of NaN from dividing by a zero variance.
#[test]
fn test_sc_sctransform_zero_matrix_returns_no_genes() {
    let mat = matrix(vec![vec![0.0, 0.0], vec![0.0, 0.0]]);
    let result = call_singlecell_builtin("sc_sctransform", vec![mat]).unwrap();
    let (_, genes) = sct_result(&result);
    assert!(genes.is_empty(), "modelled a gene that is zero everywhere");
}

// ─── sc_integrate ─────────────────────────────────────────────────────────────

#[test]
fn test_sc_integrate_removes_batch_mean() {
    // Batch 0: mean gene0 = 2.0, mean gene1 = 4.0
    // Batch 1: mean gene0 = 10.0, mean gene1 = 20.0
    let mat = matrix(vec![
        vec![1.0, 3.0],   // batch 0, cell 0
        vec![3.0, 5.0],   // batch 0, cell 1
        vec![8.0, 18.0],  // batch 1, cell 2
        vec![12.0, 22.0], // batch 1, cell 3
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
    let mat = matrix(vec![vec![2.0, 4.0], vec![4.0, 8.0]]);
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
    Value::List(
        pairs
            .iter()
            .map(|&(s, t, d)| knn_edge(s, t, d))
            .collect::<Vec<_>>()
            .into(),
    )
}

#[test]
fn test_diffusion_pseudotime_linear_chain() {
    // 5 cells in a chain: 0—1—2—3—4 with unit weights
    let embeddings = matrix(vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0], vec![4.0]]);
    let edges = make_edges(&[(0, 1, 1.0), (1, 2, 1.0), (2, 3, 1.0), (3, 4, 1.0)]);
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
        vec![0.0, 0.0],
        vec![1.0, 0.0],
        vec![0.0, 1.0],
        vec![1.0, 1.0],
    ]);
    let edges = make_edges(&[(0, 1, 10.0), (0, 2, 1.0), (2, 3, 1.0), (1, 3, 1.0)]);
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
    let embeddings = matrix(vec![vec![0.0], vec![1.0], vec![5.0], vec![6.0]]);
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
    let result = call_singlecell_builtin(
        "diffusion_pseudotime",
        vec![embeddings, edges, Value::Int(0)],
    )
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
    let scores: Vec<f64> = table
        .rows
        .iter()
        .map(|r| match &r[score_col] {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => 0.0,
        })
        .collect();
    for window in scores.windows(2) {
        assert!(
            window[0] >= window[1],
            "rows not sorted descending: {:?}",
            scores
        );
    }
}

#[test]
fn test_lr_score_all_zero_returns_empty() {
    let mat = matrix(vec![vec![0.0, 0.0], vec![0.0, 0.0]]);
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
        "sender".to_string(),
        "receiver".to_string(),
        "ligand_idx".to_string(),
        "receptor_idx".to_string(),
        "score".to_string(),
    ];
    let rows = vec![
        vec![
            Value::Str("A".into()),
            Value::Str("B".into()),
            Value::Int(0),
            Value::Int(1),
            Value::Float(3.0),
        ],
        vec![
            Value::Str("A".into()),
            Value::Str("B".into()),
            Value::Int(2),
            Value::Int(3),
            Value::Float(1.5),
        ],
        vec![
            Value::Str("B".into()),
            Value::Str("A".into()),
            Value::Int(0),
            Value::Int(1),
            Value::Float(2.0),
        ],
    ];
    Value::Table(Table::new(columns, rows))
}

fn make_pathway_table() -> Value {
    use bl_core::value::Table;
    // pathway_map: ligand_idx, receptor_idx, pathway
    let columns = vec![
        "ligand_idx".to_string(),
        "receptor_idx".to_string(),
        "pathway".to_string(),
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
    assert!(
        (top_score - 3.0).abs() < 1e-9,
        "top score should be 3.0, got {top_score}"
    );
}

#[test]
fn test_lr_aggregate_no_pathway_match() {
    // Score table references pair (5, 6) which is not in pathway_map → empty output
    use bl_core::value::Table;
    let score_cols = vec![
        "sender".to_string(),
        "receiver".to_string(),
        "ligand_idx".to_string(),
        "receptor_idx".to_string(),
        "score".to_string(),
    ];
    let score_rows = vec![vec![
        Value::Str("A".into()),
        Value::Str("B".into()),
        Value::Int(5),
        Value::Int(6),
        Value::Float(2.0),
    ]];
    let scores = Value::Table(Table::new(score_cols, score_rows));
    let pathway_map = make_pathway_table(); // only knows about (0,1) and (2,3)

    let result = call_singlecell_builtin("lr_aggregate", vec![scores, pathway_map]).unwrap();
    let table = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    assert_eq!(
        table.rows.len(),
        0,
        "expected empty table when no pathway matches"
    );
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
        vec![matrix(vec![vec![1.0]]), Value::Str("bad".to_string())],
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
    let t = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    // Each spot should have 2 neighbors (k=2, n-1=2)
    assert_eq!(t.rows.len(), 6, "3 spots × 2 neighbors = 6 rows");
    // Spot 0's nearest neighbor should be spot 1 (distance 1.0)
    let spot0: Vec<&Vec<Value>> = t.rows.iter().filter(|r| r[0] == Value::Int(0)).collect();
    assert_eq!(spot0.len(), 2);
    let neighbor_of_0: i64 = match spot0[0][1] {
        Value::Int(n) => n,
        _ => panic!(),
    };
    assert_eq!(
        neighbor_of_0, 1,
        "spot 0's nearest neighbor should be spot 1"
    );
}

#[test]
fn test_spatial_neighbors_empty_coords() {
    use bl_core::value::Table;
    let coords = Value::Table(Table::new(vec!["x".to_string(), "y".to_string()], vec![]));
    let result = call_singlecell_builtin("spatial_neighbors", vec![coords]).unwrap();
    let t = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
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
    let i = match result {
        Value::Float(f) => f,
        other => panic!("{other:?}"),
    };
    assert!(
        i > 0.3,
        "expected positive Moran's I for spatially correlated expression, got {i}"
    );
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
    let i = match result {
        Value::Float(f) => f,
        other => panic!("{other:?}"),
    };
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
    let result =
        call_singlecell_builtin("reference_classify", vec![query_mat, ref_mat, labels]).unwrap();
    let t = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    assert_eq!(t.rows.len(), 1);
    let label = match &t.rows[0][1] {
        Value::Str(s) => s.clone(),
        other => panic!("{other:?}"),
    };
    assert_eq!(label, "A");
    let conf = match &t.rows[0][2] {
        Value::Float(f) => *f,
        other => panic!("{other:?}"),
    };
    assert!(conf > 0.0 && conf <= 1.0);
}

// ─── pseudobulk_aggregate ────────────────────────────────────────────────────

#[test]
fn test_pseudobulk_aggregate_basic() {
    // 4 cells × 2 genes: 2 from cluster A sample S1, 1 from A/S2, 1 from B/S1
    let mat = matrix(vec![
        vec![1.0, 2.0], // cell 0
        vec![3.0, 4.0], // cell 1
        vec![5.0, 0.0], // cell 2
        vec![0.0, 6.0], // cell 3
    ]);
    let cell_labels = str_list(&["A", "A", "A", "B"]);
    let sample_labels = str_list(&["S1", "S1", "S2", "S1"]);
    let result = call_singlecell_builtin(
        "pseudobulk_aggregate",
        vec![mat, cell_labels, sample_labels],
    )
    .unwrap();
    let t = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    // Columns should be: A__S1, A__S2, B__S1 (sorted)
    assert_eq!(t.columns, vec!["A__S1", "A__S2", "B__S1"]);
    assert_eq!(t.rows.len(), 2, "one row per gene");
    // gene 0: A__S1 = 1+3=4, A__S2 = 5, B__S1 = 0
    let a_s1_idx = t.col_index("A__S1").unwrap();
    let sum_g0_a_s1 = match &t.rows[0][a_s1_idx] {
        Value::Float(f) => *f,
        other => panic!("{other:?}"),
    };
    assert!(
        (sum_g0_a_s1 - 4.0).abs() < 1e-9,
        "A__S1 sum for gene0 should be 4, got {sum_g0_a_s1}"
    );
}

#[test]
fn test_pseudobulk_aggregate_sparse_matches_dense() {
    // Same cells × genes counts as the dense case, as CSR — the sparse path
    // must not change the answer, and it is the only path a loaded 10x object
    // ever takes.
    let dense = vec![
        vec![1.0, 2.0],
        vec![3.0, 4.0],
        vec![5.0, 0.0],
        vec![0.0, 6.0],
    ];
    let cell_labels = str_list(&["A", "A", "A", "B"]);
    let sample_labels = str_list(&["S1", "S1", "S2", "S1"]);

    let from_dense = call_singlecell_builtin(
        "pseudobulk_aggregate",
        vec![
            matrix(dense.clone()),
            cell_labels.clone(),
            sample_labels.clone(),
        ],
    )
    .unwrap();
    let from_sparse = call_singlecell_builtin(
        "pseudobulk_aggregate",
        vec![
            Value::SparseMatrix(std::sync::Arc::new(SparseMatrix::from_dense(&dense))),
            cell_labels,
            sample_labels,
        ],
    )
    .unwrap();

    assert_eq!(from_dense, from_sparse);
}

// ─── wnn_graph ───────────────────────────────────────────────────────────────

#[test]
fn test_wnn_graph_basic() {
    // 3 cells × 2 dims each; modality A perfectly separates all cells; modality B is noise
    let mat_a = matrix(vec![vec![0.0, 0.0], vec![10.0, 0.0], vec![20.0, 0.0]]);
    let mat_b = matrix(vec![vec![1.0, 0.0], vec![1.1, 0.0], vec![1.2, 0.0]]);
    let result = call_singlecell_builtin("wnn_graph", vec![mat_a, mat_b, Value::Int(2)]).unwrap();
    let t = match result {
        Value::Table(t) => t,
        other => panic!("{other:?}"),
    };
    // 3 cells × up to 2 neighbors = up to 6 edges
    assert!(!t.rows.is_empty(), "expected edges in WNN graph");
    // All weights should be in (0, 1]
    for row in &t.rows {
        let w = match &row[2] {
            Value::Float(f) => *f,
            other => panic!("{other:?}"),
        };
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
    let v00 = match &g0[0] {
        Value::Float(f) => *f,
        other => panic!("{other:?}"),
    };
    let v01 = match &g0[1] {
        Value::Float(f) => *f,
        other => panic!("{other:?}"),
    };
    assert!(
        (v00 - 1.0).abs() < 1e-4,
        "velocity[0][0] should be ~1.0, got {v00}"
    );
    assert!(
        (v01 - (-1.0)).abs() < 1e-4,
        "velocity[0][1] should be ~-1.0, got {v01}"
    );
}
