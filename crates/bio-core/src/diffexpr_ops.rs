/// Result of differential expression analysis for a single gene.
#[derive(Debug, Clone)]
pub struct DiffExprResult {
    pub gene: String,
    pub log2fc: f64,
    pub pvalue: f64,
    pub padj: f64,
    pub mean_a: f64,
    pub mean_b: f64,
}

/// Perform differential expression analysis using Welch's t-test.
///
/// - `counts`: rows are genes, columns are samples
/// - `groups`: group assignment for each column (e.g., [0, 0, 0, 1, 1, 1])
/// - `gene_names`: optional names for each gene (row)
///
/// Returns DiffExprResult per gene with log2 fold change, p-value, and BH-adjusted p-value.
pub fn diff_expr(
    counts: &[Vec<f64>],
    groups: &[usize],
    gene_names: Option<&[String]>,
) -> Vec<DiffExprResult> {
    if counts.is_empty() || groups.is_empty() {
        return vec![];
    }

    let n_genes = counts.len();
    let n_samples = groups.len();

    // Separate sample indices by group
    let mut group_a: Vec<usize> = Vec::new();
    let mut group_b: Vec<usize> = Vec::new();
    for (i, &g) in groups.iter().enumerate() {
        if g == 0 {
            group_a.push(i);
        } else {
            group_b.push(i);
        }
    }

    if group_a.is_empty() || group_b.is_empty() {
        return vec![];
    }

    let na = group_a.len() as f64;
    let nb = group_b.len() as f64;

    let mut results = Vec::with_capacity(n_genes);

    for i in 0..n_genes {
        let row = &counts[i];
        if row.len() < n_samples {
            continue;
        }

        let vals_a: Vec<f64> = group_a.iter().map(|&j| row[j]).collect();
        let vals_b: Vec<f64> = group_b.iter().map(|&j| row[j]).collect();

        let mean_a = vals_a.iter().sum::<f64>() / na;
        let mean_b = vals_b.iter().sum::<f64>() / nb;

        let var_a = if na > 1.0 {
            vals_a.iter().map(|x| (x - mean_a).powi(2)).sum::<f64>() / (na - 1.0)
        } else {
            0.0
        };
        let var_b = if nb > 1.0 {
            vals_b.iter().map(|x| (x - mean_b).powi(2)).sum::<f64>() / (nb - 1.0)
        } else {
            0.0
        };

        // Welch's t-test
        let se = (var_a / na + var_b / nb).sqrt();
        let t_stat = if se > 1e-10 {
            (mean_a - mean_b) / se
        } else {
            0.0
        };

        // Approximate p-value using normal distribution (for large sample sizes)
        // For small samples this is approximate, but avoids needing a t-distribution table
        let pvalue = 2.0 * normal_cdf(-t_stat.abs());

        // Log2 fold change (add pseudocount to avoid log(0))
        let pseudo = 1.0;
        let log2fc = ((mean_b + pseudo) / (mean_a + pseudo)).log2();

        let gene_name = gene_names
            .and_then(|names| names.get(i))
            .cloned()
            .unwrap_or_else(|| format!("gene_{i}"));

        results.push(DiffExprResult {
            gene: gene_name,
            log2fc,
            pvalue,
            padj: 0.0, // filled in below
            mean_a,
            mean_b,
        });
    }

    // Benjamini-Hochberg correction
    bh_adjust(&mut results);

    results
}

/// Benjamini-Hochberg p-value adjustment.
fn bh_adjust(results: &mut [DiffExprResult]) {
    let n = results.len();
    if n == 0 {
        return;
    }

    // Sort by p-value
    let mut indices: Vec<usize> = (0..n).collect();
    indices.sort_by(|&a, &b| {
        results[a]
            .pvalue
            .partial_cmp(&results[b].pvalue)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // BH adjustment: padj[i] = min(p[i] * n / rank, 1.0), enforcing monotonicity
    let mut padj = vec![0.0f64; n];
    let mut prev = 1.0f64;

    for rank in (0..n).rev() {
        let idx = indices[rank];
        let adjusted = (results[idx].pvalue * n as f64 / (rank + 1) as f64).min(1.0);
        padj[idx] = adjusted.min(prev);
        prev = padj[idx];
    }

    for (i, result) in results.iter_mut().enumerate() {
        result.padj = padj[i];
    }
}

/// Approximate normal CDF using Abramowitz and Stegun approximation.
fn normal_cdf(x: f64) -> f64 {
    if x < -8.0 {
        return 0.0;
    }
    if x > 8.0 {
        return 1.0;
    }

    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let d = 0.3989422804014327; // 1/sqrt(2*pi)
    let p = d * (-x * x / 2.0).exp();
    let poly = t * (0.319381530 + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));

    if x >= 0.0 {
        1.0 - p * poly
    } else {
        p * poly
    }
}
