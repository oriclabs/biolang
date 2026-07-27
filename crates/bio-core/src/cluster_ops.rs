use std::collections::HashMap;
use std::collections::VecDeque;

/// Leiden community detection (Traag et al. 2019).
///
/// Unlike Louvain, Leiden adds a *refinement* phase that guarantees every
/// returned community is internally connected. Multi-level: local moving →
/// refine → aggregate, repeated until the graph stops shrinking. Deterministic
/// (nodes are visited in index order), so runs are reproducible.
///
/// Takes a symmetric weighted adjacency matrix (zero diagonal expected) and a
/// resolution (higher = more, smaller communities). Returns a contiguous
/// community label per node.
pub fn leiden(adjacency: &[Vec<f64>], resolution: f64) -> Vec<usize> {
    let n = adjacency.len();
    if n == 0 {
        return vec![];
    }
    let m: f64 = adjacency.iter().flat_map(|r| r.iter()).sum::<f64>() / 2.0;
    if m == 0.0 {
        return (0..n).collect();
    }

    // Working graph: off-diagonal edge weights, per-node degree, and the set of
    // original nodes each super-node represents.
    let mut adj: Vec<Vec<f64>> = adjacency.to_vec();
    let mut deg: Vec<f64> = adj.iter().map(|r| r.iter().sum()).collect();
    let mut orig: Vec<Vec<usize>> = (0..n).map(|i| vec![i]).collect();

    loop {
        let part = local_move(&adj, &deg, resolution, m);
        let refined = refine(&adj, &deg, &part, resolution, m);

        // Aggregate on the refined partition; a super-node's initial community
        // for the next level is the (non-refined) community it belonged to.
        let n_refined = *refined.iter().max().unwrap() + 1;
        if n_refined == adj.len() {
            // No merges possible → propagate `part` to original nodes and stop.
            return finalize(adjacency, &orig, &part, n);
        }

        let (new_adj, new_deg, new_orig) = aggregate(&adj, &deg, &orig, &refined, n_refined);
        adj = new_adj;
        deg = new_deg;
        orig = new_orig;
        // Continue: next iteration's local_move starts fresh on the coarser graph.
    }
}

/// One local-moving pass: greedily move each node to the neighboring community
/// that maximizes modularity gain, repeating until stable.
fn local_move(adj: &[Vec<f64>], deg: &[f64], res: f64, m: f64) -> Vec<usize> {
    let n = adj.len();
    let mut comm: Vec<usize> = (0..n).collect();
    let mut tot: Vec<f64> = deg.to_vec(); // sum of deg per community

    let mut improved = true;
    let mut iters = 0;
    while improved && iters < 100 {
        improved = false;
        iters += 1;
        for i in 0..n {
            let ci = comm[i];
            tot[ci] -= deg[i];

            // Edge weight from i to each neighboring community.
            let mut k_in: HashMap<usize, f64> = HashMap::new();
            for j in 0..n {
                if j != i && adj[i][j] != 0.0 {
                    *k_in.entry(comm[j]).or_insert(0.0) += adj[i][j];
                }
            }
            let coef = res * deg[i] / (2.0 * m);
            // Candidate: staying as a singleton in the emptied community.
            let mut best = ci;
            let mut best_gain = k_in.get(&ci).copied().unwrap_or(0.0) - coef * tot[ci];
            for (&c, &kic) in &k_in {
                let gain = kic - coef * tot[c];
                if gain > best_gain + 1e-12 {
                    best_gain = gain;
                    best = c;
                }
            }
            if best != ci {
                improved = true;
            }
            comm[i] = best;
            tot[best] += deg[i];
        }
    }
    relabel(&comm)
}

/// Refinement: within each community of `part`, grow connected sub-communities
/// by only merging a node into a sub-community it has an edge to. This is what
/// guarantees connected communities.
fn refine(adj: &[Vec<f64>], deg: &[f64], part: &[usize], res: f64, m: f64) -> Vec<usize> {
    let n = adj.len();
    let mut refined: Vec<usize> = (0..n).collect();
    let mut tot: Vec<f64> = deg.to_vec();

    // Group node indices by their community in `part`.
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    for i in 0..n {
        groups.entry(part[i]).or_default().push(i);
    }

    for members in groups.values() {
        let member_set: std::collections::HashSet<usize> = members.iter().copied().collect();
        for &i in members {
            let ri = refined[i];
            tot[ri] -= deg[i];

            // Edges from i to refined sub-communities *within the same group*.
            let mut k_in: HashMap<usize, f64> = HashMap::new();
            for &j in members {
                if j != i && adj[i][j] != 0.0 {
                    *k_in.entry(refined[j]).or_insert(0.0) += adj[i][j];
                }
            }
            let coef = res * deg[i] / (2.0 * m);
            let mut best = ri;
            // Staying put has zero connectivity requirement.
            let mut best_gain = 0.0;
            for (&r, &kic) in &k_in {
                // Connectivity requirement: only join a sub-community i touches.
                let gain = kic - coef * tot[r];
                if gain > best_gain + 1e-12 {
                    best_gain = gain;
                    best = r;
                }
            }
            let _ = &member_set;
            comm_assign(&mut refined, &mut tot, i, best, deg[i]);
        }
    }
    relabel(&refined)
}

