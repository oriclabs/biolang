use bl_core::value::Value;
use bl_runtime::matrix::call_matrix_builtin;

// ── Helper ──────────────────────────────────────────────────────

fn make_matrix_val(rows: Vec<Vec<f64>>) -> Value {
    Value::List(
        rows.into_iter()
            .map(|row| Value::List(row.into_iter().map(Value::Float).collect()))
            .collect(),
    )
}

fn mat(rows: Vec<Vec<f64>>) -> Value {
    call_matrix_builtin("matrix", vec![make_matrix_val(rows)]).unwrap()
}

// ── Construction ────────────────────────────────────────────────

#[test]
fn test_matrix_construction() {
    let list = Value::List(vec![
        Value::List(vec![Value::Int(1), Value::Int(2)]),
        Value::List(vec![Value::Int(3), Value::Int(4)]),
    ]);
    let result = call_matrix_builtin("matrix", vec![list]).unwrap();
    if let Value::Matrix(m) = result {
        assert_eq!(m.nrow, 2);
        assert_eq!(m.ncol, 2);
        assert_eq!(m.get(0, 0), 1.0);
        assert_eq!(m.get(1, 1), 4.0);
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_matrix_construction_from_empty_list() {
    let list = Value::List(vec![]);
    let result = call_matrix_builtin("matrix", vec![list]);
    // Empty list of rows should produce a 0x0 matrix or error
    // Depends on Matrix::new behavior with 0 rows
    match result {
        Ok(Value::Matrix(m)) => {
            assert_eq!(m.nrow, 0);
        }
        Err(_) => {} // acceptable
        _ => panic!("unexpected result"),
    }
}

#[test]
fn test_matrix_construction_ragged_rows() {
    let list = Value::List(vec![
        Value::List(vec![Value::Int(1), Value::Int(2)]),
        Value::List(vec![Value::Int(3)]),
    ]);
    let result = call_matrix_builtin("matrix", vec![list]);
    assert!(result.is_err(), "ragged rows should produce an error");
}

#[test]
fn test_matrix_construction_non_numeric() {
    let list = Value::List(vec![Value::List(vec![Value::Str("bad".into())])]);
    let result = call_matrix_builtin("matrix", vec![list]);
    assert!(result.is_err(), "non-numeric values should produce an error");
}

#[test]
fn test_matrix_construction_non_list() {
    let result = call_matrix_builtin("matrix", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── zeros and eye ───────────────────────────────────────────────

#[test]
fn test_zeros_and_eye() {
    let z = call_matrix_builtin("zeros", vec![Value::Int(3), Value::Int(2)]).unwrap();
    if let Value::Matrix(m) = z {
        assert_eq!(m.nrow, 3);
        assert_eq!(m.ncol, 2);
        assert_eq!(m.get(0, 0), 0.0);
    }

    let e = call_matrix_builtin("eye", vec![Value::Int(3)]).unwrap();
    if let Value::Matrix(m) = e {
        assert_eq!(m.get(0, 0), 1.0);
        assert_eq!(m.get(0, 1), 0.0);
    }
}

#[test]
fn test_identity_matrix_properties() {
    let n = 4;
    let eye = call_matrix_builtin("eye", vec![Value::Int(n)]).unwrap();
    if let Value::Matrix(m) = &eye {
        assert_eq!(m.nrow, n as usize);
        assert_eq!(m.ncol, n as usize);
        for i in 0..n as usize {
            for j in 0..n as usize {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert_eq!(m.get(i, j), expected, "eye[{i},{j}]");
            }
        }
    }

    // I * A = A (dot product with identity should be identity)
    let a = mat(vec![vec![1.0, 2.0, 3.0, 4.0], vec![5.0, 6.0, 7.0, 8.0], vec![9.0, 10.0, 11.0, 12.0], vec![13.0, 14.0, 15.0, 16.0]]);
    let result = call_matrix_builtin("dot", vec![eye, a.clone()]).unwrap();
    if let (Value::Matrix(r), Value::Matrix(orig)) = (&result, &a) {
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (r.get(i, j) - orig.get(i, j)).abs() < 1e-10,
                    "I*A should equal A at [{i},{j}]"
                );
            }
        }
    }
}

// ── Transpose ───────────────────────────────────────────────────

#[test]
fn test_transpose() {
    let list = Value::List(vec![
        Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)]),
        Value::List(vec![Value::Int(4), Value::Int(5), Value::Int(6)]),
    ]);
    let mat = call_matrix_builtin("matrix", vec![list]).unwrap();
    let result = call_matrix_builtin("transpose", vec![mat]).unwrap();
    if let Value::Matrix(m) = result {
        assert_eq!(m.nrow, 3);
        assert_eq!(m.ncol, 2);
    }
}

