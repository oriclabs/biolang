use std::collections::HashMap;

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
    let total_weight: f64 = adjacency
        .iter()
        .flat_map(|row| row.iter())
        .sum::<f64>()
        / 2.0;

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
            let remove_cost = ki_in_current - resolution * ki * sigma_current / (2.0 * total_weight);

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
                let add_gain =
                    ki_in_target - resolution * ki * sigma_target / (2.0 * total_weight);

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
