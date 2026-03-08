use bl_core::sparse_matrix::SparseMatrix;

// ============================================================
// Original tests (migrated from inline mod tests)
// ============================================================

#[test]
fn test_from_triplets() {
    let sm = SparseMatrix::from_triplets(
        &[0, 0, 1, 2],
        &[0, 2, 1, 0],
        &[1.0, 2.0, 3.0, 4.0],
        3,
        3,
    );
    assert_eq!(sm.get(0, 0), 1.0);
    assert_eq!(sm.get(0, 2), 2.0);
    assert_eq!(sm.get(1, 1), 3.0);
    assert_eq!(sm.get(2, 0), 4.0);
    assert_eq!(sm.get(1, 0), 0.0);
    assert_eq!(sm.nnz(), 4);
}

#[test]
fn test_from_dense() {
    let dense = vec![
        vec![1.0, 0.0, 2.0],
        vec![0.0, 3.0, 0.0],
        vec![4.0, 0.0, 0.0],
    ];
    let sm = SparseMatrix::from_dense(&dense);
    assert_eq!(sm.nnz(), 4);
    assert_eq!(sm.get(0, 0), 1.0);
    assert_eq!(sm.get(0, 1), 0.0);
}

#[test]
fn test_to_dense() {
    let sm = SparseMatrix::from_triplets(
        &[0, 1],
        &[0, 1],
        &[5.0, 10.0],
        2,
        2,
    );
    let dense = sm.to_dense();
    assert_eq!(dense, vec![vec![5.0, 0.0], vec![0.0, 10.0]]);
}

#[test]
fn test_row_col_sums() {
    let sm = SparseMatrix::from_triplets(
        &[0, 0, 1],
        &[0, 1, 1],
        &[1.0, 2.0, 3.0],
        2,
        2,
    );
    assert_eq!(sm.row_sums(), vec![3.0, 3.0]);
    assert_eq!(sm.col_sums(), vec![1.0, 5.0]);
}

// ============================================================
// New comprehensive tests
// ============================================================

#[test]
fn test_empty_sparse_matrix() {
    let sm = SparseMatrix::from_triplets(&[], &[], &[], 3, 4);
    assert_eq!(sm.nnz(), 0);
    assert_eq!(sm.nrow, 3);
    assert_eq!(sm.ncol, 4);
    assert_eq!(sm.get(0, 0), 0.0);
    assert_eq!(sm.get(2, 3), 0.0);
}

#[test]
fn test_empty_sparse_to_dense() {
    let sm = SparseMatrix::from_triplets(&[], &[], &[], 2, 3);
    let dense = sm.to_dense();
    assert_eq!(dense, vec![vec![0.0, 0.0, 0.0], vec![0.0, 0.0, 0.0]]);
}

#[test]
fn test_empty_sparse_row_col_sums() {
    let sm = SparseMatrix::from_triplets(&[], &[], &[], 2, 3);
    assert_eq!(sm.row_sums(), vec![0.0, 0.0]);
    assert_eq!(sm.col_sums(), vec![0.0, 0.0, 0.0]);
}

#[test]
fn test_single_element_sparse() {
    let sm = SparseMatrix::from_triplets(&[0], &[0], &[42.0], 1, 1);
    assert_eq!(sm.nnz(), 1);
    assert_eq!(sm.get(0, 0), 42.0);
    assert_eq!(sm.nrow, 1);
    assert_eq!(sm.ncol, 1);
}

#[test]
fn test_single_element_in_large_matrix() {
    let sm = SparseMatrix::from_triplets(&[5], &[3], &[99.0], 10, 10);
    assert_eq!(sm.nnz(), 1);
    assert_eq!(sm.get(5, 3), 99.0);
    assert_eq!(sm.get(0, 0), 0.0);
    assert_eq!(sm.get(9, 9), 0.0);
}

#[test]
fn test_dense_roundtrip() {
    let dense_orig = vec![
        vec![1.0, 0.0, 3.0],
        vec![0.0, 0.0, 0.0],
        vec![4.0, 5.0, 0.0],
    ];
    let sm = SparseMatrix::from_dense(&dense_orig);
    let dense_back = sm.to_dense();
    assert_eq!(dense_orig, dense_back);
}

