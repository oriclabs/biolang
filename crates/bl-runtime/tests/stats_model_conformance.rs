//! Frozen numerical conformance for the GLM, random-intercept, and Cox fitters.
//!
//! `packages/statistics/validation/run.ps1` compares 147 metrics against R, but
//! it needs R installed and is run by hand, so it cannot fail a refactor. Every
//! other automated check on these fitters asserts shape -- record lengths,
//! flags, and loose bounds -- which leaves the arithmetic itself unguarded.
//!
//! This test closes that gap. The expected values are R output, produced by
//! `packages/statistics/validation/model_conformance.R` from fixtures small
//! enough to inline, so the check runs everywhere without R and without
//! redistributing an R dataset. Regenerate with:
//!
//! ```text
//! Rscript packages/statistics/validation/model_conformance.R
//! ```
//!
//! An expected value here is R's answer, not BioLang's. If one of these fails,
//! the fitter has moved away from the reference; do not re-derive the constant
//! from BioLang output.
//!
//! Two documented definitional differences from R, both measured rather than
//! assumed:
//!
//! * `hatvalues()` reads the QR that `glm.fit` built during the final IRLS
//!   iteration, so its weights belong to the previous coefficient vector.
//!   BioLang recomputes the hat matrix at the converged coefficients. The
//!   leverage constants below are therefore R's hat recomputed at R's own
//!   converged coefficients, which BioLang reproduces to 1e-9 or better;
//!   `hatvalues()` itself differs from both in the fifth or sixth significant
//!   digit. Cook's distance inherits that difference and is pinned loosely.
//! * `glm.fit` stops when the *deviance* stops changing, while this IRLS stops
//!   when the *coefficients* stop changing. Both use 1e-8, so the two fits
//!   settle at slightly different points and coefficients agree to roughly
//!   1e-8 rather than to machine precision.

use bl_core::value::{Table, Value};
use bl_runtime::stats::call_stats_builtin;
use std::collections::HashMap;

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

