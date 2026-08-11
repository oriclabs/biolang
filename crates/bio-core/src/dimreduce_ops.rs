/// Simplified t-SNE implementation (O(n^2) pairwise distances).
///
/// Returns an n x n_components matrix of embeddings.
pub fn tsne(
    data: &[Vec<f64>],
    n_components: usize,
    perplexity: f64,
    n_iter: usize,
    learning_rate: f64,
) -> Vec<Vec<f64>> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }

    // Compute pairwise squared distances
    let mut dists = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d: f64 = data[i]
                .iter()
                .zip(&data[j])
                .map(|(a, b)| (a - b).powi(2))
                .sum();
            dists[i][j] = d;
            dists[j][i] = d;
        }
    }

    // Compute pairwise affinities (P matrix) using Gaussian kernel
    let mut p = vec![vec![0.0f64; n]; n];
    let target_entropy = perplexity.ln();

    for i in 0..n {
        // Binary search for sigma
        let mut lo = 1e-10f64;
        let mut hi = 1e4f64;
        for _ in 0..50 {
            let sigma = (lo + hi) / 2.0;
            let mut sum = 0.0f64;
            for j in 0..n {
                if i != j {
                    p[i][j] = (-dists[i][j] / (2.0 * sigma * sigma)).exp();
                    sum += p[i][j];
                }
            }
            if sum > 0.0 {
                for j in 0..n {
                    if i != j {
                        p[i][j] /= sum;
                    }
                }
            }
            // Compute entropy
            let entropy: f64 = p[i]
                .iter()
                .filter(|&&v| v > 1e-10)
                .map(|v| -v * v.ln())
                .sum();

            if entropy > target_entropy {
                hi = sigma;
            } else {
                lo = sigma;
            }
        }
    }

    // Symmetrize
    for i in 0..n {
        for j in (i + 1)..n {
            let sym = (p[i][j] + p[j][i]) / (2.0 * n as f64);
            p[i][j] = sym.max(1e-12);
            p[j][i] = sym.max(1e-12);
        }
    }

    // Initialize embeddings randomly (deterministic seed via simple LCG)
    let mut rng_state = 42u64;
    let mut embeddings = vec![vec![0.0f64; n_components]; n];
    for row in &mut embeddings {
        for val in row.iter_mut() {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            *val = ((rng_state >> 33) as f64 / u32::MAX as f64 - 0.5) * 0.01;
        }
    }

    // Gradient descent
    let mut gains = vec![vec![1.0f64; n_components]; n];
    let mut velocities = vec![vec![0.0f64; n_components]; n];
    let momentum = 0.8;

    for _iter in 0..n_iter {
        // Compute Q matrix (Student-t with 1 DOF)
        let mut q = vec![vec![0.0f64; n]; n];
        let mut q_sum = 0.0f64;
        for i in 0..n {
            for j in (i + 1)..n {
                let d: f64 = embeddings[i]
                    .iter()
                    .zip(&embeddings[j])
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                let qij = 1.0 / (1.0 + d);
                q[i][j] = qij;
                q[j][i] = qij;
                q_sum += 2.0 * qij;
            }
        }
        if q_sum > 0.0 {
            for i in 0..n {
                for j in 0..n {
                    q[i][j] /= q_sum;
                    q[i][j] = q[i][j].max(1e-12);
                }
            }
        }

        // Compute gradients
        for i in 0..n {
            for d in 0..n_components {
                let mut grad = 0.0f64;
                for j in 0..n {
                    if i != j {
                        let diff = embeddings[i][d] - embeddings[j][d];
                        let dist: f64 = embeddings[i]
                            .iter()
                            .zip(&embeddings[j])
                            .map(|(a, b)| (a - b).powi(2))
                            .sum();
                        let qij = 1.0 / (1.0 + dist);
                        grad += 4.0 * (p[i][j] - q[i][j]) * qij * diff;
                    }
                }

                // Adaptive learning rate
                if (grad > 0.0) != (velocities[i][d] > 0.0) {
                    gains[i][d] = (gains[i][d] + 0.2).min(5.0);
                } else {
                    gains[i][d] = (gains[i][d] * 0.8).max(0.01);
                }

                velocities[i][d] = momentum * velocities[i][d] - learning_rate * gains[i][d] * grad;
                embeddings[i][d] += velocities[i][d];
            }
        }

        // Center embeddings
        for d in 0..n_components {
            let mean: f64 = embeddings.iter().map(|e| e[d]).sum::<f64>() / n as f64;
            for e in &mut embeddings {
                e[d] -= mean;
            }
        }
    }

    embeddings
}

