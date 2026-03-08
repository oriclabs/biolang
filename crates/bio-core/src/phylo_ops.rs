/// A node in a phylogenetic tree.
#[derive(Debug, Clone)]
pub struct PhyloNode {
    pub name: String,
    pub distance: f64,
    pub children: Vec<usize>, // indices into the node list
}

/// Neighbor-joining algorithm for phylogenetic tree construction.
///
/// Takes a symmetric distance matrix and optional names.
/// Returns a list of nodes representing the tree structure.
pub fn neighbor_joining(distances: &[Vec<f64>], names: Option<&[String]>) -> Vec<PhyloNode> {
    let n = distances.len();
    if n < 2 {
        return names
            .map(|ns| {
                ns.iter()
                    .map(|name| PhyloNode {
                        name: name.clone(),
                        distance: 0.0,
                        children: vec![],
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    // Initialize working distance matrix and active node indices
    let mut d: Vec<Vec<f64>> = distances.to_vec();
    let mut nodes: Vec<PhyloNode> = (0..n)
        .map(|i| PhyloNode {
            name: names
                .and_then(|ns| ns.get(i))
                .cloned()
                .unwrap_or_else(|| format!("node_{i}")),
            distance: 0.0,
            children: vec![],
        })
        .collect();
    let mut active: Vec<usize> = (0..n).collect();

    while active.len() > 2 {
        let m = active.len();

        // Compute row sums for Q-matrix
        let row_sums: Vec<f64> = active
            .iter()
            .map(|&i| active.iter().map(|&j| d[i][j]).sum::<f64>())
            .collect();

        // Find the pair (i, j) that minimizes Q
        let mut min_q = f64::INFINITY;
        let mut min_pair = (0usize, 1usize);

        for ai in 0..m {
            for aj in (ai + 1)..m {
                let q = (m as f64 - 2.0) * d[active[ai]][active[aj]]
                    - row_sums[ai]
                    - row_sums[aj];
                if q < min_q {
                    min_q = q;
                    min_pair = (ai, aj);
                }
            }
        }

        let (ai, aj) = min_pair;
        let i = active[ai];
        let j = active[aj];

        // Compute distances to new node
        let dist_ij = d[i][j];
        let di = 0.5 * dist_ij + (row_sums[ai] - row_sums[aj]) / (2.0 * (m as f64 - 2.0));
        let dj = dist_ij - di;

        // Create new internal node
        let new_idx = nodes.len();
        nodes[i].distance = di.max(0.0);
        nodes[j].distance = dj.max(0.0);
        nodes.push(PhyloNode {
            name: format!("internal_{}", new_idx),
            distance: 0.0,
            children: vec![i, j],
        });

        // Update distance matrix: add row/col for new node
        let new_row: Vec<f64> = (0..nodes.len())
            .map(|k| {
                if k == new_idx {
                    0.0
                } else if k < d.len() {
                    0.5 * (d[i].get(k).copied().unwrap_or(0.0)
                        + d[j].get(k).copied().unwrap_or(0.0)
                        - dist_ij)
                } else {
                    0.0
                }
            })
            .collect();

        // Expand the distance matrix
        for row in d.iter_mut() {
            row.push(0.0);
        }
        d.push(vec![0.0; nodes.len()]);

        for k in 0..new_row.len() {
            d[new_idx][k] = new_row[k].max(0.0);
            if k < d.len() {
                d[k][new_idx] = new_row[k].max(0.0);
            }
        }

        // Remove i, j from active; add new_idx
        let mut new_active: Vec<usize> = active
            .iter()
            .copied()
            .filter(|&x| x != i && x != j)
            .collect();
        new_active.push(new_idx);
        active = new_active;
    }

    // Connect last two nodes
    if active.len() == 2 {
        let i = active[0];
        let j = active[1];
        let dist = d[i][j];
        nodes[i].distance = dist / 2.0;
        nodes[j].distance = dist / 2.0;

        let root_idx = nodes.len();
        nodes.push(PhyloNode {
            name: "root".to_string(),
            distance: 0.0,
            children: vec![i, j],
        });
        let _ = root_idx; // root is last element
    }

    nodes
}
