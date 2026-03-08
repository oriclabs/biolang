use bio_core::phylo_ops::*;

#[test]
fn test_neighbor_joining() {
    // Simple 4-taxon distance matrix
    let d = vec![
        vec![0.0, 5.0, 9.0, 9.0],
        vec![5.0, 0.0, 10.0, 10.0],
        vec![9.0, 10.0, 0.0, 8.0],
        vec![9.0, 10.0, 8.0, 0.0],
    ];
    let names = vec!["A".into(), "B".into(), "C".into(), "D".into()];
    let tree = neighbor_joining(&d, Some(&names));
    assert!(tree.len() > 4); // Should have internal nodes
}
