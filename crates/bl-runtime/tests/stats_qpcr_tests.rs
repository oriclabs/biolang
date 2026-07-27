//! Integration tests for statistics.rs and qpcr.rs builtins loaded via #[path].
//! These modules are not yet wired into lib.rs, so we load them directly.

#[path = "../src/statistics.rs"]
mod statistics;

#[path = "../src/qpcr.rs"]
mod qpcr;

use bl_core::value::{Table, Value};
use statistics::{call_statistics_builtin, is_statistics_builtin, statistics_builtin_list};
use qpcr::{call_qpcr_builtin, is_qpcr_builtin, qpcr_builtin_list};

// ── Helpers ───────────────────────────────────────────────────────────

fn float_list(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|&v| Value::Float(v)).collect::<Vec<_>>().into())
}

fn int_list(vals: &[i64]) -> Value {
    Value::List(vals.iter().map(|&v| Value::Int(v)).collect::<Vec<_>>().into())
}

fn extract_float(val: &Value) -> f64 {
    match val {
        Value::Float(f) => *f,
        Value::Int(n) => *n as f64,
        _ => panic!("expected Float, got {val:?}"),
    }
}

fn table_val(columns: Vec<&str>, rows: Vec<Vec<f64>>) -> Value {
    let cols: Vec<String> = columns.into_iter().map(|s| s.to_string()).collect();
    let r: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| row.into_iter().map(Value::Float).collect())
        .collect();
    Value::Table(Table::new(cols, r))
}

fn table_row0_float(val: &Value, col: usize) -> f64 {
    match val {
        Value::Table(t) => extract_float(&t.rows[0][col]),
        _ => panic!("expected Table, got {val:?}"),
    }
}

fn list_floats(val: &Value) -> Vec<f64> {
    match val {
        Value::List(items) => items.iter().map(extract_float).collect(),
        _ => panic!("expected List"),
    }
}

// ── Registry tests ────────────────────────────────────────────────────

#[test]
fn stats_registry_complete() {
    let list = statistics_builtin_list();
    assert!(list.len() >= 8);
    for (name, _) in &list {
        assert!(is_statistics_builtin(name), "{name} missing from is_statistics_builtin");
    }
    assert!(!is_statistics_builtin("not_a_real_builtin"));
}

#[test]
fn qpcr_registry_complete() {
    let list = qpcr_builtin_list();
    assert!(list.len() >= 5);
    for (name, _) in &list {
        assert!(is_qpcr_builtin(name), "{name} missing from is_qpcr_builtin");
    }
    assert!(!is_qpcr_builtin("not_a_real_builtin"));
}

// ── BH adjustment ────────────────────────────────────────────────────

#[test]
fn bh_adjust_basic() {
    let ps = float_list(&[0.01, 0.04, 0.03, 0.2, 0.5]);
    let result = call_statistics_builtin("bh_adjust", vec![ps]).unwrap();
    let adj = list_floats(&result);
    assert_eq!(adj.len(), 5);
    // All adjusted values must be >= original p-values and <= 1.0
    let orig = [0.01f64, 0.04, 0.03, 0.2, 0.5];
    for (a, &p) in adj.iter().zip(orig.iter()) {
        assert!(*a >= p - 1e-10, "adjusted {a} < original {p}");
        assert!(*a <= 1.0 + 1e-10, "adjusted {a} > 1.0");
    }
    // Monotonicity check: sorted original order → sorted adjusted must be non-decreasing
    let mut sorted_adj = adj.clone();
    sorted_adj.sort_by(|a, b| a.partial_cmp(b).unwrap());
    for w in sorted_adj.windows(2) {
        assert!(w[0] <= w[1] + 1e-10);
    }
}

#[test]
fn bh_adjust_single_value() {
    let ps = float_list(&[0.05]);
    let result = call_statistics_builtin("bh_adjust", vec![ps]).unwrap();
    let adj = list_floats(&result);
    assert_eq!(adj.len(), 1);
    assert!((adj[0] - 0.05).abs() < 1e-10);
}

// ── Bonferroni adjustment ─────────────────────────────────────────────

