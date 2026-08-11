use bio_core::cluster_ops::*;

#[test]
fn test_louvain_two_cliques() {
    // Two disconnected cliques of size 3
    #[rustfmt::skip]
    let adj = vec![
        vec![0.0, 1.0, 1.0, 0.0, 0.0, 0.0],
        vec![1.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        vec![1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
        vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0],
        vec![0.0, 0.0, 0.0, 1.0, 0.0, 1.0],
        vec![0.0, 0.0, 0.0, 1.0, 1.0, 0.0],
    ];
    let clusters = louvain(&adj, 1.0);
    assert_eq!(clusters.len(), 6);
    // Nodes 0,1,2 should be in same cluster; nodes 3,4,5 in another
    assert_eq!(clusters[0], clusters[1]);
    assert_eq!(clusters[1], clusters[2]);
    assert_eq!(clusters[3], clusters[4]);
    assert_eq!(clusters[4], clusters[5]);
    assert_ne!(clusters[0], clusters[3]);
}

#[test]
fn test_louvain_empty() {
    let clusters = louvain(&[], 1.0);
    assert!(clusters.is_empty());
}

// ── Louvain ─────────────────────────────────────────────────────────
//
// Exposed because Seurat's FindClusters defaults to algorithm = 1, which is
// Louvain, in v5.5.1 as in every earlier version. Reproducing a published
// Seurat analysis means running the same algorithm, not a better one.

/// Two cliques joined by a single edge — the partition any modularity method
/// must find.
fn barbell() -> Vec<Vec<f64>> {
    let n = 10;
    let mut a = vec![vec![0.0; n]; n];
    for i in 0..5 {
        for j in 0..5 {
            if i != j {
                a[i][j] = 1.0;
            }
        }
    }
    for i in 5..10 {
        for j in 5..10 {
            if i != j {
                a[i][j] = 1.0;
            }
        }
    }
    a[4][5] = 1.0;
    a[5][4] = 1.0;
    a
}

#[test]
fn louvain_separates_two_cliques() {
    let labels = bio_core::cluster_ops::louvain(&barbell(), 1.0);
    assert_eq!(labels.len(), 10);
    assert_eq!(
        labels[0..5]
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        1,
        "first clique was split: {labels:?}"
    );
    assert_eq!(
        labels[5..10]
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        1,
        "second clique was split: {labels:?}"
    );
    assert_ne!(labels[0], labels[9], "the cliques were merged: {labels:?}");
}

#[test]
fn louvain_sparse_agrees_with_the_dense_form() {
    // The sparse path is what the single-cell builtins use; it must not be a
    // different algorithm wearing the same name.
    let dense = barbell();
    let sparse: Vec<Vec<(usize, f64)>> = dense
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|(_, w)| **w != 0.0)
                .map(|(j, w)| (j, *w))
                .collect()
        })
        .collect();
    let a = bio_core::cluster_ops::louvain(&dense, 1.0);
    let b = bio_core::cluster_ops::louvain_sparse(&sparse, 1.0);
    // Labels are arbitrary; the partition is what must agree.
    let same_partition = |x: &[usize], y: &[usize]| {
        (0..x.len()).all(|i| (0..x.len()).all(|j| (x[i] == x[j]) == (y[i] == y[j])))
    };
    assert!(same_partition(&a, &b), "dense {a:?} vs sparse {b:?}");
}

#[test]
fn louvain_is_reproducible() {
    let g = barbell();
    assert_eq!(
        bio_core::cluster_ops::louvain(&g, 1.0),
        bio_core::cluster_ops::louvain(&g, 1.0)
    );
}

// ── Modularity and restarts ─────────────────────────────────────────
//
// Seurat's FindClusters defaults to n.start = 10: ten runs from different node
// orders, keeping the highest modularity. Matching that needs a modularity
// function, because without one there is no basis for "keeping the best".

fn sparse_barbell() -> Vec<Vec<(usize, f64)>> {
    barbell()
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .filter(|(_, w)| **w != 0.0)
                .map(|(j, w)| (j, *w))
                .collect()
        })
        .collect()
}

/// The correct partition of a barbell must score above the obvious wrong ones.
/// A modularity that does not rank these correctly cannot be used to choose
/// between restarts, which is the only thing it exists for here.
#[test]
fn modularity_prefers_the_partition_that_is_actually_there() {
    let g = sparse_barbell();
    let correct = [0, 0, 0, 0, 0, 1, 1, 1, 1, 1];
    let everything_together = [0; 10];
    let everything_apart: Vec<usize> = (0..10).collect();
    let split_wrong = [0, 0, 1, 1, 0, 1, 0, 1, 1, 0];

    let q = |labels: &[usize]| bio_core::cluster_ops::modularity(&g, labels, 1.0);
    assert!(
        q(&correct) > q(&everything_together),
        "one big community scored {} against the true partition's {}",
        q(&everything_together),
        q(&correct)
    );
    assert!(q(&correct) > q(&everything_apart));
    assert!(q(&correct) > q(&split_wrong));
    // A single community has zero modularity by definition: e/2m = 1, (d/2m)^2 = 1.
    assert!(q(&everything_together).abs() < 1e-12);
}

/// Restarts must never do worse than the single deterministic pass, because the
/// first restart *is* that pass and the rest can only displace it by scoring
/// higher.
#[test]
fn louvain_restarts_never_score_below_a_single_pass() {
    let g = sparse_barbell();
    for resolution in [0.5, 1.0, 2.0] {
        let single = bio_core::cluster_ops::louvain_sparse(&g, resolution);
        let restarted = bio_core::cluster_ops::louvain_sparse_restarts(&g, resolution, 10, 0);
        let q_single = bio_core::cluster_ops::modularity(&g, &single, resolution);
        let q_restarted = bio_core::cluster_ops::modularity(&g, &restarted, resolution);
        assert!(
            q_restarted >= q_single - 1e-12,
            "resolution {resolution}: restarts scored {q_restarted}, single pass {q_single}"
        );
    }
}

/// One restart has to be byte-identical to the plain call, so the new default
/// is an addition rather than a silent change of algorithm.
#[test]
fn one_restart_is_the_old_deterministic_pass() {
    let g = sparse_barbell();
    assert_eq!(
        bio_core::cluster_ops::louvain_sparse_restarts(&g, 1.0, 1, 0),
        bio_core::cluster_ops::louvain_sparse(&g, 1.0)
    );
}

/// Restarts are seeded, so the result is still reproducible run to run.
#[test]
fn louvain_restarts_are_reproducible() {
    let g = sparse_barbell();
    assert_eq!(
        bio_core::cluster_ops::louvain_sparse_restarts(&g, 1.0, 10, 0),
        bio_core::cluster_ops::louvain_sparse_restarts(&g, 1.0, 10, 0)
    );
}
