//! Tests for deconvolution.rs and metabolomics.rs
//! Uses #[path] to compile without lib.rs registration.

#[path = "../src/deconvolution.rs"]
mod deconvolution;
#[path = "../src/metabolomics.rs"]
mod metabolomics;

use bl_core::value::{Table, Value};

// ── helpers ──────────────────────────────────────────────────────────

fn float_list(vals: &[f64]) -> Value {
    Value::List(
        vals.iter()
            .map(|&f| Value::Float(f))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn make_table(cols: Vec<&str>, rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(
        cols.iter().map(|s| s.to_string()).collect(),
        rows,
    ))
}

fn get_float(v: &Value) -> f64 {
    match v {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => panic!("expected Float, got {v:?}"),
    }
}

fn get_list_floats(v: &Value) -> Vec<f64> {
    match v {
        Value::List(l) => l.iter().map(get_float).collect(),
        _ => panic!("expected List"),
    }
}

fn get_table(v: Value) -> Table {
    match v {
        Value::Table(t) => t,
        _ => panic!("expected Table"),
    }
}

// ── deconvolution tests ──────────────────────────────────────────────

#[test]
fn test_nnls_pure_component() {
    // 2 genes, 2 cell types
    // reference: col0=[1,0], col1=[0,1]
    // mixture = [1,0] → should recover ~[1, 0]
    let reference = make_table(
        vec!["typeA", "typeB"],
        vec![
            vec![Value::Float(1.0), Value::Float(0.0)],
            vec![Value::Float(0.0), Value::Float(1.0)],
        ],
    );
    let mixture = float_list(&[1.0, 0.0]);
    let result = deconvolution::call_deconvolution_builtin("nnls", vec![mixture, reference])
        .expect("nnls failed");
    let fracs = get_list_floats(&result);
    assert_eq!(fracs.len(), 2);
    // typeA should dominate
    assert!(
        fracs[0] > 0.8,
        "typeA fraction should be > 0.8, got {}",
        fracs[0]
    );
    assert!(
        fracs[1] < 0.2,
        "typeB fraction should be < 0.2, got {}",
        fracs[1]
    );
}

#[test]
fn test_nnls_mixed() {
    // 2 genes, 2 cell types
    // reference: col0=[1,0], col1=[0,1]
    // mixture = [0.5, 0.5] → should recover ~[0.5, 0.5]
    let reference = make_table(
        vec!["typeA", "typeB"],
        vec![
            vec![Value::Float(1.0), Value::Float(0.0)],
            vec![Value::Float(0.0), Value::Float(1.0)],
        ],
    );
    let mixture = float_list(&[0.5, 0.5]);
    let result = deconvolution::call_deconvolution_builtin("nnls", vec![mixture, reference])
        .expect("nnls mixed failed");
    let fracs = get_list_floats(&result);
    assert_eq!(fracs.len(), 2);
    // Should be roughly equal
    assert!((fracs[0] - 0.5).abs() < 0.15, "got {}", fracs[0]);
    assert!((fracs[1] - 0.5).abs() < 0.15, "got {}", fracs[1]);
}

#[test]
fn test_nnls_sums_to_one() {
    let reference = make_table(
        vec!["t1", "t2", "t3"],
        vec![
            vec![Value::Float(2.0), Value::Float(0.5), Value::Float(0.1)],
            vec![Value::Float(0.1), Value::Float(1.5), Value::Float(0.3)],
            vec![Value::Float(0.3), Value::Float(0.2), Value::Float(2.0)],
        ],
    );
    let mixture = float_list(&[1.0, 1.0, 1.5]);
    let result = deconvolution::call_deconvolution_builtin("nnls", vec![mixture, reference])
        .expect("nnls sum");
    let fracs = get_list_floats(&result);
    let sum: f64 = fracs.iter().sum();
    assert!((sum - 1.0).abs() < 1e-6, "fractions sum = {sum}");
}

#[test]
fn test_deconvolve_two_samples() {
    // Pure typeA sample and pure typeB sample
    let reference = make_table(
        vec!["typeA", "typeB"],
        vec![
            vec![Value::Float(1.0), Value::Float(0.0)],
            vec![Value::Float(0.0), Value::Float(1.0)],
        ],
    );
    // bulk: 2 genes × 2 samples. sample0 = pure A, sample1 = pure B
    let bulk = make_table(
        vec!["s0", "s1"],
        vec![
            vec![Value::Float(10.0), Value::Float(0.0)],
            vec![Value::Float(0.0), Value::Float(10.0)],
        ],
    );
    let result = deconvolution::call_deconvolution_builtin("deconvolve", vec![bulk, reference])
        .expect("deconvolve");
    let tbl = get_table(result);
    assert_eq!(tbl.rows.len(), 2); // 2 cell types
                                   // Row 0 = typeA fractions in [s0, s1]
    let a_in_s0 = get_float(&tbl.rows[0][0]);
    let b_in_s1 = get_float(&tbl.rows[1][1]);
    assert!(a_in_s0 > 0.8, "typeA in s0 = {a_in_s0}");
    assert!(b_in_s1 > 0.8, "typeB in s1 = {b_in_s1}");
}

#[test]
fn test_estimate_purity_column_lookup() {
    // fractions table with a column matching tumor_col
    let fracs = make_table(
        vec!["tumor", "immune"],
        vec![vec![Value::Float(0.7), Value::Float(0.3)]],
    );
    // estimate_purity(fracs, "tumor") — "tumor" is a column name (sample in this framing)
    let result = deconvolution::call_deconvolution_builtin(
        "estimate_purity",
        vec![fracs, Value::Str("tumor".to_string())],
    )
    .expect("estimate_purity");
    let vals = get_list_floats(&result);
    assert!(!vals.is_empty());
}

#[test]
fn test_cell_type_correlation_diagonal() {
    // Each row is a cell type, columns are samples
    // Self-correlation should be 1.0
    let fracs = make_table(
        vec!["s0", "s1", "s2"],
        vec![
            vec![Value::Float(0.2), Value::Float(0.5), Value::Float(0.8)],
            vec![Value::Float(0.5), Value::Float(0.3), Value::Float(0.1)],
        ],
    );
    let result = deconvolution::call_deconvolution_builtin("cell_type_correlation", vec![fracs])
        .expect("correlation");
    let tbl = get_table(result);
    // 2×2 matrix; diagonals should be 1.0
    let r00 = get_float(&tbl.rows[0][0]);
    let r11 = get_float(&tbl.rows[1][1]);
    assert!((r00 - 1.0).abs() < 1e-9, "r00 = {r00}");
    assert!((r11 - 1.0).abs() < 1e-9, "r11 = {r11}");
}

// ── metabolomics tests ────────────────────────────────────────────────

#[test]
fn test_mz_match_hit() {
    let db = make_table(
        vec!["name", "formula", "exact_mz"],
        vec![
            vec![
                Value::Str("glucose".to_string()),
                Value::Str("C6H12O6".to_string()),
                Value::Float(180.0634),
            ],
            vec![
                Value::Str("fructose".to_string()),
                Value::Str("C6H12O6".to_string()),
                Value::Float(180.0634),
            ],
            vec![
                Value::Str("alanine".to_string()),
                Value::Str("C3H7NO2".to_string()),
                Value::Float(89.0477),
            ],
        ],
    );
    // Observe glucose + 1 ppm
    let obs = 180.0634 * (1.0 + 1.0 / 1e6);
    let result = metabolomics::call_metabolomics_builtin(
        "mz_match",
        vec![Value::Float(obs), db, Value::Float(5.0)],
    )
    .expect("mz_match");
    let tbl = get_table(result);
    assert_eq!(tbl.rows.len(), 2, "should match glucose+fructose");
    // ppm_error column is last
    let ppm = get_float(tbl.rows[0].last().unwrap());
    assert!(ppm <= 5.0, "ppm_error = {ppm}");
}

#[test]
fn test_mz_match_miss() {
    let db = make_table(
        vec!["name", "formula", "exact_mz"],
        vec![vec![
            Value::Str("alanine".to_string()),
            Value::Str("C3H7NO2".to_string()),
            Value::Float(89.0477),
        ]],
    );
    let result = metabolomics::call_metabolomics_builtin(
        "mz_match",
        vec![Value::Float(200.0), db, Value::Float(5.0)],
    )
    .expect("mz_match miss");
    let tbl = get_table(result);
    assert_eq!(tbl.rows.len(), 0);
}

#[test]
fn test_isotope_correct_trivial() {
    // 0 carbons → correction matrix is identity → output == input
    let intensities = float_list(&[100.0, 0.0]);
    let result = metabolomics::call_metabolomics_builtin(
        "isotope_correct",
        vec![intensities, Value::Int(0)],
    )
    .expect("isotope_correct trivial");
    let vals = get_list_floats(&result);
    assert_eq!(vals.len(), 2);
    // With 0 carbons p13^0 * p12^0 = 1 at diagonal, off-diag = 0
    assert!((vals[0] - 100.0).abs() < 0.1, "vals[0] = {}", vals[0]);
}

#[test]
fn test_feature_group_groups_close_features() {
    let tbl = make_table(
        vec!["mz", "rt"],
        vec![
            vec![Value::Float(200.0), Value::Float(1.0)],
            vec![Value::Float(200.0005), Value::Float(1.0)], // ~2.5 ppm apart, same rt
            vec![Value::Float(300.0), Value::Float(5.0)],    // far away
        ],
    );
    let result = metabolomics::call_metabolomics_builtin(
        "feature_group",
        vec![tbl, Value::Float(5.0), Value::Float(0.1)],
    )
    .expect("feature_group");
    let out = get_table(result);
    assert_eq!(out.rows.len(), 3);
    // Group ID column is last
    let g0 = match out.rows[0].last().unwrap() {
        Value::Int(n) => *n,
        _ => panic!("expected Int"),
    };
    let g1 = match out.rows[1].last().unwrap() {
        Value::Int(n) => *n,
        _ => panic!("expected Int"),
    };
    let g2 = match out.rows[2].last().unwrap() {
        Value::Int(n) => *n,
        _ => panic!("expected Int"),
    };
    assert_eq!(g0, g1, "features 0 and 1 should be in the same group");
    assert_ne!(g0, g2, "feature 2 should be in a different group");
}

#[test]
fn test_log_transform() {
    let tbl = make_table(
        vec!["s1", "s2"],
        vec![
            vec![Value::Float(0.0), Value::Float(1.0)],
            vec![Value::Float(3.0), Value::Float(7.0)],
        ],
    );
    let result =
        metabolomics::call_metabolomics_builtin("log_transform", vec![tbl]).expect("log_transform");
    let out = get_table(result);
    // log2(0+1) = 0; log2(1+1) = 1; log2(3+1) = 2; log2(7+1) = 3
    assert_eq!(get_float(&out.rows[0][0]), 0.0);
    assert!((get_float(&out.rows[0][1]) - 1.0).abs() < 1e-9);
    assert!((get_float(&out.rows[1][0]) - 2.0).abs() < 1e-9);
    assert!((get_float(&out.rows[1][1]) - 3.0).abs() < 1e-9);
}

#[test]
fn test_pathway_enrichment_ordering() {
    // Larger background so glycolysis enrichment p is unambiguously lower
    // glycolysis: 4 members, all 4 hit
    // tca: 3 members, 0 hit
    // bg: glycolysis(4) + tca(3) + other(10) = 17 total
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for m in &["g1", "g2", "g3", "g4"] {
        rows.push(vec![
            Value::Str("glycolysis".to_string()),
            Value::Str(m.to_string()),
        ]);
    }
    for m in &["t1", "t2", "t3"] {
        rows.push(vec![
            Value::Str("tca".to_string()),
            Value::Str(m.to_string()),
        ]);
    }
    for i in 0..10usize {
        rows.push(vec![
            Value::Str("other".to_string()),
            Value::Str(format!("o{i}")),
        ]);
    }
    let db = make_table(vec!["pathway", "metabolite"], rows);

    // Hit all 4 glycolysis metabolites only
    let hits = Value::List(
        (vec![
            Value::Str("g1".to_string()),
            Value::Str("g2".to_string()),
            Value::Str("g3".to_string()),
            Value::Str("g4".to_string()),
        ])
        .into(),
    );
    let result = metabolomics::call_metabolomics_builtin("pathway_enrichment", vec![hits, db])
        .expect("pathway_enrichment");
    let out = get_table(result);
    assert_eq!(out.rows.len(), 3);
    // glycolysis should rank first (lowest p)
    let top_pw = match &out.rows[0][0] {
        Value::Str(s) => s.clone(),
        _ => panic!("expected Str"),
    };
    assert_eq!(top_pw, "glycolysis");
    // n_hits for glycolysis = 4
    let n_hits = match &out.rows[0][2] {
        Value::Int(n) => *n,
        _ => panic!("expected Int"),
    };
    assert_eq!(n_hits, 4);
}

#[test]
fn test_normalize_samples_sum() {
    let tbl = make_table(
        vec!["s1", "s2"],
        vec![
            vec![Value::Float(10.0), Value::Float(20.0)],
            vec![Value::Float(90.0), Value::Float(80.0)],
        ],
    );
    let result = metabolomics::call_metabolomics_builtin(
        "normalize_samples",
        vec![tbl, Value::Str("sum".to_string())],
    )
    .expect("normalize sum");
    let out = get_table(result);
    // s1 sums to 100, s2 sums to 100
    let s1_sum: f64 = out.rows.iter().map(|r| get_float(&r[0])).sum();
    let s2_sum: f64 = out.rows.iter().map(|r| get_float(&r[1])).sum();
    assert!((s1_sum - 1.0).abs() < 1e-9, "s1_sum = {s1_sum}");
    assert!((s2_sum - 1.0).abs() < 1e-9, "s2_sum = {s2_sum}");
}

#[test]
fn test_normalize_samples_quantile() {
    let tbl = make_table(
        vec!["s1", "s2"],
        vec![
            vec![Value::Float(1.0), Value::Float(4.0)],
            vec![Value::Float(2.0), Value::Float(3.0)],
            vec![Value::Float(3.0), Value::Float(2.0)],
            vec![Value::Float(4.0), Value::Float(1.0)],
        ],
    );
    let result = metabolomics::call_metabolomics_builtin(
        "normalize_samples",
        vec![tbl, Value::Str("quantile".to_string())],
    )
    .expect("quantile normalize");
    let out = get_table(result);
    // After quantile normalization, all samples should have the same sorted distribution
    let s1: Vec<f64> = out.rows.iter().map(|r| get_float(&r[0])).collect();
    let s2: Vec<f64> = out.rows.iter().map(|r| get_float(&r[1])).collect();
    let mut s1s = s1.clone();
    s1s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut s2s = s2.clone();
    s2s.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for (a, b) in s1s.iter().zip(s2s.iter()) {
        assert!(
            (a - b).abs() < 1e-9,
            "sorted distributions differ: {a} vs {b}"
        );
    }
}
