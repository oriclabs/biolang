// Tests for proteomics.rs and methylation.rs using the #[path] trick so
// we can exercise the modules before they are registered in lib.rs.

#[path = "../src/proteomics.rs"]
mod proteomics;

#[path = "../src/methylation.rs"]
mod methylation;

use bl_core::value::Value;
use proteomics::call_proteomics_builtin;
use methylation::call_methylation_builtin;

// ── helpers ───────────────────────────────────────────────────────────

fn float_list(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|&v| Value::Float(v)).collect::<Vec<_>>().into())
}

fn int_list(vals: &[i64]) -> Value {
    Value::List(vals.iter().map(|&v| Value::Int(v)).collect::<Vec<_>>().into())
}

fn matrix(rows: Vec<Vec<f64>>) -> Value {
    Value::List(
        rows.into_iter()
            .map(|r| Value::List(r.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>().into(),
    )
}

fn table_col(val: &Value, col: &str) -> Vec<f64> {
    match val {
        Value::Table(t) => {
            let idx = t.columns.iter().position(|c| c == col).unwrap();
            t.rows.iter().map(|row| match &row[idx] {
                Value::Float(f) => *f,
                Value::Int(n) => *n as f64,
                _ => f64::NAN,
            }).collect()
        }
        _ => panic!("expected Table"),
    }
}

fn table_str_col(val: &Value, col: &str) -> Vec<String> {
    match val {
        Value::Table(t) => {
            let idx = t.columns.iter().position(|c| c == col).unwrap();
            t.rows.iter().map(|row| match &row[idx] {
                Value::Str(s) => s.clone(),
                _ => "".to_string(),
            }).collect()
        }
        _ => panic!("expected Table"),
    }
}

fn list_floats(val: &Value) -> Vec<f64> {
    match val {
        Value::List(items) => items.iter().map(|v| match v {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            _ => f64::NAN,
        }).collect(),
        _ => panic!("expected List"),
    }
}

// ── proteomics tests ──────────────────────────────────────────────────

#[test]
fn log2_transform_adds_one_and_logs() {
    // log2(0 + 1) = 0, log2(1 + 1) = 1, log2(3 + 1) = 2
    let mat = matrix(vec![vec![0.0, 1.0, 3.0]]);
    let result = call_proteomics_builtin("log2_transform", vec![mat]).unwrap();
    let vals = list_floats(match &result {
        Value::List(rows) => &rows[0],
        _ => panic!(),
    });
    assert!((vals[0] - 0.0).abs() < 1e-9);
    assert!((vals[1] - 1.0).abs() < 1e-9);
    assert!((vals[2] - 2.0).abs() < 1e-9);
}

#[test]
fn impute_minvalue_replaces_zeros() {
    // Column mins (non-zero): col0=2, col1=4 → impute at 0.5× → 1.0, 2.0
    let mat = matrix(vec![
        vec![2.0, 0.0],
        vec![0.0, 4.0],
    ]);
    let result = call_proteomics_builtin(
        "impute_minvalue",
        vec![mat, Value::Float(0.5)],
    ).unwrap();
    let rows = match &result {
        Value::List(r) => r.clone(),
        _ => panic!(),
    };
    // Row 0, col1 was 0 → imputed to 4.0 * 0.5 = 2.0
    let r0 = list_floats(&rows[0]);
    assert!((r0[1] - 2.0).abs() < 1e-9);
    // Row 1, col0 was 0 → imputed to 2.0 * 0.5 = 1.0
    let r1 = list_floats(&rows[1]);
    assert!((r1[0] - 1.0).abs() < 1e-9);
}

#[test]
fn protein_ttest_detects_difference() {
    // 3 proteins × 6 samples (0-2 = group A, 3-5 = group B)
    // Protein 0: all group A = 10, all group B = 0  → large log2FC, small p
    // Protein 1: all = 5 → no difference
    let mat = matrix(vec![
        vec![10.0, 10.0, 10.0, 0.0, 0.0, 0.0],
        vec![ 5.0,  5.0,  5.0, 5.0, 5.0, 5.0],
    ]);
    let idx_a = int_list(&[0, 1, 2]);
    let idx_b = int_list(&[3, 4, 5]);
    let result =
        call_proteomics_builtin("protein_ttest", vec![mat, idx_a, idx_b]).unwrap();
    let pvals = table_col(&result, "p_value");
    let fcs   = table_col(&result, "log2fc");
    // Protein 0: significant, positive FC
    assert!(pvals[0] < 0.05, "protein 0 p_value should be < 0.05, got {}", pvals[0]);
    assert!(fcs[0] > 1.0, "protein 0 log2fc should be > 1, got {}", fcs[0]);
    // Protein 1: not significant (same in both groups)
    assert!(pvals[1] >= 0.999, "protein 1 p_value should be ~1, got {}", pvals[1]);
}

// ── methylation tests ─────────────────────────────────────────────────

#[test]
fn beta_mvalue_roundtrip() {
    let betas = vec![0.1, 0.5, 0.9];
    let beta_val = float_list(&betas);
    let mvalues = call_methylation_builtin("beta_to_mvalue", vec![beta_val]).unwrap();
    let back = call_methylation_builtin("mvalue_to_beta", vec![mvalues]).unwrap();
    let result = list_floats(&back);
    for (orig, got) in betas.iter().zip(result.iter()) {
        assert!(
            (orig - got).abs() < 1e-4,
            "roundtrip failed: orig={orig} got={got}"
        );
    }
}

#[test]
fn cpg_density_counts_dinucleotides() {
    // "ACGCGT" has CpG at pos 1 and 3 → 2 CpGs, length 6, density = 2/5 * 100 = 40
    let result = call_methylation_builtin(
        "cpg_density",
        vec![Value::Str("ACGCGT".to_string())],
    )
    .unwrap();
    let count = table_col(&result, "cpg_count");
    let density = table_col(&result, "density");
    assert_eq!(count[0] as i64, 2);
    assert!((density[0] - 40.0).abs() < 1e-6, "density={}", density[0]);
}

#[test]
fn differential_methylation_finds_hyper_cpgs() {
    // 4 CpGs, 4 samples (0-1 = group A, 2-3 = group B)
    // CpG 0: A=0.9, B=0.2 → delta=+0.7
    // CpG 1: A=0.2, B=0.9 → delta=-0.7
    // CpG 2: A=0.5, B=0.5 → delta=0
    let mat = matrix(vec![
        vec![0.9, 0.9, 0.2, 0.2],
        vec![0.2, 0.2, 0.9, 0.9],
        vec![0.5, 0.5, 0.5, 0.5],
    ]);
    let idx_a = int_list(&[0, 1]);
    let idx_b = int_list(&[2, 3]);
    let result = call_methylation_builtin(
        "differential_methylation",
        vec![mat, idx_a, idx_b],
    )
    .unwrap();
    let deltas = table_col(&result, "delta_beta");
    let pvals  = table_col(&result, "p_value");
    assert!(deltas[0] > 0.5, "CpG 0 delta should be ~0.7, got {}", deltas[0]);
    assert!(deltas[1] < -0.5, "CpG 1 delta should be ~-0.7, got {}", deltas[1]);
    assert!(pvals[0] < 0.05, "CpG 0 p_value should be significant, got {}", pvals[0]);
    assert!((deltas[2]).abs() < 1e-9, "CpG 2 delta should be 0, got {}", deltas[2]);
}
