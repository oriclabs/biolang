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
fn finalize(adjacency: &[Vec<f64>], orig: &[Vec<usize>], part: &[usize], n: usize) -> Vec<usize> {
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
                if out[v] == usize::MAX && label[v] == lbl && adjacency[u][v] != 0.0 {
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

/// Leiden community detection for a symmetric sparse adjacency list.
///
/// This follows the same local-move, refinement, and aggregation phases as
/// [`leiden`] without materializing an n-by-n adjacency matrix.
/// Louvain community detection on a sparse graph (Blondel et al. 2008).
///
/// `leiden_sparse` minus the refinement pass: local moving, then aggregate,
/// until the graph stops shrinking. That single omission is the whole
/// difference between the two, and it is why they disagree on real data —
/// Leiden's refinement guarantees each returned community is internally
/// connected, and Louvain's does not.
///
/// This is exposed because Seurat's `FindClusters` defaults to
/// `algorithm = 1`, which is Louvain, in v5.5.1 as in every version before it.
/// A great deal of published single-cell work therefore used Louvain, and
/// comparing against those results means running the same algorithm rather
/// than a better one.
///
/// Prefer `leiden_sparse` for new work: Louvain can return a community made of
/// two pieces with no edge between them, which is the defect Leiden was written
/// to fix. That behaviour is preserved here deliberately — a "fixed" Louvain
/// would not reproduce what it is being used to reproduce.
///
/// Deterministic: nodes are visited in index order. Seurat's Louvain takes
/// `n.start = 10` random restarts and keeps the best, so it explores more of
/// the space; this will land on a similar but not always identical partition.
/// Modularity of a partition, with a resolution parameter.
///
/// `Q = sum_c [ e_c / m - gamma * (d_c / 2m)^2 ]`, where `e_c` is the weight
/// inside community `c` counted once per edge, `d_c` the summed degree of its
/// members, and `m` the total edge weight. This is the quantity Louvain climbs;
/// without it there is no way to say which of several runs did better, which is
/// why restarts need it.
pub fn modularity(adjacency: &[Vec<(usize, f64)>], labels: &[usize], resolution: f64) -> f64 {
    let n = adjacency.len();
    if n == 0 || labels.len() != n {
        return 0.0;
    }
    let degrees: Vec<f64> = adjacency
        .iter()
        .map(|neighbors| neighbors.iter().map(|(_, weight)| weight).sum())
        .collect();
    let m = degrees.iter().sum::<f64>() / 2.0;
    if m <= 0.0 {
        return 0.0;
    }
    let n_communities = labels.iter().copied().max().unwrap_or(0) + 1;
    let mut internal = vec![0.0f64; n_communities];
    let mut total = vec![0.0f64; n_communities];
    for node in 0..n {
        total[labels[node]] += degrees[node];
        for &(neighbor, weight) in &adjacency[node] {
            if labels[neighbor] == labels[node] {
                internal[labels[node]] += weight;
            }
        }
    }
    // `internal` counted every intra-community edge from both endpoints.
    (0..n_communities)
        .map(|c| {
            let fraction = total[c] / (2.0 * m);
            internal[c] / (2.0 * m) - resolution * fraction * fraction
        })
        .sum()
}

// Seurat's algorithm=1 community optimiser follows Modularity Optimizer 1.3.0.
// Its random stream and move schedule are observable parts of the result: it
// uses java.util.Random's 48-bit generator, swaps every position with a random
// position (not Fisher-Yates), carries one stream across starts, and repeats a
// multilevel pass up to n.iter times. Keep this compatibility implementation
// separate from BioLang's general Louvain routine below.
struct JavaRandom48 {
    seed: u64,
}

impl JavaRandom48 {
    fn new(seed: u64) -> Self {
        Self {
            seed: (seed ^ 0x5DEECE66D) & ((1u64 << 48) - 1),
        }
    }

    fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self.seed.wrapping_mul(0x5DEECE66D).wrapping_add(0xB)) & ((1u64 << 48) - 1);
        (self.seed >> (48 - bits)) as i32
    }

    fn next_int(&mut self, n: usize) -> usize {
        assert!(n > 0);
        if n.is_power_of_two() {
            return ((n as u64 * self.next(31) as u64) >> 31) as usize;
        }
        loop {
            let bits = self.next(31);
            let value = bits % n as i32;
            if bits.wrapping_sub(value).wrapping_add(n as i32 - 1) >= 0 {
                return value as usize;
            }
        }
    }
}

