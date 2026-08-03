//! Eulerian cycles and paths, by Hierholzer's algorithm.
//!
//! Genome assembly is this problem wearing a costume. Reads become edges of a de
//! Bruijn graph, and a walk using every edge exactly once spells a sequence
//! containing every read — which is why five of Rosalind's assembly problems
//! reduce to the two functions here.
//!
//! Nodes are opaque strings, so the same code serves numbered graphs, k-mer
//! graphs and paired-read graphs without any of them having to agree on a
//! representation.

use std::collections::HashMap;

/// A directed multigraph: each node listed with its outgoing edges.
///
/// Parallel edges matter — two reads spanning the same junction are two edges,
/// and collapsing them would drop coverage — so this is a list per node, not a
/// set.
pub type Adjacency = HashMap<String, Vec<String>>;

/// Whether every node has as many edges in as out.
///
/// The condition for an Eulerian *cycle* to exist, given the edges are
/// connected. Checked rather than assumed, because the alternative is a walk
/// that stops early and looks like a correct answer of the wrong length.
pub fn is_balanced(graph: &Adjacency) -> bool {
    degree_difference(graph).is_empty()
}

/// Nodes whose in- and out-degrees differ, as `(node, out - in)`.
fn degree_difference(graph: &Adjacency) -> Vec<(String, i64)> {
    let mut balance: HashMap<&str, i64> = HashMap::new();
    for (node, targets) in graph {
        *balance.entry(node.as_str()).or_insert(0) += targets.len() as i64;
        for target in targets {
            *balance.entry(target.as_str()).or_insert(0) -= 1;
        }
    }
    let mut uneven: Vec<(String, i64)> = balance
        .into_iter()
        .filter(|(_, difference)| *difference != 0)
        .map(|(node, difference)| (node.to_string(), difference))
        .collect();
    uneven.sort();
    uneven
}

/// A walk using every edge exactly once and returning to where it began.
///
/// Hierholzer's algorithm: walk until stuck — which in a balanced graph can only
/// happen back at the start — then repeatedly re-enter the walk at a node that
/// still has unused edges and splice the new loop in. Linear in the number of
/// edges, against the factorial cost of searching for the walk directly.
///
/// Returns `None` when no such cycle exists, rather than a partial walk that
/// would be indistinguishable from a correct answer.
pub fn eulerian_cycle(graph: &Adjacency, start: Option<&str>) -> Option<Vec<String>> {
    let total_edges: usize = graph.values().map(Vec::len).sum();
    if total_edges == 0 {
        return None;
    }

    // A cursor per node, so an edge is never scanned twice.
    let mut next_edge: HashMap<&str, usize> = HashMap::new();
    let begin: &str = match start {
        Some(node) => node,
        None => graph.keys().next()?.as_str(),
    };

    let mut stack: Vec<&str> = vec![begin];
    let mut circuit: Vec<&str> = Vec::with_capacity(total_edges + 1);
    while let Some(&node) = stack.last() {
        let used = next_edge.entry(node).or_insert(0);
        match graph.get(node).and_then(|targets| targets.get(*used)) {
            Some(target) => {
                *used += 1;
                stack.push(target.as_str());
            }
            None => {
                circuit.push(node);
                stack.pop();
            }
        }
    }
    circuit.reverse();

    // Every edge used exactly once, or the graph was not connected.
    if circuit.len() != total_edges + 1 {
        return None;
    }
    // And it has to close. An unbalanced graph still produces a walk that uses
    // every edge — it just ends somewhere else — and returning that as a cycle
    // would be a wrong answer of exactly the right length.
    if circuit.first() != circuit.last() {
        return None;
    }
    Some(circuit.into_iter().map(str::to_string).collect())
}

