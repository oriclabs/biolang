//! Batch correction has two ways to fail and only one of them is obvious.
//!
//! The obvious one is doing nothing: donors stay separated and every cell type
//! appears twice. The dangerous one is doing too much - pulling everything
//! together until distinct cell types merge, which produces a beautifully mixed
//! UMAP that has destroyed the biology it was meant to reveal. A batch-mixing
//! metric alone is maximised by collapsing the data to a point.
//!
//! So every test here holds both ends: batches must come together AND cell
//! types must stay apart.

use bl_core::value::Value;
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::HashMap;

const PER_GROUP: usize = 60;
const TYPES: usize = 3;
const DIMS: usize = 4;

/// Three cell types, two batches, and a batch shift that is *different per cell
/// type* - which is the case a single global mean cannot fix and the reason
/// Harmony corrects within clusters.
fn scenario() -> (Vec<Vec<f64>>, Vec<usize>, Vec<usize>) {
    // Cell types sit far apart on the first two axes, and none of them sits on
    // the origin: the clustering step compares directions, so a population
    // centred at zero has no direction to speak of and would be split at random.
    let centres = [[10.0, 0.0], [0.0, 10.0], [-7.0, -7.0]];
    // Batch 1's offset differs by cell type: one moves in x, one in y, one both.
    let shifts = [[2.5, 0.0], [0.0, 2.5], [1.8, 1.8]];

    let mut embedding = Vec::new();
    let mut batches = Vec::new();
    let mut types = Vec::new();
    for cell_type in 0..TYPES {
        for batch in 0..2 {
            for i in 0..PER_GROUP {
                // Deterministic spread, so the test does not depend on a seed.
                let a = ((i * 37) % 11) as f64 * 0.06 - 0.3;
                let b = ((i * 53) % 13) as f64 * 0.05 - 0.3;
                let mut point = vec![0.0; DIMS];
                point[0] = centres[cell_type][0] + a;
                point[1] = centres[cell_type][1] + b;
                point[2] = ((i * 17) % 7) as f64 * 0.04;
                point[3] = ((i * 29) % 5) as f64 * 0.04;
                if batch == 1 {
                    point[0] += shifts[cell_type][0];
                    point[1] += shifts[cell_type][1];
                }
                embedding.push(point);
                batches.push(batch);
                types.push(cell_type);
            }
        }
    }
    (embedding, batches, types)
}

fn run(embedding: &[Vec<f64>], batches: &[usize], opts: Vec<(&str, Value)>) -> Vec<Vec<f64>> {
    let matrix = Value::List(
        embedding
            .iter()
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
    );
    let labels = Value::List(
        batches
            .iter()
            .map(|b| Value::Str(format!("donor{b}")))
            .collect::<Vec<_>>()
            .into(),
    );
    let mut args = vec![matrix, labels];
    if !opts.is_empty() {
        let mut map = HashMap::new();
        for (key, value) in opts {
            map.insert(key.to_string(), value);
        }
        args.push(Value::Record(map.into()));
    }
    match call_singlecell_builtin("harmony_integrate", args) {
        Ok(Value::List(rows)) => rows
            .iter()
            .map(|row| match row {
                Value::List(values) => values.iter().filter_map(|v| v.as_float()).collect(),
                _ => Vec::new(),
            })
            .collect(),
        other => panic!("harmony_integrate returned {other:?}"),
    }
}