fn seurat_permutation(n: usize, random: &mut JavaRandom48) -> Vec<usize> {
    let mut permutation: Vec<usize> = (0..n).collect();
    for i in 0..n {
        let j = random.next_int(n);
        permutation.swap(i, j);
    }
    permutation
}

#[derive(Clone)]
struct SeuratLouvainNetwork {
    adjacency: Vec<Vec<(usize, f64)>>,
    degrees: Vec<f64>,
    self_link_weight: f64,
}

impl SeuratLouvainNetwork {
    fn from_adjacency(adjacency: &[Vec<(usize, f64)>]) -> Self {
        Self {
            adjacency: adjacency.to_vec(),
            degrees: adjacency
                .iter()
                .map(|neighbors| neighbors.iter().map(|(_, weight)| weight).sum())
                .collect(),
            self_link_weight: 0.0,
        }
    }

    fn denominator(&self) -> f64 {
        self.degrees.iter().sum::<f64>() + self.self_link_weight
    }

    fn reduce(&self, labels: &[usize], n_clusters: usize) -> Self {
        let mut maps: Vec<HashMap<usize, f64>> = vec![HashMap::new(); n_clusters];
        let mut degrees = vec![0.0; n_clusters];
        let mut self_link_weight = self.self_link_weight;
        for node in 0..self.adjacency.len() {
            let source = labels[node];
            degrees[source] += self.degrees[node];
            for &(neighbor, weight) in &self.adjacency[node] {
                let target = labels[neighbor];
                if source == target {
                    self_link_weight += weight;
                } else {
                    *maps[source].entry(target).or_insert(0.0) += weight;
                }
            }
        }
        let adjacency = maps
            .into_iter()
            .map(|map| {
                let mut neighbors: Vec<_> = map.into_iter().collect();
                neighbors.sort_by_key(|(neighbor, _)| *neighbor);
                neighbors
            })
            .collect();
        Self {
            adjacency,
            degrees,
            self_link_weight,
        }
    }
}

fn compact_seurat_labels(labels: &mut [usize]) -> usize {
    if labels.is_empty() {
        return 0;
    }
    let mut counts = vec![0usize; labels.len()];
    for &label in labels.iter() {
        counts[label] += 1;
    }
    let mut replacement = vec![0usize; labels.len()];
    let mut n_clusters = 0;
    for (label, count) in counts.into_iter().enumerate() {
        if count > 0 {
            replacement[label] = n_clusters;
            n_clusters += 1;
        }
    }
    for label in labels {
        *label = replacement[*label];
    }
    n_clusters
}

