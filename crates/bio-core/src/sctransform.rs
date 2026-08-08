//! Regularized negative binomial normalization for UMI counts.
//!
//! An independent implementation of the method described in:
//!
//!   Hafemeister C, Satija R (2019). Normalization and variance stabilization
//!   of single-cell RNA-seq data using regularized negative binomial
//!   regression. Genome Biology 20:296.
//!
//!   Choudhary S, Satija R (2022). Comparison and evaluation of statistical
//!   error models for scRNA-seq. Genome Biology 23:27.
//!
//! Written from the published descriptions. The reference implementation is
//! GPL-3 and was not consulted; this crate is MIT and stays that way.
//!
//! # What the method is for
//!
//! Log-normalization leaves a depth-expression relationship behind: deeply
//! sequenced cells still look different from shallow ones after correction, and
//! that residual structure walks straight into the principal components. The
//! fix is to model each gene's counts against sequencing depth explicitly and
//! keep what the model cannot explain.
//!
//! Per gene, counts are negative binomial around a depth-proportional mean:
//!
//! ```text
//!   y_ij ~ NB(mu_ij, theta_j),    mu_ij = m_i * s_j / M
//! ```
//!
//! where `m_i` is cell i's total UMI count, `s_j` gene j's total, and `M` the
//! grand total. The Pearson residual `(y - mu) / sqrt(mu + mu^2/theta)` is the
//! normalized value.
//!
//! # Why the mean has no free slope
//!
//! The 2019 model regressed `log(mu)` on `log10(m_i)` with a fitted slope. The
//! 2022 paper showed that slope is essentially always 1 — counts really are
//! proportional to depth — and that fitting it overfits genes with little
//! signal. Fixing it makes the model an offset Poisson, whose maximum
//! likelihood estimate for the intercept is closed-form: `s_j / M`. So the mean
//! above needs no iteration at all.
//!
//! Everything hard is in `theta`, the overdispersion, and in regularizing it.
//!
//! # Why theta has to be regularized
//!
//! Estimating theta per gene independently is noisy, and noisiest exactly where
//! it matters: lowly expressed genes have few non-zero observations to estimate
//! from. Feeding those raw estimates into the residual gives lowly expressed
//! genes wildly overstated residuals and floods the variable-gene list with
//! noise.
//!
//! The method's answer, and the thing it is named for: theta varies smoothly
//! with expression level, so fit it on a subset of genes, smooth those
//! estimates against expression, and read every gene's theta off the smooth
//! curve. An individual gene's noisy estimate is replaced by what genes of its
//! abundance typically look like.

use std::sync::Arc;
use std::thread;

/// Options, defaulting to the published defaults.
#[derive(Debug, Clone)]
pub struct SctOptions {
    /// Genes to estimate theta on before smoothing. The curve is smooth, so
    /// sampling it does not cost accuracy, and estimating theta is the
    /// expensive step.
    pub genes_for_fit: usize,
    /// Multiplies the kernel bandwidth. The reference uses 3; a wider kernel
    /// smooths harder, which is the conservative direction for a regularizer.
    pub bandwidth_adjust: f64,
    /// Residual clip. `None` means `sqrt(n_cells / 30)`, the published default.
    pub clip: Option<f64>,
    /// Genes must be seen in at least this many cells to be modelled at all.
    pub min_cells: usize,
    /// Keep only this many genes, ranked by residual variance. `None` keeps all.
    pub n_variable_features: Option<usize>,
    /// Worker threads. 0 asks the runtime.
    pub threads: usize,
}

impl Default for SctOptions {
    fn default() -> Self {
        Self {
            genes_for_fit: 2000,
            bandwidth_adjust: 3.0,
            clip: None,
            min_cells: 5,
            n_variable_features: None,
            threads: 0,
        }
    }
}

/// A gene-major sparse view: for each gene, the `(cell, count)` pairs that are
/// non-zero. Gene-major because every step here works down a gene at a time.
pub struct GeneColumns {
    pub n_cells: usize,
    /// One entry per gene; each is `(cell_index, count)` sorted by cell.
    pub columns: Vec<Vec<(usize, f64)>>,
}

