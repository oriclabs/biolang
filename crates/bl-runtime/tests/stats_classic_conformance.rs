//! Frozen numerical conformance for the classic hypothesis tests.
//!
//! `stats_model_conformance.rs` pins the GLM, random-intercept and Cox fitters
//! to R. The classic tests had no equivalent, even though every one of them is
//! exactly computable. What they had instead were directional bounds:
//!
//! ```text
//! assert!(p > 0.2, "identical groups should have high p");
//! assert!(p < 0.1, "very different groups should have low p");
//! ```
//!
//! That catches a badly wrong p-value and not a systematically wrong one, which
//! is the failure mode that actually ships.
//!
//! Expected values are R output from
//! `packages/statistics/validation/classic_conformance.R`. Regenerate with:
//!
//! ```text
//! Rscript packages/statistics/validation/classic_conformance.R
//! ```
//!
//! Three of these pin a *variant* of R's function rather than its default,
//! because BioLang deliberately implements a different convention. Each is
//! named at the assertion. They are conventions, not defects -- but they were
//! undocumented, and a reader comparing against R would otherwise conclude
//! BioLang was wrong.

use bl_core::value::Value;
use bl_runtime::stats::call_stats_builtin;

fn numbers(values: &[f64]) -> Value {
    Value::List(
        values
            .iter()
            .copied()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    )
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record(record) = value else {
        panic!("expected Record, got {value:?}");
    };
    record
        .get(name)
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn float_field(value: &Value, name: &str) -> f64 {
    match field(value, name) {
        Value::Float(value) => *value,
        Value::Int(value) => *value as f64,
        other => panic!("expected numeric field {name}, got {other:?}"),
    }
}

fn assert_matches_r(label: &str, actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    let difference = (actual - expected).abs() / scale;
    assert!(
        difference <= tolerance,
        "{label}: BioLang {actual:.17e} vs R {expected:.17e} \
         (relative difference {difference:.3e} exceeds {tolerance:.3e})"
    );
}

#[test]
fn fisher_exact_matches_r_fisher_test() {
    let report = call_stats_builtin(
        "fisher_exact",
        vec![Value::Int(8), Value::Int(2), Value::Int(1), Value::Int(5)],
    )
    .unwrap();
    assert_matches_r(
        "p_value",
        float_field(&report, "p_value"),
        3.4965034965034968e-2,
        1e-12,
    );
    // R's fisher.test reports the conditional maximum likelihood odds ratio,
    // 15.469687462886908. This reports the sample odds ratio, (8*5)/(2*1).
    // Both are standard; they are not the same estimator.
    assert_matches_r(
        "odds_ratio (sample, not conditional MLE)",
        float_field(&report, "odds_ratio"),
        20.0,
        1e-12,
    );
}

#[test]
fn chi_square_matches_r_chisq_test() {
    let report = call_stats_builtin(
        "chi_square",
        vec![numbers(&[10.0, 20.0, 30.0]), numbers(&[20.0, 20.0, 20.0])],
    )
    .unwrap();
    assert_matches_r("statistic", float_field(&report, "chi2"), 10.0, 1e-12);
    assert_matches_r(
        "p_value",
        float_field(&report, "p_value"),
        6.7379469990854670e-3,
        1e-12,
    );
    assert_matches_r("df", float_field(&report, "df"), 2.0, 0.0);
}

#[test]
fn wilcoxon_matches_r_normal_approximation_without_continuity_correction() {
    let report = call_stats_builtin(
        "wilcoxon",
        vec![
            numbers(&[1.2, 2.4, 3.1, 4.8, 5.5]),
            numbers(&[2.0, 3.3, 4.1, 6.2, 7.9]),
        ],
    )
    .unwrap();
    assert_matches_r("statistic", float_field(&report, "statistic"), 8.0, 1e-12);
    // R defaults to the exact distribution at this sample size and reports
    // 0.42063492063492064. This uses the normal approximation without a
    // continuity correction, which is Scanpy's convention and the one
    // find_all_markers was matched to; the pinned value is
    // wilcox.test(exact = FALSE, correct = FALSE). The residual is the
    // normal CDF, not the test.
    assert_matches_r(
        "p_value (normal approximation, no continuity correction)",
        float_field(&report, "p_value"),
        3.4720763934942450e-1,
        1e-7,
    );
}

#[test]
fn anova_matches_r_aov() {
    let groups = Value::List(
        vec![
            numbers(&[1.0, 2.0, 3.0]),
            numbers(&[5.0, 6.0, 7.0]),
            numbers(&[9.0, 10.0, 11.0]),
        ]
        .into(),
    );
    let report = call_stats_builtin("anova", vec![groups]).unwrap();
    assert_matches_r(
        "f_statistic",
        float_field(&report, "f_statistic"),
        4.8000000000000043e1,
        1e-12,
    );
    assert_matches_r(
        "p_value",
        float_field(&report, "p_value"),
        2.0354162426216107e-4,
        1e-10,
    );
    assert_matches_r("df_between", float_field(&report, "df_between"), 2.0, 0.0);
    assert_matches_r("df_within", float_field(&report, "df_within"), 6.0, 0.0);
}

#[test]
fn ttest_matches_r_pooled_two_sample_t_test() {
    let report = call_stats_builtin(
        "ttest",
        vec![
            numbers(&[1.2, 2.4, 3.1, 4.8, 5.5]),
            numbers(&[2.0, 3.3, 4.1, 6.2, 7.9]),
        ],
    )
    .unwrap();
    // R's t.test defaults to Welch, which gives df = 7.3994685 and
    // p = 0.35286953859808035 here. This is the pooled (Student) form, so the
    // pinned values are t.test(var.equal = TRUE).
    assert_matches_r(
        "statistic",
        float_field(&report, "statistic"),
        -9.9124070716193036e-1,
        1e-12,
    );
    assert_matches_r(
        "p_value (pooled, not Welch)",
        float_field(&report, "p_value"),
        3.5059834433799048e-1,
        1e-11,
    );
    assert_matches_r("df", float_field(&report, "df"), 8.0, 0.0);
}

#[test]
fn p_adjust_matches_r_p_adjust() {
    let raw = numbers(&[0.01, 0.04, 0.03, 0.5, 0.2]);
    for (method, expected) in [
        (
            "bh",
            [
                5.0000000000000003e-2,
                6.6666666666666666e-2,
                6.6666666666666666e-2,
                5.0e-1,
                2.5e-1,
            ],
        ),
        (
            "bonferroni",
            [
                5.0000000000000003e-2,
                2.0000000000000001e-1,
                1.4999999999999999e-1,
                1.0,
                1.0,
            ],
        ),
    ] {
        let adjusted =
            call_stats_builtin("p_adjust", vec![raw.clone(), Value::Str(method.into())]).unwrap();
        let Value::List(values) = adjusted else {
            panic!("p_adjust should return a List");
        };
        assert_eq!(values.len(), expected.len());
        for (index, want) in expected.iter().enumerate() {
            let Value::Float(got) = values[index] else {
                panic!("expected Float at {index}");
            };
            assert_matches_r(&format!("{method}[{index}]"), got, *want, 1e-12);
        }
    }
}

#[test]
fn pearson_correlation_matches_r_cor() {
    let value = call_stats_builtin(
        "cor",
        vec![
            numbers(&[1.0, 2.0, 3.0, 4.0, 5.0]),
            numbers(&[2.0, 4.1, 5.9, 8.2, 9.8]),
        ],
    )
    .unwrap();
    let Value::Float(estimate) = value else {
        panic!("cor should return a Float, got {value:?}");
    };
    assert_matches_r("estimate", estimate, 9.9882964932988594e-1, 1e-12);
}