fn seurat_local_move(
    network: &SeuratLouvainNetwork,
    labels: &mut [usize],
    resolution: f64,
    random: &mut JavaRandom48,
) -> bool {
    let n = network.adjacency.len();
    if n <= 1 {
        return false;
    }
    let mut cluster_weight = vec![0.0; n];
    let mut nodes_per_cluster = vec![0usize; n];
    for node in 0..n {
        cluster_weight[labels[node]] += network.degrees[node];
        nodes_per_cluster[labels[node]] += 1;
    }
    let mut unused_clusters: Vec<usize> = (0..n)
        .filter(|&cluster| nodes_per_cluster[cluster] == 0)
        .collect();
    let permutation = seurat_permutation(n, random);
    let mut weights_per_cluster = vec![0.0; n];
    let mut neighboring_clusters = Vec::with_capacity(n.saturating_sub(1));
    let mut stable_nodes = 0usize;
    let mut position = 0usize;
    let mut updated = false;
    while stable_nodes < n {
        let node = permutation[position];
        neighboring_clusters.clear();
        for &(neighbor, weight) in &network.adjacency[node] {
            let cluster = labels[neighbor];
            if weights_per_cluster[cluster] == 0.0 {
                neighboring_clusters.push(cluster);
            }
            weights_per_cluster[cluster] += weight;
        }

        let old_cluster = labels[node];
        cluster_weight[old_cluster] -= network.degrees[node];
        nodes_per_cluster[old_cluster] -= 1;
        if nodes_per_cluster[old_cluster] == 0 {
            unused_clusters.push(old_cluster);
        }

        let mut best_cluster = None;
        let mut max_quality = 0.0;
        for &cluster in &neighboring_clusters {
            let quality = weights_per_cluster[cluster]
                - network.degrees[node] * cluster_weight[cluster] * resolution;
            if quality > max_quality
                || (quality == max_quality && best_cluster.is_some_and(|best| cluster < best))
            {
                best_cluster = Some(cluster);
                max_quality = quality;
            }
            weights_per_cluster[cluster] = 0.0;
        }
        let selected = if max_quality == 0.0 {
            unused_clusters.pop().unwrap_or(old_cluster)
        } else {
            best_cluster.unwrap_or(old_cluster)
        };
        cluster_weight[selected] += network.degrees[node];
        nodes_per_cluster[selected] += 1;
        if selected == old_cluster {
            stable_nodes += 1;
        } else {
            labels[node] = selected;
            stable_nodes = 1;
            updated = true;
        }
        position = if position + 1 < n { position + 1 } else { 0 };
    }
    compact_seurat_labels(labels);
    updated
}

fn seurat_louvain_pass(
    network: &SeuratLouvainNetwork,
    labels: &mut [usize],
    resolution: f64,
    random: &mut JavaRandom48,
) -> bool {
    if network.adjacency.len() <= 1 {
        return false;
    }
    let mut updated = seurat_local_move(network, labels, resolution, random);
    let n_clusters = labels.iter().copied().max().unwrap_or(0) + 1;
    if n_clusters < network.adjacency.len() {
        let reduced = network.reduce(labels, n_clusters);
        let mut reduced_labels: Vec<usize> = (0..n_clusters).collect();
        let reduced_updated =
            seurat_louvain_pass(&reduced, &mut reduced_labels, resolution, random);
        if reduced_updated {
            for label in labels.iter_mut() {
                *label = reduced_labels[*label];
            }
            compact_seurat_labels(labels);
            updated = true;
        }
    }
    updated
}

fn seurat_quality(network: &SeuratLouvainNetwork, labels: &[usize], resolution: f64) -> f64 {
    let denominator = network.denominator();
    if denominator <= 0.0 {
        return 0.0;
    }
    let n_clusters = labels.iter().copied().max().unwrap_or(0) + 1;
    let mut quality = network.self_link_weight;
    let mut cluster_weight = vec![0.0; n_clusters];
    for node in 0..network.adjacency.len() {
        cluster_weight[labels[node]] += network.degrees[node];
        for &(neighbor, weight) in &network.adjacency[node] {
            if labels[neighbor] == labels[node] {
                quality += weight;
            }
        }
    }
    quality -= cluster_weight
        .into_iter()
        .map(|weight| weight * weight * resolution)
        .sum::<f64>();
    quality / denominator
}