/// A walk using every edge exactly once, not necessarily returning to its start.
///
/// At most one node may have one more outgoing edge than incoming (the start)
/// and at most one the reverse (the end). Adding the edge between them turns the
/// problem into [`eulerian_cycle`]; the walk is then rotated so it begins where
/// that added edge would end, and the edge itself dropped.
pub fn eulerian_path(graph: &Adjacency) -> Option<Vec<String>> {
    let uneven = degree_difference(graph);
    match uneven.len() {
        // Already balanced: any Eulerian cycle is also a path.
        0 => eulerian_cycle(graph, None),
        2 => {
            let (start, end) = match (uneven[0].1, uneven[1].1) {
                (1, -1) => (uneven[0].0.clone(), uneven[1].0.clone()),
                (-1, 1) => (uneven[1].0.clone(), uneven[0].0.clone()),
                // A difference of more than one cannot be fixed by one edge.
                _ => return None,
            };

            let mut patched = graph.clone();
            patched.entry(end.clone()).or_default().push(start.clone());

            let cycle = eulerian_cycle(&patched, Some(&start))?;
            // The cycle repeats its first node at the end; drop that so what
            // remains is a plain cyclic order.
            let mut walk: Vec<String> = cycle[..cycle.len() - 1].to_vec();
            let length = walk.len();
            // Cut the cycle at the edge that was added. It can sit anywhere,
            // including across the wrap from the last node back to the first —
            // which is why this searches modularly rather than over `windows(2)`.
            let join = (0..length).find(|&i| walk[i] == end && walk[(i + 1) % length] == start)?;
            walk.rotate_left((join + 1) % length);
            Some(walk)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(lines: &[(&str, &[&str])]) -> Adjacency {
        lines
            .iter()
            .map(|(node, targets)| {
                (
                    (*node).to_string(),
                    targets.iter().map(|t| (*t).to_string()).collect(),
                )
            })
            .collect()
    }

    /// Every edge of `graph` appears exactly once along `walk`.
    fn uses_every_edge_once(graph: &Adjacency, walk: &[String]) -> bool {
        let mut remaining: HashMap<(String, String), usize> = HashMap::new();
        let mut total = 0;
        for (node, targets) in graph {
            for target in targets {
                *remaining.entry((node.clone(), target.clone())).or_insert(0) += 1;
                total += 1;
            }
        }
        if walk.len() != total + 1 {
            return false;
        }
        for pair in walk.windows(2) {
            match remaining.get_mut(&(pair[0].clone(), pair[1].clone())) {
                Some(count) if *count > 0 => *count -= 1,
                _ => return false,
            }
        }
        true
    }

    fn ba3f_graph() -> Adjacency {
        graph(&[
            ("0", &["3"]),
            ("1", &["0"]),
            ("2", &["1", "6"]),
            ("3", &["2"]),
            ("4", &["2"]),
            ("5", &["4"]),
            ("6", &["5", "8"]),
            ("7", &["9"]),
            ("8", &["7"]),
            ("9", &["6"]),
        ])
    }

    #[test]
    fn ba3f_sample() {
        let graph = ba3f_graph();
        let cycle = eulerian_cycle(&graph, Some("6")).expect("a cycle");
        // Rosalind accepts any Eulerian cycle, so the check is the property,
        // not the published string — which is one rotation among many.
        assert!(uses_every_edge_once(&graph, &cycle));
        assert_eq!(cycle.first(), cycle.last(), "a cycle returns to its start");
        assert_eq!(cycle.first().map(String::as_str), Some("6"));
    }

    #[test]
    fn ba3g_sample() {
        let graph = graph(&[
            ("0", &["2"]),
            ("1", &["3"]),
            ("2", &["1"]),
            ("3", &["0", "4"]),
            ("6", &["3", "7"]),
            ("7", &["8"]),
            ("8", &["9"]),
            ("9", &["6"]),
        ]);
        let path = eulerian_path(&graph).expect("a path");
        assert!(uses_every_edge_once(&graph, &path));
        // 6 has one more edge out than in, 4 one more in than out.
        assert_eq!(path.first().map(String::as_str), Some("6"));
        assert_eq!(path.last().map(String::as_str), Some("4"));
    }

    #[test]
    fn a_balanced_graph_has_no_odd_nodes() {
        assert!(is_balanced(&ba3f_graph()));
        let lopsided = graph(&[("a", &["b"])]);
        assert!(!is_balanced(&lopsided));
    }

    #[test]
    fn parallel_edges_are_both_used() {
        // Two reads spanning the same junction are two edges; collapsing them
        // would silently drop coverage.
        let graph = graph(&[("a", &["b", "b"]), ("b", &["a", "a"])]);
        let cycle = eulerian_cycle(&graph, Some("a")).expect("a cycle");
        assert_eq!(cycle.len(), 5, "four edges plus the return");
        assert!(uses_every_edge_once(&graph, &cycle));
    }

    #[test]
    fn a_disconnected_graph_has_no_eulerian_walk() {
        // Balanced but in two pieces: a walk cannot reach the second.
        let graph = graph(&[("a", &["b"]), ("b", &["a"]), ("c", &["d"]), ("d", &["c"])]);
        assert!(is_balanced(&graph), "each piece is balanced on its own");
        assert!(eulerian_cycle(&graph, Some("a")).is_none());
    }

    #[test]
    fn too_many_odd_nodes_means_no_path() {
        let graph = graph(&[("a", &["b"]), ("c", &["d"])]);
        assert!(eulerian_path(&graph).is_none());
    }

    #[test]
    fn an_empty_graph_yields_nothing() {
        assert!(eulerian_cycle(&Adjacency::new(), None).is_none());
        assert!(eulerian_path(&Adjacency::new()).is_none());
    }
}
