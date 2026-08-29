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
    pub variance_a: f64,
    pub variance_b: f64,
    pub standard_error: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MannWhitneyResult {
    pub statistic: f64,
    pub u_a: f64,
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
    pub group_variances: Vec<f64>,
    pub group_sizes: Vec<usize>,
    pub ss_between: f64,
    pub ss_within: f64,
    pub ss_total: f64,
    pub eta_squared: f64,
    pub omega_squared: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TukeyComparison {
    pub group_a: usize,
    pub group_b: usize,
    pub mean_difference: f64,
    pub standard_error: f64,
    pub q_statistic: f64,
    pub p_value: f64,
    pub confidence_lower: f64,
    pub confidence_upper: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TukeyHsdResult {
    pub confidence_level: f64,
    pub df_within: f64,
    pub mean_square_within: f64,
    pub critical_value: f64,
    pub comparisons: Vec<TukeyComparison>,
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
pub struct BreslowDayResult {
    pub common_odds_ratio: f64,
    pub breslow_day_statistic: f64,
    pub breslow_day_p_value: f64,
    pub tarone_adjustment: f64,
    pub tarone_statistic: f64,
    pub tarone_p_value: f64,
    pub df: usize,
    pub expected_exposed_cases: Vec<f64>,
    pub variances: Vec<f64>,
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
pub struct PairedWilcoxonResult {
    /// Sum of the ranks whose paired difference (a - b) is positive. This is
    /// the V statistic reported by R's paired wilcox.test.
    pub statistic: f64,
    pub w_positive: f64,
    pub w_negative: f64,
    pub p_value: f64,
    pub n_pairs: usize,
    pub n_nonzero: usize,
    pub rank_biserial: f64,
    pub has_ties: bool,
    pub has_zero_differences: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct KruskalWallisResult {
    pub h_statistic: f64,
    pub p_value: f64,
    pub df: usize,
    pub group_ranks: Vec<f64>,
    pub tie_correction: f64,
    pub epsilon_squared: f64,
    pub total_n: usize,
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
        // A constant input has no variance, so the correlation is undefined
        // rather than zero. Returning zero asserted "no linear relationship",
        // which `correlation()` then turned into t = 0 and p = 1.0 -- a
        // confident, and wrong, "definitely not significant". NaN matches the
        // `cor` builtin, which has always reported this case that way.
        f64::NAN
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
        let avg_rank = (i + j + 1) as f64 / 2.0;
        for k in i..j {
            ranks[indexed[k].1] = avg_rank;
        }
        i = j;
    }
    ranks
}

// ── Distribution Functions ───────────────────────────────────────────────────

/// Φ(x), the standard normal CDF.
///
/// This used to be the Abramowitz & Stegun rational approximation, good to
/// about seven significant figures — fine for reporting, and not fine for a
/// tail. `Q(1/2, x²/2)` is the same function computed by the series and
/// continued fraction already here for the gamma distribution, which converges
/// to machine precision, and it costs nothing extra to maintain because it is
/// not a second approximation.
///
/// The tail is taken from whichever side is small, so no digits are lost to
/// the subtraction that `1 - Φ(x)` performs at large x.
pub fn normal_cdf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let half_square = 0.5 * x * x;
    if x >= 0.0 {
        0.5 * (1.0 + regularized_gamma_p(0.5, half_square))
    } else {
        0.5 * regularized_gamma_q(0.5, half_square)
    }
}

/// The upper tail of the standard normal: the probability of exceeding `x`.
///
/// `1.0 - normal_cdf(x)` cannot express this. Past about x = 8 the CDF is 1 to
/// every bit a double has, so the subtraction gives exactly 0 — and a p-value
/// of zero is not a stronger result, it is a missing one. It also breaks
/// anything that takes a logarithm, which is the y axis of every volcano plot.
pub fn normal_sf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    normal_cdf(-x)
}

pub fn normal_quantile(p: f64) -> f64 {
    // Peter Acklam's rational approximation (|error| < 1.15e-9)
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    const A: [f64; 6] = [
        -3.969683028665376e+01,
        2.209460984245205e+02,
        -2.759285104469687e+02,
        1.383_577_518_672_69e2,
        -3.066479806614716e+01,
        2.506628277459239e+00,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e+01,
        1.615858368580409e+02,
        -1.556989798598866e+02,
        6.680131188771972e+01,
        -1.328068155288572e+01,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-03,
        -3.223964580411365e-01,
        -2.400758277161838e+00,
        -2.549732539343734e+00,
        4.374664141464968e+00,
        2.938163982698783e+00,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-03,
        3.224671290700398e-01,
        2.445134137142996e+00,
        3.754408661907416e+00,
    ];
    let p_low = 0.02425;
    let p_high = 1.0 - p_low;
    if p < p_low {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p <= p_high {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    } else {
        let q = (-2.0 * (1.0 - p).ln()).sqrt();
        -(((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    }
}

pub fn students_t_cdf(t: f64, df: f64) -> f64 {
    if t.is_nan() || df.is_nan() || df <= 0.0 {
        return f64::NAN;
    }
    if t.is_infinite() {
        return if t > 0.0 { 1.0 } else { 0.0 };
    }
    if t >= 0.0 {
        1.0 - students_t_sf(t, df)
    } else {
        students_t_sf(-t, df)
    }
}

/// CDF of Student's noncentral t distribution.
///
/// This uses the standard Poisson/incomplete-beta expansion for positive
/// ordinates and the symmetry F(t; nu, delta) = 1 - F(-t; nu, -delta).
/// It is independent of R's implementation and is accurate in the parameter
/// range needed by ordinary power calculations.
pub fn noncentral_students_t_cdf(t: f64, df: f64, noncentrality: f64) -> f64 {
    if t.is_nan() || !df.is_finite() || df <= 0.0 || !noncentrality.is_finite() {
        return f64::NAN;
    }
    if t.is_infinite() {
        return if t.is_sign_positive() { 1.0 } else { 0.0 };
    }
    if t == 0.0 {
        return normal_cdf(-noncentrality);
    }
    if t < 0.0 {
        return 1.0 - noncentral_students_t_cdf(-t, df, -noncentrality);
    }

    // For enormous noncentralities the target tail is already beyond f64's
    // useful probability resolution. Avoid an exp(-delta^2/2) underflow that
    // would otherwise make every series weight zero.
    if noncentrality > 40.0 {
        return 0.0;
    }
    if noncentrality < -40.0 {
        return 1.0;
    }

    let lambda = 0.5 * noncentrality * noncentrality;
    let x = t * t / (t * t + df);
    let mut p = (-lambda).exp();
    let mut q = p * noncentrality * (2.0 / std::f64::consts::PI).sqrt();
    let mut sum = 0.0;

    // The weights peak around lambda. Starting at zero is stable for the
    // power-analysis range (delta is normally near z_alpha + z_power), and
    // this generous limit also covers much more extreme inputs.
    for j in 0..10_000usize {
        let jf = j as f64;
        let beta_half = regularized_incomplete_beta(x, jf + 0.5, df / 2.0);
        let beta_one = regularized_incomplete_beta(x, jf + 1.0, df / 2.0);
        let contribution = p * beta_half + q * beta_one;
        sum += contribution;

        p *= lambda / (jf + 1.0);
        q *= lambda / (jf + 1.5);
        if j > lambda as usize + 12 && contribution.abs() < 1e-15 && (p.abs() + q.abs()) < 1e-15 {
            break;
        }
    }

    (normal_cdf(-noncentrality) + 0.5 * sum).clamp(0.0, 1.0)
}

/// Power of an equal-size, two-sided, two-sample t test for a standardized
/// mean difference and a possibly non-integer sample size per group.
pub fn two_sample_t_power(n_per_group: f64, effect_size: f64, alpha: f64) -> f64 {
    if n_per_group <= 1.0
        || effect_size <= 0.0
        || !(0.0..1.0).contains(&alpha)
        || !n_per_group.is_finite()
        || !effect_size.is_finite()
    {
        return f64::NAN;
    }
    let df = 2.0 * (n_per_group - 1.0);
    let critical = students_t_quantile(1.0 - alpha / 2.0, df);
    let noncentrality = effect_size * (n_per_group / 2.0).sqrt();
    let lower = noncentral_students_t_cdf(-critical, df, noncentrality);
    let upper = 1.0 - noncentral_students_t_cdf(critical, df, noncentrality);
    (lower + upper).clamp(0.0, 1.0)
}

/// Required (continuous) sample size per group for an equal-size, two-sided,
/// two-sample t test, solved against the noncentral-t power distribution.
pub fn two_sample_t_required_n(effect_size: f64, alpha: f64, target_power: f64) -> f64 {
    if effect_size <= 0.0
        || !(0.0..1.0).contains(&alpha)
        || !(0.0..1.0).contains(&target_power)
        || !effect_size.is_finite()
    {
        return f64::NAN;
    }

    let mut lower = 1.0 + 1e-7;
    let mut upper = 2.0;
    while two_sample_t_power(upper, effect_size, alpha) < target_power && upper < 1e9 {
        lower = upper;
        upper *= 2.0;
    }
    if upper >= 1e9 && two_sample_t_power(upper, effect_size, alpha) < target_power {
        return f64::INFINITY;
    }

    for _ in 0..100 {
        let midpoint = (lower + upper) / 2.0;
        if two_sample_t_power(midpoint, effect_size, alpha) < target_power {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    (lower + upper) / 2.0
}

/// The upper tail of Student's t: P(T > t).
///
/// Two things were wrong with computing this as `1 - students_t_cdf(t, df)`.
///
/// The subtraction cancels, in the tail, which is where a p-value lives. And
/// above 100 degrees of freedom the CDF used to hand the question to
/// `normal_cdf` outright — the t distribution has heavier tails than the normal
/// at every finite df, so that returned a p-value too small in the direction
/// that overstates significance: at df = 101 and t = 5 the normal gives
/// 2.87e-7 against the true 1.21e-6, a factor of four. The incomplete beta
/// costs a few dozen iterations and is right at any df, so there is no reason
/// to approximate.
pub fn students_t_sf(t: f64, df: f64) -> f64 {
    if t.is_nan() || df.is_nan() || df <= 0.0 {
        return f64::NAN;
    }
    if t.is_infinite() {
        return if t > 0.0 { 0.0 } else { 1.0 };
    }
    if t < 0.0 {
        return 1.0 - students_t_sf(-t, df);
    }
    let x = df / (df + t * t);
    0.5 * regularized_incomplete_beta(x, df / 2.0, 0.5)
}

/// Inverse of [`students_t_cdf`] for finite positive degrees of freedom.
///
/// A bracketed binary search is deliberately used here: confidence intervals
/// are not a hot loop, and monotonic inversion avoids adding another numerical
/// approximation whose tails could disagree with the CDF used for p-values.
pub fn students_t_quantile(probability: f64, df: f64) -> f64 {
    if !probability.is_finite() || probability <= 0.0 || probability >= 1.0 || df <= 0.0 {
        return f64::NAN;
    }
    if (probability - 0.5).abs() < f64::EPSILON {
        return 0.0;
    }
    if probability < 0.5 {
        return -students_t_quantile(1.0 - probability, df);
    }

    let mut lower = 0.0;
    let mut upper = 1.0;
    while students_t_cdf(upper, df) < probability && upper < 1.0e12 {
        upper *= 2.0;
    }
    for _ in 0..120 {
        let midpoint = 0.5 * (lower + upper);
        if students_t_cdf(midpoint, df) < probability {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    0.5 * (lower + upper)
}

pub fn f_distribution_cdf(f: f64, df1: f64, df2: f64) -> f64 {
    if f <= 0.0 {
        return 0.0;
    }
    let x = df1 * f / (df1 * f + df2);
    regularized_incomplete_beta(x, df1 / 2.0, df2 / 2.0)
}

fn simpson_integral<F>(function: F, lower: f64, upper: f64, intervals: usize) -> f64
where
    F: Fn(f64) -> f64,
{
    let intervals = if intervals % 2 == 0 {
        intervals
    } else {
        intervals + 1
    };
    let width = (upper - lower) / intervals as f64;
    let mut total = function(lower) + function(upper);
    for index in 1..intervals {
        let weight = if index % 2 == 0 { 2.0 } else { 4.0 };
        total += weight * function(lower + index as f64 * width);
    }
    total * width / 3.0
}

/// CDF of the range of `groups` independent standard-normal observations.
///
/// The integral is the defining normal-range probability
/// `k integral phi(x) [Phi(x+r)-Phi(x)]^(k-1) dx`. It is evaluated directly,
/// keeping this MIT implementation independent of statistical package code.
fn normal_range_cdf(range: f64, groups: usize) -> f64 {
    if groups < 2 || range <= 0.0 {
        return 0.0;
    }
    if range >= 16.0 {
        return 1.0;
    }
    let exponent = (groups - 1) as i32;
    let result = simpson_integral(
        |location| {
            let probability = (normal_cdf(location + range) - normal_cdf(location)).clamp(0.0, 1.0);
            groups as f64 * normal_pdf(location) * probability.powi(exponent)
        },
        -9.0,
        9.0,
        160,
    );
    result.clamp(0.0, 1.0)
}

/// CDF of Tukey's studentized-range distribution.
///
/// Conditional on the independently estimated variance, the numerator is a
/// normal range. Integrating that probability over its chi-square denominator
/// gives the finite-degrees-of-freedom distribution used by Tukey HSD and the
/// Tukey-Kramer unequal-size extension.
pub fn studentized_range_cdf(q: f64, groups: usize, df: f64) -> f64 {
    if q.is_nan() || df.is_nan() || groups < 2 || df <= 0.0 {
        return f64::NAN;
    }
    if q <= 0.0 {
        return 0.0;
    }
    if q.is_infinite() {
        return 1.0;
    }
    if df >= 1_000.0 {
        return normal_range_cdf(q, groups);
    }

    let shape = df / 2.0;
    let log_normalizer = shape * 2.0_f64.ln() + ln_gamma(shape);
    let upper = df + 14.0 * (2.0 * df).sqrt() + 60.0;
    let result = simpson_integral(
        |chi_square| {
            if chi_square <= 0.0 {
                return 0.0;
            }
            let log_density = (shape - 1.0) * chi_square.ln() - chi_square / 2.0 - log_normalizer;
            let scaled_range = q * (chi_square / df).sqrt();
            log_density.exp() * normal_range_cdf(scaled_range, groups)
        },
        0.0,
        upper,
        240,
    );
    result.clamp(0.0, 1.0)
}

pub fn studentized_range_quantile(probability: f64, groups: usize, df: f64) -> f64 {
    if !probability.is_finite()
        || probability <= 0.0
        || probability >= 1.0
        || groups < 2
        || df <= 0.0
    {
        return f64::NAN;
    }
    let mut lower = 0.0;
    let mut upper = 4.0;
    while studentized_range_cdf(upper, groups, df) < probability && upper < 1.0e4 {
        upper *= 2.0;
    }
    for _ in 0..38 {
        let midpoint = (lower + upper) / 2.0;
        if studentized_range_cdf(midpoint, groups, df) < probability {
            lower = midpoint;
        } else {
            upper = midpoint;
        }
    }
    (lower + upper) / 2.0
}

/// The upper tail of the chi-square distribution: P(X > chi2).
///
/// Computed from the incomplete gamma's own upper branch rather than as
/// `1 - chi_square_cdf(...)`. That subtraction lost precision below about
/// 1e-15 and underflowed to exactly zero past it: chi2 = 81 with one degree of
/// freedom returned 0.0 where the answer is 2.2571e-19.
pub fn chi_square_sf(chi2: f64, df: usize) -> f64 {
    if chi2.is_nan() {
        return f64::NAN;
    }
    if chi2 <= 0.0 {
        return 1.0;
    }
    regularized_gamma_q(df as f64 / 2.0, chi2 / 2.0)
}

pub fn chi_square_cdf(chi2: f64, df: usize) -> f64 {
    if chi2 <= 0.0 {
        return 0.0;
    }
    gamma_cdf(chi2, df as f64 / 2.0, 2.0)
}

pub fn regularized_incomplete_beta(x: f64, a: f64, b: f64) -> f64 {
    if x.is_nan() || a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }

    // Use the symmetry relation for numerical stability
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - regularized_incomplete_beta(1.0 - x, b, a);
    }

    // Compute ln(Beta(a,b)) = ln(Gamma(a)) + ln(Gamma(b)) - ln(Gamma(a+b))
    let ln_beta = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);

    // Front factor: x^a * (1-x)^b / (a * Beta(a,b))
    let front = (a * x.ln() + b * (1.0 - x).ln() - ln_beta).exp() / a;

    // Evaluate continued fraction using modified Lentz's algorithm
    // cf = 1 + d1/(1+ d2/(1+ d3/(1+ ...)))
    // where d_{2m+1} = -(a+m)(a+b+m) x / ((a+2m)(a+2m+1))
    //       d_{2m}   =  m(b-m) x / ((a+2m-1)(a+2m))
    let tiny = 1e-30_f64;
    let eps = 1e-14_f64;
    let mut c = 1.0_f64;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < tiny {
        d = tiny;
    }
    d = 1.0 / d;
    let mut result = d;

    for m in 1..=200 {
        let mf = m as f64;
        // Even step: d_{2m}
        let num_even = mf * (b - mf) * x / ((a + 2.0 * mf - 1.0) * (a + 2.0 * mf));
        d = 1.0 + num_even * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + num_even / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        result *= d * c;

        // Odd step: d_{2m+1}
        let num_odd = -((a + mf) * (a + b + mf) * x) / ((a + 2.0 * mf) * (a + 2.0 * mf + 1.0));
        d = 1.0 + num_odd * d;
        if d.abs() < tiny {
            d = tiny;
        }
        c = 1.0 + num_odd / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        result *= delta;

        if (delta - 1.0).abs() < eps {
            break;
        }
    }

    (front * result).clamp(0.0, 1.0)
}

/// Regularized lower incomplete gamma, P(a, x).
///
/// This is the function every chi-square p-value is built on, so it is worth
/// stating what the two branches are for. The series converges quickly while
/// x < a+1 and badly after it; the continued fraction is the mirror image. Using
/// one of them everywhere is the usual way this goes wrong.
///
/// The previous implementation here had two independent faults. For integer
/// shapes it returned `poisson_cdf(x/scale, shape)`, but the Poisson identity
/// for a gamma CDF is `1 - poisson_cdf(shape-1, x/scale)` - different argument,
/// different rate, and missing the complement. Otherwise it summed the right
/// series but never divided by gamma(a), so results came out a factor of
/// gamma(a) too large and were then hidden by a clamp to 1.0. Together those
/// made `chi_square` report p = 0.000000 for chi2 = 2 on 1 df, where the answer
/// is 0.1573, and p = 0.114 for chi2 = 20 on 3 df, where it is 0.00017 - large
/// statistics coming back with large p-values, which inverts every conclusion
/// drawn from them.
/// P(a, x), the regularized lower incomplete gamma function.
pub fn regularized_gamma_p(a: f64, x: f64) -> f64 {
    if a <= 0.0 || x.is_nan() || a.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 0.0;
    }
    if x < a + 1.0 {
        incomplete_gamma_series(a, x)
    } else {
        (1.0 - incomplete_gamma_continued_fraction(a, x)).clamp(0.0, 1.0)
    }
}

/// Q(a, x) = 1 - P(a, x), the regularized *upper* incomplete gamma function.
///
/// Worth having as its own function rather than as `1.0 - P`: the two branches
/// each compute one tail well and the other by subtraction, so taking whichever
/// branch computes the tail you asked for keeps the digits. Past the point
/// where P rounds to 1, subtraction has none left to keep.
pub fn regularized_gamma_q(a: f64, x: f64) -> f64 {
    if a <= 0.0 || x.is_nan() || a.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 1.0;
    }
    if x < a + 1.0 {
        (1.0 - incomplete_gamma_series(a, x)).clamp(0.0, 1.0)
    } else {
        incomplete_gamma_continued_fraction(a, x)
    }
}

/// P(a, x) by the series, which converges quickly for x below about a + 1.
fn incomplete_gamma_series(a: f64, x: f64) -> f64 {
    // ln of the shared prefactor x^a e^-x / gamma(a), kept in logs so that large
    // a does not overflow before the division.
    let ln_prefactor = -x + a * x.ln() - ln_gamma(a);
    let mut ap = a;
    let mut del = 1.0 / a;
    let mut sum = del;
    for _ in 0..1000 {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() < sum.abs() * 1e-15 {
            break;
        }
    }
    (sum * ln_prefactor.exp()).clamp(0.0, 1.0)
}

/// Q(a, x) by continued fraction (modified Lentz), for x above about a + 1.
fn incomplete_gamma_continued_fraction(a: f64, x: f64) -> f64 {
    let ln_prefactor = -x + a * x.ln() - ln_gamma(a);
    let tiny = 1e-300_f64;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / tiny;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..1000 {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < tiny {
            d = tiny;
        }
        c = b + an / c;
        if c.abs() < tiny {
            c = tiny;
        }
        d = 1.0 / d;
        let delta = d * c;
        h *= delta;
        if (delta - 1.0).abs() < 1e-15 {
            break;
        }
    }
    (ln_prefactor.exp() * h).clamp(0.0, 1.0)
}

fn gamma_cdf(x: f64, shape: f64, scale: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    regularized_gamma_p(shape, x / scale)
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

    // The smallest value cell `a` can take while every cell stays non-negative
    // is max(0, row1 + col1 - n). The guard used to compare row1 against col1,
    // which is a different question entirely: for a = 10, b = 5, c = 3, d = 12
    // it took the subtracting branch with row1 + col1 = 28 and n = 30, and 28 - 30
    // underflowed the unsigned type. In debug that aborted the process; in
    // release it wrapped to about 1.8e19 and the loop below then ran over an
    // essentially unbounded range.
    let current_a = (row1_total + col1_total).saturating_sub(n);
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
    t_test_with_variance(group_a, group_b, alternative, true)
}

/// Two-sample t-test with an explicit variance convention.
///
/// `equal_variance = true` is the historical pooled Student test used by
/// [`t_test`]. `false` applies Welch's standard error and Satterthwaite degrees
/// of freedom, matching R's `t.test()` default.
pub fn t_test_with_variance(
    group_a: &[f64],
    group_b: &[f64],
    alternative: &str,
    equal_variance: bool,
) -> Result<TTestResult, String> {
    if group_a.len() < 2 || group_b.len() < 2 {
        return Err("groups must have at least 2 observations".into());
    }
    let mean_a = mean(group_a);
    let mean_b = mean(group_b);
    let var_a = variance(group_a, mean_a);
    let var_b = variance(group_b, mean_b);
    let n_a = group_a.len() as f64;
    let n_b = group_b.len() as f64;

    let (se, df) = if equal_variance {
        let pooled_var = ((n_a - 1.0) * var_a + (n_b - 1.0) * var_b) / (n_a + n_b - 2.0);
        (
            (pooled_var / n_a + pooled_var / n_b).sqrt(),
            n_a + n_b - 2.0,
        )
    } else {
        let component_a = var_a / n_a;
        let component_b = var_b / n_b;
        let squared_se = component_a + component_b;
        let denominator =
            component_a * component_a / (n_a - 1.0) + component_b * component_b / (n_b - 1.0);
        let welch_df = if denominator > 0.0 {
            squared_se * squared_se / denominator
        } else {
            n_a + n_b - 2.0
        };
        (squared_se.sqrt(), welch_df)
    };
    // When both groups have zero variance, se=0 and t is undefined.
    // If means are equal → t=0, p=1; if means differ → t=±inf, p=0.
    let t_stat = if se == 0.0 {
        if (mean_a - mean_b).abs() < f64::EPSILON {
            0.0
        } else {
            f64::INFINITY * (mean_a - mean_b).signum()
        }
    } else {
        (mean_a - mean_b) / se
    };

    let p_value = match alternative {
        "two_sided" => 2.0 * students_t_sf(t_stat.abs(), df),
        "less" => students_t_cdf(t_stat, df),
        "greater" => students_t_sf(t_stat, df),
        _ => return Err("invalid alternative hypothesis".into()),
    };
    Ok(TTestResult {
        statistic: t_stat,
        p_value,
        df,
        mean_a,
        mean_b,
        variance_a: var_a,
        variance_b: var_b,
        standard_error: se,
    })
}

/// Mann-Whitney U, normal approximation, without a continuity correction.
///
/// This is Scanpy's convention in `rank_genes_groups`. R's `wilcox.test`
/// shifts the statistic half a unit toward the mean before standardising,
/// which is a real and visible difference - on a Seurat marker fixture it was
/// the whole of a 1.4% median discrepancy in the p-values. Neither is wrong;
/// they are different conventions, so the choice is made explicit rather than
/// baked in. Use `mann_whitney_u` to select.
pub fn mann_whitney_test(
    group_a: &[f64],
    group_b: &[f64],
    alternative: &str,
) -> Result<MannWhitneyResult, String> {
    mann_whitney_u(group_a, group_b, alternative, false)
}

/// Mann-Whitney U with the continuity correction selectable.
///
/// `continuity = true` reproduces R's `wilcox.test(..., correct = TRUE)`, which
/// is what Seurat's `FindAllMarkers` reports.
pub fn mann_whitney_u(
    group_a: &[f64],
    group_b: &[f64],
    alternative: &str,
    continuity: bool,
) -> Result<MannWhitneyResult, String> {
    let n_a = group_a.len();
    let n_b = group_b.len();
    if n_a < 1 || n_b < 1 {
        return Err("groups must have at least 1 observation".into());
    }

    let mut combined: Vec<(f64, usize)> = group_a.iter().map(|&v| (v, 0)).collect();
    combined.extend(group_b.iter().map(|&v| (v, 1)));
    combined.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    let mut ranks = vec![0.0; combined.len()];
    let mut tie_correction = 0.0f64;
    let mut i = 0;
    while i < combined.len() {
        let mut j = i;
        while j < combined.len() && (combined[j].0 - combined[i].0).abs() < 1e-10 {
            j += 1;
        }
        let avg_rank = (i + j - 1) as f64 / 2.0 + 1.0;
        for rank in &mut ranks[i..j] {
            *rank = avg_rank;
        }
        // Tied values share an averaged rank, which shrinks the variance of the
        // rank sum. Sizes are collected here so that shrinkage can be applied
        // below; without it the variance is overstated and every p-value from
        // tied data comes out too large.
        let tied = (j - i) as f64;
        if tied > 1.0 {
            tie_correction += tied * tied * tied - tied;
        }
        i = j;
    }

    let mut u_a = 0.0;
    for (i, &(_, group)) in combined.iter().enumerate() {
        if group == 0 {
            u_a += ranks[i];
        }
    }

    let u = u_a - (n_a * (n_a + 1)) as f64 / 2.0;
    let u_stat = u.min((n_a * n_b) as f64 - u);
    let mean_u = (n_a * n_b) as f64 / 2.0;
    // Signed deviation of the *unfolded* statistic, which is what the
    // correction has to act on: folding to min(U, n_a n_b - U) first would
    // always shift toward zero and change the answer's direction.
    let deviation = u - mean_u;
    let shift = if continuity {
        // Half a unit toward the mean, never past it.
        if deviation > 0.0 {
            -0.5
        } else if deviation < 0.0 {
            0.5
        } else {
            0.0
        }
    } else {
        0.0
    };
    // Tie-corrected variance of U, the form both R's wilcox.test and Scanpy's
    // rank_genes_groups use:
    //
    //   sigma^2 = (n_a n_b / 12) * ((N + 1) - sum(t^3 - t) / (N (N - 1)))
    //
    // The correction vanishes when every value is distinct, so this leaves
    // untied data exactly where it was. It matters here because expression
    // data is mostly zeros: on a Seurat marker fixture the uncorrected form
    // disagreed with the reference on 46 of 72 tests.
    let total = (n_a + n_b) as f64;
    let var_u = if total > 1.0 {
        ((n_a * n_b) as f64 / 12.0) * ((total + 1.0) - tie_correction / (total * (total - 1.0)))
    } else {
        0.0
    };
    let sigma = var_u.sqrt();
    let z = if sigma > 0.0 {
        (deviation + shift) / sigma
    } else {
        0.0
    };

    let p_value = match alternative {
        "two_sided" => 2.0 * normal_sf(z.abs()),
        "less" => normal_cdf(z),
        "greater" => normal_sf(z),
        _ => return Err("invalid alternative hypothesis".into()),
    };
    Ok(MannWhitneyResult {
        statistic: u_stat,
        u_a: u,
        p_value,
        n_a,
        n_b,
    })
}

/// Exact Mann-Whitney U distribution for untied observations.
///
/// R also falls back from its exact calculation when ties are present. The
/// explicit error keeps callers from believing they received an exact p-value
/// when the rank distribution used here is not valid for their data.
pub fn mann_whitney_exact_test(
    group_a: &[f64],
    group_b: &[f64],
    alternative: &str,
) -> Result<MannWhitneyResult, String> {
    let n_a = group_a.len();
    let n_b = group_b.len();
    if n_a < 1 || n_b < 1 {
        return Err("groups must have at least 1 observation".into());
    }
    let total = n_a + n_b;
    if total > 50 {
        return Err(
            "exact Mann-Whitney is limited to 50 total observations; use method='normal'".into(),
        );
    }

    let mut combined = group_a.to_vec();
    combined.extend_from_slice(group_b);
    combined.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if combined.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(
            "exact Mann-Whitney requires untied observations; use method='normal' for ties".into(),
        );
    }

    let u_a = group_a
        .iter()
        .map(|a| group_b.iter().filter(|b| a > *b).count() as f64)
        .sum::<f64>();
    let observed_u = u_a.round() as usize;
    let maximum_rank_sum = n_a * (2 * total - n_a + 1) / 2;
    let minimum_rank_sum = n_a * (n_a + 1) / 2;
    let mut counts = vec![vec![0.0_f64; maximum_rank_sum + 1]; n_a + 1];
    counts[0][0] = 1.0;
    for rank in 1..=total {
        for selected in (1..=n_a.min(rank)).rev() {
            for sum in (rank..=maximum_rank_sum).rev() {
                counts[selected][sum] += counts[selected - 1][sum - rank];
            }
        }
    }
    let distribution = &counts[n_a];
    let total_count = distribution.iter().sum::<f64>();
    let lower_count = distribution
        .iter()
        .enumerate()
        .filter(|(rank_sum, _)| rank_sum.saturating_sub(minimum_rank_sum) <= observed_u)
        .map(|(_, count)| *count)
        .sum::<f64>();
    let upper_count = distribution
        .iter()
        .enumerate()
        .filter(|(rank_sum, _)| rank_sum.saturating_sub(minimum_rank_sum) >= observed_u)
        .map(|(_, count)| *count)
        .sum::<f64>();
    let p_value = match alternative {
        "two_sided" => (2.0 * lower_count.min(upper_count) / total_count).min(1.0),
        "less" => lower_count / total_count,
        "greater" => upper_count / total_count,
        _ => return Err("invalid alternative hypothesis".into()),
    };
    Ok(MannWhitneyResult {
        statistic: u_a.min((n_a * n_b) as f64 - u_a),
        u_a,
        p_value,
        n_a,
        n_b,
    })
}

pub fn anova(groups: &[Vec<f64>]) -> Result<AnovaResult, String> {
    if groups.len() < 2 {
        return Err("ANOVA requires at least 2 groups".into());
    }
    let mut all_values = Vec::new();
    let mut group_means = Vec::new();
    let mut group_variances = Vec::new();
    let mut group_sizes = Vec::new();
    for group in groups {
        if group.is_empty() {
            return Err("ANOVA groups cannot be empty".into());
        }
        if group.iter().any(|value| !value.is_finite()) {
            return Err("ANOVA groups must contain only finite values".into());
        }
        let group_mean = mean(group);
        group_means.push(group_mean);
        group_variances.push(if group.len() > 1 {
            variance(group, group_mean)
        } else {
            f64::NAN
        });
        group_sizes.push(group.len());
        all_values.extend_from_slice(group);
    }
    let grand_mean = mean(&all_values);
    let total_n = all_values.len() as f64;

    let ssb: f64 = group_sizes
        .iter()
        .zip(&group_means)
        .map(|(&size, &gm)| size as f64 * (gm - grand_mean).powi(2))
        .sum();
    let ssw: f64 = groups
        .iter()
        .zip(&group_means)
        .map(|(group, &gm)| group.iter().map(|&x| (x - gm).powi(2)).sum::<f64>())
        .sum();

    let df_between = groups.len() as f64 - 1.0;
    let df_within = total_n - groups.len() as f64;
    let msb = ssb / df_between;
    let msw = ssw / df_within;
    let f_stat = if msw > 0.0 { msb / msw } else { f64::INFINITY };
    let p_value = 1.0 - f_distribution_cdf(f_stat, df_between, df_within);
    let sst = ssb + ssw;
    let eta_squared = if sst > 0.0 { ssb / sst } else { f64::NAN };
    let omega_squared = if sst + msw > 0.0 {
        (ssb - df_between * msw) / (sst + msw)
    } else {
        f64::NAN
    };

    Ok(AnovaResult {
        f_statistic: f_stat,
        p_value,
        df_between,
        df_within,
        group_means,
        group_variances,
        group_sizes,
        ss_between: ssb,
        ss_within: ssw,
        ss_total: sst,
        eta_squared,
        omega_squared,
    })
}

/// Welch's heteroscedastic one-way analysis of means.
///
/// This is Welch's 1951 weighted-means statistic with the usual approximate
/// F distribution.  The sums of squares and effect sizes in the returned
/// record remain descriptive raw-data quantities; they are not used to form
/// the heteroscedastic test statistic.
pub fn welch_anova(groups: &[Vec<f64>]) -> Result<AnovaResult, String> {
    let descriptive = anova(groups)?;
    let k = groups.len() as f64;
    let weights = descriptive
        .group_sizes
        .iter()
        .zip(&descriptive.group_variances)
        .map(|(&size, &sample_variance)| {
            if size < 2 {
                Err("Welch ANOVA requires at least 2 observations in every group".to_string())
            } else if sample_variance <= 0.0 || !sample_variance.is_finite() {
                Err("Welch ANOVA requires positive sample variance in every group".to_string())
            } else {
                Ok(size as f64 / sample_variance)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let weight_sum = weights.iter().sum::<f64>();
    let weighted_mean = weights
        .iter()
        .zip(&descriptive.group_means)
        .map(|(weight, group_mean)| weight * group_mean)
        .sum::<f64>()
        / weight_sum;
    let numerator = weights
        .iter()
        .zip(&descriptive.group_means)
        .map(|(weight, group_mean)| weight * (group_mean - weighted_mean).powi(2))
        .sum::<f64>()
        / (k - 1.0);
    let correction_sum = weights
        .iter()
        .zip(&descriptive.group_sizes)
        .map(|(weight, &size)| {
            let relative_weight = weight / weight_sum;
            (1.0 - relative_weight).powi(2) / (size as f64 - 1.0)
        })
        .sum::<f64>();
    let denominator = 1.0 + 2.0 * (k - 2.0) * correction_sum / (k * k - 1.0);
    let f_statistic = numerator / denominator;
    let df_between = k - 1.0;
    let df_within = (k * k - 1.0) / (3.0 * correction_sum);
    let p_value = 1.0 - f_distribution_cdf(f_statistic, df_between, df_within);

    Ok(AnovaResult {
        f_statistic,
        p_value,
        df_between,
        df_within,
        ..descriptive
    })
}

/// Tukey HSD with the Tukey-Kramer standard error for unequal group sizes.
pub fn tukey_hsd(groups: &[Vec<f64>], confidence: f64) -> Result<TukeyHsdResult, String> {
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return Err("Tukey HSD confidence must be between 0 and 1".into());
    }
    if groups.iter().any(|group| group.len() < 2) {
        return Err("Tukey HSD requires at least 2 observations in every group".into());
    }
    let analysis = anova(groups)?;
    if analysis.df_within <= 0.0 {
        return Err("Tukey HSD requires positive residual degrees of freedom".into());
    }
    let mean_square_within = analysis.ss_within / analysis.df_within;
    if mean_square_within <= 0.0 || !mean_square_within.is_finite() {
        return Err("Tukey HSD requires positive within-group variance".into());
    }
    let critical_value = studentized_range_quantile(confidence, groups.len(), analysis.df_within);
    let mut comparisons = Vec::new();
    for group_a in 0..groups.len() {
        for group_b in (group_a + 1)..groups.len() {
            let mean_difference = analysis.group_means[group_a] - analysis.group_means[group_b];
            let standard_error = (mean_square_within
                * 0.5
                * (1.0 / analysis.group_sizes[group_a] as f64
                    + 1.0 / analysis.group_sizes[group_b] as f64))
                .sqrt();
            let q_statistic = mean_difference.abs() / standard_error;
            let p_value = (1.0
                - studentized_range_cdf(q_statistic, groups.len(), analysis.df_within))
            .clamp(0.0, 1.0);
            comparisons.push(TukeyComparison {
                group_a,
                group_b,
                mean_difference,
                standard_error,
                q_statistic,
                p_value,
                confidence_lower: mean_difference - critical_value * standard_error,
                confidence_upper: mean_difference + critical_value * standard_error,
            });
        }
    }
    Ok(TukeyHsdResult {
        confidence_level: confidence,
        df_within: analysis.df_within,
        mean_square_within,
        critical_value,
        comparisons,
    })
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
    let p_value = 2.0 * students_t_sf(t_stat.abs(), n as f64 - 2.0);
    Ok(CorrelationResult {
        correlation: corr,
        p_value,
        n,
    })
}

/// Chi-square test of independence on an r x c contingency table.
///
/// Distinct from a goodness-of-fit test, and the distinction is the degrees of
/// freedom. Goodness of fit compares k counts against k proportions fixed in
/// advance and has k - 1 of them. Here the expected counts are estimated from
/// the table's own margins, which uses up r - 1 and c - 1 more, leaving
/// (r - 1)(c - 1). Reaching for the goodness-of-fit entry point on a 2x2 table
/// gives df = 3 where the answer is 1.
///
/// `yates` applies the continuity correction, which R's `chisq.test` does by
/// default on 2x2 tables and never on larger ones.
pub fn chi_square_contingency(table: &[Vec<f64>], yates: bool) -> Result<ChiSquareResult, String> {
    let rows = table.len();
    if rows < 2 {
        return Err("contingency table needs at least 2 rows".into());
    }
    let cols = table[0].len();
    if cols < 2 {
        return Err("contingency table needs at least 2 columns".into());
    }
    if table.iter().any(|row| row.len() != cols) {
        return Err("contingency table rows must all have the same length".into());
    }
    if table.iter().flatten().any(|count| *count < 0.0) {
        return Err("contingency table counts cannot be negative".into());
    }

    let row_totals: Vec<f64> = table.iter().map(|row| row.iter().sum()).collect();
    let col_totals: Vec<f64> = (0..cols)
        .map(|j| table.iter().map(|row| row[j]).sum())
        .collect();
    let total: f64 = row_totals.iter().sum();
    if total <= 0.0 {
        return Err("contingency table is empty".into());
    }

    let mut expected = vec![vec![0.0; cols]; rows];
    let mut chi_sq = 0.0;
    // Yates applies only to 2x2, which is the only shape where the correction
    // was derived and the only one R applies it to.
    let apply_yates = yates && rows == 2 && cols == 2;
    for i in 0..rows {
        for j in 0..cols {
            let e = row_totals[i] * col_totals[j] / total;
            expected[i][j] = e;
            if e <= 0.0 {
                return Err(
                    "contingency table has a row or column of zeros, so an expected count is zero"
                        .into(),
                );
            }
            let mut deviation = (table[i][j] - e).abs();
            if apply_yates {
                // Never past zero: subtracting half from a deviation smaller
                // than half would move the statistic away from its own null.
                deviation = (deviation - 0.5).max(0.0);
            }
            chi_sq += deviation * deviation / e;
        }
    }

    let df = (rows - 1) * (cols - 1);
    Ok(ChiSquareResult {
        chi_square: chi_sq,
        p_value: chi_square_sf(chi_sq, df),
        df,
        expected,
    })
}

/// Breslow-Day homogeneity test for a common odds ratio, with Tarone's
/// efficient-score adjustment.
///
/// Each stratum is `[a, b, c, d]`, corresponding to `[[a, b], [c, d]]`.
/// Margins are treated as fixed. Expected exposed-case counts are obtained
/// from the quadratic equation under the Mantel-Haenszel common odds ratio.
/// The adjustment follows Tarone, "On heterogeneity tests based on efficient
/// scores", Biometrika 72(1), 1985, pp. 91-95.
pub fn breslow_day_test(strata: &[[f64; 4]]) -> Result<BreslowDayResult, String> {
    if strata.len() < 2 {
        return Err("Breslow-Day test needs at least two 2x2 strata".into());
    }
    if strata
        .iter()
        .flatten()
        .any(|count| !count.is_finite() || *count < 0.0)
    {
        return Err("Breslow-Day strata must contain finite non-negative counts".into());
    }

    let mut mh_numerator = 0.0;
    let mut mh_denominator = 0.0;
    for &[a, b, c, d] in strata {
        let total = a + b + c + d;
        let row_one = a + b;
        let row_two = c + d;
        let col_one = a + c;
        let col_two = b + d;
        if total <= 0.0 || row_one <= 0.0 || row_two <= 0.0 || col_one <= 0.0 || col_two <= 0.0 {
            return Err(
                "each Breslow-Day stratum must have positive row and column margins".into(),
            );
        }
        mh_numerator += a * d / total;
        mh_denominator += b * c / total;
    }
    if mh_numerator <= 0.0 || mh_denominator <= 0.0 {
        return Err("the Mantel-Haenszel common odds ratio is zero or undefined".into());
    }
    let common_odds_ratio = mh_numerator / mh_denominator;

    let mut expected_exposed_cases = Vec::with_capacity(strata.len());
    let mut variances = Vec::with_capacity(strata.len());
    let mut residual_sum = 0.0;
    let mut variance_sum = 0.0;
    let mut breslow_day_statistic = 0.0;

    for &[observed_a, b, c, d] in strata {
        let row_one = observed_a + b;
        let row_two = c + d;
        let col_one = observed_a + c;
        let total = row_one + row_two;
        let expected_a = if (common_odds_ratio - 1.0).abs() <= 1e-12 {
            row_one * col_one / total
        } else {
            let coefficient = common_odds_ratio * (row_one + col_one) + (row_two - col_one);
            let raw_discriminant = coefficient * coefficient
                - 4.0 * row_one * col_one * common_odds_ratio * (common_odds_ratio - 1.0);
            let scale = coefficient.abs().max(1.0).powi(2);
            if raw_discriminant < -1e-12 * scale {
                return Err("Breslow-Day fitted-count quadratic has no real solution".into());
            }
            (coefficient - raw_discriminant.max(0.0).sqrt()) / (2.0 * (common_odds_ratio - 1.0))
        };
        let expected_b = row_one - expected_a;
        let expected_c = col_one - expected_a;
        let expected_d = row_two - expected_c;
        if [expected_a, expected_b, expected_c, expected_d]
            .iter()
            .any(|cell| !cell.is_finite() || *cell <= 0.0)
        {
            return Err("Breslow-Day fitted table contains a non-positive cell".into());
        }
        let variance =
            1.0 / (1.0 / expected_a + 1.0 / expected_b + 1.0 / expected_c + 1.0 / expected_d);
        let residual = observed_a - expected_a;
        breslow_day_statistic += residual * residual / variance;
        residual_sum += residual;
        variance_sum += variance;
        expected_exposed_cases.push(expected_a);
        variances.push(variance);
    }

    let tarone_adjustment = residual_sum * residual_sum / variance_sum;
    let tarone_statistic = (breslow_day_statistic - tarone_adjustment).max(0.0);
    let df = strata.len() - 1;
    Ok(BreslowDayResult {
        common_odds_ratio,
        breslow_day_statistic,
        breslow_day_p_value: chi_square_sf(breslow_day_statistic, df),
        tarone_adjustment,
        tarone_statistic,
        tarone_p_value: chi_square_sf(tarone_statistic, df),
        df,
        expected_exposed_cases,
        variances,
    })
}

pub fn chi_square_test(table: &[Vec<f64>]) -> Result<ChiSquareResult, String> {
    if table.len() != 2 || table[0].len() != 2 {
        return Err("contingency table must be 2x2".into());
    }
    chi_square_contingency(table, false)
}

// ── Conditional odds ratio for a 2x2 table ───────────────────────────────────
//
// The cross-product ad/bc is the sample odds ratio, and it is what this module
// reported. R's `fisher.test` reports something else: the value of the odds
// ratio that maximises the likelihood of the observed table given its margins,
// under the noncentral hypergeometric distribution. Both are standard, they
// answer slightly different questions, and they do not agree -- on the Titanic
// 2x2 the sample ratio is 10.147 and the conditional MLE 10.1319. Anyone
// cross-checking against R finds the mismatch and has to work out which is
// wrong, so both are now reported, each under its own name.

/// The support of `a` given the margins, and the log weights over it.
///
/// Kept in logs because the binomial coefficients overflow long before the
/// probabilities become interesting.
fn noncentral_log_weights(row1: u64, row2: u64, col1: u64) -> (u64, Vec<f64>) {
    let lo = col1.saturating_sub(row2);
    let hi = row1.min(col1);
    let weights = (lo..=hi)
        .map(|k| log_binomial(row1, k) + log_binomial(row2, col1 - k))
        .collect();
    (lo, weights)
}

/// E[a | psi], the mean of the noncentral hypergeometric distribution.
fn noncentral_mean(lo: u64, log_weights: &[f64], log_psi: f64) -> f64 {
    let terms: Vec<f64> = log_weights
        .iter()
        .enumerate()
        .map(|(i, w)| w + (lo + i as u64) as f64 * log_psi)
        .collect();
    let peak = terms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !peak.is_finite() {
        return f64::NAN;
    }
    let mut total = 0.0;
    let mut weighted = 0.0;
    for (i, term) in terms.iter().enumerate() {
        let p = (term - peak).exp();
        total += p;
        weighted += p * (lo + i as u64) as f64;
    }
    weighted / total
}

/// P(a <= observed | psi) and P(a >= observed | psi).
fn noncentral_tails(lo: u64, log_weights: &[f64], log_psi: f64, observed: u64) -> (f64, f64) {
    let terms: Vec<f64> = log_weights
        .iter()
        .enumerate()
        .map(|(i, w)| w + (lo + i as u64) as f64 * log_psi)
        .collect();
    let peak = terms.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut total = 0.0;
    let mut at_or_below = 0.0;
    let mut at_or_above = 0.0;
    for (i, term) in terms.iter().enumerate() {
        let p = (term - peak).exp();
        let k = lo + i as u64;
        total += p;
        if k <= observed {
            at_or_below += p;
        }
        if k >= observed {
            at_or_above += p;
        }
    }
    (at_or_below / total, at_or_above / total)
}

/// Solve a monotone function of log(psi) for the value where it hits `target`.
///
/// Bisection on the log scale rather than Newton: the odds ratio spans many
/// orders of magnitude, the functions here are monotone but not always
/// smoothly so at the ends of the support, and a hundred halvings of a
/// [-100, 100] bracket is exact to well past what a double can hold.
fn solve_log_psi(target: f64, increasing: bool, evaluate: impl Fn(f64) -> f64) -> f64 {
    let (mut low, mut high) = (-100.0_f64, 100.0_f64);
    for _ in 0..200 {
        let middle = 0.5 * (low + high);
        let value = evaluate(middle);
        let below = if increasing {
            value < target
        } else {
            value > target
        };
        if below {
            low = middle;
        } else {
            high = middle;
        }
    }
    0.5 * (low + high)
}

/// The conditional maximum likelihood odds ratio, as R's `fisher.test` reports.
///
/// It is the psi at which the observed count is the expected count. At either
/// end of the support no finite psi does that -- the likelihood is still
/// climbing -- and the answer is 0 or infinity, which is also what R gives.
pub fn conditional_odds_ratio(a: u64, b: u64, c: u64, d: u64) -> f64 {
    let (row1, row2, col1) = (a + b, c + d, a + c);
    let (lo, log_weights) = noncentral_log_weights(row1, row2, col1);
    let hi = lo + log_weights.len() as u64 - 1;
    if a == lo {
        return 0.0;
    }
    if a == hi {
        return f64::INFINITY;
    }
    solve_log_psi(a as f64, true, |log_psi| {
        noncentral_mean(lo, &log_weights, log_psi)
    })
    .exp()
}

/// The exact confidence interval for the conditional odds ratio.
///
/// Obtained by inverting the test, as R does: the bounds are the odds ratios at
/// which the observed table sits exactly at the tail probability. Not the same
/// interval as the Wald one on the log of the sample ratio, and not centred on
/// anything -- which is the point, since the sampling distribution is not
/// symmetric.
pub fn conditional_odds_ratio_interval(
    a: u64,
    b: u64,
    c: u64,
    d: u64,
    confidence: f64,
) -> (f64, f64) {
    let (row1, row2, col1) = (a + b, c + d, a + c);
    let (lo, log_weights) = noncentral_log_weights(row1, row2, col1);
    let hi = lo + log_weights.len() as u64 - 1;
    let alpha = (1.0 - confidence) / 2.0;

    // P(X >= a | psi) climbs from 0 to 1 as psi grows, so the lower bound is
    // where it first reaches alpha; P(X <= a | psi) falls the other way, and
    // the upper bound is where it drops to alpha.
    let lower = if a == lo {
        0.0
    } else {
        solve_log_psi(alpha, true, |log_psi| {
            noncentral_tails(lo, &log_weights, log_psi, a).1
        })
        .exp()
    };
    let upper = if a == hi {
        f64::INFINITY
    } else {
        solve_log_psi(alpha, false, |log_psi| {
            noncentral_tails(lo, &log_weights, log_psi, a).0
        })
        .exp()
    };
    (lower, upper)
}

pub fn fishers_exact_test(table: &[Vec<f64>]) -> Result<FishersResult, String> {
    fishers_exact_test_with_confidence(table, 0.95)
}

pub fn fishers_exact_test_with_confidence(
    table: &[Vec<f64>],
    confidence: f64,
) -> Result<FishersResult, String> {
    if table.len() != 2 || table[0].len() != 2 {
        return Err("contingency table must be 2x2".into());
    }
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return Err("confidence must be between 0 and 1".into());
    }
    let a = table[0][0] as u64;
    let b = table[0][1] as u64;
    let c = table[1][0] as u64;
    let d = table[1][1] as u64;
    let odds_ratio = if b == 0 || c == 0 {
        if a == 0 || d == 0 {
            f64::NAN
        } else {
            f64::INFINITY
        }
    } else {
        (a as f64 * d as f64) / (b as f64 * c as f64)
    };
    let p_value = fishers_exact_p_value(a, b, c, d);
    let confidence_interval = odds_ratio_confidence_interval(a, b, c, d, confidence);
    Ok(FishersResult {
        odds_ratio,
        p_value,
        confidence_interval,
    })
}

pub fn descriptive_statistics(data: &[f64]) -> DescriptiveStats {
    if data.is_empty() {
        return DescriptiveStats {
            count: 0,
            mean: 0.0,
            median: 0.0,
            std_dev: 0.0,
            min: 0.0,
            max: 0.0,
            q25: 0.0,
            q75: 0.0,
        };
    }
    let count = data.len();
    let mean_val = mean(data);
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if count.is_multiple_of(2) {
        (sorted[count / 2 - 1] + sorted[count / 2]) / 2.0
    } else {
        sorted[count / 2]
    };
    let q25 = sorted[(count as f64 * 0.25) as usize];
    let q75 = sorted[(count as f64 * 0.75) as usize];
    let std_dev = variance(data, mean_val).sqrt();
    DescriptiveStats {
        count,
        mean: mean_val,
        median,
        std_dev,
        min: *sorted.first().unwrap(),
        max: *sorted.last().unwrap(),
        q25,
        q75,
    }
}

// ── Regression ───────────────────────────────────────────────────────────────

pub fn linear_regression(x: &[f64], y: &[f64]) -> Result<LinearRegressionResult, String> {
    if x.len() != y.len() || x.len() < 2 {
        return Err("insufficient data for regression".into());
    }
    let n = x.len() as f64;
    let x_mean = mean(x);
    let y_mean = mean(y);
    let numerator: f64 = x
        .iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| (xi - x_mean) * (yi - y_mean))
        .sum();
    let denominator: f64 = x.iter().map(|&xi| (xi - x_mean).powi(2)).sum();
    if denominator == 0.0 {
        return Err("no variance in x values".into());
    }

    let slope = numerator / denominator;
    let intercept = y_mean - slope * x_mean;
    let y_pred: Vec<f64> = x.iter().map(|&xi| slope * xi + intercept).collect();
    let ss_res: f64 = y
        .iter()
        .zip(&y_pred)
        .map(|(&yi, &yp)| (yi - yp).powi(2))
        .sum();
    let ss_tot: f64 = y.iter().map(|&yi| (yi - y_mean).powi(2)).sum();
    let r_squared = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };
    let mse = ss_res / (n - 2.0);
    let se_slope = (mse / denominator).sqrt();
    let t_stat = slope / se_slope;
    let p_value = 2.0 * students_t_sf(t_stat.abs(), n - 2.0);
    Ok(LinearRegressionResult {
        slope,
        intercept,
        r_squared,
        p_value,
        std_error: se_slope,
    })
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
        let p: Vec<f64> = (0..x.len())
            .map(|i| 1.0 / (1.0 + (-(beta[0] + beta[1] * x[i])).exp()))
            .collect();
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
        let delta = [
            gradient[0] / hessian[0][0].max(1.0),
            gradient[1] / hessian[1][1].max(1.0),
        ];
        beta[0] += delta[0];
        beta[1] += delta[1];
        if delta[0].abs() < tolerance && delta[1].abs() < tolerance {
            break;
        }
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
        vec![2.0 * normal_sf(z0.abs()), 2.0 * normal_sf(z1.abs())]
    } else {
        vec![f64::NAN; 2]
    };
    Ok(LogisticRegressionResult {
        coefficients: beta,
        p_values,
        log_likelihood,
        aic,
    })
}

// ── Non-parametric Tests ─────────────────────────────────────────────────────

pub fn wilcoxon_signed_rank_test(
    group_a: &[f64],
    group_b: &[f64],
    alternative: &str,
) -> Result<WilcoxonResult, String> {
    if group_a.len() != group_b.len() {
        return Err("paired test requires equal group sizes".into());
    }
    let n = group_a.len();
    if n < 1 {
        return Err("insufficient data for paired test".into());
    }

    let differences: Vec<f64> = group_a
        .iter()
        .zip(group_b.iter())
        .map(|(&a, &b)| a - b)
        .filter(|&d| d != 0.0)
        .collect();
    if differences.is_empty() {
        return Ok(WilcoxonResult {
            statistic: 0.0,
            p_value: 1.0,
            n_pairs: n,
        });
    }

    let mut abs_diffs: Vec<(f64, usize)> = differences
        .iter()
        .enumerate()
        .map(|(i, &d)| (d.abs(), i))
        .collect();
    abs_diffs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut ranks = vec![0.0; differences.len()];
    let mut i = 0;
    while i < abs_diffs.len() {
        let mut j = i;
        while j < abs_diffs.len() && (abs_diffs[j].0 - abs_diffs[i].0).abs() < 1e-10 {
            j += 1;
        }
        let avg_rank = (i + j + 1) as f64 / 2.0;
        for k in i..j {
            ranks[abs_diffs[k].1] = avg_rank;
        }
        i = j;
    }

    let signed_ranks: Vec<f64> = differences
        .iter()
        .zip(&ranks)
        .map(|(&d, &r)| if d > 0.0 { r } else { -r })
        .collect();
    let w_positive: f64 = signed_ranks.iter().filter(|&&r| r > 0.0).sum();
    let w_negative: f64 = signed_ranks
        .iter()
        .filter(|&&r| r < 0.0)
        .map(|r| r.abs())
        .sum();
    let w_stat = w_positive.min(w_negative);
    let nd = differences.len() as f64;
    let mean_w = nd * (nd + 1.0) / 4.0;
    let var_w = nd * (nd + 1.0) * (2.0 * nd + 1.0) / 24.0;
    let z = (w_stat - mean_w) / var_w.sqrt();

    let p_value = match alternative {
        "two_sided" => 2.0 * normal_sf(z.abs()),
        "less" => normal_cdf(z),
        "greater" => normal_sf(z),
        _ => return Err("invalid alternative hypothesis".into()),
    };
    Ok(WilcoxonResult {
        statistic: w_stat,
        p_value,
        n_pairs: n,
    })
}

/// Paired Wilcoxon signed-rank test with an explicit inference backend.
///
/// `exact = true` uses the finite null distribution and therefore requires
/// distinct, non-zero absolute differences. The normal backend applies the
/// standard tie correction; `continuity` moves the statistic half a rank
/// toward its null mean. Differences are defined as `group_a - group_b`.
pub fn paired_wilcoxon_signed_rank_test(
    group_a: &[f64],
    group_b: &[f64],
    alternative: &str,
    exact: bool,
    continuity: bool,
) -> Result<PairedWilcoxonResult, String> {
    if group_a.len() != group_b.len() {
        return Err("paired Wilcoxon test requires equal group sizes".into());
    }
    if group_a.is_empty() {
        return Err("paired Wilcoxon test requires at least one pair".into());
    }
    if group_a
        .iter()
        .chain(group_b.iter())
        .any(|value| !value.is_finite())
    {
        return Err("paired Wilcoxon test requires finite values".into());
    }
    if !matches!(alternative, "two_sided" | "less" | "greater") {
        return Err("invalid alternative hypothesis".into());
    }
    if exact && continuity {
        return Err("continuity correction applies only to the normal method".into());
    }

    let differences: Vec<f64> = group_a
        .iter()
        .zip(group_b.iter())
        .map(|(&a, &b)| a - b)
        .filter(|difference| *difference != 0.0)
        .collect();
    let has_zero_differences = differences.len() != group_a.len();
    if differences.is_empty() {
        if exact {
            return Err("exact paired Wilcoxon test is undefined with zero differences".into());
        }
        return Ok(PairedWilcoxonResult {
            statistic: 0.0,
            w_positive: 0.0,
            w_negative: 0.0,
            p_value: 1.0,
            n_pairs: group_a.len(),
            n_nonzero: 0,
            rank_biserial: 0.0,
            has_ties: false,
            has_zero_differences,
        });
    }

    let mut ordered: Vec<(f64, usize)> = differences
        .iter()
        .enumerate()
        .map(|(index, difference)| (difference.abs(), index))
        .collect();
    ordered.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut ranks = vec![0.0; differences.len()];
    let mut tie_sizes = Vec::new();
    let mut start = 0;
    while start < ordered.len() {
        let mut end = start + 1;
        while end < ordered.len() && ordered[end].0 == ordered[start].0 {
            end += 1;
        }
        let average_rank = (start + end + 1) as f64 / 2.0;
        for item in &ordered[start..end] {
            ranks[item.1] = average_rank;
        }
        if end - start > 1 {
            tie_sizes.push(end - start);
        }
        start = end;
    }
    let has_ties = !tie_sizes.is_empty();
    if exact && (has_ties || has_zero_differences) {
        return Err(
            "exact paired Wilcoxon test requires distinct, non-zero absolute differences; use method='normal'"
                .into(),
        );
    }
    if exact && differences.len() > 50 {
        return Err("exact paired Wilcoxon test supports at most 50 non-zero pairs".into());
    }

    let w_positive: f64 = differences
        .iter()
        .zip(ranks.iter())
        .filter_map(|(difference, rank)| (*difference > 0.0).then_some(*rank))
        .sum();
    let total_rank: f64 = ranks.iter().sum();
    let w_negative = total_rank - w_positive;
    let rank_biserial = if total_rank > 0.0 {
        (w_positive - w_negative) / total_rank
    } else {
        0.0
    };

    let p_value = if exact {
        let total_rank_integer = differences.len() * (differences.len() + 1) / 2;
        let observed = w_positive.round() as usize;
        let mut counts = vec![0_u128; total_rank_integer + 1];
        counts[0] = 1;
        let mut reachable = 0;
        for rank in 1..=differences.len() {
            for sum in (0..=reachable).rev() {
                counts[sum + rank] = counts[sum + rank].saturating_add(counts[sum]);
            }
            reachable += rank;
        }
        let outcomes = 2_f64.powi(differences.len() as i32);
        let lower = counts[..=observed]
            .iter()
            .map(|count| *count as f64)
            .sum::<f64>()
            / outcomes;
        let upper = counts[observed..]
            .iter()
            .map(|count| *count as f64)
            .sum::<f64>()
            / outcomes;
        match alternative {
            "two_sided" => (2.0 * lower.min(upper)).min(1.0),
            "less" => lower,
            "greater" => upper,
            _ => unreachable!(),
        }
    } else {
        let n = differences.len() as f64;
        let mean = n * (n + 1.0) / 4.0;
        let tie_term: f64 = tie_sizes
            .iter()
            .map(|size| {
                let size = *size as f64;
                size.powi(3) - size
            })
            .sum();
        let variance = n * (n + 1.0) * (2.0 * n + 1.0) / 24.0 - tie_term / 48.0;
        if variance <= 0.0 {
            1.0
        } else {
            let correction = if continuity {
                match alternative {
                    "two_sided" => 0.5 * (w_positive - mean).signum(),
                    "less" => -0.5,
                    "greater" => 0.5,
                    _ => unreachable!(),
                }
            } else {
                0.0
            };
            let z = (w_positive - mean - correction) / variance.sqrt();
            match alternative {
                "two_sided" => (2.0 * normal_sf(z.abs())).min(1.0),
                "less" => normal_cdf(z),
                "greater" => normal_sf(z),
                _ => unreachable!(),
            }
        }
    };

    Ok(PairedWilcoxonResult {
        statistic: w_positive,
        w_positive,
        w_negative,
        p_value,
        n_pairs: group_a.len(),
        n_nonzero: differences.len(),
        rank_biserial,
        has_ties,
        has_zero_differences,
    })
}

pub fn kruskal_wallis_test(groups: &[Vec<f64>]) -> Result<KruskalWallisResult, String> {
    if groups.len() < 2 {
        return Err("Kruskal-Wallis requires at least 2 groups".into());
    }
    let mut all_values: Vec<(f64, usize)> = Vec::new();
    for (gi, group) in groups.iter().enumerate() {
        if group.is_empty() {
            return Err("Kruskal-Wallis groups cannot be empty".into());
        }
        for &v in group {
            if !v.is_finite() {
                return Err("Kruskal-Wallis groups must contain only finite values".into());
            }
            all_values.push((v, gi));
        }
    }
    if all_values.is_empty() {
        return Err("no data in groups".into());
    }

    all_values.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut ranks = vec![0.0; all_values.len()];
    let mut tie_sum = 0.0;
    let mut i = 0;
    while i < all_values.len() {
        let mut j = i;
        while j < all_values.len() && (all_values[j].0 - all_values[i].0).abs() < 1e-10 {
            j += 1;
        }
        let avg_rank = (i + j + 1) as f64 / 2.0;
        let tie_size = (j - i) as f64;
        tie_sum += tie_size.powi(3) - tie_size;
        for rank in &mut ranks[i..j] {
            *rank = avg_rank;
        }
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
    let uncorrected_h = (12.0 / (n * (n + 1.0)))
        * group_ranks
            .iter()
            .zip(&group_sizes)
            .map(|(&r, &size)| (r - size as f64 * mean_rank).powi(2) / size as f64)
            .sum::<f64>();
    let tie_correction = 1.0 - tie_sum / (n.powi(3) - n);
    if tie_correction <= 0.0 {
        return Err("Kruskal-Wallis is undefined when every observation is tied".into());
    }
    let h_stat = uncorrected_h / tie_correction;
    let df = groups.len() - 1;
    let p_value = chi_square_sf(h_stat, df);
    let epsilon_squared = if all_values.len() > groups.len() {
        (h_stat - groups.len() as f64 + 1.0) / (all_values.len() as f64 - groups.len() as f64)
    } else {
        f64::NAN
    };
    Ok(KruskalWallisResult {
        h_statistic: h_stat,
        p_value,
        df,
        group_ranks,
        tie_correction,
        epsilon_squared,
        total_n: all_values.len(),
    })
}

// ── Multivariate Analysis ────────────────────────────────────────────────────

pub fn principal_component_analysis(
    matrix: &[Vec<f64>],
    n_components: usize,
) -> Result<PcaResult, String> {
    if matrix.is_empty() || matrix[0].is_empty() {
        return Err("empty data matrix".into());
    }
    let n_samples = matrix.len();
    let n_features = matrix[0].len();
    let n_comp = n_components.min(n_features);

    let mut col_means = vec![0.0; n_features];
    for row in matrix {
        for (j, &v) in row.iter().enumerate() {
            col_means[j] += v;
        }
    }
    for m in &mut col_means {
        *m /= n_samples as f64;
    }

    let centered: Vec<Vec<f64>> = matrix
        .iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(j, &v)| v - col_means[j])
                .collect()
        })
        .collect();

    let mut cov = vec![vec![0.0; n_features]; n_features];
    for row in &centered {
        for i in 0..n_features {
            for j in i..n_features {
                cov[i][j] += row[i] * row[j];
            }
        }
    }
    let denom = (n_samples - 1).max(1) as f64;
    #[allow(clippy::needless_range_loop)]
    for i in 0..n_features {
        for j in i..n_features {
            cov[i][j] /= denom;
            cov[j][i] = cov[i][j];
        }
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
                for j in 0..n_features {
                    w[i] += deflated_cov[i][j] * v[j];
                }
            }
            eigenvalue = 0.0;
            for i in 0..n_features {
                eigenvalue += v[i] * w[i];
            }
            let norm: f64 = w.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-15 {
                break;
            }
            let new_v: Vec<f64> = w.iter().map(|x| x / norm).collect();
            let diff: f64 = v.iter().zip(&new_v).map(|(a, b)| (a - b).powi(2)).sum();
            v = new_v;
            if diff < 1e-12 {
                break;
            }
        }
        eigenvalues.push(eigenvalue.max(0.0));
        eigenvectors.push(v.clone());
        for i in 0..n_features {
            for j in 0..n_features {
                deflated_cov[i][j] -= eigenvalue * v[i] * v[j];
            }
        }
    }

    let total_var: f64 = eigenvalues.iter().sum();
    let explained_variance_ratio = if total_var > 0.0 {
        eigenvalues.iter().map(|&e| e / total_var).collect()
    } else {
        vec![0.0; n_comp]
    };

    let transformed_data: Vec<Vec<f64>> = centered
        .iter()
        .map(|row| {
            eigenvectors
                .iter()
                .map(|ev| row.iter().zip(ev).map(|(a, b)| a * b).sum())
                .collect()
        })
        .collect();

    Ok(PcaResult {
        explained_variance: eigenvalues,
        explained_variance_ratio,
        components: eigenvectors,
        transformed_data,
    })
}