/// Seurat 5 `FindClusters(algorithm = 1)` compatible Louvain optimisation.
pub fn louvain_seurat_restarts(
    adjacency: &[Vec<(usize, f64)>],
    resolution: f64,
    n_start: usize,
    n_iter: usize,
    seed: u64,
) -> Vec<usize> {
    let n = adjacency.len();
    if n == 0 {
        return vec![];
    }
    let network = SeuratLouvainNetwork::from_adjacency(adjacency);
    let denominator = network.denominator();
    if denominator <= 0.0 {
        return (0..n).collect();
    }
    let scaled_resolution = resolution / denominator;
    let mut random = JavaRandom48::new(seed);
    let mut best_quality = f64::NEG_INFINITY;
    let mut best_labels = Vec::new();
    for _ in 0..n_start.max(1) {
        let mut labels: Vec<usize> = (0..n).collect();
        for _ in 0..n_iter.max(1) {
            if !seurat_louvain_pass(&network, &mut labels, scaled_resolution, &mut random) {
                break;
            }
        }
        let quality = seurat_quality(&network, &labels, scaled_resolution);
        if quality > best_quality {
            best_quality = quality;
            best_labels = labels;
        }
    }

    // Seurat numbers clusters by decreasing size, preserving the old label as
    // the stable tie-break. This does not change the partition, but it makes
    // exported compatibility fixtures directly comparable.
    let n_clusters = best_labels.iter().copied().max().unwrap_or(0) + 1;
    let mut sizes = vec![0usize; n_clusters];
    for &label in &best_labels {
        sizes[label] += 1;
    }
    let mut order: Vec<usize> = (0..n_clusters).collect();
    order.sort_by(|&left, &right| sizes[right].cmp(&sizes[left]));
    let mut replacement = vec![0usize; n_clusters];
    for (new, old) in order.into_iter().enumerate() {
        replacement[old] = new;
    }
    best_labels
        .into_iter()
        .map(|label| replacement[label])
        .collect()
}

/// Louvain repeated from several randomised node orders, keeping the best.
///
/// Seurat's `FindClusters` defaults to `n.start = 10`: ten runs, each visiting
/// nodes in a different order, and the partition with the highest modularity
/// wins. A single pass in a fixed order is not equivalent no matter how long it
/// runs — local moving is greedy, so the order decides which local optimum it
/// falls into, and iterating longer from the same start converges to the same
/// place. Best-of-ten reliably finds finer partitions, which is why a single
/// pass tends to report *fewer* clusters than the reference does.
pub fn louvain_sparse_restarts(
    adjacency: &[Vec<(usize, f64)>],
    resolution: f64,
    n_start: usize,
    seed: u64,
) -> Vec<usize> {
    let n = adjacency.len();
    if n == 0 {
        return vec![];
    }
    let mut best: Option<(f64, Vec<usize>)> = None;
    for start in 0..n_start.max(1) {
        // Start 0 keeps the natural order, so `n_start = 1` reproduces exactly
        // the deterministic behaviour that came before this.
        let labels = louvain_sparse_seeded(
            adjacency,
            resolution,
            seed.wrapping_add(start as u64)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15),
            start > 0,
        );
        let quality = modularity(adjacency, &labels, resolution);
        let better = match &best {
            None => true,
            Some((best_quality, _)) => quality > *best_quality + 1e-12,
        };
        if better {
            best = Some((quality, labels));
        }
    }
    best.map(|(_, labels)| labels).unwrap_or_default()
}

/// A permutation of `0..n` from an LCG, advancing the state in place.
fn shuffled_order(n: usize, rng: &mut u64) -> Vec<usize> {
    let mut order: Vec<usize> = (0..n).collect();
    for i in (1..n).rev() {
        *rng = rng
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let j = (*rng >> 33) as usize % (i + 1);
        order.swap(i, j);
    }
    order
}

pub fn louvain_sparse(adjacency: &[Vec<(usize, f64)>], resolution: f64) -> Vec<usize> {
    louvain_sparse_seeded(adjacency, resolution, 0, false)
}

