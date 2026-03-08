use bl_core::matrix::Matrix;

// ============================================================
// Original tests (migrated from inline mod tests)
// ============================================================

#[test]
fn test_new_and_get() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    assert_eq!(m.get(0, 0), 1.0);
    assert_eq!(m.get(1, 1), 4.0);
}

#[test]
fn test_transpose() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    let t = m.transpose();
    assert_eq!(t.nrow, 3);
    assert_eq!(t.ncol, 2);
    assert_eq!(t.get(0, 0), 1.0);
    assert_eq!(t.get(2, 1), 6.0);
}

#[test]
fn test_dot() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2).unwrap();
    let c = a.dot(&b).unwrap();
    assert_eq!(c.get(0, 0), 19.0);
    assert_eq!(c.get(0, 1), 22.0);
    assert_eq!(c.get(1, 0), 43.0);
    assert_eq!(c.get(1, 1), 50.0);
}

#[test]
fn test_identity() {
    let eye = Matrix::identity(3);
    assert_eq!(eye.get(0, 0), 1.0);
    assert_eq!(eye.get(0, 1), 0.0);
    assert_eq!(eye.get(2, 2), 1.0);
}

#[test]
fn test_row_col_means() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    assert_eq!(m.row_means(), vec![1.5, 3.5]);
    assert_eq!(m.col_means(), vec![2.0, 3.0]);
}

// ============================================================
// New comprehensive tests
// ============================================================

#[test]
fn test_new_invalid_dimensions() {
    let result = Matrix::new(vec![1.0, 2.0, 3.0], 2, 2);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("data length 3 != nrow(2) * ncol(2)"));
}

#[test]
fn test_new_empty_data_nonzero_dims() {
    let result = Matrix::new(vec![], 1, 1);
    assert!(result.is_err());
}

#[test]
fn test_new_too_much_data() {
    let result = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0], 2, 2);
    assert!(result.is_err());
}

#[test]
fn test_zeros() {
    let m = Matrix::zeros(3, 4);
    assert_eq!(m.nrow, 3);
    assert_eq!(m.ncol, 4);
    assert_eq!(m.data.len(), 12);
    for &v in &m.data {
        assert_eq!(v, 0.0);
    }
}

#[test]
fn test_zeros_zero_dimensions() {
    let m = Matrix::zeros(0, 0);
    assert_eq!(m.nrow, 0);
    assert_eq!(m.ncol, 0);
    assert_eq!(m.data.len(), 0);
}

#[test]
fn test_zeros_one_dimension_zero() {
    let m = Matrix::zeros(0, 5);
    assert_eq!(m.data.len(), 0);

    let m2 = Matrix::zeros(3, 0);
    assert_eq!(m2.data.len(), 0);
}

#[test]
fn test_single_element_matrix() {
    let m = Matrix::new(vec![42.0], 1, 1).unwrap();
    assert_eq!(m.get(0, 0), 42.0);
    assert_eq!(m.nrow, 1);
    assert_eq!(m.ncol, 1);
}

#[test]
fn test_single_element_transpose() {
    let m = Matrix::new(vec![7.0], 1, 1).unwrap();
    let t = m.transpose();
    assert_eq!(t, m);
}

#[test]
fn test_single_element_dot() {
    let a = Matrix::new(vec![3.0], 1, 1).unwrap();
    let b = Matrix::new(vec![4.0], 1, 1).unwrap();
    let c = a.dot(&b).unwrap();
    assert_eq!(c.get(0, 0), 12.0);
}

#[test]
fn test_non_square_matrix() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    assert_eq!(m.nrow, 2);
    assert_eq!(m.ncol, 3);
    assert_eq!(m.get(0, 0), 1.0);
    assert_eq!(m.get(0, 2), 3.0);
    assert_eq!(m.get(1, 0), 4.0);
    assert_eq!(m.get(1, 2), 6.0);
}

#[test]
fn test_non_square_dot() {
    // (2x3) dot (3x2) = (2x2)
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    let b = Matrix::new(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], 3, 2).unwrap();
    let c = a.dot(&b).unwrap();
    assert_eq!(c.nrow, 2);
    assert_eq!(c.ncol, 2);
    // Row 0: 1*7+2*9+3*11 = 7+18+33 = 58, 1*8+2*10+3*12 = 8+20+36 = 64
    assert_eq!(c.get(0, 0), 58.0);
    assert_eq!(c.get(0, 1), 64.0);
    // Row 1: 4*7+5*9+6*11 = 28+45+66 = 139, 4*8+5*10+6*12 = 32+50+72 = 154
    assert_eq!(c.get(1, 0), 139.0);
    assert_eq!(c.get(1, 1), 154.0);
}

#[test]
fn test_dot_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2).unwrap();
    let result = a.dot(&b);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("incompatible dimensions"));
}

#[test]
fn test_dot_vector_times_matrix() {
    // (1x3) dot (3x1) = (1x1)
    let a = Matrix::new(vec![1.0, 2.0, 3.0], 1, 3).unwrap();
    let b = Matrix::new(vec![4.0, 5.0, 6.0], 3, 1).unwrap();
    let c = a.dot(&b).unwrap();
    assert_eq!(c.nrow, 1);
    assert_eq!(c.ncol, 1);
    assert_eq!(c.get(0, 0), 32.0); // 1*4 + 2*5 + 3*6
}