#[derive(Debug, Clone)]
pub struct SctResult {
    /// Row-major `n_cells * kept_genes.len()`.
    pub residuals: Vec<f64>,
    /// Indices into the original gene axis, ascending.
    pub kept_genes: Vec<usize>,
    /// Variance of the residuals, per kept gene, in the same order.
    pub residual_variance: Vec<f64>,
    /// Regularized theta, per kept gene.
    pub theta: Vec<f64>,
}

/// Digamma, the derivative of log-gamma.
///
/// Recurrence up to a large argument, then the asymptotic series. Needed
/// because the derivative of the negative binomial log-likelihood with respect
/// to theta is written in digammas, and solving for theta means evaluating it
/// once per observation per iteration.
fn digamma(mut x: f64) -> f64 {
    let mut result = 0.0;
    while x < 6.0 {
        result -= 1.0 / x;
        x += 1.0;
    }
    let inverse = 1.0 / x;
    let square = inverse * inverse;
    result + x.ln() - 0.5 * inverse
        - square
            * (1.0 / 12.0
                - square * (1.0 / 120.0 - square * (1.0 / 252.0 - square * (1.0 / 240.0))))
}

/// Derivative of the negative binomial log-likelihood with respect to theta.
///
/// With `mu` fixed, the likelihood in theta alone is
///
/// ```text
///   sum_i [ lgamma(y_i + t) - lgamma(t) + t*log(t) - t*log(t + mu_i)
///           + y_i*log(mu_i) - (y_i + t)*log(t + mu_i) ]      (+ const)
/// ```
///
/// so the score is
///
/// ```text
///   sum_i [ digamma(y_i + t) - digamma(t) + log(t) + 1
///           - log(t + mu_i) - (y_i + t)/(t + mu_i) ]
/// ```
///
/// Zero counts dominate a UMI matrix, and for those `digamma(y + t)` is just
/// `digamma(t)` — a constant that can be lifted out of the loop rather than
/// recomputed a few hundred million times. That single observation is what
/// makes fitting at this scale practical.
fn theta_score(counts: &[(usize, f64)], cell_mu: &[f64], theta: f64) -> f64 {
    let digamma_theta = digamma(theta);
    let ln_theta = theta.ln();
    let mut total = 0.0;

    // The part every cell contributes, zero or not.
    for &mu in cell_mu {
        let sum = theta + mu;
        total += ln_theta + 1.0 - sum.ln() - theta / sum;
    }
    // Corrections for the cells that actually observed something.
    for &(cell, count) in counts {
        let sum = theta + cell_mu[cell];
        total += digamma(count + theta) - digamma_theta - count / sum;
    }
    total
}

/// Maximum-likelihood theta for one gene, by bisection on the score.
///
/// Bisection rather than Newton: the score is monotone decreasing in theta over
/// the range that matters, so bisection cannot diverge, and the second
/// derivative costs another digamma evaluation per observation that buys little
/// when the interval is already bracketed.
///
/// Returns `None` when the gene shows no overdispersion at all — the score
/// stays positive however large theta gets, meaning the counts are Poisson or
/// tighter. That is a real answer, not a failure, and the caller excludes such
/// genes from the smoothing fit rather than letting an arbitrary ceiling drag
/// the curve.
fn fit_theta(counts: &[(usize, f64)], cell_mu: &[f64]) -> Option<f64> {
    const LOWER: f64 = 1e-3;
    const UPPER: f64 = 1e5;
    const ITERATIONS: usize = 60;

    if theta_score(counts, cell_mu, UPPER) > 0.0 {
        return None;
    }
    if theta_score(counts, cell_mu, LOWER) < 0.0 {
        // Overdispersed beyond the range worth modelling.
        return Some(LOWER);
    }
    let (mut low, mut high) = (LOWER, UPPER);
    for _ in 0..ITERATIONS {
        // Geometric midpoint: theta ranges over orders of magnitude, so
        // bisecting the log is what converges in a predictable number of steps.
        let middle = (low * high).sqrt();
        if theta_score(counts, cell_mu, middle) > 0.0 {
            low = middle;
        } else {
            high = middle;
        }
        if high / low < 1.0 + 1e-6 {
            break;
        }
    }
    Some((low * high).sqrt())
}