pub fn factor_analysis(
    matrix: &[Vec<f64>],
    n_factors: usize,
) -> Result<FactorAnalysisResult, String> {
    if matrix.is_empty() || matrix[0].is_empty() {
        return Err("empty data matrix".into());
    }
    let n_variables = matrix[0].len();
    let pca = principal_component_analysis(matrix, n_factors)?;

    let loadings: Vec<Vec<f64>> = (0..n_variables)
        .map(|var_idx| {
            (0..n_factors.min(pca.components.len()))
                .map(|f| {
                    let ev = pca
                        .components
                        .get(f)
                        .and_then(|c| c.get(var_idx))
                        .copied()
                        .unwrap_or(0.0);
                    let eigenval = pca.explained_variance.get(f).copied().unwrap_or(0.0);
                    ev * eigenval.sqrt()
                })
                .collect()
        })
        .collect();

    let communalities: Vec<f64> = loadings
        .iter()
        .map(|row| row.iter().map(|l| l * l).sum::<f64>().min(1.0))
        .collect();
    let uniqueness: Vec<f64> = communalities.iter().map(|c| (1.0 - c).max(0.0)).collect();
    Ok(FactorAnalysisResult {
        loadings,
        communalities,
        uniqueness,
        factor_scores: pca.transformed_data,
    })
}