fn strings(values: &[&str]) -> Value {
    Value::List(
        values
            .iter()
            .map(|value| Value::Str((*value).into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn table(columns: &[(&str, &[f64])]) -> Value {
    let rows = (0..columns[0].1.len())
        .map(|row| {
            columns
                .iter()
                .map(|(_, values)| Value::Float(values[row]))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    Value::Table(Table::new(
        columns.iter().map(|(name, _)| (*name).into()).collect(),
        rows,
    ))
}

fn options(entries: &[(&str, Value)]) -> Value {
    Value::Record(
        entries
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect::<HashMap<_, _>>()
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

/// `coefficients` is a list of records carrying `estimate` and `standard_error`.
fn coefficient(report: &Value, index: usize, name: &str) -> f64 {
    let Value::List(values) = field(report, "coefficients") else {
        panic!("coefficients should be a List");
    };
    float_field(&values[index], name)
}

/// Compares against R on a relative scale, falling back to absolute near zero.
///
/// The tolerance on each call is the measured agreement with a decimal order of
/// headroom, so a real regression fails while last-bit reassociation does not.
fn assert_matches_r(label: &str, actual: f64, expected: f64, tolerance: f64) {
    let scale = expected.abs().max(1.0);
    let difference = (actual - expected).abs() / scale;
    assert!(
        difference <= tolerance,
        "{label}: BioLang {actual:.17e} vs R {expected:.17e} \
         (relative difference {difference:.3e} exceeds {tolerance:.3e})"
    );
}

const BINOMIAL_AGE: &[f64] = &[
    20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 44.0, 48.0, 52.0, 56.0, 60.0, 64.0, 68.0, 72.0, 76.0, 80.0,
];
const BINOMIAL_MARKER: &[f64] = &[
    0.2, 1.4, 0.4, 1.2, 0.7, 0.3, 1.1, 2.0, 0.5, 2.4, 1.9, 0.8, 2.2, 1.0, 2.6, 1.3,
];
const BINOMIAL_OUTCOME: &[f64] = &[
    0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0,
];

#[test]
fn binomial_glm_matches_r_glm_binomial() {
    let report = call_stats_builtin(
        "stats_glm_diagnostics",
        vec![
            table(&[("age", BINOMIAL_AGE), ("marker", BINOMIAL_MARKER)]),
            numbers(BINOMIAL_OUTCOME),
            options(&[("family", Value::Str("binomial".into()))]),
        ],
    )
    .unwrap();

    assert_eq!(field(&report, "converged"), &Value::Bool(true));
    assert_matches_r(
        "coef_intercept",
        coefficient(&report, 0, "estimate"),
        -2.2454406547669228e0,
        1e-7,
    );
    assert_matches_r(
        "coef_age",
        coefficient(&report, 1, "estimate"),
        5.4945268460166886e-3,
        1e-7,
    );
    assert_matches_r(
        "coef_marker",
        coefficient(&report, 2, "estimate"),
        1.9231151765736298e0,
        1e-7,
    );
    assert_matches_r(
        "null_deviance",
        float_field(&report, "null_deviance"),
        2.1930054632846662e1,
        1e-12,
    );
    assert_matches_r(
        "residual_deviance",
        float_field(&report, "residual_deviance"),
        1.6385023097338483e1,
        1e-10,
    );
    assert_matches_r(
        "aic",
        float_field(&report, "aic"),
        2.2385023097338483e1,
        1e-10,
    );
    assert_matches_r(
        "brier_score",
        float_field(&report, "brier_score"),
        1.7288439563483604e-1,
        1e-10,
    );
    // R's hat recomputed at R's converged coefficients; `hatvalues()` reports
    // 3.3545864506628092e-1 from the previous iteration's working weights.
    assert_matches_r(
        "maximum_leverage",
        float_field(&report, "maximum_leverage"),
        3.3547097324685587e-1,
        1e-7,
    );
    // Cook's distance is a function of leverage, so it carries the same
    // definitional gap against `cooks.distance()`; pinned as a review-threshold
    // regression guard rather than as a parity claim.
    assert_matches_r(
        "maximum_cook_distance",
        float_field(&report, "maximum_cook_distance"),
        3.7758564157597424e-1,
        1e-4,
    );
}

#[test]
fn poisson_glm_matches_r_glm_poisson() {
    let exposure: Vec<f64> = (1..=10).map(|value| value as f64).collect();
    let report = call_stats_builtin(
        "stats_glm_diagnostics",
        vec![
            table(&[("exposure", &exposure)]),
            numbers(&[0.0, 1.0, 1.0, 2.0, 2.0, 4.0, 3.0, 5.0, 7.0, 8.0]),
            options(&[("family", Value::Str("poisson".into()))]),
        ],
    )
    .unwrap();

    assert_eq!(field(&report, "converged"), &Value::Bool(true));
    assert_matches_r(
        "coef_intercept",
        coefficient(&report, 0, "estimate"),
        -7.2115187113613388e-1,
        1e-8,
    );
    assert_matches_r(
        "coef_exposure",
        coefficient(&report, 1, "estimate"),
        2.8932724609050026e-1,
        1e-8,
    );
    assert_matches_r(
        "null_deviance",
        float_field(&report, "null_deviance"),
        2.1036509024259427e1,
        1e-12,
    );
    assert_matches_r(
        "residual_deviance",
        float_field(&report, "residual_deviance"),
        2.1891379018001014e0,
        1e-10,
    );
    assert_matches_r(
        "aic",
        float_field(&report, "aic"),
        3.2900473292937193e1,
        1e-10,
    );
    assert_matches_r(
        "expected_poisson_zeros",
        float_field(&report, "expected_poisson_zeros"),
        1.6935796274403729e0,
        1e-9,
    );
    // R's hat recomputed at R's converged coefficients; `hatvalues()` reports
    // 5.3551389340694167e-1 from the previous iteration's working weights.
    assert_matches_r(
        "maximum_leverage",
        float_field(&report, "maximum_leverage"),
        5.3551501985247552e-1,
        1e-9,
    );
}

#[test]
fn random_intercept_model_matches_r_nlme_lme_reml() {
    let time: Vec<f64> = (0..6).flat_map(|_| [0.0, 1.0, 2.0, 3.0]).collect();
    let weight = [
        10.4, 12.1, 14.6, 15.9, 15.0, 17.9, 18.4, 21.2, 6.3, 8.1, 9.4, 12.2, 13.0, 15.8, 16.5,
        19.4, 8.7, 10.2, 12.4, 13.6, 17.0, 19.5, 20.6, 23.5,
    ];
    let clusters: Vec<&str> = ["a", "b", "c", "d", "e", "f"]
        .iter()
        .flat_map(|name| std::iter::repeat_n(*name, 4))
        .collect();

    let report = call_stats_builtin(
        "stats_random_intercept_model",
        vec![
            table(&[("time", &time)]),
            numbers(&weight),
            strings(&clusters),
            options(&[("method", Value::Str("reml".into()))]),
        ],
    )
    .unwrap();

    assert_eq!(field(&report, "clusters"), &Value::Int(6));
    let Value::List(fixed) = field(&report, "fixed_effects") else {
        panic!("fixed_effects should be a List");
    };
    assert_matches_r(
        "fixed_intercept",
        float_field(&fixed[0], "estimate"),
        1.1791666666666666e1,
        1e-8,
    );
    assert_matches_r(
        "fixed_time",
        float_field(&fixed[1], "estimate"),
        1.9083333333333332e0,
        1e-8,
    );
    // The REML profile is optimised by golden section rather than by nlme's
    // Newton step, so the variance components agree less tightly than the
    // fixed effects. These bounds are the measured agreement plus headroom.
    assert_matches_r(
        "random_intercept_variance",
        float_field(&report, "random_intercept_variance"),
        1.7996524507769415e1,
        1e-6,
    );
    assert_matches_r(
        "residual_variance",
        float_field(&report, "residual_variance"),
        2.2531862745844178e-1,
        1e-6,
    );
    assert_matches_r(
        "intraclass_correlation",
        float_field(&report, "intraclass_correlation"),
        9.8763469612890920e-1,
        1e-6,
    );
}

#[test]
fn cox_diagnostics_match_r_coxph_breslow() {
    let report = call_stats_builtin(
        "stats_cox_diagnostics",
        vec![
            numbers(&[
                5.0, 8.0, 9.0, 12.0, 15.0, 18.0, 20.0, 23.0, 26.0, 30.0, 32.0, 35.0,
            ]),
            numbers(&[1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0]),
            table(&[
                (
                    "age",
                    &[
                        51.0, 62.0, 57.0, 70.0, 45.0, 66.0, 54.0, 73.0, 49.0, 61.0, 58.0, 68.0,
                    ],
                ),
                (
                    "treatment",
                    &[0.0, 0.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 0.0],
                ),
            ]),
            options(&[("include_values", Value::Bool(true))]),
        ],
    )
    .unwrap();

    assert_eq!(field(&report, "converged"), &Value::Bool(true));
    assert_eq!(field(&report, "events"), &Value::Int(8));
    assert_eq!(field(&report, "ties"), &Value::Str("breslow".into()));
    assert_matches_r(
        "coef_age",
        coefficient(&report, 0, "estimate"),
        -1.5313047766941587e-1,
        1e-8,
    );
    assert_matches_r(
        "coef_treatment",
        coefficient(&report, 1, "estimate"),
        -2.0958584593349023e0,
        1e-8,
    );
    assert_matches_r(
        "se_age",
        coefficient(&report, 0, "standard_error"),
        8.1608131145383886e-2,
        1e-8,
    );
    assert_matches_r(
        "se_treatment",
        coefficient(&report, 1, "standard_error"),
        1.1074343093386418e0,
        1e-8,
    );
    assert_matches_r(
        "partial_log_likelihood",
        float_field(&report, "partial_log_likelihood"),
        -1.0236870412074124e1,
        1e-10,
    );
    assert_matches_r(
        "likelihood_ratio",
        float_field(&report, "likelihood_ratio"),
        6.3985274970887147e0,
        1e-9,
    );

    let Value::List(martingale) = field(&report, "martingale_residuals") else {
        panic!("martingale_residuals should be a List");
    };
    let sum_squares = martingale
        .iter()
        .map(|value| match value {
            Value::Float(value) => value * value,
            other => panic!("expected Float, got {other:?}"),
        })
        .sum::<f64>();
    assert_matches_r(
        "martingale_sum_squares",
        sum_squares,
        4.1056922239550637e0,
        1e-9,
    );
}

/// The 12-row binomial fixture in `packages/statistics/tests/exploration.bl` is
/// perfectly separable: R reports `glm.fit: algorithm did not converge` and
/// drives the coefficients toward infinity. IRLS used to return its final
/// iterate with no indication of that, which is why `converged` exists.
///
/// Only the flag and the deviance are pinned. A separated logistic fit has no
/// finite MLE, so its coefficients depend entirely on where iteration stops and
/// are not a reference quantity.
#[test]
fn separable_binomial_glm_reports_non_convergence_like_r() {
    let report = call_stats_builtin(
        "stats_glm_diagnostics",
        vec![
            table(&[
                (
                    "age",
                    &[
                        20.0, 24.0, 28.0, 32.0, 36.0, 40.0, 44.0, 48.0, 52.0, 56.0, 60.0, 64.0,
                    ],
                ),
                (
                    "marker",
                    &[0.2, 0.8, 0.4, 1.2, 0.7, 1.6, 1.1, 2.0, 1.5, 2.4, 1.9, 2.8],
                ),
            ]),
            numbers(&[0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
            options(&[("family", Value::Str("binomial".into()))]),
        ],
    )
    .unwrap();

    assert_eq!(field(&report, "converged"), &Value::Bool(false));
    assert!(float_field(&report, "residual_deviance") < 1e-6);

    let Value::List(issues) = field(&report, "issues") else {
        panic!("issues should be a List");
    };
    let ids = issues
        .iter()
        .filter_map(|value| match field(value, "id") {
            Value::Str(id) => Some(id.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        ids.iter().any(|id| id == "fit_not_converged"),
        "a non-converged fit must be disclosed, got issues {ids:?}"
    );
    assert!(
        field(&report, "ascii")
            .as_str()
            .expect("ascii should be a string")
            .contains("NOT CONVERGED"),
        "the rendered summary must carry the warning too"
    );
}
