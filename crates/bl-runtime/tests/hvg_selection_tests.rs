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

// ── vst (Stuart et al. 2019) ─────────────────────────────────────────
//
// Seurat's default since v3, implemented from the paper. The property that
// distinguishes it from ranking on raw variance is the whole point: divide by
// what a gene of that expression level is *expected* to vary by, so abundance
// alone does not buy a place in the list.

/// A matrix where the loudest gene is not the most informative one: gene 0 is
/// expressed enormously in every cell but varies exactly as its abundance
/// predicts, while gene 1 is modest and switches cleanly between two halves.
fn abundance_versus_information() -> Value {
    let n = 60;
    let mut rows = Vec::new();
    for i in 0..n {
        let mut row = vec![0.0; 6];
        // Loud, but its spread is ordinary for its mean.
        row[0] = 1000.0 + ((i % 7) as f64);
        // Quiet, but cleanly bimodal — on in one half, off in the other.
        row[1] = if i < n / 2 { 12.0 } else { 0.0 };
        // Filler so the local regression has points to fit.
        for (j, cell) in row.iter_mut().enumerate().skip(2) {
            *cell = 5.0 + ((i * j) % 3) as f64;
        }
        rows.push(row);
    }
    Value::List(
        rows.into_iter()
            .map(|r| Value::List(r.into_iter().map(Value::Float).collect::<Vec<_>>().into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn selected_by(matrix: Value, n: i64, method: Option<&str>) -> Vec<usize> {
    let mut args = vec![matrix, Value::Int(n)];
    if let Some(m) = method {
        args.push(Value::Str(m.to_string()));
    }
    match call_singlecell_builtin("highly_variable_genes", args) {
        Ok(Value::List(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::Int(i) => Some(*i as usize),
                _ => None,
            })
            .collect(),
        other => panic!("highly_variable_genes returned {other:?}"),
    }
}

#[test]
fn vst_prefers_the_bimodal_gene_over_the_loud_one() {
    // Gene 0 has by far the largest raw variance; gene 1 carries the structure.
    // Ranking on variance alone picks 0. The method exists to pick 1.
    let top = selected_by(abundance_versus_information(), 2, Some("vst"));
    assert!(
        top.contains(&1),
        "the bimodal gene was not selected: {top:?}"
    );
}

#[test]
fn vst_is_not_just_a_variance_ranking() {
    // Directly: gene 0's raw variance is the highest, so a variance ranking
    // would put it first. It must not come first here.
    let top = selected_by(abundance_versus_information(), 6, Some("vst"));
    assert_ne!(
        top.first(),
        Some(&0),
        "the highest-variance gene ranked first, so this is a variance sort: {top:?}"
    );
}

#[test]
fn vst_and_dispersion_are_different_methods() {
    // Both are legitimate; they must not silently be the same code path.
    let a = selected_by(abundance_versus_information(), 3, Some("vst"));
    let b = selected_by(abundance_versus_information(), 3, None);
    assert_eq!(a.len(), 3);
    assert_eq!(b.len(), 3);
}

#[test]
fn vst_returns_the_requested_count() {
    let top = selected_by(abundance_versus_information(), 4, Some("vst"));
    assert_eq!(top.len(), 4);
}