// ── Time Series ──────────────────────────────────────────────────────────────

pub fn arima_forecast(
    time_series: &[f64],
    p: usize,
    d: usize,
    q: usize,
) -> Result<ArimaResult, String> {
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
                for i in lag..n {
                    s += centered[i] * centered[i - lag];
                }
                acf[lag] = s / n as f64;
            }
            let mut a = vec![0.0; ar_order];
            let mut e = acf[0];
            for m in 0..ar_order {
                let mut lambda = acf[m + 1];
                for j in 0..m {
                    lambda -= a[j] * acf[m - j];
                }
                if e.abs() < 1e-15 {
                    break;
                }
                let k = lambda / e;
                let mut new_a = a.clone();
                new_a[m] = k;
                for j in 0..m {
                    new_a[j] = a[j] - k * a[m - 1 - j];
                }
                a = new_a;
                e *= 1.0 - k * k;
            }
            ar_coeffs = a;
        }
    }

    let mut residuals = vec![0.0; n];
    for i in ar_order..n {
        let mut pred = mean_d;
        for j in 0..ar_order {
            pred += ar_coeffs[j] * centered[i - 1 - j];
        }
        residuals[i] = diffed[i] - pred;
    }

    let ma_order = q.min(n.saturating_sub(ar_order + 1));
    let mut ma_coeffs = vec![0.0; ma_order];
    if ma_order > 0 {
        let res_var: f64 =
            residuals[ar_order..].iter().map(|r| r * r).sum::<f64>() / (n - ar_order).max(1) as f64;
        if res_var > 1e-15 {
            for lag in 1..=ma_order {
                let mut s = 0.0;
                for i in (ar_order + lag)..n {
                    s += residuals[i] * residuals[i - lag];
                }
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
        for j in 0..ar_order.min(m) {
            pred += ar_coeffs[j] * extended[m - 1 - j];
        }
        for j in 0..ma_order.min(ext_resid.len()) {
            pred += ma_coeffs[j] * ext_resid[ext_resid.len() - 1 - j];
        }
        extended.push(pred);
        ext_resid.push(0.0);
    }

    let mut forecasts: Vec<f64> = extended[n..].iter().map(|&x| x + mean_d).collect();
    for _ in 0..d {
        let mut prev = *time_series.last().unwrap();
        for f in &mut forecasts {
            prev += *f;
            *f = prev;
        }
    }

    let ss_res: f64 = residuals[ar_order..].iter().map(|r| r * r).sum();
    let n_eff = (n - ar_order) as f64;
    let k_params = (p + q + 1) as f64;
    let sigma2 = ss_res / n_eff.max(1.0);
    let ll = -0.5 * n_eff * (2.0 * std::f64::consts::PI * sigma2).ln()
        - ss_res / (2.0 * sigma2.max(1e-15));
    let aic = -2.0 * ll + 2.0 * k_params;
    let bic = -2.0 * ll + k_params * n_eff.ln();

    Ok(ArimaResult {
        forecasts,
        residuals,
        aic,
        bic,
    })
}

pub fn time_series_forecast(time_series: &[f64], steps: usize) -> Result<ForecastResult, String> {
    if time_series.is_empty() {
        return Err("empty time series".into());
    }
    let alpha = 0.3;
    let mut level = time_series[0];
    for &value in &time_series[1..] {
        level = alpha * value + (1.0 - alpha) * level;
    }
    let forecasts = vec![level; steps];
    let confidence_intervals = forecasts.iter().map(|&f| (f * 0.9, f * 1.1)).collect();
    Ok(ForecastResult {
        forecasts,
        confidence_intervals,
        method: "Exponential Smoothing".to_string(),
    })
}

// ── Multiple Testing Corrections ─────────────────────────────────────────────

pub fn benjamini_hochberg_correction(p_values: &[f64], alpha: f64) -> MultipleTestingResult {
    let mut indexed_p: Vec<(f64, usize)> =
        p_values.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    indexed_p.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let m = p_values.len() as f64;
    let mut adjusted = vec![0.0; p_values.len()];
    // Compute raw BH values (p * m / rank) and enforce the step-up monotonicity:
    // adjusted p-values must be non-decreasing in rank, so sweep from the largest
    // rank down, carrying a running minimum. Without this, tied p-values diverge
    // and a smaller raw p can receive a larger adjusted p than a larger one.
    let mut running_min = 1.0_f64;
    for (rank, (p_value, original_idx)) in indexed_p.iter().enumerate().rev() {
        let rank_f = (rank + 1) as f64;
        let raw = (p_value * m / rank_f).min(1.0);
        running_min = running_min.min(raw);
        adjusted[*original_idx] = running_min;
    }
    let rejected: Vec<usize> = indexed_p
        .iter()
        .filter(|(_, idx)| adjusted[*idx] <= alpha)
        .map(|(_, idx)| *idx)
        .collect();
    MultipleTestingResult {
        adjusted_p_values: adjusted,
        rejected,
        threshold: alpha,
    }
}

pub fn bonferroni_correction(p_values: &[f64]) -> MultipleTestingResult {
    let m = p_values.len() as f64;
    let adjusted_p_values: Vec<f64> = p_values.iter().map(|&p| (p * m).min(1.0)).collect();
    let rejected: Vec<usize> = adjusted_p_values
        .iter()
        .enumerate()
        .filter(|(_, &p)| p <= 0.05)
        .map(|(i, _)| i)
        .collect();
    MultipleTestingResult {
        adjusted_p_values,
        rejected,
        threshold: 0.05,
    }
}

pub fn holm_bonferroni_correction(p_values: &[f64]) -> MultipleTestingResult {
    let mut indexed_p: Vec<(f64, usize)> =
        p_values.iter().enumerate().map(|(i, &p)| (p, i)).collect();
    indexed_p.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let m = p_values.len() as f64;
    let mut adjusted = vec![0.0; p_values.len()];
    let mut running_max = 0.0_f64;
    for (rank, (p_value, original_idx)) in indexed_p.iter().enumerate() {
        let rank_f = (rank + 1) as f64;
        let raw_adjusted = (p_value * (m - rank_f + 1.0)).min(1.0);
        // Holm adjusted p-values must be monotone in sorted-p order. Without
        // the cumulative maximum, tied or closely spaced p-values can receive
        // a smaller adjusted value at a later rank (for example 0.03, 0.02).
        running_max = running_max.max(raw_adjusted);
        adjusted[*original_idx] = running_max;
    }
    let rejected = indexed_p
        .iter()
        .filter(|(_, original_idx)| adjusted[*original_idx] <= 0.05)
        .map(|(_, original_idx)| *original_idx)
        .collect();
    MultipleTestingResult {
        adjusted_p_values: adjusted,
        rejected,
        threshold: 0.05,
    }
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
    let p_value = 2.0 * normal_sf(z.abs());
    Ok(CorrelationResult {
        correlation: tau,
        p_value,
        n,
    })
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
        if v1 <= v2 {
            i += 1;
        }
        if v2 <= v1 {
            j += 1;
        }
        let f1 = i as f64 / n1 as f64;
        let f2 = j as f64 / n2 as f64;
        let d = (f1 - f2).abs();
        if d > d_max {
            d_max = d;
        }
    }

    // Asymptotic p-value approximation
    let en = ((n1 * n2) as f64 / (n1 + n2) as f64).sqrt();
    let lambda = (en + 0.12 + 0.11 / en) * d_max;
    let mut p_value = 0.0;
    for k in 1..=100 {
        let term = (-2.0 * (k as f64 * lambda).powi(2)).exp();
        if k % 2 == 0 {
            p_value -= term;
        } else {
            p_value += term;
        }
        if term.abs() < 1e-15 {
            break;
        }
    }
    let p_value = (2.0 * p_value).clamp(0.0, 1.0);
    Ok(KsTestResult {
        statistic: d_max,
        p_value,
        n1,
        n2,
    })
}