#[test]
fn test_set() {
    let mut m = Matrix::zeros(2, 2);
    m.set(0, 0, 10.0);
    m.set(1, 1, 20.0);
    assert_eq!(m.get(0, 0), 10.0);
    assert_eq!(m.get(0, 1), 0.0);
    assert_eq!(m.get(1, 0), 0.0);
    assert_eq!(m.get(1, 1), 20.0);
}

#[test]
fn test_set_overwrite() {
    let mut m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    m.set(0, 1, 99.0);
    assert_eq!(m.get(0, 1), 99.0);
    // Others unchanged
    assert_eq!(m.get(0, 0), 1.0);
    assert_eq!(m.get(1, 0), 3.0);
    assert_eq!(m.get(1, 1), 4.0);
}

#[test]
fn test_row_slice() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    assert_eq!(m.row(0), vec![1.0, 2.0, 3.0]);
    assert_eq!(m.row(1), vec![4.0, 5.0, 6.0]);
}

#[test]
fn test_col_slice() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    assert_eq!(m.col(0), vec![1.0, 4.0]);
    assert_eq!(m.col(1), vec![2.0, 5.0]);
    assert_eq!(m.col(2), vec![3.0, 6.0]);
}

#[test]
fn test_add() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![10.0, 20.0, 30.0, 40.0], 2, 2).unwrap();
    let c = a.add(&b).unwrap();
    assert_eq!(c.get(0, 0), 11.0);
    assert_eq!(c.get(0, 1), 22.0);
    assert_eq!(c.get(1, 0), 33.0);
    assert_eq!(c.get(1, 1), 44.0);
}

#[test]
fn test_add_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    assert!(a.add(&b).is_err());
}

#[test]
fn test_sub() {
    let a = Matrix::new(vec![10.0, 20.0, 30.0, 40.0], 2, 2).unwrap();
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let c = a.sub(&b).unwrap();
    assert_eq!(c.get(0, 0), 9.0);
    assert_eq!(c.get(0, 1), 18.0);
    assert_eq!(c.get(1, 0), 27.0);
    assert_eq!(c.get(1, 1), 36.0);
}

#[test]
fn test_sub_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 2.0], 1, 2).unwrap();
    let b = Matrix::new(vec![1.0, 2.0], 2, 1).unwrap();
    assert!(a.sub(&b).is_err());
}

#[test]
fn test_sub_self_is_zero() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let z = m.sub(&m).unwrap();
    assert_eq!(z, Matrix::zeros(2, 2));
}

#[test]
fn test_mul_elementwise() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2).unwrap();
    let c = a.mul_elementwise(&b).unwrap();
    assert_eq!(c.get(0, 0), 5.0);
    assert_eq!(c.get(0, 1), 12.0);
    assert_eq!(c.get(1, 0), 21.0);
    assert_eq!(c.get(1, 1), 32.0);
}

#[test]
fn test_mul_elementwise_dimension_mismatch() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![1.0, 2.0, 3.0], 1, 3).unwrap();
    assert!(a.mul_elementwise(&b).is_err());
}

#[test]
fn test_scale() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let s = m.scale(3.0);
    assert_eq!(s.get(0, 0), 3.0);
    assert_eq!(s.get(0, 1), 6.0);
    assert_eq!(s.get(1, 0), 9.0);
    assert_eq!(s.get(1, 1), 12.0);
}

#[test]
fn test_scale_zero() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let s = m.scale(0.0);
    assert_eq!(s, Matrix::zeros(2, 2));
}

#[test]
fn test_scale_negative() {
    let m = Matrix::new(vec![1.0, -2.0, 3.0, -4.0], 2, 2).unwrap();
    let s = m.scale(-1.0);
    assert_eq!(s.get(0, 0), -1.0);
    assert_eq!(s.get(0, 1), 2.0);
    assert_eq!(s.get(1, 0), -3.0);
    assert_eq!(s.get(1, 1), 4.0);
}

#[test]
fn test_transpose_of_transpose() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    let tt = m.transpose().transpose();
    assert_eq!(tt, m);
}

#[test]
fn test_transpose_non_square() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, 2).unwrap();
    let t = m.transpose();
    assert_eq!(t.nrow, 2);
    assert_eq!(t.ncol, 3);
    assert_eq!(t.get(0, 0), 1.0);
    assert_eq!(t.get(0, 1), 3.0);
    assert_eq!(t.get(0, 2), 5.0);
    assert_eq!(t.get(1, 0), 2.0);
    assert_eq!(t.get(1, 1), 4.0);
    assert_eq!(t.get(1, 2), 6.0);
}

#[test]
fn test_transpose_preserves_names() {
    let mut m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    m.row_names = Some(vec!["r0".to_string(), "r1".to_string()]);
    m.col_names = Some(vec!["c0".to_string(), "c1".to_string()]);
    let t = m.transpose();
    assert_eq!(t.row_names.as_ref().unwrap(), &["c0", "c1"]);
    assert_eq!(t.col_names.as_ref().unwrap(), &["r0", "r1"]);
}

