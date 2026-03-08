use std::collections::{HashMap, HashSet};

/// A node in a de Bruijn graph.
#[derive(Debug, Clone)]
pub struct DeBruijnNode {
    pub sequence: String,
}

/// An edge in a de Bruijn graph.
#[derive(Debug, Clone)]
pub struct DeBruijnEdge {
    pub from: String,
    pub to: String,
    pub label: String,
}

/// Construct a de Bruijn graph from sequences.
///
/// Each (k-1)-mer becomes a node, and each k-mer becomes an edge
/// connecting its prefix to its suffix.
pub fn de_bruijn_graph(sequences: &[&str], k: usize) -> (Vec<DeBruijnNode>, Vec<DeBruijnEdge>) {
    let mut nodes_set: HashSet<String> = HashSet::new();
    let mut edges = Vec::new();

    for seq in sequences {
        let bytes = seq.as_bytes();
        if bytes.len() < k {
            continue;
        }
        for i in 0..=(bytes.len() - k) {
            let kmer = &seq[i..i + k];
            let prefix = &kmer[..k - 1];
            let suffix = &kmer[1..];
            nodes_set.insert(prefix.to_string());
            nodes_set.insert(suffix.to_string());
            edges.push(DeBruijnEdge {
                from: prefix.to_string(),
                to: suffix.to_string(),
                label: kmer.to_string(),
            });
        }
    }

    let mut nodes: Vec<DeBruijnNode> = nodes_set
        .into_iter()
        .map(|s| DeBruijnNode { sequence: s })
        .collect();
    nodes.sort_by(|a, b| a.sequence.cmp(&b.sequence));

    (nodes, edges)
}

/// Count edges between each pair of nodes (for multigraph representation).
pub fn edge_counts(edges: &[DeBruijnEdge]) -> HashMap<(String, String), usize> {
    let mut counts = HashMap::new();
    for e in edges {
        *counts.entry((e.from.clone(), e.to.clone())).or_insert(0) += 1;
    }
    counts
}
