use bio_core::stats_ops::*;

#[test]
fn test_mean() {
    assert_eq!(mean(&[1.0, 2.0, 3.0, 4.0, 5.0]), 3.0);
}

#[test]
fn test_variance() {
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let m = mean(&data);
    assert!((variance(&data, m) - 2.5).abs() < 1e-10);
}

#[test]
fn test_pearson_perfect() {
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [2.0, 4.0, 6.0, 8.0, 10.0];
    assert!((pearson_correlation(&x, &y) - 1.0).abs() < 1e-10);
}

#[test]
fn test_spearman_monotonic() {
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [1.0, 4.0, 9.0, 16.0, 25.0];
    assert!((spearman_correlation(&x, &y) - 1.0).abs() < 1e-10);
}

#[test]
fn test_descriptive() {
    let data = [1.0, 2.0, 3.0, 4.0, 5.0];
    let stats = descriptive_statistics(&data);
    assert_eq!(stats.count, 5);
    assert_eq!(stats.mean, 3.0);
    assert_eq!(stats.median, 3.0);
    assert_eq!(stats.min, 1.0);
    assert_eq!(stats.max, 5.0);
}

#[test]
fn test_t_test() {
    let a = [2.0, 3.0, 4.0, 5.0, 6.0];
    let b = [5.0, 6.0, 7.0, 8.0, 9.0];
    let result = t_test(&a, &b, "two_sided").unwrap();
    assert!((result.mean_a - 4.0).abs() < 1e-10);
    assert!((result.mean_b - 7.0).abs() < 1e-10);
    assert!(result.p_value < 0.05);
}

#[test]
fn test_anova() {
    let groups = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0], vec![7.0, 8.0, 9.0]];
    let result = anova(&groups).unwrap();
    assert!((result.f_statistic - 27.0).abs() < 1.0);
    assert!(result.p_value < 0.01);
}

#[test]
fn test_correlation_pearson() {
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [2.0, 4.0, 6.0, 8.0, 10.0];
    let result = correlation(&x, &y, "pearson").unwrap();
    assert!((result.correlation - 1.0).abs() < 1e-6);
}

#[test]
fn test_chi_square() {
    let table = vec![vec![10.0, 20.0], vec![20.0, 10.0]];
    let result = chi_square_test(&table).unwrap();
    assert!(result.chi_square > 3.0);
}

#[test]
fn test_linear_regression() {
    let x: Vec<f64> = (1..=10).map(|i| i as f64).collect();
    let y: Vec<f64> = x.iter().map(|&xi| 2.0 * xi + 1.0).collect();
    let result = linear_regression(&x, &y).unwrap();
    assert!((result.slope - 2.0).abs() < 0.01);
    assert!((result.intercept - 1.0).abs() < 0.01);
    assert!((result.r_squared - 1.0).abs() < 0.01);
}

#[test]
fn test_mann_whitney() {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [6.0, 7.0, 8.0, 9.0, 10.0];
    let result = mann_whitney_test(&a, &b, "two_sided").unwrap();
    assert!((result.statistic - 0.0).abs() < 1.0);
    assert!(result.p_value < 0.05);
}

#[test]
fn test_wilcoxon() {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [2.0, 3.0, 4.0, 5.0, 6.0];
    let result = wilcoxon_signed_rank_test(&a, &b, "two_sided").unwrap();
    assert_eq!(result.n_pairs, 5);
    assert!((result.statistic - 0.0).abs() < 1.0);
}

#[test]
fn test_kruskal_wallis() {
    let groups = vec![vec![1.0, 2.0, 3.0], vec![4.0, 5.0, 6.0], vec![7.0, 8.0, 9.0]];
    let result = kruskal_wallis_test(&groups).unwrap();
    assert!(result.h_statistic > 5.0);
    assert!(result.p_value < 0.05);
}

#[test]
fn test_pca() {
    let matrix = vec![
        vec![1.0, 1.1], vec![2.0, 2.1], vec![3.0, 2.9], vec![4.0, 4.1], vec![5.0, 5.0],
        vec![6.0, 5.9], vec![7.0, 7.1], vec![8.0, 8.0], vec![9.0, 9.1], vec![10.0, 10.0],
    ];
    let result = principal_component_analysis(&matrix, 2).unwrap();
    assert!(result.explained_variance_ratio[0] > 0.90);
}

#[test]
fn test_arima() {
    let ts = vec![10.0, 12.0, 11.0, 14.0, 13.0, 16.0, 15.0, 18.0, 17.0, 20.0];
    let result = arima_forecast(&ts, 1, 1, 0).unwrap();
    assert!(!result.forecasts.is_empty());
    assert!(result.forecasts[0] > 15.0);
    assert!(result.aic.is_finite());
}

#[test]
fn test_benjamini_hochberg() {
    let p_values = [0.01, 0.04, 0.03, 0.20];
    let result = benjamini_hochberg_correction(&p_values, 0.05);
    assert!(!result.rejected.is_empty());
}

#[test]
fn test_bonferroni() {
    let p_values = [0.01, 0.04, 0.03, 0.20];
    let result = bonferroni_correction(&p_values);
    assert_eq!(result.adjusted_p_values.len(), 4);
    assert!((result.adjusted_p_values[0] - 0.04).abs() < 1e-10);
}

#[test]
fn test_kendall_tau() {
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    let y = [1.0, 2.0, 3.0, 4.0, 5.0];
    let result = kendall_tau(&x, &y).unwrap();
    assert!((result.correlation - 1.0).abs() < 1e-10);
}

#[test]
fn test_ks_test() {
    let a = [1.0, 2.0, 3.0, 4.0, 5.0];
    let b = [1.1, 2.1, 3.1, 4.1, 5.1];
    let result = kolmogorov_smirnov(&a, &b).unwrap();
    assert!(result.statistic >= 0.0 && result.statistic <= 1.0);
}

#[test]
fn test_kaplan_meier() {
    let times = [1.0, 2.0, 3.0, 4.0, 5.0];
    let events = [true, false, true, false, true];
    let result = kaplan_meier(&times, &events).unwrap();
    assert!(!result.times.is_empty());
    assert!(result.survival[0] <= 1.0);
}

#[test]
fn test_multiple_regression() {
    let y: Vec<f64> = (1..=10).map(|i| 2.0 * i as f64 + 1.0).collect();
    let x: Vec<Vec<f64>> = (1..=10).map(|i| vec![i as f64]).collect();
    let result = multiple_linear_regression(&y, &x).unwrap();
    assert!((result.coefficients[1] - 2.0).abs() < 0.01);
    assert!((result.r_squared - 1.0).abs() < 0.01);
}