/// UMAP with the paper's fuzzy-neighbour graph and cross-entropy optimiser.
///
/// This compatibility wrapper preserves the original public Rust API.
pub fn umap(
    data: &[Vec<f64>],
    n_components: usize,
    n_neighbors: usize,
    n_epochs: usize,
    min_dist: f64,
) -> Vec<Vec<f64>> {
    umap_configured(
        data,
        n_components,
        n_neighbors,
        n_epochs,
        min_dist,
        1.0,
        "euclidean",
        42,
        5,
    )
}

/// Paper-derived UMAP implementation with explicit reproducibility options.
pub fn umap_configured(
    data: &[Vec<f64>],
    n_components: usize,
    n_neighbors: usize,
    n_epochs: usize,
    min_dist: f64,
    spread: f64,
    metric: &str,
    seed: u64,
    negative_sample_rate: usize,
) -> Vec<Vec<f64>> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![vec![0.0; n_components]];
    }
    let k = n_neighbors.min(n - 1).max(1);

    // Compute pairwise distances
    let mut dists = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = input_distance(&data[i], &data[j], metric);
            dists[i][j] = d;
            dists[j][i] = d;
        }
    }

    // Build k-NN graph with fuzzy set union
    let mut graph = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        let mut neighbors: Vec<(usize, f64)> = (0..n)
            .filter(|&j| j != i)
            .map(|j| (j, dists[i][j]))
            .collect();
        neighbors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        neighbors.truncate(k);

        let rho = neighbors.first().map(|n| n.1).unwrap_or(0.0);
        // Binary search for sigma
        let mut sigma = 1.0f64;
        let target = (k as f64).log2();
        let mut lo = 1e-10f64;
        let mut hi = 1e4f64;
        for _ in 0..64 {
            sigma = (lo + hi) / 2.0;
            let sum: f64 = neighbors
                .iter()
                .map(|(_, d)| (-((d - rho).max(0.0)) / sigma).exp())
                .sum();
            if sum > target {
                hi = sigma;
            } else {
                lo = sigma;
            }
        }

        for &(j, d) in &neighbors {
            let w = (-((d - rho).max(0.0)) / sigma).exp();
            graph[i][j] = w;
        }
    }

    // Symmetrize: fuzzy set union
    for i in 0..n {
        for j in (i + 1)..n {
            let sym = graph[i][j] + graph[j][i] - graph[i][j] * graph[j][i];
            graph[i][j] = sym;
            graph[j][i] = sym;
        }
    }

    let mut rng_state = seed.max(1);
    let mut embeddings = spectral_initialisation(&graph, n_components, &mut rng_state);

    // Optimize layout.
    //
    // Attraction pulls neighbours together; repulsion pushes everything else
    // apart. The previous version had only the first: it skipped any pair with
    // `w <= 0.0` before reaching the repulsion branch, and `w` is non-zero only
    // for k-nearest neighbours — so the pairs that needed pushing apart were
    // exactly the ones it never looked at. Every embedding collapsed to one
    // blob regardless of the input.
    //
    // Repulsion over all pairs would be O(n^2) per epoch. Real UMAP samples a
    // few negative pairs per positive edge instead, which is what this does.
    let (a, b) = fit_ab(spread.max(1e-6), min_dist.max(0.0));
    const INITIAL_ALPHA: f64 = 1.0;

    // Positive edges, once — iterating the dense graph every epoch was most of
    // the cost and none of the information.
    let mut edges: Vec<(usize, usize, f64)> = Vec::new();
    for i in 0..n {
        for j in (i + 1)..n {
            if graph[i][j] > 0.0 {
                edges.push((i, j, graph[i][j]));
            }
        }
    }

    let clamp_to = 4.0_f64;
    for epoch in 0..n_epochs {
        let alpha = INITIAL_ALPHA * (1.0 - epoch as f64 / n_epochs as f64);

        for &(i, j, w) in &edges {
            // ── attraction, along the edge ──
            let dist_sq: f64 = embeddings[i]
                .iter()
                .zip(&embeddings[j])
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f64>()
                .max(1e-10);
            // Reference UMAP samples each edge at a rate set by its weight
            // rather than scaling the gradient by it. Scaling instead makes
            // attraction systematically weaker than the five unscaled negative
            // samples that follow, and the layout blows apart. Sampling by
            // weight keeps the two in balance.
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            // Use all 32 high bits. Shifting by 33 but dividing by u32::MAX
            // restricted draws to [0, 0.5], roughly doubling every edge's
            // sampling probability and distorting the UMAP objective.
            let draw = ((rng_state >> 32) as f64) / (u32::MAX as f64);
            if draw > w {
                continue;
            }
            let grad = -2.0 * a * b * dist_sq.powf(b - 1.0) / (1.0 + a * dist_sq.powf(b));
            let coeff = grad * alpha;
            for d in 0..n_components {
                let diff = embeddings[i][d] - embeddings[j][d];
                let step = (coeff * diff).clamp(-clamp_to, clamp_to);
                embeddings[i][d] += step;
                embeddings[j][d] -= step;
            }

            // ── repulsion, against random non-neighbours ──
            for _ in 0..negative_sample_rate {
                rng_state = rng_state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let k = ((rng_state >> 32) as usize) % n;
                if k == i {
                    continue;
                }
                let dist_sq: f64 = embeddings[i]
                    .iter()
                    .zip(&embeddings[k])
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f64>()
                    .max(1e-10);
                let grad = 2.0 * b / ((0.001 + dist_sq) * (1.0 + a * dist_sq.powf(b)));
                let coeff = grad * alpha;
                for d in 0..n_components {
                    let diff = embeddings[i][d] - embeddings[k][d];
                    let step = (coeff * diff).clamp(-clamp_to, clamp_to);
                    embeddings[i][d] += step;
                }
            }
        }
    }

    embeddings
}