#[test]
fn test_transpose_1xn_to_nx1() {
    let list = Value::List(vec![Value::List(vec![
        Value::Float(1.0),
        Value::Float(2.0),
        Value::Float(3.0),
        Value::Float(4.0),
        Value::Float(5.0),
    ])]);
    let m = call_matrix_builtin("matrix", vec![list]).unwrap();
    // 1x5 matrix
    if let Value::Matrix(ref mat) = m {
        assert_eq!(mat.nrow, 1);
        assert_eq!(mat.ncol, 5);
    }
    let t = call_matrix_builtin("transpose", vec![m]).unwrap();
    if let Value::Matrix(mat) = t {
        assert_eq!(mat.nrow, 5);
        assert_eq!(mat.ncol, 1);
        assert_eq!(mat.get(0, 0), 1.0);
        assert_eq!(mat.get(4, 0), 5.0);
    } else {
        panic!("expected Matrix");
    }
}

// ── Dot product ─────────────────────────────────────────────────

#[test]
fn test_dot_product() {
    let a = Value::List(vec![
        Value::List(vec![Value::Int(1), Value::Int(2)]),
        Value::List(vec![Value::Int(3), Value::Int(4)]),
    ]);
    let b = Value::List(vec![
        Value::List(vec![Value::Int(5), Value::Int(6)]),
        Value::List(vec![Value::Int(7), Value::Int(8)]),
    ]);
    let ma = call_matrix_builtin("matrix", vec![a]).unwrap();
    let mb = call_matrix_builtin("matrix", vec![b]).unwrap();
    let result = call_matrix_builtin("dot", vec![ma, mb]).unwrap();
    if let Value::Matrix(m) = result {
        assert_eq!(m.get(0, 0), 19.0);
        assert_eq!(m.get(1, 1), 50.0);
    }
}

#[test]
fn test_dot_product_dimension_mismatch() {
    // 2x3 * 2x3 => error (inner dimensions 3 != 2)
    let a = mat(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    let b = mat(vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0]]);
    let result = call_matrix_builtin("dot", vec![a, b]);
    assert!(result.is_err(), "dot product with mismatched dimensions should error");
}

// ── dim ─────────────────────────────────────────────────────────

#[test]
fn test_dim() {
    let list = Value::List(vec![Value::List(vec![
        Value::Int(1),
        Value::Int(2),
        Value::Int(3),
    ])]);
    let mat = call_matrix_builtin("matrix", vec![list]).unwrap();
    let result = call_matrix_builtin("dim", vec![mat]).unwrap();
    assert_eq!(result, Value::List(vec![Value::Int(1), Value::Int(3)]));
}

// ── Arithmetic ──────────────────────────────────────────────────

#[test]
fn test_mat_arithmetic() {
    let a = Value::List(vec![
        Value::List(vec![Value::Float(1.0), Value::Float(2.0)]),
        Value::List(vec![Value::Float(3.0), Value::Float(4.0)]),
    ]);
    let b = Value::List(vec![
        Value::List(vec![Value::Float(5.0), Value::Float(6.0)]),
        Value::List(vec![Value::Float(7.0), Value::Float(8.0)]),
    ]);
    let ma = call_matrix_builtin("matrix", vec![a]).unwrap();
    let mb = call_matrix_builtin("matrix", vec![b]).unwrap();

    let sum = call_matrix_builtin("mat_add", vec![ma.clone(), mb.clone()]).unwrap();
    if let Value::Matrix(m) = sum {
        assert_eq!(m.get(0, 0), 6.0);
    }

    let scaled = call_matrix_builtin("mat_scale", vec![ma, Value::Float(2.0)]).unwrap();
    if let Value::Matrix(m) = scaled {
        assert_eq!(m.get(0, 0), 2.0);
        assert_eq!(m.get(1, 1), 8.0);
    }
}

// ── Aggregation ─────────────────────────────────────────────────