fn comm_assign(refined: &mut [usize], tot: &mut [f64], i: usize, target: usize, di: f64) {
    refined[i] = target;
    tot[target] += di;
}

/// Build the coarser graph whose nodes are the refined communities.
fn aggregate(
    adj: &[Vec<f64>],
    deg: &[f64],
    orig: &[Vec<usize>],
    refined: &[usize],
    n_new: usize,
) -> (Vec<Vec<f64>>, Vec<f64>, Vec<Vec<usize>>) {
    let n = adj.len();
    let mut new_adj = vec![vec![0.0; n_new]; n_new];
    let mut new_deg = vec![0.0; n_new];
    let mut new_orig: Vec<Vec<usize>> = vec![Vec::new(); n_new];

    for i in 0..n {
        let a = refined[i];
        new_deg[a] += deg[i];
        new_orig[a].extend_from_slice(&orig[i]);
        for j in 0..n {
            if adj[i][j] != 0.0 {
                let b = refined[j];
                if a != b {
                    new_adj[a][b] += adj[i][j];
                }
            }
        }
    }
    (new_adj, new_deg, new_orig)
}

/// Map the final super-node partition down to original nodes, then split any
/// internally-disconnected community into connected components (belt-and-braces
/// enforcement of the Leiden connectivity guarantee).
fn finalize(
    adjacency: &[Vec<f64>],
    orig: &[Vec<usize>],
    part: &[usize],
    n: usize,
) -> Vec<usize> {
    let mut label = vec![0usize; n];
    for (super_node, members) in orig.iter().enumerate() {
        for &o in members {
            label[o] = part[super_node];
        }
    }
    let label = split_disconnected(adjacency, &label, n);
    relabel(&label)
}

/// Split communities that are not internally connected into separate labels.
fn split_disconnected(adjacency: &[Vec<f64>], label: &[usize], n: usize) -> Vec<usize> {
    let mut out = vec![usize::MAX; n];
    let mut next = 0usize;
    for start in 0..n {
        if out[start] != usize::MAX {
            continue;
        }
        // BFS within this node's label over connected edges.
        let lbl = label[start];
        let mut queue = VecDeque::new();
        queue.push_back(start);
        out[start] = next;
        while let Some(u) = queue.pop_front() {
            for v in 0..n {
                if out[v] == usize::MAX
                    && label[v] == lbl
                    && adjacency[u][v] != 0.0
                {
                    out[v] = next;
                    queue.push_back(v);
                }
            }
        }
        next += 1;
    }
    out
}

/// Renumber labels to a contiguous 0-based range, preserving first-seen order.
fn relabel(labels: &[usize]) -> Vec<usize> {
    let mut mapping: HashMap<usize, usize> = HashMap::new();
    let mut next = 0;
    labels
        .iter()
        .map(|&c| {
            *mapping.entry(c).or_insert_with(|| {
                let id = next;
                next += 1;
                id
            })
        })
        .collect()
}

