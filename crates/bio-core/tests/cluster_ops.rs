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