#[test]
fn bonferroni_adjust_basic() {
    let ps = float_list(&[0.01, 0.02, 0.04]);
    let result = call_statistics_builtin("bonferroni_adjust", vec![ps]).unwrap();
    let adj = list_floats(&result);
    let expected = [0.03f64, 0.06, 0.12];
    for (a, &e) in adj.iter().zip(expected.iter()) {
        assert!((a - e).abs() < 1e-10, "got {a}, expected {e}");
    }
}

#[test]
fn bonferroni_adjust_clamps_to_1() {
    let ps = float_list(&[0.5, 0.5, 0.5]);
    let result = call_statistics_builtin("bonferroni_adjust", vec![ps]).unwrap();
    let adj = list_floats(&result);
    for &a in &adj {
        assert!((a - 1.0).abs() < 1e-10);
    }
}

// ── Fisher exact test ─────────────────────────────────────────────────

#[test]
fn fisher_exact_clear_significance() {
    // a=10, b=0, c=0, d=10 → p should be very small
    let result = call_statistics_builtin(
        "fisher_exact",
        vec![Value::Int(10), Value::Int(0), Value::Int(0), Value::Int(10)],
    )
    .unwrap();
    let p = table_row0_float(&result, 0);
    assert!(p < 0.01, "expected p < 0.01, got {p}");
}

#[test]
fn fisher_exact_no_association() {
    // a=5, b=5, c=5, d=5 → odds ratio = 1, p near 1
    let result = call_statistics_builtin(
        "fisher_exact",
        vec![Value::Int(5), Value::Int(5), Value::Int(5), Value::Int(5)],
    )
    .unwrap();
    let or_ = table_row0_float(&result, 1);
    assert!((or_ - 1.0).abs() < 1e-10, "expected OR=1.0, got {or_}");
}

// ── Chi-square test ───────────────────────────────────────────────────

#[test]
fn chi_square_uniform() {
    // Perfectly uniform observed == expected → statistic = 0
    let obs = float_list(&[10.0, 10.0, 10.0]);
    let exp = float_list(&[10.0, 10.0, 10.0]);
    let result = call_statistics_builtin("chi_square", vec![obs, exp]).unwrap();
    let stat = table_row0_float(&result, 0);
    assert!(stat.abs() < 1e-10, "expected statistic≈0, got {stat}");
    let p = table_row0_float(&result, 2);
    assert!(p > 0.9, "expected p≈1.0, got {p}");
}

#[test]
fn chi_square_large_deviation() {
    // Extreme deviation → large statistic, small p
    let obs = float_list(&[100.0, 0.0, 0.0]);
    let exp = float_list(&[33.3, 33.3, 33.3]);
    let result = call_statistics_builtin("chi_square", vec![obs, exp]).unwrap();
    let stat = table_row0_float(&result, 0);
    assert!(stat > 100.0, "expected large statistic, got {stat}");
    let p = table_row0_float(&result, 2);
    assert!(p < 0.001, "expected p < 0.001, got {p}");
}

// ── Permutation test ──────────────────────────────────────────────────

#[test]
fn permutation_test_identical_groups() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let b = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let result = call_statistics_builtin("permutation_test", vec![a, b, Value::Int(500)]).unwrap();
    let p = extract_float(&result);
    assert!(p > 0.2, "identical groups should have high p, got {p}");
}

#[test]
fn permutation_test_different_groups() {
    let a = float_list(&[100.0, 110.0, 105.0, 108.0, 102.0]);
    let b = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let result = call_statistics_builtin("permutation_test", vec![a, b, Value::Int(500)]).unwrap();
    let p = extract_float(&result);
    assert!(p < 0.1, "very different groups should have low p, got {p}");
}

// ── Bootstrap CI ──────────────────────────────────────────────────────

#[test]
fn bootstrap_ci_contains_true_mean() {
    let vals = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let result =
        call_statistics_builtin("bootstrap_ci", vec![vals, Value::Int(500), Value::Float(0.95)])
            .unwrap();
    let mean  = table_row0_float(&result, 0);
    let lower = table_row0_float(&result, 1);
    let upper = table_row0_float(&result, 2);
    assert!((mean - 3.0).abs() < 1e-10, "expected mean=3.0, got {mean}");
    assert!(lower < mean, "lower {lower} >= mean {mean}");
    assert!(upper > mean, "upper {upper} <= mean {mean}");
}

// ── Genomic inflation ─────────────────────────────────────────────────