fn centroid(points: &[Vec<f64>], keep: impl Fn(usize) -> bool) -> Vec<f64> {
    let chosen: Vec<&Vec<f64>> = points
        .iter()
        .enumerate()
        .filter(|(i, _)| keep(*i))
        .map(|(_, row)| row)
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

/// How far apart the two batches sit within each cell type, averaged.
fn batch_gap(points: &[Vec<f64>], batches: &[usize], types: &[usize]) -> f64 {
    let mut total = 0.0;
    for cell_type in 0..TYPES {
        let first = centroid(points, |i| types[i] == cell_type && batches[i] == 0);
        let second = centroid(points, |i| types[i] == cell_type && batches[i] == 1);
        total += distance(&first, &second);
    }
    total / TYPES as f64
}

/// The closest any two cell types come to each other.
fn type_separation(points: &[Vec<f64>], types: &[usize]) -> f64 {
    let centres: Vec<Vec<f64>> = (0..TYPES)
        .map(|t| centroid(points, |i| types[i] == t))
        .collect();
    let mut closest = f64::MAX;
    for a in 0..TYPES {
        for b in (a + 1)..TYPES {
            closest = closest.min(distance(&centres[a], &centres[b]));
        }
    }
    closest
}

#[test]
fn batches_come_together_and_cell_types_stay_apart() {
    // The whole test suite in one assertion pair. Either half alone is passed
    // by a broken implementation: doing nothing preserves separation, and
    // collapsing everything to a point mixes perfectly.
    let (embedding, batches, types) = scenario();
    let before_gap = batch_gap(&embedding, &batches, &types);
    let before_separation = type_separation(&embedding, &types);

    let after = run(&embedding, &batches, vec![]);
    let after_gap = batch_gap(&after, &batches, &types);
    let after_separation = type_separation(&after, &types);

    assert!(
        after_gap < before_gap * 0.25,
        "batch effect barely moved: {before_gap:.2} -> {after_gap:.2}"
    );
    assert!(
        after_separation > before_separation * 0.5,
        "cell types were merged: separation {before_separation:.2} -> {after_separation:.2}"
    );
}

#[test]
fn the_correction_differs_per_cell_type() {
    // The reason this exists rather than sc_integrate. The planted shift is a
    // different direction for each cell type, so a single global offset cannot
    // remove all three. Subtracting one per-batch mean would leave two of the
    // three cell types still split.
    let (embedding, batches, types) = scenario();
    let after = run(&embedding, &batches, vec![]);
    for cell_type in 0..TYPES {
        let first = centroid(&after, |i| types[i] == cell_type && batches[i] == 0);
        let second = centroid(&after, |i| types[i] == cell_type && batches[i] == 1);
        let gap = distance(&first, &second);
        assert!(
            gap < 1.0,
            "cell type {cell_type} still has its donors {gap:.2} apart"
        );
    }
}

#[test]
fn one_cluster_cannot_fix_a_per_cell_type_shift() {
    // Forcing a single cluster reduces this to what sc_integrate already does:
    // one global per-batch offset. The planted shift points a different way for
    // each cell type, so a single offset must leave some of them split - which
    // is the argument for the whole per-cluster construction, stated as a test
    // rather than asserted in a comment.
    let (embedding, batches, types) = scenario();
    let global = run(&embedding, &batches, vec![("n_clusters", Value::Int(1))]);
    let clustered = run(&embedding, &batches, vec![]);

    let global_gap = batch_gap(&global, &batches, &types);
    let clustered_gap = batch_gap(&clustered, &batches, &types);
    assert!(
        clustered_gap < global_gap * 0.5,
        "per-cluster correction ({clustered_gap:.2}) is no better than one global \
         offset ({global_gap:.2}) - the clustering is not doing anything"
    );
}

#[test]
fn a_single_batch_is_left_alone() {
    // Nothing to correct, and the regression would be singular. Returning the
    // input unchanged beats returning noise.
    let (embedding, _, _) = scenario();
    let one_batch = vec![0usize; embedding.len()];
    let after = run(&embedding, &one_batch, vec![]);
    for (before, now) in embedding.iter().zip(&after) {
        for (a, b) in before.iter().zip(now) {
            assert!(
                (a - b).abs() < 1e-12,
                "a single-batch embedding was altered"
            );
        }
    }
}

#[test]
fn theta_controls_how_hard_it_mixes() {
    // The documented knob has to do something, in the documented direction.
    let (embedding, batches, types) = scenario();
    let gentle = run(&embedding, &batches, vec![("theta", Value::Float(0.0))]);
    let firm = run(&embedding, &batches, vec![("theta", Value::Float(4.0))]);
    assert!(
        batch_gap(&firm, &batches, &types) <= batch_gap(&gentle, &batches, &types) + 1e-9,
        "raising theta did not mix batches at least as hard"
    );
}

#[test]
fn the_result_is_the_same_every_run() {
    // Harmony seeds its clustering; a random start would move published figures
    // between runs for no reason a reader could see.
    let (embedding, batches, _) = scenario();
    let first = run(&embedding, &batches, vec![]);
    let second = run(&embedding, &batches, vec![]);
    assert_eq!(first, second, "two identical runs disagreed");
}

#[test]
fn labels_that_do_not_match_the_cells_are_refused() {
    let (embedding, _, _) = scenario();
    let matrix = Value::List(
        embedding
            .iter()
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
    );
    let short = Value::List(vec![Value::Str("a".to_string())].into());
    assert!(call_singlecell_builtin("harmony_integrate", vec![matrix, short]).is_err());
}