#[test]
fn test_identity_times_matrix() {
    let eye = Matrix::identity(3);
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3).unwrap();
    let result = eye.dot(&m).unwrap();
    assert_eq!(result, m);
}

#[test]
fn test_matrix_times_identity() {
    let eye = Matrix::identity(3);
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0], 3, 3).unwrap();
    let result = m.dot(&eye).unwrap();
    assert_eq!(result, m);
}

#[test]
fn test_identity_single() {
    let eye = Matrix::identity(1);
    assert_eq!(eye.nrow, 1);
    assert_eq!(eye.ncol, 1);
    assert_eq!(eye.get(0, 0), 1.0);
}

#[test]
fn test_large_identity() {
    let n = 100;
    let eye = Matrix::identity(n);
    assert_eq!(eye.nrow, n);
    assert_eq!(eye.ncol, n);
    for i in 0..n {
        for j in 0..n {
            if i == j {
                assert_eq!(eye.get(i, j), 1.0);
            } else {
                assert_eq!(eye.get(i, j), 0.0);
            }
        }
    }
}

#[test]
fn test_large_matrix_identity_multiply() {
    let n = 100;
    let eye = Matrix::identity(n);
    let result = eye.dot(&eye).unwrap();
    assert_eq!(result, eye);
}

#[test]
fn test_row_sums() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    assert_eq!(m.row_sums(), vec![6.0, 15.0]);
}

#[test]
fn test_col_sums() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    assert_eq!(m.col_sums(), vec![5.0, 7.0, 9.0]);
}

#[test]
fn test_row_col_means_non_square() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 2, 3).unwrap();
    assert_eq!(m.row_means(), vec![2.0, 5.0]);
    assert_eq!(m.col_means(), vec![2.5, 3.5, 4.5]);
}

#[test]
fn test_add_commutative() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2).unwrap();
    assert_eq!(a.add(&b).unwrap(), b.add(&a).unwrap());
}

#[test]
fn test_partial_eq() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    assert_eq!(a, b);
}

#[test]
fn test_partial_eq_different_dims() {
    let a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let b = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 1, 4).unwrap();
    assert_ne!(a, b);
}

#[test]
fn test_partial_eq_nearly_equal() {
    let a = Matrix::new(vec![1.0], 1, 1).unwrap();
    let b = Matrix::new(vec![1.0 + 1e-11], 1, 1).unwrap();
    // Within 1e-10 tolerance
    assert_eq!(a, b);
}

#[test]
fn test_display() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let s = format!("{}", m);
    assert!(s.contains("Matrix: 2x2"));
    assert!(s.contains("1.0000"));
}

#[test]
fn test_clone() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let c = m.clone();
    assert_eq!(m, c);
}

#[test]
fn test_new_zero_by_zero() {
    let m = Matrix::new(vec![], 0, 0).unwrap();
    assert_eq!(m.nrow, 0);
    assert_eq!(m.ncol, 0);
    assert_eq!(m.data.len(), 0);
}

#[test]
fn test_row_names_and_col_names() {
    let mut m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    assert!(m.row_names.is_none());
    assert!(m.col_names.is_none());

    m.row_names = Some(vec!["gene_a".to_string(), "gene_b".to_string()]);
    m.col_names = Some(vec!["sample_1".to_string(), "sample_2".to_string()]);
    assert_eq!(m.row_names.as_ref().unwrap()[0], "gene_a");
    assert_eq!(m.col_names.as_ref().unwrap()[1], "sample_2");
}

#[test]
fn test_dot_produces_correct_names() {
    let mut a = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    a.row_names = Some(vec!["r0".to_string(), "r1".to_string()]);
    a.col_names = Some(vec!["c0".to_string(), "c1".to_string()]);

    let mut b = Matrix::new(vec![5.0, 6.0, 7.0, 8.0], 2, 2).unwrap();
    b.col_names = Some(vec!["d0".to_string(), "d1".to_string()]);

    let c = a.dot(&b).unwrap();
    assert_eq!(c.row_names.as_ref().unwrap(), &["r0", "r1"]);
    assert_eq!(c.col_names.as_ref().unwrap(), &["d0", "d1"]);
}

#[test]
fn test_scale_identity_one() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let s = m.scale(1.0);
    assert_eq!(s, m);
}

#[test]
fn test_add_zeros_is_identity() {
    let m = Matrix::new(vec![1.0, 2.0, 3.0, 4.0], 2, 2).unwrap();
    let z = Matrix::zeros(2, 2);
    assert_eq!(m.add(&z).unwrap(), m);
}

#[test]
fn test_mul_elementwise_with_identity_row() {
    let m = Matrix::new(vec![3.0, 5.0, 7.0, 9.0], 2, 2).unwrap();
    let ones = Matrix::new(vec![1.0, 1.0, 1.0, 1.0], 2, 2).unwrap();
    assert_eq!(m.mul_elementwise(&ones).unwrap(), m);
}