/// UMAP from a directed k-nearest-neighbour list.
///
/// Rows contain `(cell_index, distance)` pairs. Keeping approximate indexing
/// outside the optimiser makes the fuzzy-set and cross-entropy implementation
/// reusable without ever allocating an n-by-n matrix.
pub fn umap_from_knn(
    neighbours: &[Vec<(usize, f64)>],
    n_components: usize,
    n_epochs: usize,
    min_dist: f64,
    spread: f64,
    seed: u64,
    negative_sample_rate: usize,
) -> Vec<Vec<f64>> {
    use std::collections::{BTreeMap, BTreeSet};

    let n = neighbours.len();
    if n == 0 {
        return vec![];
    }
    if n == 1 {
        return vec![vec![0.0; n_components]];
    }

    let mut directed: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for (i, row) in neighbours.iter().enumerate() {
        let valid: Vec<(usize, f64)> = row
            .iter()
            .copied()
            .filter(|(j, distance)| *j < n && *j != i && distance.is_finite())
            .collect();
        if valid.is_empty() {
            continue;
        }
        let rho = valid
            .iter()
            .map(|(_, distance)| *distance)
            .fold(f64::INFINITY, f64::min);
        let target = (valid.len() as f64).log2();
        let mut lo = 1e-10_f64;
        let mut hi = 1e4_f64;
        let mut sigma = 1.0_f64;
        for _ in 0..64 {
            sigma = (lo + hi) / 2.0;
            let sum: f64 = valid
                .iter()
                .map(|(_, distance)| (-((distance - rho).max(0.0)) / sigma).exp())
                .sum();
            if sum > target {
                hi = sigma;
            } else {
                lo = sigma;
            }
        }
        for &(j, distance) in &valid {
            let weight = (-((distance - rho).max(0.0)) / sigma).exp();
            directed.insert((i, j), weight);
        }
    }

    // Fuzzy union: a + b - ab. BTree containers make edge order and therefore
    // stochastic optimisation deterministic for a fixed seed.
    let mut pairs = BTreeSet::new();
    for &(i, j) in directed.keys() {
        pairs.insert((i.min(j), i.max(j)));
    }
    let edges: Vec<(usize, usize, f64)> = pairs
        .into_iter()
        .filter_map(|(i, j)| {
            let forward = directed.get(&(i, j)).copied().unwrap_or(0.0);
            let reverse = directed.get(&(j, i)).copied().unwrap_or(0.0);
            let union = forward + reverse - forward * reverse;
            (union > 0.0).then_some((i, j, union))
        })
        .collect();

    let mut rng_state = seed.max(1);
    let mut embeddings = spectral_initialisation_sparse(n, &edges, n_components, &mut rng_state);
    let (a, b) = fit_ab(spread.max(1e-6), min_dist.max(0.0));
    let clamp_to = 4.0_f64;

    for epoch in 0..n_epochs {
        let alpha = 1.0 - epoch as f64 / n_epochs.max(1) as f64;
        for &(i, j, weight) in &edges {
            rng_state = rng_state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let draw = ((rng_state >> 32) as f64) / u32::MAX as f64;
            if draw > weight {
                continue;
            }

            let distance_squared = embeddings[i]
                .iter()
                .zip(&embeddings[j])
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f64>()
                .max(1e-10);
            let gradient = -2.0 * a * b * distance_squared.powf(b - 1.0)
                / (1.0 + a * distance_squared.powf(b));
            for dimension in 0..n_components {
                let difference = embeddings[i][dimension] - embeddings[j][dimension];
                let step = (gradient * alpha * difference).clamp(-clamp_to, clamp_to);
                embeddings[i][dimension] += step;
                embeddings[j][dimension] -= step;
            }

            for _ in 0..negative_sample_rate {
                rng_state = rng_state
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let negative = ((rng_state >> 32) as usize) % n;
                if negative == i {
                    continue;
                }
                let distance_squared = embeddings[i]
                    .iter()
                    .zip(&embeddings[negative])
                    .map(|(x, y)| (x - y).powi(2))
                    .sum::<f64>()
                    .max(1e-10);
                let gradient =
                    2.0 * b / ((0.001 + distance_squared) * (1.0 + a * distance_squared.powf(b)));
                for dimension in 0..n_components {
                    let difference = embeddings[i][dimension] - embeddings[negative][dimension];
                    let step = (gradient * alpha * difference).clamp(-clamp_to, clamp_to);
                    embeddings[i][dimension] += step;
                }
            }
        }
    }
    embeddings
}