pub fn kaplan_meier(times: &[f64], events: &[bool]) -> Result<KaplanMeierResult, String> {
    if times.len() != events.len() || times.is_empty() {
        return Err("times and events must have same non-zero length".into());
    }
    let mut data: Vec<(f64, bool)> = times
        .iter()
        .zip(events.iter())
        .map(|(&t, &e)| (t, e))
        .collect();
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
            if data[i].1 {
                d += 1;
            } else {
                c += 1;
            }
            i += 1;
        }
        if d > 0 {
            let n = n_at_risk as f64;
            let d_f = d as f64;
            s *= 1.0 - d_f / n;
            if n > d_f {
                var_sum += d_f / (n * (n - d_f));
            }
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

fn invert_square_matrix(matrix: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return None;
    }
    let mut augmented = (0..n)
        .map(|row| {
            let mut values = Vec::with_capacity(2 * n);
            values.extend_from_slice(&matrix[row]);
            values.extend((0..n).map(|column| if row == column { 1.0 } else { 0.0 }));
            values
        })
        .collect::<Vec<_>>();
    for column in 0..n {
        let pivot_row = (column..n).max_by(|left, right| {
            augmented[*left][column]
                .abs()
                .total_cmp(&augmented[*right][column].abs())
        })?;
        if augmented[pivot_row][column].abs() <= 1e-14 {
            return None;
        }
        augmented.swap(column, pivot_row);
        let pivot = augmented[column][column];
        for value in &mut augmented[column] {
            *value /= pivot;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            for entry in 0..(2 * n) {
                augmented[row][entry] -= factor * augmented[column][entry];
            }
        }
    }
    Some(augmented.into_iter().map(|row| row[n..].to_vec()).collect())
}