#[test]
fn genomic_inflation_uniform_ps_near_one() {
    // Uniformly spaced p-values → lambda ≈ 1.0 (no inflation)
    let ps: Vec<f64> = (1..=100).map(|i| i as f64 / 101.0).collect();
    let result = call_statistics_builtin("genomic_inflation", vec![float_list(&ps)]).unwrap();
    let lambda = extract_float(&result);
    assert!(
        (lambda - 1.0).abs() < 0.2,
        "expected lambda≈1.0 for uniform ps, got {lambda}"
    );
}

#[test]
fn genomic_inflation_inflated() {
    // All very small p-values → lambda >> 1
    let ps: Vec<f64> = (1..=20).map(|i| i as f64 * 1e-5).collect();
    let result = call_statistics_builtin("genomic_inflation", vec![float_list(&ps)]).unwrap();
    let lambda = extract_float(&result);
    assert!(lambda > 1.5, "expected lambda > 1.5 for small ps, got {lambda}");
}

// ── Pearson correlation ───────────────────────────────────────────────

#[test]
fn pearson_correlation_perfect_positive() {
    let x = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = float_list(&[2.0, 4.0, 6.0, 8.0, 10.0]);
    let result = call_statistics_builtin("pearson_correlation", vec![x, y]).unwrap();
    let r = extract_float(&result);
    assert!((r - 1.0).abs() < 1e-10, "expected r=1.0, got {r}");
}

#[test]
fn pearson_correlation_perfect_negative() {
    let x = float_list(&[1.0, 2.0, 3.0, 4.0, 5.0]);
    let y = float_list(&[10.0, 8.0, 6.0, 4.0, 2.0]);
    let result = call_statistics_builtin("pearson_correlation", vec![x, y]).unwrap();
    let r = extract_float(&result);
    assert!((r + 1.0).abs() < 1e-10, "expected r=-1.0, got {r}");
}

#[test]
fn pearson_correlation_zero_variance() {
    let x = float_list(&[1.0, 1.0, 1.0]);
    let y = float_list(&[1.0, 2.0, 3.0]);
    let result = call_statistics_builtin("pearson_correlation", vec![x, y]).unwrap();
    let r = extract_float(&result);
    assert!(r.abs() < 1e-10, "expected r=0.0 for zero-variance x, got {r}");
}

// ── qPCR: delta_ct ────────────────────────────────────────────────────

#[test]
fn delta_ct_scalar() {
    let result = call_qpcr_builtin("delta_ct", vec![Value::Float(28.5), Value::Float(25.0)]).unwrap();
    let dct = extract_float(&result);
    assert!((dct - 3.5).abs() < 1e-10, "expected 3.5, got {dct}");
}

#[test]
fn delta_ct_lists() {
    let s = float_list(&[28.0, 29.0, 30.0]);
    let r = float_list(&[25.0, 25.0, 25.0]);
    let result = call_qpcr_builtin("delta_ct", vec![s, r]).unwrap();
    let dcts = list_floats(&result);
    assert_eq!(dcts, vec![3.0, 4.0, 5.0]);
}

// ── qPCR: delta_delta_ct ──────────────────────────────────────────────

#[test]
fn delta_delta_ct_no_change() {
    // ΔΔCt = 0 → fold change = 2^0 = 1.0
    let result = call_qpcr_builtin("delta_delta_ct", vec![Value::Float(3.0), Value::Float(3.0)]).unwrap();
    let fc = extract_float(&result);
    assert!((fc - 1.0).abs() < 1e-10, "expected fold change=1.0, got {fc}");
}

#[test]
fn delta_delta_ct_two_fold_up() {
    // ΔCt_sample - ΔCt_control = -1 → 2^1 = 2.0
    let result = call_qpcr_builtin("delta_delta_ct", vec![Value::Float(2.0), Value::Float(3.0)]).unwrap();
    let fc = extract_float(&result);
    assert!((fc - 2.0).abs() < 1e-10, "expected fold change=2.0, got {fc}");
}

// ── qPCR: pcr_efficiency ─────────────────────────────────────────────