fn input_distance(a: &[f64], b: &[f64], metric: &str) -> f64 {
    if metric.eq_ignore_ascii_case("cosine") {
        let dot: f64 = a.iter().zip(b).map(|(x, y)| x * y).sum();
        let na: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
        let nb: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
        if na <= 1e-15 || nb <= 1e-15 {
            return if na <= 1e-15 && nb <= 1e-15 { 0.0 } else { 1.0 };
        }
        return (1.0 - dot / (na * nb)).clamp(0.0, 2.0);
    }
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y) * (x - y))
        .sum::<f64>()
        .sqrt()
}

/// Fit UMAP's differentiable distance curve to the target defined by spread
/// and min_dist. Gradient descent is over log(a), log(b), keeping both positive.
fn fit_ab(spread: f64, min_dist: f64) -> (f64, f64) {
    let mut log_a = 0.0_f64;
    let mut log_b = 0.0_f64;
    const SAMPLES: usize = 300;
    for iteration in 0..2000 {
        let a = log_a.exp();
        let b = log_b.exp();
        let mut grad_a = 0.0;
        let mut grad_b = 0.0;
        for index in 1..=SAMPLES {
            let x = spread * 3.0 * index as f64 / SAMPLES as f64;
            let target = if x <= min_dist {
                1.0
            } else {
                (-(x - min_dist) / spread).exp()
            };
            let power = x.powf(2.0 * b);
            let denominator = 1.0 + a * power;
            let predicted = 1.0 / denominator;
            let error = predicted - target;
            let d_da = -a * power / (denominator * denominator);
            let d_db = if x > 0.0 {
                -a * power * 2.0 * b * x.ln() / (denominator * denominator)
            } else {
                0.0
            };
            grad_a += 2.0 * error * d_da;
            grad_b += 2.0 * error * d_db;
        }
        let rate = 0.5 / (SAMPLES as f64 * (1.0 + iteration as f64 / 500.0));
        log_a = (log_a - rate * grad_a).clamp(-8.0, 8.0);
        log_b = (log_b - rate * grad_b).clamp(-4.0, 4.0);
    }
    (log_a.exp(), log_b.exp())
}