fn louvain_sparse_seeded(
    adjacency: &[Vec<(usize, f64)>],
    resolution: f64,
    seed: u64,
    shuffle: bool,
) -> Vec<usize> {
    let n = adjacency.len();
    if n == 0 {
        return vec![];
    }
    let mut rng = seed;
    let degrees: Vec<f64> = adjacency
        .iter()
        .map(|neighbors| neighbors.iter().map(|(_, weight)| weight).sum())
        .collect();
    let m = degrees.iter().sum::<f64>() / 2.0;
    if m == 0.0 {
        return (0..n).collect();
    }

    let mut graph = adjacency.to_vec();
    let mut graph_degrees = degrees;
    let mut original_nodes: Vec<Vec<usize>> = (0..n).map(|node| vec![node]).collect();

    loop {
        // A fresh order at every level, not just the first. Aggregation changes
        // both the node count and which merges are on offer, so a level visited
        // in the same order every time re-makes the same greedy choice however
        // the level below it came out.
        let order = if shuffle {
            shuffled_order(graph.len(), &mut rng)
        } else {
            (0..graph.len()).collect()
        };
        let partition = sparse_local_move_ordered(&graph, &graph_degrees, resolution, m, &order);
        let n_new = partition.iter().copied().max().unwrap_or(0) + 1;
        if n_new == graph.len() {
            let mut label = vec![0usize; n];
            for (super_node, members) in original_nodes.iter().enumerate() {
                for &node in members {
                    label[node] = partition[super_node];
                }
            }
            return relabel(&label);
        }
        let (next_graph, next_degrees, next_original) =
            sparse_aggregate(&graph, &graph_degrees, &original_nodes, &partition, n_new);
        graph = next_graph;
        graph_degrees = next_degrees;
        original_nodes = next_original;
    }
}

pub fn leiden_sparse(adjacency: &[Vec<(usize, f64)>], resolution: f64) -> Vec<usize> {
    let n = adjacency.len();
    if n == 0 {
        return vec![];
    }
    let degrees: Vec<f64> = adjacency
        .iter()
        .map(|neighbors| neighbors.iter().map(|(_, weight)| weight).sum())
        .collect();
    let m = degrees.iter().sum::<f64>() / 2.0;
    if m == 0.0 {
        return (0..n).collect();
    }

    let original_adjacency = adjacency;
    let mut graph = adjacency.to_vec();
    let mut graph_degrees = degrees;
    let mut original_nodes: Vec<Vec<usize>> = (0..n).map(|node| vec![node]).collect();

    loop {
        let partition = sparse_local_move(&graph, &graph_degrees, resolution, m);
        let refined = sparse_refine(&graph, &graph_degrees, &partition, resolution, m);
        let n_refined = refined.iter().copied().max().unwrap_or(0) + 1;
        if n_refined == graph.len() {
            return sparse_finalize(original_adjacency, &original_nodes, &partition, n);
        }

        let (next_graph, next_degrees, next_original_nodes) =
            sparse_aggregate(&graph, &graph_degrees, &original_nodes, &refined, n_refined);
        graph = next_graph;
        graph_degrees = next_degrees;
        original_nodes = next_original_nodes;
    }
}

fn sparse_local_move(
    adjacency: &[Vec<(usize, f64)>],
    degrees: &[f64],
    resolution: f64,
    m: f64,
) -> Vec<usize> {
    let order: Vec<usize> = (0..adjacency.len()).collect();
    sparse_local_move_ordered(adjacency, degrees, resolution, m, &order)
}

fn sparse_local_move_ordered(
    adjacency: &[Vec<(usize, f64)>],
    degrees: &[f64],
    resolution: f64,
    m: f64,
    order: &[usize],
) -> Vec<usize> {
    let n = adjacency.len();
    let mut communities: Vec<usize> = (0..n).collect();
    let mut totals = degrees.to_vec();

    for _ in 0..100 {
        let mut improved = false;
        for &node in order {
            let current = communities[node];
            totals[current] -= degrees[node];
            let mut weights_by_community = HashMap::new();
            for &(neighbor, weight) in &adjacency[node] {
                if neighbor != node {
                    *weights_by_community
                        .entry(communities[neighbor])
                        .or_insert(0.0) += weight;
                }
            }
            let coefficient = resolution * degrees[node] / (2.0 * m);
            let mut best = current;
            let mut best_gain = weights_by_community.get(&current).copied().unwrap_or(0.0)
                - coefficient * totals[current];
            for (&candidate, &internal_weight) in &weights_by_community {
                let gain = internal_weight - coefficient * totals[candidate];
                if gain > best_gain + 1e-12 {
                    best_gain = gain;
                    best = candidate;
                }
            }
            improved |= best != current;
            communities[node] = best;
            totals[best] += degrees[node];
        }
        if !improved {
            break;
        }
    }
    relabel(&communities)
}