#[test]
fn pcr_efficiency_ideal() {
    // Ideal PCR: slope = -3.32 → efficiency ≈ 1.0 (100%)
    let cts  = float_list(&[30.0, 26.68, 23.36, 20.04, 16.72]);
    let ldil = float_list(&[0.0, 1.0, 2.0, 3.0, 4.0]);
    let result = call_qpcr_builtin("pcr_efficiency", vec![cts, ldil]).unwrap();
    let eff = table_row0_float(&result, 0);
    assert!((eff - 1.0).abs() < 0.05, "expected efficiency≈1.0, got {eff}");
}

// ── qPCR: reference_normalize ────────────────────────────────────────

#[test]
fn reference_normalize_basic() {
    // 3 genes × 2 samples; gene 0 is the reference
    // ct_table rows: gene0=[20,20], gene1=[25,25], gene2=[30,28]
    let t = table_val(
        vec!["s1", "s2"],
        vec![vec![20.0, 20.0], vec![25.0, 25.0], vec![30.0, 28.0]],
    );
    let ref_idx = int_list(&[0]);
    let result = call_qpcr_builtin("reference_normalize", vec![t, ref_idx]).unwrap();
    match &result {
        Value::Table(t) => {
            // gene0 row should be all zeros
            assert!((extract_float(&t.rows[0][0]) - 0.0).abs() < 1e-10);
            assert!((extract_float(&t.rows[0][1]) - 0.0).abs() < 1e-10);
            // gene1 row: 25-20 = 5, 25-20 = 5
            assert!((extract_float(&t.rows[1][0]) - 5.0).abs() < 1e-10);
            assert!((extract_float(&t.rows[1][1]) - 5.0).abs() < 1e-10);
            // gene2 row: 30-20 = 10, 28-20 = 8
            assert!((extract_float(&t.rows[2][0]) - 10.0).abs() < 1e-10);
            assert!((extract_float(&t.rows[2][1]) - 8.0).abs() < 1e-10);
        }
        _ => panic!("expected Table"),
    }
}

// ── qPCR: genorm_stability ────────────────────────────────────────────

#[test]
fn genorm_stability_most_stable_first() {
    // 4 genes × 4 samples; genes 0,1,2,3 are all candidate refs
    // Make gene 0 very stable (constant ratio), genes 2,3 more variable
    let t = table_val(
        vec!["s1", "s2", "s3", "s4"],
        vec![
            vec![10.0, 10.0, 10.0, 10.0], // gene0: perfectly stable
            vec![11.0, 11.0, 11.0, 11.0], // gene1: also stable
            vec![10.0, 12.0, 8.0, 14.0],  // gene2: variable
            vec![10.0, 14.0, 6.0, 16.0],  // gene3: most variable
        ],
    );
    let ref_idx = int_list(&[0, 1, 2, 3]);
    let result = call_qpcr_builtin("genorm_stability", vec![t, ref_idx]).unwrap();
    match &result {
        Value::Table(t) => {
            assert_eq!(t.rows.len(), 4, "should have 4 stability scores");
            // First row should have the lowest (best) M-score
            let first_m  = extract_float(&t.rows[0][1]);
            let last_m   = extract_float(&t.rows[t.rows.len() - 1][1]);
            assert!(first_m <= last_m, "expected ascending M-scores, first={first_m} last={last_m}");
        }
        _ => panic!("expected Table"),
    }
}

// ── Error cases ───────────────────────────────────────────────────────

#[test]
fn bh_adjust_type_error() {
    let bad = Value::Str("not_a_list".to_string());
    assert!(call_statistics_builtin("bh_adjust", vec![bad]).is_err());
}

#[test]
fn chi_square_length_mismatch() {
    let obs = float_list(&[1.0, 2.0]);
    let exp = float_list(&[1.0, 2.0, 3.0]);
    assert!(call_statistics_builtin("chi_square", vec![obs, exp]).is_err());
}

#[test]
fn delta_ct_length_mismatch() {
    let s = float_list(&[28.0, 29.0]);
    let r = float_list(&[25.0]);
    assert!(call_qpcr_builtin("delta_ct", vec![s, r]).is_err());
}

#[test]
fn unknown_statistics_builtin() {
    let result = call_statistics_builtin("no_such_fn", vec![]);
    assert!(result.is_err());
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("no_such_fn"), "error should mention builtin name");
}

#[test]
fn unknown_qpcr_builtin() {
    let result = call_qpcr_builtin("no_such_fn", vec![]);
    assert!(result.is_err());
}