/// Leading non-trivial eigenvectors of the normalised fuzzy adjacency.
fn spectral_initialisation_sparse(
    n: usize,
    edges: &[(usize, usize, f64)],
    n_components: usize,
    rng_state: &mut u64,
) -> Vec<Vec<f64>> {
    let mut degree = vec![0.0_f64; n];
    for &(i, j, weight) in edges {
        degree[i] += weight;
        degree[j] += weight;
    }
    for value in &mut degree {
        *value = value.max(1e-12);
    }
    let trivial: Vec<f64> = {
        let mut vector: Vec<f64> = degree.iter().map(|value| value.sqrt()).collect();
        normalise(&mut vector);
        vector
    };
    let mut vectors: Vec<Vec<f64>> = (0..n_components)
        .map(|component| {
            (0..n)
                .map(|cell| (((cell + 1) * (component + 1)) as f64 * 1.618_033_988_75).sin())
                .collect()
        })
        .collect();
    orthogonalise_vectors(&mut vectors, &trivial);
    for _ in 0..100 {
        let mut next = Vec::with_capacity(vectors.len());
        for vector in &vectors {
            let mut out = vec![0.0; n];
            for &(i, j, weight) in edges {
                let normalised = weight / (degree[i] * degree[j]).sqrt();
                out[i] += normalised * vector[j];
                out[j] += normalised * vector[i];
            }
            next.push(out);
        }
        orthogonalise_vectors(&mut next, &trivial);
        vectors = next;
    }

    let largest = vectors
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1e-12);
    (0..n)
        .map(|cell| {
            vectors
                .iter()
                .map(|vector| {
                    *rng_state = rng_state
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let jitter = ((*rng_state >> 33) as f64 / u32::MAX as f64 - 0.5) * 1e-4;
                    vector[cell] / largest * 10.0 + jitter
                })
                .collect()
        })
        .collect()
}

fn spectral_initialisation(
    graph: &[Vec<f64>],
    n_components: usize,
    rng_state: &mut u64,
) -> Vec<Vec<f64>> {
    let n = graph.len();
    let degree: Vec<f64> = graph
        .iter()
        .map(|row| row.iter().sum::<f64>().max(1e-12))
        .collect();
    let trivial: Vec<f64> = {
        let mut vector: Vec<f64> = degree.iter().map(|d| d.sqrt()).collect();
        normalise(&mut vector);
        vector
    };
    let mut vectors: Vec<Vec<f64>> = (0..n_components)
        .map(|component| {
            (0..n)
                .map(|cell| (((cell + 1) * (component + 1)) as f64 * 1.618_033_988_75).sin())
                .collect()
        })
        .collect();
    orthogonalise_vectors(&mut vectors, &trivial);
    for _ in 0..100 {
        let mut next = Vec::with_capacity(vectors.len());
        for vector in &vectors {
            let mut out = vec![0.0; n];
            for i in 0..n {
                for j in 0..n {
                    let weight = graph[i][j];
                    if weight > 0.0 {
                        out[i] += weight * vector[j] / (degree[i] * degree[j]).sqrt();
                    }
                }
            }
            next.push(out);
        }
        orthogonalise_vectors(&mut next, &trivial);
        vectors = next;
    }

    let largest = vectors
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1e-12);
    (0..n)
        .map(|cell| {
            vectors
                .iter()
                .map(|vector| {
                    *rng_state = rng_state
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    let jitter = ((*rng_state >> 33) as f64 / u32::MAX as f64 - 0.5) * 1e-4;
                    vector[cell] / largest * 10.0 + jitter
                })
                .collect()
        })
        .collect()
}

fn orthogonalise_vectors(vectors: &mut [Vec<f64>], trivial: &[f64]) {
    for i in 0..vectors.len() {
        subtract_projection(&mut vectors[i], trivial);
        let (before, after) = vectors.split_at_mut(i);
        let current = &mut after[0];
        for basis in before.iter() {
            subtract_projection(current, basis);
        }
        normalise(current);
    }
}

fn subtract_projection(vector: &mut [f64], basis: &[f64]) {
    let coefficient: f64 = vector.iter().zip(basis).map(|(a, b)| a * b).sum();
    for (value, direction) in vector.iter_mut().zip(basis) {
        *value -= coefficient * direction;
    }
}

