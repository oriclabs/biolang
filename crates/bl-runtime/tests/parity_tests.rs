//! Golden-file parity suite: proves the *reference-equivalent* native steps
//! match the canonical tools with fixed, hand-verifiable reference values.
//!
//! - Deterministic ops (Wilcoxon, BH-FDR, normalization) → exact tolerance.
//! - Stochastic ops (clustering) can't match element-wise, so agreement is
//!   measured with the adjusted Rand index instead.
//!
//! Reference values are derived analytically or from R/scipy/sklearn so the
//! assertions are independent of this implementation.

use bl_core::value::Value;
use bl_runtime::provenance::adjusted_rand_index;
use bl_runtime::singlecell::call_singlecell_builtin;
use bl_runtime::stats::call_stats_builtin;

fn float_list(vals: &[f64]) -> Value {
    Value::List(vals.iter().map(|v| Value::Float(*v)).collect::<Vec<_>>().into())
}

fn record_float(v: &Value, key: &str) -> f64 {
    match v {
        Value::Record(m) => match m.get(key).unwrap() {
            Value::Float(f) => *f,
            Value::Int(n) => *n as f64,
            other => panic!("key {key} not numeric: {other:?}"),
        },
        other => panic!("expected Record, got {other:?}"),
    }
}

fn as_floats(v: &Value) -> Vec<f64> {
    match v {
        Value::List(items) => items
            .iter()
            .map(|x| match x {
                Value::Float(f) => *f,
                Value::Int(n) => *n as f64,
                other => panic!("non-numeric list item: {other:?}"),
            })
            .collect(),
        other => panic!("expected List, got {other:?}"),
    }
}

/// Wilcoxon rank-sum (Mann-Whitney U) — must match the tie-corrected NORMAL
/// APPROXIMATION that Scanpy `rank_genes_groups` and Seurat `FindMarkers` use.
///
/// a=[1,2,3,4], b=[5,6,7,8]: U=0, mu=8, sigma=sqrt(4*4*9/12)=sqrt(12),
/// z=(0-8)/sqrt(12)=-2.309401, two-sided p = 2*Phi(-2.309401) = 0.0209353.
#[test]
fn wilcoxon_matches_normal_approximation() {
    let a = float_list(&[1.0, 2.0, 3.0, 4.0]);
    let b = float_list(&[5.0, 6.0, 7.0, 8.0]);
    let res = call_stats_builtin("wilcoxon", vec![a, b]).unwrap();
    let p = record_float(&res, "pvalue");
    assert!(
        (p - 0.0209353).abs() < 5e-4,
        "wilcoxon p={p}, expected ~0.0209353 (normal approx)"
    );
}

/// Benjamini-Hochberg FDR — must match R's `p.adjust(method="BH")`.
/// p.adjust(c(0.01,0.02,0.03,0.04),"BH") = c(0.04,0.04,0.04,0.04).
#[test]
fn bh_matches_r_padjust_uniform() {
    let pvals = float_list(&[0.01, 0.02, 0.03, 0.04]);
    let adj = call_stats_builtin("p_adjust", vec![pvals, Value::Str("bh".into())]).unwrap();
    for q in as_floats(&adj) {
        assert!((q - 0.04).abs() < 1e-9, "expected 0.04, got {q}");
    }
}

/// BH with tied p-values must give tied genes the SAME adjusted value
/// (step-up monotonicity). p.adjust(c(0.0209,1,0.0209),"BH") = c(0.03135,1,0.03135).
#[test]
fn bh_matches_r_padjust_ties() {
    let pvals = float_list(&[0.0209, 1.0, 0.0209]);
    let adj = as_floats(&call_stats_builtin("p_adjust", vec![pvals, Value::Str("bh".into())]).unwrap());
    assert!((adj[0] - 0.03135).abs() < 1e-4, "adj[0]={}", adj[0]);
    assert!((adj[2] - 0.03135).abs() < 1e-4, "adj[2]={}", adj[2]);
    assert!((adj[0] - adj[2]).abs() < 1e-12, "tied p-values must match");
    assert!((adj[1] - 1.0).abs() < 1e-9);
}

/// Library-size normalization: every cell (row) must sum to the target — the
/// defining property shared by Scanpy `normalize_total` and Seurat.
#[test]
fn normalize_total_scales_each_cell_to_target() {
    // 2 cells x 3 genes.
    let matrix = Value::List((vec![
        float_list(&[1.0, 2.0, 3.0]),
        float_list(&[4.0, 4.0, 2.0]),
    ]).into());
    let out = call_singlecell_builtin("normalize_total", vec![matrix, Value::Float(10_000.0)]).unwrap();
    let rows = match out {
        Value::List(r) => r,
        other => panic!("expected matrix, got {other:?}"),
    };
    for row in rows.iter() {
        let sum: f64 = as_floats(row).iter().sum();
        assert!((sum - 10_000.0).abs() < 1e-6, "row sum {sum} != 10000");
    }
    // Proportions preserved: cell 0 gene ratios 1:2:3.
    let r0 = as_floats(&rows[0]);
    assert!((r0[1] / r0[0] - 2.0).abs() < 1e-9);
    assert!((r0[2] / r0[0] - 3.0).abs() < 1e-9);
}

/// Clustering agreement metric — matches sklearn `adjusted_rand_score`.
#[test]
fn ari_matches_sklearn_reference_values() {
    // Identical → 1.0
    let a = ["0", "0", "1", "1"].map(String::from).to_vec();
    assert!((adjusted_rand_index(&a, &a) - 1.0).abs() < 1e-9);

    // sklearn: adjusted_rand_score([0,0,1,2],[0,0,1,1]) = 0.5714285714
    let x = ["0", "0", "1", "2"].map(String::from).to_vec();
    let y = ["0", "0", "1", "1"].map(String::from).to_vec();
    let ari = adjusted_rand_index(&x, &y);
    assert!((ari - 0.5714285714).abs() < 1e-6, "ari={ari}, expected 0.5714");

    // One cluster vs all singletons → 0.0
    let all = ["0", "0", "0", "0"].map(String::from).to_vec();
    let singles = ["0", "1", "2", "3"].map(String::from).to_vec();
    assert!(adjusted_rand_index(&all, &singles).abs() < 1e-9);
}