/// Gaussian kernel regression, evaluated at `at`.
fn kernel_smooth(points: &[(f64, f64)], at: f64, bandwidth: f64) -> f64 {
    let mut weight_total = 0.0;
    let mut weighted = 0.0;
    for &(x, y) in points {
        let z = (x - at) / bandwidth;
        // Beyond four bandwidths the weight is under 1e-4 and contributes
        // nothing but arithmetic.
        if z.abs() > 4.0 {
            continue;
        }
        let weight = (-0.5 * z * z).exp();
        weight_total += weight;
        weighted += weight * y;
    }
    if weight_total > 0.0 {
        weighted / weight_total
    } else {
        // Nothing within reach: fall back to the nearest observation rather
        // than to zero, which would be a silent, wrong answer.
        points
            .iter()
            .min_by(|a, b| (a.0 - at).abs().total_cmp(&(b.0 - at).abs()))
            .map(|&(_, y)| y)
            .unwrap_or(0.0)
    }
}

/// Silverman's rule for a Gaussian kernel, times the adjustment factor.
///
/// The reference selects bandwidth by the Sheather-Jones plug-in rule. This
/// uses Silverman's rule of thumb instead, which is the standard alternative
/// and typically lands within a small factor. It is a deliberate substitution
/// and the one place in this module where the numbers can be expected to drift
/// from the reference for reasons other than floating point: a different
/// bandwidth means a slightly differently smoothed theta curve.
fn silverman_bandwidth(values: &[f64], adjust: f64) -> f64 {
    let n = values.len();
    if n < 2 {
        return 1.0;
    }
    let mean = values.iter().sum::<f64>() / n as f64;
    let variance = values.iter().map(|v| (v - mean) * (v - mean)).sum::<f64>() / (n - 1) as f64;
    let sd = variance.max(0.0).sqrt();

    let mut sorted: Vec<f64> = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let quantile = |q: f64| -> f64 {
        let position = q * (n - 1) as f64;
        let low = position.floor() as usize;
        let high = position.ceil() as usize;
        let fraction = position - low as f64;
        sorted[low] * (1.0 - fraction) + sorted[high] * fraction
    };
    let iqr = quantile(0.75) - quantile(0.25);

    // Silverman takes the smaller of the two spread estimates so a heavy tail
    // does not inflate the bandwidth; IQR/1.349 is the normal-consistent form.
    let spread = if iqr > 0.0 {
        sd.min(iqr / 1.349)
    } else {
        sd
    };
    let bandwidth = 0.9 * spread * (n as f64).powf(-0.2) * adjust;
    if bandwidth > 0.0 && bandwidth.is_finite() {
        bandwidth
    } else {
        1.0
    }
}