/// Louvain community detection algorithm.
///
/// Takes an adjacency matrix (symmetric, weighted) and returns cluster assignments.
/// The resolution parameter controls granularity (higher = more clusters).
pub fn louvain(adjacency: &[Vec<f64>], resolution: f64) -> Vec<usize> {
    let n = adjacency.len();
    if n == 0 {
        return vec![];
    }

    // Initialize: each node in its own community
    let mut community: Vec<usize> = (0..n).collect();

    // Compute total edge weight
    let total_weight: f64 = adjacency.iter().flat_map(|row| row.iter()).sum::<f64>() / 2.0;

    if total_weight == 0.0 {
        return community;
    }

    // Node strengths (weighted degree)
    let strengths: Vec<f64> = adjacency
        .iter()
        .map(|row| row.iter().sum::<f64>())
        .collect();

    let mut improved = true;
    let max_iter = 100;
    let mut iter = 0;

    while improved && iter < max_iter {
        improved = false;
        iter += 1;

        for i in 0..n {
            let current_comm = community[i];

            // Compute modularity gain for moving i to each neighbor's community
            let mut best_comm = current_comm;
            let mut best_gain = 0.0f64;

            // Get neighboring communities
            let mut neighbor_comms: HashMap<usize, f64> = HashMap::new();
            for j in 0..n {
                if adjacency[i][j] > 0.0 && i != j {
                    *neighbor_comms.entry(community[j]).or_insert(0.0) += adjacency[i][j];
                }
            }

            // Also consider staying in current community
            let ki = strengths[i];

            // Weight of edges from i to nodes in its current community
            let ki_in_current: f64 = (0..n)
                .filter(|&j| community[j] == current_comm && j != i)
                .map(|j| adjacency[i][j])
                .sum();

            // Sum of strengths in current community (excluding i)
            let sigma_current: f64 = (0..n)
                .filter(|&j| community[j] == current_comm && j != i)
                .map(|j| strengths[j])
                .sum();

            // Modularity loss from removing i from current community
            let remove_cost =
                ki_in_current - resolution * ki * sigma_current / (2.0 * total_weight);

            for (&target_comm, &ki_in_target) in &neighbor_comms {
                if target_comm == current_comm {
                    continue;
                }

                // Sum of strengths in target community
                let sigma_target: f64 = (0..n)
                    .filter(|&j| community[j] == target_comm)
                    .map(|j| strengths[j])
                    .sum();

                // Modularity gain from adding i to target community
                let add_gain = ki_in_target - resolution * ki * sigma_target / (2.0 * total_weight);

                let delta_q = add_gain - remove_cost;

                if delta_q > best_gain {
                    best_gain = delta_q;
                    best_comm = target_comm;
                }
            }

            if best_comm != current_comm {
                community[i] = best_comm;
                improved = true;
            }
        }
    }

    // Renumber communities to be contiguous starting from 0
    let mut mapping: HashMap<usize, usize> = HashMap::new();
    let mut next_id = 0;
    for c in &mut community {
        let new_id = *mapping.entry(*c).or_insert_with(|| {
            let id = next_id;
            next_id += 1;
            id
        });
        *c = new_id;
    }

    community
}

#[cfg(test)]
mod leiden_tests {
    use super::*;

    // Build an undirected unweighted adjacency from an edge list.
    fn graph(n: usize, edges: &[(usize, usize)]) -> Vec<Vec<f64>> {
        let mut a = vec![vec![0.0; n]; n];
        for &(u, v) in edges {
            a[u][v] = 1.0;
            a[v][u] = 1.0;
        }
        a
    }

    // Is every community internally connected? (The Leiden guarantee.)
    fn all_connected(adj: &[Vec<f64>], labels: &[usize]) -> bool {
        let n = adj.len();
        for c in 0..=*labels.iter().max().unwrap() {
            let members: Vec<usize> = (0..n).filter(|&i| labels[i] == c).collect();
            if members.len() <= 1 {
                continue;
            }
            let mut seen = vec![false; n];
            let mut stack = vec![members[0]];
            seen[members[0]] = true;
            while let Some(u) = stack.pop() {
                for &v in &members {
                    if !seen[v] && adj[u][v] != 0.0 {
                        seen[v] = true;
                        stack.push(v);
                    }
                }
            }
            if members.iter().any(|&i| !seen[i]) {
                return false;
            }
        }
        true
    }

    #[test]
    fn two_triangles_two_communities() {
        // Triangle {0,1,2} and triangle {3,4,5} joined by a single edge 2-3.
        let g = graph(6, &[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5), (2, 3)]);
        let labels = leiden(&g, 1.0);
        assert_eq!(*labels.iter().max().unwrap() + 1, 2, "expected 2 communities: {labels:?}");
        assert_eq!(labels[0], labels[1]);
        assert_eq!(labels[1], labels[2]);
        assert_eq!(labels[3], labels[4]);
        assert_ne!(labels[0], labels[3]);
    }

    #[test]
    fn communities_are_connected() {
        // Two 4-cliques joined by one edge; every community must be connected.
        let g = graph(
            8,
            &[
                (0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3),
                (4, 5), (4, 6), (4, 7), (5, 6), (5, 7), (6, 7),
                (3, 4),
            ],
        );
        let labels = leiden(&g, 1.0);
        assert!(all_connected(&g, &labels), "communities not connected: {labels:?}");
    }

    #[test]
    fn deterministic() {
        let g = graph(6, &[(0, 1), (1, 2), (0, 2), (3, 4), (4, 5), (3, 5), (2, 3)]);
        assert_eq!(leiden(&g, 1.0), leiden(&g, 1.0));
    }

    #[test]
    fn empty_and_singleton() {
        assert_eq!(leiden(&[], 1.0), Vec::<usize>::new());
        assert_eq!(leiden(&[vec![0.0]], 1.0), vec![0]);
    }
}