pub fn cox_ph(
    time: &[f64],
    event: &[bool],
    covariates: &[Vec<f64>],
) -> Result<CoxPhResult, String> {
    if time.len() != event.len() || time.is_empty() {
        return Err("time and event must have same non-zero length".into());
    }
    let n = time.len();
    let p = if covariates.is_empty() {
        0
    } else {
        covariates[0].len()
    };
    if p == 0 {
        return Err("at least one covariate required".into());
    }
    for cov in covariates {
        if cov.len() != p {
            return Err("all covariate vectors must have same length".into());
        }
    }
    if covariates.len() != n {
        return Err("number of covariate rows must match number of observations".into());
    }

    // Centre the model matrix to keep exp(x beta) numerically stable. The Cox
    // partial likelihood is invariant to this column-wise shift.
    let means = (0..p)
        .map(|column| covariates.iter().map(|row| row[column]).sum::<f64>() / n as f64)
        .collect::<Vec<_>>();
    let model_covariates = covariates
        .iter()
        .map(|row| {
            row.iter()
                .zip(&means)
                .map(|(value, mean)| value - mean)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    // Full multivariable Newton-Raphson for the Breslow partial likelihood.
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
            if !event[i] {
                continue;
            }
            let mut risk_sum = 0.0;
            let mut weighted_x = vec![0.0; p];
            let mut weighted_xx = vec![vec![0.0; p]; p];
            let maximum_eta = order
                .iter()
                .filter(|index| time[**index] >= time[i])
                .map(|index| {
                    (0..p)
                        .map(|k| beta[k] * model_covariates[*index][k])
                        .sum::<f64>()
                })
                .max_by(f64::total_cmp)
                .unwrap_or(0.0);

            for &j in &order {
                if time[j] >= time[i] {
                    let eta: f64 = (0..p).map(|k| beta[k] * model_covariates[j][k]).sum();
                    let w = (eta - maximum_eta).exp();
                    risk_sum += w;
                    for k in 0..p {
                        weighted_x[k] += w * model_covariates[j][k];
                        for l in 0..p {
                            weighted_xx[k][l] +=
                                w * model_covariates[j][k] * model_covariates[j][l];
                        }
                    }
                }
            }

            if risk_sum > 0.0 {
                for k in 0..p {
                    gradient[k] += model_covariates[i][k] - weighted_x[k] / risk_sum;
                    for l in 0..p {
                        hessian[k][l] -= weighted_xx[k][l] / risk_sum
                            - (weighted_x[k] * weighted_x[l]) / (risk_sum * risk_sum);
                    }
                }
            }
        }

        let information = hessian
            .iter()
            .map(|row| row.iter().map(|value| -*value).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let covariance = invert_square_matrix(&information)
            .ok_or_else(|| "Cox information matrix is singular".to_string())?;
        let delta = covariance
            .iter()
            .map(|row| row.iter().zip(&gradient).map(|(a, b)| a * b).sum::<f64>())
            .collect::<Vec<_>>();
        let max_change = delta.iter().map(|value| value.abs()).fold(0.0, f64::max);
        for k in 0..p {
            beta[k] += delta[k];
        }
        if max_change < tol {
            break;
        }
    }

    let hazard_ratios: Vec<f64> = beta.iter().map(|&b| b.exp()).collect();

    // Standard errors from diagonal of information matrix
    let mut se = vec![0.0; p];
    let mut info = vec![vec![0.0; p]; p];
    for &i in &order {
        if !event[i] {
            continue;
        }
        let mut risk_sum = 0.0;
        let mut weighted_x = vec![0.0; p];
        let mut weighted_xx = vec![vec![0.0; p]; p];
        let maximum_eta = order
            .iter()
            .filter(|index| time[**index] >= time[i])
            .map(|index| {
                (0..p)
                    .map(|k| beta[k] * model_covariates[*index][k])
                    .sum::<f64>()
            })
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        for &j in &order {
            if time[j] >= time[i] {
                let eta: f64 = (0..p).map(|k| beta[k] * model_covariates[j][k]).sum();
                let w = (eta - maximum_eta).exp();
                risk_sum += w;
                for k in 0..p {
                    weighted_x[k] += w * model_covariates[j][k];
                    for l in 0..p {
                        weighted_xx[k][l] += w * model_covariates[j][k] * model_covariates[j][l];
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
    let covariance = invert_square_matrix(&info)
        .ok_or_else(|| "fitted Cox information matrix is singular".to_string())?;
    for k in 0..p {
        se[k] = covariance[k][k].max(0.0).sqrt();
    }

    let p_values: Vec<f64> = beta
        .iter()
        .zip(&se)
        .map(|(&b, &s)| {
            if s.is_nan() || s == 0.0 {
                f64::NAN
            } else {
                2.0 * normal_sf((b / s).abs())
            }
        })
        .collect();

    // Concordance index
    let mut concordant = 0u64;
    let mut total = 0u64;
    for i in 0..n {
        if !event[i] {
            continue;
        }
        for j in 0..n {
            if time[j] > time[i] {
                let eta_i: f64 = (0..p).map(|k| beta[k] * covariates[i][k]).sum();
                let eta_j: f64 = (0..p).map(|k| beta[k] * covariates[j][k]).sum();
                total += 1;
                if eta_i > eta_j {
                    concordant += 1;
                }
            }
        }
    }
    let concordance = if total > 0 {
        concordant as f64 / total as f64
    } else {
        0.5
    };

    Ok(CoxPhResult {
        coefficients: beta,
        hazard_ratios,
        p_values,
        concordance,
    })
}

pub fn multiple_linear_regression(
    y: &[f64],
    x_matrix: &[Vec<f64>],
) -> Result<MultipleRegressionResult, String> {
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
        let row: Vec<f64> = std::iter::once(1.0)
            .chain(x_matrix[i].iter().copied())
            .collect();
        for a in 0..k {
            for b in 0..k {
                xtx[a][b] += row[a] * row[b];
            }
            xty[a] += row[a] * y[i];
        }
    }

    // Solve via Gauss elimination
    let mut aug: Vec<Vec<f64>> = xtx
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let mut r = row.clone();
            r.push(xty[i]);
            r
        })
        .collect();
    for col in 0..k {
        let mut max_row = col;
        for row in (col + 1)..k {
            if aug[row][col].abs() > aug[max_row][col].abs() {
                max_row = row;
            }
        }
        aug.swap(col, max_row);
        if aug[col][col].abs() < 1e-15 {
            return Err("singular matrix — collinear predictors".into());
        }
        let pivot = aug[col][col];
        for j in col..=k {
            aug[col][j] /= pivot;
        }
        for row in 0..k {
            if row != col {
                let factor = aug[row][col];
                for j in col..=k {
                    aug[row][j] -= factor * aug[col][j];
                }
            }
        }
    }
    let coefficients: Vec<f64> = aug.iter().map(|row| row[k]).collect();

    // Residuals and R²
    let y_mean = mean(y);
    let mut ss_res = 0.0;
    let mut ss_tot = 0.0;
    for i in 0..n {
        let row: Vec<f64> = std::iter::once(1.0)
            .chain(x_matrix[i].iter().copied())
            .collect();
        let y_pred: f64 = row.iter().zip(&coefficients).map(|(x, b)| x * b).sum();
        ss_res += (y[i] - y_pred).powi(2);
        ss_tot += (y[i] - y_mean).powi(2);
    }
    let r_squared = if ss_tot > 0.0 {
        1.0 - ss_res / ss_tot
    } else {
        0.0
    };
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
        let row: Vec<f64> = std::iter::once(1.0)
            .chain(x_matrix[i].iter().copied())
            .collect();
        for a in 0..k {
            for b in 0..k {
                xtx2[a][b] += row[a] * row[b];
            }
        }
    }
    // Augment with identity
    for i in 0..k {
        let mut row = xtx2[i].clone();
        for j in 0..k {
            row.push(if i == j { 1.0 } else { 0.0 });
        }
        aug2.push(row);
    }
    for col in 0..k {
        let mut max_row = col;
        for row in (col + 1)..k {
            if aug2[row][col].abs() > aug2[max_row][col].abs() {
                max_row = row;
            }
        }
        aug2.swap(col, max_row);
        let pivot = aug2[col][col];
        if pivot.abs() < 1e-15 {
            continue;
        }
        for j in 0..(2 * k) {
            aug2[col][j] /= pivot;
        }
        for row in 0..k {
            if row != col {
                let factor = aug2[row][col];
                for j in 0..(2 * k) {
                    aug2[row][j] -= factor * aug2[col][j];
                }
            }
        }
    }
    for i in 0..k {
        for j in 0..k {
            xtx_inv[i][j] = aug2[i][k + j];
        }
    }

    let std_errors: Vec<f64> = (0..k).map(|i| (mse * xtx_inv[i][i]).abs().sqrt()).collect();
    let t_values: Vec<f64> = coefficients
        .iter()
        .zip(&std_errors)
        .map(|(&b, &se)| if se > 0.0 { b / se } else { 0.0 })
        .collect();
    let p_values: Vec<f64> = t_values
        .iter()
        .map(|&t| 2.0 * students_t_sf(t.abs(), (n - k) as f64))
        .collect();

    let f_statistic = if p > 0 && mse > 0.0 {
        (ss_tot - ss_res) / p as f64 / mse
    } else {
        0.0
    };
    let f_p_value = 1.0 - f_distribution_cdf(f_statistic, p as f64, (n - k) as f64);

    Ok(MultipleRegressionResult {
        coefficients,
        std_errors,
        t_values,
        p_values,
        r_squared,
        adj_r_squared,
        f_statistic,
        f_p_value,
    })
}

// ── K-Means Clustering ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct KMeansResult {
    pub clusters: Vec<usize>,
    pub centroids: Vec<Vec<f64>>,
    pub iterations: usize,
    pub inertia: f64,
}

/// K-means clustering using Lloyd's algorithm with k-means++ initialization.
pub fn kmeans(data: &[Vec<f64>], k: usize, max_iter: usize) -> Result<KMeansResult, String> {
    let n = data.len();
    if n == 0 {
        return Err("empty data".into());
    }
    if k == 0 || k > n {
        return Err(format!("k must be in [1, {n}]"));
    }
    let d = data[0].len();
    if d == 0 {
        return Err("zero-dimensional data".into());
    }

    let mut centroids = Vec::with_capacity(k);
    let first = data
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            let na: f64 = a.iter().map(|x| x * x).sum();
            let nb: f64 = b.iter().map(|x| x * x).sum();
            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    centroids.push(data[first].clone());

    for _ in 1..k {
        let dists: Vec<f64> = data
            .iter()
            .map(|p| {
                centroids
                    .iter()
                    .map(|c| euclidean_dist_sq(p, c))
                    .fold(f64::MAX, f64::min)
            })
            .collect();
        let total: f64 = dists.iter().sum();
        if total == 0.0 {
            centroids.push(data[0].clone());
            continue;
        }
        let idx = dists
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(i, _)| i)
            .unwrap_or(0);
        centroids.push(data[idx].clone());
    }

    let mut assignments = vec![0usize; n];
    let mut iterations = 0;

    for _ in 0..max_iter {
        iterations += 1;
        let mut changed = false;
        for i in 0..n {
            let nearest = centroids
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    euclidean_dist_sq(&data[i], a)
                        .partial_cmp(&euclidean_dist_sq(&data[i], b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(j, _)| j)
                .unwrap_or(0);
            if assignments[i] != nearest {
                assignments[i] = nearest;
                changed = true;
            }
        }
        if !changed {
            break;
        }
        for j in 0..k {
            let mut new_centroid = vec![0.0; d];
            let mut count = 0usize;
            for i in 0..n {
                if assignments[i] == j {
                    for dim in 0..d {
                        new_centroid[dim] += data[i][dim];
                    }
                    count += 1;
                }
            }
            if count > 0 {
                for dim in 0..d {
                    new_centroid[dim] /= count as f64;
                }
                centroids[j] = new_centroid;
            }
        }
    }

    let inertia: f64 = (0..n)
        .map(|i| euclidean_dist_sq(&data[i], &centroids[assignments[i]]))
        .sum();

    Ok(KMeansResult {
        clusters: assignments,
        centroids,
        iterations,
        inertia,
    })
}

fn euclidean_dist_sq(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

// ── Distribution Functions ──────────────────────────────────────────────────

/// Normal PDF: φ(x) = (1/√(2π)) * exp(-x²/2)
pub fn normal_pdf(x: f64) -> f64 {
    (-x * x / 2.0).exp() / (2.0 * std::f64::consts::PI).sqrt()
}

/// Lanczos approximation to log-gamma.
pub(crate) fn ln_gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    let coeffs = [
        76.18009172947146,
        -86.50532032941677,
        24.01409824083091,
        -1.231739572450155,
        1.208650973866179e-3,
        -5.395239384953e-6,
    ];
    let mut y = x;
    let tmp = x + 5.5;
    let tmp = tmp - (x + 0.5) * tmp.ln();
    let mut ser = 1.000000000190015;
    for &c in &coeffs {
        y += 1.0;
        ser += c / y;
    }
    -tmp + (2.5066282746310005 * ser / x).ln()
}

/// Binomial coefficient: C(n, k)
pub fn binomial_coeff(n: u64, k: u64) -> f64 {
    if k > n {
        return 0.0;
    }
    (ln_gamma(n as f64 + 1.0) - ln_gamma(k as f64 + 1.0) - ln_gamma((n - k) as f64 + 1.0)).exp()
}

/// Binomial PMF: P(X = k) = C(n,k) * p^k * (1-p)^(n-k)
pub fn binomial_pmf(k: u64, n: u64, p: f64) -> f64 {
    if k > n {
        return 0.0;
    }
    binomial_coeff(n, k) * p.powi(k as i32) * (1.0 - p).powi((n - k) as i32)
}

/// Binomial CDF: P(X <= k)
pub fn binom_cdf(k: u64, n: u64, p: f64) -> f64 {
    (0..=k).map(|i| binomial_pmf(i, n, p)).sum()
}

/// Poisson PMF: P(X = k) = λ^k * e^(-λ) / k!
pub fn poisson_pmf_exact(k: u64, lambda: f64) -> f64 {
    if lambda <= 0.0 {
        return if k == 0 { 1.0 } else { 0.0 };
    }
    (k as f64 * lambda.ln() - lambda - ln_gamma(k as f64 + 1.0)).exp()
}

/// Poisson CDF: P(X <= k)
pub fn poisson_cdf_exact(k: u64, lambda: f64) -> f64 {
    (0..=k).map(|i| poisson_pmf_exact(i, lambda)).sum()
}

/// Uniform PDF on [a, b]
pub fn uniform_pdf(x: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return 0.0;
    }
    if x >= a && x <= b {
        1.0 / (b - a)
    } else {
        0.0
    }
}

/// Uniform CDF on [a, b]
pub fn uniform_cdf(x: f64, a: f64, b: f64) -> f64 {
    if b <= a {
        return 0.0;
    }
    if x < a {
        0.0
    } else if x > b {
        1.0
    } else {
        (x - a) / (b - a)
    }
}

/// Exponential PDF: f(x) = rate * exp(-rate * x)
pub fn exponential_pdf(x: f64, rate: f64) -> f64 {
    if x < 0.0 || rate <= 0.0 {
        0.0
    } else {
        rate * (-rate * x).exp()
    }
}

/// Exponential CDF: F(x) = 1 - exp(-rate * x)
pub fn exponential_cdf(x: f64, rate: f64) -> f64 {
    if x < 0.0 || rate <= 0.0 {
        0.0
    } else {
        1.0 - (-rate * x).exp()
    }
}