fn normalise(vector: &mut [f64]) {
    let norm = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
    if norm > 1e-15 {
        for value in vector {
            *value /= norm;
        }
    }
}

#[cfg(test)]
mod umap_layout_tests {
    use super::*;

    /// Three groups far apart in the input, tight within themselves.
    /// Any embedding worth the name must keep them apart.
    fn three_clumps() -> Vec<Vec<f64>> {
        let mut points = Vec::new();
        for group in 0..3 {
            for i in 0..60 {
                points.push(vec![
                    f64::from(group) * 100.0 + f64::from(i % 10) * 0.1,
                    f64::from(i / 10) * 0.1,
                    0.0,
                ]);
            }
        }
        points
    }

    fn centroid(embedding: &[Vec<f64>], group: usize) -> (f64, f64) {
        let rows = &embedding[group * 60..(group + 1) * 60];
        let n = rows.len() as f64;
        (
            rows.iter().map(|r| r[0]).sum::<f64>() / n,
            rows.iter().map(|r| r[1]).sum::<f64>() / n,
        )
    }

    fn spread(embedding: &[Vec<f64>], group: usize) -> f64 {
        let (cx, cy) = centroid(embedding, group);
        let rows = &embedding[group * 60..(group + 1) * 60];
        rows.iter()
            .map(|r| ((r[0] - cx).powi(2) + (r[1] - cy).powi(2)).sqrt())
            .sum::<f64>()
            / rows.len() as f64
    }

    fn gap(embedding: &[Vec<f64>], a: usize, b: usize) -> f64 {
        let (ax, ay) = centroid(embedding, a);
        let (bx, by) = centroid(embedding, b);
        ((ax - bx).powi(2) + (ay - by).powi(2)).sqrt()
    }

    #[test]
    fn separated_input_stays_separated() {
        // The property the whole method exists for, and the one that was
        // missing: the layout had attraction but no reachable repulsion — every
        // non-neighbour pair hit a `continue` before the repulsive branch — so
        // any input at all collapsed into a single blob. Nothing tested the
        // algorithm, only the plot that draws it.
        let embedding = umap(&three_clumps(), 2, 15, 200, 0.1);
        assert_eq!(embedding.len(), 180);

        let widest = (0..3)
            .map(|g| spread(&embedding, g))
            .fold(f64::MIN, f64::max);
        for (a, b) in [(0, 1), (0, 2), (1, 2)] {
            let apart = gap(&embedding, a, b);
            assert!(
                apart > widest,
                "groups {a} and {b} are {apart:.2} apart but groups are {widest:.2} wide \
                 — the embedding did not separate them"
            );
        }
    }

    #[test]
    fn the_layout_does_not_collapse_to_a_point() {
        // The failure mode directly: a blob passes any "did it run" check.
        let embedding = umap(&three_clumps(), 2, 15, 200, 0.1);
        let xs: Vec<f64> = embedding.iter().map(|r| r[0]).collect();
        let extent =
            xs.iter().fold(f64::MIN, |m, v| m.max(*v)) - xs.iter().fold(f64::MAX, |m, v| m.min(*v));
        assert!(extent > 1.0, "embedding spans only {extent:.3}");
    }

    #[test]
    fn sparse_knn_entry_point_preserves_separated_groups() {
        let data = three_clumps();
        let neighbours: Vec<Vec<(usize, f64)>> = (0..data.len())
            .map(|i| {
                let mut row: Vec<(usize, f64)> = (0..data.len())
                    .filter(|&j| j != i)
                    .map(|j| (j, input_distance(&data[i], &data[j], "euclidean")))
                    .collect();
                row.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                row.truncate(15);
                row
            })
            .collect();
        let embedding = umap_from_knn(&neighbours, 2, 200, 0.1, 1.0, 42, 5);
        assert_eq!(embedding.len(), data.len());
        assert!(embedding.iter().flatten().all(|value| value.is_finite()));
        let widest = (0..3)
            .map(|group| spread(&embedding, group))
            .fold(f64::MIN, f64::max);
        assert!(gap(&embedding, 0, 2) > widest);
    }

    #[test]
    fn the_same_input_gives_the_same_layout() {
        // Seeded, so published figures do not move between runs.
        let data = three_clumps();
        assert_eq!(umap(&data, 2, 15, 200, 0.1), umap(&data, 2, 15, 200, 0.1));
    }
}