#[test]
fn test_dense_roundtrip_all_nonzero() {
    let dense_orig = vec![
        vec![1.0, 2.0],
        vec![3.0, 4.0],
    ];
    let sm = SparseMatrix::from_dense(&dense_orig);
    assert_eq!(sm.nnz(), 4);
    let dense_back = sm.to_dense();
    assert_eq!(dense_orig, dense_back);
}

#[test]
fn test_dense_roundtrip_all_zeros() {
    let dense_orig = vec![
        vec![0.0, 0.0],
        vec![0.0, 0.0],
    ];
    let sm = SparseMatrix::from_dense(&dense_orig);
    assert_eq!(sm.nnz(), 0);
    let dense_back = sm.to_dense();
    assert_eq!(dense_orig, dense_back);
}

#[test]
fn test_from_triplets_duplicate_entries() {
    // Same (row, col) appears twice — both entries are stored (CSR allows duplicates)
    let sm = SparseMatrix::from_triplets(
        &[0, 0],
        &[0, 0],
        &[1.0, 2.0],
        1,
        1,
    );
    // nnz stores both entries
    assert_eq!(sm.nnz(), 2);
    // get() returns the first match found
    let val = sm.get(0, 0);
    assert!(val == 1.0 || val == 2.0);
}

#[test]
fn test_from_triplets_unsorted_input() {
    // Entries given out of order should still work (from_triplets sorts)
    let sm = SparseMatrix::from_triplets(
        &[2, 0, 1],
        &[0, 1, 2],
        &[30.0, 10.0, 20.0],
        3,
        3,
    );
    assert_eq!(sm.get(0, 1), 10.0);
    assert_eq!(sm.get(1, 2), 20.0);
    assert_eq!(sm.get(2, 0), 30.0);
}

#[test]
fn test_get_out_of_range_row() {
    let sm = SparseMatrix::from_triplets(&[0], &[0], &[1.0], 2, 2);
    // Row out of range returns 0.0 (checked in source)
    assert_eq!(sm.get(5, 0), 0.0);
}

#[test]
fn test_get_missing_column() {
    let sm = SparseMatrix::from_triplets(&[0], &[0], &[1.0], 2, 5);
    assert_eq!(sm.get(0, 4), 0.0);
    assert_eq!(sm.get(1, 0), 0.0);
}

#[test]
fn test_large_sparse_matrix() {
    let n = 1000;
    // Diagonal matrix
    let rows: Vec<usize> = (0..n).collect();
    let cols: Vec<usize> = (0..n).collect();
    let vals: Vec<f64> = (0..n).map(|i| i as f64 + 1.0).collect();
    let sm = SparseMatrix::from_triplets(&rows, &cols, &vals, n, n);
    assert_eq!(sm.nnz(), n);
    assert_eq!(sm.get(0, 0), 1.0);
    assert_eq!(sm.get(999, 999), 1000.0);
    assert_eq!(sm.get(500, 501), 0.0);
}

#[test]
fn test_large_sparse_row_sums() {
    let n = 100;
    let rows: Vec<usize> = (0..n).collect();
    let cols: Vec<usize> = (0..n).collect();
    let vals: Vec<f64> = vec![1.0; n];
    let sm = SparseMatrix::from_triplets(&rows, &cols, &vals, n, n);
    let sums = sm.row_sums();
    for s in &sums {
        assert_eq!(*s, 1.0);
    }
}

#[test]
fn test_row_sums_with_empty_rows() {
    // Row 1 has no entries
    let sm = SparseMatrix::from_triplets(
        &[0, 2],
        &[0, 1],
        &[5.0, 10.0],
        3,
        2,
    );
    assert_eq!(sm.row_sums(), vec![5.0, 0.0, 10.0]);
}

#[test]
fn test_col_sums_with_empty_cols() {
    // Col 1 has no entries
    let sm = SparseMatrix::from_triplets(
        &[0, 1],
        &[0, 2],
        &[5.0, 10.0],
        2,
        3,
    );
    assert_eq!(sm.col_sums(), vec![5.0, 0.0, 10.0]);
}

