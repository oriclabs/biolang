use bio_core::graph_ops::*;

#[test]
fn test_de_bruijn() {
    let seqs = vec!["ACGTACGT"];
    let (nodes, edges) = de_bruijn_graph(&seqs, 4);
    assert!(!nodes.is_empty());
    assert!(!edges.is_empty());
    // Each 4-mer creates an edge from its 3-mer prefix to 3-mer suffix
    assert_eq!(edges.len(), 5); // ACGT, CGTA, GTAC, TACG, ACGT
}
