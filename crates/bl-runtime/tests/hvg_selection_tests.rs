//! highly_variable_genes must rank dispersion against expression level.
//!
//! It ranked cv2 = variance / mean^2, which is maximised by genes with a
//! near-zero mean: a transcript seen in two cells of 2700 has an enormous cv2
//! by chance. On PBMC3k that selected lncRNAs and lowly-expressed genes while
//! every canonical marker - LYZ, MS4A1, GNLY, PPBP, CD14, NKG7, CD8A - was
//! absent from the top 2000, and clustering then split the sample into 17
//! groups where it has nine populations.

use bl_core::value::Value;
use bl_runtime::singlecell::call_singlecell_builtin;

fn matrix_value(rows: Vec<Vec<f64>>) -> Value {
    Value::List(
        rows.into_iter()
            .map(|row| Value::List(row.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn selected(rows: Vec<Vec<f64>>, n: i64) -> Vec<usize> {
    let result = call_singlecell_builtin(
        "highly_variable_genes",
        vec![matrix_value(rows), Value::Int(n)],
    )
    .unwrap();
    match result {
        Value::List(items) => items
            .iter()
            .map(|v| match v {
                Value::Int(i) => *i as usize,
                other => panic!("expected Int, got {other:?}"),
            })
            .collect(),
        other => panic!("expected List, got {other:?}"),
    }
}

/// Gene 0 is genuinely bimodal at a healthy expression level - the shape a
/// marker gene has. Gene 1 is a rare transcript caught in a single cell, which
/// is noise. Ranking by cv2 preferred gene 1; it must not.
#[test]
fn a_real_marker_beats_a_single_cell_blip() {
    let cells = 40;
    let mut rows = Vec::new();
    for i in 0..cells {
        let marker = if i < cells / 2 { 12.0 } else { 0.5 };
        let blip = if i == 0 { 1.0 } else { 0.0 };
        // A pair of steady housekeeping genes, so the bins have company.
        rows.push(vec![marker, blip, 6.0, 5.5]);
    }
    let top = selected(rows, 1);
    assert_eq!(
        top,
        vec![0],
        "expected the bimodal marker (gene 0), not the single-cell blip (gene 1)"
    );
}

#[test]
fn genes_never_observed_are_not_selected() {
    let rows = (0..30)
        .map(|i| vec![if i < 15 { 9.0 } else { 1.0 }, 0.0, 0.0, 4.0])
        .collect();
    let top = selected(rows, 3);
    assert!(!top.contains(&1), "an all-zero gene was selected: {top:?}");
    assert!(!top.contains(&2), "an all-zero gene was selected: {top:?}");
}

#[test]
fn selection_is_capped_by_the_gene_count() {
    let rows = (0..12).map(|i| vec![i as f64, 3.0, 7.0]).collect();
    assert!(selected(rows, 50).len() <= 3);
}

#[test]
fn requesting_none_returns_none() {
    let rows = (0..8).map(|i| vec![i as f64, 2.0]).collect();
    assert!(selected(rows, 0).is_empty());
}