#[test]
fn test_nnz_empty() {
    let sm = SparseMatrix::from_triplets(&[], &[], &[], 5, 5);
    assert_eq!(sm.nnz(), 0);
}

#[test]
fn test_nnz_full() {
    let dense = vec![
        vec![1.0, 2.0, 3.0],
        vec![4.0, 5.0, 6.0],
    ];
    let sm = SparseMatrix::from_dense(&dense);
    assert_eq!(sm.nnz(), 6);
}

#[test]
fn test_sparse_preserves_values_via_dense() {
    let rows = vec![0, 0, 1, 2, 2];
    let cols = vec![0, 2, 1, 0, 2];
    let vals = vec![1.5, 2.5, 3.5, 4.5, 5.5];
    let sm = SparseMatrix::from_triplets(&rows, &cols, &vals, 3, 3);
    let dense = sm.to_dense();

    assert_eq!(dense[0][0], 1.5);
    assert_eq!(dense[0][1], 0.0);
    assert_eq!(dense[0][2], 2.5);
    assert_eq!(dense[1][0], 0.0);
    assert_eq!(dense[1][1], 3.5);
    assert_eq!(dense[1][2], 0.0);
    assert_eq!(dense[2][0], 4.5);
    assert_eq!(dense[2][1], 0.0);
    assert_eq!(dense[2][2], 5.5);
}

#[test]
fn test_from_dense_empty_input() {
    let dense: Vec<Vec<f64>> = vec![];
    let sm = SparseMatrix::from_dense(&dense);
    assert_eq!(sm.nrow, 0);
    assert_eq!(sm.ncol, 0);
    assert_eq!(sm.nnz(), 0);
}

#[test]
fn test_normalize_log1p_cpm() {
    let sm = SparseMatrix::from_triplets(
        &[0, 1],
        &[0, 0],
        &[100.0, 200.0],
        2,
        1,
    );
    let normed = sm.normalize_log1p_cpm();
    assert_eq!(normed.nnz(), 2);
    // Both entries are in column 0, col_sum = 300
    // Entry (0,0): (100/300 * 1e6 + 1).ln()
    let expected_0 = (100.0 / 300.0 * 1e6 + 1.0_f64).ln();
    let expected_1 = (200.0 / 300.0 * 1e6 + 1.0_f64).ln();
    assert!((normed.get(0, 0) - expected_0).abs() < 1e-6);
    assert!((normed.get(1, 0) - expected_1).abs() < 1e-6);
}

#[test]
fn test_normalize_log1p_cpm_preserves_sparsity() {
    let sm = SparseMatrix::from_triplets(
        &[0, 2],
        &[0, 1],
        &[50.0, 100.0],
        3,
        2,
    );
    let normed = sm.normalize_log1p_cpm();
    assert_eq!(normed.nnz(), sm.nnz());
    assert_eq!(normed.nrow, 3);
    assert_eq!(normed.ncol, 2);
    // Zero entries remain structurally absent
    assert_eq!(normed.get(1, 0), 0.0);
}

#[test]
fn test_normalize_scale() {
    // Simple case: single row with known values
    let sm = SparseMatrix::from_triplets(
        &[0, 0],
        &[0, 1],
        &[4.0, 8.0],
        1,
        3, // ncol=3, so zero-count matters
    );
    let normed = sm.normalize_scale();
    assert_eq!(normed.nnz(), 2);
    // mean = (4+8+0)/3 = 4.0
    // var = ((4-4)^2 + (8-4)^2 + (0-4)^2)/3 = (0+16+16)/3 = 32/3
    // std = sqrt(32/3)
    let mean = 4.0;
    let std_dev = (32.0 / 3.0_f64).sqrt();
    let expected_0 = (4.0 - mean) / std_dev;
    let expected_1 = (8.0 - mean) / std_dev;
    assert!((normed.get(0, 0) - expected_0).abs() < 1e-10);
    assert!((normed.get(0, 1) - expected_1).abs() < 1e-10);
}

#[test]
fn test_normalize_scale_empty_row() {
    // Row 1 has no entries — should be left as-is (all zeros)
    let sm = SparseMatrix::from_triplets(
        &[0],
        &[0],
        &[5.0],
        2,
        2,
    );
    let normed = sm.normalize_scale();
    assert_eq!(normed.get(1, 0), 0.0);
    assert_eq!(normed.get(1, 1), 0.0);
}

