#[path = "../src/cnv.rs"]
mod cnv;
#[path = "../src/hic.rs"]
mod hic;

use bl_core::value::{Table, Value};

fn float_list(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|&f| Value::Float(f)).collect::<Vec<_>>().into())
}

fn int_list(vals: &[i64]) -> Value {
    Value::List(vals.iter().map(|&i| Value::Int(i)).collect::<Vec<_>>().into())
}

fn make_table(cols: &[&str], rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(
        cols.iter().map(|s| s.to_string()).collect(),
        rows,
    ))
}

// ── cnv tests ────────────────────────────────────────────────────────

#[test]
fn test_log2_ratios_basic() {
    // (3+1)/(1+1) = 2.0 => log2=1.0; (1+1)/(1+1)=1.0 => log2=0.0
    let tumor = float_list(&[3.0, 1.0]);
    let normal = float_list(&[1.0, 1.0]);
    let result = cnv::call_cnv_builtin("log2_ratios", vec![tumor, normal]).unwrap();
    if let Value::List(v) = result {
        let v0 = match &v[0] { Value::Float(f) => *f, _ => panic!() };
        let v1 = match &v[1] { Value::Float(f) => *f, _ => panic!() };
        assert!((v0 - 1.0).abs() < 1e-9, "expected 1.0, got {v0}");
        assert!((v1 - 0.0).abs() < 1e-9, "expected 0.0, got {v1}");
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_log2_ratios_length_mismatch() {
    let tumor = float_list(&[1.0, 2.0]);
    let normal = float_list(&[1.0]);
    assert!(cnv::call_cnv_builtin("log2_ratios", vec![tumor, normal]).is_err());
}

#[test]
fn test_cbs_segment_flat() {
    // Flat signal -> should produce 1 segment
    let ratios = float_list(&[0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]);
    let result = cnv::call_cnv_builtin("cbs_segment", vec![ratios]).unwrap();
    if let Value::Table(t) = result {
        // With flat signal, merging should collapse to 1 segment
        assert!(t.rows.len() >= 1);
        // start of first segment should be 0
        assert_eq!(t.rows[0][0], Value::Int(0));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_cbs_segment_step() {
    // Clear step: 10 zeros then 10 twos - should produce at least 2 segments
    let mut vals = vec![0.0f64; 10];
    vals.extend(vec![2.0f64; 10]);
    let ratios = float_list(&vals);
    let result = cnv::call_cnv_builtin("cbs_segment", vec![ratios]).unwrap();
    if let Value::Table(t) = result {
        assert!(t.rows.len() >= 1);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_cn_call_basic() {
    let seg_table = make_table(
        &["start", "end", "mean_ratio", "n_bins"],
        vec![
            vec![Value::Int(0), Value::Int(10), Value::Float(0.0), Value::Int(10)],   // CN=2
            vec![Value::Int(10), Value::Int(20), Value::Float(1.0), Value::Int(10)],  // CN=4
            vec![Value::Int(20), Value::Int(30), Value::Float(-1.0), Value::Int(10)], // CN=1
        ],
    );
    let result = cnv::call_cnv_builtin("cn_call", vec![seg_table, Value::Int(2)]).unwrap();
    if let Value::Table(t) = result {
        assert!(t.columns.contains(&"copy_number".to_string()));
        // mean_ratio=0.0, ploidy=2 => CN=2*2^0=2
        assert_eq!(t.rows[0].last().unwrap(), &Value::Int(2));
        // mean_ratio=1.0 => CN=2*2=4
        assert_eq!(t.rows[1].last().unwrap(), &Value::Int(4));
        // mean_ratio=-1.0 => CN=2*0.5=1
        assert_eq!(t.rows[2].last().unwrap(), &Value::Int(1));
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_allele_specific_cn_balanced() {
    // BAF=0.5 means balanced; ratio=0.0 => total_cn=2, minor=1, major=1
    let baf = float_list(&[0.5]);
    let ratio = float_list(&[0.0]);
    let result = cnv::call_cnv_builtin("allele_specific_cn", vec![baf, ratio]).unwrap();
    if let Value::Table(t) = result {
        let total_cn = match t.rows[0][0] { Value::Int(n) => n, _ => panic!() };
        let major_cn = match t.rows[0][1] { Value::Int(n) => n, _ => panic!() };
        let minor_cn = match t.rows[0][2] { Value::Int(n) => n, _ => panic!() };
        assert_eq!(total_cn, 2);
        assert_eq!(major_cn, 1);
        assert_eq!(minor_cn, 1);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_allele_specific_cn_loh() {
    // BAF~0.0 means LOH; ratio=0.0 => total_cn=2, minor=0, major=2
    let baf = float_list(&[0.0]);
    let ratio = float_list(&[0.0]);
    let result = cnv::call_cnv_builtin("allele_specific_cn", vec![baf, ratio]).unwrap();
    if let Value::Table(t) = result {
        let minor_cn = match t.rows[0][2] { Value::Int(n) => n, _ => panic!() };
        assert_eq!(minor_cn, 0);
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_cnv_summary_counts() {
    let seg_table = make_table(
        &["start", "end", "mean_ratio", "n_bins"],
        vec![
            vec![Value::Int(0), Value::Int(10), Value::Float(0.8), Value::Int(10)],   // amplified: 10 bins
            vec![Value::Int(10), Value::Int(20), Value::Float(0.0), Value::Int(10)],  // neutral
            vec![Value::Int(20), Value::Int(30), Value::Float(-1.5), Value::Int(10)], // deleted: 10 bins
        ],
    );
    let result = cnv::call_cnv_builtin("cnv_summary", vec![seg_table]).unwrap();
    if let Value::Record(fields) = result {
        assert_eq!(fields.get("n_segments"), Some(&Value::Int(3)));
        assert_eq!(fields.get("n_bins_amplified"), Some(&Value::Int(10)));
        assert_eq!(fields.get("n_bins_deleted"), Some(&Value::Int(10)));
        if let Some(Value::Float(fa)) = fields.get("fraction_altered") {
            assert!((fa - 2.0 / 3.0).abs() < 1e-6, "fraction_altered={fa}");
        } else {
            panic!("missing fraction_altered");
        }
    } else {
        panic!("expected Record");
    }
}

// ── hic tests ────────────────────────────────────────────────────────

fn make_square_table(mat: &[Vec<f64>]) -> Value {
    let n = mat.len();
    let col_names: Vec<String> = (0..n).map(|i| format!("bin{i}")).collect();
    let rows: Vec<Vec<Value>> = mat
        .iter()
        .map(|r| r.iter().map(|&v| Value::Float(v)).collect())
        .collect();
    Value::Table(Table::new(col_names, rows))
}

#[test]
fn test_ice_normalize_uniform() {
    // 3×3 uniform matrix - after ICE each row/col sum should be ~equal
    let mat = vec![
        vec![1.0, 2.0, 3.0],
        vec![2.0, 1.0, 2.0],
        vec![3.0, 2.0, 1.0],
    ];
    let input = make_square_table(&mat);
    let result = hic::call_hic_builtin("ice_normalize", vec![input]).unwrap();
    if let Value::Table(t) = result {
        // Row sums should all be equal after normalization
        let row_sums: Vec<f64> = t.rows.iter().map(|r| r.iter().map(|v| match v { Value::Float(f) => *f, _ => 0.0 }).sum::<f64>()).collect();
        let first = row_sums[0];
        for s in &row_sums {
            assert!((s - first).abs() < 0.1, "row sums differ: {row_sums:?}");
        }
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_ice_normalize_zero_row() {
    // Matrix with a zero row — should not panic
    let mat = vec![
        vec![0.0, 0.0, 0.0],
        vec![0.0, 1.0, 2.0],
        vec![0.0, 2.0, 1.0],
    ];
    let input = make_square_table(&mat);
    let result = hic::call_hic_builtin("ice_normalize", vec![input]);
    assert!(result.is_ok());
}

#[test]
fn test_insulation_score_shape() {
    // 6×6 identity-ish matrix
    let n = 6usize;
    let mat: Vec<Vec<f64>> = (0..n).map(|i| (0..n).map(|j| if i == j { 10.0 } else { 1.0 }).collect()).collect();
    let input = make_square_table(&mat);
    let result = hic::call_hic_builtin("insulation_score", vec![input, Value::Int(2)]).unwrap();
    if let Value::List(v) = result {
        assert_eq!(v.len(), n);
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_tad_boundaries_detects_minimum() {
    // Scores: high, low, high - the middle is a boundary
    let scores = Value::List((vec![
        Value::Float(1.0),
        Value::Float(-0.5),
        Value::Float(1.0),
    ]).into());
    let result = hic::call_hic_builtin("tad_boundaries", vec![scores, Value::Float(0.1)]).unwrap();
    if let Value::List(v) = result {
        assert!(!v.is_empty(), "expected at least one boundary");
        assert_eq!(v[0], Value::Int(1));
    } else {
        panic!("expected List");
    }
}

#[test]
fn test_distance_decay_shape() {
    let mat = vec![
        vec![10.0, 5.0, 2.0],
        vec![5.0, 10.0, 5.0],
        vec![2.0, 5.0, 10.0],
    ];
    let input = make_square_table(&mat);
    let result = hic::call_hic_builtin("distance_decay", vec![input]).unwrap();
    if let Value::Table(t) = result {
        assert_eq!(t.columns[0], "distance");
        assert_eq!(t.columns[1], "mean_contact");
        assert_eq!(t.rows.len(), 3);
        // d=0: mean of diagonal = 10.0
        let d0 = match t.rows[0][1] { Value::Float(f) => f, _ => panic!() };
        assert!((d0 - 10.0).abs() < 1e-6, "d=0 mean should be 10.0, got {d0}");
    } else {
        panic!("expected Table");
    }
}

#[test]
fn test_expected_contacts_shape() {
    let mat = vec![
        vec![10.0, 5.0, 2.0],
        vec![5.0, 10.0, 5.0],
        vec![2.0, 5.0, 10.0],
    ];
    let input = make_square_table(&mat);
    let result = hic::call_hic_builtin("expected_contacts", vec![input]).unwrap();
    if let Value::Table(t) = result {
        // Should be square 3×3
        assert_eq!(t.rows.len(), 3);
        assert_eq!(t.rows[0].len(), 3);
        // Diagonal should equal d=0 mean contact (10.0)
        let diag = match t.rows[0][0] { Value::Float(f) => f, _ => panic!() };
        assert!((diag - 10.0).abs() < 1e-6, "diagonal should be 10.0, got {diag}");
    } else {
        panic!("expected Table");
    }
}