/// Run the transform.
pub fn sctransform(data: &GeneColumns, options: &SctOptions) -> SctResult {
    let n_cells = data.n_cells;
    let n_genes = data.columns.len();
    if n_cells == 0 || n_genes == 0 {
        return SctResult {
            residuals: vec![],
            kept_genes: vec![],
            residual_variance: vec![],
            theta: vec![],
        };
    }

    // ── Margins ──────────────────────────────────────────────────────
    let mut cell_totals = vec![0.0f64; n_cells];
    let mut gene_totals = vec![0.0f64; n_genes];
    for (gene, column) in data.columns.iter().enumerate() {
        for &(cell, count) in column {
            cell_totals[cell] += count;
            gene_totals[gene] += count;
        }
    }
    let grand_total: f64 = gene_totals.iter().sum::<f64>().max(1e-12);

    // Genes seen in too few cells cannot support an overdispersion estimate.
    let modelled: Vec<usize> = (0..n_genes)
        .filter(|&gene| data.columns[gene].len() >= options.min_cells && gene_totals[gene] > 0.0)
        .collect();
    if modelled.is_empty() {
        return SctResult {
            residuals: vec![],
            kept_genes: vec![],
            residual_variance: vec![],
            theta: vec![],
        };
    }

    // The regression covariate: log10 of the gene's mean count per cell.
    let log_mean = |gene: usize| -> f64 { (gene_totals[gene] / n_cells as f64).log10() };

    // ── Which genes to estimate theta on ─────────────────────────────
    //
    // Evenly through the expression range rather than at random. The curve is
    // being fitted *against* expression, so what it needs is coverage of that
    // axis; a uniform sample over genes would crowd wherever genes happen to be
    // dense and leave the ends thin, which is where extrapolation hurts.
    let mut by_expression = modelled.clone();
    by_expression.sort_by(|&a, &b| log_mean(a).total_cmp(&log_mean(b)));
    let fit_genes: Vec<usize> = if by_expression.len() <= options.genes_for_fit {
        by_expression.clone()
    } else {
        let step = by_expression.len() as f64 / options.genes_for_fit as f64;
        (0..options.genes_for_fit)
            .map(|index| by_expression[((index as f64 * step) as usize).min(by_expression.len() - 1)])
            .collect()
    };

    // ── Estimate theta on those genes ────────────────────────────────
    let cell_totals = Arc::new(cell_totals);
    let fitted = parallel_map(options.threads, fit_genes.clone(), {
        let cell_totals = Arc::clone(&cell_totals);
        let columns: Vec<Vec<(usize, f64)>> =
            fit_genes.iter().map(|&g| data.columns[g].clone()).collect();
        let totals: Vec<f64> = fit_genes.iter().map(|&g| gene_totals[g]).collect();
        move |slot: usize, _gene: usize| {
            let scale = totals[slot] / grand_total;
            let mu: Vec<f64> = cell_totals.iter().map(|&m| (m * scale).max(1e-12)).collect();
            fit_theta(&columns[slot], &mu)
        }
    });

    // ── Regularize ───────────────────────────────────────────────────
    //
    // Genes with no detectable overdispersion are dropped from the fit rather
    // than pinned to a ceiling: an arbitrary large value is not an observation,
    // and letting a run of them into the smoother bends the curve toward a
    // number nobody measured.
    let observations: Vec<(f64, f64)> = fit_genes
        .iter()
        .zip(&fitted)
        .filter_map(|(&gene, theta)| theta.map(|t| (log_mean(gene), t.log10())))
        .collect();

    let regularized_theta: Vec<f64> = if observations.len() < 2 {
        // Nothing to smooth against. Poisson is the honest fallback: it is what
        // "no overdispersion could be estimated" actually means.
        vec![f64::INFINITY; modelled.len()]
    } else {
        let x_values: Vec<f64> = observations.iter().map(|&(x, _)| x).collect();
        let bandwidth = silverman_bandwidth(&x_values, options.bandwidth_adjust);
        modelled
            .iter()
            .map(|&gene| {
                10f64.powf(kernel_smooth(&observations, log_mean(gene), bandwidth))
            })
            .collect()
    };

    // ── Residuals ────────────────────────────────────────────────────
    let clip = options
        .clip
        .unwrap_or_else(|| (n_cells as f64 / 30.0).sqrt());

    let n_modelled = modelled.len();
    let mut residuals = vec![0.0f64; n_cells * n_modelled];
    let mut residual_variance = vec![0.0f64; n_modelled];

    for (slot, &gene) in modelled.iter().enumerate() {
        let scale = gene_totals[gene] / grand_total;
        let theta = regularized_theta[slot];
        let column = &data.columns[gene];

        // Walk the cells in order, taking counts from the sparse column as they
        // come up. Every cell contributes: a zero count has a non-zero residual,
        // which is exactly why the output is dense.
        let mut next = 0usize;
        let mut sum = 0.0;
        let mut sum_squares = 0.0;
        for cell in 0..n_cells {
            let count = if next < column.len() && column[next].0 == cell {
                let value = column[next].1;
                next += 1;
                value
            } else {
                0.0
            };
            let mu = (cell_totals[cell] * scale).max(1e-12);
            let variance = if theta.is_finite() {
                mu + mu * mu / theta
            } else {
                mu // Poisson limit
            };
            let residual = ((count - mu) / variance.sqrt()).clamp(-clip, clip);
            residuals[cell * n_modelled + slot] = residual;
            sum += residual;
            sum_squares += residual * residual;
        }
        let mean = sum / n_cells as f64;
        residual_variance[slot] = (sum_squares / n_cells as f64 - mean * mean).max(0.0);
    }

    let theta_out = regularized_theta;

    // ── Optionally keep only the most variable genes ─────────────────
    let Some(wanted) = options.n_variable_features.filter(|&n| n < n_modelled) else {
        return SctResult {
            residuals,
            kept_genes: modelled,
            residual_variance,
            theta: theta_out,
        };
    };

    let mut ranked: Vec<usize> = (0..n_modelled).collect();
    ranked.sort_by(|&a, &b| {
        residual_variance[b]
            .total_cmp(&residual_variance[a])
            .then(a.cmp(&b))
    });
    ranked.truncate(wanted);
    ranked.sort_unstable();

    let mut kept = vec![0.0f64; n_cells * wanted];
    for cell in 0..n_cells {
        for (out, &slot) in ranked.iter().enumerate() {
            kept[cell * wanted + out] = residuals[cell * n_modelled + slot];
        }
    }
    SctResult {
        residuals: kept,
        kept_genes: ranked.iter().map(|&slot| modelled[slot]).collect(),
        residual_variance: ranked.iter().map(|&slot| residual_variance[slot]).collect(),
        theta: ranked.iter().map(|&slot| theta_out[slot]).collect(),
    }
}

