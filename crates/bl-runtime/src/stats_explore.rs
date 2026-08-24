//! Explainable exploratory statistics.
//!
//! This module separates calculated facts from heuristic suggestions.  It does
//! not delete observations, transform data, or claim that a statistical test is
//! correct from distribution shape alone.  Public BioLang package functions in
//! `packages/statistics` provide the friendly API; the builtins here keep the
//! calculations fast, deterministic, and identical in native and WASM builds.

use crate::plot::{
    box_geometry, histogram_geometry, linear_fit_geometry, normal_qq_geometry, Scale, SvgCanvas,
};
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Table, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub(crate) fn call(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "stats_explore" => explore_numeric(args),
        "stats_compare" => compare_groups(args),
        "stats_relationship" => explore_relationship(args),
        "stats_categories" => explore_categories(args),
        "stats_guide" => add_guidance(args),
        "stats_explain" => explain(args),
        "stats_distribution_plot" => distribution_plot(args),
        "stats_distribution_ascii" => distribution_ascii(args),
        "stats_normal_diagram" => normal_diagram(args),
        "stats_visualize" => visualize_report(args),
        "stats_preprocess" => preprocessing_guide(args),
        "stats_profile" => profile_table(args),
        "stats_missingness" => missingness_report(args),
        "stats_design_check" => design_check(args),
        "stats_transform_preview" => transform_preview(args),
        "stats_uncertainty" => uncertainty_report(args),
        "stats_shape" => shape_diagnostics(args),
        "stats_normal_qq_plot" | "normal_qq_plot" => normal_qq_plot(args),
        "stats_group_plot" => group_diagnostic_plot(args),
        "stats_relationship_plot" => relationship_diagnostic_plot(args),
        "stats_categorical_plot" => categorical_diagnostic_plot(args),
        "stats_missingness_plot" => missingness_plot(args),
        "stats_normalization_guide" => normalization_guide(args),
        "stats_associations" => association_screen(args),
        "stats_scan" => scan_table(args),
        "stats_overview_ascii" => overview_ascii(args),
        "stats_linear_diagnostics" => linear_diagnostics(args),
        "stats_linear_diagnostic_plot" => linear_diagnostic_plot(args),
        "stats_report" => dataset_report(args),
        "stats_distribution_clues" => distribution_clues(args),
        "stats_multiple_linear_diagnostics" => multiple_linear_diagnostics(args),
        "stats_omics_profile" => omics_profile(args),
        "stats_robust_linear_diagnostics" => robust_linear_diagnostics(args),
        "stats_weighted_summary" => weighted_summary(args),
        "stats_time_series_diagnostics" => time_series_diagnostics(args),
        "stats_cluster_diagnostics" => cluster_diagnostics(args),
        "stats_means" => means_guide(args),
        "stats_decision_map" => decision_map(args),
        "stats_glm_diagnostics" => glm_diagnostics(args),
        "stats_random_intercept_model" => random_intercept_model(args),
        "stats_cox_diagnostics" => cox_diagnostics(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown exploratory statistics builtin '{name}'"),
            None,
        )),
    }
}

fn text(value: impl Into<String>) -> Value {
    Value::Str(value.into())
}

fn list(values: Vec<Value>) -> Value {
    Value::List(values.into())
}

fn record(entries: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    let map = entries
        .into_iter()
        .map(|(key, value)| (key.into(), value))
        .collect::<HashMap<_, _>>();
    Value::Record(map.into())
}

fn string_list(values: impl IntoIterator<Item = impl Into<String>>) -> Value {
    list(values.into_iter().map(|value| text(value.into())).collect())
}

fn options(args: &[Value], index: usize, function: &str) -> Result<HashMap<String, Value>> {
    match args.get(index) {
        None | Some(Value::Nil) => Ok(HashMap::new()),
        Some(Value::Record(map)) => Ok(map.as_ref().clone()),
        Some(other) => Err(BioLangError::type_error(
            format!(
                "{function}() options must be Record, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

#[derive(Clone)]
struct NumericData {
    values: Vec<f64>,
    original_indices: Vec<usize>,
    total: usize,
    missing: usize,
    non_finite: usize,
}

fn numeric_data(value: &Value, function: &str) -> Result<NumericData> {
    let Value::List(items) = value else {
        return Err(BioLangError::type_error(
            format!("{function}() requires List, got {}", value.type_of()),
            None,
        ));
    };

    let mut values = Vec::with_capacity(items.len());
    let mut original_indices = Vec::with_capacity(items.len());
    let mut missing = 0usize;
    let mut non_finite = 0usize;
    for (index, item) in items.iter().enumerate() {
        let number = match item {
            Value::Nil => {
                missing += 1;
                continue;
            }
            Value::Int(number) => *number as f64,
            Value::Float(number) => *number,
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "{function}() values must be numeric or Nil; index {index} is {}",
                        other.type_of()
                    ),
                    None,
                ))
            }
        };
        if number.is_finite() {
            values.push(number);
            original_indices.push(index);
        } else {
            non_finite += 1;
        }
    }
    if values.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() has no finite numeric observations"),
            None,
        ));
    }
    Ok(NumericData {
        values,
        original_indices,
        total: items.len(),
        missing,
        non_finite,
    })
}

fn quantile_sorted(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
}

fn median(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    quantile_sorted(&sorted, 0.5)
}

fn sample_skewness(values: &[f64], mean: f64, sample_sd: f64) -> Option<f64> {
    let n = values.len();
    if n < 3 || sample_sd <= f64::EPSILON {
        return None;
    }
    let standardized_cubes = values
        .iter()
        .map(|value| ((value - mean) / sample_sd).powi(3))
        .sum::<f64>();
    Some(n as f64 * standardized_cubes / ((n - 1) * (n - 2)) as f64)
}

#[derive(Clone)]
struct NumericSummary {
    n: usize,
    min: f64,
    max: f64,
    mean: f64,
    median: f64,
    q1: f64,
    q3: f64,
    /// Absent below two observations. The sample form divides by n - 1, so a
    /// single observation has no variance to report; returning zero would say
    /// the data has no spread when what is true is that its spread is unknown,
    /// and `suggestion.spread` would then recommend quoting it.
    variance: Option<f64>,
    sd: Option<f64>,
    iqr: f64,
    mad: f64,
    skewness: Option<f64>,
    lower_fence: f64,
    upper_fence: f64,
    outlier_positions: Vec<usize>,
    unique: usize,
    zero_count: usize,
    negative_count: usize,
    mode: Option<(f64, usize)>,
}

fn summarize(data: &NumericData) -> NumericSummary {
    let n = data.values.len();
    let mut sorted = data.values.clone();
    sorted.sort_by(f64::total_cmp);
    let mean = data.values.iter().sum::<f64>() / n as f64;
    let variance = (n > 1).then(|| {
        data.values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (n - 1) as f64
    });
    let sd = variance.map(f64::sqrt);
    let median_value = quantile_sorted(&sorted, 0.5);
    let q1 = quantile_sorted(&sorted, 0.25);
    let q3 = quantile_sorted(&sorted, 0.75);
    let iqr = q3 - q1;
    let deviations = data
        .values
        .iter()
        .map(|value| (value - median_value).abs())
        .collect::<Vec<_>>();
    let mad = median(&deviations);
    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;
    let outlier_positions = data
        .values
        .iter()
        .enumerate()
        .filter_map(|(position, value)| {
            (*value < lower_fence || *value > upper_fence).then_some(position)
        })
        .collect::<Vec<_>>();

    let mut frequencies: HashMap<u64, usize> = HashMap::new();
    for value in &data.values {
        let normalized = if *value == 0.0 { 0.0 } else { *value };
        *frequencies.entry(normalized.to_bits()).or_default() += 1;
    }
    let max_frequency = frequencies.values().copied().max().unwrap_or(0);
    let mode = if max_frequency > 1 {
        frequencies
            .into_iter()
            .filter(|(_, count)| *count == max_frequency)
            .map(|(bits, count)| (f64::from_bits(bits), count))
            .min_by(|left, right| left.0.total_cmp(&right.0))
    } else {
        None
    };

    NumericSummary {
        n,
        min: sorted[0],
        max: sorted[n - 1],
        mean,
        median: median_value,
        q1,
        q3,
        variance,
        sd,
        iqr,
        mad,
        skewness: sd.and_then(|sd| sample_skewness(&data.values, mean, sd)),
        lower_fence,
        upper_fence,
        outlier_positions,
        unique: sorted
            .windows(2)
            .filter(|pair| pair[0].to_bits() != pair[1].to_bits())
            .count()
            + 1,
        zero_count: data.values.iter().filter(|value| **value == 0.0).count(),
        negative_count: data.values.iter().filter(|value| **value < 0.0).count(),
        mode,
    }
}

fn number(value: Option<f64>) -> Value {
    value.map(Value::Float).unwrap_or(Value::Nil)
}

fn fmt_number(value: f64) -> String {
    let magnitude = value.abs();
    if magnitude != 0.0 && (magnitude >= 1_000_000.0 || magnitude < 0.001) {
        format!("{value:.4e}")
    } else {
        let formatted = format!("{value:.4}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn shape_label(summary: &NumericSummary) -> (&'static str, &'static str) {
    if (summary.max - summary.min).abs() <= f64::EPSILON {
        return ("constant", "All usable observations have the same value.");
    }
    let Some(skewness) = summary.skewness else {
        return (
            "not_assessed",
            "There are too few observations to assess asymmetry reliably.",
        );
    };
    match skewness {
        value if value >= 1.0 => (
            "strong_right_skew",
            "The right tail is much longer than the left tail.",
        ),
        value if value >= 0.5 => (
            "moderate_right_skew",
            "The right tail is longer than the left tail.",
        ),
        value if value <= -1.0 => (
            "strong_left_skew",
            "The left tail is much longer than the right tail.",
        ),
        value if value <= -0.5 => (
            "moderate_left_skew",
            "The left tail is longer than the right tail.",
        ),
        _ => (
            "roughly_symmetric",
            "No strong asymmetry is visible in the skewness summary.",
        ),
    }
}

fn approach(name: &str, use_when: &str, limitation: &str, fit: &str) -> Value {
    record([
        ("name", text(name)),
        ("use_when", text(use_when)),
        ("limitation_here", text(limitation)),
        ("fit", text(fit)),
    ])
}

fn issue(
    id: &str,
    observation: impl Into<String>,
    evidence: impl Into<String>,
    level: &str,
) -> Value {
    record([
        ("id", text(id)),
        ("observation", text(observation.into())),
        ("evidence", text(evidence.into())),
        ("level", text(level)),
        ("is_diagnosis", Value::Bool(false)),
    ])
}

fn preprocessing_option(
    name: &str,
    status: &str,
    useful_when: &str,
    changes: &str,
    caution: &str,
    code: &str,
) -> Value {
    record([
        ("name", text(name)),
        ("status", text(status)),
        ("useful_when", text(useful_when)),
        ("changes", text(changes)),
        ("caution", text(caution)),
        ("example", text(code)),
        ("automatically_applied", Value::Bool(false)),
    ])
}

fn preprocessing_record(data: &NumericData, summary: &NumericSummary, data_type: &str) -> Value {
    let zero_fraction = summary.zero_count as f64 / summary.n as f64;
    let non_integer_count = data
        .values
        .iter()
        .filter(|value| (**value - value.round()).abs() > 1e-10)
        .count();
    let mut issues = Vec::new();
    if data.missing > 0 {
        issues.push(issue(
            "missing_values",
            format!("{} Nil value(s) are present.", data.missing),
            "Missingness can bias an analysis when it depends on group, outcome, or measurement quality.",
            "review",
        ));
    }
    if data.non_finite > 0 {
        issues.push(issue(
            "non_finite_values",
            format!("{} NaN or infinite value(s) are present.", data.non_finite),
            "They were excluded from numerical summaries and their origin should be traced.",
            "review",
        ));
    }
    if (summary.max - summary.min).abs() <= f64::EPSILON {
        issues.push(issue(
            "constant_data",
            "All usable observations are identical.",
            "Variance is zero, so scaling, correlation, and many model coefficients are undefined.",
            "blocking",
        ));
    }
    if !summary.outlier_positions.is_empty() {
        issues.push(issue(
            "distant_observations",
            format!(
                "{} observation(s) lie outside the Tukey fences.",
                summary.outlier_positions.len()
            ),
            "These may be genuine biology, another subgroup, a scale effect, or measurement/data-entry problems.",
            "review",
        ));
    }
    if summary.skewness.is_some_and(|value| value.abs() >= 1.0) {
        issues.push(issue(
            "strong_asymmetry",
            "The distribution has strong sample skewness.",
            format!(
                "Adjusted skewness is {}.",
                summary.skewness.map(fmt_number).unwrap_or_default()
            ),
            "review",
        ));
    }
    if zero_fraction >= 0.20 {
        issues.push(issue(
            "many_zeros",
            format!(
                "{} of {} usable values are zero ({}%).",
                summary.zero_count,
                summary.n,
                fmt_number(zero_fraction * 100.0)
            ),
            "This is a descriptive zero fraction, not proof of a zero-inflated probability model.",
            "review",
        ));
    }
    if summary.n >= 20 && summary.unique <= 10 {
        issues.push(issue(
            "low_resolution_or_heaping",
            format!(
                "{} observations contain only {} distinct values.",
                summary.n, summary.unique
            ),
            "This can be expected for ordinal/count data or can reflect rounding, detection limits, or data entry conventions.",
            "clue",
        ));
    }
    let positive_min = data
        .values
        .iter()
        .copied()
        .filter(|value| *value > 0.0)
        .min_by(f64::total_cmp);
    if let Some(minimum) = positive_min {
        if summary.max / minimum > 100.0 {
            issues.push(issue(
                "wide_positive_range",
                "Positive observations span more than a 100-fold range.",
                format!(
                    "Largest / smallest positive value = {}.",
                    fmt_number(summary.max / minimum)
                ),
                "clue",
            ));
        }
    }
    match data_type {
        "count" | "counts" => {
            if summary.negative_count > 0 || non_integer_count > 0 {
                issues.push(issue(
                    "count_contract_mismatch",
                    "Values labelled as counts include negative or non-integer observations.",
                    format!(
                        "{} negative and {} non-integer usable value(s).",
                        summary.negative_count, non_integer_count
                    ),
                    "blocking",
                ));
            }
        }
        "proportion" | "probability" => {
            let outside = data
                .values
                .iter()
                .filter(|value| **value < 0.0 || **value > 1.0)
                .count();
            if outside > 0 {
                issues.push(issue(
                    "bounded_contract_mismatch",
                    format!("{outside} value(s) lie outside 0 to 1."),
                    "Values labelled as proportions or probabilities should normally be bounded by 0 and 1.",
                    "blocking",
                ));
            }
        }
        _ => {}
    }

    let robust = summary.skewness.is_some_and(|value| value.abs() >= 0.5)
        || !summary.outlier_positions.is_empty();
    let mut suggestions = vec![preprocessing_option(
        "keep original scale",
        if robust {
            "alternative"
        } else {
            "preferred_start"
        },
        "The original units are interpretable and model diagnostics are acceptable.",
        "Nothing; values and units remain unchanged.",
        "A model may still need another variance structure or distribution.",
        "values",
    )];
    if summary.min > 0.0 && summary.skewness.is_some_and(|value| value >= 0.75) {
        suggestions.push(preprocessing_option(
            "log transform",
            "candidate",
            "Values are positive, changes are multiplicative, and right-skew is scientifically meaningful.",
            "Ratios become differences and large values are compressed.",
            "Invalid for zero/negative values and changes the estimand; do not use it only to obtain significance.",
            "values |> map(|x| log(x))",
        ));
    }
    if summary.min >= 0.0
        && summary.zero_count > 0
        && summary.skewness.is_some_and(|value| value >= 0.75)
    {
        suggestions.push(preprocessing_option(
            "log1p transform",
            "candidate",
            "Non-negative values contain zeros and have a long right tail.",
            "Computes log(1 + x), retaining zero while compressing large values.",
            "The added 1 is scale-dependent and may be inappropriate for continuous measurements below one.",
            "values |> map(|x| log(1.0 + x))",
        ));
    }
    if summary.sd.is_some_and(|sd| sd > f64::EPSILON) {
        suggestions.push(preprocessing_option(
            "z-score standardization",
            "task_dependent",
            "Variables with different units must contribute comparably to PCA, clustering, or regularized models.",
            "Subtracts the mean and divides by SD; resulting mean is zero and SD is one.",
            "Does not repair skewness or outliers, removes original units, and is unnecessary for many single-variable tests.",
            "values |> normalize(\"zscore\")",
        ));
    }
    if robust && summary.mad > f64::EPSILON {
        suggestions.push(preprocessing_option(
            "robust standardization",
            "candidate_for_distance_methods",
            "Distance-based methods need comparable scales but mean/SD are strongly influenced by distant values.",
            "Subtracts the median and divides by MAD (often with a consistency factor).",
            "Changes units, does not resolve hidden groups, and must use one documented MAD convention.",
            "values |> map(|x| (x - median(values)) / (1.4826 * report.summary.mad))",
        ));
    }
    suggestions.push(preprocessing_option(
        "min-max scaling",
        "alternative",
        "An algorithm explicitly requires a fixed numerical interval.",
        "Maps the observed minimum to zero and maximum to one.",
        "Highly sensitive to extremes and future observations can fall outside the fitted interval.",
        "normalize(values, \"minmax\")",
    ));
    if matches!(data_type, "count" | "counts") {
        suggestions.push(preprocessing_option(
            "library-size or exposure normalization",
            "requires_more_data",
            "Counts come from samples with different sequencing depth, exposure time, or opportunity.",
            "Uses a sample-level denominator or model offset before comparing rates.",
            "A single count vector is insufficient; provide the sample-by-feature layout and valid exposure/library totals.",
            "# supply the count matrix and sample-level exposures first",
        ));
    }
    if matches!(data_type, "proportion" | "probability") && summary.min > 0.0 && summary.max < 1.0 {
        suggestions.push(preprocessing_option(
            "logit transform",
            "model_dependent",
            "A method models an unbounded transform of proportions strictly between zero and one.",
            "Maps (0, 1) to the real line using log(p / (1 - p)).",
            "Undefined at zero and one; binomial models are often preferable when numerators and denominators are available.",
            "values |> map(|p| log(p / (1.0 - p)))",
        ));
    }

    let issue_lines = issues
        .iter()
        .filter_map(|value| match value {
            Value::Record(item) => item.get("observation").and_then(Value::as_str),
            _ => None,
        })
        .map(|observation| format!("  - {observation}"))
        .collect::<Vec<_>>();
    let suggestion_lines = suggestions
        .iter()
        .filter_map(|value| match value {
            Value::Record(item) => {
                Some((item.get("name")?.as_str()?, item.get("status")?.as_str()?))
            }
            _ => None,
        })
        .map(|(name, status)| format!("  - {name} [{status}]"))
        .collect::<Vec<_>>();
    let explanation = format!(
        "Preprocessing guidance\n\nDeclared data type\n  {data_type}\n\nObservable issues\n{}\n\nOptions (none applied)\n{}\n\nImportant\n  Normalization is chosen for a measurement process and analysis goal, not because one curve looks more normal. Missing values, batch effects, exposure/library sizes, and the experimental unit need explicit context.",
        if issue_lines.is_empty() {
            "  No issue crossed the current descriptive clue thresholds.".into()
        } else {
            issue_lines.join("\n")
        },
        suggestion_lines.join("\n"),
    );

    record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("preprocessing")),
        ("data_type", text(data_type)),
        ("issues", list(issues)),
        ("suggestions", list(suggestions)),
        ("automatic_changes", Value::Bool(false)),
        (
            "missingness_policy",
            text("Nil and non-finite values are disclosed and excluded from summaries; no imputation is performed."),
        ),
        (
            "normalization_note",
            text("Normalization is chosen for a measurement process and analysis goal, not because one curve looks more normal."),
        ),
        (
            "required_context",
            string_list([
                "measurement type and units",
                "experimental unit",
                "sample, feature, and replicate axes",
                "known bounds or detection limits",
                "batch and exposure/library-size variables",
                "scientific estimand and downstream method",
            ]),
        ),
        (
            "quick_explanation",
            text("Preprocessing clues and alternatives were calculated; no changes were applied."),
        ),
        ("explanation", text(explanation)),
    ])
}

fn numeric_report(data: NumericData, variable: &str, data_type: &str) -> Value {
    let summary = summarize(&data);
    let (shape, shape_explanation) = shape_label(&summary);
    let robust = summary.skewness.is_some_and(|value| value.abs() >= 0.5)
        || !summary.outlier_positions.is_empty();
    let center_name = if robust { "median" } else { "mean" };
    // A single observation has no sample spread of any kind, so recommending
    // one would ask the reader to quote a number that does not exist. The IQR
    // of one value is zero and the SD is undefined; neither describes anything.
    let spread_name = match (summary.sd, robust) {
        (None, _) => "none: a single observation has no spread to report",
        (Some(_), true) => "IQR",
        (Some(_), false) => "standard deviation",
    };
    let center_reason = if robust {
        "The median is less affected by asymmetry and unusually distant observations."
    } else {
        "The distribution has no strong asymmetry or Tukey outlier flags."
    };
    let spread_reason = match (summary.sd, robust) {
        (None, _) => {
            "Sample spread needs at least two observations; the sample variance divides by n - 1."
        }
        (Some(_), true) => "The IQR describes the middle half without depending on the mean.",
        (Some(_), false) => {
            "Standard deviation describes distance from the mean in the original units."
        }
    };

    let outliers = summary
        .outlier_positions
        .iter()
        .map(|position| {
            let value = data.values[*position];
            let side = if value < summary.lower_fence { "below" } else { "above" };
            record([
                ("index", Value::Int(data.original_indices[*position] as i64)),
                ("value", Value::Float(value)),
                (
                    "reason",
                    text(format!(
                        "Value is {side} the 1.5 x IQR fence; this is a review flag, not evidence of error."
                    )),
                ),
            ])
        })
        .collect::<Vec<_>>();

    let mut clues = vec![record([
        ("id", text("shape")),
        ("observation", text(shape_explanation)),
        (
            "evidence",
            text(match summary.skewness {
                Some(value) => format!("Adjusted sample skewness = {}.", fmt_number(value)),
                None => "Skewness was not calculated for fewer than three observations.".into(),
            }),
        ),
        (
            "certainty",
            text(if summary.n < 20 {
                "limited"
            } else {
                "descriptive"
            }),
        ),
    ])];
    if summary.sd.is_none() {
        clues.push(record([
            ("id", text("single_observation")),
            (
                "observation",
                text("Only one observation was used, so this variable has no measurable spread."),
            ),
            (
                "evidence",
                text(
                    "Sample variance divides by n - 1, which is zero here, so variance and \
                     standard deviation are reported as absent rather than as zero.",
                ),
            ),
            ("certainty", text("flag_only")),
        ]));
    }
    if !outliers.is_empty() {
        clues.push(record([
            ("id", text("possible_outliers")),
            (
                "observation",
                text(format!(
                    "{} observation(s) fall outside the Tukey fences.",
                    outliers.len()
                )),
            ),
            (
                "evidence",
                text(format!(
                    "Lower fence = {}; upper fence = {}.",
                    fmt_number(summary.lower_fence),
                    fmt_number(summary.upper_fence)
                )),
            ),
            ("certainty", text("flag_only")),
        ]));
    }
    if data.missing + data.non_finite > 0 {
        clues.push(record([
            ("id", text("excluded_values")),
            (
                "observation",
                text(format!(
                    "{} value(s) were excluded from numerical calculations.",
                    data.missing + data.non_finite
                )),
            ),
            (
                "evidence",
                text(format!(
                    "{} Nil and {} non-finite value(s).",
                    data.missing, data.non_finite
                )),
            ),
            ("certainty", text("exact")),
        ]));
    }

    let mean_limit = if robust {
        "Sensitive to the observed asymmetry or flagged values."
    } else {
        "Can still conceal separate biological subgroups."
    };
    let median_limit = if robust {
        "Does not describe how far the tails extend."
    } else {
        "Uses less information about distances between observations."
    };
    let mut transformations = Vec::new();
    if summary.min > 0.0 && summary.skewness.is_some_and(|value| value >= 0.75) {
        let logged = data
            .values
            .iter()
            .map(|value| value.ln())
            .collect::<Vec<_>>();
        let log_data = NumericData {
            original_indices: (0..logged.len()).collect(),
            total: logged.len(),
            missing: 0,
            non_finite: 0,
            values: logged,
        };
        let log_summary = summarize(&log_data);
        transformations.push(record([
            ("name", text("log")),
            ("status", text("candidate_not_applied")),
            (
                "reason",
                text("All usable values are positive and the original scale is right-skewed."),
            ),
            ("original_skewness", number(summary.skewness)),
            ("transformed_skewness", number(log_summary.skewness)),
            (
                "caution",
                text("Use a log scale only when ratios or multiplicative changes are scientifically meaningful."),
            ),
        ]));
    }

    let preprocessing = preprocessing_record(&data, &summary, data_type);
    let centre_spread_guidance = means_guide(vec![list(
        data.values.iter().copied().map(Value::Float).collect(),
    )])
    .expect("a validated numeric vector always produces centre/spread guidance");
    let visual_guide = centre_spread_visual(&data, &summary, center_name, spread_name);
    let preprocessing_issue_count = match &preprocessing {
        Value::Record(record) => match record.get("issues") {
            Some(Value::List(items)) => items.len(),
            _ => 0,
        },
        _ => 0,
    };
    let quick = format!(
        "{}: {} usable observation(s). Suggested summary: {} with {}. {}",
        variable, summary.n, center_name, spread_name, shape_explanation
    );
    let explanation = format!(
        "Exploratory statistics: {variable}\n\nData used\n  Received: {}\n  Finite numeric observations: {}\n  Nil values: {}\n  Non-finite values: {}\n\nShape\n  {}\n  Skewness: {}\n\nCentre\n  Mean: {}\n  Median: {}\n  Suggested: {}\n  Why: {}\n\nSpread\n  Standard deviation: {}\n  Variance: {}\n  IQR: {} (Q1 {} to Q3 {})\n  MAD: {}\n  Suggested: {}\n  Why: {}\n\nPossible outliers\n  {} Tukey-fence flag(s). A flag is a reason to inspect an observation, not delete it.\n\nPreprocessing\n  {} observable issue clue(s). See report.preprocessing for alternatives; no changes were applied.\n\nLimit\n  Distribution summaries cannot establish independence, pairing, biological groups, or the experimental unit.",
        data.total,
        summary.n,
        data.missing,
        data.non_finite,
        shape_explanation,
        summary.skewness.map(fmt_number).unwrap_or_else(|| "not assessed".into()),
        fmt_number(summary.mean),
        fmt_number(summary.median),
        center_name,
        center_reason,
        summary
            .sd
            .map(fmt_number)
            .unwrap_or_else(|| "not defined".into()),
        summary
            .variance
            .map(fmt_number)
            .unwrap_or_else(|| "not defined".into()),
        fmt_number(summary.iqr),
        fmt_number(summary.q1),
        fmt_number(summary.q3),
        fmt_number(summary.mad),
        spread_name,
        spread_reason,
        outliers.len(),
        preprocessing_issue_count,
    );

    record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("numeric")),
        ("variable", text(variable)),
        (
            "data",
            record([
                ("received", Value::Int(data.total as i64)),
                ("used", Value::Int(summary.n as i64)),
                ("missing", Value::Int(data.missing as i64)),
                ("non_finite", Value::Int(data.non_finite as i64)),
                ("calculations_use_all_finite_values", Value::Bool(true)),
                ("unique", Value::Int(summary.unique as i64)),
                ("zeros", Value::Int(summary.zero_count as i64)),
                ("negative", Value::Int(summary.negative_count as i64)),
            ]),
        ),
        (
            "summary",
            record([
                ("min", Value::Float(summary.min)),
                ("q1", Value::Float(summary.q1)),
                ("median", Value::Float(summary.median)),
                ("q3", Value::Float(summary.q3)),
                ("max", Value::Float(summary.max)),
                ("mean", Value::Float(summary.mean)),
                ("variance", number(summary.variance)),
                ("sd", number(summary.sd)),
                ("iqr", Value::Float(summary.iqr)),
                ("mad", Value::Float(summary.mad)),
                ("mad_normal_consistent", Value::Float(summary.mad * 1.4826)),
                ("skewness", number(summary.skewness)),
                (
                    "mode",
                    summary
                        .mode
                        .map(|mode| Value::Float(mode.0))
                        .unwrap_or(Value::Nil),
                ),
                (
                    "mode_count",
                    Value::Int(summary.mode.map(|mode| mode.1).unwrap_or(0) as i64),
                ),
            ]),
        ),
        (
            "shape",
            record([
                ("label", text(shape)),
                ("explanation", text(shape_explanation)),
                ("is_diagnostic", Value::Bool(false)),
            ]),
        ),
        (
            "suggestion",
            record([
                ("center", text(center_name)),
                ("center_reason", text(center_reason)),
                ("spread", text(spread_name)),
                ("spread_reason", text(spread_reason)),
                ("is_heuristic", Value::Bool(true)),
            ]),
        ),
        ("clues", list(clues)),
        ("outliers", list(outliers)),
        (
            "outlier_rule",
            record([
                ("name", text("Tukey 1.5 x IQR fences")),
                ("lower", Value::Float(summary.lower_fence)),
                ("upper", Value::Float(summary.upper_fence)),
                ("action", text("inspect, do not automatically remove")),
            ]),
        ),
        (
            "alternatives",
            list(vec![
                approach(
                    "mean + standard deviation",
                    "The variable is reasonably symmetric and a mean has scientific meaning.",
                    mean_limit,
                    if robust { "alternative" } else { "suggested" },
                ),
                approach(
                    "median + IQR",
                    "The variable is skewed or contains influential observations.",
                    median_limit,
                    if robust { "suggested" } else { "alternative" },
                ),
                approach(
                    "median + MAD",
                    "A robust distance measure is useful for automated comparisons.",
                    "MAD is less familiar and should be defined when reported.",
                    "alternative",
                ),
                approach(
                    "quantiles or full distribution",
                    "One centre would conceal important tails or subgroups.",
                    "Requires a longer presentation than one summary pair.",
                    "always_available",
                ),
            ]),
        ),
        ("transformations", list(transformations)),
        ("centre_spread_guidance", centre_spread_guidance),
        ("visual_guide", visual_guide),
        ("preprocessing", preprocessing),
        ("quick_explanation", text(quick)),
        ("explanation", text(explanation)),
        (
            "limitations",
            string_list([
                "Shape labels are descriptive clues, not distribution tests.",
                "Tukey fences flag observations but do not identify errors.",
                "Study design cannot be inferred from a numeric vector.",
                "Use stats_shape() for bin-width-sensitive multiple-peak evidence; it does not diagnose biological populations.",
            ]),
        ),
    ])
}

fn centre_spread_visual(
    data: &NumericData,
    summary: &NumericSummary,
    center_name: &str,
    spread_name: &str,
) -> Value {
    let range = (summary.max - summary.min).abs();
    let position = |value: f64| {
        if range <= f64::EPSILON {
            50.0
        } else {
            (5.0 + 90.0 * (value - summary.min) / range).clamp(5.0, 95.0)
        }
    };
    let mean_x = position(summary.mean);
    let median_x = position(summary.median);
    let q1_x = position(summary.q1);
    let q3_x = position(summary.q3);
    // With fewer than two observations there is no SD, so the band collapses
    // onto the mean rather than being drawn as a measured spread of nothing.
    let spread = summary.sd.unwrap_or(0.0);
    let sd_low_x = position(summary.mean - spread);
    let sd_high_x = position(summary.mean + spread);
    let normal_references = [0.6827, 0.9545, 0.9973];
    let sd_bands = summary
        .sd
        .map(|sd| {
            (1..=3)
                .map(|multiple| {
                    let distance = multiple as f64 * sd;
                    let lower = summary.mean - distance;
                    let upper = summary.mean + distance;
                    let count = data
                        .values
                        .iter()
                        .filter(|value| **value >= lower && **value <= upper)
                        .count();
                    record([
                        ("multiple", Value::Int(multiple as i64)),
                        ("lower", Value::Float(lower)),
                        ("upper", Value::Float(upper)),
                        ("observed_count", Value::Int(count as i64)),
                        (
                            "observed_proportion",
                            Value::Float(count as f64 / summary.n as f64),
                        ),
                        (
                            "normal_reference_proportion",
                            Value::Float(normal_references[multiple - 1]),
                        ),
                    ])
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let raw_skew = summary.skewness;
    let log_summary = if summary.min > 0.0 {
        let logged = data
            .values
            .iter()
            .map(|value| value.ln())
            .collect::<Vec<_>>();
        Some(summarize(&NumericData {
            original_indices: (0..logged.len()).collect(),
            total: logged.len(),
            missing: 0,
            non_finite: 0,
            values: logged,
        }))
    } else {
        None
    };
    let scale_clue = match (&log_summary, raw_skew) {
        (Some(logged), Some(raw)) if raw.abs() >= 0.75 => format!(
            "A log preview is eligible because values are positive. Raw skewness {}; log skewness {}. Use it only for a ratio or multiplicative question.",
            fmt_number(raw),
            logged
                .skewness
                .map(fmt_number)
                .unwrap_or_else(|| "not assessed".into())
        ),
        (Some(_), _) => "Values are positive, so a log preview is possible, but shape alone is not a reason to transform them.".into(),
        (None, _) => "A plain log transform is not defined for these observed values because at least one value is zero or negative.".into(),
    };
    let band_summary = summary
        .sd
        .map(|sd| {
            (1..=3)
                .map(|multiple| {
                    let distance = multiple as f64 * sd;
                    let count = data
                        .values
                        .iter()
                        .filter(|value| {
                            **value >= summary.mean - distance && **value <= summary.mean + distance
                        })
                        .count();
                    format!(
                        "{multiple} SD: {:.1}% observed",
                        100.0 * count as f64 / summary.n as f64
                    )
                })
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_else(|| "SD bands need at least two observations".into());
    let ascii = format!(
        "CENTRE + SPREAD + SCALE\n\nrange     {}  |------------------------------|  {}\nIQR                 Q1 ===== median ===== Q3\nmean/SD          mean-SD <---- mean ----> mean+SD\n\nSuggested descriptive pair: {center_name} + {spread_name}\nSD is a typical distance from the mean. Variance = SD x SD; it is in squared units, not a second or whole width.\nObserved coverage: {band_summary}.\nThe 68% / 95% / 99.7% rule is a normal-distribution reference, not an outlier test.\n{scale_clue}\nUncertainty is separate: use an interval/SE that respects the sampling design.",
        fmt_number(summary.min),
        fmt_number(summary.max),
    );
    let svg = format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 760 220" role="img" aria-labelledby="cst csd"><title id="cst">Centre, spread, and scale guide</title><desc id="csd">The mean with one standard deviation and the median with the interquartile range on the observed scale.</desc><style>.t{{font:14px system-ui;fill:#172033}}.s{{font:12px system-ui;fill:#455468}}.axis{{stroke:#64748b;stroke-width:2}}.iqr{{stroke:#0f766e;stroke-width:12;stroke-linecap:round}}.sd{{stroke:#2563eb;stroke-width:5;stroke-linecap:round}}.mark{{stroke:#172033;stroke-width:2}}</style><text class="t" x="20" y="26">Centre + spread on the observed scale</text><line class="axis" x1="38" y1="80" x2="722" y2="80"/><line class="sd" x1="{sd_low}" y1="72" x2="{sd_high}" y2="72"/><line class="mark" x1="{mean}" y1="58" x2="{mean}" y2="88"/><text class="s" x="{mean}" y="52" text-anchor="middle">mean</text><text class="s" x="38" y="102">mean - SD</text><text class="s" x="722" y="102" text-anchor="end">mean + SD</text><line class="iqr" x1="{q1}" y1="140" x2="{q3}" y2="140"/><line class="mark" x1="{median}" y1="124" x2="{median}" y2="156"/><text class="s" x="{q1}" y="166" text-anchor="middle">Q1</text><text class="s" x="{median}" y="118" text-anchor="middle">median</text><text class="s" x="{q3}" y="166" text-anchor="middle">Q3</text><text class="s" x="20" y="202">SD: distance around the mean. Variance = SD². IQR: width of the middle 50%.</text></svg>"#,
        sd_low = 38.0 + sd_low_x / 100.0 * 684.0,
        sd_high = 38.0 + sd_high_x / 100.0 * 684.0,
        mean = 38.0 + mean_x / 100.0 * 684.0,
        q1 = 38.0 + q1_x / 100.0 * 684.0,
        median = 38.0 + median_x / 100.0 * 684.0,
        q3 = 38.0 + q3_x / 100.0 * 684.0,
    );
    record([
        ("ascii", text(ascii)),
        ("svg", text(svg)),
        ("sd_bands", list(sd_bands)),
        (
            "sd_band_caution",
            text(
                "Observed 1/2/3-SD coverage is descriptive. The 68/95/99.7 percentages are references only when a normal model is reasonable, and values beyond a band are not automatically errors or outliers.",
            ),
        ),
        (
            "reading_order",
            string_list([
                "Look at shape and separate groups first.",
                "Choose a centre that answers the scientific question.",
                "Pair mean with SD, or median with IQR/MAD when robustness is needed.",
                "Inspect tails and flagged observations; do not delete them automatically.",
                "Consider a log scale only for positive, ratio-like measurements.",
                "Keep data spread separate from uncertainty in an estimate.",
            ]),
        ),
        ("scale_clue", text(scale_clue)),
        ("raw_skewness", number(raw_skew)),
        (
            "log_skewness",
            number(log_summary.as_ref().and_then(|value| value.skewness)),
        ),
        ("automatic_choice", Value::Bool(false)),
    ])
}

fn explore_numeric(args: Vec<Value>) -> Result<Value> {
    let opts = options(&args, 1, "stats_explore")?;
    let variable = opts.get("name").and_then(Value::as_str).unwrap_or("values");
    let data_type = opts
        .get("data_type")
        .and_then(Value::as_str)
        .unwrap_or("unspecified");
    Ok(numeric_report(
        numeric_data(&args[0], "stats_explore")?,
        variable,
        data_type,
    ))
}

fn category_label(value: &Value) -> Option<String> {
    match value {
        Value::Nil => None,
        Value::Str(value) => Some(value.to_string()),
        Value::Int(value) => Some(value.to_string()),
        Value::Float(value) if value.is_finite() => Some(fmt_number(*value)),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn compare_groups(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_compare")?;
    let Value::List(groups) = &args[1] else {
        return Err(BioLangError::type_error(
            format!(
                "stats_compare() groups must be List, got {}",
                args[1].type_of()
            ),
            None,
        ));
    };
    if groups.len() != data.total {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "stats_compare() values and groups must have equal length ({} vs {})",
                data.total,
                groups.len()
            ),
            None,
        ));
    }
    let opts = options(&args, 2, "stats_compare")?;
    let paired = opts.get("paired").and_then(Value::as_bool).unwrap_or(false);
    let data_type = opts
        .get("data_type")
        .and_then(Value::as_str)
        .unwrap_or("unspecified");
    let mut labels = Vec::<String>::new();
    let mut positions = HashMap::<String, usize>::new();
    let mut grouped = Vec::<Vec<(usize, f64)>>::new();
    let mut excluded_group = 0usize;
    for (clean_position, original_index) in data.original_indices.iter().enumerate() {
        let Some(label) = category_label(&groups[*original_index]) else {
            excluded_group += 1;
            continue;
        };
        let group_position = *positions.entry(label.clone()).or_insert_with(|| {
            labels.push(label);
            grouped.push(Vec::new());
            labels.len() - 1
        });
        grouped[group_position].push((*original_index, data.values[clean_position]));
    }
    if labels.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_compare() requires at least two non-empty groups",
            None,
        ));
    }

    let group_reports = labels
        .iter()
        .zip(grouped.iter())
        .map(|(label, values)| {
            let group_data = NumericData {
                values: values.iter().map(|(_, value)| *value).collect(),
                original_indices: values.iter().map(|(index, _)| *index).collect(),
                total: values.len(),
                missing: 0,
                non_finite: 0,
            };
            record([
                ("group", text(label)),
                ("exploration", numeric_report(group_data, label, data_type)),
            ])
        })
        .collect::<Vec<_>>();
    let independent_group_variances = if !paired {
        let variances = grouped
            .iter()
            .map(|group| {
                if group.len() < 2 {
                    return None;
                }
                let mean = group.iter().map(|(_, value)| *value).sum::<f64>() / group.len() as f64;
                Some(
                    group
                        .iter()
                        .map(|(_, value)| (value - mean).powi(2))
                        .sum::<f64>()
                        / (group.len() - 1) as f64,
                )
            })
            .collect::<Option<Vec<_>>>();
        variances
    } else {
        None
    };
    let (variance_ratio, unequal_spread_clue, variance_note) = match independent_group_variances {
        Some(variances) => {
            let lower = variances[0].min(variances[1]);
            let upper = variances[0].max(variances[1]);
            if lower <= f64::EPSILON && upper > f64::EPSILON {
                (
                    None,
                    true,
                    if labels.len() == 2 {
                        "One group has essentially no observed spread while the other does. Welch's t-test avoids a pooled-variance assumption, but inspect the measurements and design before testing.".to_string()
                    } else {
                        "At least one group has essentially no observed spread while another does. Welch ANOVA cannot estimate a finite weight for a zero-variance group, so inspect the measurements before testing.".to_string()
                    },
                )
            } else {
                let ratio = if upper <= f64::EPSILON { 1.0 } else { upper / lower };
                let clue = ratio >= 2.0;
                let note = if clue {
                    if labels.len() == 2 {
                        format!(
                            "The larger sample variance is {} times the smaller one. This is an unequal-spread clue, so use Welch's t-test rather than pooling variances.",
                            fmt_number(ratio)
                        )
                    } else {
                        format!(
                            "The largest sample variance is {} times the smallest one. This is an unequal-spread clue, so prefer Welch ANOVA to classical equal-variance ANOVA.",
                            fmt_number(ratio)
                        )
                    }
                } else {
                    if labels.len() == 2 {
                        format!(
                            "The sample-variance ratio is {}. No strong unequal-spread clue appears here, but Welch's t-test remains a safe default for independent means.",
                            fmt_number(ratio)
                        )
                    } else {
                        format!(
                            "The largest-to-smallest sample-variance ratio is {}. No strong unequal-spread clue appears here, but Welch ANOVA remains a safe default for independent means.",
                            fmt_number(ratio)
                        )
                    }
                };
                (Some(ratio), clue, note)
            }
        }
        None => (
            None,
            false,
            "A variance comparison needs at least two usable observations in every independent group.".to_string(),
        ),
    };
    let primary = if paired {
        "Inspect within-pair differences before choosing a paired analysis."
    } else if labels.len() == 2 {
        "For a difference in independent-group means, use Welch's t-test and report the mean difference, confidence interval, and effect size."
    } else {
        "For differences among independent-group means, use Welch ANOVA and report the global effect size; follow a detected difference with an explicitly adjusted comparison procedure."
    };
    let choices = if labels.len() == 2 {
        if paired {
            vec![
                approach(
                    "paired t-test",
                    "Within-pair differences are reasonably symmetric.",
                    "Requires correctly aligned pairs.",
                    "candidate",
                ),
                approach(
                    "Wilcoxon signed-rank",
                    "A rank-based paired comparison is appropriate.",
                    "Tests ranked differences and is not simply a median test.",
                    "alternative",
                ),
                approach(
                    "paired permutation or bootstrap",
                    "You want uncertainty with fewer parametric assumptions.",
                    "The resampling unit must be the pair.",
                    "alternative",
                ),
            ]
        } else {
            vec![
                approach(
                    "Welch's t-test",
                    "The scientific estimand is a difference in means.",
                    "Independence comes from study design, not this report.",
                    "candidate",
                ),
                approach(
                    "Mann-Whitney rank-sum",
                    "A rank/distribution comparison is meaningful.",
                    "It is not automatically a test of medians.",
                    "alternative",
                ),
                approach(
                    "permutation or bootstrap",
                    "You want a directly simulated null or interval.",
                    "Exchangeability and the resampling unit still require justification.",
                    "alternative",
                ),
                approach(
                    "regression",
                    "Covariates or batch variables need adjustment.",
                    "The model form and diagnostics must be checked.",
                    "alternative",
                ),
            ]
        }
    } else {
        vec![
            approach(
                "Welch ANOVA",
                "The estimand is group mean differences.",
                "Follow a detected difference with planned contrasts or pairwise Welch tests using multiplicity correction.",
                "candidate",
            ),
            approach(
                "classical one-way ANOVA plus Tukey HSD",
                "An equal-variance mean model is scientifically and diagnostically defensible.",
                "Tukey HSD controls family-wise error for all pairwise comparisons and uses the shared ANOVA residual variance.",
                "alternative",
            ),
            approach(
                "Kruskal-Wallis",
                "A rank/distribution comparison is the intended estimand.",
                "It is not simply a median test, and follow-up comparisons need their own rank-based procedure.",
                "alternative",
            ),
            approach(
                "regression or permutation model",
                "Covariates, interactions, or batch variables matter.",
                "Requires model diagnostics and a specified reference group.",
                "alternative",
            ),
        ]
    };
    let group_names = labels.join(", ");
    let explanation = format!(
        "Grouped exploratory statistics\n\nGroups\n  {}\n  {} usable group(s); {} observation(s) lacked a usable group label.\n\nSuggested next step\n  {}\n\nSpread check\n  {}\n\nDesign information required\n  BioLang cannot infer independence, pairing, experimental units, batches, or confounding from these vectors. paired was explicitly set to {}.\n\nReport effect sizes and confidence intervals alongside any p-value.",
        group_names,
        labels.len(),
        excluded_group,
        primary,
        variance_note,
        paired,
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("grouped_numeric")),
        ("groups", list(group_reports)),
        ("group_names", string_list(labels.clone())),
        ("paired", Value::Bool(paired)),
        ("excluded_missing_group", Value::Int(excluded_group as i64)),
        ("suggestion", text(primary)),
        (
            "recommended_test",
            text(if paired {
                "paired analysis"
            } else if labels.len() == 2 {
                "welch_t"
            } else {
                "welch_anova"
            }),
        ),
        (
            "recommended_call",
            text(if paired {
                "ttest_paired(before, after)"
            } else if labels.len() == 2 {
                "ttest(group_a, group_b, {variance: \"welch\"})"
            } else {
                "anova(groups, {variance: \"welch\"})"
            }),
        ),
        ("variance_ratio", number(variance_ratio)),
        ("unequal_spread_clue", Value::Bool(unequal_spread_clue)),
        ("variance_note", text(variance_note)),
        ("alternatives", list(choices)),
        ("quick_explanation", text(primary)),
        ("explanation", text(explanation)),
        (
            "limitations",
            string_list([
                "The experimental unit and independence must come from the study design.",
                "Distribution shape alone does not select a hypothesis test.",
                "A p-value should not replace an effect size and confidence interval.",
            ]),
        ),
    ]))
}

fn preprocessing_guide(args: Vec<Value>) -> Result<Value> {
    let opts = options(&args, 1, "stats_preprocess")?;
    let variable = opts.get("name").and_then(Value::as_str).unwrap_or("values");
    let data_type = opts
        .get("data_type")
        .and_then(Value::as_str)
        .unwrap_or("unspecified");
    let report = numeric_report(
        numeric_data(&args[0], "stats_preprocess")?,
        variable,
        data_type,
    );
    let Value::Record(report) = report else {
        unreachable!();
    };
    Ok(report
        .get("preprocessing")
        .cloned()
        .expect("numeric reports always include preprocessing"))
}

fn complete_pairs(
    left: &Value,
    right: &Value,
    function: &str,
) -> Result<(Vec<f64>, Vec<f64>, usize)> {
    let Value::List(xs) = left else {
        return Err(BioLangError::type_error(
            format!("{function}() x must be List, got {}", left.type_of()),
            None,
        ));
    };
    let Value::List(ys) = right else {
        return Err(BioLangError::type_error(
            format!("{function}() y must be List, got {}", right.type_of()),
            None,
        ));
    };
    if xs.len() != ys.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() x and y must have equal length"),
            None,
        ));
    }
    let mut clean_x = Vec::new();
    let mut clean_y = Vec::new();
    let mut excluded = 0usize;
    for (index, (x, y)) in xs.iter().zip(ys.iter()).enumerate() {
        let convert = |value: &Value| match value {
            Value::Nil => Ok(None),
            Value::Int(value) => Ok(Some(*value as f64)),
            Value::Float(value) if value.is_finite() => Ok(Some(*value)),
            Value::Float(_) => Ok(None),
            other => Err(BioLangError::type_error(
                format!(
                    "{function}() pairs must be numeric or Nil; index {index} contains {}",
                    other.type_of()
                ),
                None,
            )),
        };
        match (convert(x)?, convert(y)?) {
            (Some(x), Some(y)) => {
                clean_x.push(x);
                clean_y.push(y);
            }
            _ => excluded += 1,
        }
    }
    if clean_x.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() requires at least two complete finite pairs"),
            None,
        ));
    }
    Ok((clean_x, clean_y, excluded))
}

fn pearson(xs: &[f64], ys: &[f64]) -> Option<(f64, f64, f64)> {
    let n = xs.len() as f64;
    let mean_x = xs.iter().sum::<f64>() / n;
    let mean_y = ys.iter().sum::<f64>() / n;
    let covariance = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let sum_xx = xs.iter().map(|x| (x - mean_x).powi(2)).sum::<f64>();
    let sum_yy = ys.iter().map(|y| (y - mean_y).powi(2)).sum::<f64>();
    if sum_xx <= f64::EPSILON || sum_yy <= f64::EPSILON {
        return None;
    }
    let r = covariance / (sum_xx * sum_yy).sqrt();
    let slope = covariance / sum_xx;
    let intercept = mean_y - slope * mean_x;
    Some((r, slope, intercept))
}

fn average_ranks(values: &[f64]) -> Vec<f64> {
    let mut ordered = values.iter().copied().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|left, right| left.1.total_cmp(&right.1));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0usize;
    while start < ordered.len() {
        let mut end = start + 1;
        while end < ordered.len() && ordered[end].1.to_bits() == ordered[start].1.to_bits() {
            end += 1;
        }
        let average = ((start + 1) as f64 + end as f64) / 2.0;
        for position in start..end {
            ranks[ordered[position].0] = average;
        }
        start = end;
    }
    ranks
}

fn explore_relationship(args: Vec<Value>) -> Result<Value> {
    let _opts = options(&args, 2, "stats_relationship")?;
    let (xs, ys, excluded) = complete_pairs(&args[0], &args[1], "stats_relationship")?;
    let linear = pearson(&xs, &ys);
    let rank_x = average_ranks(&xs);
    let rank_y = average_ranks(&ys);
    let spearman = pearson(&rank_x, &rank_y).map(|result| result.0);
    let strength = linear
        .map(|result| match result.0.abs() {
            value if value >= 0.7 => "strong linear association",
            value if value >= 0.4 => "moderate linear association",
            value if value >= 0.2 => "weak linear association",
            _ => "little linear association",
        })
        .unwrap_or("linear association is undefined because one variable is constant");
    let explanation = format!(
        "Relationship exploration\n\nData used\n  Complete finite pairs: {}\n  Excluded incomplete pairs: {}\n\nObserved association\n  Pearson r: {}\n  Spearman rho: {}\n  Description: {}.\n\nInterpretation\n  Pearson describes linear association. Spearman describes monotonic rank association. Neither establishes causation, agreement, or absence of a nonlinear relationship. Always inspect the scatterplot and influential observations.",
        xs.len(),
        excluded,
        linear.map(|result| fmt_number(result.0)).unwrap_or_else(|| "undefined".into()),
        spearman.map(fmt_number).unwrap_or_else(|| "undefined".into()),
        strength,
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("relationship")),
        ("complete_pairs", Value::Int(xs.len() as i64)),
        ("excluded_pairs", Value::Int(excluded as i64)),
        ("pearson", number(linear.map(|result| result.0))),
        ("spearman", number(spearman)),
        ("slope", number(linear.map(|result| result.1))),
        ("intercept", number(linear.map(|result| result.2))),
        ("r_squared", number(linear.map(|result| result.0.powi(2)))),
        ("description", text(strength)),
        (
            "alternatives",
            list(vec![
                approach(
                    "scatterplot + Pearson correlation",
                    "A linear association is the target.",
                    "Sensitive to influential points and blind to many nonlinear patterns.",
                    "candidate",
                ),
                approach(
                    "scatterplot + Spearman correlation",
                    "A monotonic rank association is the target.",
                    "Does not quantify the slope in the original units.",
                    "alternative",
                ),
                approach(
                    "regression",
                    "Prediction or covariate adjustment is required.",
                    "Requires residual and model-form diagnostics.",
                    "alternative",
                ),
                approach(
                    "agreement analysis",
                    "The variables are two methods measuring the same quantity.",
                    "Correlation alone can be high despite systematic disagreement.",
                    "alternative",
                ),
            ]),
        ),
        (
            "quick_explanation",
            text(format!("{} complete pairs; {}.", xs.len(), strength)),
        ),
        ("explanation", text(explanation)),
        (
            "limitations",
            string_list([
                "Correlation does not establish causation.",
                "A low Pearson correlation does not rule out nonlinear association.",
                "Repeated measures and confounding are not inferred from paired vectors.",
            ]),
        ),
    ]))
}

fn explore_categories(args: Vec<Value>) -> Result<Value> {
    let _opts = options(&args, 1, "stats_categories")?;
    let Value::List(items) = &args[0] else {
        return Err(BioLangError::type_error(
            format!(
                "stats_categories() requires List, got {}",
                args[0].type_of()
            ),
            None,
        ));
    };
    let mut labels = Vec::<String>::new();
    let mut counts = Vec::<usize>::new();
    let mut positions = HashMap::<String, usize>::new();
    let mut missing = 0usize;
    for (index, item) in items.iter().enumerate() {
        let Some(label) = category_label(item) else {
            if matches!(item, Value::Nil) {
                missing += 1;
                continue;
            }
            return Err(BioLangError::type_error(
                format!(
                    "stats_categories() values must be Str, number, Bool, or Nil; index {index} is {}",
                    item.type_of()
                ),
                None,
            ));
        };
        let position = *positions.entry(label.clone()).or_insert_with(|| {
            labels.push(label);
            counts.push(0);
            labels.len() - 1
        });
        counts[position] += 1;
    }
    let used = counts.iter().sum::<usize>();
    if used == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_categories() has no non-missing categories",
            None,
        ));
    }
    let levels = labels
        .iter()
        .zip(counts.iter())
        .map(|(label, count)| {
            record([
                ("value", text(label)),
                ("count", Value::Int(*count as i64)),
                ("proportion", Value::Float(*count as f64 / used as f64)),
            ])
        })
        .collect::<Vec<_>>();
    let max_count = counts.iter().copied().max().unwrap_or(0);
    let modes = labels
        .iter()
        .zip(counts.iter())
        .filter_map(|(label, count)| (*count == max_count).then_some(label.clone()))
        .collect::<Vec<_>>();
    let rare = labels
        .iter()
        .zip(counts.iter())
        .filter_map(|(label, count)| {
            ((*count as f64 / used as f64) < 0.05).then_some(label.clone())
        })
        .collect::<Vec<_>>();
    let explanation = format!(
        "Categorical exploration\n\nData used\n  Received: {}\n  Non-missing: {}\n  Missing: {}\n  Levels: {}\n\nMost frequent level(s)\n  {} (count {}).\n\nUseful summaries\n  Report counts and proportions. A mode names the most frequent level but does not show the balance of all levels.{}",
        items.len(),
        used,
        missing,
        labels.len(),
        modes.join(", "),
        max_count,
        if rare.is_empty() { String::new() } else { format!("\n\nClue\n  Rare level(s) below 5%: {}. Consider whether sparse categories are scientifically meaningful before combining them.", rare.join(", ")) },
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("categorical")),
        ("received", Value::Int(items.len() as i64)),
        ("used", Value::Int(used as i64)),
        ("missing", Value::Int(missing as i64)),
        ("n_levels", Value::Int(labels.len() as i64)),
        ("levels", list(levels)),
        ("modes", string_list(modes)),
        ("rare_levels", string_list(rare)),
        (
            "suggestion",
            text("Report counts and proportions; plot every scientifically meaningful level."),
        ),
        (
            "alternatives",
            list(vec![
                approach(
                    "count and proportion table",
                    "You need a complete categorical description.",
                    "Long tables may need ordering or grouping for display.",
                    "suggested",
                ),
                approach(
                    "bar chart",
                    "Readers need to compare category frequencies visually.",
                    "The baseline must start at zero; ordering should be disclosed.",
                    "suggested_visual",
                ),
                approach(
                    "mode",
                    "One most-common label is genuinely useful.",
                    "Hides all remaining category frequencies and may be tied.",
                    "alternative",
                ),
            ]),
        ),
        (
            "quick_explanation",
            text(format!(
                "{} non-missing values across {} level(s).",
                used,
                labels.len()
            )),
        ),
        ("explanation", text(explanation)),
        (
            "limitations",
            string_list([
                "A frequency table does not explain why categories differ.",
                "Combining rare levels requires scientific justification.",
                "Category labels are kept in first-observed order.",
            ]),
        ),
    ]))
}

fn add_guidance(args: Vec<Value>) -> Result<Value> {
    let Value::Record(report) = &args[0] else {
        return Err(BioLangError::type_error(
            format!(
                "stats_guide() requires an exploration Record, got {}",
                args[0].type_of()
            ),
            None,
        ));
    };
    let context = options(&args, 1, "stats_guide")?;
    let mut output = report.as_ref().clone();
    let question = context
        .get("question")
        .and_then(Value::as_str)
        .unwrap_or("not specified");
    let experimental_unit = context.get("experimental_unit").and_then(Value::as_str);
    let design_note = match experimental_unit {
        Some(unit) => format!(
            "The user identified '{unit}' as the experimental unit. Verify that rows are independent at this level."
        ),
        None => "Experimental unit was not specified. BioLang will not infer independence from the values.".into(),
    };
    output.insert(
        "guidance".into(),
        record([
            ("question", text(question)),
            (
                "experimental_unit",
                experimental_unit.map(text).unwrap_or(Value::Nil),
            ),
            ("design_note", text(design_note)),
            ("context_received", Value::Record(context.into())),
            ("automatic_test_selection", Value::Bool(false)),
            (
                "next_step",
                text("Choose the scientific estimand first, then use the listed alternatives and verify their assumptions."),
            ),
        ]),
    );
    Ok(Value::Record(output.into()))
}

fn explain(args: Vec<Value>) -> Result<Value> {
    let Value::Record(report) = &args[0] else {
        return Err(BioLangError::type_error(
            format!(
                "stats_explain() requires an exploration Record, got {}",
                args[0].type_of()
            ),
            None,
        ));
    };
    let detail = match args.get(1) {
        None => "learning",
        Some(Value::Str(value)) => value.as_str(),
        Some(other) => {
            return Err(BioLangError::type_error(
                format!(
                    "stats_explain() detail must be Str, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let key = if detail == "quick" {
        "quick_explanation"
    } else {
        "explanation"
    };
    let Some(Value::Str(explanation)) = report.get(key) else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_explain() record is not a BioLang exploration report",
            None,
        ));
    };
    if detail == "audit" {
        let schema = report
            .get("schema")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        Ok(text(format!(
            "{}\n\nAudit\n  Schema: {}\n  Calculations are deterministic and use all disclosed finite observations.\n  Suggestions are heuristic and never modify the input data.",
            explanation, schema
        )))
    } else {
        Ok(Value::Str(explanation.clone()))
    }
}

fn distribution_plot(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_distribution_plot")?;
    let summary = summarize(&data);
    let opts = options(&args, 1, "stats_distribution_plot")?;
    let width = opts
        .get("width")
        .and_then(Value::as_float)
        .unwrap_or(860.0)
        .max(480.0);
    let height = opts
        .get("height")
        .and_then(Value::as_float)
        .unwrap_or(460.0)
        .max(360.0);
    let title = opts
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Distribution guide");
    let bins = opts
        .get("bins")
        .and_then(Value::as_int)
        .map(|value| value.clamp(3, 100) as usize)
        .unwrap_or_else(|| (summary.n as f64).sqrt().round().clamp(5.0, 40.0) as usize);
    let max_points = opts
        .get("max_points")
        .and_then(Value::as_int)
        .map(|value| value.max(1) as usize)
        .unwrap_or(2_000);

    let span = (summary.max - summary.min).abs();
    let padding = if span <= f64::EPSILON {
        summary.max.abs().max(1.0) * 0.1
    } else {
        span * 0.05
    };
    let x_scale = Scale {
        domain: (summary.min - padding, summary.max + padding),
        range: (60.0, width - 30.0),
    };
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 60.0;
    canvas.margin.right = 30.0;
    canvas.margin.top = 50.0;
    canvas.margin.bottom = 55.0;

    let histogram = histogram_geometry(
        &[
            Value::List(
                data.values
                    .iter()
                    .copied()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
            Value::Record(HashMap::from([("bins".into(), Value::Int(bins as i64))]).into()),
        ],
        "stats_distribution_plot",
    )?;
    let max_count = histogram.counts.iter().copied().max().unwrap_or(1).max(1);
    let hist_top = 65.0;
    let hist_bottom = height * 0.49;
    for (index, count) in histogram.counts.iter().enumerate() {
        let bar_height = (*count as f64 / max_count as f64) * (hist_bottom - hist_top);
        let left = x_scale.map(histogram.edges[index]);
        let right = x_scale.map(histogram.edges[index + 1]);
        canvas.add_rect(
            left,
            hist_bottom - bar_height,
            (right - left - 1.0).max(1.0),
            bar_height,
            "#bfdbfe",
        );
    }
    canvas.add_text(62.0, hist_top - 6.0, "count", "start", 11.0);

    let dot_y = height * 0.60;
    let stride = data.values.len().div_ceil(max_points).max(1);
    for (position, value) in data.values.iter().enumerate().step_by(stride) {
        let is_outlier = value < &summary.lower_fence || value > &summary.upper_fence;
        let jitter = ((data.original_indices[position] * 37) % 7) as f64 - 3.0;
        canvas.add_circle(
            x_scale.map(*value),
            dot_y + jitter * 2.2,
            3.0,
            if is_outlier { "#dc2626" } else { "#475569" },
        );
    }

    let line_top = 54.0;
    let line_bottom = height * 0.70;
    let mean_x = x_scale.map(summary.mean);
    let median_x = x_scale.map(summary.median);
    canvas.add_line(mean_x, line_top, mean_x, line_bottom, "#dc2626", 2.0);
    canvas.add_line(median_x, line_top, median_x, line_bottom, "#2563eb", 2.0);
    canvas.add_text(
        mean_x,
        43.0,
        &format!("mean {}", fmt_number(summary.mean)),
        "middle",
        11.0,
    );
    canvas.add_text(
        median_x,
        57.0,
        &format!("median {}", fmt_number(summary.median)),
        "middle",
        11.0,
    );

    let iqr_y = height * 0.73;
    canvas.add_rect(
        x_scale.map(summary.q1),
        iqr_y,
        (x_scale.map(summary.q3) - x_scale.map(summary.q1))
            .abs()
            .max(1.0),
        18.0,
        "#a7f3d0",
    );
    canvas.add_line(median_x, iqr_y, median_x, iqr_y + 18.0, "#065f46", 2.0);
    canvas.add_text(62.0, iqr_y + 14.0, "Q1 - Q3", "start", 11.0);

    let sd_y = height * 0.80;
    // The 1/2/3-SD reference bands are a normal-model aid; without an SD
    // there is nothing to reference, so none are drawn.
    for multiplier in (1..=3).rev().filter(|_| summary.sd.is_some()) {
        let left = x_scale
            .map(summary.mean - multiplier as f64 * summary.sd.unwrap_or(0.0))
            .max(60.0);
        let right = x_scale
            .map(summary.mean + multiplier as f64 * summary.sd.unwrap_or(0.0))
            .min(width - 30.0);
        if right > left {
            let color = match multiplier {
                1 => "#fde68a",
                2 => "#fef3c7",
                _ => "#fffbeb",
            };
            canvas.add_rect(left, sd_y, right - left, 16.0, color);
        }
    }
    canvas.add_text(62.0, sd_y + 13.0, "mean +/- 1, 2, 3 SD", "start", 10.0);

    let display_note = if stride == 1 {
        format!(
            "All {} observations are shown; calculations use all finite values.",
            summary.n
        )
    } else {
        format!(
            "Dots show every {stride}th observation ({} of {}); calculations and histogram use all finite values.",
            data.values.iter().step_by(stride).count(),
            summary.n
        )
    };
    canvas.add_text(62.0, height - 61.0, &display_note, "start", 10.0);
    canvas.draw_x_axis(&x_scale, "value");
    canvas.draw_title(title);
    Ok(text(canvas.render()))
}

fn distribution_ascii(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_distribution_ascii")?;
    let summary = summarize(&data);
    let opts = options(&args, 1, "stats_distribution_ascii")?;
    let width = opts
        .get("width")
        .and_then(Value::as_int)
        .map(|value| value.clamp(20, 100) as usize)
        .unwrap_or(56);
    let height = opts
        .get("height")
        .and_then(Value::as_int)
        .map(|value| value.clamp(4, 24) as usize)
        .unwrap_or(10);
    let title = opts
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Distribution guide (ASCII)");

    let span = summary.max - summary.min;
    let mut counts = vec![0usize; width];
    for value in &data.values {
        let index = if span.abs() <= f64::EPSILON {
            width / 2
        } else {
            (((value - summary.min) / span) * (width - 1) as f64)
                .floor()
                .clamp(0.0, (width - 1) as f64) as usize
        };
        counts[index] += 1;
    }
    let max_count = counts.iter().copied().max().unwrap_or(1).max(1);
    let plot_height = height.min(max_count).max(1);
    let scaled: Vec<usize> = counts
        .iter()
        .map(|count| ((*count as f64 / max_count as f64) * plot_height as f64).ceil() as usize)
        .collect();
    let position = |value: f64| -> usize {
        if span.abs() <= f64::EPSILON {
            width / 2
        } else {
            (((value - summary.min) / span) * (width - 1) as f64)
                .round()
                .clamp(0.0, (width - 1) as f64) as usize
        }
    };

    let mut output = String::new();
    output.push_str(title);
    output.push('\n');
    output.push_str(&format!(
        "n={} finite values; peak bin count={}\n",
        summary.n, max_count
    ));
    for level in (1..=plot_height).rev() {
        output.push_str(&format!(
            "{:>6} |",
            (max_count * level).div_ceil(plot_height)
        ));
        for bar_height in &scaled {
            output.push(if *bar_height >= level { '#' } else { ' ' });
        }
        output.push_str("|\n");
    }
    output.push_str("       +");
    output.push_str(&"-".repeat(width));
    output.push_str("+\n");

    let mut center_line = vec![' '; width];
    let mean_position = position(summary.mean);
    let median_position = position(summary.median);
    center_line[mean_position] = 'A';
    center_line[median_position] = if mean_position == median_position {
        '*'
    } else {
        'M'
    };
    output.push_str("center  |");
    output.extend(center_line);
    output.push_str("|  A=mean M=median *=both\n");

    let mut quartile_line = vec![' '; width];
    let q1_position = position(summary.q1);
    let q3_position = position(summary.q3);
    for cell in quartile_line
        .iter_mut()
        .take(q3_position + 1)
        .skip(q1_position)
    {
        *cell = '=';
    }
    quartile_line[q1_position] = '[';
    quartile_line[q3_position] = ']';
    output.push_str("IQR     |");
    output.extend(quartile_line);
    output.push_str("|  middle 50%\n");

    let missing_note = if data.missing == 0 && data.non_finite == 0 {
        "none excluded".to_string()
    } else {
        format!(
            "{} missing and {} non-finite excluded",
            data.missing, data.non_finite
        )
    };
    output.push_str(&format!(
        "range {} to {} | mean {} | median {}\n",
        fmt_number(summary.min),
        fmt_number(summary.max),
        fmt_number(summary.mean),
        fmt_number(summary.median)
    ));
    output.push_str(&format!(
        "SD {} | IQR {} | Tukey review flags {}\n",
        summary
            .sd
            .map(fmt_number)
            .unwrap_or_else(|| "not defined".into()),
        fmt_number(summary.iqr),
        summary.outlier_positions.len()
    ));
    output.push_str(&format!(
        "All {} finite observations contribute to the histogram; {}.\n",
        summary.n, missing_note
    ));
    output.push_str(
        "SD bands imply 68-95-99.7% coverage only for an approximately normal distribution.",
    );

    Ok(text(output))
}

fn normal_density(z: f64) -> f64 {
    (-0.5 * z * z).exp()
}

fn normal_x(z: f64, left: f64, right: f64) -> f64 {
    left + ((z + 4.0) / 8.0) * (right - left)
}

fn normal_y(z: f64, baseline: f64, curve_height: f64) -> f64 {
    baseline - normal_density(z) * curve_height
}

fn normal_area_path(
    start: f64,
    end: f64,
    left: f64,
    right: f64,
    baseline: f64,
    curve_height: f64,
) -> String {
    let steps = (((end - start).abs() * 36.0).ceil() as usize).max(2);
    let mut path = format!("M {:.2} {:.2}", normal_x(start, left, right), baseline);
    for index in 0..=steps {
        let z = start + (end - start) * index as f64 / steps as f64;
        path.push_str(&format!(
            " L {:.2} {:.2}",
            normal_x(z, left, right),
            normal_y(z, baseline, curve_height)
        ));
    }
    path.push_str(&format!(
        " L {:.2} {:.2} Z",
        normal_x(end, left, right),
        baseline
    ));
    path
}

fn normal_curve_path(left: f64, right: f64, baseline: f64, curve_height: f64) -> String {
    let mut path = String::new();
    for index in 0..=320 {
        let z = -4.0 + 8.0 * index as f64 / 320.0;
        let command = if index == 0 { 'M' } else { 'L' };
        path.push_str(&format!(
            "{command} {:.2} {:.2} ",
            normal_x(z, left, right),
            normal_y(z, baseline, curve_height)
        ));
    }
    path
}

fn normal_tail_probability(z: f64, tail: &str) -> (f64, String) {
    let cdf = bl_core::bio_core::stats_ops::normal_cdf(z);
    match tail {
        "left" => (cdf, format!("P(Z <= {})", fmt_number(z))),
        "right" => (1.0 - cdf, format!("P(Z >= {})", fmt_number(z))),
        _ => {
            let probability = 2.0 * bl_core::bio_core::stats_ops::normal_sf(z.abs());
            (probability, format!("P(|Z| >= {})", fmt_number(z.abs())))
        }
    }
}

fn normal_observed_notes(data: Option<&NumericData>) -> (String, String) {
    let Some(data) = data else {
        return (
            "No observations supplied: the percentages describe an ideal normal distribution."
                .into(),
            "Use observed data to compare its coverage with the reference bands.".into(),
        );
    };
    let summary = summarize(data);
    let coverage = summary
        .sd
        .map(|sd| {
            (1..=3)
                .map(|multiple| {
                    let distance = multiple as f64 * sd;
                    let count = data
                        .values
                        .iter()
                        .filter(|value| {
                            **value >= summary.mean - distance && **value <= summary.mean + distance
                        })
                        .count();
                    format!("{:.1}%", 100.0 * count as f64 / summary.n as f64)
                })
                .collect::<Vec<_>>()
                .join(" / ")
        })
        .unwrap_or_else(|| "not defined / not defined / not defined".into());
    let observed = format!(
        "Observed within 1 / 2 / 3 SD: {coverage} (n={}; mean {}; SD {}).",
        summary.n,
        fmt_number(summary.mean),
        summary
            .sd
            .map(fmt_number)
            .unwrap_or_else(|| "not defined".into())
    );
    let shape = if summary.sd.is_none_or(|sd| sd <= f64::EPSILON) {
        "The observations have no measurable spread, so a normal-curve comparison is not meaningful."
    } else if summary.n < 20 {
        "Shape evidence is limited below 20 observations; treat the normal percentages as a reference only."
    } else if summary.skewness.is_some_and(|value| value.abs() >= 0.5)
        || !summary.outlier_positions.is_empty()
    {
        "The observed data show asymmetry or Tukey review flags; prefer their measured coverage over the normal rule."
    } else {
        "No strong asymmetry or Tukey review flags were detected; a Q-Q plot can provide another normal-shape check."
    };
    (observed, shape.into())
}

fn normal_diagram_ascii(
    data: Option<&NumericData>,
    opts: &HashMap<String, Value>,
    z: Option<f64>,
    tail: &str,
) -> String {
    let width = opts
        .get("width")
        .and_then(Value::as_int)
        .map(|value| value.clamp(41, 101) as usize)
        .unwrap_or(65);
    let height = opts
        .get("height")
        .and_then(Value::as_int)
        .map(|value| value.clamp(8, 20) as usize)
        .unwrap_or(12);
    let mut output = String::from("NORMAL DISTRIBUTION: SD AREAS\n");
    for row in 0..height {
        let level = (height - 1 - row) as f64 / (height - 1) as f64;
        for column in 0..width {
            let current_z = -4.0 + 8.0 * column as f64 / (width - 1) as f64;
            let density = normal_density(current_z);
            let on_curve = (density - level).abs() <= 0.55 / height as f64;
            let highlighted = z.is_some_and(|threshold| match tail {
                "left" => current_z <= threshold,
                "right" => current_z >= threshold,
                _ => current_z.abs() >= threshold.abs(),
            });
            let fill = if current_z.abs() <= 1.0 {
                '1'
            } else if current_z.abs() <= 2.0 {
                '2'
            } else if current_z.abs() <= 3.0 {
                '3'
            } else {
                '.'
            };
            output.push(if on_curve {
                '*'
            } else if density >= level {
                if highlighted {
                    '!'
                } else {
                    fill
                }
            } else {
                ' '
            });
        }
        output.push('\n');
    }
    output.push_str(&"-".repeat(width));
    output.push_str("\nz:       -3       -2       -1        0       +1       +2       +3\n");
    output.push_str("1 = within +/-1 SD: 68.27%\n");
    output.push_str("1+2 = within +/-2 SD: 95.45%\n");
    output.push_str("1+2+3 = within +/-3 SD: 99.73%\n");
    output.push_str(
        "These percentages describe an ideal normal curve; they are not an outlier rule.\n",
    );
    let (observed, shape) = normal_observed_notes(data);
    output.push_str(&observed);
    output.push('\n');
    output.push_str(&shape);
    if let Some(z) = z {
        let (probability, label) = normal_tail_probability(z, tail);
        output.push_str(&format!(
            "\n! = highlighted {tail} tail: {label} = {:.6}.",
            probability
        ));
    }
    output
}

fn normal_diagram_svg(
    data: Option<&NumericData>,
    opts: &HashMap<String, Value>,
    z: Option<f64>,
    tail: &str,
) -> String {
    let width = opts
        .get("width")
        .and_then(Value::as_float)
        .unwrap_or(860.0)
        .clamp(760.0, 1_200.0);
    let height = opts
        .get("height")
        .and_then(Value::as_float)
        .unwrap_or(500.0)
        .clamp(460.0, 720.0);
    let title = opts
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Normal distribution: what 1, 2, and 3 SD mean");
    let left = 62.0;
    let right = width - 35.0;
    let baseline = height - 205.0;
    let curve_height = baseline - 55.0;
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = left;
    canvas.margin.right = width - right;
    canvas.margin.top = 45.0;
    canvas.margin.bottom = height - baseline;

    let regions = [
        (-4.0, -3.0, "#f8fafc"),
        (-3.0, -2.0, "#dbeafe"),
        (-2.0, -1.0, "#93c5fd"),
        (-1.0, 1.0, "#60a5fa"),
        (1.0, 2.0, "#93c5fd"),
        (2.0, 3.0, "#dbeafe"),
        (3.0, 4.0, "#f8fafc"),
    ];
    for (start, end, fill) in regions {
        let path = normal_area_path(start, end, left, right, baseline, curve_height);
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="{fill}" stroke="none" />"#
        ));
    }
    if let Some(threshold) = z {
        let clamped = threshold.clamp(-4.0, 4.0);
        let ranges = match tail {
            "left" => vec![(-4.0, clamped)],
            "right" => vec![(clamped, 4.0)],
            _ => {
                let magnitude = clamped.abs();
                vec![(-4.0, -magnitude), (magnitude, 4.0)]
            }
        };
        for (start, end) in ranges.into_iter().filter(|(start, end)| end > start) {
            let path = normal_area_path(start, end, left, right, baseline, curve_height);
            canvas.elements.push(format!(
                r##"<path d="{path}" fill="#f97316" fill-opacity="0.58" stroke="none" />"##
            ));
        }
    }
    let curve = normal_curve_path(left, right, baseline, curve_height);
    canvas.elements.push(format!(
        r##"<path d="{curve}" fill="none" stroke="#172033" stroke-width="2.2" />"##
    ));
    canvas.add_line(left, baseline, right, baseline, "#334155", 1.2);
    for marker in -3..=3 {
        let x = normal_x(marker as f64, left, right);
        canvas.add_line(x, baseline, x, baseline + 7.0, "#334155", 1.0);
        if marker != 0 {
            canvas.add_line(
                x,
                baseline,
                x,
                normal_y(marker as f64, baseline, curve_height),
                "#94a3b8",
                1.0,
            );
        }
        let label = if marker == 0 {
            "mean".into()
        } else {
            format!("{marker:+} SD")
        };
        canvas.add_text(x, baseline + 22.0, &label, "middle", 11.0);
    }
    canvas.draw_title(title);

    let legend_y = baseline + 52.0;
    canvas.add_rect(left, legend_y - 11.0, 18.0, 11.0, "#60a5fa");
    canvas.add_text(
        left + 25.0,
        legend_y,
        "Central +/- 1 SD region: 68.27%",
        "start",
        12.0,
    );
    canvas.add_rect(left, legend_y + 15.0, 18.0, 11.0, "#93c5fd");
    canvas.add_text(
        left + 25.0,
        legend_y + 26.0,
        "Including the 1-to-2 SD shoulders: 95.45% total",
        "start",
        12.0,
    );
    canvas.add_rect(left, legend_y + 41.0, 18.0, 11.0, "#dbeafe");
    canvas.add_text(
        left + 25.0,
        legend_y + 52.0,
        "Including the 2-to-3 SD shoulders: 99.73% total",
        "start",
        12.0,
    );
    canvas.add_text(
        width / 2.0,
        legend_y,
        "SD is distance around the mean; variance is SD squared.",
        "start",
        11.0,
    );
    canvas.add_text(
        width / 2.0,
        legend_y + 22.0,
        "The percentages are normal-model references, not outlier cutoffs.",
        "start",
        11.0,
    );
    let (observed, shape) = normal_observed_notes(data);
    canvas.add_text(left, legend_y + 82.0, &observed, "start", 11.0);
    canvas.add_text(left, legend_y + 102.0, &shape, "start", 10.0);
    if let Some(z) = z {
        let (probability, label) = normal_tail_probability(z, tail);
        canvas.add_text(
            left,
            legend_y + 124.0,
            &format!("Orange highlight ({tail}): {label} = {:.6}", probability),
            "start",
            11.0,
        );
    }
    canvas.render()
}

fn normal_diagram(args: Vec<Value>) -> Result<Value> {
    let (data, opts) = match args.as_slice() {
        [] => (None, HashMap::new()),
        [Value::Record(opts)] => (None, opts.as_ref().clone()),
        [Value::List(items)] if items.is_empty() => (None, HashMap::new()),
        [values] => (
            Some(numeric_data(values, "stats_normal_diagram")?),
            HashMap::new(),
        ),
        [Value::List(items), Value::Record(opts)] if items.is_empty() => {
            (None, opts.as_ref().clone())
        }
        [values, Value::Record(opts)] => (
            Some(numeric_data(values, "stats_normal_diagram")?),
            opts.as_ref().clone(),
        ),
        [_, other] => {
            return Err(BioLangError::type_error(
                format!(
                    "stats_normal_diagram() options must be Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::ArityError,
                "stats_normal_diagram() accepts optional values and options",
                None,
            ))
        }
    };
    let format = opts.get("format").and_then(Value::as_str).unwrap_or("svg");
    let tail = opts.get("tail").and_then(Value::as_str).unwrap_or("two");
    if !matches!(tail, "left" | "right" | "two") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_normal_diagram() tail must be 'left', 'right', or 'two'",
            None,
        ));
    }
    let z = match opts.get("z") {
        None | Some(Value::Nil) => None,
        Some(Value::Int(value)) => Some(*value as f64),
        Some(Value::Float(value)) if value.is_finite() => Some(*value),
        Some(other) => {
            return Err(BioLangError::type_error(
                format!(
                    "stats_normal_diagram() z must be finite numeric, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    match format {
        "ascii" => Ok(text(normal_diagram_ascii(data.as_ref(), &opts, z, tail))),
        "svg" => Ok(text(normal_diagram_svg(data.as_ref(), &opts, z, tail))),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_normal_diagram() format must be 'svg' or 'ascii'",
            None,
        )),
    }
}

fn visualize_report(args: Vec<Value>) -> Result<Value> {
    let Value::Record(report) = &args[0] else {
        return Err(BioLangError::type_error(
            format!(
                "stats_visualize() requires an exploration Record, got {}",
                args[0].type_of()
            ),
            None,
        ));
    };
    let opts = options(&args, 1, "stats_visualize")?;
    let format = opts.get("format").and_then(Value::as_str).unwrap_or("svg");
    if !matches!(format, "svg" | "ascii") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_visualize() format must be 'svg' or 'ascii'",
            None,
        ));
    }
    let Some(Value::Record(visual)) = report.get("visual_guide") else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_visualize() report has no visual_guide; pass a record from stat.explore()",
            None,
        ));
    };
    visual.get(format).cloned().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("stats_visualize() report has no {format} visual"),
            None,
        )
    })
}

fn require_table<'a>(value: &'a Value, function: &str) -> Result<&'a Table> {
    match value {
        Value::Table(table) => Ok(table),
        other => Err(BioLangError::type_error(
            format!("{function}() requires a Table, got {}", other.type_of()),
            None,
        )),
    }
}

fn value_label(value: &Value) -> String {
    match value {
        Value::Nil => "<missing>".into(),
        Value::Float(number) if !number.is_finite() => "<non-finite>".into(),
        value => category_label(value).unwrap_or_else(|| format!("<{:?}>", value.type_of())),
    }
}

fn is_missing_value(value: &Value) -> bool {
    matches!(value, Value::Nil) || matches!(value, Value::Float(number) if !number.is_finite())
}

fn column_numeric_data(table: &Table, column: usize) -> NumericData {
    let mut values = Vec::new();
    let mut original_indices = Vec::new();
    let mut missing = 0usize;
    let mut non_finite = 0usize;
    for (row_index, row) in table.rows.iter().enumerate() {
        match row.get(column).unwrap_or(&Value::Nil) {
            Value::Nil => missing += 1,
            Value::Int(value) => {
                values.push(*value as f64);
                original_indices.push(row_index);
            }
            Value::Float(value) if value.is_finite() => {
                values.push(*value);
                original_indices.push(row_index);
            }
            Value::Float(_) => non_finite += 1,
            _ => {}
        }
    }
    NumericData {
        values,
        original_indices,
        total: table.rows.len(),
        missing,
        non_finite,
    }
}

fn compact_summary(summary: &NumericSummary) -> Value {
    record([
        ("n", Value::Int(summary.n as i64)),
        ("min", Value::Float(summary.min)),
        ("q1", Value::Float(summary.q1)),
        ("median", Value::Float(summary.median)),
        ("q3", Value::Float(summary.q3)),
        ("max", Value::Float(summary.max)),
        ("mean", Value::Float(summary.mean)),
        ("variance", number(summary.variance)),
        ("sd", number(summary.sd)),
        ("iqr", Value::Float(summary.iqr)),
        ("mad", Value::Float(summary.mad)),
        ("skewness", number(summary.skewness)),
        (
            "tukey_review_flags",
            Value::Int(summary.outlier_positions.len() as i64),
        ),
    ])
}

fn option_column(opts: &HashMap<String, Value>, name: &str) -> Option<String> {
    opts.get(name)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

fn checked_column(table: &Table, name: &str, function: &str) -> Result<usize> {
    table.col_index(name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() column '{name}' does not exist"),
            None,
        )
    })
}

fn column_range_violations(
    table: &Table,
    column: usize,
    opts: &HashMap<String, Value>,
) -> (Option<f64>, Option<f64>, usize) {
    let Some(Value::Record(ranges)) = opts.get("ranges") else {
        return (None, None, 0);
    };
    let Some(Value::Record(bounds)) = ranges.get(&table.columns[column]) else {
        return (None, None, 0);
    };
    let minimum = bounds.get("min").and_then(Value::as_float);
    let maximum = bounds.get("max").and_then(Value::as_float);
    let violations = table
        .rows
        .iter()
        .filter_map(|row| row.get(column).and_then(Value::as_float))
        .filter(|value| {
            minimum.is_some_and(|minimum| *value < minimum)
                || maximum.is_some_and(|maximum| *value > maximum)
        })
        .count();
    (minimum, maximum, violations)
}

fn design_record(table: &Table, opts: &HashMap<String, Value>) -> Result<Value> {
    let subject_name = option_column(opts, "subject_column");
    let group_name = option_column(opts, "group_column");
    let batch_name = option_column(opts, "batch_column");
    let time_name = option_column(opts, "time_column");
    let cluster_name = option_column(opts, "cluster_column");
    let replicate_name = option_column(opts, "replicate_column");
    let weights_name = option_column(opts, "weights_column");
    let assignment_name = option_column(opts, "assignment_unit_column");
    let subject = subject_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;
    let group = group_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;
    let batch = batch_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;
    let time = time_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;
    let cluster = cluster_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;
    let replicate = replicate_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;
    let weights = weights_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;
    let assignment_unit = assignment_name
        .as_deref()
        .map(|name| checked_column(table, name, "stats_design_check"))
        .transpose()?;

    let mut issues = Vec::new();
    let mut design_clues = Vec::new();
    let mut group_counts = HashMap::<String, usize>::new();
    let mut group_order = Vec::<String>::new();
    if let Some(column) = group {
        for row in &table.rows {
            if let Some(value) = row.get(column).and_then(category_label) {
                if !group_counts.contains_key(&value) {
                    group_order.push(value.clone());
                }
                *group_counts.entry(value).or_default() += 1;
            }
        }
        if group_counts.len() < 2 {
            issues.push(issue(
                "single_group",
                "The declared group column contains fewer than two observed groups.",
                "A between-group comparison cannot be identified from this table.",
                "blocking",
            ));
        }
        let smallest = group_counts.values().copied().min().unwrap_or(0);
        let largest = group_counts.values().copied().max().unwrap_or(0);
        if smallest > 0 && largest >= smallest * 3 {
            issues.push(issue(
                "unbalanced_groups",
                format!("Largest group has {largest} rows and smallest has {smallest}."),
                "Large imbalance can reduce precision and make model assumptions more influential.",
                "review",
            ));
        }
        if smallest > 0 && smallest < 5 {
            issues.push(issue(
                "small_group",
                format!("At least one group has only {smallest} observed row(s)."),
                "Shape estimates and asymptotic tests are fragile in very small groups.",
                "review",
            ));
        }
        if let Some(control_level) = opts.get("control_level").and_then(Value::as_str) {
            if !group_counts.contains_key(control_level) {
                issues.push(issue(
                    "declared_control_absent",
                    format!("Declared control level '{control_level}' was not observed."),
                    "Verify coding, filtering, and whether a contemporaneous control is present.",
                    "blocking",
                ));
            }
        }
    }

    let mut repeated_subjects = 0usize;
    let mut unique_subjects = 0usize;
    let mut subject_counts = HashMap::<String, usize>::new();
    if let Some(column) = subject {
        for row in &table.rows {
            if let Some(value) = row.get(column).and_then(category_label) {
                *subject_counts.entry(value).or_default() += 1;
            }
        }
        unique_subjects = subject_counts.len();
        repeated_subjects = subject_counts.values().filter(|count| **count > 1).count();
        if repeated_subjects > 0 {
            issues.push(issue(
                "repeated_experimental_units",
                format!("{repeated_subjects} subject/experimental-unit ID(s) occur more than once."),
                "Rows are not independent unless the analysis models the repeated measurements or aggregates at the intended unit.",
                "review",
            ));
        }
    }

    if let (Some(subject_column), Some(group_column)) = (subject, group) {
        let mut subject_groups = HashMap::<String, HashSet<String>>::new();
        for row in &table.rows {
            let Some(subject_value) = row.get(subject_column).and_then(category_label) else {
                continue;
            };
            let Some(group_value) = row.get(group_column).and_then(category_label) else {
                continue;
            };
            subject_groups
                .entry(subject_value)
                .or_default()
                .insert(group_value);
        }
        let paired_subjects = subject_groups
            .values()
            .filter(|groups| groups.len() > 1)
            .count();
        let groups_observed = group_counts.len();
        let complete_blocks = groups_observed > 1
            && !subject_groups.is_empty()
            && subject_groups
                .values()
                .all(|groups| groups.len() == groups_observed);
        if paired_subjects > 0 {
            design_clues.push(record([
                ("id", text("paired_or_crossover_clue")),
                ("subjects_spanning_groups", Value::Int(paired_subjects as i64)),
                ("complete_subject_blocks", Value::Bool(complete_blocks)),
                ("interpretation", text("Some experimental units occur in more than one group; preserve within-unit pairing or crossover structure in estimation and resampling.")),
            ]));
        } else if repeated_subjects > 0 && groups_observed > 1 {
            design_clues.push(record([
                ("id", text("subjects_nested_in_group_clue")),
                ("subjects", Value::Int(subject_groups.len() as i64)),
                ("interpretation", text("Observed subjects occur within one group and may be repeated within that group; rows are nested within experimental units.")),
            ]));
        }
    }

    if let (Some(subject_column), Some(time_column)) = (subject, time) {
        let mut subject_times = HashMap::<String, HashSet<String>>::new();
        for row in &table.rows {
            let Some(subject_value) = row.get(subject_column).and_then(category_label) else {
                continue;
            };
            let Some(time_value) = row.get(time_column).and_then(category_label) else {
                continue;
            };
            subject_times
                .entry(subject_value)
                .or_default()
                .insert(time_value);
        }
        let longitudinal_subjects = subject_times
            .values()
            .filter(|times| times.len() > 1)
            .count();
        if longitudinal_subjects > 0 {
            design_clues.push(record([
                ("id", text("longitudinal_clue")),
                ("subjects_spanning_times", Value::Int(longitudinal_subjects as i64)),
                ("interpretation", text("Some experimental units occur at multiple times; model within-unit dependence and time ordering rather than treating rows as independent.")),
            ]));
        }
    }

    if let Some(cluster_column) = cluster {
        let mut cluster_counts = HashMap::<String, usize>::new();
        for row in &table.rows {
            if let Some(value) = row.get(cluster_column).and_then(category_label) {
                *cluster_counts.entry(value).or_default() += 1;
            }
        }
        let repeated_clusters = cluster_counts.values().filter(|count| **count > 1).count();
        if repeated_clusters > 0 {
            design_clues.push(record([
                ("id", text("clustered_observations_clue")),
                ("clusters", Value::Int(cluster_counts.len() as i64)),
                ("clusters_with_multiple_rows", Value::Int(repeated_clusters as i64)),
                ("interpretation", text("Rows share a declared cluster; uncertainty and validation should preserve the cluster as the resampling or modelling unit where appropriate.")),
            ]));
        }
    }

    if replicate.is_some() {
        design_clues.push(record([
            ("id", text("replicate_role_declared")),
            ("interpretation", text("A replicate column was declared. Confirm whether its levels are technical repeats, biological replicates, or independent experimental units before aggregation.")),
        ]));
    }

    if let Some(assignment_column) = assignment_unit {
        let mut assignment_counts = HashMap::<String, usize>::new();
        for row in &table.rows {
            if let Some(value) = row.get(assignment_column).and_then(category_label) {
                *assignment_counts.entry(value).or_default() += 1;
            }
        }
        design_clues.push(record([
            ("id", text("assignment_unit_declared")),
            ("assignment_units", Value::Int(assignment_counts.len() as i64)),
            ("interpretation", text("Treatment assignment occurs at the declared unit; effective replication and randomization checks must use that unit rather than automatically using rows.")),
        ]));
    }

    if let Some(weights_column) = weights {
        let mut valid = Vec::new();
        let mut invalid = 0usize;
        for row in &table.rows {
            match row.get(weights_column).and_then(finite_number) {
                Some(value) if value > 0.0 => valid.push(value),
                _ => invalid += 1,
            }
        }
        if invalid > 0 {
            issues.push(issue(
                "invalid_sampling_weights",
                format!("{invalid} declared sampling-weight value(s) are missing, non-finite, or non-positive."),
                "The target population and weighted estimand are undefined until weight provenance and validity are resolved.",
                "blocking",
            ));
        }
        let ratio = valid
            .iter()
            .copied()
            .min_by(f64::total_cmp)
            .zip(valid.iter().copied().max_by(f64::total_cmp))
            .map(|(minimum, maximum)| maximum / minimum);
        design_clues.push(record([
            ("id", text("sampling_weights_declared")),
            ("valid_weights", Value::Int(valid.len() as i64)),
            ("maximum_minimum_ratio", number(ratio)),
            ("interpretation", text("Point estimates, uncertainty, and effective sample size may need the supplied sampling or inverse-probability weights; clarify their construction and target population.")),
        ]));
    }

    let randomized = opts.get("randomized").and_then(Value::as_bool);
    let blinded = opts.get("blinded").and_then(Value::as_bool);
    let sampling_method = opts
        .get("sampling_method")
        .and_then(Value::as_str)
        .unwrap_or("not supplied");
    if group.is_some() && randomized.is_none() {
        design_clues.push(record([
            ("id", text("assignment_process_unspecified")),
            ("interpretation", text("The table cannot reveal whether group assignment was randomized, observational, blocked, matched, or otherwise constrained.")),
        ]));
    }

    if let (Some(batch_column), Some(group_column)) = (batch, group) {
        let mut batch_groups = HashMap::<String, HashSet<String>>::new();
        for row in &table.rows {
            let Some(batch_value) = row.get(batch_column).and_then(category_label) else {
                continue;
            };
            let Some(group_value) = row.get(group_column).and_then(category_label) else {
                continue;
            };
            batch_groups
                .entry(batch_value)
                .or_default()
                .insert(group_value);
        }
        let observed_groups = batch_groups
            .values()
            .flat_map(|groups| groups.iter().cloned())
            .collect::<HashSet<_>>();
        if batch_groups.len() > 1
            && observed_groups.len() > 1
            && batch_groups.values().all(|groups| groups.len() == 1)
        {
            issues.push(issue(
                "batch_group_confounding",
                "Every observed batch contains only one observed group.",
                "The table cannot separate a group effect from a batch effect without overlap or external assumptions.",
                "blocking",
            ));
        }
    }

    if let (Some(subject_column), Some(time_column)) = (subject, time) {
        let mut combinations = HashSet::new();
        let mut duplicate_subject_times = 0usize;
        for row in &table.rows {
            let Some(subject_value) = row.get(subject_column).and_then(category_label) else {
                continue;
            };
            let Some(time_value) = row.get(time_column).and_then(category_label) else {
                continue;
            };
            if !combinations.insert((subject_value, time_value)) {
                duplicate_subject_times += 1;
            }
        }
        if duplicate_subject_times > 0 {
            issues.push(issue(
                "duplicate_subject_time",
                format!("{duplicate_subject_times} repeated subject/time combination(s) were found."),
                "Clarify whether these are technical replicates, duplicate rows, or distinct observations.",
                "review",
            ));
        }
    }

    let groups = group_order
        .into_iter()
        .map(|name| {
            let count = group_counts[&name];
            record([("name", text(name)), ("rows", Value::Int(count as i64))])
        })
        .collect::<Vec<_>>();
    let supplied = [
        subject_name.as_deref(),
        group_name.as_deref(),
        batch_name.as_deref(),
        time_name.as_deref(),
        cluster_name.as_deref(),
        replicate_name.as_deref(),
        weights_name.as_deref(),
        assignment_name.as_deref(),
    ]
    .iter()
    .flatten()
    .count();
    let explanation = format!(
        "Design check\n\nRows: {}\nContext columns supplied: {}\nRepeated experimental units: {}\nIssue clues: {}\n\nThese are structural checks, not proof that a design is valid or invalid.",
        table.rows.len(), supplied, repeated_subjects, issues.len()
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("design_check")),
        ("rows", Value::Int(table.rows.len() as i64)),
        ("groups", list(groups)),
        ("repeated_subjects", Value::Int(repeated_subjects as i64)),
        ("unique_subjects", Value::Int(unique_subjects as i64)),
        ("design_clues", list(design_clues)),
        ("independence_established", Value::Bool(false)),
        (
            "declared_context",
            record([
                ("randomized", randomized.map(Value::Bool).unwrap_or(Value::Nil)),
                ("blinded", blinded.map(Value::Bool).unwrap_or(Value::Nil)),
                ("sampling_method", text(sampling_method)),
                (
                    "control_level",
                    opts.get("control_level")
                        .and_then(Value::as_str)
                        .map(text)
                        .unwrap_or(Value::Nil),
                ),
                (
                    "weights_column",
                    weights_name.as_deref().map(text).unwrap_or(Value::Nil),
                ),
                (
                    "assignment_unit_column",
                    assignment_name.as_deref().map(text).unwrap_or(Value::Nil),
                ),
            ]),
        ),
        ("issues", list(issues)),
        ("automatic_test_selection", Value::Bool(false)),
        (
            "required_context",
            string_list([
                "experimental unit and subject identifier",
                "biological versus technical replicate",
                "group allocation and randomisation",
                "batch, time, and blocking variables",
                "planned estimand and dependence structure",
                "sampling frame, inclusion probabilities, controls, randomization, and blinding",
            ]),
        ),
        (
            "quick_explanation",
            text("Study-design structure was checked from the supplied column roles; no test was selected."),
        ),
        ("explanation", text(explanation)),
    ]))
}

fn design_check(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_design_check")?;
    let opts = options(&args, 1, "stats_design_check")?;
    design_record(table, &opts)
}

fn missingness_record(table: &Table, opts: &HashMap<String, Value>) -> Result<Value> {
    let max_columns = opts
        .get("max_missingness_columns")
        .and_then(Value::as_int)
        .unwrap_or(50)
        .clamp(1, 500) as usize;
    let checked_columns = table.columns.len().min(max_columns);
    let mut columns = Vec::new();
    let mut missing_by_row = Vec::with_capacity(table.rows.len());
    for row in &table.rows {
        missing_by_row.push(
            (0..table.columns.len())
                .filter(|column| match row.get(*column).unwrap_or(&Value::Nil) {
                    Value::Nil => true,
                    Value::Float(value) => !value.is_finite(),
                    _ => false,
                })
                .count(),
        );
    }
    for (column, name) in table.columns.iter().enumerate() {
        let missing = table
            .rows
            .iter()
            .filter(|row| match row.get(column).unwrap_or(&Value::Nil) {
                Value::Nil => true,
                Value::Float(value) => !value.is_finite(),
                _ => false,
            })
            .count();
        columns.push(record([
            ("name", text(name)),
            ("missing", Value::Int(missing as i64)),
            (
                "fraction",
                Value::Float(if table.rows.is_empty() {
                    0.0
                } else {
                    missing as f64 / table.rows.len() as f64
                }),
            ),
        ]));
    }
    let mut co_missing = Vec::new();
    for left in 0..checked_columns {
        for right in left + 1..checked_columns {
            let count = table
                .rows
                .iter()
                .filter(|row| {
                    let is_missing = |column: usize| match row.get(column).unwrap_or(&Value::Nil) {
                        Value::Nil => true,
                        Value::Float(value) => !value.is_finite(),
                        _ => false,
                    };
                    is_missing(left) && is_missing(right)
                })
                .count();
            if count > 0 {
                co_missing.push(record([
                    ("left", text(&table.columns[left])),
                    ("right", text(&table.columns[right])),
                    ("rows", Value::Int(count as i64)),
                ]));
            }
        }
    }

    let max_patterns = opts
        .get("max_missingness_patterns")
        .and_then(Value::as_int)
        .unwrap_or(20)
        .clamp(1, 100) as usize;
    let mut pattern_counts = HashMap::<String, (Vec<String>, usize)>::new();
    for row in &table.rows {
        let names = (0..checked_columns)
            .filter(|column| is_missing_value(row.get(*column).unwrap_or(&Value::Nil)))
            .map(|column| table.columns[column].clone())
            .collect::<Vec<_>>();
        let key = names.join("\u{1f}");
        let entry = pattern_counts.entry(key).or_insert((names, 0));
        entry.1 += 1;
    }
    let mut pattern_values = pattern_counts.into_values().collect::<Vec<_>>();
    pattern_values.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.0.cmp(&right.0)));
    let patterns_total = pattern_values.len();
    let patterns = pattern_values
        .into_iter()
        .take(max_patterns)
        .map(|(names, count)| {
            record([
                ("missing_columns", string_list(names)),
                ("rows", Value::Int(count as i64)),
                (
                    "fraction",
                    Value::Float(if table.rows.is_empty() {
                        0.0
                    } else {
                        count as f64 / table.rows.len() as f64
                    }),
                ),
            ])
        })
        .collect::<Vec<_>>();

    // Compare numeric values between rows where another column is observed or
    // missing. This can reveal informative missingness, but cannot identify the
    // missing-data mechanism without design and collection-process knowledge.
    let max_comparisons = opts
        .get("max_missingness_comparisons")
        .and_then(Value::as_int)
        .unwrap_or(100)
        .clamp(1, 1000) as usize;
    let mut comparisons = Vec::new();
    'missing_column: for missing_column in 0..checked_columns {
        let missing_count = table
            .rows
            .iter()
            .filter(|row| is_missing_value(row.get(missing_column).unwrap_or(&Value::Nil)))
            .count();
        if missing_count == 0 || missing_count == table.rows.len() {
            continue;
        }
        for numeric_column in 0..checked_columns {
            if numeric_column == missing_column
                || screen_column_kind(table, numeric_column) != Some(ScreenColumnKind::Numeric)
            {
                continue;
            }
            let mut observed_group = Vec::new();
            let mut missing_group = Vec::new();
            for row in &table.rows {
                let Some(value) = row.get(numeric_column).and_then(finite_number) else {
                    continue;
                };
                if is_missing_value(row.get(missing_column).unwrap_or(&Value::Nil)) {
                    missing_group.push(value);
                } else {
                    observed_group.push(value);
                }
            }
            if observed_group.len() < 2 || missing_group.len() < 2 {
                continue;
            }
            let observed_data = NumericData {
                values: observed_group,
                original_indices: Vec::new(),
                total: 0,
                missing: 0,
                non_finite: 0,
            };
            let missing_data = NumericData {
                values: missing_group,
                original_indices: Vec::new(),
                total: 0,
                missing: 0,
                non_finite: 0,
            };
            let observed_summary = summarize(&observed_data);
            let missing_summary = summarize(&missing_data);
            // A group with a single observation contributes no variance, so it
            // adds nothing to the pooled estimate rather than being counted as
            // a measured spread of zero.
            let pooled_sd = (((observed_summary.n - 1) as f64
                * observed_summary.variance.unwrap_or(0.0)
                + (missing_summary.n - 1) as f64 * missing_summary.variance.unwrap_or(0.0))
                / (observed_summary.n + missing_summary.n - 2) as f64)
                .sqrt();
            let standardized_difference = if pooled_sd > f64::EPSILON {
                Some((missing_summary.mean - observed_summary.mean) / pooled_sd)
            } else {
                None
            };
            comparisons.push(record([
                ("missingness_column", text(&table.columns[missing_column])),
                ("numeric_column", text(&table.columns[numeric_column])),
                ("observed_rows", Value::Int(observed_summary.n as i64)),
                ("missing_rows", Value::Int(missing_summary.n as i64)),
                ("observed_mean", Value::Float(observed_summary.mean)),
                ("missing_mean", Value::Float(missing_summary.mean)),
                ("observed_median", Value::Float(observed_summary.median)),
                ("missing_median", Value::Float(missing_summary.median)),
                (
                    "standardized_mean_difference",
                    number(standardized_difference),
                ),
            ]));
            if comparisons.len() >= max_comparisons {
                break 'missing_column;
            }
        }
    }

    let mut by_group = Vec::new();
    if let Some(group_name) = option_column(opts, "group_column") {
        let group_column = checked_column(table, &group_name, "stats_missingness")?;
        let mut grouped = HashMap::<String, Vec<usize>>::new();
        let mut group_order = Vec::new();
        for (row_index, row) in table.rows.iter().enumerate() {
            if let Some(group) = row.get(group_column).and_then(category_label) {
                if !grouped.contains_key(&group) {
                    group_order.push(group.clone());
                }
                grouped.entry(group).or_default().push(row_index);
            }
        }
        for group in group_order {
            let rows = &grouped[&group];
            for (column, name) in table.columns.iter().enumerate() {
                if column == group_column {
                    continue;
                }
                let missing = rows
                    .iter()
                    .filter(|row_index| {
                        match table.rows[**row_index].get(column).unwrap_or(&Value::Nil) {
                            Value::Nil => true,
                            Value::Float(value) => !value.is_finite(),
                            _ => false,
                        }
                    })
                    .count();
                by_group.push(record([
                    ("group", text(&group)),
                    ("column", text(name)),
                    ("rows", Value::Int(rows.len() as i64)),
                    ("missing", Value::Int(missing as i64)),
                    ("fraction", Value::Float(missing as f64 / rows.len() as f64)),
                ]));
            }
        }
    }
    let complete_rows = missing_by_row.iter().filter(|count| **count == 0).count();
    let explanation = format!(
        "Missingness map\n\nRows: {}\nComplete rows: {}\nRows with at least one missing/non-finite value: {}\nCo-missing column pairs: {}\n\nDifferent missingness rates are clues only; the missing-data mechanism cannot be established from this table alone.",
        table.rows.len(),
        complete_rows,
        table.rows.len() - complete_rows,
        co_missing.len()
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("missingness")),
        ("rows", Value::Int(table.rows.len() as i64)),
        ("columns", list(columns)),
        (
            "missing_by_row",
            list(missing_by_row.into_iter().map(|count| Value::Int(count as i64)).collect()),
        ),
        ("complete_rows", Value::Int(complete_rows as i64)),
        ("co_missing", list(co_missing)),
        ("by_group", list(by_group)),
        ("patterns", list(patterns)),
        ("patterns_total", Value::Int(patterns_total as i64)),
        (
            "patterns_truncated",
            Value::Bool(patterns_total > max_patterns),
        ),
        ("observed_missing_comparisons", list(comparisons)),
        ("columns_checked_for_pairs", Value::Int(checked_columns as i64)),
        (
            "columns_truncated_for_pairs",
            Value::Int(table.columns.len().saturating_sub(checked_columns) as i64),
        ),
        ("mechanism_diagnosed", Value::Bool(false)),
        (
            "quick_explanation",
            text("Missing and non-finite values were mapped by row, column, pair, and optional group; no imputation was performed."),
        ),
        ("explanation", text(explanation)),
    ]))
}

fn missingness_report(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_missingness")?;
    let opts = options(&args, 1, "stats_missingness")?;
    missingness_record(table, &opts)
}

fn profile_table(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_profile")?;
    let opts = options(&args, 1, "stats_profile")?;
    let types = table.column_types();
    let mut columns = Vec::new();
    let mut issues = Vec::new();
    for (column, name) in table.columns.iter().enumerate() {
        let values = table
            .rows
            .iter()
            .map(|row| row.get(column).unwrap_or(&Value::Nil))
            .collect::<Vec<_>>();
        let missing = values
            .iter()
            .filter(|value| matches!(value, Value::Nil))
            .count();
        let non_finite = values
            .iter()
            .filter(|value| matches!(value, Value::Float(number) if !number.is_finite()))
            .count();
        let labels = values
            .iter()
            .filter(|value| match value {
                Value::Nil => false,
                Value::Float(number) => number.is_finite(),
                _ => true,
            })
            .map(|value| value_label(value))
            .collect::<HashSet<_>>();
        let used = table.rows.len().saturating_sub(missing + non_finite);
        let id_like = used >= 5 && labels.len() == used;
        let constant = used > 0 && labels.len() == 1;
        let (expected_min, expected_max, range_violations) =
            column_range_violations(table, column, &opts);
        if missing + non_finite > 0 {
            issues.push(issue(
                "column_missingness",
                format!("Column '{name}' has {} missing/non-finite value(s).", missing + non_finite),
                "Inspect whether missingness varies by group, batch, outcome, or measurement quality.",
                "review",
            ));
        }
        if types[column] == "any" {
            issues.push(issue(
                "mixed_column_type",
                format!("Column '{name}' contains incompatible value types."),
                "Mixed types often arise from parsing, sentinel strings, or schema drift.",
                "review",
            ));
        }
        if constant {
            issues.push(issue(
                "constant_column",
                format!("Column '{name}' is constant among observed values."),
                "It contributes no variation to association, scaling, or prediction.",
                "review",
            ));
        }
        if range_violations > 0 {
            issues.push(issue(
                "expected_range_violation",
                format!("Column '{name}' has {range_violations} value(s) outside its declared range."),
                "Verify units, coding, entry errors, and whether the declared scientific range is correct.",
                "review",
            ));
        }
        let summary = if matches!(types[column], "int" | "dbl") {
            let data = column_numeric_data(table, column);
            if data.values.is_empty() {
                Value::Nil
            } else {
                compact_summary(&summarize(&data))
            }
        } else {
            Value::Nil
        };
        columns.push(record([
            ("name", text(name)),
            ("type", text(types[column])),
            ("rows", Value::Int(table.rows.len() as i64)),
            ("used", Value::Int(used as i64)),
            ("missing", Value::Int(missing as i64)),
            ("non_finite", Value::Int(non_finite as i64)),
            ("unique", Value::Int(labels.len() as i64)),
            ("constant", Value::Bool(constant)),
            ("id_like", Value::Bool(id_like)),
            ("expected_min", number(expected_min)),
            ("expected_max", number(expected_max)),
            ("range_violations", Value::Int(range_violations as i64)),
            ("summary", summary),
        ]));
    }

    let mut seen_rows = HashSet::new();
    let mut duplicate_rows = 0usize;
    for row in &table.rows {
        let key = row
            .iter()
            .map(value_label)
            .collect::<Vec<_>>()
            .join("\u{1f}");
        if !seen_rows.insert(key) {
            duplicate_rows += 1;
        }
    }
    if duplicate_rows > 0 {
        issues.push(issue(
            "duplicate_rows",
            format!("{duplicate_rows} row(s) duplicate an earlier row across all columns."),
            "Duplicates may be valid repeated records; verify identifiers and replicate definitions before removal.",
            "review",
        ));
    }
    let mut name_counts = HashMap::<&str, usize>::new();
    for name in &table.columns {
        *name_counts.entry(name).or_default() += 1;
    }
    let duplicate_names = name_counts.values().filter(|count| **count > 1).count();
    if duplicate_names > 0 {
        issues.push(issue(
            "duplicate_column_names",
            format!("{duplicate_names} column name(s) are repeated."),
            "Ambiguous names can make downstream column selection target the wrong variable.",
            "blocking",
        ));
    }
    let missingness = missingness_record(table, &opts)?;
    let design = design_record(table, &opts)?;
    let explanation = format!(
        "Dataset profile\n\nRows: {}\nColumns: {}\nDuplicate rows: {}\nObservable issue clues: {}\n\nThe profile uses every row. ID-like, outlier, range, and design labels are review clues rather than automatic cleaning decisions.",
        table.rows.len(), table.columns.len(), duplicate_rows, issues.len()
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("table_profile")),
        ("rows", Value::Int(table.rows.len() as i64)),
        ("n_columns", Value::Int(table.columns.len() as i64)),
        ("columns", list(columns)),
        ("duplicate_rows", Value::Int(duplicate_rows as i64)),
        ("duplicate_column_names", Value::Int(duplicate_names as i64)),
        ("issues", list(issues)),
        ("missingness", missingness),
        ("design", design),
        ("automatic_changes", Value::Bool(false)),
        (
            "quick_explanation",
            text("Every table column was profiled and structural review clues were recorded; no values or rows were changed."),
        ),
        ("explanation", text(explanation)),
    ]))
}

fn transform_preview(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_transform_preview")?;
    let method = match &args[1] {
        Value::Str(method) => method.as_str(),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "stats_transform_preview() method must be Str, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let opts = options(&args, 2, "stats_transform_preview")?;
    let before = summarize(&data);
    let (values, formula, changes, caution) = match method {
        "log" => {
            if before.min <= 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "stats_transform_preview() log requires every finite value to be greater than zero",
                    None,
                ));
            }
            (
                data.values.iter().map(|value| value.ln()).collect(),
                "log(x)",
                "Ratios on the original scale become differences and high values are compressed.",
                "The original units and additive interpretation are lost.",
            )
        }
        "log1p" => {
            if before.min < 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "stats_transform_preview() log1p requires non-negative finite values",
                    None,
                ));
            }
            (
                data.values.iter().map(|value| value.ln_1p()).collect(),
                "log(1 + x)",
                "Zero is retained and high values are compressed.",
                "The added one is scale-dependent and can strongly alter continuous values below one.",
            )
        }
        "sqrt" => {
            if before.min < 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "stats_transform_preview() sqrt requires non-negative finite values",
                    None,
                ));
            }
            (
                data.values.iter().map(|value| value.sqrt()).collect(),
                "sqrt(x)",
                "Large non-negative values are compressed less strongly than by a log transform.",
                "Units change to square-root units and the scientific estimand may change.",
            )
        }
        "zscore" => {
            if before.sd.is_none_or(|sd| sd <= f64::EPSILON) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "stats_transform_preview() zscore is undefined for zero-variance data",
                    None,
                ));
            }
            (
                data.values
                    .iter()
                    .map(|value| (value - before.mean) / before.sd.unwrap_or(1.0))
                    .collect(),
                "(x - mean) / sample_sd",
                "The output is centred at zero with sample SD one.",
                "Skewness and influential observations are not repaired, and original units are removed.",
            )
        }
        "robust" => {
            let scale = before.mad * 1.4826;
            if scale <= f64::EPSILON {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "stats_transform_preview() robust scaling is undefined when MAD is zero",
                    None,
                ));
            }
            (
                data.values
                    .iter()
                    .map(|value| (value - before.median) / scale)
                    .collect(),
                "(x - median) / (1.4826 * MAD)",
                "Median-centred distances are expressed in normal-consistent MAD units.",
                "The scale may be zero for discrete/heaped data and original units are removed.",
            )
        }
        "minmax" => {
            let span = before.max - before.min;
            if span <= f64::EPSILON {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "stats_transform_preview() minmax is undefined for constant data",
                    None,
                ));
            }
            (
                data.values
                    .iter()
                    .map(|value| (value - before.min) / span)
                    .collect(),
                "(x - min) / (max - min)",
                "The observed minimum becomes zero and maximum becomes one.",
                "The result is highly sensitive to extremes and new values may fall outside zero to one.",
            )
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "stats_transform_preview() unknown method '{method}'; expected log, log1p, sqrt, zscore, robust, or minmax"
                ),
                None,
            ))
        }
    };
    let transformed = NumericData {
        original_indices: data.original_indices.clone(),
        total: data.total,
        missing: data.missing,
        non_finite: data.non_finite,
        values,
    };
    let after = summarize(&transformed);
    let include_values = opts
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let span_ratio = if before.max - before.min > f64::EPSILON {
        Some((after.max - after.min) / (before.max - before.min))
    } else {
        None
    };
    let explanation = format!(
        "Transformation preview: {method}\n\nBefore\n  skewness: {}\n  SD: {}\n  IQR: {}\n\nAfter\n  skewness: {}\n  SD: {}\n  IQR: {}\n\nWhat changes\n  {changes}\n\nCaution\n  {caution}\n\nThe input was not modified.",
        before.skewness.map(fmt_number).unwrap_or_else(|| "not assessed".into()),
        before.sd.map(fmt_number).unwrap_or_else(|| "not defined".into()),
        fmt_number(before.iqr),
        after.skewness.map(fmt_number).unwrap_or_else(|| "not assessed".into()),
        after.sd.map(fmt_number).unwrap_or_else(|| "not defined".into()),
        fmt_number(after.iqr),
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("transform_preview")),
        ("method", text(method)),
        ("formula", text(formula)),
        ("before", compact_summary(&before)),
        ("after", compact_summary(&after)),
        ("changes", text(changes)),
        ("caution", text(caution)),
        ("span_ratio", number(span_ratio)),
        ("zeros_before", Value::Int(before.zero_count as i64)),
        ("zeros_after", Value::Int(after.zero_count as i64)),
        ("rank_order_preserved", Value::Bool(true)),
        ("input_modified", Value::Bool(false)),
        (
            "values",
            if include_values {
                list(transformed.values.into_iter().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        (
            "quick_explanation",
            text(format!(
                "Previewed {method} on all finite values; the input was not modified."
            )),
        ),
        ("explanation", text(explanation)),
    ]))
}

fn lcg_next(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn resample(values: &[f64], state: &mut u64) -> Vec<f64> {
    (0..values.len())
        .map(|_| values[((lcg_next(state) >> 32) as usize) % values.len()])
        .collect()
}

fn statistic(values: &[f64], name: &str) -> Option<f64> {
    match name {
        "mean" => Some(values.iter().sum::<f64>() / values.len() as f64),
        "median" => Some(median(values)),
        "sd" => {
            let data = NumericData {
                values: values.to_vec(),
                original_indices: (0..values.len()).collect(),
                total: values.len(),
                missing: 0,
                non_finite: 0,
            };
            summarize(&data).sd
        }
        _ => None,
    }
}

fn uncertainty_report(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_uncertainty")?;
    let opts = options(&args, 1, "stats_uncertainty")?;
    let statistic_name = opts
        .get("statistic")
        .and_then(Value::as_str)
        .unwrap_or("mean");
    let confidence = opts
        .get("confidence")
        .and_then(Value::as_float)
        .unwrap_or(0.95);
    if !(0.5..1.0).contains(&confidence) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_uncertainty() confidence must be between 0.5 and 1",
            None,
        ));
    }
    let repetitions = opts
        .get("repetitions")
        .and_then(Value::as_int)
        .unwrap_or(2_000)
        .clamp(100, 100_000) as usize;
    let seed = opts.get("seed").and_then(Value::as_int).unwrap_or(42) as u64;
    let mut state = seed;
    let mut estimates = Vec::with_capacity(repetitions);
    let (observed, estimand) = if let Some(other) = opts.get("other") {
        let other = numeric_data(other, "stats_uncertainty")?;
        let base_statistic = match statistic_name {
            "difference_mean" => "mean",
            "difference_median" => "median",
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "stats_uncertainty() with other requires statistic difference_mean or difference_median",
                    None,
                ))
            }
        };
        let observed = statistic(&data.values, base_statistic).unwrap()
            - statistic(&other.values, base_statistic).unwrap();
        for _ in 0..repetitions {
            estimates.push(
                statistic(&resample(&data.values, &mut state), base_statistic).unwrap()
                    - statistic(&resample(&other.values, &mut state), base_statistic).unwrap(),
            );
        }
        (observed, "independent-group difference")
    } else if let Some(y) = opts.get("y") {
        let (xs, ys, _) = complete_pairs(&args[0], y, "stats_uncertainty")?;
        if !matches!(statistic_name, "pearson" | "spearman") {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "stats_uncertainty() with y requires statistic pearson or spearman",
                None,
            ));
        }
        let correlation = |left: &[f64], right: &[f64]| {
            if statistic_name == "spearman" {
                pearson(&average_ranks(left), &average_ranks(right)).map(|value| value.0)
            } else {
                pearson(left, right).map(|value| value.0)
            }
        };
        let observed = correlation(&xs, &ys).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "stats_uncertainty() correlation is undefined for a constant variable",
                None,
            )
        })?;
        for _ in 0..repetitions {
            let mut sample_x = Vec::with_capacity(xs.len());
            let mut sample_y = Vec::with_capacity(ys.len());
            for _ in 0..xs.len() {
                let index = ((lcg_next(&mut state) >> 32) as usize) % xs.len();
                sample_x.push(xs[index]);
                sample_y.push(ys[index]);
            }
            if let Some(value) = correlation(&sample_x, &sample_y) {
                estimates.push(value);
            }
        }
        (observed, "paired-vector correlation")
    } else {
        let observed = statistic(&data.values, statistic_name).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "stats_uncertainty() statistic must be mean, median, or sd",
                None,
            )
        })?;
        for _ in 0..repetitions {
            estimates.push(statistic(&resample(&data.values, &mut state), statistic_name).unwrap());
        }
        (observed, "one-sample statistic")
    };
    if estimates.len() < 100 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_uncertainty() produced too few finite bootstrap estimates",
            None,
        ));
    }
    estimates.sort_by(f64::total_cmp);
    let alpha = (1.0 - confidence) / 2.0;
    let lower = quantile_sorted(&estimates, alpha);
    let upper = quantile_sorted(&estimates, 1.0 - alpha);
    let bootstrap_mean = estimates.iter().sum::<f64>() / estimates.len() as f64;
    let standard_error = (estimates
        .iter()
        .map(|estimate| (estimate - bootstrap_mean).powi(2))
        .sum::<f64>()
        / (estimates.len() - 1) as f64)
        .sqrt();
    let explanation = format!(
        "Bootstrap uncertainty\n\nEstimand: {estimand}\nStatistic: {statistic_name}\nObserved estimate: {}\n{}% percentile interval: {} to {}\nBootstrap standard error: {}\nReplicates: {}\nSeed: {}\n\nThis interval describes sampling uncertainty under the resampling scheme; it does not correct bias, dependence, confounding, or measurement error.",
        fmt_number(observed),
        fmt_number(confidence * 100.0),
        fmt_number(lower),
        fmt_number(upper),
        fmt_number(standard_error),
        estimates.len(),
        seed,
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("uncertainty")),
        ("estimand", text(estimand)),
        ("statistic", text(statistic_name)),
        ("estimate", Value::Float(observed)),
        ("confidence", Value::Float(confidence)),
        ("lower", Value::Float(lower)),
        ("upper", Value::Float(upper)),
        ("standard_error", Value::Float(standard_error)),
        ("repetitions", Value::Int(estimates.len() as i64)),
        ("seed", Value::Int(seed as i64)),
        ("method", text("deterministic percentile bootstrap")),
        (
            "limitations",
            string_list([
                "Rows must represent the resampling unit.",
                "Independent-group resampling does not model pairing.",
                "Percentile intervals can be biased in small or highly skewed samples.",
                "The interval does not include systematic measurement error or confounding.",
            ]),
        ),
        (
            "quick_explanation",
            text(format!(
                "{statistic_name} = {}; interval {} to {}.",
                fmt_number(observed),
                fmt_number(lower),
                fmt_number(upper)
            )),
        ),
        ("explanation", text(explanation)),
    ]))
}

fn shape_evidence(data: &NumericData, bins: usize) -> Value {
    let summary = summarize(data);
    let span = summary.max - summary.min;
    let mut counts = vec![0usize; bins];
    for value in &data.values {
        let index = if span <= f64::EPSILON {
            bins / 2
        } else {
            (((value - summary.min) / span) * bins as f64)
                .floor()
                .clamp(0.0, (bins - 1) as f64) as usize
        };
        counts[index] += 1;
    }
    let max_count = counts.iter().copied().max().unwrap_or(0);
    let threshold = ((max_count as f64) * 0.15).ceil() as usize;
    let candidates = (0..bins)
        .filter(|index| {
            let left = if *index == 0 { 0 } else { counts[*index - 1] };
            let right = if *index + 1 == bins {
                0
            } else {
                counts[*index + 1]
            };
            counts[*index] >= threshold
                && counts[*index] > 0
                && counts[*index] >= left
                && counts[*index] >= right
                && (counts[*index] > left || counts[*index] > right)
        })
        .collect::<Vec<_>>();
    let mut peaks = Vec::new();
    for candidate in candidates {
        if let Some(previous) = peaks.last_mut() {
            if candidate <= *previous + 1 {
                if counts[candidate] > counts[*previous] {
                    *previous = candidate;
                }
                continue;
            }
        }
        peaks.push(candidate);
    }
    let mean = summary.mean;
    let m2 = data
        .values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / data.values.len() as f64;
    let excess_kurtosis = if m2 <= f64::EPSILON {
        None
    } else {
        Some(
            data.values
                .iter()
                .map(|value| (value - mean).powi(4))
                .sum::<f64>()
                / data.values.len() as f64
                / m2.powi(2)
                - 3.0,
        )
    };
    let mut observed = data.values.clone();
    observed.sort_by(f64::total_cmp);
    let expected = (0..observed.len())
        .map(|index| {
            let probability = (index as f64 + 0.5) / observed.len() as f64;
            bl_core::bio_core::stats_ops::normal_quantile(probability)
        })
        .collect::<Vec<_>>();
    let qq_correlation = pearson(&expected, &observed).map(|value| value.0);
    let multi_peak_clue = data.values.len() >= 20 && peaks.len() >= 2;
    record([
        ("bins", Value::Int(bins as i64)),
        (
            "histogram_counts",
            list(
                counts
                    .into_iter()
                    .map(|count| Value::Int(count as i64))
                    .collect(),
            ),
        ),
        (
            "peak_bins",
            list(
                peaks
                    .into_iter()
                    .map(|peak| Value::Int(peak as i64))
                    .collect(),
            ),
        ),
        ("multiple_peak_clue", Value::Bool(multi_peak_clue)),
        ("multimodality_diagnosed", Value::Bool(false)),
        ("excess_kurtosis", number(excess_kurtosis)),
        ("normal_qq_correlation", number(qq_correlation)),
        ("normality_diagnosed", Value::Bool(false)),
    ])
}

fn shape_diagnostics(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_shape")?;
    let opts = options(&args, 1, "stats_shape")?;
    let bins = opts
        .get("bins")
        .and_then(Value::as_int)
        .map(|value| value.clamp(5, 100) as usize)
        .unwrap_or_else(|| (data.values.len() as f64).sqrt().round().clamp(5.0, 40.0) as usize);
    let evidence = shape_evidence(&data, bins);
    let multi_peak = match &evidence {
        Value::Record(record) => record
            .get("multiple_peak_clue")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        _ => false,
    };
    let explanation = format!(
        "Shape diagnostics\n\nFinite observations: {}\nHistogram bins: {}\nMultiple-peak clue: {}\n\nHistogram peaks, skewness, kurtosis, and normal Q-Q alignment are descriptive evidence. None establishes a probability distribution or distinct biological populations.",
        data.values.len(), bins, if multi_peak { "present" } else { "not detected at this bin width" }
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("shape_diagnostics")),
        ("summary", compact_summary(&summarize(&data))),
        ("evidence", evidence),
        (
            "sensitivity",
            text("Peak counts depend on histogram bin width; compare several reasonable widths and inspect the raw observations."),
        ),
        (
            "quick_explanation",
            text("Distribution-shape evidence was calculated without assigning a diagnostic distribution label."),
        ),
        ("explanation", text(explanation)),
    ]))
}

fn plot_format(opts: &HashMap<String, Value>) -> &str {
    opts.get("format").and_then(Value::as_str).unwrap_or("svg")
}

fn padded_domain(minimum: f64, maximum: f64) -> (f64, f64) {
    let span = maximum - minimum;
    let padding = if span.abs() <= f64::EPSILON {
        maximum.abs().max(1.0) * 0.1
    } else {
        span * 0.08
    };
    (minimum - padding, maximum + padding)
}

fn ascii_scatter(
    xs: &[f64],
    ys: &[f64],
    width: usize,
    height: usize,
    title: &str,
    x_label: &str,
    y_label: &str,
) -> String {
    let min_x = xs.iter().copied().min_by(f64::total_cmp).unwrap_or(0.0);
    let max_x = xs.iter().copied().max_by(f64::total_cmp).unwrap_or(1.0);
    let min_y = ys.iter().copied().min_by(f64::total_cmp).unwrap_or(0.0);
    let max_y = ys.iter().copied().max_by(f64::total_cmp).unwrap_or(1.0);
    let x_span = (max_x - min_x).abs();
    let y_span = (max_y - min_y).abs();
    let mut grid = vec![vec![' '; width]; height];
    for (x, y) in xs.iter().zip(ys) {
        let column = if x_span <= f64::EPSILON {
            width / 2
        } else {
            (((x - min_x) / x_span) * (width - 1) as f64).round() as usize
        };
        let row = if y_span <= f64::EPSILON {
            height / 2
        } else {
            height - 1 - (((y - min_y) / y_span) * (height - 1) as f64).round() as usize
        };
        grid[row][column] = if grid[row][column] == ' ' { '*' } else { '#' };
    }
    let mut output = format!(
        "{title}\n{y_label}: {} to {}\n",
        fmt_number(min_y),
        fmt_number(max_y)
    );
    for row in grid {
        output.push('|');
        output.extend(row);
        output.push_str("|\n");
    }
    output.push('+');
    output.push_str(&"-".repeat(width));
    output.push_str("+\n");
    output.push_str(&format!(
        "{x_label}: {} to {} | * one point, # overlapping points",
        fmt_number(min_x),
        fmt_number(max_x)
    ));
    output
}

fn normal_qq_values(data: &NumericData) -> (Vec<f64>, Vec<f64>) {
    normal_qq_geometry(&data.values)
        .map(|geometry| (geometry.theoretical, geometry.sample))
        .unwrap_or_default()
}

fn normal_qq_plot(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_normal_qq_plot")?;
    if data.values.len() < 3 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_normal_qq_plot() requires at least three finite values",
            None,
        ));
    }
    let opts = options(&args, 1, "stats_normal_qq_plot")?;
    let geometry = normal_qq_geometry(&data.values)?;
    let expected = &geometry.theoretical;
    let observed = &geometry.sample;
    if plot_format(&opts) == "ascii" {
        return Ok(text(ascii_scatter(
            expected,
            observed,
            opts.get("width")
                .and_then(Value::as_int)
                .unwrap_or(56)
                .clamp(20, 100) as usize,
            opts.get("height")
                .and_then(Value::as_int)
                .unwrap_or(16)
                .clamp(8, 30) as usize,
            "Normal Q-Q diagnostic (ASCII)",
            "theoretical normal quantile",
            "observed value",
        )));
    }
    let width = opts
        .get("width")
        .and_then(Value::as_float)
        .unwrap_or(720.0)
        .max(480.0);
    let height = opts
        .get("height")
        .and_then(Value::as_float)
        .unwrap_or(500.0)
        .max(360.0);
    let title = opts
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Normal Q-Q diagnostic");
    let x_domain = padded_domain(expected[0], expected[expected.len() - 1]);
    let y_domain = padded_domain(observed[0], observed[observed.len() - 1]);
    let x_scale = Scale {
        domain: x_domain,
        range: (65.0, width - 30.0),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (height - 60.0, 45.0),
    };
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 65.0;
    canvas.margin.right = 30.0;
    canvas.margin.top = 45.0;
    canvas.margin.bottom = 60.0;
    canvas.add_line(
        x_scale.map(x_domain.0),
        y_scale.map(geometry.line_intercept + geometry.line_slope * x_domain.0),
        x_scale.map(x_domain.1),
        y_scale.map(geometry.line_intercept + geometry.line_slope * x_domain.1),
        "#94a3b8",
        2.0,
    );
    let stride = expected.len().div_ceil(5_000).max(1);
    for (x, y) in expected.iter().zip(observed.iter()).step_by(stride) {
        canvas.add_circle(x_scale.map(*x), y_scale.map(*y), 3.0, "#2563eb");
    }
    canvas.draw_x_axis(&x_scale, "theoretical normal quantile");
    canvas.draw_y_axis(&y_scale, "observed value");
    canvas.draw_title(title);
    canvas.add_text(
        68.0,
        height - 38.0,
        "Curvature is a clue; this plot does not diagnose normality.",
        "start",
        10.0,
    );
    Ok(text(canvas.render()))
}

fn relationship_diagnostic_plot(args: Vec<Value>) -> Result<Value> {
    let (xs, ys, excluded) = complete_pairs(&args[0], &args[1], "stats_relationship_plot")?;
    let opts = options(&args, 2, "stats_relationship_plot")?;
    let interval = opts
        .get("interval")
        .and_then(Value::as_str)
        .unwrap_or("none");
    if !matches!(interval, "none" | "confidence" | "prediction") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_relationship_plot() interval must be none, confidence, or prediction",
            None,
        ));
    }
    let confidence = opts
        .get("confidence")
        .and_then(Value::as_float)
        .unwrap_or(0.95);
    let observed_min_x = xs.iter().copied().min_by(f64::total_cmp).unwrap();
    let observed_max_x = xs.iter().copied().max_by(f64::total_cmp).unwrap();
    let fit_at = (0..=100)
        .map(|index| observed_min_x + (observed_max_x - observed_min_x) * index as f64 / 100.0)
        .collect::<Vec<_>>();
    if interval != "none" && xs.len() < 3 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_relationship_plot() interval bands require at least three complete pairs",
            None,
        ));
    }
    let fit = if xs.len() >= 3 {
        Some(linear_fit_geometry(&xs, &ys, &fit_at, confidence)?)
    } else {
        None
    };
    let (_, fallback_slope, fallback_intercept) = pearson(&xs, &ys).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_relationship_plot() requires variation in both x and y",
            None,
        )
    })?;
    let slope = fit.as_ref().map_or(fallback_slope, |value| value.slope);
    let intercept = fit
        .as_ref()
        .map_or(fallback_intercept, |value| value.intercept);
    if plot_format(&opts) == "ascii" {
        let mut chart = ascii_scatter(
            &xs,
            &ys,
            opts.get("width")
                .and_then(Value::as_int)
                .unwrap_or(56)
                .clamp(20, 100) as usize,
            opts.get("height")
                .and_then(Value::as_int)
                .unwrap_or(16)
                .clamp(8, 30) as usize,
            "Relationship diagnostic (ASCII)",
            opts.get("x_label").and_then(Value::as_str).unwrap_or("x"),
            opts.get("y_label").and_then(Value::as_str).unwrap_or("y"),
        );
        chart.push_str(&format!(
            "\n{} complete pairs; {} excluded.",
            xs.len(),
            excluded
        ));
        return Ok(text(chart));
    }
    let width = opts
        .get("width")
        .and_then(Value::as_float)
        .unwrap_or(720.0)
        .max(480.0);
    let height = opts
        .get("height")
        .and_then(Value::as_float)
        .unwrap_or(500.0)
        .max(360.0);
    let x_domain = padded_domain(observed_min_x, observed_max_x);
    let mut observed_min_y = ys.iter().copied().min_by(f64::total_cmp).unwrap();
    let mut observed_max_y = ys.iter().copied().max_by(f64::total_cmp).unwrap();
    if interval != "none" {
        for point in &fit.as_ref().expect("interval fit was checked").points {
            let (lower, upper) = if interval == "prediction" {
                (point.prediction_lower, point.prediction_upper)
            } else {
                (point.confidence_lower, point.confidence_upper)
            };
            observed_min_y = observed_min_y.min(lower);
            observed_max_y = observed_max_y.max(upper);
        }
    }
    let y_domain = padded_domain(observed_min_y, observed_max_y);
    let x_scale = Scale {
        domain: x_domain,
        range: (65.0, width - 30.0),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (height - 60.0, 45.0),
    };
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 65.0;
    canvas.margin.right = 30.0;
    canvas.margin.top = 45.0;
    canvas.margin.bottom = 60.0;
    if interval != "none" {
        let fit = fit.as_ref().expect("interval fit was checked");
        let mut band = fit
            .points
            .iter()
            .map(|point| {
                let upper = if interval == "prediction" {
                    point.prediction_upper
                } else {
                    point.confidence_upper
                };
                format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(upper))
            })
            .collect::<Vec<_>>();
        band.extend(fit.points.iter().rev().map(|point| {
            let lower = if interval == "prediction" {
                point.prediction_lower
            } else {
                point.confidence_lower
            };
            format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(lower))
        }));
        canvas.elements.push(format!(
            r##"<polygon points="{}" fill="#bfdbfe" fill-opacity="0.55" stroke="none" />"##,
            band.join(" ")
        ));
    }
    let stride = xs.len().div_ceil(10_000).max(1);
    for (x, y) in xs.iter().zip(&ys).step_by(stride) {
        canvas.add_circle(x_scale.map(*x), y_scale.map(*y), 3.0, "#2563eb");
    }
    canvas.add_line(
        x_scale.map(observed_min_x),
        y_scale.map(intercept + slope * observed_min_x),
        x_scale.map(observed_max_x),
        y_scale.map(intercept + slope * observed_max_x),
        "#dc2626",
        2.0,
    );
    canvas.draw_x_axis(
        &x_scale,
        opts.get("x_label").and_then(Value::as_str).unwrap_or("x"),
    );
    canvas.draw_y_axis(
        &y_scale,
        opts.get("y_label").and_then(Value::as_str).unwrap_or("y"),
    );
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Relationship diagnostic"),
    );
    canvas.add_text(
        68.0,
        height - 38.0,
        &format!(
            "{} complete pairs; {} excluded. Red line: least-squares fit{}.",
            xs.len(),
            excluded,
            if interval == "none" {
                "".to_string()
            } else {
                format!("; blue: {:.0}% {interval} band", confidence * 100.0)
            }
        ),
        "start",
        10.0,
    );
    Ok(text(canvas.render()))
}

fn grouped_values(
    values: &Value,
    groups: &Value,
    function: &str,
) -> Result<Vec<(String, Vec<f64>)>> {
    let data = numeric_data(values, function)?;
    let Value::List(group_values) = groups else {
        return Err(BioLangError::type_error(
            format!("{function}() groups must be List, got {}", groups.type_of()),
            None,
        ));
    };
    if group_values.len() != data.total {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() values and groups must have equal length"),
            None,
        ));
    }
    let mut labels = Vec::new();
    let mut positions = HashMap::new();
    let mut result = Vec::<(String, Vec<f64>)>::new();
    for (clean_index, original_index) in data.original_indices.iter().enumerate() {
        let Some(label) = group_values.get(*original_index).and_then(category_label) else {
            continue;
        };
        let position = *positions.entry(label.clone()).or_insert_with(|| {
            labels.push(label.clone());
            result.push((label, Vec::new()));
            result.len() - 1
        });
        result[position].1.push(data.values[clean_index]);
    }
    if result.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() has no complete value/group observations"),
            None,
        ));
    }
    Ok(result)
}

fn group_diagnostic_plot(args: Vec<Value>) -> Result<Value> {
    let groups = grouped_values(&args[0], &args[1], "stats_group_plot")?;
    let opts = options(&args, 2, "stats_group_plot")?;
    let all = groups
        .iter()
        .flat_map(|(_, values)| values.iter().copied())
        .collect::<Vec<_>>();
    let minimum = all.iter().copied().min_by(f64::total_cmp).unwrap();
    let maximum = all.iter().copied().max_by(f64::total_cmp).unwrap();
    if plot_format(&opts) == "ascii" {
        let width = opts
            .get("width")
            .and_then(Value::as_int)
            .unwrap_or(48)
            .clamp(20, 90) as usize;
        let span = maximum - minimum;
        let position = |value: f64| {
            if span <= f64::EPSILON {
                width / 2
            } else {
                (((value - minimum) / span) * (width - 1) as f64).round() as usize
            }
        };
        let name_width = groups
            .iter()
            .map(|(name, _)| name.len())
            .max()
            .unwrap_or(5)
            .min(20);
        let mut output = format!(
            "Grouped distribution diagnostic (ASCII)\nscale {} to {}\n",
            fmt_number(minimum),
            fmt_number(maximum)
        );
        for (name, values) in &groups {
            let geometry = box_geometry(name, values, "type7", 1.5);
            let mut line = vec![' '; width];
            for cell in line
                .iter_mut()
                .take(position(geometry.q3) + 1)
                .skip(position(geometry.q1))
            {
                *cell = '=';
            }
            line[position(geometry.whisker_high)] = '|';
            line[position(geometry.whisker_low)] = '|';
            line[position(geometry.q1)] = '[';
            line[position(geometry.q3)] = ']';
            line[position(geometry.median)] = 'M';
            for (_, value) in &geometry.outliers {
                line[position(*value)] = 'o';
            }
            output.push_str(&format!("{:>name_width$} |", name));
            output.extend(line);
            output.push_str(&format!("| n={}\n", values.len()));
        }
        output.push_str(
            "M=median, [ ]=IQR, outer |=1.5 IQR whiskers, o=review flag; type-7 quartiles.",
        );
        return Ok(text(output));
    }
    let width = opts
        .get("width")
        .and_then(Value::as_float)
        .unwrap_or(780.0)
        .max(520.0);
    let height = opts
        .get("height")
        .and_then(Value::as_float)
        .unwrap_or(500.0)
        .max(360.0);
    let y_domain = padded_domain(minimum, maximum);
    let y_scale = Scale {
        domain: y_domain,
        range: (height - 70.0, 45.0),
    };
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 70.0;
    canvas.margin.right = 25.0;
    canvas.margin.top = 45.0;
    canvas.margin.bottom = 70.0;
    let step = canvas.plot_width() / groups.len() as f64;
    for (group_index, (name, values)) in groups.iter().enumerate() {
        let x = canvas.margin.left + step * (group_index as f64 + 0.5);
        let geometry = box_geometry(name, values, "type7", 1.5);
        canvas.add_line(
            x,
            y_scale.map(geometry.whisker_low),
            x,
            y_scale.map(geometry.whisker_high),
            "#1e3a8a",
            1.5,
        );
        canvas.add_rect(
            x - step * 0.22,
            y_scale.map(geometry.q3),
            step * 0.44,
            (y_scale.map(geometry.q1) - y_scale.map(geometry.q3))
                .abs()
                .max(1.0),
            "#bfdbfe",
        );
        canvas.add_line(
            x - step * 0.22,
            y_scale.map(geometry.median),
            x + step * 0.22,
            y_scale.map(geometry.median),
            "#1e3a8a",
            2.0,
        );
        let stride = values.len().div_ceil(1_000).max(1);
        for (index, value) in values.iter().enumerate().step_by(stride) {
            let jitter = (((index * 37) % 101) as f64 / 100.0 - 0.5) * step * 0.32;
            canvas.add_circle(x + jitter, y_scale.map(*value), 2.5, "#475569");
        }
        canvas.add_text(x, height - 42.0, name, "middle", 11.0);
        canvas.add_text(
            x,
            height - 27.0,
            &format!("n={}", values.len()),
            "middle",
            9.0,
        );
    }
    canvas.draw_y_axis(
        &y_scale,
        opts.get("y_label")
            .and_then(Value::as_str)
            .unwrap_or("value"),
    );
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Grouped distribution diagnostic"),
    );
    Ok(text(canvas.render()))
}

fn category_counts(value: &Value, function: &str) -> Result<(Vec<String>, Vec<usize>, usize)> {
    let Value::List(items) = value else {
        return Err(BioLangError::type_error(
            format!("{function}() requires a List, got {}", value.type_of()),
            None,
        ));
    };
    let mut labels = Vec::<String>::new();
    let mut counts = Vec::<usize>::new();
    let mut positions = HashMap::<String, usize>::new();
    let mut missing = 0usize;
    for item in items.iter() {
        let Some(label) = category_label(item) else {
            if matches!(item, Value::Nil) {
                missing += 1;
                continue;
            }
            return Err(BioLangError::type_error(
                format!("{function}() categories must be scalar values or Nil"),
                None,
            ));
        };
        let position = *positions.entry(label.clone()).or_insert_with(|| {
            labels.push(label);
            counts.push(0);
            labels.len() - 1
        });
        counts[position] += 1;
    }
    Ok((labels, counts, missing))
}

fn categorical_diagnostic_plot(args: Vec<Value>) -> Result<Value> {
    let (labels, counts, missing) = category_counts(&args[0], "stats_categorical_plot")?;
    let opts = options(&args, 1, "stats_categorical_plot")?;
    if labels.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_categorical_plot() has no observed categories",
            None,
        ));
    }
    let maximum = counts.iter().copied().max().unwrap_or(1);
    if plot_format(&opts) == "ascii" {
        let width = opts
            .get("width")
            .and_then(Value::as_int)
            .unwrap_or(40)
            .clamp(10, 80) as usize;
        let label_width = labels.iter().map(String::len).max().unwrap_or(5).min(24);
        let mut output = String::from("Categorical frequency diagnostic (ASCII)\n");
        for (label, count) in labels.iter().zip(&counts) {
            let bar = ((*count as f64 / maximum as f64) * width as f64)
                .round()
                .max(1.0) as usize;
            output.push_str(&format!(
                "{:>label_width$} |{} {}\n",
                label,
                "#".repeat(bar),
                count
            ));
        }
        output.push_str(&format!(
            "Missing: {missing}; first-observed category order retained."
        ));
        return Ok(text(output));
    }
    let width = opts
        .get("width")
        .and_then(Value::as_float)
        .unwrap_or(760.0)
        .max(520.0);
    let height = opts
        .get("height")
        .and_then(Value::as_float)
        .unwrap_or(480.0)
        .max(340.0);
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 70.0;
    canvas.margin.right = 25.0;
    canvas.margin.top = 45.0;
    canvas.margin.bottom = 80.0;
    let step = canvas.plot_width() / labels.len() as f64;
    for (index, (label, count)) in labels.iter().zip(&counts).enumerate() {
        let bar_height = *count as f64 / maximum as f64 * canvas.plot_height();
        let x = canvas.margin.left + step * index as f64 + step * 0.12;
        canvas.add_rect(
            x,
            canvas.margin.top + canvas.plot_height() - bar_height,
            step * 0.76,
            bar_height,
            "#60a5fa",
        );
        canvas.add_text(
            x + step * 0.38,
            canvas.margin.top + canvas.plot_height() - bar_height - 5.0,
            &count.to_string(),
            "middle",
            10.0,
        );
        canvas.add_text_rotated(x + step * 0.38, height - 50.0, label, -25.0, "end", 10.0);
    }
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Categorical frequency diagnostic"),
    );
    canvas.add_text(
        72.0,
        height - 15.0,
        &format!("Missing: {missing}; first-observed category order retained."),
        "start",
        10.0,
    );
    Ok(text(canvas.render()))
}

fn missingness_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_missingness_plot")?;
    let opts = options(&args, 1, "stats_missingness_plot")?;
    let max_rows = opts
        .get("max_rows")
        .and_then(Value::as_int)
        .unwrap_or(100)
        .clamp(5, 1_000) as usize;
    let max_columns = opts
        .get("max_columns")
        .and_then(Value::as_int)
        .unwrap_or(40)
        .clamp(2, 100) as usize;
    let row_stride = table.rows.len().div_ceil(max_rows).max(1);
    let column_stride = table.columns.len().div_ceil(max_columns).max(1);
    let displayed_rows = (0..table.rows.len())
        .step_by(row_stride)
        .collect::<Vec<_>>();
    let displayed_columns = (0..table.columns.len())
        .step_by(column_stride)
        .collect::<Vec<_>>();
    let is_missing =
        |row: usize, column: usize| match table.rows[row].get(column).unwrap_or(&Value::Nil) {
            Value::Nil => true,
            Value::Float(value) => !value.is_finite(),
            _ => false,
        };
    if plot_format(&opts) == "ascii" {
        let mut output =
            String::from("Missingness map (ASCII): X=missing/non-finite, .=observed\n");
        output.push_str("columns: ");
        output.push_str(
            &displayed_columns
                .iter()
                .map(|column| table.columns[*column].as_str())
                .collect::<Vec<_>>()
                .join(" | "),
        );
        output.push('\n');
        for row in &displayed_rows {
            output.push_str(&format!("{:>6} |", row));
            for column in &displayed_columns {
                output.push(if is_missing(*row, *column) { 'X' } else { '.' });
            }
            output.push_str("|\n");
        }
        output.push_str(&format!("Display strides: row {row_stride}, column {column_stride}; use stat.missingness() for full-data counts."));
        return Ok(text(output));
    }
    let cell = 12.0;
    let width = (displayed_columns.len() as f64 * cell + 130.0).max(480.0);
    let height = (displayed_rows.len() as f64 * cell + 120.0).max(320.0);
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 80.0;
    canvas.margin.top = 55.0;
    for (display_row, row) in displayed_rows.iter().enumerate() {
        for (display_column, column) in displayed_columns.iter().enumerate() {
            canvas.add_rect(
                80.0 + display_column as f64 * cell,
                55.0 + display_row as f64 * cell,
                cell - 1.0,
                cell - 1.0,
                if is_missing(*row, *column) {
                    "#dc2626"
                } else {
                    "#e2e8f0"
                },
            );
        }
    }
    for (display_column, column) in displayed_columns.iter().enumerate() {
        canvas.add_text_rotated(
            86.0 + display_column as f64 * cell,
            48.0,
            &table.columns[*column],
            -55.0,
            "start",
            9.0,
        );
    }
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Missingness map"),
    );
    canvas.add_text(80.0, height - 22.0, &format!("Red=missing/non-finite. Display strides: row {row_stride}, column {column_stride}; full counts use every cell."), "start", 10.0);
    Ok(text(canvas.render()))
}

struct MatrixFacts {
    rows: usize,
    columns: usize,
    cells: usize,
    zeros: usize,
    negative: usize,
    non_integer: usize,
    non_finite: usize,
    row_totals: Vec<f64>,
    column_totals: Vec<f64>,
}

fn update_matrix_fact(facts: &mut MatrixFacts, row: usize, column: usize, value: f64) {
    if !value.is_finite() {
        facts.non_finite += 1;
        return;
    }
    if value == 0.0 {
        facts.zeros += 1;
    }
    if value < 0.0 {
        facts.negative += 1;
    }
    if (value - value.round()).abs() > 1e-10 {
        facts.non_integer += 1;
    }
    facts.row_totals[row] += value;
    facts.column_totals[column] += value;
}

fn matrix_facts(value: &Value, function: &str) -> Result<MatrixFacts> {
    match value {
        Value::Matrix(matrix) => {
            let mut facts = MatrixFacts {
                rows: matrix.nrow,
                columns: matrix.ncol,
                cells: matrix.nrow.saturating_mul(matrix.ncol),
                zeros: 0,
                negative: 0,
                non_integer: 0,
                non_finite: 0,
                row_totals: vec![0.0; matrix.nrow],
                column_totals: vec![0.0; matrix.ncol],
            };
            for row in 0..matrix.nrow {
                for column in 0..matrix.ncol {
                    update_matrix_fact(&mut facts, row, column, matrix.get(row, column));
                }
            }
            Ok(facts)
        }
        Value::SparseMatrix(matrix) => {
            let mut facts = MatrixFacts {
                rows: matrix.nrow,
                columns: matrix.ncol,
                cells: matrix.nrow.saturating_mul(matrix.ncol),
                zeros: matrix
                    .nrow
                    .saturating_mul(matrix.ncol)
                    .saturating_sub(matrix.data.len()),
                negative: 0,
                non_integer: 0,
                non_finite: 0,
                row_totals: vec![0.0; matrix.nrow],
                column_totals: vec![0.0; matrix.ncol],
            };
            for row in 0..matrix.nrow {
                for position in matrix.indptr[row]..matrix.indptr[row + 1] {
                    let value = matrix.data[position];
                    let column = matrix.indices[position];
                    if value == 0.0 {
                        facts.zeros += 1;
                    }
                    if !value.is_finite() {
                        facts.non_finite += 1;
                        continue;
                    }
                    if value < 0.0 {
                        facts.negative += 1;
                    }
                    if (value - value.round()).abs() > 1e-10 {
                        facts.non_integer += 1;
                    }
                    facts.row_totals[row] += value;
                    facts.column_totals[column] += value;
                }
            }
            Ok(facts)
        }
        Value::Table(table) => {
            let mut facts = MatrixFacts {
                rows: table.rows.len(),
                columns: table.columns.len(),
                cells: table.rows.len().saturating_mul(table.columns.len()),
                zeros: 0,
                negative: 0,
                non_integer: 0,
                non_finite: 0,
                row_totals: vec![0.0; table.rows.len()],
                column_totals: vec![0.0; table.columns.len()],
            };
            for (row_index, row) in table.rows.iter().enumerate() {
                for column in 0..table.columns.len() {
                    let value = row.get(column).unwrap_or(&Value::Nil);
                    let number = match value {
                        Value::Int(value) => *value as f64,
                        Value::Float(value) => *value,
                        Value::Nil => {
                            facts.non_finite += 1;
                            continue;
                        }
                        other => {
                            return Err(BioLangError::type_error(
                                format!(
                                    "{function}() table cell row {row_index}, column {column} is {} rather than numeric",
                                    other.type_of()
                                ),
                                None,
                            ))
                        }
                    };
                    update_matrix_fact(&mut facts, row_index, column, number);
                }
            }
            Ok(facts)
        }
        Value::List(rows) => {
            if rows.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("{function}() matrix cannot be empty"),
                    None,
                ));
            }
            let columns = match &rows[0] {
                Value::List(row) => row.len(),
                other => {
                    return Err(BioLangError::type_error(
                        format!("{function}() requires a Matrix, SparseMatrix, numeric Table, or List of Lists; first row is {}", other.type_of()),
                        None,
                    ))
                }
            };
            if columns == 0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("{function}() matrix cannot have zero columns"),
                    None,
                ));
            }
            let mut facts = MatrixFacts {
                rows: rows.len(),
                columns,
                cells: rows.len().saturating_mul(columns),
                zeros: 0,
                negative: 0,
                non_integer: 0,
                non_finite: 0,
                row_totals: vec![0.0; rows.len()],
                column_totals: vec![0.0; columns],
            };
            for (row_index, row) in rows.iter().enumerate() {
                let Value::List(row) = row else {
                    return Err(BioLangError::type_error(
                        format!("{function}() row {row_index} is not a List"),
                        None,
                    ));
                };
                if row.len() != columns {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("{function}() matrix rows have unequal lengths"),
                        None,
                    ));
                }
                for (column, value) in row.iter().enumerate() {
                    let number = match value {
                        Value::Int(value) => *value as f64,
                        Value::Float(value) => *value,
                        Value::Nil => {
                            facts.non_finite += 1;
                            continue;
                        }
                        other => {
                            return Err(BioLangError::type_error(
                                format!("{function}() cell row {row_index}, column {column} is {} rather than numeric", other.type_of()),
                                None,
                            ))
                        }
                    };
                    update_matrix_fact(&mut facts, row_index, column, number);
                }
            }
            Ok(facts)
        }
        other => Err(BioLangError::type_error(
            format!("{function}() requires a Matrix, SparseMatrix, numeric Table, or List of Lists; got {}", other.type_of()),
            None,
        )),
    }
}

fn guidance_option(
    name: &str,
    status: &str,
    useful_when: &str,
    needs: &str,
    caution: &str,
) -> Value {
    record([
        ("name", text(name)),
        ("status", text(status)),
        ("useful_when", text(useful_when)),
        ("required_inputs", text(needs)),
        ("caution", text(caution)),
        ("automatically_applied", Value::Bool(false)),
    ])
}

fn normalization_guide(args: Vec<Value>) -> Result<Value> {
    let facts = matrix_facts(&args[0], "stats_normalization_guide")?;
    let opts = options(&args, 1, "stats_normalization_guide")?;
    let data_type = opts
        .get("data_type")
        .and_then(Value::as_str)
        .unwrap_or("counts");
    let sample_axis = opts
        .get("sample_axis")
        .and_then(Value::as_str)
        .unwrap_or("rows");
    if !matches!(sample_axis, "rows" | "columns") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_normalization_guide() sample_axis must be rows or columns",
            None,
        ));
    }
    let sample_totals = if sample_axis == "rows" {
        &facts.row_totals
    } else {
        &facts.column_totals
    };
    let zero_samples = sample_totals.iter().filter(|total| **total == 0.0).count();
    let positive_totals = sample_totals
        .iter()
        .copied()
        .filter(|total| *total > 0.0 && total.is_finite())
        .collect::<Vec<_>>();
    let minimum_total = positive_totals.iter().copied().min_by(f64::total_cmp);
    let maximum_total = positive_totals.iter().copied().max_by(f64::total_cmp);
    let depth_ratio = minimum_total
        .zip(maximum_total)
        .map(|(minimum, maximum)| maximum / minimum);
    let zero_fraction = if facts.cells == 0 {
        0.0
    } else {
        facts.zeros as f64 / facts.cells as f64
    };
    let mut issues = Vec::new();
    if facts.non_finite > 0 {
        issues.push(issue(
            "non_finite_matrix_values",
            format!(
                "{} matrix cell(s) are missing or non-finite.",
                facts.non_finite
            ),
            "The origin and missingness mechanism need review before normalization.",
            "blocking",
        ));
    }
    if zero_samples > 0 {
        issues.push(issue(
            "zero_total_samples",
            format!("{zero_samples} declared sample(s) have total zero."),
            "Size-factor and log-ratio normalizations are undefined for an all-zero sample.",
            "blocking",
        ));
    }
    if depth_ratio.is_some_and(|ratio| ratio >= 3.0) {
        issues.push(issue(
            "unequal_sample_totals",
            format!("Largest/smallest positive sample total is {}.", fmt_number(depth_ratio.unwrap())),
            "Unequal totals may represent sequencing depth, exposure, biomass, composition, or real global change; the denominator must match the experiment.",
            "review",
        ));
    }
    if matches!(data_type, "count" | "counts") && (facts.negative > 0 || facts.non_integer > 0) {
        issues.push(issue(
            "count_matrix_contract_mismatch",
            format!("Count matrix has {} negative and {} non-integer finite cell(s).", facts.negative, facts.non_integer),
            "Raw counts should ordinarily be non-negative integers; transformed expression needs a different declared data type.",
            "blocking",
        ));
    }

    let mut suggestions = Vec::new();
    match data_type {
        "count" | "counts" => {
            suggestions.push(guidance_option(
                "model counts with a library/exposure offset",
                "preferred_for_inference",
                "The outcome is a count and the statistical model supports offsets and overdispersion.",
                "raw counts, sample totals or exposure, design matrix, experimental unit",
                "A total-count offset assumes the chosen total is an appropriate opportunity measure.",
            ));
            suggestions.push(guidance_option(
                "median-ratio or robust size factors",
                "candidate",
                "Most features are not expected to shift in the same direction and composition differs between samples.",
                "sample-by-feature raw count matrix with enough shared observed features",
                "The stability assumption can fail under global shifts or extreme sparsity.",
            ));
            suggestions.push(guidance_option(
                "counts per million / total-count scaling",
                "descriptive_candidate",
                "A simple within-sample abundance display is needed.",
                "raw counts and a scientifically defensible sample total",
                "A dominant feature can distort every other relative abundance; this does not model count variance.",
            ));
            suggestions.push(guidance_option(
                "variance-stabilising or log-normalised values",
                "visualisation_or_distance_candidate",
                "PCA, clustering, or visualisation needs a less mean-dependent scale.",
                "raw count matrix, fitted size factors, and a documented transform",
                "Use raw counts or the method-required scale for inference; pseudocounts alter low counts.",
            ));
        }
        "compositional" => {
            suggestions.push(guidance_option(
                "centred log-ratio transform",
                "candidate",
                "Only relative composition is observed and log-ratios answer the scientific question.",
                "positive components or a justified zero-replacement model",
                "Zeros require explicit treatment and every coordinate is relative to the geometric mean.",
            ));
            suggestions.push(guidance_option(
                "log-ratio model using a reference component",
                "alternative",
                "A stable, interpretable reference component is scientifically defensible.",
                "chosen reference and positive numerator/denominator values",
                "Results depend on the reference; a convenient reference is not automatically valid.",
            ));
        }
        "proportion" | "probability" => {
            suggestions.push(guidance_option(
                "binomial/beta-binomial model",
                "preferred_when_denominators_exist",
                "Numerators and denominators are observed and sampling variation matters.",
                "success counts, denominators, and design variables",
                "A proportion alone discards denominator-dependent precision.",
            ));
            suggestions.push(guidance_option(
                "logit transform",
                "model_dependent",
                "Values lie strictly between zero and one and an unbounded scale is required.",
                "proportions without boundary values or a justified boundary correction",
                "Undefined at zero and one; it does not restore discarded denominator information.",
            ));
        }
        _ => {
            suggestions.push(guidance_option(
                "keep original measurement scale",
                "preferred_start",
                "Units are interpretable and model diagnostics are adequate.",
                "measurement units, experimental unit, and analysis goal",
                "Different variables may still need scaling for distance-based methods.",
            ));
            suggestions.push(guidance_option(
                "column-wise z-score or robust scaling",
                "distance_method_candidate",
                "PCA, clustering, or regularisation should not be dominated by measurement units.",
                "training-data centre/scale estimates fitted without data leakage",
                "Scaling removes units, can amplify noisy low-variance variables, and must be applied from training to validation data unchanged.",
            ));
        }
    }
    suggestions.push(guidance_option(
        "batch-aware modelling or correction",
        "requires_metadata",
        "Known technical batches affect measurements and are not perfectly confounded with biology.",
        "batch labels, biological groups, experimental units, and the downstream model",
        "Correction can erase biology when batch and group are confounded; inspect overlap first.",
    ));
    let explanation = format!(
        "Matrix normalization guidance\n\nShape: {} rows x {} columns\nDeclared samples: {}\nDeclared data type: {}\nZero fraction: {}%\nZero-total samples: {}\nSample-total ratio: {}\nIssue clues: {}\n\nNo normalization was applied. The correct denominator and transformation depend on the measurement process, sample axis, design, and downstream estimand.",
        facts.rows,
        facts.columns,
        sample_totals.len(),
        data_type,
        fmt_number(zero_fraction * 100.0),
        zero_samples,
        depth_ratio.map(fmt_number).unwrap_or_else(|| "not available".into()),
        issues.len(),
    );
    Ok(record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("normalization_guidance")),
        ("rows", Value::Int(facts.rows as i64)),
        ("columns", Value::Int(facts.columns as i64)),
        ("cells", Value::Int(facts.cells as i64)),
        ("data_type", text(data_type)),
        ("sample_axis", text(sample_axis)),
        ("samples", Value::Int(sample_totals.len() as i64)),
        ("zeros", Value::Int(facts.zeros as i64)),
        ("zero_fraction", Value::Float(zero_fraction)),
        ("negative", Value::Int(facts.negative as i64)),
        ("non_integer", Value::Int(facts.non_integer as i64)),
        ("non_finite", Value::Int(facts.non_finite as i64)),
        ("zero_total_samples", Value::Int(zero_samples as i64)),
        ("minimum_positive_sample_total", number(minimum_total)),
        ("maximum_positive_sample_total", number(maximum_total)),
        ("sample_total_ratio", number(depth_ratio)),
        (
            "sample_totals",
            list(sample_totals.iter().copied().map(Value::Float).collect()),
        ),
        ("issues", list(issues)),
        ("suggestions", list(suggestions)),
        ("automatic_changes", Value::Bool(false)),
        (
            "quick_explanation",
            text("The complete matrix was audited and domain-aware normalization alternatives were listed; no values were changed."),
        ),
        ("explanation", text(explanation)),
    ]))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScreenColumnKind {
    Numeric,
    Categorical,
}

fn screen_column_kind(table: &Table, column: usize) -> Option<ScreenColumnKind> {
    let mut kind = None;
    for row in &table.rows {
        match row.get(column).unwrap_or(&Value::Nil) {
            Value::Nil => {}
            Value::Float(value) if !value.is_finite() => {}
            Value::Int(_) | Value::Float(_) => match kind {
                None | Some(ScreenColumnKind::Numeric) => kind = Some(ScreenColumnKind::Numeric),
                Some(ScreenColumnKind::Categorical) => return None,
            },
            Value::Str(_) | Value::Bool(_) => match kind {
                None | Some(ScreenColumnKind::Categorical) => {
                    kind = Some(ScreenColumnKind::Categorical)
                }
                Some(ScreenColumnKind::Numeric) => return None,
            },
            _ => return None,
        }
    }
    kind
}

fn finite_number(value: &Value) -> Option<f64> {
    match value {
        Value::Int(value) => Some(*value as f64),
        Value::Float(value) if value.is_finite() => Some(*value),
        _ => None,
    }
}

fn table_numeric_pairs(table: &Table, left: usize, right: usize) -> (Vec<f64>, Vec<f64>) {
    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for row in &table.rows {
        if let (Some(x), Some(y)) = (
            row.get(left).and_then(finite_number),
            row.get(right).and_then(finite_number),
        ) {
            xs.push(x);
            ys.push(y);
        }
    }
    (xs, ys)
}

fn cramers_v(table: &Table, left: usize, right: usize) -> Option<(f64, usize, usize, usize)> {
    let mut left_levels = HashMap::<String, usize>::new();
    let mut right_levels = HashMap::<String, usize>::new();
    let mut observations = Vec::new();
    for row in &table.rows {
        let (Some(left_value), Some(right_value)) = (
            row.get(left).and_then(category_label),
            row.get(right).and_then(category_label),
        ) else {
            continue;
        };
        let next_left = left_levels.len();
        let left_index = *left_levels.entry(left_value).or_insert(next_left);
        let next_right = right_levels.len();
        let right_index = *right_levels.entry(right_value).or_insert(next_right);
        observations.push((left_index, right_index));
    }
    let rows = left_levels.len();
    let columns = right_levels.len();
    let n = observations.len();
    if n < 2 || rows < 2 || columns < 2 {
        return None;
    }
    let mut cells = vec![0usize; rows * columns];
    let mut row_totals = vec![0usize; rows];
    let mut column_totals = vec![0usize; columns];
    for (row, column) in observations {
        cells[row * columns + column] += 1;
        row_totals[row] += 1;
        column_totals[column] += 1;
    }
    let mut chi_squared = 0.0;
    for row in 0..rows {
        for column in 0..columns {
            let expected = row_totals[row] as f64 * column_totals[column] as f64 / n as f64;
            if expected > 0.0 {
                chi_squared += (cells[row * columns + column] as f64 - expected).powi(2) / expected;
            }
        }
    }
    let denominator = n as f64 * (rows.min(columns) - 1) as f64;
    Some(((chi_squared / denominator).sqrt(), n, rows, columns))
}

fn correlation_ratio(
    table: &Table,
    category_column: usize,
    numeric_column: usize,
) -> Option<(f64, usize, usize)> {
    let mut order = Vec::<String>::new();
    let mut groups = HashMap::<String, Vec<f64>>::new();
    let mut all = Vec::new();
    for row in &table.rows {
        let (Some(category), Some(value)) = (
            row.get(category_column).and_then(category_label),
            row.get(numeric_column).and_then(finite_number),
        ) else {
            continue;
        };
        if !groups.contains_key(&category) {
            order.push(category.clone());
        }
        groups.entry(category).or_default().push(value);
        all.push(value);
    }
    if all.len() < 3 || groups.len() < 2 {
        return None;
    }
    let mean = all.iter().sum::<f64>() / all.len() as f64;
    let total = all.iter().map(|value| (value - mean).powi(2)).sum::<f64>();
    if total <= f64::EPSILON {
        return None;
    }
    let between = order
        .iter()
        .map(|name| {
            let values = &groups[name];
            let group_mean = values.iter().sum::<f64>() / values.len() as f64;
            values.len() as f64 * (group_mean - mean).powi(2)
        })
        .sum::<f64>();
    Some(((between / total).clamp(0.0, 1.0), all.len(), groups.len()))
}

fn association_strength(score: f64) -> &'static str {
    match score.abs() {
        value if value >= 0.9 => "very strong clue",
        value if value >= 0.7 => "strong clue",
        value if value >= 0.4 => "moderate clue",
        value if value >= 0.2 => "weak clue",
        _ => "little clue",
    }
}

fn association_score(value: &Value) -> f64 {
    match value {
        Value::Record(report) => report
            .get("screening_score")
            .and_then(Value::as_float)
            .unwrap_or(0.0),
        _ => 0.0,
    }
}

fn association_record(table: &Table, opts: &HashMap<String, Value>) -> Value {
    let threshold = opts
        .get("association_threshold")
        .or_else(|| opts.get("threshold"))
        .and_then(Value::as_float)
        .unwrap_or(0.7)
        .clamp(0.0, 1.0);
    let max_columns = opts
        .get("max_association_columns")
        .or_else(|| opts.get("max_columns"))
        .and_then(Value::as_int)
        .unwrap_or(50)
        .clamp(2, 500) as usize;
    let max_pairs = opts
        .get("max_pairs")
        .and_then(Value::as_int)
        .unwrap_or(100)
        .clamp(1, 10_000) as usize;
    let max_levels = opts
        .get("max_levels")
        .and_then(Value::as_int)
        .unwrap_or(20)
        .clamp(2, 1_000) as usize;

    let mut declared_exclusions = HashSet::<String>::new();
    if let Some(subject) = opts.get("subject_column").and_then(Value::as_str) {
        declared_exclusions.insert(subject.to_string());
    }
    if let Some(Value::List(columns)) = opts.get("exclude_columns") {
        for column in columns.iter().filter_map(Value::as_str) {
            declared_exclusions.insert(column.to_string());
        }
    }
    let declared_categorical = opts
        .get("categorical_columns")
        .and_then(|value| match value {
            Value::List(columns) => Some(
                columns
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect::<HashSet<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default();
    let mut eligible = Vec::<(usize, ScreenColumnKind)>::new();
    let mut skipped_declared = Vec::new();
    let mut skipped_mixed_or_empty = Vec::new();
    let mut skipped_high_cardinality = Vec::new();
    for column in 0..table.columns.len() {
        if declared_exclusions.contains(&table.columns[column]) {
            skipped_declared.push(table.columns[column].clone());
            continue;
        }
        let Some(mut kind) = screen_column_kind(table, column) else {
            skipped_mixed_or_empty.push(table.columns[column].clone());
            continue;
        };
        if declared_categorical.contains(&table.columns[column]) {
            kind = ScreenColumnKind::Categorical;
        }
        if kind == ScreenColumnKind::Categorical {
            let levels = table
                .rows
                .iter()
                .filter_map(|row| row.get(column).and_then(category_label))
                .collect::<HashSet<_>>()
                .len();
            if levels > max_levels {
                skipped_high_cardinality.push(table.columns[column].clone());
                continue;
            }
        }
        eligible.push((column, kind));
    }
    let eligible_total = eligible.len();
    eligible.truncate(max_columns);
    let mut pairs = Vec::new();
    for left_position in 0..eligible.len() {
        for right_position in left_position + 1..eligible.len() {
            let (left, left_kind) = eligible[left_position];
            let (right, right_kind) = eligible[right_position];
            let result = match (left_kind, right_kind) {
                (ScreenColumnKind::Numeric, ScreenColumnKind::Numeric) => {
                    let (xs, ys) = table_numeric_pairs(table, left, right);
                    let Some((pearson_value, _, _)) = pearson(&xs, &ys) else {
                        continue;
                    };
                    let spearman_value =
                        pearson(&average_ranks(&xs), &average_ranks(&ys)).map(|value| value.0);
                    record([
                        ("left", text(&table.columns[left])),
                        ("right", text(&table.columns[right])),
                        ("kind", text("numeric_numeric")),
                        ("complete", Value::Int(xs.len() as i64)),
                        (
                            "excluded",
                            Value::Int(table.rows.len().saturating_sub(xs.len()) as i64),
                        ),
                        ("pearson", Value::Float(pearson_value)),
                        ("spearman", number(spearman_value)),
                        ("cramers_v", Value::Nil),
                        ("eta_squared", Value::Nil),
                        ("screening_score", Value::Float(pearson_value.abs())),
                        (
                            "direction",
                            text(if pearson_value < 0.0 {
                                "negative"
                            } else {
                                "positive"
                            }),
                        ),
                        ("strength", text(association_strength(pearson_value))),
                    ])
                }
                (ScreenColumnKind::Categorical, ScreenColumnKind::Categorical) => {
                    let Some((value, complete, left_levels, right_levels)) =
                        cramers_v(table, left, right)
                    else {
                        continue;
                    };
                    record([
                        ("left", text(&table.columns[left])),
                        ("right", text(&table.columns[right])),
                        ("kind", text("categorical_categorical")),
                        ("complete", Value::Int(complete as i64)),
                        (
                            "excluded",
                            Value::Int(table.rows.len().saturating_sub(complete) as i64),
                        ),
                        ("left_levels", Value::Int(left_levels as i64)),
                        ("right_levels", Value::Int(right_levels as i64)),
                        ("pearson", Value::Nil),
                        ("spearman", Value::Nil),
                        ("cramers_v", Value::Float(value)),
                        ("eta_squared", Value::Nil),
                        ("screening_score", Value::Float(value)),
                        ("direction", text("unsigned")),
                        ("strength", text(association_strength(value))),
                    ])
                }
                _ => {
                    let (category, numeric) = if left_kind == ScreenColumnKind::Categorical {
                        (left, right)
                    } else {
                        (right, left)
                    };
                    let Some((value, complete, levels)) =
                        correlation_ratio(table, category, numeric)
                    else {
                        continue;
                    };
                    record([
                        ("left", text(&table.columns[left])),
                        ("right", text(&table.columns[right])),
                        ("kind", text("categorical_numeric")),
                        ("complete", Value::Int(complete as i64)),
                        (
                            "excluded",
                            Value::Int(table.rows.len().saturating_sub(complete) as i64),
                        ),
                        ("category_levels", Value::Int(levels as i64)),
                        ("pearson", Value::Nil),
                        ("spearman", Value::Nil),
                        ("cramers_v", Value::Nil),
                        ("eta_squared", Value::Float(value)),
                        ("screening_score", Value::Float(value.sqrt())),
                        ("direction", text("unsigned")),
                        ("strength", text(association_strength(value.sqrt()))),
                    ])
                }
            };
            pairs.push(result);
        }
    }
    pairs.sort_by(|left, right| {
        association_score(right)
            .total_cmp(&association_score(left))
            .then_with(|| value_label(left).cmp(&value_label(right)))
    });
    let pairs_computed = pairs.len();
    let high_pairs = pairs
        .iter()
        .filter(|pair| association_score(pair) >= threshold)
        .count();
    pairs.truncate(max_pairs);
    let explanation = format!(
        "Association screen\n\nEligible columns used: {} of {}\nPairs computed: {}\nPairs returned: {}\nPairs at or above {}: {}\n\nThis is an exploratory redundancy and structure screen. It does not establish causation, agreement, independence, or statistical significance.",
        eligible.len(),
        eligible_total,
        pairs_computed,
        pairs.len(),
        fmt_number(threshold),
        high_pairs
    );
    record([
        ("schema", text("biolang.stats.exploration/v1")),
        ("kind", text("association_screen")),
        ("rows", Value::Int(table.rows.len() as i64)),
        ("eligible_columns", Value::Int(eligible_total as i64)),
        ("columns_used", Value::Int(eligible.len() as i64)),
        (
            "columns_truncated",
            Value::Int(eligible_total.saturating_sub(eligible.len()) as i64),
        ),
        ("pairs_computed", Value::Int(pairs_computed as i64)),
        ("pairs_returned", Value::Int(pairs.len() as i64)),
        ("pairs_truncated", Value::Bool(pairs_computed > pairs.len())),
        ("threshold", Value::Float(threshold)),
        ("high_association_pairs", Value::Int(high_pairs as i64)),
        ("pairs", list(pairs)),
        ("skipped_declared", string_list(skipped_declared)),
        ("skipped_mixed_or_empty", string_list(skipped_mixed_or_empty)),
        (
            "skipped_high_cardinality",
            string_list(skipped_high_cardinality),
        ),
        ("hypothesis_tests_performed", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Pairwise screening does not adjust for confounding, repeated measures, or multiple comparisons.",
                "Pearson measures linear association; Spearman measures monotonic rank association.",
                "Cramer's V and eta-squared are unsigned and do not identify a causal direction.",
                "High-cardinality categorical columns are skipped by default to bound work and avoid ID-like variables.",
                "Integer-coded categories are numeric unless listed in categorical_columns.",
            ]),
        ),
        ("quick_explanation", text("Pairwise structure was screened with type-appropriate effect-size clues; no significance tests were run.")),
        ("explanation", text(explanation)),
    ])
}

fn association_screen(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_associations")?;
    let opts = options(&args, 1, "stats_associations")?;
    Ok(association_record(table, &opts))
}

fn recommendation(
    id: &str,
    priority: &str,
    evidence: impl Into<String>,
    why: impl Into<String>,
    next_step: impl Into<String>,
    example: impl Into<String>,
) -> Value {
    record([
        ("id", text(id)),
        ("priority", text(priority)),
        ("evidence", text(evidence.into())),
        ("why", text(why.into())),
        ("next_step", text(next_step.into())),
        ("example", text(example.into())),
        ("automatically_applied", Value::Bool(false)),
    ])
}

fn record_int(value: &Value, name: &str) -> i64 {
    match value {
        Value::Record(map) => map.get(name).and_then(Value::as_int).unwrap_or(0),
        _ => 0,
    }
}

fn record_string<'a>(value: &'a Value, name: &str) -> Option<&'a str> {
    match value {
        Value::Record(map) => map.get(name).and_then(Value::as_str),
        _ => None,
    }
}

fn nested_list<'a>(value: &'a Value, outer: &str, inner: &str) -> Option<&'a [Value]> {
    let Value::Record(root) = value else {
        return None;
    };
    let Value::Record(nested) = root.get(outer)? else {
        return None;
    };
    let Value::List(values) = nested.get(inner)? else {
        return None;
    };
    Some(values)
}

fn compact_numeric_detail(data: &NumericData, name: &str, data_type: &str) -> Value {
    let summary = summarize(data);
    let (shape, shape_explanation) = shape_label(&summary);
    let robust = summary.skewness.is_some_and(|value| value.abs() >= 0.5)
        || !summary.outlier_positions.is_empty();
    record([
        ("name", text(name)),
        ("kind", text("numeric")),
        ("data_type", text(data_type)),
        ("received", Value::Int(data.total as i64)),
        ("used", Value::Int(data.values.len() as i64)),
        ("missing", Value::Int(data.missing as i64)),
        ("non_finite", Value::Int(data.non_finite as i64)),
        ("summary", compact_summary(&summary)),
        ("shape", text(shape)),
        ("shape_explanation", text(shape_explanation)),
        (
            "suggested_center",
            text(if robust { "median" } else { "mean" }),
        ),
        (
            "suggested_spread",
            text(if robust { "IQR" } else { "standard deviation" }),
        ),
        ("suggestion_is_heuristic", Value::Bool(true)),
        (
            "full_detail_example",
            text(format!("stat.explore(values, {{name: \"{name}\"}})")),
        ),
    ])
}

fn compact_categorical_detail(table: &Table, column: usize) -> Value {
    let mut order = Vec::<String>::new();
    let mut counts = HashMap::<String, usize>::new();
    let mut missing = 0usize;
    for row in &table.rows {
        let Some(label) = row.get(column).and_then(category_label) else {
            missing += 1;
            continue;
        };
        if !counts.contains_key(&label) {
            order.push(label.clone());
        }
        *counts.entry(label).or_default() += 1;
    }
    let mut levels = order
        .iter()
        .enumerate()
        .map(|(position, label)| (label.clone(), counts[label], position))
        .collect::<Vec<_>>();
    levels.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| left.2.cmp(&right.2)));
    let levels_returned = levels.len().min(5);
    let top_levels = levels
        .into_iter()
        .take(5)
        .map(|(label, count, _)| {
            record([
                ("level", text(label)),
                ("count", Value::Int(count as i64)),
                (
                    "proportion",
                    Value::Float(if table.rows.is_empty() {
                        0.0
                    } else {
                        count as f64 / table.rows.len() as f64
                    }),
                ),
            ])
        })
        .collect::<Vec<_>>();
    record([
        ("name", text(&table.columns[column])),
        ("kind", text("categorical")),
        ("received", Value::Int(table.rows.len() as i64)),
        (
            "used",
            Value::Int(table.rows.len().saturating_sub(missing) as i64),
        ),
        ("missing", Value::Int(missing as i64)),
        ("levels", Value::Int(counts.len() as i64)),
        ("top_levels", list(top_levels)),
        ("levels_returned", Value::Int(levels_returned as i64)),
        (
            "levels_truncated",
            Value::Bool(counts.len() > levels_returned),
        ),
        ("full_detail_example", text("stat.categorical(categories)")),
    ])
}

fn scan_table(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_scan")?;
    let opts = options(&args, 1, "stats_scan")?;
    let options_value = Value::Record(opts.clone().into());
    let profile = profile_table(vec![args[0].clone(), options_value])?;
    let associations = association_record(table, &opts);
    let max_details = opts
        .get("max_detail_columns")
        .and_then(Value::as_int)
        .unwrap_or(50)
        .clamp(1, 500) as usize;
    let data_type = opts
        .get("data_type")
        .and_then(Value::as_str)
        .unwrap_or("unspecified");
    let mut details = Vec::new();
    let mut skewed = 0usize;
    let mut outlier_columns = 0usize;
    for column in 0..table.columns.len().min(max_details) {
        match screen_column_kind(table, column) {
            Some(ScreenColumnKind::Numeric) => {
                let data = column_numeric_data(table, column);
                if data.values.is_empty() {
                    continue;
                }
                let summary = summarize(&data);
                if summary.skewness.is_some_and(|value| value.abs() >= 1.0) {
                    skewed += 1;
                }
                if !summary.outlier_positions.is_empty() {
                    outlier_columns += 1;
                }
                details.push(compact_numeric_detail(
                    &data,
                    &table.columns[column],
                    data_type,
                ));
            }
            Some(ScreenColumnKind::Categorical) => {
                details.push(compact_categorical_detail(table, column));
            }
            None => {}
        }
    }

    let missing_cells = table
        .rows
        .iter()
        .flat_map(|row| row.iter())
        .filter(|value| {
            matches!(value, Value::Nil)
                || matches!(value, Value::Float(number) if !number.is_finite())
        })
        .count();
    let duplicate_rows = record_int(&profile, "duplicate_rows");
    let high_pairs = record_int(&associations, "high_association_pairs");
    let mut recommendations = Vec::new();
    if missing_cells > 0 {
        let group_option = opts
            .get("group_column")
            .and_then(Value::as_str)
            .map(|name| format!(", {{group_column: \"{name}\"}}"))
            .unwrap_or_default();
        recommendations.push(recommendation(
            "inspect_missingness",
            "high",
            format!("{missing_cells} missing or non-finite table cell(s) were observed."),
            "Complete-case analysis or imputation can change the population and uncertainty.",
            "Map missingness by variable and, when justified, by biological group or batch before choosing a strategy.",
            format!("stat.missingness(data{group_option})"),
        ));
    }
    if duplicate_rows > 0 {
        recommendations.push(recommendation(
            "verify_duplicates",
            "high",
            format!("{duplicate_rows} row(s) duplicate an earlier complete row."),
            "A duplicate can be an error, a technical replicate, or a valid repeated observation.",
            "Resolve identifiers and the experimental unit before removing anything.",
            "stat.profile(data, {subject_column: \"subject_id\"})",
        ));
    }
    if opts.get("subject_column").is_none() {
        recommendations.push(recommendation(
            "declare_experimental_unit",
            "foundation",
            "No subject_column was supplied to the scan.",
            "Independence, pairing, and replication cannot be inferred safely from values or column names.",
            "Declare the subject or experimental-unit column when one exists.",
            "stat.scan(data, {subject_column: \"subject_id\", group_column: \"group\"})",
        ));
    }
    if let Some(design_issues) = nested_list(&profile, "design", "issues") {
        for design_issue in design_issues {
            let Some(id) = record_string(design_issue, "id") else {
                continue;
            };
            let observation = record_string(design_issue, "observation")
                .unwrap_or("A design-structure clue was observed.");
            match id {
                "batch_group_confounding" => recommendations.push(recommendation(
                    "resolve_batch_group_confounding",
                    "blocking",
                    observation,
                    "The observed table cannot separate the biological group contrast from the batch contrast without overlap or external assumptions.",
                    "Do not report a group effect as separable from batch; redesign, obtain overlap, or state the identifiability limit.",
                    "stat.design_check(data, {group_column: \"group\", batch_column: \"batch\"})",
                )),
                "repeated_experimental_units" => recommendations.push(recommendation(
                    "model_repeated_units",
                    "high",
                    observation,
                    "Rows from the same experimental unit are generally dependent.",
                    "Use the declared experimental unit for aggregation, pairing, blocking, mixed modelling, or cluster-aware resampling as scientifically appropriate.",
                    "stat.design_check(data, {subject_column: \"subject_id\"})",
                )),
                "duplicate_subject_time" => recommendations.push(recommendation(
                    "resolve_duplicate_subject_times",
                    "high",
                    observation,
                    "Repeated subject/time combinations may be duplicates, technical replicates, or distinct measurements.",
                    "Identify the replicate type before aggregation or modelling.",
                    "stat.design_check(data, {subject_column: \"subject_id\", time_column: \"time\"})",
                )),
                "small_group" | "unbalanced_groups" => recommendations.push(recommendation(
                    "review_group_information",
                    "medium",
                    observation,
                    "Small or highly unequal groups can make estimates imprecise and assumptions influential.",
                    "Show every group size and observation, emphasize effect uncertainty, and use a design-appropriate model.",
                    "stat.compare(values, groups)",
                )),
                _ => {}
            }
        }
    }
    if let Some(design_clues) = nested_list(&profile, "design", "design_clues") {
        for clue in design_clues {
            let Some(id) = record_string(clue, "id") else {
                continue;
            };
            match id {
                "paired_or_crossover_clue" => recommendations.push(recommendation(
                    "preserve_pairing",
                    "high",
                    "Some declared experimental units occur in more than one group.",
                    "Discarding within-unit alignment loses information and can give the wrong uncertainty.",
                    "Verify treatment order and carry the subject/block structure into estimation and resampling.",
                    "stat.design_check(data, {subject_column: \"subject_id\", group_column: \"group\"})",
                )),
                "longitudinal_clue" => recommendations.push(recommendation(
                    "model_time_within_unit",
                    "high",
                    "Some declared experimental units occur at multiple times.",
                    "Repeated time points share an experimental unit and may also have ordered dependence.",
                    "Plot trajectories and use a longitudinal, mixed, blocked, or cluster-aware method appropriate to the estimand.",
                    "stat.design_check(data, {subject_column: \"subject_id\", time_column: \"time\"})",
                )),
                "clustered_observations_clue" => recommendations.push(recommendation(
                    "preserve_clusters",
                    "high",
                    "Multiple rows share a declared cluster.",
                    "Row-wise uncertainty or validation can leak information and overstate effective sample size.",
                    "Keep clusters intact in modelling, resampling, train/test splitting, or aggregation as scientifically appropriate.",
                    "stat.design_check(data, {cluster_column: \"site_id\"})",
                )),
                _ => {}
            }
        }
    }
    if high_pairs > 0 {
        recommendations.push(recommendation(
            "review_strong_associations",
            "medium",
            format!("{high_pairs} pair(s) reached the exploratory association threshold."),
            "Strong association may indicate redundancy, confounding, shared measurement, leakage, or expected biology.",
            "Plot important pairs and interpret them using the study design; do not select predictors from this screen alone.",
            "stat.associations(data, {threshold: 0.7})",
        ));
    }
    if skewed > 0 {
        recommendations.push(recommendation(
            "preview_transformations",
            "medium",
            format!("{skewed} detailed numeric column(s) had |adjusted skewness| >= 1."),
            "A long tail can make a mean, SD, and linear-scale plot unrepresentative.",
            "Compare original and transformed summaries, then choose based on measurement meaning and model diagnostics.",
            "stat.preview_transform(values, \"log1p\")",
        ));
    }
    if outlier_columns > 0 {
        recommendations.push(recommendation(
            "inspect_influential_values",
            "medium",
            format!("{outlier_columns} detailed numeric column(s) contained Tukey review flags."),
            "A flagged value may be valid biology and need not be an error or an influential model point.",
            "Inspect provenance, units, group membership, and model influence without automatic deletion.",
            "stat.distribution_plot(values)",
        ));
    }
    recommendations.push(recommendation(
        "report_uncertainty",
        "foundation",
        "Point summaries alone do not show sampling uncertainty.",
        "Effect sizes and intervals make magnitude and precision visible alongside any p-value.",
        "Choose the estimand and resampling unit from the design, then report an interval.",
        "stat.uncertainty(values, {statistic: \"median\", seed: 42})",
    ));

    recommendations.sort_by_key(|item| match record_string(item, "priority") {
        Some("blocking") => 0,
        Some("high") => 1,
        Some("medium") => 2,
        _ => 3,
    });

    let recommendation_text = recommendations
        .iter()
        .filter_map(|item| {
            Some(format!(
                "  - [{}] {}\n    Evidence: {}",
                record_string(item, "priority")?,
                record_string(item, "next_step")?,
                record_string(item, "evidence")?
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");
    let explanation = format!(
        "Guided dataset scan\n\nRows: {}\nColumns: {}\nDetailed columns: {}\nMissing/non-finite cells: {}\nHigh association clues: {}\n\nPrioritized next steps\n{}\n\nAll calculations are descriptive and non-mutating. Scientific roles, causal structure, and the experimental unit remain analyst-supplied context.",
        table.rows.len(),
        table.columns.len(),
        details.len(),
        missing_cells,
        high_pairs,
        recommendation_text
    );
    Ok(record([
        ("schema", text("biolang.stats.guided-scan/v1")),
        ("kind", text("guided_dataset_scan")),
        ("rows", Value::Int(table.rows.len() as i64)),
        ("columns", Value::Int(table.columns.len() as i64)),
        ("profile", profile),
        ("associations", associations),
        ("column_details", list(details)),
        (
            "detail_columns_truncated",
            Value::Int(table.columns.len().saturating_sub(max_details) as i64),
        ),
        ("recommendations", list(recommendations)),
        ("automatic_changes", Value::Bool(false)),
        ("automatic_test_selection", Value::Bool(false)),
        (
            "quick_explanation",
            text("The dataset was profiled, screened, and translated into evidence-linked next steps; nothing was changed."),
        ),
        ("explanation", text(explanation)),
    ]))
}

fn shortened(value: &str, width: usize) -> String {
    let count = value.chars().count();
    if count <= width {
        return value.to_string();
    }
    let keep = width.saturating_sub(3);
    format!("{}...", value.chars().take(keep).collect::<String>())
}

fn overview_ascii(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_overview_ascii")?;
    let opts = options(&args, 1, "stats_overview_ascii")?;
    let name_width = opts
        .get("name_width")
        .and_then(Value::as_int)
        .unwrap_or(22)
        .clamp(10, 40) as usize;
    let max_columns = opts
        .get("max_columns")
        .and_then(Value::as_int)
        .unwrap_or(100)
        .clamp(1, 1_000) as usize;
    let mut output = format!(
        "BioLang dataset overview\nrows={} columns={} (showing up to {})\n\n",
        table.rows.len(),
        table.columns.len(),
        max_columns
    );
    output.push_str(&format!(
        "{:<name_width$}  {:<11} {:>7} {:>7} {:>7}  {}\n",
        "column", "type", "used", "miss", "unique", "centre / top"
    ));
    output.push_str(&format!(
        "{}  {} {} {} {}  {}\n",
        "-".repeat(name_width),
        "-".repeat(11),
        "-".repeat(7),
        "-".repeat(7),
        "-".repeat(7),
        "-".repeat(24)
    ));
    for column in 0..table.columns.len().min(max_columns) {
        let values = table
            .rows
            .iter()
            .map(|row| row.get(column).unwrap_or(&Value::Nil))
            .collect::<Vec<_>>();
        let missing = values
            .iter()
            .filter(|value| {
                matches!(value, Value::Nil)
                    || matches!(value, Value::Float(number) if !number.is_finite())
            })
            .count();
        let unique = values
            .iter()
            .filter(|value| {
                !matches!(value, Value::Nil)
                    && !matches!(value, Value::Float(number) if !number.is_finite())
            })
            .map(|value| value_label(value))
            .collect::<HashSet<_>>()
            .len();
        let used = table.rows.len().saturating_sub(missing);
        let (kind, summary) = match screen_column_kind(table, column) {
            Some(ScreenColumnKind::Numeric) => {
                let data = column_numeric_data(table, column);
                if data.values.is_empty() {
                    ("numeric", "no finite values".to_string())
                } else {
                    let summary = summarize(&data);
                    (
                        "numeric",
                        format!(
                            "median={} IQR={} mean={} SD={}",
                            fmt_number(summary.median),
                            fmt_number(summary.iqr),
                            fmt_number(summary.mean),
                            summary
                                .sd
                                .map(fmt_number)
                                .unwrap_or_else(|| "not defined".into())
                        ),
                    )
                }
            }
            Some(ScreenColumnKind::Categorical) => {
                let mut counts = HashMap::<String, usize>::new();
                let mut order = Vec::new();
                for value in &values {
                    if let Some(label) = category_label(value) {
                        if !counts.contains_key(&label) {
                            order.push(label.clone());
                        }
                        *counts.entry(label).or_default() += 1;
                    }
                }
                let top = order
                    .into_iter()
                    .max_by_key(|label| counts[label])
                    .map(|label| format!("top={} ({})", shortened(&label, 18), counts[&label]))
                    .unwrap_or_else(|| "no observed values".into());
                ("categorical", top)
            }
            None => ("mixed/empty", "review schema".to_string()),
        };
        output.push_str(&format!(
            "{:<name_width$}  {:<11} {:>7} {:>7} {:>7}  {}\n",
            shortened(&table.columns[column], name_width),
            kind,
            used,
            missing,
            unique,
            summary
        ));
    }
    if table.columns.len() > max_columns {
        output.push_str(&format!(
            "\n{} additional column(s) omitted; calculations were not silently sampled.\n",
            table.columns.len() - max_columns
        ));
    }
    output.push_str("\nmiss includes Nil and non-finite numbers. Numeric spread uses sample SD and type-7 quartiles. No data were changed.");
    Ok(text(output))
}

struct LinearDiagnosticFacts {
    xs: Vec<f64>,
    fitted: Vec<f64>,
    residuals: Vec<f64>,
    excluded: usize,
    slope: f64,
    intercept: f64,
}

fn linear_facts(x: &Value, y: &Value, function: &str) -> Result<LinearDiagnosticFacts> {
    let (xs, ys, excluded) = complete_pairs(x, y, function)?;
    if xs.len() < 3 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() requires at least three complete finite pairs"),
            None,
        ));
    }
    let Some((_, slope, intercept)) = pearson(&xs, &ys) else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() requires variation in both x and y"),
            None,
        ));
    };
    let fitted = xs
        .iter()
        .map(|value| intercept + slope * value)
        .collect::<Vec<_>>();
    let residuals = ys
        .iter()
        .zip(&fitted)
        .map(|(observed, predicted)| observed - predicted)
        .collect::<Vec<_>>();
    Ok(LinearDiagnosticFacts {
        xs,
        fitted,
        residuals,
        excluded,
        slope,
        intercept,
    })
}

fn linear_diagnostic_record(facts: &LinearDiagnosticFacts, include_values: bool) -> Value {
    let n = facts.xs.len();
    let residual_data = NumericData {
        values: facts.residuals.clone(),
        original_indices: (0..n).collect(),
        total: n,
        missing: 0,
        non_finite: 0,
    };
    let residual_summary = summarize(&residual_data);
    let residual_ss = facts
        .residuals
        .iter()
        .map(|value| value.powi(2))
        .sum::<f64>();
    let mse = if n > 2 {
        residual_ss / (n - 2) as f64
    } else {
        0.0
    };
    let mean_x = facts.xs.iter().sum::<f64>() / n as f64;
    let sum_xx = facts
        .xs
        .iter()
        .map(|value| (value - mean_x).powi(2))
        .sum::<f64>();
    let absolute_residuals = facts
        .residuals
        .iter()
        .map(|value| value.abs())
        .collect::<Vec<_>>();
    let scale_association = pearson(&facts.fitted, &absolute_residuals).map(|value| value.0);
    let squared_x = facts
        .xs
        .iter()
        .map(|value| (value - mean_x).powi(2))
        .collect::<Vec<_>>();
    let curvature_association = pearson(&squared_x, &facts.residuals).map(|value| value.0);
    let (expected, observed) = normal_qq_values(&residual_data);
    let qq_correlation = pearson(&expected, &observed).map(|value| value.0);
    let durbin_watson = if residual_ss > f64::EPSILON {
        Some(
            facts
                .residuals
                .windows(2)
                .map(|pair| (pair[1] - pair[0]).powi(2))
                .sum::<f64>()
                / residual_ss,
        )
    } else {
        None
    };
    let mut cooks = Vec::with_capacity(n);
    let mut standardized_flags = 0usize;
    for (x, residual) in facts.xs.iter().zip(&facts.residuals) {
        let leverage = if sum_xx > f64::EPSILON {
            1.0 / n as f64 + (x - mean_x).powi(2) / sum_xx
        } else {
            1.0 / n as f64
        };
        let standardized = if mse > f64::EPSILON && leverage < 1.0 {
            residual / (mse * (1.0 - leverage)).sqrt()
        } else {
            0.0
        };
        if standardized.abs() >= 3.0 {
            standardized_flags += 1;
        }
        let cook = if mse > f64::EPSILON && leverage < 1.0 {
            residual.powi(2) / (2.0 * mse) * leverage / (1.0 - leverage).powi(2)
        } else {
            0.0
        };
        cooks.push(cook);
    }
    let cook_threshold = 4.0 / n as f64;
    let influential_flags = cooks
        .iter()
        .filter(|value| **value > cook_threshold)
        .count();
    let maximum_cook = cooks.iter().copied().max_by(f64::total_cmp).unwrap_or(0.0);
    let mut issues = Vec::new();
    if n >= 8 && qq_correlation.is_some_and(|value| value < 0.97) {
        issues.push(issue(
            "residual_qq_curvature",
            format!(
                "Residual normal-Q-Q correlation is {}.",
                fmt_number(qq_correlation.unwrap())
            ),
            "Inspect the Q-Q plot; tail curvature may affect small-sample intervals and tests.",
            "review",
        ));
    }
    if scale_association.is_some_and(|value| value.abs() >= 0.3) {
        issues.push(issue(
            "changing_residual_spread",
            format!(
                "Correlation between fitted values and |residual| is {}.",
                fmt_number(scale_association.unwrap())
            ),
            "A changing residual envelope is a heteroscedasticity clue, not a diagnosis.",
            "review",
        ));
    }
    if curvature_association.is_some_and(|value| value.abs() >= 0.3) {
        issues.push(issue(
            "residual_curvature",
            format!(
                "Correlation between residuals and centred x-squared is {}.",
                fmt_number(curvature_association.unwrap())
            ),
            "A curved residual pattern can indicate that a straight line misses structure.",
            "review",
        ));
    }
    if influential_flags > 0 {
        issues.push(issue(
            "influential_observations",
            format!("{influential_flags} observation(s) exceed the exploratory Cook's-distance threshold 4/n."),
            "Influence is model-specific; inspect provenance and sensitivity without automatic deletion.",
            "review",
        ));
    }
    if standardized_flags > 0 {
        issues.push(issue(
            "large_standardized_residuals",
            format!("{standardized_flags} observation(s) have |internally standardized residual| >= 3."),
            "Large residuals can be valid observations, recording errors, or evidence of model mismatch.",
            "review",
        ));
    }
    let fitted_value = if include_values {
        list(facts.fitted.iter().copied().map(Value::Float).collect())
    } else {
        Value::Nil
    };
    let residual_value = if include_values {
        list(facts.residuals.iter().copied().map(Value::Float).collect())
    } else {
        Value::Nil
    };
    let cook_value = if include_values {
        list(cooks.iter().copied().map(Value::Float).collect())
    } else {
        Value::Nil
    };
    let explanation = format!(
        "Simple linear-model diagnostics\n\nComplete pairs: {}\nExcluded pairs: {}\nSlope: {}\nResidual SD: {}\nQ-Q correlation: {}\nCook review flags: {}\n\nThese are visual and effect-size clues. They do not prove normality, constant variance, independence, correct functional form, or absence of confounding.",
        n,
        facts.excluded,
        fmt_number(facts.slope),
        residual_summary.sd.map(fmt_number).unwrap_or_else(|| "not defined".into()),
        qq_correlation.map(fmt_number).unwrap_or_else(|| "undefined".into()),
        influential_flags
    );
    record([
        ("schema", text("biolang.stats.linear-diagnostics/v1")),
        ("kind", text("simple_linear_diagnostics")),
        ("complete_pairs", Value::Int(n as i64)),
        ("excluded_pairs", Value::Int(facts.excluded as i64)),
        ("slope", Value::Float(facts.slope)),
        ("intercept", Value::Float(facts.intercept)),
        ("residual_summary", compact_summary(&residual_summary)),
        ("residual_mse", Value::Float(mse)),
        ("normal_qq_correlation", number(qq_correlation)),
        ("fitted_absolute_residual_correlation", number(scale_association)),
        ("curvature_correlation", number(curvature_association)),
        ("durbin_watson_in_observation_order", number(durbin_watson)),
        ("cook_threshold", Value::Float(cook_threshold)),
        ("maximum_cook_distance", Value::Float(maximum_cook)),
        ("cook_review_flags", Value::Int(influential_flags as i64)),
        ("standardized_residual_flags", Value::Int(standardized_flags as i64)),
        ("issues", list(issues)),
        ("fitted", fitted_value),
        ("residuals", residual_value),
        ("cook_distances", cook_value),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Diagnostics apply to a simple straight-line model with one predictor.",
                "Durbin-Watson uses current observation order; it is meaningful only when that order represents time or another justified sequence.",
                "Thresholds are review heuristics and should not be converted into automatic data removal.",
                "Confounding, measurement error, grouping, and repeated observations require study-design information.",
            ]),
        ),
        ("quick_explanation", text("Residual form, spread, tails, order, and influence were screened without declaring the model valid or invalid.")),
        ("explanation", text(explanation)),
    ])
}

fn linear_diagnostics(args: Vec<Value>) -> Result<Value> {
    let facts = linear_facts(&args[0], &args[1], "stats_linear_diagnostics")?;
    let opts = options(&args, 2, "stats_linear_diagnostics")?;
    let include_values = opts
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    Ok(linear_diagnostic_record(&facts, include_values))
}

fn linear_diagnostic_plot(args: Vec<Value>) -> Result<Value> {
    let facts = linear_facts(&args[0], &args[1], "stats_linear_diagnostic_plot")?;
    let opts = options(&args, 2, "stats_linear_diagnostic_plot")?;
    let view = opts
        .get("view")
        .and_then(Value::as_str)
        .unwrap_or("residuals");
    if view == "qq" {
        return normal_qq_plot(vec![
            Value::List(
                facts
                    .residuals
                    .iter()
                    .copied()
                    .map(Value::Float)
                    .collect::<Vec<_>>()
                    .into(),
            ),
            Value::Record(opts.into()),
        ]);
    }
    if view != "residuals" {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_linear_diagnostic_plot() view must be residuals or qq",
            None,
        ));
    }
    if plot_format(&opts) == "ascii" {
        let mut chart = ascii_scatter(
            &facts.fitted,
            &facts.residuals,
            opts.get("width")
                .and_then(Value::as_int)
                .unwrap_or(56)
                .clamp(20, 100) as usize,
            opts.get("height")
                .and_then(Value::as_int)
                .unwrap_or(16)
                .clamp(8, 30) as usize,
            "Residuals versus fitted (ASCII)",
            "fitted",
            "residual",
        );
        chart.push_str("\nLook for a roughly horizontal, equally wide cloud around zero; patterns are clues, not diagnoses.");
        return Ok(text(chart));
    }
    let width = opts
        .get("width")
        .and_then(Value::as_float)
        .unwrap_or(720.0)
        .max(480.0);
    let height = opts
        .get("height")
        .and_then(Value::as_float)
        .unwrap_or(500.0)
        .max(360.0);
    let fitted_min = facts.fitted.iter().copied().min_by(f64::total_cmp).unwrap();
    let fitted_max = facts.fitted.iter().copied().max_by(f64::total_cmp).unwrap();
    let residual_min = facts
        .residuals
        .iter()
        .copied()
        .min_by(f64::total_cmp)
        .unwrap()
        .min(0.0);
    let residual_max = facts
        .residuals
        .iter()
        .copied()
        .max_by(f64::total_cmp)
        .unwrap()
        .max(0.0);
    let x_scale = Scale {
        domain: padded_domain(fitted_min, fitted_max),
        range: (65.0, width - 30.0),
    };
    let y_scale = Scale {
        domain: padded_domain(residual_min, residual_max),
        range: (height - 60.0, 45.0),
    };
    let mut canvas = SvgCanvas::new(width, height);
    canvas.margin.left = 65.0;
    canvas.margin.right = 30.0;
    canvas.margin.top = 45.0;
    canvas.margin.bottom = 60.0;
    let stride = facts.fitted.len().div_ceil(10_000).max(1);
    for (fitted, residual) in facts.fitted.iter().zip(&facts.residuals).step_by(stride) {
        canvas.add_circle(x_scale.map(*fitted), y_scale.map(*residual), 3.0, "#2563eb");
    }
    canvas.add_line(
        x_scale.map(x_scale.domain.0),
        y_scale.map(0.0),
        x_scale.map(x_scale.domain.1),
        y_scale.map(0.0),
        "#dc2626",
        1.5,
    );
    canvas.draw_x_axis(&x_scale, "fitted value");
    canvas.draw_y_axis(&y_scale, "residual");
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Residuals versus fitted"),
    );
    canvas.add_text(
        68.0,
        height - 38.0,
        "Red line: zero. Curvature or changing width is a review clue, not a diagnosis.",
        "start",
        10.0,
    );
    Ok(text(canvas.render()))
}

fn log_gamma(value: f64) -> f64 {
    // Lanczos approximation, adequate for descriptive distribution scoring.
    const COEFFICIENTS: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if value < 0.5 {
        return std::f64::consts::PI.ln()
            - (std::f64::consts::PI * value).sin().ln()
            - log_gamma(1.0 - value);
    }
    let z = value - 1.0;
    let mut x = COEFFICIENTS[0];
    for (index, coefficient) in COEFFICIENTS.iter().enumerate().skip(1) {
        x += coefficient / (z + index as f64);
    }
    let t = z + 7.5;
    0.5 * (2.0 * std::f64::consts::PI).ln() + (z + 0.5) * t.ln() - t + x.ln()
}

fn distribution_candidate(
    name: &str,
    available: bool,
    parameters: Value,
    log_likelihood: Option<f64>,
    parameter_count: usize,
    caution: &str,
) -> Value {
    record([
        ("name", text(name)),
        ("available", Value::Bool(available)),
        ("parameters", parameters),
        ("log_likelihood", number(log_likelihood)),
        (
            "aic",
            number(log_likelihood.map(|value| 2.0 * parameter_count as f64 - 2.0 * value)),
        ),
        ("caution", text(caution)),
    ])
}

fn distribution_clues(args: Vec<Value>) -> Result<Value> {
    let data = numeric_data(&args[0], "stats_distribution_clues")?;
    let _opts = options(&args, 1, "stats_distribution_clues")?;
    if data.values.len() < 3 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_distribution_clues() requires at least three finite values",
            None,
        ));
    }
    let n = data.values.len() as f64;
    let mean = data.values.iter().sum::<f64>() / n;
    let population_variance = data
        .values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / n;
    let nonnegative_integers = data
        .values
        .iter()
        .all(|value| *value >= 0.0 && (*value - value.round()).abs() <= 1e-10);
    let all_positive = data.values.iter().all(|value| *value > 0.0);
    let zeros = data.values.iter().filter(|value| **value == 0.0).count();

    let normal_log_likelihood = (population_variance > f64::EPSILON)
        .then(|| -0.5 * n * ((2.0 * std::f64::consts::PI).ln() + 1.0 + population_variance.ln()));
    let normal = distribution_candidate(
        "normal",
        normal_log_likelihood.is_some(),
        record([
            ("mean", Value::Float(mean)),
            ("sd_mle", Value::Float(population_variance.sqrt())),
        ]),
        normal_log_likelihood,
        2,
        "A symmetric bell-shaped density is a model assumption, not a property established by an AIC rank or Q-Q plot.",
    );

    let (lognormal_parameters, lognormal_log_likelihood) = if all_positive {
        let logs = data
            .values
            .iter()
            .map(|value| value.ln())
            .collect::<Vec<_>>();
        let log_mean = logs.iter().sum::<f64>() / n;
        let log_variance = logs
            .iter()
            .map(|value| (value - log_mean).powi(2))
            .sum::<f64>()
            / n;
        let likelihood = (log_variance > f64::EPSILON).then(|| {
            -data.values.iter().map(|value| value.ln()).sum::<f64>()
                - 0.5 * n * ((2.0 * std::f64::consts::PI).ln() + log_variance.ln() + 1.0)
        });
        (
            record([
                ("log_mean", Value::Float(log_mean)),
                ("log_sd_mle", Value::Float(log_variance.sqrt())),
            ]),
            likelihood,
        )
    } else {
        (record([] as [(&str, Value); 0]), None)
    };
    let lognormal = distribution_candidate(
        "lognormal",
        lognormal_log_likelihood.is_some(),
        lognormal_parameters,
        lognormal_log_likelihood,
        2,
        "Only positive values are supported; zeros need a scientifically justified observation or censoring model, not an arbitrary pseudocount.",
    );

    let poisson_log_likelihood = (nonnegative_integers && mean > 0.0).then(|| {
        data.values
            .iter()
            .map(|value| value * mean.ln() - mean - log_gamma(value + 1.0))
            .sum::<f64>()
    });
    let poisson = distribution_candidate(
        "poisson",
        poisson_log_likelihood.is_some(),
        record([("lambda", Value::Float(mean))]),
        poisson_log_likelihood,
        1,
        "Poisson sampling requires count-scale observations and equates conditional mean and variance.",
    );

    let theta = if nonnegative_integers && population_variance > mean && mean > 0.0 {
        Some(mean * mean / (population_variance - mean))
    } else {
        None
    };
    let negative_binomial_log_likelihood = theta.map(|theta| {
        data.values
            .iter()
            .map(|value| {
                log_gamma(value + theta) - log_gamma(theta) - log_gamma(value + 1.0)
                    + theta * (theta / (theta + mean)).ln()
                    + value * (mean / (theta + mean)).ln()
            })
            .sum::<f64>()
    });
    let negative_binomial = distribution_candidate(
        "negative_binomial",
        negative_binomial_log_likelihood.is_some(),
        record([("mean", Value::Float(mean)), ("theta", number(theta))]),
        negative_binomial_log_likelihood,
        2,
        "The method-of-moments dispersion is an exploratory fit; covariates and shrinkage are normally needed for biological count inference.",
    );

    let mut candidates = vec![normal, lognormal, poisson, negative_binomial];
    let best_aic = candidates
        .iter()
        .filter_map(|candidate| match candidate {
            Value::Record(map) => map.get("aic").and_then(Value::as_float),
            _ => None,
        })
        .min_by(f64::total_cmp);
    for candidate in &mut candidates {
        let Value::Record(map) = candidate else {
            continue;
        };
        let delta = map
            .get("aic")
            .and_then(Value::as_float)
            .zip(best_aic)
            .map(|(aic, best)| aic - best);
        Arc::make_mut(map).insert("delta_aic".into(), number(delta));
    }

    let expected_poisson_zeros = if mean >= 0.0 {
        Some(n * (-mean).exp())
    } else {
        None
    };
    let zero_ratio = expected_poisson_zeros
        .filter(|expected| *expected > f64::EPSILON)
        .map(|expected| zeros as f64 / expected);
    let centred_m2 = population_variance;
    let centred_m3 = data
        .values
        .iter()
        .map(|value| (value - mean).powi(3))
        .sum::<f64>()
        / n;
    let centred_m4 = data
        .values
        .iter()
        .map(|value| (value - mean).powi(4))
        .sum::<f64>()
        / n;
    let moment_skewness = (centred_m2 > f64::EPSILON).then(|| centred_m3 / centred_m2.powf(1.5));
    let moment_kurtosis = (centred_m2 > f64::EPSILON).then(|| centred_m4 / centred_m2.powi(2));
    let bimodality_coefficient = moment_skewness
        .zip(moment_kurtosis)
        .filter(|(_, kurtosis)| *kurtosis > f64::EPSILON)
        .map(|(skewness, kurtosis)| (skewness.powi(2) + 1.0) / kurtosis);
    let variance_mean_ratio = (mean > 0.0).then(|| population_variance / mean);
    let mut issues = Vec::new();
    if zero_ratio.is_some_and(|ratio| ratio >= 2.0) && zeros >= 3 {
        issues.push(issue(
            "more_zeros_than_poisson_fit",
            format!(
                "Observed zero count is {zeros}; fitted Poisson expectation is {}.",
                fmt_number(expected_poisson_zeros.unwrap())
            ),
            "Extra zeros can arise from mixtures, exposure, censoring, sampling depth, or model misspecification; they do not by themselves establish zero inflation.",
            "review",
        ));
    }
    if variance_mean_ratio.is_some_and(|ratio| ratio >= 1.5) && nonnegative_integers {
        issues.push(issue(
            "count_overdispersion_clue",
            format!(
                "Count-scale population variance/mean is {}.",
                fmt_number(variance_mean_ratio.unwrap())
            ),
            "Variation exceeds a Poisson mean-variance relation; inspect covariates, exposure, grouping, and a negative-binomial alternative.",
            "review",
        ));
    }
    if bimodality_coefficient.is_some_and(|value| value > 5.0 / 9.0) {
        issues.push(issue(
            "possible_mixture_shape",
            format!(
                "Moment bimodality coefficient is {}.",
                fmt_number(bimodality_coefficient.unwrap())
            ),
            "A high coefficient is only a mixture clue and is unstable in small or heavy-tailed samples; inspect the distribution and known groups.",
            "review",
        ));
    }
    Ok(record([
        ("schema", text("biolang.stats.distribution-clues/v1")),
        ("kind", text("distribution_family_clues")),
        ("n", Value::Int(data.values.len() as i64)),
        ("excluded", Value::Int((data.missing + data.non_finite) as i64)),
        ("nonnegative_integers", Value::Bool(nonnegative_integers)),
        ("all_positive", Value::Bool(all_positive)),
        ("zeros", Value::Int(zeros as i64)),
        ("mean", Value::Float(mean)),
        ("population_variance", Value::Float(population_variance)),
        ("variance_mean_ratio", number(variance_mean_ratio)),
        ("expected_poisson_zeros", number(expected_poisson_zeros)),
        ("observed_expected_zero_ratio", number(zero_ratio)),
        ("bimodality_coefficient", number(bimodality_coefficient)),
        ("candidates", list(candidates)),
        ("issues", list(issues)),
        ("model_selected", Value::Bool(false)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "AIC values compare only the fitted candidate families and do not establish that any candidate is adequate.",
                "The observations are treated as exchangeable and without covariates solely for this descriptive screen.",
                "Mixtures, censoring, truncation, exposure, batch effects, and repeated measurements require explicit models and study context.",
            ]),
        ),
        (
            "quick_explanation",
            text("Several common distribution families were scored as descriptive clues; no family or statistical test was selected."),
        ),
    ]))
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn dataset_report(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "stats_report")?;
    let opts = options(&args, 1, "stats_report")?;
    let format = opts.get("format").and_then(Value::as_str).unwrap_or("html");
    if !matches!(format, "html" | "markdown") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_report() format must be html or markdown",
            None,
        ));
    }
    let title = opts
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("BioLang data health report");
    let generated_at = opts
        .get("generated_at")
        .and_then(Value::as_str)
        .unwrap_or("not supplied");
    let scan = scan_table(vec![args[0].clone(), Value::Record(opts.clone().into())])?;
    let overview = overview_ascii(vec![args[0].clone(), Value::Record(opts.clone().into())])?;
    let overview_text = overview.as_str().unwrap_or("");
    let recommendations = match &scan {
        Value::Record(map) => map.get("recommendations"),
        _ => None,
    };
    let association_pairs = match &scan {
        Value::Record(map) => map.get("associations").and_then(|value| match value {
            Value::Record(associations) => associations.get("pairs"),
            _ => None,
        }),
        _ => None,
    };
    let max_pairs = opts
        .get("report_pairs")
        .and_then(Value::as_int)
        .unwrap_or(10)
        .clamp(0, 50) as usize;
    let max_guidance_columns = opts
        .get("report_numeric_columns")
        .and_then(Value::as_int)
        .unwrap_or(12)
        .clamp(0, 50) as usize;
    let mut centre_scale_guidance = Vec::new();
    let mut centre_scale_rows = Vec::new();
    for (column_index, column) in table.columns.iter().enumerate() {
        if centre_scale_rows.len() >= max_guidance_columns {
            break;
        }
        let values = list(
            table
                .rows
                .iter()
                .map(|row| row.get(column_index).cloned().unwrap_or(Value::Nil))
                .collect(),
        );
        let Ok(data) = numeric_data(&values, "stats_report") else {
            continue;
        };
        if data.values.len() < 2 {
            continue;
        }
        let summary = summarize(&data);
        let robust = summary.skewness.is_some_and(|value| value.abs() >= 0.5)
            || !summary.outlier_positions.is_empty();
        let centre = if robust { "median" } else { "mean" };
        let spread = if robust { "IQR" } else { "standard deviation" };
        let scale = if summary.min > 0.0 && summary.skewness.is_some_and(|value| value >= 0.75) {
            "preview log only for a ratio/multiplicative question"
        } else {
            "retain original scale unless the estimand requires another scale"
        };
        let uncertainty = if robust {
            "quantile/bootstrap interval using the sampling unit"
        } else {
            "design-aware mean SE/confidence interval"
        };
        centre_scale_rows.push((
            column.to_string(),
            centre.to_string(),
            spread.to_string(),
            scale.to_string(),
            uncertainty.to_string(),
        ));
        centre_scale_guidance.push(record([
            ("column", text(column.to_string())),
            ("centre", text(centre)),
            ("spread", text(spread)),
            ("scale", text(scale)),
            ("uncertainty", text(uncertainty)),
            ("raw_skewness", number(summary.skewness)),
            ("heuristic", Value::Bool(true)),
            ("automatic_choice", Value::Bool(false)),
        ]));
    }
    let centre_markdown = if centre_scale_rows.is_empty() {
        "No eligible numeric columns were found.".into()
    } else {
        let rows = centre_scale_rows
            .iter()
            .map(|(column, centre, spread, scale, uncertainty)| {
                format!("| {column} | {centre} | {spread} | {scale} | {uncertainty} |")
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "| Column | Centre clue | Matching spread | Scale clue | Uncertainty |\n|---|---|---|---|---|\n{rows}\n\nThese are descriptive clues, not automatic analysis choices."
        )
    };
    let centre_html = if centre_scale_rows.is_empty() {
        "<p>No eligible numeric columns were found.</p>".into()
    } else {
        let rows = centre_scale_rows
            .iter()
            .map(|(column, centre, spread, scale, uncertainty)| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    html_escape(column),
                    html_escape(centre),
                    html_escape(spread),
                    html_escape(scale),
                    html_escape(uncertainty),
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!("<table><thead><tr><th>Column</th><th>Centre clue</th><th>Matching spread</th><th>Scale clue</th><th>Uncertainty</th></tr></thead><tbody>{rows}</tbody></table><p><small>These are descriptive clues, not automatic analysis choices.</small></p>")
    };
    let decision = decision_map(Vec::new())?;
    let decision_ascii = record_string(&decision, "ascii").unwrap_or("");
    let decision_svg = record_string(&decision, "svg").unwrap_or("");
    let profile = match &scan {
        Value::Record(map) => map.get("profile"),
        _ => None,
    };
    let duplicate_rows = profile
        .map(|value| record_int(value, "duplicate_rows"))
        .unwrap_or(0);
    let (complete_rows, missing_pattern_count) = profile
        .and_then(|value| match value {
            Value::Record(map) => map.get("missingness"),
            _ => None,
        })
        .map(|missingness| {
            (
                record_int(missingness, "complete_rows"),
                record_int(missingness, "patterns_total"),
            )
        })
        .unwrap_or((0, 0));
    let design_value = profile.and_then(|value| match value {
        Value::Record(map) => map.get("design"),
        _ => None,
    });
    let design_issue_count = design_value
        .and_then(|value| match value {
            Value::Record(map) => map.get("issues"),
            _ => None,
        })
        .and_then(|value| match value {
            Value::List(items) => Some(items.len()),
            _ => None,
        })
        .unwrap_or(0);
    let design_clue_names = design_value
        .and_then(|value| match value {
            Value::Record(map) => map.get("design_clues"),
            _ => None,
        })
        .and_then(|value| match value {
            Value::List(items) => Some(
                items
                    .iter()
                    .filter_map(|item| record_string(item, "id"))
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .unwrap_or_default();
    let missing_cells = table
        .rows
        .iter()
        .flat_map(|row| row.iter())
        .filter(|value| is_missing_value(value))
        .count();
    let mut recommendation_rows = Vec::new();
    if let Some(Value::List(items)) = recommendations {
        for item in items.iter() {
            recommendation_rows.push((
                record_string(item, "priority").unwrap_or("review"),
                record_string(item, "next_step").unwrap_or("Review the evidence."),
                record_string(item, "evidence").unwrap_or("No evidence text supplied."),
                record_string(item, "example").unwrap_or(""),
            ));
        }
    }
    let mut pair_rows = Vec::new();
    if let Some(Value::List(items)) = association_pairs {
        for item in items.iter().take(max_pairs) {
            let score = match item {
                Value::Record(map) => map.get("score").and_then(Value::as_float),
                _ => None,
            };
            pair_rows.push((
                record_string(item, "left").unwrap_or("?"),
                record_string(item, "right").unwrap_or("?"),
                record_string(item, "measure").unwrap_or("effect size"),
                score,
            ));
        }
    }
    let backend = crate::gpu::execution_summary();
    let seed = opts.get("seed").and_then(Value::as_int);
    let mut option_keys = opts.keys().cloned().collect::<Vec<_>>();
    option_keys.sort();
    let option_summary = option_keys
        .iter()
        .map(|key| format!("{key}={}", opts[key]))
        .collect::<Vec<_>>()
        .join(", ");
    let content = if format == "markdown" {
        let recommendations_text = recommendation_rows
            .iter()
            .map(|(priority, next, evidence, example)| {
                format!(
                    "- **{priority}** — {next}\n  - Evidence: {evidence}\n  - BioLang: `{example}`"
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let pairs_text = if pair_rows.is_empty() {
            "No eligible association pairs were returned.".into()
        } else {
            pair_rows
                .iter()
                .map(|(left, right, measure, score)| {
                    format!(
                        "- `{left}` / `{right}` — {measure}: {}",
                        score.map(fmt_number).unwrap_or_else(|| "undefined".into())
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!(
            "# {title}\n\n## Reproducibility\n\n- BioLang: {}\n- Backend: {backend}\n- Generated at: {generated_at}\n- Seed: {}\n- Options: {}\n\n## Dataset overview\n\n```text\n{overview_text}\n```\n\n## Integrity, missingness, and design\n\n- Complete rows: {complete_rows} of {}\n- Missing/non-finite cells: {missing_cells}\n- Distinct missingness patterns: {missing_pattern_count}\n- Duplicate complete rows: {duplicate_rows}\n- Design issue clues: {design_issue_count}\n- Design structures: {}\n\n## Centre, spread, scale, and uncertainty\n\n{centre_markdown}\n\n```text\n{decision_ascii}\n```\n\n## Prioritized next steps\n\n{recommendations_text}\n\n## Strongest association clues\n\n{pairs_text}\n\n## Interpretation boundary\n\nThis report is descriptive and non-mutating. It does not diagnose a missing-data mechanism, establish causality, select a statistical test, or validate the experimental design.\n",
            env!("CARGO_PKG_VERSION"),
            seed.map(|value| value.to_string()).unwrap_or_else(|| "not supplied".into()),
            if option_summary.is_empty() { "defaults" } else { &option_summary },
            table.rows.len(),
            if design_clue_names.is_empty() { "none declared or detected".into() } else { design_clue_names.join(", ") },
        )
    } else {
        let recommendation_html = recommendation_rows
            .iter()
            .map(|(priority, next, evidence, example)| {
                format!("<li><strong>{}</strong> — {}<br><small>Evidence: {}</small><pre><code>{}</code></pre></li>", html_escape(priority), html_escape(next), html_escape(evidence), html_escape(example))
            })
            .collect::<Vec<_>>()
            .join("");
        let pair_html = pair_rows
            .iter()
            .map(|(left, right, measure, score)| {
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                    html_escape(left),
                    html_escape(right),
                    html_escape(measure),
                    score.map(fmt_number).unwrap_or_else(|| "undefined".into())
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!(
            "<!doctype html><html><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>{}</title><style>body{{font:16px/1.55 system-ui,sans-serif;max-width:1100px;margin:auto;padding:2rem;color:#172033}}h1,h2{{color:#123b5d}}pre{{white-space:pre-wrap;background:#f4f7fa;padding:1rem;border-radius:.5rem;overflow:auto}}table{{border-collapse:collapse;width:100%}}th,td{{border:1px solid #ccd5df;padding:.45rem;text-align:left}}li{{margin:.8rem 0}}svg{{max-width:100%;height:auto}}.boundary{{border-left:4px solid #d97706;padding:.8rem;background:#fff7ed}}</style></head><body><h1>{}</h1><h2>Reproducibility</h2><table><tr><th>BioLang</th><td>{}</td></tr><tr><th>Backend</th><td>{}</td></tr><tr><th>Generated at</th><td>{}</td></tr><tr><th>Seed</th><td>{}</td></tr><tr><th>Options</th><td>{}</td></tr></table><h2>Dataset overview</h2><pre>{}</pre><h2>Integrity, missingness, and design</h2><table><tr><th>Complete rows</th><td>{} of {}</td></tr><tr><th>Missing/non-finite cells</th><td>{}</td></tr><tr><th>Missingness patterns</th><td>{}</td></tr><tr><th>Duplicate rows</th><td>{}</td></tr><tr><th>Design issue clues</th><td>{}</td></tr><tr><th>Design structures</th><td>{}</td></tr></table><h2>Centre, spread, scale, and uncertainty</h2>{}{}<h2>Prioritized next steps</h2><ol>{}</ol><h2>Strongest association clues</h2><table><thead><tr><th>Left</th><th>Right</th><th>Measure</th><th>Score</th></tr></thead><tbody>{}</tbody></table><h2>Interpretation boundary</h2><p class=\"boundary\">This report is descriptive and non-mutating. It does not diagnose a missing-data mechanism, establish causality, select a statistical test, or validate the experimental design.</p></body></html>",
            html_escape(title),
            html_escape(title),
            env!("CARGO_PKG_VERSION"),
            html_escape(&backend),
            html_escape(generated_at),
            seed.map(|value| value.to_string()).unwrap_or_else(|| "not supplied".into()),
            html_escape(if option_summary.is_empty() { "defaults" } else { &option_summary }),
            html_escape(overview_text),
            complete_rows,
            table.rows.len(),
            missing_cells,
            missing_pattern_count,
            duplicate_rows,
            design_issue_count,
            html_escape(&if design_clue_names.is_empty() { "none declared or detected".into() } else { design_clue_names.join(", ") }),
            centre_html,
            decision_svg,
            recommendation_html,
            pair_html,
        )
    };
    Ok(record([
        ("schema", text("biolang.stats.report/v1")),
        ("kind", text("data_health_report")),
        ("format", text(format)),
        (
            "mime_type",
            text(if format == "html" {
                "text/html"
            } else {
                "text/markdown"
            }),
        ),
        ("title", text(title)),
        ("content", text(content)),
        ("scan", scan),
        ("centre_scale_guidance", list(centre_scale_guidance)),
        ("decision_map", decision),
        (
            "provenance",
            record([
                ("biolang_version", text(env!("CARGO_PKG_VERSION"))),
                ("backend", text(backend)),
                ("generated_at", text(generated_at)),
                ("seed", seed.map(Value::Int).unwrap_or(Value::Nil)),
                ("options", text(option_summary)),
                ("rows", Value::Int(table.rows.len() as i64)),
                ("columns", Value::Int(table.columns.len() as i64)),
            ]),
        ),
        ("automatic_changes", Value::Bool(false)),
    ]))
}

fn option_string_list(
    opts: &HashMap<String, Value>,
    name: &str,
    function: &str,
) -> Result<Vec<String>> {
    match opts.get(name) {
        None | Some(Value::Nil) => Ok(Vec::new()),
        Some(Value::List(values)) => values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                    BioLangError::type_error(
                        format!("{function}() option '{name}' item {index} must be Str"),
                        None,
                    )
                })
            })
            .collect(),
        Some(other) => Err(BioLangError::type_error(
            format!(
                "{function}() option '{name}' must be List of Str, got {}",
                other.type_of()
            ),
            None,
        )),
    }
}

enum PredictorEncoding {
    Numeric,
    Categorical(Vec<String>),
}

struct PreparedModel {
    x: Vec<Vec<f64>>,
    y: Vec<f64>,
    feature_names: Vec<String>,
    original_rows: Vec<usize>,
    excluded_rows: usize,
    encodings: Vec<Value>,
}

fn prepare_model(
    predictors: &Table,
    outcome: &Value,
    opts: &HashMap<String, Value>,
    function: &str,
) -> Result<PreparedModel> {
    let Value::List(outcomes) = outcome else {
        return Err(BioLangError::type_error(
            format!(
                "{function}() outcome must be List, got {}",
                outcome.type_of()
            ),
            None,
        ));
    };
    if outcomes.len() != predictors.rows.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{function}() outcome length {} does not match predictor rows {}",
                outcomes.len(),
                predictors.rows.len()
            ),
            None,
        ));
    }
    let mut excluded_names = option_string_list(opts, "exclude_columns", function)?
        .into_iter()
        .collect::<HashSet<_>>();
    if let Some(group_column) = opts.get("validation_group_column") {
        let Some(group_column) = group_column.as_str() else {
            return Err(BioLangError::type_error(
                format!("{function}() option 'validation_group_column' must be Str"),
                None,
            ));
        };
        if !predictors.columns.iter().any(|name| name == group_column) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() validation group column '{group_column}' was not found"),
                None,
            ));
        }
        excluded_names.insert(group_column.to_string());
    }
    let declared_categorical = option_string_list(opts, "categorical_columns", function)?
        .into_iter()
        .collect::<HashSet<_>>();
    let max_levels = opts
        .get("max_category_levels")
        .and_then(Value::as_int)
        .unwrap_or(20)
        .clamp(2, 100) as usize;
    let mut columns = Vec::<(usize, PredictorEncoding)>::new();
    let mut feature_names = Vec::new();
    let mut feature_groups = HashMap::<String, Vec<usize>>::new();
    let mut encodings = Vec::new();
    let mut predictor_columns = (0..predictors.columns.len()).collect::<Vec<_>>();
    predictor_columns
        .sort_by(|left, right| predictors.columns[*left].cmp(&predictors.columns[*right]));
    for column in predictor_columns {
        let name = &predictors.columns[column];
        if excluded_names.contains(name) {
            continue;
        }
        let force_categorical = declared_categorical.contains(name);
        let kind = if force_categorical {
            Some(ScreenColumnKind::Categorical)
        } else {
            screen_column_kind(predictors, column)
        };
        match kind {
            Some(ScreenColumnKind::Numeric) => {
                let index = feature_names.len();
                feature_names.push(name.clone());
                feature_groups.insert(name.clone(), vec![index]);
                columns.push((column, PredictorEncoding::Numeric));
                encodings.push(record([
                    ("column", text(name)),
                    ("encoding", text("numeric")),
                    ("features", string_list([name])),
                ]));
            }
            Some(ScreenColumnKind::Categorical) => {
                let mut levels = Vec::<String>::new();
                let mut seen = HashSet::new();
                for row in &predictors.rows {
                    let Some(label) = row.get(column).and_then(category_label) else {
                        continue;
                    };
                    if seen.insert(label.clone()) {
                        levels.push(label);
                    }
                }
                if levels.len() > max_levels {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("{function}() categorical predictor '{name}' has {} levels; raise max_category_levels only when that model is intentional", levels.len()),
                        None,
                    ));
                }
                if levels.len() < 2 {
                    continue;
                }
                let reference = levels[0].clone();
                let indices = levels
                    .iter()
                    .skip(1)
                    .map(|level| {
                        let index = feature_names.len();
                        feature_names.push(format!("{name}[{level}]"));
                        index
                    })
                    .collect::<Vec<_>>();
                feature_groups.insert(name.clone(), indices);
                let encoded_features = levels
                    .iter()
                    .skip(1)
                    .map(|level| format!("{name}[{level}]"))
                    .collect::<Vec<_>>();
                columns.push((column, PredictorEncoding::Categorical(levels)));
                encodings.push(record([
                    ("column", text(name)),
                    ("encoding", text("treatment_contrast")),
                    ("reference", text(reference)),
                    ("features", string_list(encoded_features)),
                ]));
            }
            None => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("{function}() predictor '{name}' is empty or mixes unsupported types"),
                    None,
                ));
            }
        }
    }
    if feature_names.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() requires at least one non-constant predictor"),
            None,
        ));
    }
    let interactions = option_string_list(opts, "interactions", function)?;
    let mut interaction_specs = Vec::<(String, Vec<usize>, Vec<usize>)>::new();
    for interaction in interactions {
        let Some((left, right)) = interaction.split_once(':') else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "{function}() interaction '{interaction}' must have form column_a:column_b"
                ),
                None,
            ));
        };
        let Some(left_indices) = feature_groups.get(left) else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() interaction column '{left}' is unavailable"),
                None,
            ));
        };
        let Some(right_indices) = feature_groups.get(right) else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() interaction column '{right}' is unavailable"),
                None,
            ));
        };
        interaction_specs.push((interaction, left_indices.clone(), right_indices.clone()));
    }
    let base_feature_count = feature_names.len();
    for (interaction, left, right) in &interaction_specs {
        for left_index in left {
            for right_index in right {
                feature_names.push(format!(
                    "{}:{}",
                    feature_names[*left_index], feature_names[*right_index]
                ));
            }
        }
        encodings.push(record([
            ("column", text(interaction)),
            ("encoding", text("interaction_products")),
            (
                "features_added",
                Value::Int((left.len() * right.len()) as i64),
            ),
        ]));
    }
    let max_features = opts
        .get("max_model_features")
        .and_then(Value::as_int)
        .unwrap_or(100)
        .clamp(1, 500) as usize;
    if feature_names.len() > max_features {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() encoding produced {} features, above max_model_features={max_features}", feature_names.len()),
            None,
        ));
    }

    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut original_rows = Vec::new();
    for (row_index, row) in predictors.rows.iter().enumerate() {
        let Some(outcome_value) = finite_number(&outcomes[row_index]) else {
            continue;
        };
        let mut features = Vec::with_capacity(feature_names.len());
        let mut complete = true;
        for (column, encoding) in &columns {
            match encoding {
                PredictorEncoding::Numeric => {
                    let Some(value) = row.get(*column).and_then(finite_number) else {
                        complete = false;
                        break;
                    };
                    features.push(value);
                }
                PredictorEncoding::Categorical(levels) => {
                    let Some(value) = row.get(*column).and_then(category_label) else {
                        complete = false;
                        break;
                    };
                    for level in levels.iter().skip(1) {
                        features.push(if &value == level { 1.0 } else { 0.0 });
                    }
                }
            }
        }
        if !complete {
            continue;
        }
        debug_assert_eq!(features.len(), base_feature_count);
        for (_, left, right) in &interaction_specs {
            for left_index in left {
                for right_index in right {
                    features.push(features[*left_index] * features[*right_index]);
                }
            }
        }
        x.push(features);
        y.push(outcome_value);
        original_rows.push(row_index);
    }
    Ok(PreparedModel {
        excluded_rows: predictors.rows.len().saturating_sub(x.len()),
        x,
        y,
        feature_names,
        original_rows,
        encodings,
    })
}

fn inverse_xtx(x: &[Vec<f64>]) -> Option<Vec<Vec<f64>>> {
    let p = x.first()?.len() + 1;
    let mut augmented = vec![vec![0.0; p * 2]; p];
    for row in x {
        for left in 0..p {
            let left_value = if left == 0 { 1.0 } else { row[left - 1] };
            for right in 0..p {
                let right_value = if right == 0 { 1.0 } else { row[right - 1] };
                augmented[left][right] += left_value * right_value;
            }
        }
    }
    for index in 0..p {
        augmented[index][p + index] = 1.0;
    }
    for column in 0..p {
        let pivot_row = (column..p).max_by(|left, right| {
            augmented[*left][column]
                .abs()
                .total_cmp(&augmented[*right][column].abs())
        })?;
        augmented.swap(column, pivot_row);
        let pivot = augmented[column][column];
        if pivot.abs() <= 1e-12 {
            return None;
        }
        for value in &mut augmented[column] {
            *value /= pivot;
        }
        for row in 0..p {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            for index in 0..p * 2 {
                augmented[row][index] -= factor * augmented[column][index];
            }
        }
    }
    Some(augmented.into_iter().map(|row| row[p..].to_vec()).collect())
}

fn inverse_xtwx(x: &[Vec<f64>], weights: &[f64]) -> Option<Vec<Vec<f64>>> {
    if x.len() != weights.len() {
        return None;
    }
    let p = x.first()?.len() + 1;
    let mut augmented = vec![vec![0.0; p * 2]; p];
    for (row, weight) in x.iter().zip(weights) {
        for left in 0..p {
            let left_value = if left == 0 { 1.0 } else { row[left - 1] };
            for right in 0..p {
                let right_value = if right == 0 { 1.0 } else { row[right - 1] };
                augmented[left][right] += weight * left_value * right_value;
            }
        }
    }
    for index in 0..p {
        augmented[index][p + index] = 1.0;
    }
    for column in 0..p {
        let pivot_row = (column..p).max_by(|left, right| {
            augmented[*left][column]
                .abs()
                .total_cmp(&augmented[*right][column].abs())
        })?;
        augmented.swap(column, pivot_row);
        let pivot = augmented[column][column];
        if pivot.abs() <= 1e-12 {
            return None;
        }
        for value in &mut augmented[column] {
            *value /= pivot;
        }
        for row in 0..p {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            for index in 0..p * 2 {
                augmented[row][index] -= factor * augmented[column][index];
            }
        }
    }
    Some(augmented.into_iter().map(|row| row[p..].to_vec()).collect())
}

fn glm_diagnostics(args: Vec<Value>) -> Result<Value> {
    let function = "stats_glm_diagnostics";
    let predictors = require_table(&args[0], function)?;
    let opts = options(&args, 2, function)?;
    let family = opts
        .get("family")
        .and_then(Value::as_str)
        .unwrap_or("binomial");
    if !matches!(family, "binomial" | "logistic" | "poisson") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() family must be binomial or poisson"),
            None,
        ));
    }
    let family = if family == "logistic" {
        "binomial"
    } else {
        family
    };
    let prepared = prepare_model(predictors, &args[1], &opts, function)?;
    let n = prepared.y.len();
    let feature_count = prepared.feature_names.len();
    let parameter_count = feature_count + 1;
    if n <= parameter_count {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() has {n} complete rows but needs more than {parameter_count} for the encoded model"),
            None,
        ));
    }
    if family == "binomial"
        && prepared
            .y
            .iter()
            .any(|value| *value != 0.0 && *value != 1.0)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() binomial outcomes must be exactly 0 or 1"),
            None,
        ));
    }
    if family == "poisson"
        && prepared
            .y
            .iter()
            .any(|value| *value < 0.0 || value.fract().abs() > 1e-12)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() Poisson outcomes must be non-negative integers"),
            None,
        ));
    }

    let fit = if family == "binomial" {
        crate::stats::logistic_regression_multi(&prepared.y, &prepared.x)
    } else {
        crate::stats::poisson_regression(&prepared.y, &prepared.x)
    }
    .map_err(|message| BioLangError::runtime(ErrorKind::TypeError, message, None))?;
    let coefficients = fit.coefficients;
    let p_values = fit.p_values;
    let fitter_aic = fit.aic;
    let converged = fit.converged;
    let iterations = fit.iterations;
    let design_rows = prepared
        .x
        .iter()
        .map(|row| {
            std::iter::once(1.0)
                .chain(row.iter().copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let linear_predictors = design_rows
        .iter()
        .map(|row| dot(row, &coefficients))
        .collect::<Vec<_>>();
    let fitted = linear_predictors
        .iter()
        .map(|eta| {
            if family == "binomial" {
                1.0 / (1.0 + (-eta).exp())
            } else {
                eta.exp().min(1e10)
            }
        })
        .collect::<Vec<_>>();
    let variances = fitted
        .iter()
        .map(|mu| {
            if family == "binomial" {
                (mu * (1.0 - mu)).max(1e-10)
            } else {
                (*mu).max(1e-10)
            }
        })
        .collect::<Vec<_>>();
    let inverse = inverse_xtwx(&prepared.x, &variances).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() weighted model matrix is singular or numerically unstable"),
            None,
        )
    })?;
    let pearson_residuals = prepared
        .y
        .iter()
        .zip(&fitted)
        .zip(&variances)
        .map(|((observed, fitted), variance)| (observed - fitted) / variance.sqrt())
        .collect::<Vec<_>>();
    let deviance_residuals = prepared
        .y
        .iter()
        .zip(&fitted)
        .map(|(observed, fitted)| {
            let component = if family == "binomial" {
                let mu = fitted.clamp(1e-15, 1.0 - 1e-15);
                let observed_term = if *observed > 0.0 {
                    observed * (observed / mu).ln()
                } else {
                    0.0
                };
                let complement = 1.0 - observed;
                let complement_term = if complement > 0.0 {
                    complement * (complement / (1.0 - mu)).ln()
                } else {
                    0.0
                };
                2.0 * (observed_term + complement_term)
            } else if *observed > 0.0 {
                2.0 * (observed * (observed / fitted.max(1e-15)).ln() - (observed - fitted))
            } else {
                2.0 * fitted
            };
            (observed - fitted).signum() * component.max(0.0).sqrt()
        })
        .collect::<Vec<_>>();
    let residual_deviance = deviance_residuals
        .iter()
        .map(|value| value * value)
        .sum::<f64>();
    let degrees_freedom = n - parameter_count;
    let pearson_chi_squared = pearson_residuals
        .iter()
        .map(|value| value * value)
        .sum::<f64>();
    let dispersion = pearson_chi_squared / degrees_freedom as f64;
    let null_mean = (prepared.y.iter().sum::<f64>() / n as f64).clamp(
        if family == "binomial" { 1e-15 } else { 0.0 },
        if family == "binomial" {
            1.0 - 1e-15
        } else {
            f64::INFINITY
        },
    );
    let null_deviance = prepared
        .y
        .iter()
        .map(|observed| {
            if family == "binomial" {
                let first = if *observed > 0.0 {
                    observed * (observed / null_mean).ln()
                } else {
                    0.0
                };
                let complement = 1.0 - observed;
                let second = if complement > 0.0 {
                    complement * (complement / (1.0 - null_mean)).ln()
                } else {
                    0.0
                };
                2.0 * (first + second)
            } else if *observed > 0.0 {
                2.0 * (observed * (observed / null_mean.max(1e-15)).ln() - (observed - null_mean))
            } else {
                2.0 * null_mean
            }
        })
        .sum::<f64>();
    let log_likelihood = if family == "binomial" {
        prepared
            .y
            .iter()
            .zip(&fitted)
            .map(|(observed, fitted)| {
                let mu = fitted.clamp(1e-15, 1.0 - 1e-15);
                observed * mu.ln() + (1.0 - observed) * (1.0 - mu).ln()
            })
            .sum::<f64>()
    } else {
        prepared
            .y
            .iter()
            .zip(&fitted)
            .map(|(observed, fitted)| {
                observed * fitted.max(1e-15).ln() - fitted - log_gamma(observed + 1.0)
            })
            .sum::<f64>()
    };
    let aic = -2.0 * log_likelihood + 2.0 * parameter_count as f64;
    let confidence = opts
        .get("confidence")
        .and_then(Value::as_float)
        .unwrap_or(0.95);
    if !(0.5..1.0).contains(&confidence) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() confidence must be between 0.5 and 1"),
            None,
        ));
    }
    let critical = bl_core::bio_core::stats_ops::normal_quantile((1.0 + confidence) / 2.0);
    let mut coefficient_rows = Vec::new();
    for index in 0..parameter_count {
        let standard_error = inverse[index][index].max(0.0).sqrt();
        coefficient_rows.push(record([
            (
                "name",
                text(if index == 0 {
                    "(intercept)"
                } else {
                    &prepared.feature_names[index - 1]
                }),
            ),
            ("estimate", Value::Float(coefficients[index])),
            ("standard_error", Value::Float(standard_error)),
            ("p_value", Value::Float(p_values[index])),
            (
                "confidence_lower",
                Value::Float(coefficients[index] - critical * standard_error),
            ),
            (
                "confidence_upper",
                Value::Float(coefficients[index] + critical * standard_error),
            ),
            ("effect_ratio", Value::Float(coefficients[index].exp())),
        ]));
    }
    let mut leverages = Vec::with_capacity(n);
    let mut standardized_pearson = Vec::with_capacity(n);
    let mut cooks = Vec::with_capacity(n);
    for ((row, weight), pearson) in design_rows.iter().zip(&variances).zip(&pearson_residuals) {
        let transformed = inverse
            .iter()
            .map(|inverse_row| dot(inverse_row, row))
            .collect::<Vec<_>>();
        let leverage = (weight * dot(row, &transformed)).clamp(0.0, 1.0);
        let denominator = (1.0 - leverage).max(1e-12);
        leverages.push(leverage);
        standardized_pearson.push(pearson / denominator.sqrt());
        cooks.push(pearson.powi(2) * leverage / (parameter_count as f64 * denominator.powi(2)));
    }
    let cook_threshold = 4.0 / n as f64;
    let leverage_threshold = 2.0 * parameter_count as f64 / n as f64;
    let mut review_rows = Vec::new();
    for index in 0..n {
        if standardized_pearson[index].abs() >= 3.0
            || cooks[index] > cook_threshold
            || leverages[index] > leverage_threshold
        {
            review_rows.push(record([
                ("row", Value::Int(prepared.original_rows[index] as i64)),
                ("observed", Value::Float(prepared.y[index])),
                ("fitted", Value::Float(fitted[index])),
                (
                    "standardized_pearson",
                    Value::Float(standardized_pearson[index]),
                ),
                ("leverage", Value::Float(leverages[index])),
                ("cook_distance", Value::Float(cooks[index])),
                ("action", text("inspect; do not automatically delete")),
            ]));
        }
    }
    let observed_zeros = prepared.y.iter().filter(|value| **value == 0.0).count();
    let expected_zeros =
        (family == "poisson").then(|| fitted.iter().map(|value| (-value).exp()).sum::<f64>());
    let brier_score = (family == "binomial").then(|| {
        prepared
            .y
            .iter()
            .zip(&fitted)
            .map(|(observed, fitted)| (observed - fitted).powi(2))
            .sum::<f64>()
            / n as f64
    });
    let mut calibration = Vec::new();
    if family == "binomial" {
        let mut order = (0..n).collect::<Vec<_>>();
        order.sort_by(|left, right| fitted[*left].total_cmp(&fitted[*right]));
        let bins = n.min(10);
        for bin_index in 0..bins {
            let start = bin_index * n / bins;
            let end = (bin_index + 1) * n / bins;
            if start == end {
                continue;
            }
            let indices = &order[start..end];
            calibration.push(record([
                ("bin", Value::Int((bin_index + 1) as i64)),
                ("observations", Value::Int(indices.len() as i64)),
                (
                    "mean_fitted",
                    Value::Float(
                        indices.iter().map(|index| fitted[*index]).sum::<f64>()
                            / indices.len() as f64,
                    ),
                ),
                (
                    "observed_event_fraction",
                    Value::Float(
                        indices.iter().map(|index| prepared.y[*index]).sum::<f64>()
                            / indices.len() as f64,
                    ),
                ),
            ]));
        }
    }
    let mut issues = Vec::new();
    if dispersion > 1.5 {
        issues.push(issue(
            "overdispersion_clue",
            format!("Pearson dispersion is {}.", fmt_number(dispersion)),
            if family == "poisson" {
                "Inspect exposure, omitted structure, dependence, and negative-binomial/quasi-Poisson alternatives."
            } else {
                "Inspect dependence, omitted structure, and grouped-binomial assumptions; Bernoulli dispersion is not a free fitted parameter."
            },
            "review",
        ));
    }
    if !converged {
        issues.push(issue(
            "fit_not_converged",
            format!("Iteratively reweighted least squares reached its {iterations}-iteration limit without the coefficients settling."),
            "Treat every coefficient, interval, and diagnostic below as provisional; inspect separation, collinearity, and predictor scaling before interpreting them.",
            "blocking",
        ));
    }
    if fitted.iter().any(|value| *value < 1e-6)
        || (family == "binomial" && fitted.iter().any(|value| *value > 1.0 - 1e-6))
    {
        issues.push(issue(
            "boundary_fit_clue",
            "At least one fitted mean is extremely close to the response boundary.",
            "Inspect separation, sparse factor levels, extrapolation, and coefficient stability.",
            "review",
        ));
    }
    if !review_rows.is_empty() {
        issues.push(issue(
            "influence_review",
            format!("{} row(s) crossed a residual, leverage, or Cook review threshold.", review_rows.len()),
            "Inspect data provenance and refit sensitivity; do not delete rows from these flags alone.",
            "review",
        ));
    }
    let ascii = format!(
        "GLM diagnostic ({family}, n={n}, parameters={parameter_count})\n{}residual deviance={} on {} df\nnull deviance={}  Pearson dispersion={}\nAIC={}{}\nreview rows={} (inspect, do not automatically delete)\n\nCoefficients are on the link scale; exp(coefficient) is a conditional odds/rate ratio. No causal interpretation or model selection is automatic.",
        if converged {
            String::new()
        } else {
            format!("NOT CONVERGED after {iterations} IRLS iterations; every number below is provisional\n")
        },
        fmt_number(residual_deviance),
        degrees_freedom,
        fmt_number(null_deviance),
        fmt_number(dispersion),
        fmt_number(aic),
        if (aic - fitter_aic).abs() > 1e-8 && family == "poisson" {
            " (includes the Poisson factorial constant)"
        } else {
            ""
        },
        review_rows.len(),
    );
    let include_values = opts
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(record([
        ("schema", text("biolang.stats.glm-diagnostics/v1")),
        ("kind", text("glm_diagnostics")),
        ("family", text(family)),
        ("complete_rows", Value::Int(n as i64)),
        ("excluded_rows", Value::Int(prepared.excluded_rows as i64)),
        ("encoded_predictors", Value::Int(feature_count as i64)),
        ("parameters", Value::Int(parameter_count as i64)),
        ("degrees_freedom", Value::Int(degrees_freedom as i64)),
        ("encodings", list(prepared.encodings)),
        ("coefficients", list(coefficient_rows)),
        ("log_likelihood", Value::Float(log_likelihood)),
        ("aic", Value::Float(aic)),
        ("null_deviance", Value::Float(null_deviance)),
        ("residual_deviance", Value::Float(residual_deviance)),
        ("pearson_chi_squared", Value::Float(pearson_chi_squared)),
        ("pearson_dispersion", Value::Float(dispersion)),
        ("brier_score", number(brier_score)),
        ("observed_zeros", Value::Int(observed_zeros as i64)),
        ("expected_poisson_zeros", number(expected_zeros)),
        ("calibration_bins", list(calibration)),
        ("cook_threshold", Value::Float(cook_threshold)),
        ("leverage_threshold", Value::Float(leverage_threshold)),
        (
            "maximum_leverage",
            Value::Float(leverages.iter().copied().max_by(f64::total_cmp).unwrap_or(0.0)),
        ),
        (
            "maximum_cook_distance",
            Value::Float(cooks.iter().copied().max_by(f64::total_cmp).unwrap_or(0.0)),
        ),
        ("review_rows", list(review_rows)),
        (
            "fitted",
            if include_values {
                list(fitted.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        (
            "pearson_residuals",
            if include_values {
                list(pearson_residuals.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        (
            "deviance_residuals",
            if include_values {
                list(deviance_residuals.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        ("converged", Value::Bool(converged)),
        ("iterations", Value::Int(iterations as i64)),
        ("ascii", text(ascii)),
        ("issues", list(issues)),
        ("model_selected", Value::Bool(false)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Every number here is conditional on `converged`; an unconverged fit still returns coefficients, and they are provisional.",
                "Residual, leverage, and Cook thresholds are diagnostic review clues, not deletion rules.",
                "Pearson dispersion is descriptive; grouped binomial, repeated observations, survey designs, and exposure offsets require explicit structure.",
                "Calibration bins are equal-count descriptive summaries and are not a formal goodness-of-fit test.",
                "Coefficient intervals use model-based large-sample Wald standard errors.",
            ]),
        ),
    ]))
}

fn invert_with_log_determinant(matrix: &[Vec<f64>]) -> Option<(Vec<Vec<f64>>, f64)> {
    let n = matrix.len();
    if n == 0 || matrix.iter().any(|row| row.len() != n) {
        return None;
    }
    let mut augmented = vec![vec![0.0; 2 * n]; n];
    for row in 0..n {
        for column in 0..n {
            augmented[row][column] = matrix[row][column];
        }
        augmented[row][n + row] = 1.0;
    }
    let mut log_determinant = 0.0;
    for column in 0..n {
        let pivot_row = (column..n).max_by(|left, right| {
            augmented[*left][column]
                .abs()
                .total_cmp(&augmented[*right][column].abs())
        })?;
        augmented.swap(column, pivot_row);
        let pivot = augmented[column][column];
        if !pivot.is_finite() || pivot.abs() <= 1e-12 {
            return None;
        }
        log_determinant += pivot.abs().ln();
        for index in 0..2 * n {
            augmented[column][index] /= pivot;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            for index in 0..2 * n {
                augmented[row][index] -= factor * augmented[column][index];
            }
        }
    }
    Some((
        augmented.into_iter().map(|row| row[n..].to_vec()).collect(),
        log_determinant,
    ))
}

fn apply_random_intercept_inverse(
    values: &[f64],
    groups: &[usize],
    group_sizes: &[usize],
    lambda: f64,
) -> Vec<f64> {
    let mut sums = vec![0.0; group_sizes.len()];
    for (value, group) in values.iter().zip(groups) {
        sums[*group] += value;
    }
    values
        .iter()
        .zip(groups)
        .map(|(value, group)| {
            value - lambda / (1.0 + lambda * group_sizes[*group] as f64) * sums[*group]
        })
        .collect()
}

struct RandomInterceptProfile {
    beta: Vec<f64>,
    beta_information_inverse: Vec<Vec<f64>>,
    sigma_residual_squared: f64,
    log_likelihood: f64,
}

fn random_intercept_profile(
    design: &[Vec<f64>],
    outcome: &[f64],
    groups: &[usize],
    group_sizes: &[usize],
    lambda: f64,
    reml: bool,
) -> Option<RandomInterceptProfile> {
    let n = outcome.len();
    let p = design.first()?.len();
    if n <= p || lambda < 0.0 || !lambda.is_finite() {
        return None;
    }
    let inverse_outcome = apply_random_intercept_inverse(outcome, groups, group_sizes, lambda);
    let mut inverse_columns = vec![vec![0.0; n]; p];
    for column in 0..p {
        let values = design.iter().map(|row| row[column]).collect::<Vec<_>>();
        inverse_columns[column] =
            apply_random_intercept_inverse(&values, groups, group_sizes, lambda);
    }
    let mut information = vec![vec![0.0; p]; p];
    let mut score = vec![0.0; p];
    for left in 0..p {
        score[left] = design
            .iter()
            .zip(&inverse_outcome)
            .map(|(row, value)| row[left] * value)
            .sum();
        for right in 0..p {
            information[left][right] = design
                .iter()
                .zip(&inverse_columns[right])
                .map(|(row, value)| row[left] * value)
                .sum();
        }
    }
    let (information_inverse, log_information_determinant) =
        invert_with_log_determinant(&information)?;
    let beta = information_inverse
        .iter()
        .map(|row| dot(row, &score))
        .collect::<Vec<_>>();
    let residuals = design
        .iter()
        .zip(outcome)
        .map(|(row, observed)| observed - dot(row, &beta))
        .collect::<Vec<_>>();
    let inverse_residuals = apply_random_intercept_inverse(&residuals, groups, group_sizes, lambda);
    let quadratic = dot(&residuals, &inverse_residuals).max(f64::MIN_POSITIVE);
    let scale_df = if reml { n - p } else { n };
    let sigma_residual_squared = quadratic / scale_df as f64;
    let log_v0_determinant = group_sizes
        .iter()
        .map(|size| (1.0 + lambda * *size as f64).ln())
        .sum::<f64>();
    let log_likelihood = -0.5
        * (scale_df as f64
            * ((2.0 * std::f64::consts::PI).ln() + sigma_residual_squared.ln() + 1.0)
            + log_v0_determinant
            + if reml {
                log_information_determinant
            } else {
                0.0
            });
    Some(RandomInterceptProfile {
        beta,
        beta_information_inverse: information_inverse,
        sigma_residual_squared,
        log_likelihood,
    })
}

fn random_intercept_model(args: Vec<Value>) -> Result<Value> {
    let function = "stats_random_intercept_model";
    let predictors = require_table(&args[0], function)?;
    let Value::List(cluster_values) = &args[2] else {
        return Err(BioLangError::type_error(
            format!(
                "{function}() clusters must be List, got {}",
                args[2].type_of()
            ),
            None,
        ));
    };
    if cluster_values.len() != predictors.rows.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{function}() cluster length {} does not match predictor rows {}",
                cluster_values.len(),
                predictors.rows.len()
            ),
            None,
        ));
    }
    let opts = options(&args, 3, function)?;
    let method = opts
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or("reml")
        .to_ascii_lowercase();
    if !matches!(method.as_str(), "reml" | "ml") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() method must be reml or ml"),
            None,
        ));
    }
    let reml = method == "reml";
    let prepared = prepare_model(predictors, &args[1], &opts, function)?;
    let mut x = Vec::new();
    let mut y = Vec::new();
    let mut original_rows = Vec::new();
    let mut cluster_labels = Vec::new();
    let mut cluster_lookup = HashMap::<String, usize>::new();
    let mut groups = Vec::new();
    let mut excluded_cluster_rows = 0usize;
    for index in 0..prepared.y.len() {
        let original_row = prepared.original_rows[index];
        let Some(label) = cluster_values.get(original_row).and_then(category_label) else {
            excluded_cluster_rows += 1;
            continue;
        };
        let group = if let Some(group) = cluster_lookup.get(&label) {
            *group
        } else {
            let group = cluster_labels.len();
            cluster_labels.push(label.clone());
            cluster_lookup.insert(label, group);
            group
        };
        x.push(prepared.x[index].clone());
        y.push(prepared.y[index]);
        original_rows.push(original_row);
        groups.push(group);
    }
    let n = y.len();
    let fixed_features = prepared.feature_names.len();
    let parameter_count = fixed_features + 1;
    let cluster_count = cluster_labels.len();
    if cluster_count < 2 || n <= parameter_count || n <= cluster_count {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() needs at least two clusters, repeated observations, and more complete rows than fixed-effect parameters"),
            None,
        ));
    }
    let mut group_sizes = vec![0usize; cluster_count];
    for group in &groups {
        group_sizes[*group] += 1;
    }
    if group_sizes.iter().all(|size| *size == 1) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() cannot estimate a random-intercept variance when every cluster has one observation"),
            None,
        ));
    }
    let design = x
        .iter()
        .map(|row| {
            std::iter::once(1.0)
                .chain(row.iter().copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let evaluate = |log_lambda: f64| {
        let lambda = log_lambda.exp();
        random_intercept_profile(&design, &y, &groups, &group_sizes, lambda, reml)
            .map(|profile| (profile.log_likelihood, profile))
    };
    let boundary_profile = random_intercept_profile(&design, &y, &groups, &group_sizes, 0.0, reml)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() fixed-effect information matrix is singular"),
                None,
            )
        })?;
    let mut best_log_lambda = -16.0;
    let mut best_log_likelihood = boundary_profile.log_likelihood;
    for index in 0..=128 {
        let candidate = -16.0 + index as f64 * 0.25;
        if let Some((likelihood, _)) = evaluate(candidate) {
            if likelihood > best_log_likelihood {
                best_log_likelihood = likelihood;
                best_log_lambda = candidate;
            }
        }
    }
    let mut left = (best_log_lambda - 0.5).max(-20.0);
    let mut right = (best_log_lambda + 0.5).min(20.0);
    let golden = (5.0_f64.sqrt() - 1.0) / 2.0;
    let mut c = right - golden * (right - left);
    let mut d = left + golden * (right - left);
    let mut fc = evaluate(c)
        .map(|value| value.0)
        .unwrap_or(f64::NEG_INFINITY);
    let mut fd = evaluate(d)
        .map(|value| value.0)
        .unwrap_or(f64::NEG_INFINITY);
    for _ in 0..100 {
        if (right - left).abs() < 1e-10 {
            break;
        }
        if fc > fd {
            right = d;
            d = c;
            fd = fc;
            c = right - golden * (right - left);
            fc = evaluate(c)
                .map(|value| value.0)
                .unwrap_or(f64::NEG_INFINITY);
        } else {
            left = c;
            c = d;
            fc = fd;
            d = left + golden * (right - left);
            fd = evaluate(d)
                .map(|value| value.0)
                .unwrap_or(f64::NEG_INFINITY);
        }
    }
    let optimized_log_lambda = (left + right) / 2.0;
    let optimized = evaluate(optimized_log_lambda);
    let (lambda, profile) = if let Some((likelihood, profile)) = optimized {
        if likelihood > boundary_profile.log_likelihood + 1e-10 {
            (optimized_log_lambda.exp(), profile)
        } else {
            (0.0, boundary_profile)
        }
    } else {
        (0.0, boundary_profile)
    };
    let residual_variance = profile.sigma_residual_squared;
    let random_intercept_variance = lambda * residual_variance;
    let total_variance = random_intercept_variance + residual_variance;
    let icc = if total_variance > 0.0 {
        random_intercept_variance / total_variance
    } else {
        0.0
    };
    let confidence = opts
        .get("confidence")
        .and_then(Value::as_float)
        .unwrap_or(0.95);
    if !(0.5..1.0).contains(&confidence) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() confidence must be between 0.5 and 1"),
            None,
        ));
    }
    let critical = bl_core::bio_core::stats_ops::normal_quantile((1.0 + confidence) / 2.0);
    let fixed_names = std::iter::once("(intercept)".to_string())
        .chain(prepared.feature_names.iter().cloned())
        .collect::<Vec<_>>();
    let fixed_effects = profile
        .beta
        .iter()
        .enumerate()
        .map(|(index, estimate)| {
            let standard_error = (residual_variance
                * profile.beta_information_inverse[index][index].max(0.0))
            .sqrt();
            let z = if standard_error > 0.0 {
                estimate / standard_error
            } else {
                0.0
            };
            record([
                ("name", text(&fixed_names[index])),
                ("estimate", Value::Float(*estimate)),
                ("standard_error", Value::Float(standard_error)),
                ("z_value", Value::Float(z)),
                (
                    "p_value_normal_approximation",
                    Value::Float(2.0 * bl_core::bio_core::stats_ops::normal_sf(z.abs())),
                ),
                (
                    "confidence_lower",
                    Value::Float(estimate - critical * standard_error),
                ),
                (
                    "confidence_upper",
                    Value::Float(estimate + critical * standard_error),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let marginal_fitted = design
        .iter()
        .map(|row| dot(row, &profile.beta))
        .collect::<Vec<_>>();
    let marginal_residuals = y
        .iter()
        .zip(&marginal_fitted)
        .map(|(observed, fitted)| observed - fitted)
        .collect::<Vec<_>>();
    let mut group_residual_sums = vec![0.0; cluster_count];
    for (residual, group) in marginal_residuals.iter().zip(&groups) {
        group_residual_sums[*group] += residual;
    }
    let random_effect_values = (0..cluster_count)
        .map(|group| {
            lambda / (1.0 + lambda * group_sizes[group] as f64) * group_residual_sums[group]
        })
        .collect::<Vec<_>>();
    let conditional_fitted = marginal_fitted
        .iter()
        .zip(&groups)
        .map(|(fitted, group)| fitted + random_effect_values[*group])
        .collect::<Vec<_>>();
    let conditional_residuals = y
        .iter()
        .zip(&conditional_fitted)
        .map(|(observed, fitted)| observed - fitted)
        .collect::<Vec<_>>();
    let max_cluster_details = opts
        .get("max_cluster_details")
        .and_then(Value::as_int)
        .unwrap_or(50)
        .clamp(0, 500) as usize;
    let random_effects = cluster_labels
        .iter()
        .enumerate()
        .take(max_cluster_details)
        .map(|(group, label)| {
            record([
                ("cluster", text(label)),
                ("observations", Value::Int(group_sizes[group] as i64)),
                (
                    "random_intercept",
                    Value::Float(random_effect_values[group]),
                ),
                (
                    "shrinkage_weight",
                    Value::Float(
                        lambda * group_sizes[group] as f64
                            / (1.0 + lambda * group_sizes[group] as f64),
                    ),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    if lambda == 0.0 || random_intercept_variance <= 1e-10 * residual_variance.max(1.0) {
        issues.push(issue(
            "variance_boundary",
            "The estimated random-intercept variance is on or very near zero.",
            "Treat the random effect as a boundary estimate; compare scientific design requirements and sensitivity rather than relying on an ordinary chi-square test.",
            "review",
        ));
    }
    let minimum_size = group_sizes.iter().copied().min().unwrap_or(0);
    let maximum_size = group_sizes.iter().copied().max().unwrap_or(0);
    if maximum_size > minimum_size.saturating_mul(3).max(1) {
        issues.push(issue(
            "unequal_cluster_sizes",
            format!("Cluster sizes range from {minimum_size} to {maximum_size}."),
            "Inspect weighting, informative cluster size, and whether a random slope or time structure is needed.",
            "review",
        ));
    }
    let include_values = opts
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ascii = format!(
        "Random-intercept model ({}, n={n}, clusters={cluster_count})\nrandom-intercept SD={}  residual SD={}  ICC={}\nlog likelihood={}{}\n\nThe fixed effects describe conditional mean differences. Cluster intercepts are partially pooled; no random slope, temporal correlation, or causal interpretation is added automatically.",
        method.to_uppercase(),
        fmt_number(random_intercept_variance.sqrt()),
        fmt_number(residual_variance.sqrt()),
        fmt_number(icc),
        fmt_number(profile.log_likelihood),
        if lambda == 0.0 { " (variance boundary)" } else { "" },
    );
    Ok(record([
        ("schema", text("biolang.stats.random-intercept/v1")),
        ("kind", text("random_intercept_model")),
        ("method", text(method)),
        ("complete_rows", Value::Int(n as i64)),
        (
            "excluded_rows",
            Value::Int((prepared.excluded_rows + excluded_cluster_rows) as i64),
        ),
        ("clusters", Value::Int(cluster_count as i64)),
        ("minimum_cluster_size", Value::Int(minimum_size as i64)),
        ("maximum_cluster_size", Value::Int(maximum_size as i64)),
        ("fixed_effects", list(fixed_effects)),
        ("encodings", list(prepared.encodings)),
        (
            "random_intercept_variance",
            Value::Float(random_intercept_variance),
        ),
        ("random_intercept_sd", Value::Float(random_intercept_variance.sqrt())),
        ("residual_variance", Value::Float(residual_variance)),
        ("residual_sd", Value::Float(residual_variance.sqrt())),
        ("intraclass_correlation", Value::Float(icc)),
        ("variance_ratio", Value::Float(lambda)),
        ("log_likelihood_profile", Value::Float(profile.log_likelihood)),
        ("random_effects", list(random_effects)),
        (
            "random_effects_truncated",
            Value::Bool(cluster_count > max_cluster_details),
        ),
        (
            "marginal_fitted",
            if include_values {
                list(marginal_fitted.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        (
            "conditional_fitted",
            if include_values {
                list(conditional_fitted.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        (
            "conditional_residuals",
            if include_values {
                list(conditional_residuals.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        (
            "original_rows",
            if include_values {
                list(original_rows.iter().map(|row| Value::Int(*row as i64)).collect())
            } else {
                Value::Nil
            },
        ),
        ("ascii", text(ascii)),
        ("issues", list(issues)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Only one random intercept is fitted; random slopes, crossed/nested effects, and residual correlation structures are not included.",
                "Fixed-effect p-values and intervals use a normal approximation; small-sample denominator degrees of freedom are not estimated.",
                "Variance-component boundary inference does not follow an ordinary chi-square reference distribution.",
                "Missing outcomes, predictors, or cluster labels are excluded together and disclosed.",
            ]),
        ),
    ]))
}

struct CoxEvaluation {
    log_likelihood: f64,
    score: Vec<f64>,
    information: Vec<Vec<f64>>,
}

fn evaluate_cox_breslow(
    time: &[f64],
    event: &[bool],
    x: &[Vec<f64>],
    beta: &[f64],
) -> Option<CoxEvaluation> {
    let p = beta.len();
    let linear = x.iter().map(|row| dot(row, beta)).collect::<Vec<_>>();
    let mut event_times = time
        .iter()
        .zip(event)
        .filter_map(|(time, event)| event.then_some(*time))
        .collect::<Vec<_>>();
    event_times.sort_by(f64::total_cmp);
    event_times.dedup_by(|left, right| (*left - *right).abs() <= 1e-12);
    let mut log_likelihood = 0.0;
    let mut score = vec![0.0; p];
    let mut information = vec![vec![0.0; p]; p];
    for event_time in event_times {
        let event_rows = (0..time.len())
            .filter(|index| event[*index] && (time[*index] - event_time).abs() <= 1e-12)
            .collect::<Vec<_>>();
        let event_count = event_rows.len();
        if event_count == 0 {
            continue;
        }
        let risk_rows = (0..time.len())
            .filter(|index| time[*index] >= event_time)
            .collect::<Vec<_>>();
        let maximum_linear = risk_rows
            .iter()
            .map(|index| linear[*index])
            .max_by(f64::total_cmp)?;
        let mut risk_sum = 0.0;
        let mut first_moment = vec![0.0; p];
        let mut second_moment = vec![vec![0.0; p]; p];
        for index in risk_rows {
            let weight = (linear[index] - maximum_linear).exp();
            risk_sum += weight;
            for left in 0..p {
                first_moment[left] += weight * x[index][left];
                for right in 0..p {
                    second_moment[left][right] += weight * x[index][left] * x[index][right];
                }
            }
        }
        if risk_sum <= 0.0 || !risk_sum.is_finite() {
            return None;
        }
        log_likelihood += event_rows.iter().map(|index| linear[*index]).sum::<f64>()
            - event_count as f64 * (maximum_linear + risk_sum.ln());
        for left in 0..p {
            let expected_left = first_moment[left] / risk_sum;
            score[left] += event_rows.iter().map(|index| x[*index][left]).sum::<f64>()
                - event_count as f64 * expected_left;
            for right in 0..p {
                information[left][right] += event_count as f64
                    * (second_moment[left][right] / risk_sum
                        - expected_left * first_moment[right] / risk_sum);
            }
        }
    }
    Some(CoxEvaluation {
        log_likelihood,
        score,
        information,
    })
}

fn cox_diagnostics(args: Vec<Value>) -> Result<Value> {
    let function = "stats_cox_diagnostics";
    let Value::List(times) = &args[0] else {
        return Err(BioLangError::type_error(
            format!("{function}() time must be List, got {}", args[0].type_of()),
            None,
        ));
    };
    let Value::List(events) = &args[1] else {
        return Err(BioLangError::type_error(
            format!("{function}() event must be List, got {}", args[1].type_of()),
            None,
        ));
    };
    let predictors = require_table(&args[2], function)?;
    if times.len() != events.len() || times.len() != predictors.rows.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() time, event, and predictor rows must have equal length"),
            None,
        ));
    }
    let opts = options(&args, 3, function)?;
    let time_value = Value::List(times.clone());
    let prepared = prepare_model(predictors, &time_value, &opts, function)?;
    let mut time = Vec::new();
    let mut event = Vec::new();
    let mut x = Vec::new();
    let mut original_rows = Vec::new();
    let mut excluded_event_rows = 0usize;
    for index in 0..prepared.y.len() {
        let original_row = prepared.original_rows[index];
        let event_value = match events.get(original_row) {
            Some(Value::Bool(value)) => Some(*value),
            Some(Value::Int(value)) if *value == 0 || *value == 1 => Some(*value == 1),
            Some(Value::Float(value)) if *value == 0.0 || *value == 1.0 => Some(*value == 1.0),
            Some(Value::Nil) | None => None,
            Some(other) => {
                return Err(BioLangError::type_error(
                    format!("{function}() event at row {original_row} must be Bool, 0, 1, or Nil; got {}", other.type_of()),
                    None,
                ));
            }
        };
        let Some(event_value) = event_value else {
            excluded_event_rows += 1;
            continue;
        };
        if prepared.y[index] <= 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() time at row {original_row} must be positive"),
                None,
            ));
        }
        time.push(prepared.y[index]);
        event.push(event_value);
        x.push(prepared.x[index].clone());
        original_rows.push(original_row);
    }
    let n = time.len();
    let p = prepared.feature_names.len();
    let event_count = event.iter().filter(|value| **value).count();
    if n <= p || event_count <= p {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() has {event_count} events and needs more events than the {p} encoded predictors"),
            None,
        ));
    }
    let means = (0..p)
        .map(|column| x.iter().map(|row| row[column]).sum::<f64>() / n as f64)
        .collect::<Vec<_>>();
    let centred_x = x
        .iter()
        .map(|row| {
            row.iter()
                .zip(&means)
                .map(|(value, mean)| value - mean)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut beta = vec![0.0; p];
    let mut converged = false;
    let mut iterations = 0usize;
    for iteration in 0..100 {
        iterations = iteration + 1;
        let current = evaluate_cox_breslow(&time, &event, &centred_x, &beta).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() partial likelihood became numerically unstable"),
                None,
            )
        })?;
        let (inverse, _) = invert_with_log_determinant(&current.information).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() information matrix is singular; inspect collinearity or sparse events"),
                None,
            )
        })?;
        let delta = inverse
            .iter()
            .map(|row| dot(row, &current.score))
            .collect::<Vec<_>>();
        let mut step = 1.0;
        let mut accepted = None;
        while step >= 1.0 / 1_048_576.0 {
            let candidate = beta
                .iter()
                .zip(&delta)
                .map(|(value, change)| value + step * change)
                .collect::<Vec<_>>();
            if let Some(evaluated) = evaluate_cox_breslow(&time, &event, &centred_x, &candidate) {
                if evaluated.log_likelihood >= current.log_likelihood - 1e-10 {
                    accepted = Some(candidate);
                    break;
                }
            }
            step *= 0.5;
        }
        let Some(candidate) = accepted else {
            break;
        };
        let maximum_change = beta
            .iter()
            .zip(&candidate)
            .map(|(left, right)| (left - right).abs())
            .fold(0.0, f64::max);
        beta = candidate;
        if maximum_change < 1e-9 {
            converged = true;
            break;
        }
    }
    let fitted_evaluation =
        evaluate_cox_breslow(&time, &event, &centred_x, &beta).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() could not evaluate the fitted partial likelihood"),
                None,
            )
        })?;
    let (covariance, _) =
        invert_with_log_determinant(&fitted_evaluation.information).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() fitted information matrix is singular"),
                None,
            )
        })?;
    let null_evaluation = evaluate_cox_breslow(&time, &event, &centred_x, &vec![0.0; p])
        .expect("a validated risk set is evaluable at zero coefficients");
    let likelihood_ratio =
        2.0 * (fitted_evaluation.log_likelihood - null_evaluation.log_likelihood);
    let likelihood_ratio_p =
        bl_core::bio_core::stats_ops::chi_square_sf(likelihood_ratio.max(0.0), p);
    let confidence = opts
        .get("confidence")
        .and_then(Value::as_float)
        .unwrap_or(0.95);
    if !(0.5..1.0).contains(&confidence) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() confidence must be between 0.5 and 1"),
            None,
        ));
    }
    let critical = bl_core::bio_core::stats_ops::normal_quantile((1.0 + confidence) / 2.0);
    let coefficient_rows = beta
        .iter()
        .enumerate()
        .map(|(index, estimate)| {
            let standard_error = covariance[index][index].max(0.0).sqrt();
            let z = if standard_error > 0.0 {
                estimate / standard_error
            } else {
                0.0
            };
            record([
                ("name", text(&prepared.feature_names[index])),
                ("estimate", Value::Float(*estimate)),
                ("standard_error", Value::Float(standard_error)),
                ("z_value", Value::Float(z)),
                (
                    "p_value",
                    Value::Float(2.0 * bl_core::bio_core::stats_ops::normal_sf(z.abs())),
                ),
                ("hazard_ratio", Value::Float(estimate.exp())),
                (
                    "hazard_ratio_lower",
                    Value::Float((estimate - critical * standard_error).exp()),
                ),
                (
                    "hazard_ratio_upper",
                    Value::Float((estimate + critical * standard_error).exp()),
                ),
            ])
        })
        .collect::<Vec<_>>();

    let linear_predictors = x.iter().map(|row| dot(row, &beta)).collect::<Vec<_>>();
    let mut event_times = time
        .iter()
        .zip(&event)
        .filter_map(|(time, event)| event.then_some(*time))
        .collect::<Vec<_>>();
    event_times.sort_by(f64::total_cmp);
    event_times.dedup_by(|left, right| (*left - *right).abs() <= 1e-12);
    let max_baseline_rows = opts
        .get("max_baseline_rows")
        .and_then(Value::as_int)
        .unwrap_or(500)
        .clamp(0, 10_000) as usize;
    let mut baseline_rows = Vec::new();
    let mut cumulative_hazard = 0.0;
    let mut cumulative_hazard_at_row = vec![0.0; n];
    let mut schoenfeld_times = Vec::new();
    let mut schoenfeld_by_feature = vec![Vec::<f64>::new(); p];
    for event_time in &event_times {
        let event_indices = (0..n)
            .filter(|index| event[*index] && (time[*index] - event_time).abs() <= 1e-12)
            .collect::<Vec<_>>();
        let risk_indices = (0..n)
            .filter(|index| time[*index] >= *event_time)
            .collect::<Vec<_>>();
        let maximum_linear = risk_indices
            .iter()
            .map(|index| linear_predictors[*index])
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        let scaled_weights = risk_indices
            .iter()
            .map(|index| (linear_predictors[*index] - maximum_linear).exp())
            .collect::<Vec<_>>();
        let scaled_sum = scaled_weights.iter().sum::<f64>();
        let log_risk_sum = maximum_linear + scaled_sum.ln();
        let increment = event_indices.len() as f64 * (-log_risk_sum).exp();
        cumulative_hazard += increment;
        if baseline_rows.len() < max_baseline_rows {
            baseline_rows.push(record([
                ("time", Value::Float(*event_time)),
                ("events", Value::Int(event_indices.len() as i64)),
                ("hazard_increment", Value::Float(increment)),
                ("cumulative_hazard", Value::Float(cumulative_hazard)),
                (
                    "baseline_survival",
                    Value::Float((-cumulative_hazard).exp()),
                ),
            ]));
        }
        let mut expected = vec![0.0; p];
        for (risk_position, row) in risk_indices.iter().enumerate() {
            for feature in 0..p {
                expected[feature] += scaled_weights[risk_position] * x[*row][feature] / scaled_sum;
            }
        }
        for row in event_indices {
            schoenfeld_times.push(*event_time);
            for feature in 0..p {
                schoenfeld_by_feature[feature].push(x[row][feature] - expected[feature]);
            }
        }
    }
    // Evaluate the fitted cumulative baseline hazard at every observed time.
    cumulative_hazard = 0.0;
    for event_time in &event_times {
        let deaths = (0..n)
            .filter(|index| event[*index] && (time[*index] - event_time).abs() <= 1e-12)
            .count();
        let risk_log_values = (0..n)
            .filter(|index| time[*index] >= *event_time)
            .map(|index| linear_predictors[index])
            .collect::<Vec<_>>();
        let maximum = risk_log_values
            .iter()
            .copied()
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        let log_sum = maximum
            + risk_log_values
                .iter()
                .map(|value| (value - maximum).exp())
                .sum::<f64>()
                .ln();
        cumulative_hazard += deaths as f64 * (-log_sum).exp();
        for row in 0..n {
            if time[row] >= *event_time {
                cumulative_hazard_at_row[row] = cumulative_hazard;
            }
        }
    }
    let martingale_residuals = (0..n)
        .map(|index| {
            (if event[index] { 1.0 } else { 0.0 })
                - cumulative_hazard_at_row[index] * linear_predictors[index].exp()
        })
        .collect::<Vec<_>>();
    let deviance_residuals = martingale_residuals
        .iter()
        .zip(&event)
        .map(|(martingale, event)| {
            let event_number = if *event { 1.0 } else { 0.0 };
            let inside = if *event {
                -2.0 * (martingale + (event_number - martingale).max(1e-15).ln())
            } else {
                -2.0 * martingale
            };
            martingale.signum() * inside.max(0.0).sqrt()
        })
        .collect::<Vec<_>>();
    let mut ph_rows = Vec::new();
    let mut ph_global_chi_squared = 0.0;
    let mut ph_issue = false;
    for feature in 0..p {
        let correlation = pearson(&schoenfeld_times, &schoenfeld_by_feature[feature])
            .map(|value| value.0)
            .unwrap_or(0.0);
        let approximate_z = correlation * (event_count as f64).sqrt();
        let approximate_p = 2.0 * bl_core::bio_core::stats_ops::normal_sf(approximate_z.abs());
        ph_global_chi_squared += approximate_z.powi(2);
        ph_issue |= correlation.abs() >= 0.2;
        ph_rows.push(record([
            ("name", text(&prepared.feature_names[feature])),
            (
                "schoenfeld_event_time_correlation",
                Value::Float(correlation),
            ),
            ("approximate_z", Value::Float(approximate_z)),
            ("approximate_p_value", Value::Float(approximate_p)),
            ("formal_cox_zph_test", Value::Bool(false)),
        ]));
    }
    let ph_global_p = bl_core::bio_core::stats_ops::chi_square_sf(ph_global_chi_squared, p);
    let mut comparable_pairs = 0usize;
    let mut concordance_credit = 0.0;
    for left in 0..n {
        if !event[left] {
            continue;
        }
        for right in 0..n {
            if time[right] > time[left] {
                comparable_pairs += 1;
                if linear_predictors[left] > linear_predictors[right] + 1e-12 {
                    concordance_credit += 1.0;
                } else if (linear_predictors[left] - linear_predictors[right]).abs() <= 1e-12 {
                    concordance_credit += 0.5;
                }
            }
        }
    }
    let concordance = if comparable_pairs > 0 {
        concordance_credit / comparable_pairs as f64
    } else {
        0.5
    };
    let deviance_review_rows = deviance_residuals
        .iter()
        .enumerate()
        .filter(|(_, residual)| residual.abs() >= 3.0)
        .map(|(index, residual)| {
            record([
                ("row", Value::Int(original_rows[index] as i64)),
                ("time", Value::Float(time[index])),
                ("event", Value::Bool(event[index])),
                ("deviance_residual", Value::Float(*residual)),
                ("action", text("inspect; do not automatically delete")),
            ])
        })
        .collect::<Vec<_>>();
    let tied_event_times = event_times
        .iter()
        .filter(|event_time| {
            (0..n)
                .filter(|index| event[*index] && (time[*index] - **event_time).abs() <= 1e-12)
                .count()
                > 1
        })
        .count();
    let mut issues = Vec::new();
    if !converged {
        issues.push(issue(
            "convergence_review",
            format!("The coefficient update did not meet tolerance within {iterations} iterations."),
            "Inspect scaling, collinearity, separation, sparse events, and coefficient stability before interpretation.",
            "review",
        ));
    }
    if ph_issue {
        issues.push(issue(
            "proportional_hazards_clue",
            "At least one raw Schoenfeld residual correlation with event time has magnitude at least 0.2.",
            "Inspect time-varying effects and plots, then confirm with a formal proportional-hazards diagnostic such as cox.zph.",
            "review",
        ));
    }
    if !deviance_review_rows.is_empty() {
        issues.push(issue(
            "deviance_residual_review",
            format!(
                "{} observation(s) have absolute deviance residual at least 3.",
                deviance_review_rows.len()
            ),
            "Inspect data provenance and model sensitivity; this threshold is not a deletion rule.",
            "review",
        ));
    }
    let include_values = opts
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let ascii = format!(
        "Cox proportional-hazards diagnostic (n={n}, events={event_count}, predictors={p})\nBreslow ties: {tied_event_times} tied event time(s)\npartial log likelihood={}  LR chi-square={} p={}\nconcordance={} from {comparable_pairs} comparable pairs\nSchoenfeld screen chi-square={} p={} (descriptive approximation; not cox.zph)\n\nHazard ratios are conditional event-rate ratios under proportional hazards. They are not individual risk reductions or survival-time ratios.",
        fmt_number(fitted_evaluation.log_likelihood),
        fmt_number(likelihood_ratio),
        fmt_number(likelihood_ratio_p),
        fmt_number(concordance),
        fmt_number(ph_global_chi_squared),
        fmt_number(ph_global_p),
    );
    Ok(record([
        ("schema", text("biolang.stats.cox-diagnostics/v1")),
        ("kind", text("cox_diagnostics")),
        ("ties", text("breslow")),
        ("complete_rows", Value::Int(n as i64)),
        (
            "excluded_rows",
            Value::Int((prepared.excluded_rows + excluded_event_rows) as i64),
        ),
        ("events", Value::Int(event_count as i64)),
        ("censored", Value::Int((n - event_count) as i64)),
        ("tied_event_times", Value::Int(tied_event_times as i64)),
        ("encoded_predictors", Value::Int(p as i64)),
        ("coefficients", list(coefficient_rows)),
        ("encodings", list(prepared.encodings)),
        ("converged", Value::Bool(converged)),
        ("iterations", Value::Int(iterations as i64)),
        (
            "partial_log_likelihood",
            Value::Float(fitted_evaluation.log_likelihood),
        ),
        ("aic_partial", Value::Float(-2.0 * fitted_evaluation.log_likelihood + 2.0 * p as f64)),
        ("likelihood_ratio", Value::Float(likelihood_ratio)),
        ("likelihood_ratio_df", Value::Int(p as i64)),
        ("likelihood_ratio_p_value", Value::Float(likelihood_ratio_p)),
        ("concordance", Value::Float(concordance)),
        ("comparable_pairs", Value::Int(comparable_pairs as i64)),
        ("baseline_hazard", list(baseline_rows)),
        (
            "baseline_hazard_truncated",
            Value::Bool(event_times.len() > max_baseline_rows),
        ),
        ("proportional_hazards_screen", list(ph_rows)),
        (
            "ph_screen_global_chi_squared",
            Value::Float(ph_global_chi_squared),
        ),
        ("ph_screen_global_p_value", Value::Float(ph_global_p)),
        ("formal_cox_zph_test", Value::Bool(false)),
        ("deviance_review_rows", list(deviance_review_rows)),
        (
            "martingale_residuals",
            if include_values {
                list(martingale_residuals.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        (
            "deviance_residuals",
            if include_values {
                list(deviance_residuals.iter().copied().map(Value::Float).collect())
            } else {
                Value::Nil
            },
        ),
        ("ascii", text(ascii)),
        ("issues", list(issues)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "The fit uses the Breslow approximation for tied event times; Efron and exact partial likelihood are not implemented here.",
                "The Schoenfeld screen uses raw residual correlation with event time and is explicitly not a formal cox.zph test.",
                "Competing risks, recurrent events, left truncation, interval censoring, frailty, strata, and time-varying covariates require dedicated models.",
                "Residual thresholds are inspection clues and never automatic deletion rules.",
            ]),
        ),
    ]))
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left, right)| left * right)
        .sum()
}

fn multiple_linear_diagnostics(args: Vec<Value>) -> Result<Value> {
    let predictors = require_table(&args[0], "stats_multiple_linear_diagnostics")?;
    let opts = options(&args, 2, "stats_multiple_linear_diagnostics")?;
    let prepared = prepare_model(
        predictors,
        &args[1],
        &opts,
        "stats_multiple_linear_diagnostics",
    )?;
    let n = prepared.y.len();
    let p = prepared.feature_names.len();
    if n <= p + 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("stats_multiple_linear_diagnostics() has {n} complete rows but needs more than {} for {p} encoded predictors plus an intercept", p + 1),
            None,
        ));
    }
    let fit = bl_core::bio_core::stats_ops::multiple_linear_regression(&prepared.y, &prepared.x)
        .map_err(|message| BioLangError::runtime(ErrorKind::TypeError, message, None))?;
    let inverse = inverse_xtx(&prepared.x).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_multiple_linear_diagnostics() model matrix is singular or numerically unstable",
            None,
        )
    })?;
    let coefficients = fit.coefficients;
    let design_rows = prepared
        .x
        .iter()
        .map(|row| {
            std::iter::once(1.0)
                .chain(row.iter().copied())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let fitted = design_rows
        .iter()
        .map(|row| dot(row, &coefficients))
        .collect::<Vec<_>>();
    let residuals = prepared
        .y
        .iter()
        .zip(&fitted)
        .map(|(observed, fitted)| observed - fitted)
        .collect::<Vec<_>>();
    let residual_ss = residuals.iter().map(|value| value.powi(2)).sum::<f64>();
    let degrees_freedom = n - p - 1;
    let mse = residual_ss / degrees_freedom as f64;
    let confidence = opts
        .get("confidence")
        .and_then(Value::as_float)
        .unwrap_or(0.95);
    if !(0.5..1.0).contains(&confidence) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_multiple_linear_diagnostics() confidence must be between 0.5 and 1",
            None,
        ));
    }
    let critical = bl_core::bio_core::stats_ops::normal_quantile((1.0 + confidence) / 2.0);
    let mut coefficient_rows = Vec::new();
    let mut max_vif = 0.0_f64;
    for index in 0..=p {
        let standard_error = (mse * inverse[index][index].max(0.0)).sqrt();
        let vif = if index == 0 {
            None
        } else {
            let mean = prepared.x.iter().map(|row| row[index - 1]).sum::<f64>() / n as f64;
            let centred_ss = prepared
                .x
                .iter()
                .map(|row| (row[index - 1] - mean).powi(2))
                .sum::<f64>();
            Some((inverse[index][index] * centred_ss).max(0.0))
        };
        if let Some(value) = vif {
            max_vif = max_vif.max(value);
        }
        coefficient_rows.push(record([
            (
                "name",
                text(if index == 0 {
                    "(intercept)"
                } else {
                    &prepared.feature_names[index - 1]
                }),
            ),
            ("estimate", Value::Float(coefficients[index])),
            ("standard_error", Value::Float(standard_error)),
            ("p_value", Value::Float(fit.p_values[index])),
            (
                "confidence_lower",
                Value::Float(coefficients[index] - critical * standard_error),
            ),
            (
                "confidence_upper",
                Value::Float(coefficients[index] + critical * standard_error),
            ),
            ("vif", number(vif)),
        ]));
    }
    let mut leverages = Vec::with_capacity(n);
    let mut cooks = Vec::with_capacity(n);
    let mut standardized_residuals = Vec::with_capacity(n);
    for (row, residual) in design_rows.iter().zip(&residuals) {
        let transformed = inverse
            .iter()
            .map(|inverse_row| dot(inverse_row, row))
            .collect::<Vec<_>>();
        let leverage = dot(row, &transformed).clamp(0.0, 1.0);
        let standard = if mse > f64::EPSILON && leverage < 1.0 {
            residual / (mse * (1.0 - leverage)).sqrt()
        } else {
            0.0
        };
        let cook = if mse > f64::EPSILON && leverage < 1.0 {
            residual.powi(2) / ((p + 1) as f64 * mse) * leverage / (1.0 - leverage).powi(2)
        } else {
            0.0
        };
        leverages.push(leverage);
        standardized_residuals.push(standard);
        cooks.push(cook);
    }
    let cook_threshold = 4.0 / n as f64;
    let cook_flags = cooks
        .iter()
        .filter(|value| **value > cook_threshold)
        .count();
    let maximum_cook_distance = cooks.iter().copied().max_by(f64::total_cmp).unwrap_or(0.0);
    let leverage_threshold = 2.0 * (p + 1) as f64 / n as f64;
    let leverage_flags = leverages
        .iter()
        .filter(|value| **value > leverage_threshold)
        .count();
    let standardized_flags = standardized_residuals
        .iter()
        .filter(|value| value.abs() >= 3.0)
        .count();
    let residual_data = NumericData {
        values: residuals.clone(),
        original_indices: (0..n).collect(),
        total: n,
        missing: 0,
        non_finite: 0,
    };
    let (qq_expected, qq_observed) = normal_qq_values(&residual_data);
    let qq_correlation = pearson(&qq_expected, &qq_observed).map(|value| value.0);
    let absolute_residuals = residuals
        .iter()
        .map(|value| value.abs())
        .collect::<Vec<_>>();
    let scale_association = pearson(&fitted, &absolute_residuals).map(|value| value.0);
    let durbin_watson = (residual_ss > f64::EPSILON).then(|| {
        residuals
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).powi(2))
            .sum::<f64>()
            / residual_ss
    });

    let requested_folds = opts
        .get("validation_folds")
        .and_then(Value::as_int)
        .unwrap_or(5)
        .clamp(0, 20) as usize;
    let folds = requested_folds.min(n);
    let seed = opts
        .get("seed")
        .and_then(Value::as_int)
        .unwrap_or(42)
        .unsigned_abs() as usize;
    let validation_group_column = opts.get("validation_group_column").and_then(Value::as_str);
    let mut validation_group_ids = Vec::new();
    let mut validation_group_count = 0usize;
    if let Some(column_name) = validation_group_column {
        let column = predictors
            .columns
            .iter()
            .position(|name| name == column_name)
            .expect("validation group column checked while preparing model");
        let mut ids = HashMap::<String, usize>::new();
        for original_row in &prepared.original_rows {
            let Some(label) = predictors.rows[*original_row]
                .get(column)
                .and_then(category_label)
            else {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("stats_multiple_linear_diagnostics() validation group column '{column_name}' is missing at complete model row {original_row}"),
                    None,
                ));
            };
            let next = ids.len();
            let id = *ids.entry(label).or_insert(next);
            validation_group_ids.push(id);
        }
        validation_group_count = ids.len();
    }
    let folds = if validation_group_column.is_some() {
        requested_folds.min(validation_group_count)
    } else {
        requested_folds.min(n)
    };
    let mut validation_errors = Vec::new();
    if folds >= 2 {
        for fold in 0..folds {
            let mut train_x = Vec::new();
            let mut train_y = Vec::new();
            let mut test_rows = Vec::new();
            for row in 0..n {
                let allocation = if validation_group_column.is_some() {
                    (validation_group_ids[row] + seed) % folds
                } else {
                    (row + seed) % folds
                };
                if allocation == fold {
                    test_rows.push(row);
                } else {
                    train_x.push(prepared.x[row].clone());
                    train_y.push(prepared.y[row]);
                }
            }
            if train_y.len() <= p + 1 || test_rows.is_empty() {
                validation_errors.clear();
                break;
            }
            let Ok(fold_fit) =
                bl_core::bio_core::stats_ops::multiple_linear_regression(&train_y, &train_x)
            else {
                validation_errors.clear();
                break;
            };
            for row in test_rows {
                let design = std::iter::once(1.0)
                    .chain(prepared.x[row].iter().copied())
                    .collect::<Vec<_>>();
                validation_errors.push(prepared.y[row] - dot(&design, &fold_fit.coefficients));
            }
        }
    }
    let validation_rmse = (!validation_errors.is_empty()).then(|| {
        (validation_errors
            .iter()
            .map(|value| value.powi(2))
            .sum::<f64>()
            / validation_errors.len() as f64)
            .sqrt()
    });
    let validation_mae = (!validation_errors.is_empty()).then(|| {
        validation_errors
            .iter()
            .map(|value| value.abs())
            .sum::<f64>()
            / validation_errors.len() as f64
    });
    let include_values = opts
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut fitted_intervals = Vec::new();
    if include_values {
        for index in 0..n {
            let leverage = leverages[index];
            let mean_margin = critical * (mse * leverage).sqrt();
            let prediction_margin = critical * (mse * (1.0 + leverage)).sqrt();
            fitted_intervals.push(record([
                ("row", Value::Int(prepared.original_rows[index] as i64)),
                ("observed", Value::Float(prepared.y[index])),
                ("fitted", Value::Float(fitted[index])),
                ("residual", Value::Float(residuals[index])),
                ("mean_lower", Value::Float(fitted[index] - mean_margin)),
                ("mean_upper", Value::Float(fitted[index] + mean_margin)),
                (
                    "prediction_lower",
                    Value::Float(fitted[index] - prediction_margin),
                ),
                (
                    "prediction_upper",
                    Value::Float(fitted[index] + prediction_margin),
                ),
                ("leverage", Value::Float(leverage)),
                ("cook_distance", Value::Float(cooks[index])),
            ]));
        }
    }
    let mut issues = Vec::new();
    if max_vif >= 5.0 {
        issues.push(issue(
            "multicollinearity_clue",
            format!("Maximum encoded-feature VIF is {}.", fmt_number(max_vif)),
            "Coefficient estimates can be unstable when predictors contain overlapping information; inspect design, coding, and scientific redundancy.",
            "review",
        ));
    }
    if cook_flags > 0 || leverage_flags > 0 {
        issues.push(issue(
            "influence_clues",
            format!("{cook_flags} Cook-distance and {leverage_flags} leverage review flag(s) were observed."),
            "Inspect provenance and sensitivity; influential observations must not be deleted automatically.",
            "review",
        ));
    }
    if scale_association.is_some_and(|value| value.abs() >= 0.3) {
        issues.push(issue(
            "changing_residual_spread",
            format!("Correlation between fitted values and |residual| is {}.", fmt_number(scale_association.unwrap())),
            "Changing spread may require a different variance model, scale, or robust uncertainty calculation.",
            "review",
        ));
    }
    Ok(record([
        ("schema", text("biolang.stats.multiple-linear-diagnostics/v1")),
        ("kind", text("multiple_linear_diagnostics")),
        ("complete_rows", Value::Int(n as i64)),
        ("excluded_rows", Value::Int(prepared.excluded_rows as i64)),
        ("encoded_predictors", Value::Int(p as i64)),
        ("degrees_freedom", Value::Int(degrees_freedom as i64)),
        ("encodings", list(prepared.encodings)),
        ("coefficients", list(coefficient_rows)),
        ("r_squared", Value::Float(fit.r_squared)),
        ("adjusted_r_squared", Value::Float(fit.adj_r_squared)),
        ("residual_mse", Value::Float(mse)),
        ("normal_qq_correlation", number(qq_correlation)),
        ("fitted_absolute_residual_correlation", number(scale_association)),
        ("durbin_watson_in_observation_order", number(durbin_watson)),
        ("maximum_vif", Value::Float(max_vif)),
        ("cook_threshold", Value::Float(cook_threshold)),
        (
            "maximum_cook_distance",
            Value::Float(maximum_cook_distance),
        ),
        ("cook_review_flags", Value::Int(cook_flags as i64)),
        ("leverage_threshold", Value::Float(leverage_threshold)),
        ("leverage_review_flags", Value::Int(leverage_flags as i64)),
        ("standardized_residual_flags", Value::Int(standardized_flags as i64)),
        ("validation_folds", Value::Int(if validation_errors.is_empty() { 0 } else { folds as i64 })),
        (
            "validation_method",
            text(if validation_group_column.is_some() {
                "group-held-out deterministic folds"
            } else {
                "row-held-out deterministic folds"
            }),
        ),
        (
            "validation_group_column",
            validation_group_column.map(text).unwrap_or(Value::Nil),
        ),
        (
            "validation_groups",
            Value::Int(validation_group_count as i64),
        ),
        ("validation_rmse", number(validation_rmse)),
        ("validation_mae", number(validation_mae)),
        ("confidence", Value::Float(confidence)),
        ("interval_method", text("large-sample normal approximation")),
        ("fitted_intervals", list(fitted_intervals)),
        ("issues", list(issues)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Categorical predictors use first-observed treatment contrasts; inspect the returned encoding before interpretation.",
                "Confidence and prediction intervals use a large-sample normal critical value rather than a finite-sample t critical value.",
                "Deterministic folds estimate predictive error but do not replace external validation; set validation_group_column for subjects, sites, batches, or other non-independent rows.",
                "Residual checks cannot establish causal interpretation, correct functional form, independence, or absence of measurement error.",
            ]),
        ),
        (
            "quick_explanation",
            text("A multivariable linear model was encoded and checked for residual, influence, collinearity, interval, and held-out prediction clues."),
        ),
    ]))
}

fn weighted_linear_coefficients(x: &[Vec<f64>], y: &[f64], weights: &[f64]) -> Option<Vec<f64>> {
    let p = x.first()?.len() + 1;
    let mut system = vec![vec![0.0; p + 1]; p];
    for ((row, outcome), weight) in x.iter().zip(y).zip(weights) {
        let design = std::iter::once(1.0)
            .chain(row.iter().copied())
            .collect::<Vec<_>>();
        for left in 0..p {
            system[left][p] += weight * design[left] * outcome;
            for right in 0..p {
                system[left][right] += weight * design[left] * design[right];
            }
        }
    }
    for column in 0..p {
        let pivot = (column..p).max_by(|left, right| {
            system[*left][column]
                .abs()
                .total_cmp(&system[*right][column].abs())
        })?;
        system.swap(column, pivot);
        let divisor = system[column][column];
        if divisor.abs() <= 1e-12 {
            return None;
        }
        for index in column..=p {
            system[column][index] /= divisor;
        }
        for row in 0..p {
            if row == column {
                continue;
            }
            let factor = system[row][column];
            for index in column..=p {
                system[row][index] -= factor * system[column][index];
            }
        }
    }
    Some(system.into_iter().map(|row| row[p]).collect())
}

fn robust_linear_diagnostics(args: Vec<Value>) -> Result<Value> {
    let function = "stats_robust_linear_diagnostics";
    let predictors = require_table(&args[0], function)?;
    let opts = options(&args, 2, function)?;
    let prepared = prepare_model(predictors, &args[1], &opts, function)?;
    let n = prepared.y.len();
    let p = prepared.feature_names.len();
    if n <= p + 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{function}() has {n} complete rows but needs more than {}",
                p + 1
            ),
            None,
        ));
    }
    let ols = bl_core::bio_core::stats_ops::multiple_linear_regression(&prepared.y, &prepared.x)
        .map_err(|message| BioLangError::runtime(ErrorKind::TypeError, message, None))?;
    let tuning = opts
        .get("huber_k")
        .and_then(Value::as_float)
        .unwrap_or(1.345);
    if !tuning.is_finite() || tuning <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() huber_k must be positive and finite"),
            None,
        ));
    }
    let max_iterations = opts
        .get("max_iterations")
        .and_then(Value::as_int)
        .unwrap_or(100)
        .clamp(1, 1000) as usize;
    let tolerance = opts
        .get("tolerance")
        .and_then(Value::as_float)
        .unwrap_or(1e-8);
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() tolerance must be positive and finite"),
            None,
        ));
    }
    let mut coefficients = ols.coefficients.clone();
    let mut weights = vec![1.0; n];
    let mut scale = 0.0;
    let mut converged = false;
    let mut iterations = 0usize;
    for iteration in 1..=max_iterations {
        iterations = iteration;
        let residuals = prepared
            .x
            .iter()
            .zip(&prepared.y)
            .map(|(row, outcome)| {
                let fitted = coefficients[0]
                    + row
                        .iter()
                        .zip(coefficients.iter().skip(1))
                        .map(|(value, coefficient)| value * coefficient)
                        .sum::<f64>();
                outcome - fitted
            })
            .collect::<Vec<_>>();
        let residual_center = median(&residuals);
        let deviations = residuals
            .iter()
            .map(|residual| (residual - residual_center).abs())
            .collect::<Vec<_>>();
        scale = median(&deviations) / 0.674_489_750_196_081_7;
        if scale <= f64::EPSILON {
            scale = (residuals.iter().map(|value| value * value).sum::<f64>() / n as f64).sqrt();
        }
        if scale <= f64::EPSILON {
            converged = true;
            break;
        }
        for (weight, residual) in weights.iter_mut().zip(&residuals) {
            let standardized = residual.abs() / scale;
            *weight = if standardized <= tuning {
                1.0
            } else {
                tuning / standardized
            };
        }
        let Some(updated) = weighted_linear_coefficients(&prepared.x, &prepared.y, &weights) else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() weighted model matrix became singular"),
                None,
            ));
        };
        let change = updated
            .iter()
            .zip(&coefficients)
            .map(|(new, old)| (new - old).abs() / (1.0 + old.abs()))
            .fold(0.0_f64, f64::max);
        coefficients = updated;
        if change <= tolerance {
            converged = true;
            break;
        }
    }
    let names = std::iter::once("(intercept)".to_string())
        .chain(prepared.feature_names.iter().cloned())
        .collect::<Vec<_>>();
    let coefficient_rows = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            let absolute_change = coefficients[index] - ols.coefficients[index];
            record([
                ("name", text(name)),
                ("ols_estimate", Value::Float(ols.coefficients[index])),
                ("huber_estimate", Value::Float(coefficients[index])),
                ("absolute_change", Value::Float(absolute_change)),
                (
                    "relative_change",
                    Value::Float(absolute_change.abs() / (ols.coefficients[index].abs() + 1e-12)),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let downweighted = weights.iter().filter(|weight| **weight < 0.999_999).count();
    let minimum_weight = weights
        .iter()
        .copied()
        .min_by(f64::total_cmp)
        .unwrap_or(1.0);
    let include_values = opts
        .get("include_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(record([
        ("schema", text("biolang.stats.robust-linear-diagnostics/v1")),
        ("kind", text("robust_linear_diagnostics")),
        ("complete_rows", Value::Int(n as i64)),
        ("excluded_rows", Value::Int(prepared.excluded_rows as i64)),
        ("encodings", list(prepared.encodings)),
        ("method", text("Huber M-estimation by iteratively reweighted least squares")),
        ("huber_k", Value::Float(tuning)),
        ("scale", Value::Float(scale)),
        ("iterations", Value::Int(iterations as i64)),
        ("converged", Value::Bool(converged)),
        ("coefficients", list(coefficient_rows)),
        ("downweighted_rows", Value::Int(downweighted as i64)),
        ("minimum_weight", Value::Float(minimum_weight)),
        (
            "weights",
            if include_values {
                Value::List(weights.into_iter().map(Value::Float).collect::<Vec<_>>().into())
            } else {
                Value::List(Vec::<Value>::new().into())
            },
        ),
        ("formal_inference_provided", Value::Bool(false)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Huber estimates are a sensitivity analysis, not permission to delete downweighted observations.",
                "No p-values or confidence intervals are reported because valid robust inference depends on the sampling and dependence design.",
                "Robust fitting does not repair confounding, dependence, an incorrect functional form, or measurement error.",
            ]),
        ),
        (
            "quick_explanation",
            text("Compare the Huber and OLS coefficients. Large changes identify conclusions that depend strongly on large residuals."),
        ),
    ]))
}

fn weighted_summary(args: Vec<Value>) -> Result<Value> {
    let function = "stats_weighted_summary";
    let opts = options(&args, 2, function)?;
    let (Value::List(values), Value::List(weights)) = (&args[0], &args[1]) else {
        return Err(BioLangError::type_error(
            format!("{function}() values and weights must both be List"),
            None,
        ));
    };
    if values.len() != weights.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() values and weights must have equal length"),
            None,
        ));
    }
    let mut pairs = Vec::<(f64, f64)>::new();
    let mut excluded = 0usize;
    let mut zero_weights = 0usize;
    for (index, (value, weight)) in values.iter().zip(weights.iter()).enumerate() {
        if matches!(value, Value::Nil) || matches!(weight, Value::Nil) {
            excluded += 1;
            continue;
        }
        let Some(value) = finite_number(value) else {
            return Err(BioLangError::type_error(
                format!("{function}() value at index {index} must be finite numeric or Nil"),
                None,
            ));
        };
        let Some(weight) = finite_number(weight) else {
            return Err(BioLangError::type_error(
                format!("{function}() weight at index {index} must be finite numeric or Nil"),
                None,
            ));
        };
        if weight < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("{function}() weight at index {index} is negative"),
                None,
            ));
        }
        if weight == 0.0 {
            zero_weights += 1;
        }
        pairs.push((value, weight));
    }
    let sum_weights = pairs.iter().map(|pair| pair.1).sum::<f64>();
    let sum_squared_weights = pairs.iter().map(|pair| pair.1 * pair.1).sum::<f64>();
    if sum_weights <= f64::EPSILON || sum_squared_weights <= f64::EPSILON {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() requires at least one positive weight"),
            None,
        ));
    }
    let weighted_mean = pairs.iter().map(|pair| pair.0 * pair.1).sum::<f64>() / sum_weights;
    let variance_denominator = sum_weights - sum_squared_weights / sum_weights;
    let weighted_variance = (variance_denominator > f64::EPSILON).then(|| {
        pairs
            .iter()
            .map(|pair| pair.1 * (pair.0 - weighted_mean).powi(2))
            .sum::<f64>()
            / variance_denominator
    });
    let effective_n = sum_weights * sum_weights / sum_squared_weights;
    let positive_count = pairs.iter().filter(|pair| pair.1 > 0.0).count();
    let design_effect = positive_count as f64 / effective_n;
    let unweighted_mean = pairs.iter().map(|pair| pair.0).sum::<f64>() / pairs.len() as f64;
    pairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    let weighted_quantile = |probability: f64| {
        let target = probability * sum_weights;
        let mut cumulative = 0.0;
        for (value, weight) in &pairs {
            cumulative += weight;
            if cumulative >= target {
                return *value;
            }
        }
        pairs.last().map(|pair| pair.0).unwrap_or(weighted_mean)
    };
    let maximum_weight_share = pairs
        .iter()
        .map(|pair| pair.1 / sum_weights)
        .max_by(f64::total_cmp)
        .unwrap_or(0.0);
    let weight_kind = opts
        .get("weight_kind")
        .and_then(Value::as_str)
        .unwrap_or("unspecified");
    if !matches!(
        weight_kind,
        "unspecified" | "frequency" | "probability" | "analytic"
    ) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{function}() weight_kind must be unspecified, frequency, probability, or analytic"
            ),
            None,
        ));
    }
    Ok(record([
        ("schema", text("biolang.stats.weighted-summary/v1")),
        ("kind", text("weighted_summary")),
        ("weight_kind", text(weight_kind)),
        ("complete_pairs", Value::Int(pairs.len() as i64)),
        ("excluded_pairs", Value::Int(excluded as i64)),
        ("zero_weights", Value::Int(zero_weights as i64)),
        ("sum_weights", Value::Float(sum_weights)),
        ("effective_sample_size", Value::Float(effective_n)),
        ("unequal_weight_design_effect", Value::Float(design_effect)),
        ("maximum_weight_share", Value::Float(maximum_weight_share)),
        ("unweighted_mean", Value::Float(unweighted_mean)),
        ("weighted_mean", Value::Float(weighted_mean)),
        ("weighted_mean_shift", Value::Float(weighted_mean - unweighted_mean)),
        ("weighted_variance", number(weighted_variance)),
        ("weighted_sd", number(weighted_variance.map(f64::sqrt))),
        ("weighted_q1", Value::Float(weighted_quantile(0.25))),
        ("weighted_median", Value::Float(weighted_quantile(0.5))),
        ("weighted_q3", Value::Float(weighted_quantile(0.75))),
        ("formal_survey_inference_provided", Value::Bool(false)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Weighted quantiles use the first empirical value whose cumulative weight reaches the requested probability; software conventions differ.",
                "The variance describes weighted spread. It is not a design-based standard error for a complex survey.",
                "Strata, clusters, finite-population corrections, calibration, and replicate weights require a survey-design estimator.",
            ]),
        ),
        (
            "quick_explanation",
            text("Compare weighted and unweighted centres, then inspect effective sample size and weight concentration before relying on weighted results."),
        ),
    ]))
}

fn means_guide(args: Vec<Value>) -> Result<Value> {
    let function = "stats_means";
    let data = numeric_data(&args[0], function)?;
    let opts = options(&args, 1, function)?;
    let summary = summarize(&data);
    let trim_fraction = opts
        .get("trim_fraction")
        .and_then(Value::as_float)
        .unwrap_or(0.1);
    if !trim_fraction.is_finite() || !(0.0..0.5).contains(&trim_fraction) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() trim_fraction must be at least 0 and below 0.5"),
            None,
        ));
    }
    let mut sorted = data.values.clone();
    sorted.sort_by(f64::total_cmp);
    let trim_each_side = ((sorted.len() as f64 * trim_fraction).floor() as usize)
        .min(sorted.len().saturating_sub(1) / 2);
    let retained = &sorted[trim_each_side..sorted.len() - trim_each_side];
    let trimmed_mean = retained.iter().sum::<f64>() / retained.len() as f64;
    let mut winsorized = sorted.clone();
    if trim_each_side > 0 {
        let lower = sorted[trim_each_side];
        let upper = sorted[sorted.len() - trim_each_side - 1];
        for value in &mut winsorized[..trim_each_side] {
            *value = lower;
        }
        let upper_start = winsorized.len() - trim_each_side;
        for value in &mut winsorized[upper_start..] {
            *value = upper;
        }
    }
    let winsorized_mean = winsorized.iter().sum::<f64>() / winsorized.len() as f64;
    let winsorized_sd = if winsorized.len() > 1 {
        (winsorized
            .iter()
            .map(|value| (value - winsorized_mean).powi(2))
            .sum::<f64>()
            / (winsorized.len() - 1) as f64)
            .sqrt()
    } else {
        0.0
    };
    let all_positive = data.values.iter().all(|value| *value > 0.0);
    let (geometric_mean, geometric_sd, log_skewness) = if all_positive {
        let logs = data
            .values
            .iter()
            .map(|value| value.ln())
            .collect::<Vec<_>>();
        let log_mean = logs.iter().sum::<f64>() / logs.len() as f64;
        let log_variance = if logs.len() > 1 {
            logs.iter()
                .map(|value| (value - log_mean).powi(2))
                .sum::<f64>()
                / (logs.len() - 1) as f64
        } else {
            0.0
        };
        let log_sd = log_variance.sqrt();
        (
            Some(log_mean.exp()),
            Some(log_sd.exp()),
            sample_skewness(&logs, log_mean, log_sd),
        )
    } else {
        (None, None, None)
    };
    let harmonic_mean = all_positive.then(|| {
        data.values.len() as f64 / data.values.iter().map(|value| 1.0 / value).sum::<f64>()
    });
    let root_mean_square = (data.values.iter().map(|value| value * value).sum::<f64>()
        / data.values.len() as f64)
        .sqrt();
    let mode_value = summary.mode.map(|value| value.0);
    let mode_count = summary.mode.map(|value| value.1 as i64);
    let arithmetic_status = if summary.skewness.is_some_and(|value| value.abs() < 0.5)
        && summary.outlier_positions.is_empty()
    {
        "suggested descriptive pair"
    } else {
        "compare with robust pair"
    };
    let median_status = if summary.skewness.is_some_and(|value| value.abs() >= 0.5)
        || !summary.outlier_positions.is_empty()
    {
        "suggested descriptive pair"
    } else {
        "useful robust companion"
    };
    let multiplicative_clue = all_positive
        && summary.skewness.is_some_and(|value| value > 0.5)
        && log_skewness.is_some_and(|logged| logged.abs() < summary.skewness.unwrap_or(0.0).abs());
    let pairs = vec![
        record([
            ("centre", text("arithmetic mean")),
            ("spread", text("standard deviation")),
            ("status", text(arithmetic_status)),
            ("use_when", text("Differences are additive and the distribution is reasonably symmetric without dominant extremes.")),
            ("avoid_as_typical_when", text("A long tail, mixture, or influential extreme makes equal-share balance unlike a typical observation.")),
        ]),
        record([
            ("centre", text("median")),
            ("spread", text("IQR or MAD")),
            ("status", text(median_status)),
            ("use_when", text("The distribution is skewed, heavy-tailed, ordinal, or contains valid extremes.")),
            ("avoid_as_only_summary_when", text("The scientific target is an additive total or expected value, for which the arithmetic mean answers a different question.")),
        ]),
        record([
            ("centre", text("geometric mean")),
            ("spread", text("geometric SD or a multiplicative interval")),
            ("status", text(if multiplicative_clue { "multiplicative-scale clue" } else { "requires a positive multiplicative target" })),
            ("use_when", text("Values are positive and ratios, fold changes, or compound growth are scientifically meaningful.")),
            ("avoid_as_default_when", text("Zeros, negative values, additive effects, or raw count sampling define the problem.")),
        ]),
        record([
            ("centre", text("weighted mean")),
            ("spread", text("weighted SD plus design-aware standard error")),
            ("status", text("requires justified external weights")),
            ("use_when", text("Observations represent unequal frequencies, precision, exposure, or sampling probabilities.")),
            ("avoid_as_default_when", text("Weights are chosen after seeing outcomes or the survey clustering/stratification is ignored.")),
        ]),
        record([
            ("centre", text("harmonic mean")),
            ("spread", text("raw rate quantiles plus uncertainty for the target rate")),
            ("status", text("special-purpose positive-rate aggregate")),
            ("use_when", text("Averaging positive rates with a common fixed numerator or amount of work.")),
            ("avoid_as_generic_centre_when", text("The denominator or exposure differs, or values are not positive rates.")),
        ]),
        record([
            ("centre", text("trimmed mean")),
            ("spread", text("winsorized SD or bootstrap interval")),
            ("status", text("predeclared robust sensitivity summary")),
            ("use_when", text("A symmetric trimming rule was chosen in advance and both centre efficiency and tail resistance matter.")),
            ("avoid_as_default_when", text("Trimming would hide a real subgroup, data error, or scientifically meaningful extreme.")),
        ]),
        record([
            ("centre", text("mode")),
            ("spread", text("counts or proportions across categories/peaks")),
            ("status", text("frequency summary, not a universal numeric centre")),
            ("use_when", text("The most common category, discrete value, or distribution peak answers the question.")),
            ("avoid_as_default_when", text("Continuous measurements have few exact repeats or multiple peaks represent subpopulations.")),
        ]),
    ];
    Ok(record([
        ("schema", text("biolang.stats.means/v1")),
        ("kind", text("means_guide")),
        ("observations", Value::Int(summary.n as i64)),
        ("excluded", Value::Int((data.missing + data.non_finite) as i64)),
        ("arithmetic_mean", Value::Float(summary.mean)),
        ("median", Value::Float(summary.median)),
        ("mode", number(mode_value)),
        ("mode_count", mode_count.map(Value::Int).unwrap_or(Value::Nil)),
        ("geometric_mean", number(geometric_mean)),
        ("geometric_sd", number(geometric_sd)),
        (
            "geometric_one_sd_lower",
            number(geometric_mean.zip(geometric_sd).map(|(centre, spread)| centre / spread)),
        ),
        (
            "geometric_one_sd_upper",
            number(geometric_mean.zip(geometric_sd).map(|(centre, spread)| centre * spread)),
        ),
        ("harmonic_mean", number(harmonic_mean)),
        ("trim_fraction", Value::Float(trim_fraction)),
        ("trimmed_each_side", Value::Int(trim_each_side as i64)),
        ("trimmed_mean", Value::Float(trimmed_mean)),
        ("winsorized_sd", Value::Float(winsorized_sd)),
        ("root_mean_square", Value::Float(root_mean_square)),
        ("sample_sd", number(summary.sd)),
        ("iqr", Value::Float(summary.iqr)),
        ("mad", Value::Float(summary.mad)),
        ("raw_skewness", number(summary.skewness)),
        ("log_skewness", number(log_skewness)),
        ("all_values_positive", Value::Bool(all_positive)),
        ("multiplicative_scale_clue", Value::Bool(multiplicative_clue)),
        ("centre_spread_pairs", list(pairs)),
        ("automatic_choice", Value::Bool(false)),
        ("input_modified", Value::Bool(false)),
        (
            "quick_explanation",
            text("An average is a scientific question, not merely a formula. Choose its matching spread and uncertainty from the same scale and sampling design."),
        ),
    ]))
}

fn decision_map(args: Vec<Value>) -> Result<Value> {
    let opts = options(&args, 0, "stats_decision_map")?;
    let title = opts
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Choose the question before the statistic");
    let paths = vec![
        record([
            ("question", text("additive equal-share centre")),
            ("centre", text("arithmetic mean")),
            ("spread", text("standard deviation")),
            ("scale", text("original additive scale")),
            (
                "uncertainty",
                text("design-aware SE or confidence interval"),
            ),
        ]),
        record([
            (
                "question",
                text("typical ranked value or skew-resistant centre"),
            ),
            ("centre", text("median")),
            ("spread", text("IQR or MAD")),
            (
                "scale",
                text("original scale; preview transforms separately"),
            ),
            (
                "uncertainty",
                text("quantile or bootstrap interval using the correct resampling unit"),
            ),
        ]),
        record([
            (
                "question",
                text("positive ratios, folds, or compound growth"),
            ),
            ("centre", text("geometric mean")),
            ("spread", text("geometric SD or multiplicative interval")),
            (
                "scale",
                text("log scale for calculation, back-transform for reporting"),
            ),
            (
                "uncertainty",
                text("log-scale interval, then back-transform endpoints"),
            ),
        ]),
        record([
            (
                "question",
                text("unequal representation, exposure, or precision"),
            ),
            ("centre", text("weighted mean")),
            ("spread", text("weighted SD")),
            ("scale", text("scale justified by the estimand")),
            (
                "uncertainty",
                text("survey/design-aware SE; weights alone are insufficient"),
            ),
        ]),
        record([
            (
                "question",
                text("positive rates over a fixed amount of work"),
            ),
            ("centre", text("harmonic mean")),
            ("spread", text("raw rate quantiles")),
            ("scale", text("rate or reciprocal-time scale")),
            (
                "uncertainty",
                text("interval for the target aggregate rate"),
            ),
        ]),
        record([
            ("question", text("most common category, value, or peak")),
            ("centre", text("mode")),
            (
                "spread",
                text("counts/proportions across categories or peaks"),
            ),
            (
                "scale",
                text("categorical or carefully binned numeric scale"),
            ),
            (
                "uncertainty",
                text("binomial/multinomial interval when applicable"),
            ),
        ]),
    ];
    let ascii = format!(
        "{title}\n\nQUESTION                     CENTRE          SPREAD             SCALE              UNCERTAINTY\nadditive equal share    -> mean        -> SD             -> original       -> design-aware CI\ntypical / skew-resistant-> median      -> IQR or MAD     -> original       -> quantile/bootstrap CI\nratios / fold changes   -> geometric   -> geometric SD   -> log/backtransform-> log-scale CI\nunequal representation  -> weighted    -> weighted SD    -> justified      -> design-aware SE\nfixed-work rates        -> harmonic    -> rate quantiles -> rate/reciprocal-> target-rate CI\nmost frequent category  -> mode        -> proportions    -> categorical    -> binomial/multinomial CI\n\nBefore following a row: identify units, experimental unit, dependence, censoring, and the scientific estimand. No row is selected automatically."
    );
    let rows = [
        (
            "Additive equal share",
            "Mean",
            "SD",
            "Original",
            "Design-aware CI",
        ),
        (
            "Typical / skew-resistant",
            "Median",
            "IQR / MAD",
            "Original",
            "Quantile/bootstrap CI",
        ),
        (
            "Ratios / folds",
            "Geometric",
            "Geometric SD",
            "Log -> back",
            "Log-scale CI",
        ),
        (
            "Unequal representation",
            "Weighted",
            "Weighted SD",
            "Justified",
            "Design-aware SE",
        ),
        (
            "Fixed-work rates",
            "Harmonic",
            "Rate quantiles",
            "Rate",
            "Rate CI",
        ),
        (
            "Most frequent",
            "Mode",
            "Proportions",
            "Categorical",
            "Binomial/multinomial CI",
        ),
    ];
    let mut svg_rows = String::new();
    for (index, (question, centre, spread, scale, uncertainty)) in rows.iter().enumerate() {
        let y = 82 + index * 46;
        let fill = if index % 2 == 0 { "#f8fafc" } else { "#eef6f8" };
        svg_rows.push_str(&format!(
            "<rect x=\"18\" y=\"{}\" width=\"964\" height=\"42\" rx=\"5\" fill=\"{}\"/><text class=\"cell\" x=\"30\" y=\"{}\">{}</text><text class=\"cell\" x=\"305\" y=\"{}\">{}</text><text class=\"cell\" x=\"445\" y=\"{}\">{}</text><text class=\"cell\" x=\"610\" y=\"{}\">{}</text><text class=\"cell\" x=\"752\" y=\"{}\">{}</text>",
            y,
            fill,
            y + 26,
            html_escape(question),
            y + 26,
            html_escape(centre),
            y + 26,
            html_escape(spread),
            y + 26,
            html_escape(scale),
            y + 26,
            html_escape(uncertainty),
        ));
    }
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1000 390\" role=\"img\" aria-labelledby=\"dmt dmd\"><title id=\"dmt\">{}</title><desc id=\"dmd\">A decision map linking a scientific question to a compatible centre, spread, scale, and uncertainty method.</desc><style>.title{{font:700 20px system-ui;fill:#123b5d}}.head{{font:700 13px system-ui;fill:#334155}}.cell{{font:13px system-ui;fill:#172033}}.note{{font:12px system-ui;fill:#7c2d12}}</style><text class=\"title\" x=\"18\" y=\"30\">{}</text><text class=\"head\" x=\"30\" y=\"68\">QUESTION</text><text class=\"head\" x=\"305\" y=\"68\">CENTRE</text><text class=\"head\" x=\"445\" y=\"68\">SPREAD</text><text class=\"head\" x=\"610\" y=\"68\">SCALE</text><text class=\"head\" x=\"752\" y=\"68\">UNCERTAINTY</text>{}<text class=\"note\" x=\"18\" y=\"374\">Check units, design, dependence, censoring, and estimand first. BioLang does not select a row automatically.</text></svg>",
        html_escape(title),
        html_escape(title),
        svg_rows,
    );
    Ok(record([
        ("schema", text("biolang.stats.decision-map/v1")),
        ("kind", text("centre_spread_scale_uncertainty_map")),
        ("title", text(title)),
        ("paths", list(paths)),
        ("ascii", text(ascii)),
        ("svg", text(svg)),
        ("automatic_choice", Value::Bool(false)),
        (
            "interpretation_boundary",
            text("The map organizes questions; it cannot infer the estimand or sampling design from observed values."),
        ),
    ]))
}

fn time_series_diagnostics(args: Vec<Value>) -> Result<Value> {
    let function = "stats_time_series_diagnostics";
    let data = numeric_data(&args[0], function)?;
    let opts = options(&args, 1, function)?;
    if data.missing > 0 || data.non_finite > 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() does not compress missing time points; impute, model, or explicitly regularize the time axis first"),
            None,
        ));
    }
    let n = data.values.len();
    if n < 5 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() requires at least five ordered observations"),
            None,
        ));
    }
    let default_lag = ((n as f64).sqrt().floor() as usize).clamp(1, 20);
    let max_lag = opts
        .get("max_lag")
        .and_then(Value::as_int)
        .unwrap_or(default_lag as i64)
        .clamp(1, n.saturating_sub(2).min(100) as i64) as usize;
    let mean = data.values.iter().sum::<f64>() / n as f64;
    let centred_ss = data
        .values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>();
    if centred_ss <= f64::EPSILON {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() is undefined for a constant series"),
            None,
        ));
    }
    let autocorrelations = (1..=max_lag)
        .map(|lag| {
            let correlation = (lag..n)
                .map(|index| (data.values[index] - mean) * (data.values[index - lag] - mean))
                .sum::<f64>()
                / centred_ss;
            record([
                ("lag", Value::Int(lag as i64)),
                ("autocorrelation", Value::Float(correlation)),
            ])
        })
        .collect::<Vec<_>>();
    let acf_values = autocorrelations
        .iter()
        .filter_map(|value| match value {
            Value::Record(map) => map.get("autocorrelation").and_then(Value::as_float),
            _ => None,
        })
        .collect::<Vec<_>>();
    let ljung_box_q = n as f64
        * (n + 2) as f64
        * acf_values
            .iter()
            .enumerate()
            .map(|(index, correlation)| correlation.powi(2) / (n - index - 1) as f64)
            .sum::<f64>();
    let ljung_box_p = bl_core::bio_core::stats_ops::chi_square_sf(ljung_box_q, max_lag);
    let index = (0..n).map(|value| value as f64).collect::<Vec<_>>();
    let trend = pearson(&index, &data.values);
    let differences = data
        .values
        .windows(2)
        .map(|pair| pair[1] - pair[0])
        .collect::<Vec<_>>();
    let difference_mean = differences.iter().sum::<f64>() / differences.len() as f64;
    let difference_sd = (differences
        .iter()
        .map(|value| (value - difference_mean).powi(2))
        .sum::<f64>()
        / (differences.len() - 1) as f64)
        .sqrt();
    let threshold = 1.96 / (n as f64).sqrt();
    let notable_lags = acf_values
        .iter()
        .enumerate()
        .filter(|(_, value)| value.abs() > threshold)
        .map(|(index, _)| Value::Int((index + 1) as i64))
        .collect::<Vec<_>>();
    let mut ascii = format!(
        "Ordered-series diagnostic (n={n})\ntrend/step={}  first-difference SD={}\n\nACF\n",
        fmt_number(trend.map(|value| value.1).unwrap_or(0.0)),
        fmt_number(difference_sd)
    );
    for (index, correlation) in acf_values.iter().enumerate() {
        let bars = (correlation.abs() * 20.0).round().clamp(0.0, 20.0) as usize;
        ascii.push_str(&format!(
            "lag {:>3} {:+.4} |{}\n",
            index + 1,
            correlation,
            "#".repeat(bars)
        ));
    }
    ascii.push_str(&format!(
        "\nLjung-Box Q({max_lag})={} p={}\nNo model selected; regular spacing is assumed.",
        fmt_number(ljung_box_q),
        fmt_number(ljung_box_p)
    ));
    let mut issues = Vec::new();
    if trend.is_some_and(|value| value.0.abs() >= 0.5) {
        issues.push(issue(
            "ordered_trend_clue",
            format!("Correlation with observation order is {}.", fmt_number(trend.unwrap().0)),
            "Model the time scale and scientific intervention points; order is not a substitute for actual timestamps.",
            "review",
        ));
    }
    if ljung_box_p < 0.05 {
        issues.push(issue(
            "serial_dependence_clue",
            format!("Ljung-Box Q({max_lag}) is {} with p={}.", fmt_number(ljung_box_q), fmt_number(ljung_box_p)),
            "Use time-aware models or resampling; ordinary independent-row uncertainty can be too optimistic.",
            "review",
        ));
    }
    Ok(record([
        ("schema", text("biolang.stats.time-series-diagnostics/v1")),
        ("kind", text("time_series_diagnostics")),
        ("observations", Value::Int(n as i64)),
        ("max_lag", Value::Int(max_lag as i64)),
        ("autocorrelations", list(autocorrelations)),
        ("approximate_acf_review_threshold", Value::Float(threshold)),
        ("notable_lags", list(notable_lags)),
        ("ljung_box_q", Value::Float(ljung_box_q)),
        ("ljung_box_df", Value::Int(max_lag as i64)),
        ("ljung_box_p_value", Value::Float(ljung_box_p)),
        ("order_correlation", number(trend.map(|value| value.0))),
        ("trend_per_observation", number(trend.map(|value| value.1))),
        ("trend_intercept", number(trend.map(|value| value.2))),
        ("first_difference_mean", Value::Float(difference_mean)),
        ("first_difference_sd", Value::Float(difference_sd)),
        ("ascii", text(ascii)),
        ("issues", list(issues)),
        ("model_selected", Value::Bool(false)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "Observation spacing is assumed regular; provide a scientifically valid regular series before using lag diagnostics.",
                "The ACF threshold is an approximate visual clue and is not multiplicity-adjusted.",
                "Trend and autocorrelation checks do not select an ARIMA, state-space, seasonal, or intervention model.",
            ]),
        ),
        (
            "quick_explanation",
            text("These checks show trend, lag dependence, and first-difference scale while preserving the original ordered series."),
        ),
    ]))
}

fn cluster_diagnostics(args: Vec<Value>) -> Result<Value> {
    let function = "stats_cluster_diagnostics";
    let opts = options(&args, 2, function)?;
    let (Value::List(values), Value::List(clusters)) = (&args[0], &args[1]) else {
        return Err(BioLangError::type_error(
            format!("{function}() values and clusters must both be List"),
            None,
        ));
    };
    if values.len() != clusters.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{function}() values and clusters must have equal length"),
            None,
        ));
    }
    let mut grouped = HashMap::<String, Vec<f64>>::new();
    let mut order = Vec::<String>::new();
    let mut excluded = 0usize;
    for (index, (value, cluster)) in values.iter().zip(clusters.iter()).enumerate() {
        if matches!(value, Value::Nil) || matches!(cluster, Value::Nil) {
            excluded += 1;
            continue;
        }
        let Some(value) = finite_number(value) else {
            return Err(BioLangError::type_error(
                format!("{function}() value at index {index} must be finite numeric or Nil"),
                None,
            ));
        };
        let Some(cluster) = category_label(cluster) else {
            return Err(BioLangError::type_error(
                format!("{function}() cluster at index {index} must be scalar or Nil"),
                None,
            ));
        };
        if !grouped.contains_key(&cluster) {
            order.push(cluster.clone());
        }
        grouped.entry(cluster).or_default().push(value);
    }
    let group_count = grouped.len();
    let n = grouped.values().map(Vec::len).sum::<usize>();
    if group_count < 2 || n <= group_count {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{function}() requires at least two clusters and at least one repeated observation"
            ),
            None,
        ));
    }
    let grand_mean = grouped.values().flatten().sum::<f64>() / n as f64;
    let mut between_ss = 0.0;
    let mut within_ss = 0.0;
    let mut sum_squared_sizes = 0usize;
    let mut sizes = Vec::with_capacity(group_count);
    let mut details = Vec::new();
    let mut ascii_rows = Vec::new();
    let max_details = opts
        .get("max_cluster_details")
        .and_then(Value::as_int)
        .unwrap_or(20)
        .clamp(0, 100) as usize;
    for label in &order {
        let observations = &grouped[label];
        let group_mean = observations.iter().sum::<f64>() / observations.len() as f64;
        between_ss += observations.len() as f64 * (group_mean - grand_mean).powi(2);
        within_ss += observations
            .iter()
            .map(|value| (value - group_mean).powi(2))
            .sum::<f64>();
        sizes.push(observations.len());
        sum_squared_sizes += observations.len() * observations.len();
        if details.len() < max_details {
            details.push(record([
                ("cluster", text(label)),
                ("observations", Value::Int(observations.len() as i64)),
                ("mean", Value::Float(group_mean)),
            ]));
            ascii_rows.push(format!(
                "{:<18} n={:>4} mean={}",
                label,
                observations.len(),
                fmt_number(group_mean)
            ));
        }
    }
    let between_ms = between_ss / (group_count - 1) as f64;
    let within_ms = within_ss / (n - group_count) as f64;
    let effective_cluster_size =
        (n as f64 - sum_squared_sizes as f64 / n as f64) / (group_count - 1) as f64;
    let denominator = between_ms + (effective_cluster_size - 1.0) * within_ms;
    let icc = if denominator.abs() > f64::EPSILON {
        (between_ms - within_ms) / denominator
    } else {
        0.0
    };
    let mean_cluster_size = n as f64 / group_count as f64;
    let nonnegative_icc = icc.max(0.0);
    let approximate_design_effect = 1.0 + (mean_cluster_size - 1.0) * nonnegative_icc;
    let approximate_effective_n = n as f64 / approximate_design_effect;
    let total_ss = between_ss + within_ss;
    let between_fraction = (total_ss > f64::EPSILON).then(|| between_ss / total_ss);
    let minimum_size = sizes.iter().copied().min().unwrap_or(0);
    let maximum_size = sizes.iter().copied().max().unwrap_or(0);
    let mut issues = Vec::new();
    if icc >= 0.05 {
        issues.push(issue(
            "within_cluster_similarity",
            format!("One-way random-effects ICC is {}.", fmt_number(icc)),
            "Use cluster-aware uncertainty, resampling, aggregation, GEE, or a mixed model chosen for the scientific estimand.",
            "review",
        ));
    }
    if maximum_size > minimum_size.saturating_mul(3).max(1) {
        issues.push(issue(
            "unequal_cluster_sizes",
            format!("Cluster sizes range from {minimum_size} to {maximum_size}."),
            "Strongly unequal sizes can change weighting and precision; inspect the unit-level data and model assumptions.",
            "review",
        ));
    }
    let mut ascii = format!(
        "Cluster diagnostic (n={n}, clusters={group_count})\nICC={}  approximate effective n={}\n\n",
        fmt_number(icc),
        fmt_number(approximate_effective_n)
    );
    ascii.push_str(&ascii_rows.join("\n"));
    if group_count > max_details {
        ascii.push_str(&format!(
            "\n... {} more cluster(s) not shown",
            group_count - max_details
        ));
    }
    ascii.push_str("\n\nDescriptive one-way clue; no mixed model fitted.");
    Ok(record([
        ("schema", text("biolang.stats.cluster-diagnostics/v1")),
        ("kind", text("cluster_diagnostics")),
        ("complete_observations", Value::Int(n as i64)),
        ("excluded_observations", Value::Int(excluded as i64)),
        ("clusters", Value::Int(group_count as i64)),
        ("minimum_cluster_size", Value::Int(minimum_size as i64)),
        ("maximum_cluster_size", Value::Int(maximum_size as i64)),
        ("mean_cluster_size", Value::Float(mean_cluster_size)),
        ("effective_cluster_size", Value::Float(effective_cluster_size)),
        ("grand_mean", Value::Float(grand_mean)),
        ("between_cluster_mean_square", Value::Float(between_ms)),
        ("within_cluster_mean_square", Value::Float(within_ms)),
        ("intraclass_correlation", Value::Float(icc)),
        ("between_cluster_variance_fraction", number(between_fraction)),
        (
            "approximate_unequal_independence_design_effect",
            Value::Float(approximate_design_effect),
        ),
        (
            "approximate_effective_sample_size",
            Value::Float(approximate_effective_n),
        ),
        ("cluster_details", list(details)),
        ("ascii", text(ascii)),
        (
            "cluster_details_truncated",
            Value::Bool(group_count > max_details),
        ),
        ("issues", list(issues)),
        ("mixed_model_fitted", Value::Bool(false)),
        ("input_modified", Value::Bool(false)),
        (
            "limitations",
            string_list([
                "The one-way ICC is a descriptive random-intercept clue; it does not fit fixed effects, random slopes, nesting, or crossed effects.",
                "The design effect and effective sample size are approximations based on mean cluster size and a non-negative ICC.",
                "Negative ICC estimates are retained as finite-sample evidence but are truncated to zero only for the design-effect approximation.",
            ]),
        ),
        (
            "quick_explanation",
            text("The ICC estimates how similar outcomes are within the declared cluster; the design-effect approximation shows why repeated rows do not provide the same information as independent rows."),
        ),
    ]))
}

struct AxisMoments {
    means: Vec<f64>,
    variances: Vec<f64>,
    zero_fractions: Vec<f64>,
}

fn axis_moments(value: &Value, axis: &str, function: &str) -> Result<AxisMoments> {
    let facts = matrix_facts(value, function)?;
    let along_rows = axis == "rows";
    let groups = if along_rows {
        facts.rows
    } else {
        facts.columns
    };
    let group_size = if along_rows {
        facts.columns
    } else {
        facts.rows
    };
    let mut sums = vec![0.0; groups];
    let mut sum_squares = vec![0.0; groups];
    let mut zeros = vec![0usize; groups];
    let mut finite = vec![0usize; groups];
    let mut update = |row: usize, column: usize, value: f64| {
        let index = if along_rows { row } else { column };
        if !value.is_finite() {
            return;
        }
        finite[index] += 1;
        sums[index] += value;
        sum_squares[index] += value * value;
        if value == 0.0 {
            zeros[index] += 1;
        }
    };
    match value {
        Value::Matrix(matrix) => {
            for row in 0..matrix.nrow {
                for column in 0..matrix.ncol {
                    update(row, column, matrix.get(row, column));
                }
            }
        }
        Value::SparseMatrix(matrix) => {
            drop(update);
            finite.fill(group_size);
            zeros.fill(group_size);
            for row in 0..matrix.nrow {
                for position in matrix.indptr[row]..matrix.indptr[row + 1] {
                    let column = matrix.indices[position];
                    let value = matrix.data[position];
                    let index = if along_rows { row } else { column };
                    if !value.is_finite() {
                        finite[index] = finite[index].saturating_sub(1);
                        zeros[index] = zeros[index].saturating_sub(1);
                    } else {
                        sums[index] += value;
                        sum_squares[index] += value * value;
                        if value != 0.0 {
                            zeros[index] = zeros[index].saturating_sub(1);
                        }
                    }
                }
            }
        }
        Value::Table(table) => {
            for (row_index, row) in table.rows.iter().enumerate() {
                for column in 0..table.columns.len() {
                    let value = row.get(column).unwrap_or(&Value::Nil);
                    match value {
                        Value::Int(value) => update(row_index, column, *value as f64),
                        Value::Float(value) => update(row_index, column, *value),
                        Value::Nil => {}
                        other => {
                            return Err(BioLangError::type_error(
                                format!("{function}() matrix table contains {} at row {row_index}, column {column}", other.type_of()),
                                None,
                            ));
                        }
                    }
                }
            }
        }
        Value::List(rows) => {
            for (row_index, row) in rows.iter().enumerate() {
                let Value::List(row) = row else {
                    return Err(BioLangError::type_error(
                        format!("{function}() matrix row {row_index} is not a List"),
                        None,
                    ));
                };
                for (column, value) in row.iter().enumerate() {
                    match value {
                        Value::Int(value) => update(row_index, column, *value as f64),
                        Value::Float(value) => update(row_index, column, *value),
                        Value::Nil => {}
                        other => {
                            return Err(BioLangError::type_error(
                                format!("{function}() matrix contains {} at row {row_index}, column {column}", other.type_of()),
                                None,
                            ));
                        }
                    }
                }
            }
        }
        _ => unreachable!("matrix_facts validated the matrix value"),
    }
    let mut means = Vec::with_capacity(groups);
    let mut variances = Vec::with_capacity(groups);
    let mut zero_fractions = Vec::with_capacity(groups);
    for index in 0..groups {
        if finite[index] == 0 {
            means.push(f64::NAN);
            variances.push(f64::NAN);
            zero_fractions.push(f64::NAN);
            continue;
        }
        let count = finite[index] as f64;
        let mean = sums[index] / count;
        let variance = if finite[index] > 1 {
            ((sum_squares[index] - sums[index] * sums[index] / count) / (finite[index] - 1) as f64)
                .max(0.0)
        } else {
            0.0
        };
        means.push(mean);
        variances.push(variance);
        zero_fractions.push(zeros[index] as f64 / group_size.max(1) as f64);
    }
    Ok(AxisMoments {
        means,
        variances,
        zero_fractions,
    })
}

fn coefficient_of_variation(values: &[f64]) -> Option<f64> {
    let finite = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if finite.len() < 2 {
        return None;
    }
    let mean = finite.iter().sum::<f64>() / finite.len() as f64;
    if mean.abs() <= f64::EPSILON {
        return None;
    }
    let variance = finite
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (finite.len() - 1) as f64;
    Some(variance.sqrt() / mean.abs())
}

fn omics_profile(args: Vec<Value>) -> Result<Value> {
    let function = "stats_omics_profile";
    let facts = matrix_facts(&args[0], function)?;
    let opts = options(&args, 1, function)?;
    let modality = opts
        .get("modality")
        .and_then(Value::as_str)
        .unwrap_or("generic");
    if !matches!(
        modality,
        "generic" | "bulk_rnaseq" | "single_cell" | "proteomics" | "metabolomics" | "microbiome"
    ) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_omics_profile() modality must be generic, bulk_rnaseq, single_cell, proteomics, metabolomics, or microbiome",
            None,
        ));
    }
    let sample_axis = opts
        .get("sample_axis")
        .and_then(Value::as_str)
        .unwrap_or("rows");
    if !matches!(sample_axis, "rows" | "columns") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "stats_omics_profile() sample_axis must be rows or columns",
            None,
        ));
    }
    let feature_axis = if sample_axis == "rows" {
        "columns"
    } else {
        "rows"
    };
    let sample_moments = axis_moments(&args[0], sample_axis, function)?;
    let feature_moments = axis_moments(&args[0], feature_axis, function)?;
    let sample_totals = if sample_axis == "rows" {
        &facts.row_totals
    } else {
        &facts.column_totals
    };
    let sample_count = sample_totals.len();
    let feature_count = feature_moments.means.len();
    let zero_fraction = facts.zeros as f64 / facts.cells.max(1) as f64;
    let sample_total_cv = coefficient_of_variation(sample_totals);
    let valid_feature_pairs = feature_moments
        .means
        .iter()
        .zip(&feature_moments.variances)
        .filter(|(mean, variance)| mean.is_finite() && variance.is_finite())
        .map(|(mean, variance)| (*mean, *variance))
        .collect::<Vec<_>>();
    let feature_means = valid_feature_pairs
        .iter()
        .map(|(mean, _)| *mean)
        .collect::<Vec<_>>();
    let feature_variances = valid_feature_pairs
        .iter()
        .map(|(_, variance)| *variance)
        .collect::<Vec<_>>();
    let mean_variance_correlation =
        pearson(&feature_means, &feature_variances).map(|value| value.0);
    let median_sample_zero_fraction = {
        let mut values = sample_moments
            .zero_fractions
            .iter()
            .copied()
            .filter(|value| value.is_finite())
            .collect::<Vec<_>>();
        values.sort_by(f64::total_cmp);
        (!values.is_empty()).then(|| median(&values))
    };
    let zero_total_samples = sample_totals.iter().filter(|value| **value == 0.0).count();
    let mut ranked_features = (0..feature_count)
        .filter_map(|index| {
            let mean = feature_moments.means[index];
            let variance = feature_moments.variances[index];
            (mean.is_finite() && variance.is_finite() && mean > 0.0)
                .then_some((index, variance / mean))
        })
        .collect::<Vec<_>>();
    ranked_features.sort_by(|left, right| {
        right
            .1
            .total_cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let max_features = opts
        .get("top_variable_features")
        .and_then(Value::as_int)
        .unwrap_or(20)
        .clamp(0, 100) as usize;
    let variable_feature_clues = ranked_features
        .into_iter()
        .take(max_features)
        .map(|(index, dispersion)| {
            record([
                ("index", Value::Int(index as i64)),
                ("mean", Value::Float(feature_moments.means[index])),
                ("variance", Value::Float(feature_moments.variances[index])),
                ("variance_mean_ratio", Value::Float(dispersion)),
                (
                    "zero_fraction",
                    Value::Float(feature_moments.zero_fractions[index]),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let mut issues = Vec::new();
    if facts.non_finite > 0 {
        issues.push(issue(
            "missing_or_non_finite_matrix_values",
            format!(
                "{} matrix cell(s) are missing or non-finite.",
                facts.non_finite
            ),
            "Map the measurement and missingness process before normalization or imputation.",
            "review",
        ));
    }
    if zero_total_samples > 0 {
        issues.push(issue(
            "zero_total_samples",
            format!("{zero_total_samples} declared sample(s) have total zero."),
            "Many normalizations and distance calculations are undefined or uninformative for an all-zero sample.",
            "blocking",
        ));
    }
    if matches!(modality, "bulk_rnaseq" | "single_cell")
        && (facts.negative > 0 || facts.non_integer > 0)
    {
        issues.push(issue(
            "raw_count_contract_mismatch",
            format!("The matrix contains {} negative and {} non-integer finite value(s).", facts.negative, facts.non_integer),
            "Declare whether this is raw count, normalized, or residual data before applying count-specific QC or models.",
            "blocking",
        ));
    }
    if modality == "single_cell" && zero_fraction < 0.2 {
        issues.push(issue(
            "unexpectedly_dense_single_cell_matrix",
            format!("Overall zero fraction is {}%.", fmt_number(zero_fraction * 100.0)),
            "The input may already be transformed, filtered, aggregated, or stored on a non-count assay.",
            "review",
        ));
    }
    if sample_total_cv.is_some_and(|value| value >= 0.5) {
        issues.push(issue(
            "unequal_sample_totals",
            format!("Sample-total coefficient of variation is {}.", fmt_number(sample_total_cv.unwrap())),
            "Depth, biomass, loading, exposure, composition, or genuine global shifts may contribute; choose a denominator from the measurement process.",
            "review",
        ));
    }
    let suggestions = match modality {
        "bulk_rnaseq" => vec![
            guidance_option(
                "library-size and composition QC",
                "first",
                "Raw gene counts are arranged by biological sample.",
                "raw counts, sample metadata, experimental unit",
                "Do not infer sample quality from library size alone.",
            ),
            guidance_option(
                "negative-binomial count model with robust size factors",
                "inference_candidate",
                "Replicated count data are compared across a valid design.",
                "raw counts and design matrix",
                "Batch/group confounding and low replication cannot be repaired by normalization.",
            ),
            guidance_option(
                "variance-stabilised PCA and sample correlation",
                "diagnostic_candidate",
                "Sample structure and outliers need visual review.",
                "fitted normalization and transformation",
                "Fit transformations without leaking validation outcomes.",
            ),
        ],
        "single_cell" => vec![
            guidance_option(
                "cell-level library, detection, and biology-aware QC",
                "first",
                "Raw cell-by-gene counts are available.",
                "counts plus mitochondrial/ribosomal annotations and sample IDs",
                "Universal cutoffs can remove valid cell states.",
            ),
            guidance_option(
                "sample-aware normalization and dimensional reduction",
                "analysis_candidate",
                "Cell states are explored after transparent QC.",
                "raw counts, sample/batch labels, retained genes",
                "Cells do not replace biological replicates.",
            ),
            guidance_option(
                "pseudobulk or mixed/sample-aware inference",
                "inference_candidate",
                "Conditions are compared across biological samples.",
                "sample IDs, group labels, cell-state definition",
                "Cell-level tests can create pseudoreplication.",
            ),
        ],
        "proteomics" => vec![
            guidance_option(
                "missingness by run, sample, and abundance",
                "first",
                "Non-detections and censored signals are present.",
                "intensity matrix and acquisition metadata",
                "A single imputation rule rarely represents every missingness process.",
            ),
            guidance_option(
                "log-scale and loading-normalization preview",
                "candidate",
                "Positive intensities are right-skewed and sample loading differs.",
                "raw intensities and QC standards",
                "Zeros and negative background-corrected values require explicit handling.",
            ),
        ],
        "metabolomics" => vec![
            guidance_option(
                "blank, batch, drift, and internal-standard QC",
                "first",
                "Acquisition order and pooled controls are available.",
                "intensity matrix and run metadata",
                "Biological group correction is unsafe when confounded with run order.",
            ),
            guidance_option(
                "log and robust scaling preview",
                "candidate",
                "Positive features span orders of magnitude.",
                "documented detection and replacement rules",
                "Scaling can amplify noisy low-abundance features.",
            ),
        ],
        "microbiome" => vec![
            guidance_option(
                "library and prevalence QC",
                "first",
                "Taxon counts have varying depth and sparsity.",
                "raw counts, sample metadata, taxonomic scope",
                "Rare taxa are not automatically errors.",
            ),
            guidance_option(
                "compositional log-ratio analysis",
                "analysis_candidate",
                "Relative abundance is the measurement scale.",
                "justified zero treatment and reference/log-ratio choice",
                "Total-count scaling does not remove compositional dependence.",
            ),
        ],
        _ => vec![guidance_option(
            "declare the measurement process",
            "first",
            "The matrix modality is not yet specified.",
            "units, sample axis, feature meaning, experimental design",
            "A generic matrix shape cannot determine a valid normalization or model.",
        )],
    };
    Ok(record([
        ("schema", text("biolang.stats.omics-profile/v1")),
        ("kind", text("omics_matrix_profile")),
        ("modality", text(modality)),
        ("sample_axis", text(sample_axis)),
        ("rows", Value::Int(facts.rows as i64)),
        ("columns", Value::Int(facts.columns as i64)),
        ("cells", Value::Int(facts.cells as i64)),
        ("samples", Value::Int(sample_count as i64)),
        ("features", Value::Int(feature_count as i64)),
        ("zeros", Value::Int(facts.zeros as i64)),
        ("zero_fraction", Value::Float(zero_fraction)),
        ("negative", Value::Int(facts.negative as i64)),
        ("non_integer", Value::Int(facts.non_integer as i64)),
        ("non_finite", Value::Int(facts.non_finite as i64)),
        ("zero_total_samples", Value::Int(zero_total_samples as i64)),
        ("sample_total_cv", number(sample_total_cv)),
        ("median_sample_zero_fraction", number(median_sample_zero_fraction)),
        ("feature_mean_variance_correlation", number(mean_variance_correlation)),
        ("sample_totals", list(sample_totals.iter().copied().map(Value::Float).collect())),
        ("variable_feature_clues", list(variable_feature_clues)),
        ("variable_features_selected", Value::Bool(false)),
        ("issues", list(issues)),
        ("suggestions", list(suggestions)),
        ("automatic_changes", Value::Bool(false)),
        ("memory_behavior", text("Streaming axis moments use O(samples + features) additional memory and never densify a SparseMatrix.")),
        (
            "limitations",
            string_list([
                "Feature indices refer to the supplied matrix axis; attach names in the calling analysis when available.",
                "Variance/mean ranking is a descriptive clue and is not a substitute for modality-specific variable-feature modelling.",
                "Sample metadata, experimental units, batches, controls, and feature annotations are required for scientific QC decisions.",
            ]),
        ),
        ("quick_explanation", text("The matrix was profiled on its supplied storage representation and modality-aware next steps were listed without changing values.")),
    ]))
}
