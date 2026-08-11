//! Conformance against Seurat's MIT-licensed implementations.
//!
//! These two operations were verified equivalent to the functions Seurat ships
//! — `ComputeSNN` and `LogNorm`, both MIT C++ — not merely close to them. That
//! is a strong guarantee and an easy one to lose: a plausible-looking change to
//! a neighbour loop or a normalisation constant would still pass every other
//! test in this repository.
//!
//! The full parity harness lives in the `biolang-workflows` repository, because
//! it needs R, Seurat, and a multi-gigabyte oracle. That means it cannot run in
//! this repository's CI, so nothing here would notice if these two drifted.
//! These fixtures close that gap: the reference outputs are checked in, so the
//! guarantee is enforced on every build with no R and no network.
//!
//! `tests/fixtures/seurat/PROVENANCE.md` records how they were produced and
//! with which versions. Only Seurat's *outputs* are stored — no Seurat source
//! is copied or derived here.

use bl_core::value::Value;
use bl_runtime::singlecell::call_singlecell_builtin;
use std::path::PathBuf;

fn fixture(name: &str) -> String {
    let path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "fixtures", "seurat", name]
        .iter()
        .collect();
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read fixture {}: {error}", path.display()))
}

/// Parse a headerless numeric CSV into rows.
fn numeric_rows(text: &str) -> Vec<Vec<f64>> {
    text.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(',')
                .map(|cell| {
                    cell.trim()
                        .parse::<f64>()
                        .unwrap_or_else(|error| panic!("bad number {cell:?}: {error}"))
                })
                .collect()
        })
        .collect()
}

fn matrix_value(rows: &[Vec<f64>]) -> Value {
    Value::List(
        rows.iter()
            .map(|row| {
                Value::List(
                    row.iter()
                        .copied()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                )
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

fn record_field(record: &Value, field: &str) -> f64 {
    match record {
        Value::Record(map) => match map.get(field) {
            Some(Value::Float(value)) => *value,
            Some(Value::Int(value)) => *value as f64,
            other => panic!("field {field} was {other:?}"),
        },
        other => panic!("expected a Record, got {other:?}"),
    }
}

/// `snn_graph` must reproduce `Seurat:::ComputeSNN` edge for edge.
///
/// Two details are load-bearing and both are easy to get wrong in a way no
/// other test would catch. Seurat's ranked-neighbour matrix includes the query
/// cell itself among its k columns, so the Jaccard denominator is `2k - shared`
/// with k counting self. And its prune test is strictly less than, so an edge
/// sitting exactly on the threshold is kept — Seurat's own documentation says
/// "less than or equal to" while its code says less than.
#[test]
fn snn_graph_matches_seurat_compute_snn() {
    let embeddings = numeric_rows(&fixture("snn_input.csv"));
    assert_eq!(embeddings.len(), 200, "fixture shape changed");

    let result = call_singlecell_builtin(
        "snn_graph",
        vec![
            matrix_value(&embeddings),
            Value::Int(20),
            Value::Float(1.0 / 15.0),
        ],
    )
    .expect("snn_graph failed");

    let edges = match &result {
        Value::List(items) => items.clone(),
        other => panic!("snn_graph returned {other:?}"),
    };

    let mut produced: Vec<(i64, i64, f64)> = edges
        .iter()
        .map(|edge| {
            (
                record_field(edge, "source") as i64,
                record_field(edge, "target") as i64,
                record_field(edge, "weight"),
            )
        })
        .collect();
    produced.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

    let expected: Vec<(i64, i64, f64)> = fixture("snn_expected.csv")
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            (
                parts[0].trim().parse().unwrap(),
                parts[1].trim().parse().unwrap(),
                parts[2].trim().parse().unwrap(),
            )
        })
        .collect();

    assert_eq!(
        produced.len(),
        expected.len(),
        "edge count differs from Seurat: {} vs {}",
        produced.len(),
        expected.len()
    );

    for (index, (got, want)) in produced.iter().zip(&expected).enumerate() {
        assert_eq!(
            (got.0, got.1),
            (want.0, want.1),
            "edge {index} connects different cells"
        );
        // One ULP at this magnitude; the measured worst case was 5e-16.
        assert!(
            (got.2 - want.2).abs() < 1e-12,
            "edge {index} ({}, {}): weight {} vs Seurat {}",
            got.0,
            got.1,
            got.2,
            want.2
        );
    }
}

/// `normalize_total` then `log1p_transform` must reproduce `Seurat::LogNormalize`.
///
/// The fixture is stored genes x cells, matching Seurat; BioLang's single-cell
/// builtins take cells x genes, so it is transposed on the way in and back on
/// the way out. Getting that orientation wrong is the classic silent failure
/// here, and it would show up as a shape mismatch rather than a wrong number.
#[test]
fn log_normalize_matches_seurat() {
    let counts = numeric_rows(&fixture("lognorm_counts.csv"));
    let expected = numeric_rows(&fixture("lognorm_expected.csv"));
    let (n_genes, n_cells) = (counts.len(), counts[0].len());
    assert_eq!((n_genes, n_cells), (60, 40), "fixture shape changed");
    assert_eq!(expected.len(), n_genes);

    let cells: Vec<Vec<f64>> = (0..n_cells)
        .map(|cell| (0..n_genes).map(|gene| counts[gene][cell]).collect())
        .collect();

    let scaled = call_singlecell_builtin(
        "normalize_total",
        vec![matrix_value(&cells), Value::Float(10_000.0)],
    )
    .expect("normalize_total failed");
    let logged =
        call_singlecell_builtin("log1p_transform", vec![scaled]).expect("log1p_transform failed");

    let rows = match &logged {
        Value::List(items) => items
            .iter()
            .map(|row| match row {
                Value::List(values) => values
                    .iter()
                    .map(|value| match value {
                        Value::Float(v) => *v,
                        Value::Int(v) => *v as f64,
                        other => panic!("non-numeric {other:?}"),
                    })
                    .collect::<Vec<f64>>(),
                other => panic!("row was {other:?}"),
            })
            .collect::<Vec<_>>(),
        other => panic!("log1p_transform returned {other:?}"),
    };
    assert_eq!(rows.len(), n_cells);

    let mut worst = 0.0f64;
    for gene in 0..n_genes {
        for cell in 0..n_cells {
            worst = worst.max((rows[cell][gene] - expected[gene][cell]).abs());
        }
    }
    // Measured worst case over 240,000 values was 5.3e-15.
    assert!(
        worst < 1e-12,
        "largest deviation from Seurat::LogNormalize was {worst:e}"
    );
}
