//! Pure statistical functions. No framework dependencies.

use serde::Serialize;

// ── Result Types ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct TTestResult {
    pub statistic: f64,
    pub p_value: f64,
    pub df: f64,
    pub mean_a: f64,
    pub mean_b: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MannWhitneyResult {
    pub statistic: f64,
    pub p_value: f64,
    pub n_a: usize,
    pub n_b: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct AnovaResult {
    pub f_statistic: f64,
    pub p_value: f64,
    pub df_between: f64,
    pub df_within: f64,
    pub group_means: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorrelationResult {
    pub correlation: f64,
    pub p_value: f64,
    pub n: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChiSquareResult {
    pub chi_square: f64,
    pub p_value: f64,
    pub df: usize,
    pub expected: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FishersResult {
    pub odds_ratio: f64,
    pub p_value: f64,
    pub confidence_interval: (f64, f64),
}

#[derive(Debug, Clone, Serialize)]
pub struct DescriptiveStats {
    pub count: usize,
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub q25: f64,
    pub q75: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinearRegressionResult {
    pub slope: f64,
    pub intercept: f64,
    pub r_squared: f64,
    pub p_value: f64,
    pub std_error: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogisticRegressionResult {
    pub coefficients: Vec<f64>,
    pub p_values: Vec<f64>,
    pub log_likelihood: f64,
    pub aic: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct WilcoxonResult {
    pub statistic: f64,
    pub p_value: f64,
    pub n_pairs: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct KruskalWallisResult {
    pub h_statistic: f64,
    pub p_value: f64,
    pub df: usize,
    pub group_ranks: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PcaResult {
    pub explained_variance: Vec<f64>,
    pub explained_variance_ratio: Vec<f64>,
    pub components: Vec<Vec<f64>>,
    pub transformed_data: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FactorAnalysisResult {
    pub loadings: Vec<Vec<f64>>,
    pub communalities: Vec<f64>,
    pub uniqueness: Vec<f64>,
    pub factor_scores: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArimaResult {
    pub forecasts: Vec<f64>,
    pub residuals: Vec<f64>,
    pub aic: f64,
    pub bic: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForecastResult {
    pub forecasts: Vec<f64>,
    pub confidence_intervals: Vec<(f64, f64)>,
    pub method: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MultipleTestingResult {
    pub adjusted_p_values: Vec<f64>,
    pub rejected: Vec<usize>,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct KsTestResult {
    pub statistic: f64,
    pub p_value: f64,
    pub n1: usize,
    pub n2: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct KaplanMeierResult {
    pub times: Vec<f64>,
    pub survival: Vec<f64>,
    pub ci_lower: Vec<f64>,
    pub ci_upper: Vec<f64>,
    pub at_risk: Vec<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CoxPhResult {
    pub coefficients: Vec<f64>,
    pub hazard_ratios: Vec<f64>,
    pub p_values: Vec<f64>,
    pub concordance: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MultipleRegressionResult {
    pub coefficients: Vec<f64>,
    pub std_errors: Vec<f64>,
    pub t_values: Vec<f64>,
    pub p_values: Vec<f64>,
    pub r_squared: f64,
    pub adj_r_squared: f64,
    pub f_statistic: f64,
    pub f_p_value: f64,
}

// ── Helper Functions ─────────────────────────────────────────────────────────

pub fn mean(data: &[f64]) -> f64 {
    data.iter().sum::<f64>() / data.len() as f64
}

pub fn variance(data: &[f64], mean_val: f64) -> f64 {
    data.iter().map(|x| (x - mean_val).powi(2)).sum::<f64>() / (data.len() - 1) as f64
}

pub fn pearson_correlation(x: &[f64], y: &[f64]) -> f64 {
    let n = x.len() as f64;
    let sum_x: f64 = x.iter().sum();
    let sum_y: f64 = y.iter().sum();
    let sum_xy: f64 = x.iter().zip(y.iter()).map(|(a, b)| a * b).sum();
    let sum_x2: f64 = x.iter().map(|a| a * a).sum();
    let sum_y2: f64 = y.iter().map(|b| b * b).sum();

    let numerator = n * sum_xy - sum_x * sum_y;
    let denominator = ((n * sum_x2 - sum_x * sum_x) * (n * sum_y2 - sum_y * sum_y)).sqrt();

    if denominator == 0.0 {
        0.0
    } else {
        numerator / denominator
    }
}

pub fn spearman_correlation(x: &[f64], y: &[f64]) -> f64 {
    let x_ranks = rank_transform(x);
    let y_ranks = rank_transform(y);
    pearson_correlation(&x_ranks, &y_ranks)
}

pub fn rank_transform(data: &[f64]) -> Vec<f64> {
    let mut indexed: Vec<(f64, usize)> = data.iter().enumerate().map(|(i, &v)| (v, i)).collect();
    indexed.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut ranks = vec![0.0; data.len()];
    let mut i = 0;
    while i < indexed.len() {
        let mut j = i;
        while j < indexed.len() && (indexed[j].0 - indexed[i].0).abs() < 1e-10 {
            j += 1;
        }
        let avg_rank = (i + j) as f64 / 2.0 + 1.0;
        for k in i..j {
            ranks[indexed[k].1] = avg_rank;
        }
        i = j;
    }
    ranks
}

// ── Distribution Functions ───────────────────────────────────────────────────

pub fn normal_cdf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.2316419 * x.abs());
    let y = t * (0.319381530 - t * (0.356563782 - t * (1.781477937 - t * 1.330274429)));
    if x > 0.0 {
        1.0 - y * (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
    } else {
        y * (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
    }
}

pub fn normal_quantile(p: f64) -> f64 {
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    let q = p - 0.5;
    if q.abs() <= 0.42 {
        q * (2.50662823884 - 18.61500062529 * q * q + 41.39119773534 * q * q * q * q
            - 25.44106049637 * q * q * q * q * q * q)
    } else {
        let sign = if q > 0.0 { 1.0 } else { -1.0 };
        let q_abs = q.abs();
        let ln_val = (1.0 - q_abs).ln();
        let num = 0.886226899 - ln_val * (0.886226899 - ln_val * (0.374901 * ln_val + 0.488 * ln_val * ln_val));
        sign * num
    }
}

pub fn students_t_cdf(t: f64, df: f64) -> f64 {
    if df > 100.0 {
        return normal_cdf(t);
    }
    let x = df / (df + t * t);
    let p = 0.5 * regularized_incomplete_beta(x, df / 2.0, 0.5);
    if t >= 0.0 { 1.0 - p } else { p }
}

pub fn f_distribution_cdf(f: f64, df1: f64, df2: f64) -> f64 {
    if f <= 0.0 {
        return 0.0;
    }
    let x = df1 * f / (df1 * f + df2);
    regularized_incomplete_beta(x, df1 / 2.0, df2 / 2.0)
}

pub fn chi_square_cdf(chi2: f64, df: usize) -> f64 {
    if chi2 <= 0.0 {
        return 0.0;
    }
    gamma_cdf(chi2, df as f64 / 2.0, 2.0)
}

pub fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    if x >= 1.0 { return 1.0; }

    if x < (a + 1.0) / (a + b + 2.0) {
        let mut result = 0.0;
        let mut term = 1.0;
        let mut k = 0;
        while k < 100 {
            if k == 0 {
                term = 1.0;
            } else {
                term *= (a + b + k as f64 - 1.0) * x / (a + k as f64);
            }
            result += term;
            if term.abs() < 1e-12 { break; }
            k += 1;
        }
        result *= x.powf(a) / a;
        result.min(1.0)
    } else {
        1.0 - regularized_incomplete_beta(1.0 - x, b, a)
    }
}

fn gamma_cdf(x: f64, shape: f64, scale: f64) -> f64 {
    if x <= 0.0 { return 0.0; }
    let scaled_x = x / scale;
    if (shape - shape.round()).abs() < 1e-10 && shape <= 20.0 {
        poisson_cdf(scaled_x.floor() as i32, shape)
    } else {
        let mut result = 0.0;
        let mut term = 1.0 / shape;
        let mut k = 1;
        while k < 50 {
            result += term;
            term *= scaled_x / (shape + k as f64);
            if term.abs() < 1e-12 { break; }
            k += 1;
        }
        result *= scaled_x.powf(shape) * (-scaled_x).exp();
        result.min(1.0)
    }
}

fn poisson_cdf(k: i32, lambda: f64) -> f64 {
    if k < 0 { return 0.0; }
    let mut cdf = 0.0;
    let mut pmf = (-lambda).exp();
    for i in 0..=k {
        if i > 0 { pmf *= lambda / i as f64; }
        cdf += pmf;
    }
    cdf.min(1.0)
}

// ── Fisher's Exact Test Helpers ──────────────────────────────────────────────

pub fn hypergeometric_prob(k: u64, n1: u64, n2: u64, n: u64) -> f64 {
    if k > n1 || k > n2 || n2 - k > n - n1 {
        return 0.0;
    }
    let log_prob = log_binomial(n1, k) + log_binomial(n - n1, n2 - k) - log_binomial(n, n2);
    log_prob.exp()
}

pub fn log_binomial(n: u64, k: u64) -> f64 {
    if k > n || k == 0 || n == 0 {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut result = 0.0;
    for i in 1..=k {
        result += ((n - k + i) as f64).ln() - (i as f64).ln();
    }
    result
}

fn fishers_exact_p_value(a: u64, b: u64, c: u64, d: u64) -> f64 {
    let n = a + b + c + d;
    let row1_total = a + b;
    let col1_total = a + c;
    let observed_prob = hypergeometric_prob(a, row1_total, col1_total, n);

    let current_a = if row1_total < col1_total { 0 } else { row1_total + col1_total - n };
    let max_a = row1_total.min(col1_total);

    let mut p_value = 0.0;
    for i in current_a..=max_a {
        let prob = hypergeometric_prob(i, row1_total, col1_total, n);
        if prob <= observed_prob {
            p_value += prob;
        }
    }
    p_value.min(1.0)
}

fn odds_ratio_confidence_interval(a: u64, b: u64, c: u64, d: u64, confidence: f64) -> (f64, f64) {
    if a == 0 || b == 0 || c == 0 || d == 0 {
        return (0.0, f64::INFINITY);
    }
    let or = (a as f64 * d as f64) / (b as f64 * c as f64);
    let log_or = or.ln();
    let se_log_or =
        ((1.0 / a as f64) + (1.0 / b as f64) + (1.0 / c as f64) + (1.0 / d as f64)).sqrt();
    let z = normal_quantile((1.0 + confidence) / 2.0);
    let margin = z * se_log_or;
    ((log_or - margin).exp(), (log_or + margin).exp())
}

// ── Statistical Tests ────────────────────────────────────────────────────────

pub fn t_test(group_a: &[f64], group_b: &[f64], alternative: &str) -> Result<TTestResult, String> {
    if group_a.len() < 2 || group_b.len() < 2 {
        return Err("groups must have at least 2 observations".into());
    }
    let mean_a = mean(group_a);
    let mean_b = mean(group_b);
    let var_a = variance(group_a, mean_a);
    let var_b = variance(group_b, mean_b);
    let n_a = group_a.len() as f64;
    let n_b = group_b.len() as f64;

    let pooled_var = ((n_a - 1.0) * var_a + (n_b - 1.0) * var_b) / (n_a + n_b - 2.0);
    let se = (pooled_var / n_a + pooled_var / n_b).sqrt();
    let t_stat = (mean_a - mean_b) / se;
    let df = n_a + n_b - 2.0;

    let p_value = match alternative {
        "two_sided" => 2.0 * (1.0 - students_t_cdf(t_stat.abs(), df)),
        "less" => students_t_cdf(t_stat, df),
        "greater" => 1.0 - students_t_cdf(t_stat, df),
        _ => return Err("invalid alternative hypothesis".into()),
    };
    Ok(TTestResult { statistic: t_stat, p_value, df, mean_a, mean_b })
}

pub fn mann_whitney_test(group_a: &[f64], group_b: &[f64], alternative: &str) -> Result<MannWhitneyResult, String> {
    let n_a = group_a.len();
    let n_b = group_b.len();
    if n_a < 1 || n_b < 1 {
        return Err("groups must have at least 1 observation".into());
    }

    let mut combined: Vec<(f64, usize)> = group_a.iter().map(|&v| (v, 0)).collect();
    combined.extend(group_b.iter().map(|&v| (v, 1)));
    combined.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut ranks = vec![0.0; combined.len()];
    let mut i = 0;
    while i < combined.len() {
        let mut j = i;
        while j < combined.len() && (combined[j].0 - combined[i].0).abs() < 1e-10 { j += 1; }
        let avg_rank = (i + j - 1) as f64 / 2.0 + 1.0;
        for rank in &mut ranks[i..j] { *rank = avg_rank; }
        i = j;
    }

    let mut u_a = 0.0;
    for (i, &(_, group)) in combined.iter().enumerate() {
        if group == 0 { u_a += ranks[i]; }
    }

    let u = u_a - (n_a * (n_a + 1)) as f64 / 2.0;
    let u_stat = u.min((n_a * n_b) as f64 - u);
    let mean_u = (n_a * n_b) as f64 / 2.0;
    let var_u = (n_a * n_b * (n_a + n_b + 1)) as f64 / 12.0;
    let z = (u_stat - mean_u) / var_u.sqrt();

    let p_value = match alternative {
        "two_sided" => 2.0 * (1.0 - normal_cdf(z.abs())),
        "less" => normal_cdf(z),
        "greater" => 1.0 - normal_cdf(z),
        _ => return Err("invalid alternative hypothesis".into()),
    };
    Ok(MannWhitneyResult { statistic: u_stat, p_value, n_a, n_b })
}

pub fn anova(groups: &[Vec<f64>]) -> Result<AnovaResult, String> {
    if groups.len() < 2 {
        return Err("ANOVA requires at least 2 groups".into());
    }
    let mut all_values = Vec::new();
    let mut group_means = Vec::new();
    let mut group_sizes = Vec::new();
    for group in groups {
        if group.is_empty() { return Err("groups cannot be empty".into()); }
        group_means.push(mean(group));
        group_sizes.push(group.len());
        all_values.extend_from_slice(group);
    }
    let grand_mean = mean(&all_values);
    let total_n = all_values.len() as f64;

    let ssb: f64 = group_sizes.iter().zip(&group_means)
        .map(|(&size, &gm)| size as f64 * (gm - grand_mean).powi(2)).sum();
    let ssw: f64 = groups.iter().zip(&group_means)
        .map(|(group, &gm)| group.iter().map(|&x| (x - gm).powi(2)).sum::<f64>()).sum();

    let df_between = groups.len() as f64 - 1.0;
    let df_within = total_n - groups.len() as f64;
    let msb = ssb / df_between;
    let msw = ssw / df_within;
    let f_stat = if msw > 0.0 { msb / msw } else { f64::INFINITY };
    let p_value = 1.0 - f_distribution_cdf(f_stat, df_between, df_within);

    Ok(AnovaResult { f_statistic: f_stat, p_value, df_between, df_within, group_means })
}

pub fn correlation(x: &[f64], y: &[f64], method: &str) -> Result<CorrelationResult, String> {
    if x.len() != y.len() || x.len() < 2 {
        return Err("x and y must have same length and at least 2 observations".into());
    }
    let n = x.len();
    let corr = match method {
        "pearson" => pearson_correlation(x, y),
        "spearman" => spearman_correlation(x, y),
        _ => return Err("invalid correlation method".into()),
    };
    let t_stat = corr * ((n as f64 - 2.0) / (1.0 - corr.powi(2))).sqrt();
    let p_value = 2.0 * (1.0 - students_t_cdf(t_stat.abs(), n as f64 - 2.0));
    Ok(CorrelationResult { correlation: corr, p_value, n })
}

pub fn chi_square_test(table: &[Vec<f64>]) -> Result<ChiSquareResult, String> {
    if table.len() != 2 || table[0].len() != 2 {
        return Err("contingency table must be 2x2".into());
    }
    let a = table[0][0]; let b = table[0][1];
    let c = table[1][0]; let d = table[1][1];
    let row_totals = [a + b, c + d];
    let col_totals = [a + c, b + d];
    let total = row_totals[0] + row_totals[1];

    let mut expected = vec![vec![0.0; 2]; 2];
    for i in 0..2 {
        for j in 0..2 {
            expected[i][j] = row_totals[i] * col_totals[j] / total;
        }
    }
    let chi_sq = (a - expected[0][0]).powi(2) / expected[0][0]
        + (b - expected[0][1]).powi(2) / expected[0][1]
        + (c - expected[1][0]).powi(2) / expected[1][0]
        + (d - expected[1][1]).powi(2) / expected[1][1];
    let df = 1;
    let p_value = 1.0 - chi_square_cdf(chi_sq, df);
    Ok(ChiSquareResult { chi_square: chi_sq, p_value, df, expected })
}

pub fn fishers_exact_test(table: &[Vec<f64>]) -> Result<FishersResult, String> {
    if table.len() != 2 || table[0].len() != 2 {
        return Err("contingency table must be 2x2".into());
    }
    let a = table[0][0] as u64; let b = table[0][1] as u64;
    let c = table[1][0] as u64; let d = table[1][1] as u64;
    let odds_ratio = if b == 0 || c == 0 {
        if a == 0 || d == 0 { f64::NAN } else { f64::INFINITY }
    } else {
        (a as f64 * d as f64) / (b as f64 * c as f64)
    };
    let p_value = fishers_exact_p_value(a, b, c, d);
    let confidence_interval = odds_ratio_confidence_interval(a, b, c, d, 0.95);
    Ok(FishersResult { odds_ratio, p_value, confidence_interval })
}

pub fn descriptive_statistics(data: &[f64]) -> DescriptiveStats {
    if data.is_empty() {
        return DescriptiveStats { count: 0, mean: 0.0, median: 0.0, std_dev: 0.0, min: 0.0, max: 0.0, q25: 0.0, q75: 0.0 };
    }
    let count = data.len();
    let mean_val = mean(data);
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if count.is_multiple_of(2) { (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0 } else { sorted[count / 2] };
    let q25 = sorted[(count as f64 * 0.25) as usize];
    let q75 = sorted[(count as f64 * 0.75) as usize];
    let std_dev = variance(data, mean_val).sqrt();
    DescriptiveStats { count, mean: mean_val, median, std_dev, min: *sorted.first().unwrap(), max: *sorted.last().unwrap(), q25, q75 }
}

// ── Regression ───────────────────────────────────────────────────────────────

pub fn linear_regression(x: &[f64], y: &[f64]) -> Result<LinearRegressionResult, String> {
    if x.len() != y.len() || x.len() < 2 {
        return Err("insufficient data for regression".into());
    }
    let n = x.len() as f64;
    let x_mean = mean(x);
    let y_mean = mean(y);
    let numerator: f64 = x.iter().zip(y.iter()).map(|(&xi, &yi)| (xi - x_mean) * (yi - y_mean)).sum();
    let denominator: f64 = x.iter().map(|&xi| (xi - x_mean).powi(2)).sum();
    if denominator == 0.0 { return Err("no variance in x values".into()); }

    let slope = numerator / denominator;
    let intercept = y_mean - slope * x_mean;
    let y_pred: Vec<f64> = x.iter().map(|&xi| slope * xi + intercept).collect();
    let ss_res: f64 = y.iter().zip(&y_pred).map(|(&yi, &yp)| (yi - yp).powi(2)).sum();
    let ss_tot: f64 = y.iter().map(|&yi| (yi - y_mean).powi(2)).sum();
    let r_squared = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 };
    let mse = ss_res / (n - 2.0);
    let se_slope = (mse / denominator).sqrt();
    let t_stat = slope / se_slope;
    let p_value = 2.0 * (1.0 - students_t_cdf(t_stat.abs(), n - 2.0));
    Ok(LinearRegressionResult { slope, intercept, r_squared, p_value, std_error: se_slope })
}

pub fn logistic_regression(x: &[f64], y: &[f64]) -> Result<LogisticRegressionResult, String> {
    if x.len() != y.len() || x.len() < 2 {
        return Err("insufficient data for logistic regression".into());
    }
    for &yi in y {
        if yi != 0.0 && yi != 1.0 {
            return Err("y values must be binary (0 or 1) for logistic regression".into());
        }
    }

    let mut beta = vec![0.0; 2];
    let max_iter = 100;
    let tolerance = 1e-6;

    for _ in 0..max_iter {
        let p: Vec<f64> = (0..x.len()).map(|i| 1.0 / (1.0 + (-(beta[0] + beta[1] * x[i])).exp())).collect();
        let mut gradient = [0.0; 2];
        for (i, &xi) in x.iter().enumerate() {
            let error = y[i] - p[i];
            gradient[0] += error;
            gradient[1] += error * xi;
        }
        let mut hessian = vec![vec![0.0; 2]; 2];
        for (i, &xi) in x.iter().enumerate() {
            let w = p[i] * (1.0 - p[i]);
            hessian[0][0] += w;
            hessian[0][1] += w * xi;
            hessian[1][0] += w * xi;
            hessian[1][1] += w * xi * xi;
        }
        let delta = [gradient[0] / hessian[0][0].max(1.0), gradient[1] / hessian[1][1].max(1.0)];
        beta[0] += delta[0];
        beta[1] += delta[1];
        if delta[0].abs() < tolerance && delta[1].abs() < tolerance { break; }
    }

    let mut log_likelihood = 0.0;
    for (i, &xi) in x.iter().enumerate() {
        let p_i = 1.0 / (1.0 + (-(beta[0] + beta[1] * xi)).exp());
        log_likelihood += y[i] * p_i.ln() + (1.0 - y[i]) * (1.0 - p_i).ln();
    }
    let aic = 2.0 * beta.len() as f64 - 2.0 * log_likelihood;

    let mut hessian_final = vec![vec![0.0; 2]; 2];
    for &xi in x {
        let p_i = 1.0 / (1.0 + (-(beta[0] + beta[1] * xi)).exp());
        let w = p_i * (1.0 - p_i);
        hessian_final[0][0] += w;
        hessian_final[0][1] += w * xi;
        hessian_final[1][0] += w * xi;
        hessian_final[1][1] += w * xi * xi;
    }
    let det = hessian_final[0][0] * hessian_final[1][1] - hessian_final[0][1] * hessian_final[1][0];
    let p_values = if det.abs() > 1e-12 {
        let se0 = (hessian_final[1][1] / det).abs().sqrt();
        let se1 = (hessian_final[0][0] / det).abs().sqrt();
        let z0 = if se0 > 0.0 { beta[0] / se0 } else { 0.0 };
        let z1 = if se1 > 0.0 { beta[1] / se1 } else { 0.0 };
        vec![2.0 * (1.0 - normal_cdf(z0.abs())), 2.0 * (1.0 - normal_cdf(z1.abs()))]
    } else {
        vec![f64::NAN; 2]
    };
    Ok(LogisticRegressionResult { coefficients: beta, p_values, log_likelihood, aic })
}

// ── Non-parametric Tests ─────────────────────────────────────────────────────

pub fn wilcoxon_signed_rank_test(group_a: &[f64], group_b: &[f64], alternative: &str) -> Result<WilcoxonResult, String> {
    if group_a.len() != group_b.len() {
        return Err("paired test requires equal group sizes".into());
    }
    let n = group_a.len();
    if n < 1 { return Err("insufficient data for paired test".into()); }

    let differences: Vec<f64> = group_a.iter().zip(group_b.iter()).map(|(&a, &b)| a - b).filter(|&d| d != 0.0).collect();
    if differences.is_empty() {
        return Ok(WilcoxonResult { statistic: 0.0, p_value: 1.0, n_pairs: n });
    }

    let mut abs_diffs: Vec<(f64, usize)> = differences.iter().enumerate().map(|(i, &d)| (d.abs(), i)).collect();
    abs_diffs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut ranks = vec![0.0; differences.len()];
    let mut i = 0;
    while i < abs_diffs.len() {
        let mut j = i;
        while j < abs_diffs.len() && (abs_diffs[j].0 - abs_diffs[i].0).abs() < 1e-10 { j += 1; }
        let avg_rank = (i + j) as f64 / 2.0 + 1.0;
        for k in i..j { ranks[abs_diffs[k].1] = avg_rank; }
        i = j;
    }

    let signed_ranks: Vec<f64> = differences.iter().zip(&ranks).map(|(&d, &r)| if d > 0.0 { r } else { -r }).collect();
    let w_positive: f64 = signed_ranks.iter().filter(|&&r| r > 0.0).sum();
    let w_negative: f64 = signed_ranks.iter().filter(|&&r| r < 0.0).map(|r| r.abs()).sum();
    let w_stat = w_positive.min(w_negative);
    let nd = differences.len() as f64;
    let mean_w = nd * (nd + 1.0) / 4.0;
    let var_w = nd * (nd + 1.0) * (2.0 * nd + 1.0) / 24.0;
    let z = (w_stat - mean_w) / var_w.sqrt();

    let p_value = match alternative {
        "two_sided" => 2.0 * (1.0 - normal_cdf(z.abs())),
        "less" => normal_cdf(z),
        "greater" => 1.0 - normal_cdf(z),
        _ => return Err("invalid alternative hypothesis".into()),
    };
    Ok(WilcoxonResult { statistic: w_stat, p_value, n_pairs: n })
}

pub fn kruskal_wallis_test(groups: &[Vec<f64>]) -> Result<KruskalWallisResult, String> {
    if groups.len() < 2 { return Err("Kruskal-Wallis requires at least 2 groups".into()); }
    let mut all_values: Vec<(f64, usize)> = Vec::new();
    for (gi, group) in groups.iter().enumerate() {
        for &v in group { all_values.push((v, gi)); }
    }
    if all_values.is_empty() { return Err("no data in groups".into()); }

    all_values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut ranks = vec![0.0; all_values.len()];
    let mut i = 0;
    while i < all_values.len() {
        let mut j = i;
        while j < all_values.len() && (all_values[j].0 - all_values[i].0).abs() < 1e-10 { j += 1; }
        let avg_rank = (i + j) as f64 / 2.0 + 1.0;
        for rank in &mut ranks[i..j] { *rank = avg_rank; }
        i = j;
    }

    let mut group_ranks = vec![0.0; groups.len()];
    let mut group_sizes = vec![0usize; groups.len()];
    for ((_, gi), &rank) in all_values.iter().zip(&ranks) {
        group_ranks[*gi] += rank;
        group_sizes[*gi] += 1;
    }

    let n = all_values.len() as f64;
    let mean_rank = (n + 1.0) / 2.0;
    let h_stat = (12.0 / (n * (n + 1.0)))
        * group_ranks.iter().zip(&group_sizes)
            .map(|(&r, &size)| (r - size as f64 * mean_rank).powi(2) / size as f64).sum::<f64>();
    let df = groups.len() - 1;
    let p_value = 1.0 - chi_square_cdf(h_stat, df);
    Ok(KruskalWallisResult { h_statistic: h_stat, p_value, df, group_ranks })
}

// ── Multivariate Analysis ────────────────────────────────────────────────────

pub fn principal_component_analysis(matrix: &[Vec<f64>], n_components: usize) -> Result<PcaResult, String> {
    if matrix.is_empty() || matrix[0].is_empty() {
        return Err("empty data matrix".into());
    }
    let n_samples = matrix.len();
    let n_features = matrix[0].len();
    let n_comp = n_components.min(n_features);

    let mut col_means = vec![0.0; n_features];
    for row in matrix {
        for (j, &v) in row.iter().enumerate() { col_means[j] += v; }
    }
    for m in &mut col_means { *m /= n_samples as f64; }

    let centered: Vec<Vec<f64>> = matrix.iter()
        .map(|row| row.iter().enumerate().map(|(j, &v)| v - col_means[j]).collect()).collect();

    let mut cov = vec![vec![0.0; n_features]; n_features];
    for row in &centered {
        for i in 0..n_features {
            for j in i..n_features { cov[i][j] += row[i] * row[j]; }
        }
    }
    let denom = (n_samples - 1).max(1) as f64;
    #[allow(clippy::needless_range_loop)]
    for i in 0..n_features {
        for j in i..n_features { cov[i][j] /= denom; cov[j][i] = cov[i][j]; }
    }

    let mut eigenvectors: Vec<Vec<f64>> = Vec::new();
    let mut eigenvalues: Vec<f64> = Vec::new();
    let mut deflated_cov = cov;

    for _ in 0..n_comp {
        let mut v = vec![1.0 / (n_features as f64).sqrt(); n_features];
        let mut eigenvalue = 0.0;
        for _ in 0..200 {
            let mut w = vec![0.0; n_features];
            for i in 0..n_features {
                for j in 0..n_features { w[i] += deflated_cov[i][j] * v[j]; }
            }
            eigenvalue = 0.0;
            for i in 0..n_features { eigenvalue += v[i] * w[i]; }
            let norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-15 { break; }
            let new_v: Vec<f64> = w.iter().map(|x| x / norm).collect();
            let diff: f64 = v.iter().zip(&new_v).map(|(a, b)| (a - b).powi(2)).sum();
            v = new_v;
            if diff < 1e-12 { break; }
        }
        eigenvalues.push(eigenvalue.max(0.0));
        eigenvectors.push(v.clone());
        for i in 0..n_features {
            for j in 0..n_features { deflated_cov[i][j] -= eigenvalue * v[i] * v[j]; }
        }
    }

    let total_var: f64 = eigenvalues.iter().sum();
    let explained_variance_ratio = if total_var > 0.0 {
        eigenvalues.iter().map(|&e| e / total_var).collect()
    } else { vec![0.0; n_comp] };

    let transformed_data: Vec<Vec<f64>> = centered.iter()
        .map(|row| eigenvectors.iter().map(|ev| row.iter().zip(ev).map(|(a, b)| a * b).sum()).collect()).collect();

    Ok(PcaResult { explained_variance: eigenvalues, explained_variance_ratio, components: eigenvectors, transformed_data })
}

pub fn factor_analysis(matrix: &[Vec<f64>], n_factors: usize) -> Result<FactorAnalysisResult, String> {
    if matrix.is_empty() || matrix[0].is_empty() {
        return Err("empty data matrix".into());
    }
    let n_variables = matrix[0].len();
    let pca = principal_component_analysis(matrix, n_factors)?;

    let loadings: Vec<Vec<f64>> = (0..n_variables).map(|var_idx| {
        (0..n_factors.min(pca.components.len())).map(|f| {
            let ev = pca.components.get(f).and_then(|c| c.get(var_idx)).copied().unwrap_or(0.0);
            let eigenval = pca.explained_variance.get(f).copied().unwrap_or(0.0);
            ev * eigenval.sqrt()
        }).collect()
    }).collect();

    let communalities: Vec<f64> = loadings.iter().map(|row| row.iter().map(|l| l * l).sum::<f64>().min(1.0)).collect();
    let uniqueness: Vec<f64> = communalities.iter().map(|c| (1.0 - c).max(0.0)).collect();
    Ok(FactorAnalysisResult { loadings, communalities, uniqueness, factor_scores: pca.transformed_data })
}

// ── Time Series ──────────────────────────────────────────────────────────────

pub fn arima_forecast(time_series: &[f64], p: usize, d: usize, q: usize) -> Result<ArimaResult, String> {
    if time_series.len() < p.max(q) + d + 2 {
        return Err("insufficient data for ARIMA".into());
    }

    let mut diffed = time_series.to_vec();
    for _ in 0..d {
        let prev = diffed.clone();
        diffed = prev.windows(2).map(|w| w[1] - w[0]).collect();
    }

    let n = diffed.len();
    let mean_d = diffed.iter().sum::<f64>() / n as f64;
    let centered: Vec<f64> = diffed.iter().map(|&x| x - mean_d).collect();

    let ar_order = p.max(1);
    let mut ar_coeffs = vec![0.0; ar_order];

    if ar_order > 0 && n > ar_order {
        let var: f64 = centered.iter().map(|x| x * x).sum::<f64>() / n as f64;
        if var > 1e-15 {
            let mut acf = vec![0.0; ar_order + 1];
            for lag in 0..=ar_order {
                let mut s = 0.0;
                for i in lag..n { s += centered[i] * centered[i - lag]; }
                acf[lag] = s / n as f64;
            }
            let mut a = vec![0.0; ar_order];
            let mut e = acf[0];
            for m in 0..ar_order {
                let mut lambda = acf[m + 1];
                for j in 0..m { lambda -= a[j] * acf[m - j]; }
                if e.abs() < 1e-15 { break; }
                let k = lambda / e;
                let mut new_a = a.clone();
                new_a[m] = k;
                for j in 0..m { new_a[j] = a[j] - k * a[m - 1 - j]; }
                a = new_a;
                e *= 1.0 - k * k;
            }
            ar_coeffs = a;
        }
    }

    let mut residuals = vec![0.0; n];
    for i in ar_order..n {
        let mut pred = mean_d;
        for j in 0..ar_order { pred += ar_coeffs[j] * centered[i - 1 - j]; }
        residuals[i] = diffed[i] - pred;
    }

    let ma_order = q.min(n.saturating_sub(ar_order + 1));
    let mut ma_coeffs = vec![0.0; ma_order];
    if ma_order > 0 {
        let res_var: f64 = residuals[ar_order..].iter().map(|r| r * r).sum::<f64>() / (n - ar_order).max(1) as f64;
        if res_var > 1e-15 {
            for lag in 1..=ma_order {
                let mut s = 0.0;
                for i in (ar_order + lag)..n { s += residuals[i] * residuals[i - lag]; }
                ma_coeffs[lag - 1] = s / ((n - ar_order - lag).max(1) as f64 * res_var);
            }
        }
    }

    let n_forecast = 5;
    let mut extended = centered.clone();
    let mut ext_resid = residuals.clone();
    for _ in 0..n_forecast {
        let mut pred = mean_d;
        let m = extended.len();
        for j in 0..ar_order.min(m) { pred += ar_coeffs[j] * extended[m - 1 - j]; }
        for j in 0..ma_order.min(ext_resid.len()) { pred += ma_coeffs[j] * ext_resid[ext_resid.len() - 1 - j]; }
        extended.push(pred);
        ext_resid.push(0.0);
    }

    let mut forecasts: Vec<f64> = extended[n..].iter().map(|&x| x + mean_d).collect();
    for _ in 0..d {
        let mut prev = *time_series.last().unwrap();
        for f in &mut forecasts { prev += *f; *f = prev; }
    }

    let ss_res: f64 = residuals[ar_order..].iter().map(|r| r * r).sum();
    let n_eff = (n - ar_order) as f64;
    let k_params = (p + q + 1) as f64;
    let sigma2 = ss_res / n_eff.max(1.0);
    let ll = -0.5 * n_eff * (2.0 * std::f64::consts::PI * sigma2).ln() - ss_res / (2.0 * sigma2.max(1e-15));
    let aic = -2.0 * ll + 2.0 * k_params;
    let bic = -2.0 * ll + k_params * n_eff.ln();

    Ok(ArimaResult { forecasts, residuals, aic, bic })
}

pub fn time_series_forecast(time_series: &[f64], steps: usize) -> Result<ForecastResult, String> {
    if time_series.is_empty() { return Err("empty time series".into()); }
    let alpha = 0.3;
    let mut level = time_series[0];
    for &value in &time_series[1..] { level = alpha * value + (1.0 - alpha) * level; }
    let forecasts = vec![level; steps];
    let confidence_intervals = forecasts.iter().map(|&f| (f * 0.9, f * 1.1)).collect();
    Ok(ForecastResult { forecasts, confidence_intervals, method: "Exponential Smoothing".to_string() })
}

// ── Multiple Testing Corrections ─────────────────────────────────────────────

pub fn benjamini_hochberg_correction(p_values: &[f64], alpha: f64) -> MultipleTestingResult {
    let mut indexed_p: Vec<(f64, usize)> = p_values.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    indexed_p.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let m = p_values.len() as f64;
    let mut adjusted = vec![0.0; p_values.len()];
    let mut rejected = Vec::new();
    for (rank, (p_value, original_idx)) in indexed_p.iter().enumerate() {
        let rank_f = (rank + 1) as f64;
        let adjusted_p = p_value * m / rank_f;
        adjusted[*original_idx] = adjusted_p.min(1.0);
        if adjusted_p <= alpha { rejected.push(*original_idx); }
    }
    MultipleTestingResult { adjusted_p_values: adjusted, rejected, threshold: alpha }
}

pub fn bonferroni_correction(p_values: &[f64]) -> MultipleTestingResult {
    let m = p_values.len() as f64;
    let adjusted_p_values: Vec<f64> = p_values.iter().map(|&p| (p * m).min(1.0)).collect();
    let rejected: Vec<usize> = adjusted_p_values.iter().enumerate().filter(|(_, &p)| p <= 0.05).map(|(i, _)| i).collect();
    MultipleTestingResult { adjusted_p_values, rejected, threshold: 0.05 }
}

pub fn holm_bonferroni_correction(p_values: &[f64]) -> MultipleTestingResult {
    let mut indexed_p: Vec<(f64, usize)> = p_values.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    indexed_p.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let m = p_values.len() as f64;
    let mut adjusted = vec![0.0; p_values.len()];
    let mut rejected = Vec::new();
    for (rank, (p_value, original_idx)) in indexed_p.iter().enumerate() {
        let rank_f = (rank + 1) as f64;
        let adjusted_p = p_value * (m - rank_f + 1.0);
        adjusted[*original_idx] = adjusted_p.min(1.0);
        if adjusted_p <= 0.05 { rejected.push(*original_idx); }
    }
    MultipleTestingResult { adjusted_p_values: adjusted, rejected, threshold: 0.05 }
}

pub fn kendall_tau(x: &[f64], y: &[f64]) -> Result<CorrelationResult, String> {
    if x.len() != y.len() || x.len() < 2 {
        return Err("x and y must have same length and at least 2 observations".into());
    }
    let n = x.len();
    let mut concordant = 0i64;
    let mut discordant = 0i64;
    for i in 0..n {
        for j in (i + 1)..n {
            let x_diff = x[i] - x[j];
            let y_diff = y[i] - y[j];
            let product = x_diff * y_diff;
            if product > 0.0 {
                concordant += 1;
            } else if product < 0.0 {
                discordant += 1;
            }
        }
    }
    let total_pairs = (n * (n - 1)) as f64 / 2.0;
    let tau = (concordant - discordant) as f64 / total_pairs;
    // Normal approximation for p-value
    let var = (2.0 * (2.0 * n as f64 + 5.0)) / (9.0 * n as f64 * (n as f64 - 1.0));
    let z = tau / var.sqrt();
    let p_value = 2.0 * (1.0 - normal_cdf(z.abs()));
    Ok(CorrelationResult { correlation: tau, p_value, n })
}

pub fn kolmogorov_smirnov(sample1: &[f64], sample2: &[f64]) -> Result<KsTestResult, String> {
    if sample1.is_empty() || sample2.is_empty() {
        return Err("samples must be non-empty".into());
    }
    let n1 = sample1.len();
    let n2 = sample2.len();
    let mut sorted1 = sample1.to_vec();
    let mut sorted2 = sample2.to_vec();
    sorted1.sort_by(|a, b| a.partial_cmp(b).unwrap());
    sorted2.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut d_max: f64 = 0.0;
    let mut i = 0usize;
    let mut j = 0usize;
    while i < n1 || j < n2 {
        let v1 = if i < n1 { sorted1[i] } else { f64::INFINITY };
        let v2 = if j < n2 { sorted2[j] } else { f64::INFINITY };
        if v1 <= v2 { i += 1; }
        if v2 <= v1 { j += 1; }
        let f1 = i as f64 / n1 as f64;
        let f2 = j as f64 / n2 as f64;
        let d = (f1 - f2).abs();
        if d > d_max { d_max = d; }
    }

    // Asymptotic p-value approximation
    let en = ((n1 * n2) as f64 / (n1 + n2) as f64).sqrt();
    let lambda = (en + 0.12 + 0.11 / en) * d_max;
    let mut p_value = 0.0;
    for k in 1..=100 {
        let term = (-2.0 * (k as f64 * lambda).powi(2)).exp();
        if k % 2 == 0 { p_value -= term; } else { p_value += term; }
        if term.abs() < 1e-15 { break; }
    }
    let p_value = (2.0 * p_value).clamp(0.0, 1.0);
    Ok(KsTestResult { statistic: d_max, p_value, n1, n2 })
}

pub fn kaplan_meier(times: &[f64], events: &[bool]) -> Result<KaplanMeierResult, String> {
    if times.len() != events.len() || times.is_empty() {
        return Err("times and events must have same non-zero length".into());
    }
    let mut data: Vec<(f64, bool)> = times.iter().zip(events.iter()).map(|(&t, &e)| (t, e)).collect();
    data.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut result_times = Vec::new();
    let mut survival = Vec::new();
    let mut ci_lower = Vec::new();
    let mut ci_upper = Vec::new();
    let mut at_risk_vec = Vec::new();
    let mut n_at_risk = data.len();
    let mut s = 1.0;
    let mut var_sum = 0.0; // Greenwood's formula

    let mut i = 0;
    while i < data.len() {
        let t = data[i].0;
        let mut d = 0usize; // events at this time
        let mut c = 0usize; // censored at this time
        while i < data.len() && (data[i].0 - t).abs() < 1e-10 {
            if data[i].1 { d += 1; } else { c += 1; }
            i += 1;
        }
        if d > 0 {
            let n = n_at_risk as f64;
            let d_f = d as f64;
            s *= 1.0 - d_f / n;
            if n > d_f { var_sum += d_f / (n * (n - d_f)); }
            let se = s * var_sum.sqrt();
            let z = 1.96;
            result_times.push(t);
            survival.push(s);
            ci_lower.push((s - z * se).max(0.0));
            ci_upper.push((s + z * se).min(1.0));
            at_risk_vec.push(n_at_risk);
        }
        n_at_risk -= d + c;
    }

    Ok(KaplanMeierResult {
        times: result_times,
        survival,
        ci_lower,
        ci_upper,
        at_risk: at_risk_vec,
    })
}

pub fn cox_ph(time: &[f64], event: &[bool], covariates: &[Vec<f64>]) -> Result<CoxPhResult, String> {
    if time.len() != event.len() || time.is_empty() {
        return Err("time and event must have same non-zero length".into());
    }
    let n = time.len();
    let p = if covariates.is_empty() { 0 } else { covariates[0].len() };
    if p == 0 { return Err("at least one covariate required".into()); }
    for cov in covariates {
        if cov.len() != p { return Err("all covariate vectors must have same length".into()); }
    }
    if covariates.len() != n {
        return Err("number of covariate rows must match number of observations".into());
    }

    // Newton-Raphson for partial likelihood
    let mut beta = vec![0.0; p];
    let max_iter = 50;
    let tol = 1e-8;

    // Sort by time
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| time[a].partial_cmp(&time[b]).unwrap());

    for _ in 0..max_iter {
        let mut gradient = vec![0.0; p];
        let mut hessian = vec![vec![0.0; p]; p];

        for &i in &order {
            if !event[i] { continue; }
            let mut risk_sum = 0.0;
            let mut weighted_x = vec![0.0; p];
            let mut weighted_xx = vec![vec![0.0; p]; p];

            for &j in &order {
                if time[j] >= time[i] {
                    let eta: f64 = (0..p).map(|k| beta[k] * covariates[j][k]).sum();
                    let w = eta.exp();
                    risk_sum += w;
                    for k in 0..p {
                        weighted_x[k] += w * covariates[j][k];
                        for l in 0..p {
                            weighted_xx[k][l] += w * covariates[j][k] * covariates[j][l];
                        }
                    }
                }
            }

            if risk_sum > 0.0 {
                for k in 0..p {
                    gradient[k] += covariates[i][k] - weighted_x[k] / risk_sum;
                    for l in 0..p {
                        hessian[k][l] -= weighted_xx[k][l] / risk_sum
                            - (weighted_x[k] * weighted_x[l]) / (risk_sum * risk_sum);
                    }
                }
            }
        }

        // Simple diagonal update (avoid matrix inversion)
        let mut max_change = 0.0;
        for k in 0..p {
            if hessian[k][k].abs() > 1e-15 {
                let delta = -gradient[k] / hessian[k][k];
                beta[k] += delta;
                if delta.abs() > max_change { max_change = delta.abs(); }
            }
        }
        if max_change < tol { break; }
    }

    let hazard_ratios: Vec<f64> = beta.iter().map(|&b| b.exp()).collect();

    // Standard errors from diagonal of information matrix
    let mut se = vec![0.0; p];
    let mut info = vec![vec![0.0; p]; p];
    for &i in &order {
        if !event[i] { continue; }
        let mut risk_sum = 0.0;
        let mut weighted_x = vec![0.0; p];
        let mut weighted_xx = vec![vec![0.0; p]; p];
        for &j in &order {
            if time[j] >= time[i] {
                let eta: f64 = (0..p).map(|k| beta[k] * covariates[j][k]).sum();
                let w = eta.exp();
                risk_sum += w;
                for k in 0..p {
                    weighted_x[k] += w * covariates[j][k];
                    for l in 0..p {
                        weighted_xx[k][l] += w * covariates[j][k] * covariates[j][l];
                    }
                }
            }
        }
        if risk_sum > 0.0 {
            for k in 0..p {
                for l in 0..p {
                    info[k][l] += weighted_xx[k][l] / risk_sum
                        - (weighted_x[k] * weighted_x[l]) / (risk_sum * risk_sum);
                }
            }
        }
    }
    for k in 0..p {
        se[k] = if info[k][k] > 0.0 { (1.0 / info[k][k]).sqrt() } else { f64::NAN };
    }

    let p_values: Vec<f64> = beta.iter().zip(&se).map(|(&b, &s)| {
        if s.is_nan() || s == 0.0 { f64::NAN } else { 2.0 * (1.0 - normal_cdf((b / s).abs())) }
    }).collect();

    // Concordance index
    let mut concordant = 0u64;
    let mut total = 0u64;
    for i in 0..n {
        if !event[i] { continue; }
        for j in 0..n {
            if time[j] > time[i] {
                let eta_i: f64 = (0..p).map(|k| beta[k] * covariates[i][k]).sum();
                let eta_j: f64 = (0..p).map(|k| beta[k] * covariates[j][k]).sum();
                total += 1;
                if eta_i > eta_j { concordant += 1; }
            }
        }
    }
    let concordance = if total > 0 { concordant as f64 / total as f64 } else { 0.5 };

    Ok(CoxPhResult { coefficients: beta, hazard_ratios, p_values, concordance })
}

pub fn multiple_linear_regression(y: &[f64], x_matrix: &[Vec<f64>]) -> Result<MultipleRegressionResult, String> {
    let n = y.len();
    if n < 3 || x_matrix.len() != n {
        return Err("insufficient data for multiple regression".into());
    }
    let p = x_matrix[0].len(); // number of predictors (without intercept)
    let k = p + 1; // include intercept
    if n <= k {
        return Err("need more observations than predictors".into());
    }

    // Build X matrix with intercept column
    let mut xtx = vec![vec![0.0; k]; k];
    let mut xty = vec![0.0; k];
    for i in 0..n {
        let row: Vec<f64> = std::iter::once(1.0).chain(x_matrix[i].iter().copied()).collect();
        for a in 0..k {
            for b in 0..k {
                xtx[a][b] += row[a] * row[b];
            }
            xty[a] += row[a] * y[i];
        }
    }

    // Solve via Gauss elimination
    let mut aug: Vec<Vec<f64>> = xtx.iter().enumerate()
        .map(|(i, row)| { let mut r = row.clone(); r.push(xty[i]); r })
        .collect();
    for col in 0..k {
        let mut max_row = col;
        for row in (col + 1)..k {
            if aug[row][col].abs() > aug[max_row][col].abs() { max_row = row; }
        }
        aug.swap(col, max_row);
        if aug[col][col].abs() < 1e-15 {
            return Err("singular matrix — collinear predictors".into());
        }
        let pivot = aug[col][col];
        for j in col..=k { aug[col][j] /= pivot; }
        for row in 0..k {
            if row != col {
                let factor = aug[row][col];
                for j in col..=k { aug[row][j] -= factor * aug[col][j]; }
            }
        }
    }
    let coefficients: Vec<f64> = aug.iter().map(|row| row[k]).collect();

    // Residuals and R²
    let y_mean = mean(y);
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for i in 0..n {
        let row: Vec<f64> = std::iter::once(1.0).chain(x_matrix[i].iter().copied()).collect();
        let y_pred: f64 = row.iter().zip(&coefficients).map(|(x, b)| x * b).sum();
        ss_res += (y[i] - y_pred).powi(2);
        ss_tot += (y[i] - y_mean).powi(2);
    }
    let r_squared = if ss_tot > 0.0 { 1.0 - ss_res / ss_tot } else { 0.0 };
    let adj_r_squared = 1.0 - (1.0 - r_squared) * (n as f64 - 1.0) / (n as f64 - k as f64);
    let mse = ss_res / (n - k) as f64;

    // Invert XtX for standard errors (diagonal already computed in aug if we re-do)
    // Use the already-reduced augmented matrix — we need (X'X)^-1
    // Rebuild and invert
    let mut xtx_inv = vec![vec![0.0; k]; k];
    let mut aug2: Vec<Vec<f64>> = Vec::new();
    // Re-build XtX
    let mut xtx2 = vec![vec![0.0; k]; k];
    for i in 0..n {
        let row: Vec<f64> = std::iter::once(1.0).chain(x_matrix[i].iter().copied()).collect();
        for a in 0..k {
            for b in 0..k { xtx2[a][b] += row[a] * row[b]; }
        }
    }
    // Augment with identity
    for i in 0..k {
        let mut row = xtx2[i].clone();
        for j in 0..k { row.push(if i == j { 1.0 } else { 0.0 }); }
        aug2.push(row);
    }
    for col in 0..k {
        let mut max_row = col;
        for row in (col + 1)..k {
            if aug2[row][col].abs() > aug2[max_row][col].abs() { max_row = row; }
        }
        aug2.swap(col, max_row);
        let pivot = aug2[col][col];
        if pivot.abs() < 1e-15 { continue; }
        for j in 0..(2 * k) { aug2[col][j] /= pivot; }
        for row in 0..k {
            if row != col {
                let factor = aug2[row][col];
                for j in 0..(2 * k) { aug2[row][j] -= factor * aug2[col][j]; }
            }
        }
    }
    for i in 0..k {
        for j in 0..k { xtx_inv[i][j] = aug2[i][k + j]; }
    }

    let std_errors: Vec<f64> = (0..k).map(|i| (mse * xtx_inv[i][i]).abs().sqrt()).collect();
    let t_values: Vec<f64> = coefficients.iter().zip(&std_errors)
        .map(|(&b, &se)| if se > 0.0 { b / se } else { 0.0 }).collect();
    let p_values: Vec<f64> = t_values.iter()
        .map(|&t| 2.0 * (1.0 - students_t_cdf(t.abs(), (n - k) as f64))).collect();

    let f_statistic = if p > 0 && mse > 0.0 {
        (ss_tot - ss_res) / p as f64 / mse
    } else { 0.0 };
    let f_p_value = 1.0 - f_distribution_cdf(f_statistic, p as f64, (n - k) as f64);

    Ok(MultipleRegressionResult {
        coefficients, std_errors, t_values, p_values,
        r_squared, adj_r_squared, f_statistic, f_p_value,
    })
}