#[test]
fn test_row_col_aggregation() {
    let list = Value::List(vec![
        Value::List(vec![Value::Float(1.0), Value::Float(2.0)]),
        Value::List(vec![Value::Float(3.0), Value::Float(4.0)]),
    ]);
    let mat = call_matrix_builtin("matrix", vec![list]).unwrap();

    let rs = call_matrix_builtin("row_sums", vec![mat.clone()]).unwrap();
    assert_eq!(
        rs,
        Value::List(vec![Value::Float(3.0), Value::Float(7.0)])
    );

    let cm = call_matrix_builtin("col_means", vec![mat]).unwrap();
    assert_eq!(
        cm,
        Value::List(vec![Value::Float(2.0), Value::Float(3.0)])
    );
}

// ── PCA ─────────────────────────────────────────────────────────

#[test]
fn test_pca() {
    let list = Value::List(vec![
        Value::List(vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)]),
        Value::List(vec![Value::Float(4.0), Value::Float(5.0), Value::Float(6.0)]),
        Value::List(vec![Value::Float(7.0), Value::Float(8.0), Value::Float(9.0)]),
        Value::List(vec![
            Value::Float(10.0),
            Value::Float(11.0),
            Value::Float(12.0),
        ]),
    ]);
    let mat = call_matrix_builtin("matrix", vec![list]).unwrap();
    let result = call_matrix_builtin("pca", vec![mat, Value::Int(2)]).unwrap();
    if let Value::Record(map) = result {
        assert!(map.contains_key("explained_variance"));
        assert!(map.contains_key("transformed"));
    } else {
        panic!("expected Record");
    }
}

#[test]
fn test_pca_single_variable() {
    // 4 samples x 1 variable
    let m = mat(vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0]]);
    let result = call_matrix_builtin("pca", vec![m, Value::Int(1)]);
    // Should succeed (or handle gracefully)
    match result {
        Ok(Value::Record(map)) => {
            assert!(map.contains_key("explained_variance"));
        }
        Err(_) => {} // acceptable if PCA can't handle 1 variable
        _ => panic!("unexpected result type"),
    }
}

#[test]
fn test_pca_more_variables_than_samples() {
    // 2 samples x 5 variables
    let m = mat(vec![
        vec![1.0, 2.0, 3.0, 4.0, 5.0],
        vec![6.0, 7.0, 8.0, 9.0, 10.0],
    ]);
    let result = call_matrix_builtin("pca", vec![m, Value::Int(2)]);
    // Should succeed or error gracefully
    match result {
        Ok(Value::Record(map)) => {
            assert!(map.contains_key("explained_variance"));
        }
        Err(_) => {} // acceptable
        _ => panic!("unexpected result type"),
    }
}

// ── Correlation matrix ──────────────────────────────────────────

#[test]
fn test_cor_matrix() {
    let list = Value::List(vec![
        Value::List(vec![Value::Float(1.0), Value::Float(2.0)]),
        Value::List(vec![Value::Float(3.0), Value::Float(4.0)]),
        Value::List(vec![Value::Float(5.0), Value::Float(6.0)]),
    ]);
    let mat = call_matrix_builtin("matrix", vec![list]).unwrap();
    let result = call_matrix_builtin("cor_matrix", vec![mat]).unwrap();
    if let Value::Matrix(m) = result {
        assert!((m.get(0, 0) - 1.0).abs() < 1e-10); // Self-correlation = 1
        assert!((m.get(0, 1) - 1.0).abs() < 1e-10); // Perfect correlation
    }
}

#[test]
fn test_cor_matrix_constant_column() {
    // One constant column => correlation with it is degenerate (NaN or 0.0)
    let m = mat(vec![
        vec![1.0, 5.0],
        vec![2.0, 5.0],
        vec![3.0, 5.0],
    ]);
    let result = call_matrix_builtin("cor_matrix", vec![m]).unwrap();
    if let Value::Matrix(cm) = result {
        assert!((cm.get(0, 0) - 1.0).abs() < 1e-10, "self-correlation of non-constant column");
        // correlation involving a constant column is degenerate (NaN or 0)
        let cross = cm.get(0, 1);
        assert!(
            cross.is_nan() || cross.abs() < 1e-10,
            "correlation with constant column should be NaN or 0, got {cross}"
        );
    } else {
        panic!("expected Matrix");
    }
}

// ── Distance matrix ─────────────────────────────────────────────

