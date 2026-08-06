//! CCA finds what two datasets share, not what either one is.
//!
//! The claim that makes it the starting point for integration: variation
//! present in only one dataset - a batch effect - scores poorly, while
//! variation present in both scores well. That is testable directly, by
//! planting one of each and checking which one the leading axis picks up.
//!
//! Kept deliberately tiny. The cross-product is cells x cells and
//! Matrix::svd is O(n^4) - `max_iter = 200 * n` around an O(n^3) QR step -
//! so it stalls above roughly 20x20 in a debug build and 100x100 in release.
//! That is a real limit on CCA here, not a property of these tests: Seurat
//! CCA on thousands of cells needs an eigensolver this crate does not have
//! yet, which is why harmony_integrate is the tool for real integration.
//!
//! As with Harmony, the assertions come in pairs. An embedding that collapses
//! everything to a point would "align the batches" perfectly while destroying
//! the biology, so alignment is never asserted on its own.

use bl_core::value::Value;
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::HashMap;

const PER_TYPE: usize = 5;
const TYPES: usize = 3;
const GENES: usize = 12;

/// One dataset: three cell types over shared genes, plus a dataset-wide offset
/// standing in for a batch effect. Each cell type switches on its own block of
/// genes, so the shared structure is real and known.
fn dataset(offset: f64) -> (Vec<Vec<f64>>, Vec<usize>) {
    let mut rows = Vec::new();
    let mut types = Vec::new();
    for cell_type in 0..TYPES {
        for i in 0..PER_TYPE {
            let mut row = vec![0.0; GENES];
            for gene in 0..GENES {
                // Each type owns four genes; a little deterministic wobble
                // stops the matrix being rank-deficient.
                let owned = gene / 4 == cell_type;
                row[gene] = if owned { 3.0 } else { 0.2 }
                    + ((i * (gene + 3)) % 7) as f64 * 0.03
                    // The batch effect: the same push on every cell of this
                    // dataset, which is what CCA is meant not to reward.
                    + offset;
            }
            rows.push(row);
            types.push(cell_type);
        }
    }
    (rows, types)
}

fn as_value(rows: &[Vec<f64>]) -> Value {
    Value::List(
        rows.iter()
            .map(|row| {
                Value::List(
                    row.iter()
                        .map(|v| Value::Float(*v))
                        .collect::<Vec<_>>()
                        .into(),
                )
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

fn rows_of(value: &Value) -> Vec<Vec<f64>> {
    match value {
        Value::List(items) => items
            .iter()
            .map(|row| match row {
                Value::List(values) => values.iter().filter_map(|v| v.as_float()).collect(),
                _ => Vec::new(),
            })
            .collect(),
        _ => panic!("expected a matrix, got {value:?}"),
    }
}

/// Runs cca and returns (u, v).
fn cca(a: &[Vec<f64>], b: &[Vec<f64>], k: i64) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
    let mut opts = HashMap::new();
    opts.insert("k".to_string(), Value::Int(k));
    match call_singlecell_builtin(
        "cca",
        vec![as_value(a), as_value(b), Value::Record(opts.into())],
    ) {
        Ok(Value::Record(map)) => (
            rows_of(map.get("u").expect("no u")),
            rows_of(map.get("v").expect("no v")),
        ),
        other => panic!("cca returned {other:?}"),
    }
}

fn centroid(points: &[Vec<f64>], types: &[usize], want: usize) -> Vec<f64> {
    let chosen: Vec<&Vec<f64>> = points
        .iter()
        .zip(types)
        .filter(|(_, t)| **t == want)
        .map(|(row, _)| row)
        .collect();
    let n = chosen.len() as f64;
    (0..points[0].len())
        .map(|d| chosen.iter().map(|row| row[d]).sum::<f64>() / n)
        .collect()
}

fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

#[test]
fn the_two_datasets_land_on_the_same_axes_with_their_types_intact() {
    // The pair that matters. Matching cell types must line up across datasets
    // despite a batch offset present in only one of them - and different cell
    // types must not.
    let (a, types) = dataset(0.0);
    let (b, _) = dataset(1.5);
    let (u, v) = cca(&a, &b, 3);
    assert_eq!(u.len(), a.len());
    assert_eq!(v.len(), b.len());

    for cell_type in 0..TYPES {
        let here = centroid(&u, &types, cell_type);
        let there = centroid(&v, &types, cell_type);
        let same = distance(&here, &there);

        // Against the nearest *different* type in the other dataset.
        let nearest_other = (0..TYPES)
            .filter(|other| *other != cell_type)
            .map(|other| distance(&here, &centroid(&v, &types, other)))
            .fold(f64::MAX, f64::min);

        assert!(
            same < nearest_other,
            "type {cell_type} is {same:.3} from its match across datasets but \
             {nearest_other:.3} from a different type - the axes are not shared"
        );
    }
}

#[test]
fn a_batch_offset_does_not_become_the_leading_axis() {
    // The property that makes CCA the right starting point. The offset is the
    // single biggest difference between the two matrices, and a decomposition
    // of either one alone would lead with it. Here it must not separate the
    // datasets, because it is not shared.
    let (a, types) = dataset(0.0);
    let (b, _) = dataset(3.0);
    let (u, v) = cca(&a, &b, 2);

    // Distance between the two datasets' overall centroids, against the spread
    // of cell types within one of them. If the offset had been captured, the
    // datasets would be further apart than the biology.
    let all = vec![0usize; u.len()];
    let dataset_gap = distance(&centroid(&u, &all, 0), &centroid(&v, &all, 0));
    let biology = (0..TYPES)
        .map(|t| distance(&centroid(&u, &types, t), &centroid(&u, &all, 0)))
        .fold(f64::MIN, f64::max);

    assert!(
        dataset_gap < biology,
        "the datasets are {dataset_gap:.3} apart but the cell types only span \
         {biology:.3} - the batch effect became the leading axis"
    );
}

#[test]
fn cell_types_stay_distinct_in_the_shared_space() {
    // The half that a collapsing embedding would fail. Three planted types must
    // still be three.
    let (a, types) = dataset(0.0);
    let (b, _) = dataset(0.8);
    let (u, _) = cca(&a, &b, 3);
    let all = vec![0usize; u.len()];
    let overall = centroid(&u, &all, 0);
    for cell_type in 0..TYPES {
        let away = distance(&centroid(&u, &types, cell_type), &overall);
        assert!(
            away > 1e-6,
            "cell type {cell_type} sits on the global centroid - the space collapsed"
        );
    }
}

#[test]
fn k_bounds_the_number_of_axes() {
    let (a, _) = dataset(0.0);
    let (b, _) = dataset(0.5);
    let (u, v) = cca(&a, &b, 4);
    assert_eq!(u[0].len(), 4);
    assert_eq!(v[0].len(), 4);
}

#[test]
fn mismatched_genes_are_refused_rather_than_lined_up_by_position() {
    // Quietly using whichever genes happen to align would produce a result that
    // looks fine and means nothing.
    let (a, _) = dataset(0.0);
    let narrow: Vec<Vec<f64>> = a.iter().map(|row| row[..GENES - 2].to_vec()).collect();
    let error = call_singlecell_builtin("cca", vec![as_value(&a), as_value(&narrow)]).unwrap_err();
    assert!(
        format!("{error}").contains("same genes"),
        "unhelpful error: {error}"
    );
}

#[test]
fn empty_input_is_refused() {
    let (a, _) = dataset(0.0);
    let empty = Value::List(vec![].into());
    assert!(call_singlecell_builtin("cca", vec![as_value(&a), empty]).is_err());
}