fn sparse_refine(
    adjacency: &[Vec<(usize, f64)>],
    degrees: &[f64],
    partition: &[usize],
    resolution: f64,
    m: f64,
) -> Vec<usize> {
    let n = adjacency.len();
    let mut refined: Vec<usize> = (0..n).collect();
    let mut totals = degrees.to_vec();

    for node in 0..n {
        let current = refined[node];
        totals[current] -= degrees[node];
        let mut weights_by_community = HashMap::new();
        for &(neighbor, weight) in &adjacency[node] {
            if neighbor != node && partition[neighbor] == partition[node] {
                *weights_by_community.entry(refined[neighbor]).or_insert(0.0) += weight;
            }
        }
        let coefficient = resolution * degrees[node] / (2.0 * m);
        let mut best = current;
        let mut best_gain = 0.0;
        for (&candidate, &internal_weight) in &weights_by_community {
            let gain = internal_weight - coefficient * totals[candidate];
            if gain > best_gain + 1e-12 {
                best_gain = gain;
                best = candidate;
            }
        }
        refined[node] = best;
        totals[best] += degrees[node];
    }
    relabel(&refined)
}

fn sparse_aggregate(
    adjacency: &[Vec<(usize, f64)>],
    degrees: &[f64],
    original_nodes: &[Vec<usize>],
    refined: &[usize],
    n_new: usize,
) -> (Vec<Vec<(usize, f64)>>, Vec<f64>, Vec<Vec<usize>>) {
    let mut edge_maps: Vec<HashMap<usize, f64>> = vec![HashMap::new(); n_new];
    let mut new_degrees = vec![0.0; n_new];
    let mut new_original_nodes = vec![Vec::new(); n_new];

    for node in 0..adjacency.len() {
        let source = refined[node];
        new_degrees[source] += degrees[node];
        new_original_nodes[source].extend_from_slice(&original_nodes[node]);
        for &(neighbor, weight) in &adjacency[node] {
            let target = refined[neighbor];
            if source != target {
                *edge_maps[source].entry(target).or_insert(0.0) += weight;
            }
        }
    }

    let graph = edge_maps
        .into_iter()
        .map(|edges| {
            let mut neighbors: Vec<(usize, f64)> = edges.into_iter().collect();
            neighbors.sort_by_key(|(neighbor, _)| *neighbor);
            neighbors
        })
        .collect();
    (graph, new_degrees, new_original_nodes)
}

fn sparse_finalize(
    adjacency: &[Vec<(usize, f64)>],
    original_nodes: &[Vec<usize>],
    partition: &[usize],
    n: usize,
) -> Vec<usize> {
    let mut labels = vec![0usize; n];
    for (super_node, members) in original_nodes.iter().enumerate() {
        for &original_node in members {
            labels[original_node] = partition[super_node];
        }
    }

    let mut connected = vec![usize::MAX; n];
    let mut next_label = 0;
    for start in 0..n {
        if connected[start] != usize::MAX {
            continue;
        }
        let expected_label = labels[start];
        let mut queue = VecDeque::new();
        queue.push_back(start);
        connected[start] = next_label;
        while let Some(node) = queue.pop_front() {
            for &(neighbor, weight) in &adjacency[node] {
                if weight != 0.0
                    && connected[neighbor] == usize::MAX
                    && labels[neighbor] == expected_label
                {
                    connected[neighbor] = next_label;
                    queue.push_back(neighbor);
                }
            }
        }
        next_label += 1;
    }
    relabel(&connected)
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
        assert_eq!(
            *labels.iter().max().unwrap() + 1,
            2,
            "expected 2 communities: {labels:?}"
        );
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
                (0, 1),
                (0, 2),
                (0, 3),
                (1, 2),
                (1, 3),
                (2, 3),
                (4, 5),
                (4, 6),
                (4, 7),
                (5, 6),
                (5, 7),
                (6, 7),
                (3, 4),
            ],
        );
        let labels = leiden(&g, 1.0);
        assert!(
            all_connected(&g, &labels),
            "communities not connected: {labels:?}"
        );
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
