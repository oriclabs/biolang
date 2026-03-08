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
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
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

/// Simplified UMAP implementation.
///
/// Uses k-nearest neighbors graph + force-directed layout.
pub fn umap(
    data: &[Vec<f64>],
    n_components: usize,
    n_neighbors: usize,
    n_epochs: usize,
    min_dist: f64,
) -> Vec<Vec<f64>> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    let k = n_neighbors.min(n - 1).max(1);

    // Compute pairwise distances
    let mut dists = vec![vec![0.0f64; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d: f64 = data[i]
                .iter()
                .zip(&data[j])
                .map(|(a, b)| (a - b).powi(2))
                .sum::<f64>()
                .sqrt();
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
        let target = (k as f64).ln();
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

    // Initialize embeddings with spectral-like initialization (simple: scaled random)
    let mut rng_state = 42u64;
    let mut embeddings = vec![vec![0.0f64; n_components]; n];
    for row in &mut embeddings {
        for val in row.iter_mut() {
            rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            *val = ((rng_state >> 33) as f64 / u32::MAX as f64 - 0.5) * 10.0;
        }
    }

    // Optimize layout
    let a = 1.0 / (1.0 + min_dist.powi(2));
    let b = 1.0;

    for epoch in 0..n_epochs {
        let alpha = 1.0 - epoch as f64 / n_epochs as f64;

        for i in 0..n {
            for j in (i + 1)..n {
                let w = graph[i][j];
                if w <= 0.0 {
                    continue;
                }

                let dist_sq: f64 = embeddings[i]
                    .iter()
                    .zip(&embeddings[j])
                    .map(|(a, b)| (a - b).powi(2))
                    .sum();
                let dist_sq = dist_sq.max(1e-10);

                // Attractive force
                let grad_coeff = -2.0 * a * b * dist_sq.powf(b - 1.0) / (1.0 + a * dist_sq.powf(b));
                let force = w * grad_coeff * alpha;

                for d in 0..n_components {
                    let diff = embeddings[i][d] - embeddings[j][d];
                    let f = force * diff;
                    let f = f.clamp(-4.0, 4.0);
                    embeddings[i][d] += f;
                    embeddings[j][d] -= f;
                }

                // Repulsive force (only for non-neighbors, approximate with negative sampling)
                if w < 0.01 {
                    let rep = 2.0 * b / ((0.001 + dist_sq) * (1.0 + a * dist_sq.powf(b)));
                    let rep = rep * alpha;
                    for d in 0..n_components {
                        let diff = embeddings[i][d] - embeddings[j][d];
                        let f = (rep * diff).clamp(-4.0, 4.0);
                        embeddings[i][d] += f;
                        embeddings[j][d] -= f;
                    }
                }
            }
        }
    }

    embeddings
}