/// Map `f` over `items`, in order, across threads.
///
/// A few lines of `std::thread` rather than a dependency: `bio-core` carries
/// only `serde`, and one embarrassingly parallel loop does not justify changing
/// that. Work is split into contiguous chunks, which suits this because the
/// items are sorted by expression and adjacent genes cost about the same.
fn parallel_map<T, R, F>(threads: usize, items: Vec<T>, f: F) -> Vec<R>
where
    T: Send + Sync + Copy + 'static,
    R: Send + Default + Clone + 'static,
    F: Fn(usize, T) -> R + Send + Sync + Clone + 'static,
{
    let count = items.len();
    if count == 0 {
        return vec![];
    }
    let threads = if threads == 0 {
        thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        threads
    }
    .min(count)
    .max(1);

    if threads == 1 {
        return items
            .into_iter()
            .enumerate()
            .map(|(index, item)| f(index, item))
            .collect();
    }

    let chunk = count.div_ceil(threads);
    let items = Arc::new(items);
    let mut handles = Vec::with_capacity(threads);
    for worker in 0..threads {
        let start = worker * chunk;
        if start >= count {
            break;
        }
        let end = (start + chunk).min(count);
        let items = Arc::clone(&items);
        let f = f.clone();
        handles.push(thread::spawn(move || {
            (start..end).map(|index| f(index, items[index])).collect::<Vec<R>>()
        }));
    }
    let mut out = Vec::with_capacity(count);
    for handle in handles {
        out.extend(handle.join().unwrap_or_default());
    }
    out.resize(count, R::default());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digamma_matches_known_values() {
        // digamma(1) = -gamma, digamma(0.5) = -gamma - 2 ln 2.
        const EULER_MASCHERONI: f64 = 0.577_215_664_901_532_9;
        assert!((digamma(1.0) + EULER_MASCHERONI).abs() < 1e-9);
        assert!(
            (digamma(0.5) + EULER_MASCHERONI + 2.0 * 2.0f64.ln()).abs() < 1e-9,
            "digamma(0.5) = {}",
            digamma(0.5)
        );
        // Recurrence: digamma(x+1) - digamma(x) = 1/x.
        for x in [0.3, 1.7, 4.2, 11.0] {
            assert!((digamma(x + 1.0) - digamma(x) - 1.0 / x).abs() < 1e-9);
        }
    }

    /// Poisson counts have no overdispersion, so theta must run away rather
    /// than settle on some finite value. Returning None here is what keeps such
    /// genes out of the smoothing fit.
    #[test]
    fn a_poisson_gene_reports_no_overdispersion() {
        let n_cells = 400;
        let mu = vec![5.0; n_cells];
        // Deterministic counts with variance close to the mean.
        let counts: Vec<(usize, f64)> = (0..n_cells)
            .map(|cell| {
                let jitter = ((cell as f64 * 0.7).sin() * 2.2).round();
                (cell, (5.0 + jitter).max(0.0))
            })
            .filter(|&(_, count)| count > 0.0)
            .collect();
        assert_eq!(fit_theta(&counts, &mu), None);
    }

    /// A gene whose counts swing far wider than Poisson must produce a small,
    /// finite theta — small theta means heavy overdispersion.
    #[test]
    fn an_overdispersed_gene_gets_a_small_theta() {
        let n_cells = 400;
        let mu = vec![5.0; n_cells];
        // Half the cells at zero, half at 10: variance 25 against mean 5.
        let counts: Vec<(usize, f64)> = (0..n_cells)
            .filter(|cell| cell % 2 == 0)
            .map(|cell| (cell, 10.0))
            .collect();
        let theta = fit_theta(&counts, &mu).expect("should detect overdispersion");
        assert!(
            theta < 20.0,
            "theta {theta} is too large for counts this overdispersed"
        );
    }

    #[test]
    fn kernel_smoothing_follows_the_trend() {
        // y = 2x, sampled; the smoother should track it in the interior.
        let points: Vec<(f64, f64)> = (0..100)
            .map(|i| {
                let x = i as f64 / 10.0;
                (x, 2.0 * x)
            })
            .collect();
        for at in [2.0, 5.0, 7.0] {
            let got = kernel_smooth(&points, at, 0.5);
            assert!(
                (got - 2.0 * at).abs() < 0.2,
                "at {at}: smoothed to {got}, expected about {}",
                2.0 * at
            );
        }
    }

    fn two_population_data() -> GeneColumns {
        // 200 cells, 60 genes. Genes 0-9 are on in the first half only.
        let n_cells = 200;
        let mut columns = vec![Vec::new(); 60];
        for cell in 0..n_cells {
            for (gene, column) in columns.iter_mut().enumerate() {
                let base = if gene < 10 && cell < n_cells / 2 { 20.0 } else { 3.0 };
                let jitter = ((cell * (gene + 3)) % 5) as f64;
                let count = base + jitter;
                if count > 0.0 {
                    column.push((cell, count));
                }
            }
        }
        GeneColumns { n_cells, columns }
    }

    #[test]
    fn residuals_are_dense_and_shaped_correctly() {
        let data = two_population_data();
        let result = sctransform(&data, &SctOptions::default());
        assert_eq!(result.kept_genes.len(), 60);
        assert_eq!(result.residuals.len(), 200 * 60);
        // A zero count still has a non-zero residual; that is the whole point.
        assert!(
            result.residuals.iter().any(|&r| r != 0.0),
            "every residual was zero"
        );
    }

    /// The genes that actually separate the two populations must rank above
    /// the ones that do not. This is the property variable-feature selection
    /// depends on.
    #[test]
    fn structured_genes_outrank_flat_ones() {
        let data = two_population_data();
        let options = SctOptions {
            n_variable_features: Some(10),
            ..Default::default()
        };
        let result = sctransform(&data, &options);
        assert_eq!(result.kept_genes.len(), 10);
        assert!(
            result.kept_genes.iter().all(|&g| g < 10),
            "selection missed the structured genes: {:?}",
            result.kept_genes
        );
    }

    #[test]
    fn residuals_respect_the_clip() {
        let data = two_population_data();
        let options = SctOptions {
            clip: Some(1.5),
            ..Default::default()
        };
        let result = sctransform(&data, &options);
        assert!(
            result.residuals.iter().all(|&r| r.abs() <= 1.5 + 1e-12),
            "a residual escaped the clip"
        );
    }

    #[test]
    fn threading_does_not_change_the_answer() {
        let data = two_population_data();
        let single = sctransform(&data, &SctOptions { threads: 1, ..Default::default() });
        let many = sctransform(&data, &SctOptions { threads: 8, ..Default::default() });
        assert_eq!(single.kept_genes, many.kept_genes);
        for (a, b) in single.residuals.iter().zip(&many.residuals) {
            assert!((a - b).abs() < 1e-12, "{a} vs {b}");
        }
    }
}