#[test]
fn test_dist_matrix() {
    let list = Value::List(vec![
        Value::List(vec![Value::Float(0.0), Value::Float(0.0)]),
        Value::List(vec![Value::Float(3.0), Value::Float(4.0)]),
    ]);
    let mat = call_matrix_builtin("matrix", vec![list]).unwrap();
    let result =
        call_matrix_builtin("dist_matrix", vec![mat, Value::Str("euclidean".into())]).unwrap();
    if let Value::Matrix(m) = result {
        assert!((m.get(0, 1) - 5.0).abs() < 1e-10); // 3-4-5 triangle
        assert_eq!(m.get(0, 0), 0.0); // Self-distance = 0
    }
}

#[test]
fn test_dist_matrix_single_point() {
    let m = mat(vec![vec![1.0, 2.0, 3.0]]);
    let result =
        call_matrix_builtin("dist_matrix", vec![m, Value::Str("euclidean".into())]).unwrap();
    if let Value::Matrix(dm) = result {
        assert_eq!(dm.nrow, 1);
        assert_eq!(dm.ncol, 1);
        assert_eq!(dm.get(0, 0), 0.0);
    } else {
        panic!("expected Matrix");
    }
}

#[test]
fn test_dist_matrix_unknown_method() {
    let m = mat(vec![vec![1.0], vec![2.0]]);
    let result = call_matrix_builtin("dist_matrix", vec![m, Value::Str("cosine".into())]);
    assert!(result.is_err(), "unknown distance method should error");
}

#[test]
fn test_dist_matrix_manhattan() {
    let m = mat(vec![vec![0.0, 0.0], vec![3.0, 4.0]]);
    let result =
        call_matrix_builtin("dist_matrix", vec![m, Value::Str("manhattan".into())]).unwrap();
    if let Value::Matrix(dm) = result {
        assert!((dm.get(0, 1) - 7.0).abs() < 1e-10); // |3| + |4| = 7
    }
}

// ── matrix_to_table ─────────────────────────────────────────────

#[test]
fn test_matrix_to_table() {
    let list = Value::List(vec![
        Value::List(vec![Value::Float(1.0), Value::Float(2.0)]),
        Value::List(vec![Value::Float(3.0), Value::Float(4.0)]),
    ]);
    let mat = call_matrix_builtin("matrix", vec![list]).unwrap();
    let result = call_matrix_builtin("matrix_to_table", vec![mat]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.num_rows(), 2);
        assert_eq!(t.num_cols(), 2);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_matrix_to_table_default_column_names() {
    // Matrix without col_names should get default V1, V2, ... names
    let m = mat(vec![vec![1.0, 2.0, 3.0]]);
    let result = call_matrix_builtin("matrix_to_table", vec![m]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.columns[0], "V1");
        assert_eq!(t.columns[1], "V2");
        assert_eq!(t.columns[2], "V3");
    } else {
        panic!("expected Table");
    }
}

// ── mat_map requires HOF ────────────────────────────────────────

#[test]
fn test_mat_map_requires_hof() {
    let m = mat(vec![vec![1.0]]);
    let result = call_matrix_builtin("mat_map", vec![m, Value::Int(2)]);
    assert!(result.is_err(), "mat_map should error via call_matrix_builtin");
}

// ── Unknown builtin ─────────────────────────────────────────────

#[test]
fn test_unknown_matrix_builtin() {
    let r = call_matrix_builtin("nonexistent", vec![]);
    assert!(r.is_err());
}

// ── Type errors ─────────────────────────────────────────────────

#[test]
fn test_dim_wrong_type() {
    let r = call_matrix_builtin("dim", vec![Value::Int(1)]);
    assert!(r.is_err());
}

#[test]
fn test_transpose_wrong_type() {
    let r = call_matrix_builtin("transpose", vec![Value::Str("bad".into())]);
    assert!(r.is_err());
}

#[test]
fn test_dot_wrong_type() {
    let m = mat(vec![vec![1.0]]);
    let r = call_matrix_builtin("dot", vec![m, Value::Int(1)]);
    assert!(r.is_err());
}

#[test]
fn test_zeros_wrong_type() {
    let r = call_matrix_builtin("zeros", vec![Value::Str("a".into()), Value::Int(1)]);
    assert!(r.is_err());
}

#[test]
fn test_eye_wrong_type() {
    let r = call_matrix_builtin("eye", vec![Value::Float(1.5)]);
    assert!(r.is_err());
}

#[test]
fn test_mat_scale_wrong_type() {
    let m = mat(vec![vec![1.0]]);
    let r = call_matrix_builtin("mat_scale", vec![m, Value::Str("bad".into())]);
    assert!(r.is_err());
}