#[test]
fn test_partial_eq() {
    let a = SparseMatrix::from_triplets(&[0, 1], &[0, 1], &[1.0, 2.0], 2, 2);
    let b = SparseMatrix::from_triplets(&[0, 1], &[0, 1], &[1.0, 2.0], 2, 2);
    assert_eq!(a, b);
}

#[test]
fn test_partial_eq_different_values() {
    let a = SparseMatrix::from_triplets(&[0], &[0], &[1.0], 1, 1);
    let b = SparseMatrix::from_triplets(&[0], &[0], &[2.0], 1, 1);
    assert_ne!(a, b);
}

#[test]
fn test_partial_eq_different_dims() {
    let a = SparseMatrix::from_triplets(&[0], &[0], &[1.0], 1, 2);
    let b = SparseMatrix::from_triplets(&[0], &[0], &[1.0], 2, 1);
    assert_ne!(a, b);
}

#[test]
fn test_display() {
    let sm = SparseMatrix::from_triplets(&[0], &[0], &[1.0], 3, 3);
    let s = format!("{}", sm);
    assert!(s.contains("SparseMatrix: 3x3"));
    assert!(s.contains("1 non-zero"));
    // 1 - 1/9 = 88.9%
    assert!(s.contains("88.9% sparse"));
}

#[test]
fn test_display_empty() {
    let sm = SparseMatrix::from_triplets(&[], &[], &[], 0, 0);
    let s = format!("{}", sm);
    assert!(s.contains("0x0"));
    assert!(s.contains("100.0% sparse"));
}

#[test]
fn test_clone() {
    let sm = SparseMatrix::from_triplets(&[0, 1], &[1, 0], &[3.0, 7.0], 2, 2);
    let c = sm.clone();
    assert_eq!(sm, c);
}

#[test]
fn test_row_col_names() {
    let mut sm = SparseMatrix::from_triplets(&[0], &[0], &[1.0], 2, 2);
    assert!(sm.row_names.is_none());
    assert!(sm.col_names.is_none());

    sm.row_names = Some(vec!["gene_a".to_string(), "gene_b".to_string()]);
    sm.col_names = Some(vec!["cell_1".to_string(), "cell_2".to_string()]);
    assert_eq!(sm.row_names.as_ref().unwrap()[0], "gene_a");
    assert_eq!(sm.col_names.as_ref().unwrap()[1], "cell_2");
}

#[test]
fn test_indptr_structure() {
    // Verify the CSR indptr is correctly constructed
    let sm = SparseMatrix::from_triplets(
        &[0, 0, 2],
        &[0, 2, 1],
        &[1.0, 2.0, 3.0],
        3,
        3,
    );
    // indptr length = nrow + 1
    assert_eq!(sm.indptr.len(), 4);
    assert_eq!(sm.indptr[0], 0);
    // Row 0 has 2 entries, row 1 has 0, row 2 has 1
    assert_eq!(sm.indptr[1], 2);
    assert_eq!(sm.indptr[2], 2);
    assert_eq!(sm.indptr[3], 3);
}

#[test]
fn test_negative_values() {
    let sm = SparseMatrix::from_triplets(
        &[0, 1],
        &[0, 1],
        &[-5.0, -10.0],
        2,
        2,
    );
    assert_eq!(sm.get(0, 0), -5.0);
    assert_eq!(sm.get(1, 1), -10.0);
    assert_eq!(sm.row_sums(), vec![-5.0, -10.0]);
    assert_eq!(sm.col_sums(), vec![-5.0, -10.0]);
}

#[test]
fn test_from_dense_non_square() {
    let dense = vec![
        vec![1.0, 0.0, 0.0, 2.0],
        vec![0.0, 3.0, 0.0, 0.0],
    ];
    let sm = SparseMatrix::from_dense(&dense);
    assert_eq!(sm.nrow, 2);
    assert_eq!(sm.ncol, 4);
    assert_eq!(sm.nnz(), 3);
    assert_eq!(sm.get(0, 3), 2.0);
    assert_eq!(sm.get(1, 1), 3.0);
}
