//! Distribution for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

/// Type 7 quantiles — R's default, and what this runtime's `quantile()` gives.
///
/// The box plot used to take `sorted[n / 4]` and `sorted[3 * n / 4]`, which is
/// the nearest-rank rule. On the book's ozone column that puts the top of the
/// box at 64 while `quantile(ozone, 0.75)` reports 63.25, so the picture and
/// the numbers printed beside it disagreed about the same data; on the ten
/// values 1 to 10 the two rules give 3 and 8 against 3.25 and 7.75. Expects
/// `sorted` already sorted and non-empty.
pub(crate) fn quantile_type7(sorted: &[f64], p: f64) -> f64 {
    let h = (sorted.len() - 1) as f64 * p;
    let lower = h.floor() as usize;
    let upper = (lower + 1).min(sorted.len() - 1);
    sorted[lower] + (h - h.floor()) * (sorted[upper] - sorted[lower])
}

/// The numbers in a list argument, for the plots that take one list.
///
/// Numeric strings are accepted because a column read from a CSV arrives as
/// text often enough that rejecting it would be the wrong default.
pub(super) fn numeric_list(value: &Value, who: &str) -> Result<Vec<f64>> {
    let items = match value {
        Value::List(items) => items,
        _ => {
            return Err(BioLangError::type_error(
                format!("{who}() requires List of numbers"),
                None,
            ))
        }
    };
    let mut numbers = Vec::with_capacity(items.len());
    for item in items.iter() {
        match item {
            Value::Int(n) => numbers.push(*n as f64),
            Value::Float(f) => numbers.push(*f),
            Value::Str(s) => {
                if let Ok(f) = s.parse::<f64>() {
                    numbers.push(f);
                }
            }
            _ => {}
        }
    }
    if numbers.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{who}() received no numeric values - check that your data contains numbers, not strings"
            ),
            None,
        ));
    }
    Ok(numbers)
}

pub(super) fn finite_numeric_list(value: &Value, who: &str) -> Result<(Vec<f64>, usize)> {
    let numbers = numeric_list(value, who)?;
    let original_len = numbers.len();
    let finite = numbers
        .into_iter()
        .filter(|number| number.is_finite())
        .collect::<Vec<_>>();
    if finite.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() received no finite numeric values"),
            None,
        ));
    }
    let dropped = original_len - finite.len();
    Ok((finite, dropped))
}

pub(super) fn median_sorted(sorted: &[f64]) -> f64 {
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

pub(super) fn tukey_hinges(sorted: &[f64]) -> (f64, f64) {
    let half = sorted.len().div_ceil(2);
    (
        median_sorted(&sorted[..half]),
        median_sorted(&sorted[sorted.len() - half..]),
    )
}

#[derive(Clone)]
pub(crate) struct BoxGeometry {
    pub(crate) group: String,
    pub(crate) n: usize,
    pub(crate) q1: f64,
    pub(crate) median: f64,
    pub(crate) q3: f64,
    pub(crate) whisker_low: f64,
    pub(crate) whisker_high: f64,
    pub(crate) outliers: Vec<(usize, f64)>,
    pub(crate) dropped: usize,
}

pub(crate) fn box_geometry(
    name: &str,
    values: &[f64],
    method: &str,
    coefficient: f64,
) -> BoxGeometry {
    let mut indexed = values
        .iter()
        .copied()
        .enumerate()
        .filter(|(_, value)| value.is_finite())
        .collect::<Vec<_>>();
    let dropped = values.len() - indexed.len();
    indexed.sort_by(|left, right| left.1.total_cmp(&right.1));
    let sorted = indexed.iter().map(|(_, value)| *value).collect::<Vec<_>>();
    let (q1, q3) = if method == "tukey" {
        tukey_hinges(&sorted)
    } else {
        (quantile_type7(&sorted, 0.25), quantile_type7(&sorted, 0.75))
    };
    let median = median_sorted(&sorted);
    let iqr = q3 - q1;
    let low_fence = q1 - coefficient * iqr;
    let high_fence = q3 + coefficient * iqr;
    let whisker_low = sorted
        .iter()
        .copied()
        .find(|value| *value >= low_fence)
        .unwrap_or(sorted[0]);
    let whisker_high = sorted
        .iter()
        .copied()
        .rev()
        .find(|value| *value <= high_fence)
        .unwrap_or(sorted[sorted.len() - 1]);
    let outliers = indexed
        .iter()
        .filter(|(_, value)| *value < whisker_low || *value > whisker_high)
        .copied()
        .collect();
    BoxGeometry {
        group: name.to_string(),
        n: sorted.len(),
        q1,
        median,
        q3,
        whisker_low,
        whisker_high,
        outliers,
        dropped,
    }
}

pub(super) fn builtin_boxplot_data(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let method = get_opt_str(&opts, "method", "type7").to_ascii_lowercase();
    if !matches!(method.as_str(), "type7" | "tukey") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "boxplot_data() method must be 'type7' or 'tukey'",
            None,
        ));
    }
    let coefficient = get_opt_f64(&opts, "coef", 1.5);
    if !coefficient.is_finite() || coefficient < 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "boxplot_data() coef must be a finite non-negative number",
            None,
        ));
    }
    let groups = match &args[0] {
        Value::List(_) => {
            let (values, dropped) = finite_numeric_list(&args[0], "boxplot_data")?;
            let mut geometry = box_geometry("values", &values, &method, coefficient);
            geometry.dropped += dropped;
            vec![geometry]
        }
        Value::Table(table) => {
            let mut groups = Vec::new();
            for column in &table.columns {
                let values = extract_table_col(table, column)?;
                if values.iter().any(|value| value.is_finite()) {
                    groups.push(box_geometry(column, &values, &method, coefficient));
                }
            }
            if groups.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "boxplot_data() table contains no numeric columns",
                    None,
                ));
            }
            groups
        }
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "boxplot_data() requires List or Table, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let group_rows = groups
        .iter()
        .enumerate()
        .map(|(index, group)| {
            vec![
                Value::Int(index as i64),
                Value::Str(group.group.clone().into()),
                Value::Int(group.n as i64),
                Value::Float(group.q1),
                Value::Float(group.median),
                Value::Float(group.q3),
                Value::Float(group.q3 - group.q1),
                Value::Float(group.whisker_low),
                Value::Float(group.whisker_high),
                Value::Int(group.outliers.len() as i64),
                Value::Int(group.dropped as i64),
            ]
        })
        .collect();
    let outlier_rows = groups
        .iter()
        .enumerate()
        .flat_map(|(group_index, group)| {
            group.outliers.iter().map(move |(source_row, value)| {
                vec![
                    Value::Int(group_index as i64),
                    Value::Str(group.group.clone().into()),
                    Value::Int(*source_row as i64),
                    Value::Float(*value),
                ]
            })
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("boxplot".into())),
            ("method".into(), Value::Str(method.into())),
            ("coefficient".into(), Value::Float(coefficient)),
            (
                "groups".into(),
                Value::Table(Table::new(
                    vec![
                        "group_index".into(),
                        "group".into(),
                        "n".into(),
                        "q1".into(),
                        "median".into(),
                        "q3".into(),
                        "iqr".into(),
                        "whisker_low".into(),
                        "whisker_high".into(),
                        "outlier_count".into(),
                        "dropped_non_finite".into(),
                    ],
                    group_rows,
                )),
            ),
            (
                "outliers".into(),
                Value::Table(Table::new(
                    vec![
                        "group_index".into(),
                        "group".into(),
                        "source_row".into(),
                        "value".into(),
                    ],
                    outlier_rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone)]
pub(super) struct EcdfPoint {
    pub(super) row: usize,
    pub(super) x: f64,
    pub(super) count: usize,
    pub(super) cumulative: usize,
    pub(super) fraction_before: f64,
    pub(super) fraction: f64,
}

pub(super) fn ecdf_geometry(value: &Value, who: &str) -> Result<(Vec<EcdfPoint>, usize, usize)> {
    let (mut values, dropped) = finite_numeric_list(value, who)?;
    values.sort_by(f64::total_cmp);
    let n = values.len();
    let mut points = Vec::new();
    let mut start = 0usize;
    while start < n {
        let x = values[start];
        let mut end = start + 1;
        while end < n && values[end] == x {
            end += 1;
        }
        points.push(EcdfPoint {
            row: points.len(),
            x,
            count: end - start,
            cumulative: end,
            fraction_before: start as f64 / n as f64,
            fraction: end as f64 / n as f64,
        });
        start = end;
    }
    Ok((points, n, dropped))
}

pub(super) fn builtin_ecdf_data(args: Vec<Value>) -> Result<Value> {
    let (points, n, dropped) = ecdf_geometry(&args[0], "ecdf_data")?;
    let rows = points
        .into_iter()
        .map(|point| {
            vec![
                Value::Int(point.row as i64),
                Value::Float(point.x),
                Value::Int(point.count as i64),
                Value::Int(point.cumulative as i64),
                Value::Float(point.fraction_before),
                Value::Float(point.fraction),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("ecdf".into())),
            ("n".into(), Value::Int(n as i64)),
            ("dropped_non_finite".into(), Value::Int(dropped as i64)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "row".into(),
                        "x".into(),
                        "count".into(),
                        "cumulative_count".into(),
                        "fraction_before".into(),
                        "fraction".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

pub(super) fn refined_normal_quantile(probability: f64) -> f64 {
    let mut estimate = bl_core::bio_core::stats_ops::normal_quantile(probability);
    // One Newton correction removes the final few ulps left by the fast
    // rational approximation used by the general qnorm builtin. Plotting
    // positions are deterministic and only O(n), so the extra CDF evaluation
    // is preferable to visibly asymmetric or oracle-dependent Q-Q tails.
    for _ in 0..2 {
        let density = (-0.5 * estimate * estimate).exp() / (2.0 * std::f64::consts::PI).sqrt();
        if density <= f64::MIN_POSITIVE {
            break;
        }
        estimate -= (bl_core::bio_core::stats_ops::normal_cdf(estimate) - probability) / density;
    }
    estimate
}

#[derive(Clone, Debug)]
pub(crate) struct NormalQqGeometry {
    pub(crate) probabilities: Vec<f64>,
    pub(crate) theoretical: Vec<f64>,
    pub(crate) sample: Vec<f64>,
    pub(crate) line_intercept: f64,
    pub(crate) line_slope: f64,
    pub(crate) dropped: usize,
}

/// Renderer-independent normal Q-Q coordinates using R's `ppoints()` rule and
/// the quartile line drawn by `qqline()`. Keeping this here makes the guided
/// statistics plot, diagnostics, and public geometry builtin use one declared
/// convention instead of three subtly different approximations.
pub(crate) fn normal_qq_geometry(values: &[f64]) -> Result<NormalQqGeometry> {
    let mut sample = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    let dropped = values.len() - sample.len();
    if sample.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "normal Q-Q geometry requires at least one finite value",
            None,
        ));
    }
    sample.sort_by(f64::total_cmp);
    let n = sample.len();
    let offset = if n <= 10 { 0.375 } else { 0.5 };
    let denominator = n as f64 + 1.0 - 2.0 * offset;
    let probabilities = (0..n)
        .map(|index| (index as f64 + 1.0 - offset) / denominator)
        .collect::<Vec<_>>();
    let theoretical = probabilities
        .iter()
        .map(|probability| refined_normal_quantile(*probability))
        .collect::<Vec<_>>();
    let sample_q1 = quantile_type7(&sample, 0.25);
    let sample_q3 = quantile_type7(&sample, 0.75);
    let theoretical_q1 = refined_normal_quantile(0.25);
    let theoretical_q3 = refined_normal_quantile(0.75);
    let line_slope = (sample_q3 - sample_q1) / (theoretical_q3 - theoretical_q1);
    let line_intercept = sample_q1 - line_slope * theoretical_q1;
    Ok(NormalQqGeometry {
        probabilities,
        theoretical,
        sample,
        line_intercept,
        line_slope,
        dropped,
    })
}

pub(super) fn builtin_normal_qq_data(args: Vec<Value>) -> Result<Value> {
    let (values, separately_dropped) = finite_numeric_list(&args[0], "normal_qq_data")?;
    let geometry = normal_qq_geometry(&values)?;
    let rows = geometry
        .sample
        .iter()
        .enumerate()
        .map(|(index, value)| {
            vec![
                Value::Int(index as i64),
                Value::Float(geometry.probabilities[index]),
                Value::Float(geometry.theoretical[index]),
                Value::Float(*value),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("normal_qq".into())),
            ("plotting_position".into(), Value::Str("R_ppoints".into())),
            ("n".into(), Value::Int(geometry.sample.len() as i64)),
            (
                "dropped_non_finite".into(),
                Value::Int((separately_dropped + geometry.dropped) as i64),
            ),
            (
                "line_intercept".into(),
                Value::Float(geometry.line_intercept),
            ),
            ("line_slope".into(), Value::Float(geometry.line_slope)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "row".into(),
                        "probability".into(),
                        "theoretical".into(),
                        "sample".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

pub(super) fn builtin_violin_data(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let (mut values, dropped) = finite_numeric_list(&args[0], "violin_data")?;
    values.sort_by(f64::total_cmp);
    let adjust = get_opt_f64(&opts, "adjust", 1.0);
    if !adjust.is_finite() || adjust <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin_data() adjust must be a positive finite number",
            None,
        ));
    }
    let points_number = get_opt_f64(&opts, "points", 256.0);
    if !points_number.is_finite()
        || points_number.fract() != 0.0
        || !(16.0..=4096.0).contains(&points_number)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin_data() points must be a whole number from 16 to 4096",
            None,
        ));
    }
    let points = points_number as usize;
    let bandwidth = match opts.get("bandwidth") {
        Some(value) => value
            .as_float()
            .filter(|number| number.is_finite() && *number > 0.0)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "violin_data() bandwidth must be a positive finite number",
                    None,
                )
            })?,
        None => silverman_bandwidth(&values) * adjust,
    };
    let density = gaussian_kde(&values, bandwidth, points);
    let peak = density.iter().map(|(_, value)| *value).fold(0.0, f64::max);
    let rows = density
        .into_iter()
        .enumerate()
        .map(|(index, (x, value))| {
            vec![
                Value::Int(index as i64),
                Value::Float(x),
                Value::Float(value),
                Value::Float(if peak > 0.0 { value / peak } else { 0.0 }),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("violin".into())),
            ("kernel".into(), Value::Str("gaussian".into())),
            ("bandwidth_method".into(), Value::Str("bw.nrd0".into())),
            ("bandwidth".into(), Value::Float(bandwidth)),
            ("n".into(), Value::Int(values.len() as i64)),
            ("dropped_non_finite".into(), Value::Int(dropped as i64)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec!["row".into(), "x".into(), "density".into(), "scaled".into()],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct LinearFitPoint {
    pub(crate) x: f64,
    pub(crate) fitted: f64,
    pub(crate) confidence_lower: f64,
    pub(crate) confidence_upper: f64,
    pub(crate) prediction_lower: f64,
    pub(crate) prediction_upper: f64,
}

#[derive(Clone, Debug)]
pub(crate) struct LinearFitGeometry {
    pub(crate) n: usize,
    pub(crate) slope: f64,
    pub(crate) intercept: f64,
    pub(crate) degrees_of_freedom: usize,
    pub(crate) residual_mse: f64,
    pub(crate) residual_standard_error: f64,
    pub(crate) confidence_level: f64,
    pub(crate) critical_value: f64,
    pub(crate) points: Vec<LinearFitPoint>,
}

/// Ordinary least-squares line geometry with intervals for the mean response
/// and for a new observation. These are deliberately distinct: a prediction
/// band contains the irreducible residual variance and must therefore be wider.
pub(crate) fn linear_fit_geometry(
    xs: &[f64],
    ys: &[f64],
    at: &[f64],
    confidence_level: f64,
) -> Result<LinearFitGeometry> {
    if xs.len() != ys.len() || xs.len() < 3 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear fit geometry requires at least three paired values",
            None,
        ));
    }
    if !confidence_level.is_finite() || !(0.0..1.0).contains(&confidence_level) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear_fit_data() confidence must be between 0 and 1",
            None,
        ));
    }
    if xs
        .iter()
        .chain(ys)
        .chain(at)
        .any(|value| !value.is_finite())
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear fit geometry requires finite numeric values",
            None,
        ));
    }
    let n = xs.len();
    let mean_x = xs.iter().sum::<f64>() / n as f64;
    let mean_y = ys.iter().sum::<f64>() / n as f64;
    let sum_xx = xs.iter().map(|x| (x - mean_x).powi(2)).sum::<f64>();
    if sum_xx <= f64::EPSILON {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "linear_fit_data() requires variation in x",
            None,
        ));
    }
    let sum_xy = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| (x - mean_x) * (y - mean_y))
        .sum::<f64>();
    let slope = sum_xy / sum_xx;
    let intercept = mean_y - slope * mean_x;
    let residual_sum_squares = xs
        .iter()
        .zip(ys)
        .map(|(x, y)| (y - (intercept + slope * x)).powi(2))
        .sum::<f64>();
    let degrees_of_freedom = n - 2;
    let residual_mse = residual_sum_squares / degrees_of_freedom as f64;
    let residual_standard_error = residual_mse.sqrt();
    let critical_value = bl_core::bio_core::stats_ops::students_t_quantile(
        0.5 + confidence_level / 2.0,
        degrees_of_freedom as f64,
    );
    let points = at
        .iter()
        .map(|x| {
            let fitted = intercept + slope * x;
            let mean_leverage = 1.0 / n as f64 + (x - mean_x).powi(2) / sum_xx;
            let confidence_margin = critical_value * (residual_mse * mean_leverage).sqrt();
            let prediction_margin = critical_value * (residual_mse * (1.0 + mean_leverage)).sqrt();
            LinearFitPoint {
                x: *x,
                fitted,
                confidence_lower: fitted - confidence_margin,
                confidence_upper: fitted + confidence_margin,
                prediction_lower: fitted - prediction_margin,
                prediction_upper: fitted + prediction_margin,
            }
        })
        .collect();
    Ok(LinearFitGeometry {
        n,
        slope,
        intercept,
        degrees_of_freedom,
        residual_mse,
        residual_standard_error,
        confidence_level,
        critical_value,
        points,
    })
}

pub(super) fn paired_finite_lists(
    x: &Value,
    y: &Value,
    who: &str,
) -> Result<(Vec<f64>, Vec<f64>, usize)> {
    let (Value::List(x_items), Value::List(y_items)) = (x, y) else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() requires two Lists"),
            None,
        ));
    };
    if x_items.len() != y_items.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() x and y must have equal length"),
            None,
        ));
    }
    let mut xs = Vec::with_capacity(x_items.len());
    let mut ys = Vec::with_capacity(y_items.len());
    let mut dropped = 0usize;
    for (x, y) in x_items.iter().zip(y_items.iter()) {
        match (x.as_float(), y.as_float()) {
            (Some(x), Some(y)) if x.is_finite() && y.is_finite() => {
                xs.push(x);
                ys.push(y);
            }
            _ => dropped += 1,
        }
    }
    Ok((xs, ys, dropped))
}

pub(super) fn builtin_linear_fit_data(args: Vec<Value>) -> Result<Value> {
    let opts = match args.get(2) {
        None | Some(Value::Nil) => HashMap::new(),
        Some(Value::Record(values)) => values.as_ref().clone(),
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "linear_fit_data() options must be a Record",
                None,
            ))
        }
    };
    let (xs, ys, dropped) = paired_finite_lists(&args[0], &args[1], "linear_fit_data")?;
    let confidence = get_opt_f64(&opts, "confidence", 0.95);
    let mut at = match opts.get("at") {
        Some(value) => finite_numeric_list(value, "linear_fit_data")?.0,
        None => {
            let mut values = xs.clone();
            values.sort_by(f64::total_cmp);
            values.dedup_by(|left, right| *left == *right);
            values
        }
    };
    at.sort_by(f64::total_cmp);
    let geometry = linear_fit_geometry(&xs, &ys, &at, confidence)?;
    let rows = geometry
        .points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            vec![
                Value::Int(index as i64),
                Value::Float(point.x),
                Value::Float(point.fitted),
                Value::Float(point.confidence_lower),
                Value::Float(point.confidence_upper),
                Value::Float(point.prediction_lower),
                Value::Float(point.prediction_upper),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("linear_fit".into())),
            ("n".into(), Value::Int(geometry.n as i64)),
            ("dropped_incomplete".into(), Value::Int(dropped as i64)),
            ("slope".into(), Value::Float(geometry.slope)),
            ("intercept".into(), Value::Float(geometry.intercept)),
            (
                "degrees_of_freedom".into(),
                Value::Int(geometry.degrees_of_freedom as i64),
            ),
            ("residual_mse".into(), Value::Float(geometry.residual_mse)),
            (
                "residual_standard_error".into(),
                Value::Float(geometry.residual_standard_error),
            ),
            (
                "confidence_level".into(),
                Value::Float(geometry.confidence_level),
            ),
            (
                "critical_value".into(),
                Value::Float(geometry.critical_value),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "row".into(),
                        "x".into(),
                        "fitted".into(),
                        "confidence_lower".into(),
                        "confidence_upper".into(),
                        "prediction_lower".into(),
                        "prediction_upper".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct CategoricalGeometry {
    pub(crate) labels: Vec<String>,
    pub(crate) counts: Vec<usize>,
    pub(crate) n_total: usize,
    pub(crate) n_observed: usize,
    pub(crate) missing: usize,
}

pub(super) fn categorical_label(value: &Value) -> Option<String> {
    match value {
        Value::Str(value) => Some(value.to_string()),
        Value::Int(value) => Some(value.to_string()),
        Value::Float(value) if value.is_finite() => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

/// First-observed categorical frequencies. The order is part of the geometry:
/// silently sorting labels changes which bar a reader associates with a group.
pub(crate) fn categorical_geometry(value: &Value, who: &str) -> Result<CategoricalGeometry> {
    let Value::List(items) = value else {
        return Err(BioLangError::type_error(
            format!("{who}() requires a List, got {}", value.type_of()),
            None,
        ));
    };
    let mut labels = Vec::new();
    let mut counts = Vec::new();
    let mut positions = HashMap::<String, usize>::new();
    let mut missing = 0usize;
    for item in items.iter() {
        if matches!(item, Value::Nil) {
            missing += 1;
            continue;
        }
        let Some(label) = categorical_label(item) else {
            return Err(BioLangError::type_error(
                format!("{who}() categories must be finite scalar values or Nil"),
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
    if labels.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() has no observed categories"),
            None,
        ));
    }
    Ok(CategoricalGeometry {
        labels,
        counts,
        n_total: items.len(),
        n_observed: items.len() - missing,
        missing,
    })
}

pub(super) fn builtin_categorical_data(args: Vec<Value>) -> Result<Value> {
    let geometry = categorical_geometry(&args[0], "categorical_data")?;
    let rows = geometry
        .labels
        .iter()
        .zip(&geometry.counts)
        .enumerate()
        .map(|(index, (label, count))| {
            vec![
                Value::Int(index as i64),
                Value::Str(label.clone().into()),
                Value::Int(*count as i64),
                Value::Float(*count as f64 / geometry.n_observed as f64),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("categorical".into())),
            ("ordering".into(), Value::Str("first_observed".into())),
            ("n_total".into(), Value::Int(geometry.n_total as i64)),
            ("n_observed".into(), Value::Int(geometry.n_observed as i64)),
            ("missing".into(), Value::Int(geometry.missing as i64)),
            (
                "data".into(),
                Value::Table(Table::new(
                    vec![
                        "category_index".into(),
                        "label".into(),
                        "count".into(),
                        "proportion".into(),
                    ],
                    rows,
                )),
            ),
        ])
        .into(),
    ))
}

#[derive(Clone, Debug)]
pub(crate) struct MissingnessCell {
    pub(crate) display_row: usize,
    pub(crate) display_column: usize,
    pub(crate) source_row: usize,
    pub(crate) source_column: usize,
    pub(crate) missing: bool,
}

#[derive(Clone, Debug)]
pub(crate) struct MissingnessGeometry {
    pub(crate) n_rows: usize,
    pub(crate) n_columns: usize,
    pub(crate) missing_cells: usize,
    pub(crate) row_stride: usize,
    pub(crate) column_stride: usize,
    pub(crate) displayed_rows: Vec<usize>,
    pub(crate) displayed_columns: Vec<usize>,
    pub(crate) column_missing: Vec<usize>,
    pub(crate) cells: Vec<MissingnessCell>,
}

pub(crate) fn value_is_missing(value: &Value) -> bool {
    matches!(value, Value::Nil) || matches!(value, Value::Float(number) if !number.is_finite())
}

/// Full missing counts plus a deterministic, bounded display grid. Counts use
/// every table cell; strides affect only the cells handed to a renderer.
pub(crate) fn missingness_geometry(
    table: &Table,
    max_rows: usize,
    max_columns: usize,
) -> MissingnessGeometry {
    let row_stride = table.rows.len().div_ceil(max_rows.max(1)).max(1);
    let column_stride = table.columns.len().div_ceil(max_columns.max(1)).max(1);
    let displayed_rows = (0..table.rows.len())
        .step_by(row_stride)
        .collect::<Vec<_>>();
    let displayed_columns = (0..table.columns.len())
        .step_by(column_stride)
        .collect::<Vec<_>>();
    let mut column_missing = vec![0usize; table.columns.len()];
    for row in &table.rows {
        for (column, missing) in column_missing.iter_mut().enumerate() {
            if value_is_missing(row.get(column).unwrap_or(&Value::Nil)) {
                *missing += 1;
            }
        }
    }
    let missing_cells = column_missing.iter().sum();
    let cells = displayed_rows
        .iter()
        .enumerate()
        .flat_map(|(display_row, source_row)| {
            displayed_columns
                .iter()
                .enumerate()
                .map(move |(display_column, source_column)| MissingnessCell {
                    display_row,
                    display_column,
                    source_row: *source_row,
                    source_column: *source_column,
                    missing: value_is_missing(
                        table.rows[*source_row]
                            .get(*source_column)
                            .unwrap_or(&Value::Nil),
                    ),
                })
        })
        .collect();
    MissingnessGeometry {
        n_rows: table.rows.len(),
        n_columns: table.columns.len(),
        missing_cells,
        row_stride,
        column_stride,
        displayed_rows,
        displayed_columns,
        column_missing,
        cells,
    }
}

pub(super) fn builtin_missingness_data(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "missingness_data")?;
    let opts = parse_options(&args);
    let max_rows = geometry_limit(&opts, "max_rows", 100, 10_000)?;
    let max_columns = geometry_limit(&opts, "max_columns", 40, 1_000)?;
    let geometry = missingness_geometry(table, max_rows, max_columns);
    let row_rows = geometry
        .displayed_rows
        .iter()
        .enumerate()
        .map(|(display_row, source_row)| {
            vec![
                Value::Int(display_row as i64),
                Value::Int(*source_row as i64),
            ]
        })
        .collect();
    let column_rows = geometry
        .displayed_columns
        .iter()
        .enumerate()
        .map(|(display_column, source_column)| {
            vec![
                Value::Int(display_column as i64),
                Value::Int(*source_column as i64),
                Value::Str(table.columns[*source_column].clone().into()),
            ]
        })
        .collect();
    let summary_rows = table
        .columns
        .iter()
        .enumerate()
        .map(|(source_column, name)| {
            let count = geometry.column_missing[source_column];
            vec![
                Value::Int(source_column as i64),
                Value::Str(name.clone().into()),
                Value::Int(count as i64),
                Value::Float(if geometry.n_rows == 0 {
                    0.0
                } else {
                    count as f64 / geometry.n_rows as f64
                }),
            ]
        })
        .collect();
    let cell_rows = geometry
        .cells
        .iter()
        .map(|cell| {
            vec![
                Value::Int(cell.display_row as i64),
                Value::Int(cell.display_column as i64),
                Value::Int(cell.source_row as i64),
                Value::Int(cell.source_column as i64),
                Value::Bool(cell.missing),
            ]
        })
        .collect();
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str("biolang.plot.geometry/v1".into()),
            ),
            ("kind".into(), Value::Str("missingness".into())),
            ("n_rows".into(), Value::Int(geometry.n_rows as i64)),
            ("n_columns".into(), Value::Int(geometry.n_columns as i64)),
            (
                "missing_cells".into(),
                Value::Int(geometry.missing_cells as i64),
            ),
            ("row_stride".into(), Value::Int(geometry.row_stride as i64)),
            (
                "column_stride".into(),
                Value::Int(geometry.column_stride as i64),
            ),
            (
                "displayed_rows".into(),
                Value::Table(Table::new(
                    vec!["display_row".into(), "source_row".into()],
                    row_rows,
                )),
            ),
            (
                "displayed_columns".into(),
                Value::Table(Table::new(
                    vec![
                        "display_column".into(),
                        "source_column".into(),
                        "column".into(),
                    ],
                    column_rows,
                )),
            ),
            (
                "column_summary".into(),
                Value::Table(Table::new(
                    vec![
                        "source_column".into(),
                        "column".into(),
                        "missing_count".into(),
                        "missing_fraction".into(),
                    ],
                    summary_rows,
                )),
            ),
            (
                "cells".into(),
                Value::Table(Table::new(
                    vec![
                        "display_row".into(),
                        "display_column".into(),
                        "source_row".into(),
                        "source_column".into(),
                        "missing".into(),
                    ],
                    cell_rows,
                )),
            ),
        ])
        .into(),
    ))
}

pub(super) fn geometry_limit(
    opts: &HashMap<String, Value>,
    key: &str,
    default: usize,
    maximum: usize,
) -> Result<usize> {
    let Some(value) = opts.get(key) else {
        return Ok(default);
    };
    let number = match value {
        Value::Int(value) if *value > 0 => *value as usize,
        Value::Float(value) if value.is_finite() && *value >= 1.0 && value.fract() == 0.0 => {
            *value as usize
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("missingness_data() option '{key}' must be a positive whole number"),
                None,
            ))
        }
    };
    if number > maximum {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("missingness_data() option '{key}' exceeds the safety limit of {maximum}"),
            None,
        ));
    }
    Ok(number)
}

/// The empirical cumulative distribution: for each value, the fraction of the
/// data at or below it.
///
/// Drawn as the step function it actually is rather than joined with straight
/// lines, because the distribution really is flat between observations. Unlike
/// a histogram it has no bin width, so it shows the data without a parameter
/// that changes the story being told.
pub(super) fn builtin_ecdf_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Empirical CDF").to_string();

    let (geometry, _, _) = ecdf_geometry(&args[0], "ecdf_plot")?;
    let x_values = geometry.iter().map(|point| point.x).collect::<Vec<_>>();
    let (lo, hi) = col_range(&x_values);
    let span = if (hi - lo).abs() < f64::EPSILON {
        1.0
    } else {
        hi - lo
    };

    let mut canvas = SvgCanvas::new(width, height);
    let right_edge = canvas.margin.left + canvas.plot_width();
    let x_scale = Scale {
        domain: (lo, lo + span),
        range: (canvas.margin.left, right_edge),
    };
    let y_scale = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // One polyline rather than two line elements per observation: the same
    // picture, and a tenth of the file for a column of a few hundred values,
    // which matters when the SVG is inlined into a page.
    let mut points = Vec::with_capacity(2 * geometry.len() + 2);
    points.push(format!(
        "{:.1},{:.1}",
        x_scale.map(geometry[0].x),
        y_scale.map(0.0)
    ));
    for (index, point) in geometry.iter().enumerate() {
        let x = x_scale.map(point.x);
        let y = y_scale.map(point.fraction);
        // The riser at the observation, then the flat run to the next one.
        points.push(format!("{x:.1},{y:.1}"));
        let next_x = match geometry.get(index + 1) {
            Some(next) => x_scale.map(next.x),
            None => right_edge,
        };
        points.push(format!("{next_x:.1},{y:.1}"));
    }
    canvas.elements.push(format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="1.5" />"#,
        points.join(" "),
        PALETTE[0]
    ));

    canvas.draw_x_axis(
        &Scale {
            domain: (lo, lo + span),
            range: (lo, lo + span),
        },
        &axis_label(&opts, "xlabel", "Value"),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        },
        &axis_label(&opts, "ylabel", "Proportion at or below"),
    );
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}

/// Silverman's rule of thumb for a kernel bandwidth: the width R's `bw.nrd0`
/// picks, computed the same way so the two agree.
///
/// `0.9 * min(sd, IQR/1.34) * n^(-1/5)`. The `min` is what keeps a long tail
/// from inflating the standard deviation and oversmoothing everything else,
/// and the IQR falls back to the sd when the middle half of the data is a
/// single repeated value. Input order must not affect a density estimate, so
/// this function sorts its own copy before taking quartiles.
pub(crate) fn silverman_bandwidth(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let values = sorted.as_slice();
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let sd = if values.len() > 1 {
        (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt()
    } else {
        1.0
    };
    // Type 7, matching quantile() elsewhere in the runtime and R's default.
    let quantile = |p: f64| -> f64 {
        let h = (n - 1.0) * p;
        let lower = h.floor() as usize;
        let upper = (lower + 1).min(values.len() - 1);
        values[lower] + (h - h.floor()) * (values[upper] - values[lower])
    };
    let iqr = quantile(0.75) - quantile(0.25);
    // R's fallback chain, in R's order: the IQR estimate, then the standard
    // deviation, then the magnitude of a single observation, then 1. Each step
    // exists because the one before it can be exactly zero -- on a column of
    // repeated values every measure of spread is -- and a bandwidth of zero
    // divides by zero and draws nothing.
    let mut spread = sd.min(iqr / 1.34);
    if spread <= 0.0 {
        spread = sd;
    }
    if spread <= 0.0 {
        spread = values[0].abs();
    }
    if spread <= 0.0 {
        spread = 1.0;
    }
    0.9 * spread * n.powf(-0.2)
}

pub(crate) fn gaussian_kde(values: &[f64], bandwidth: f64, steps: usize) -> Vec<(f64, f64)> {
    let bandwidth = bandwidth.max(f64::MIN_POSITIVE);
    let (data_lo, data_hi) = col_range(values);
    let lo = data_lo - 3.0 * bandwidth;
    let hi = data_hi + 3.0 * bandwidth;
    gaussian_kde_between(values, bandwidth, steps, lo, hi)
}

/// Evaluate the same Gaussian KDE on an explicit range. `geom_violin()` uses
/// the observed group range when its default `trim = TRUE` is active, whereas
/// a standalone density curve conventionally includes the kernel tails.
pub(crate) fn gaussian_kde_between(
    values: &[f64],
    bandwidth: f64,
    steps: usize,
    lo: f64,
    hi: f64,
) -> Vec<(f64, f64)> {
    let bandwidth = bandwidth.max(f64::MIN_POSITIVE);
    let steps = steps.max(2);
    let normaliser = 1.0 / (values.len() as f64 * bandwidth * (2.0 * std::f64::consts::PI).sqrt());
    (0..steps)
        .map(|step| {
            let x = lo + (hi - lo) * step as f64 / (steps - 1) as f64;
            let density = values
                .iter()
                .map(|value| {
                    let z = (x - value) / bandwidth;
                    (-0.5 * z * z).exp()
                })
                .sum::<f64>()
                * normaliser;
            (x, density)
        })
        .collect()
}

/// A Gaussian kernel density estimate: a smooth stand-in for a histogram that
/// does not depend on where the bin edges happen to fall.
///
/// The default bandwidth is Silverman's rule of thumb,
/// `0.9 * min(sd, IQR/1.34) * n^(-1/5)`, which is what R's `bw.nrd0` computes,
/// so the two agree by construction. Pass `bandwidth` to override it - and do
/// look at more than one, because bandwidth is to a density what bin width is
/// to a histogram: a choice that changes the shape being argued for.
pub(super) fn builtin_density_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Density").to_string();

    let mut values = numeric_list(&args[0], "density_plot")?;
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let bandwidth =
        get_opt_f64(&opts, "bandwidth", silverman_bandwidth(&values)).max(f64::MIN_POSITIVE);

    let steps = 256usize;
    let densities = gaussian_kde(&values, bandwidth, steps);
    let lo = densities[0].0;
    let hi = densities[densities.len() - 1].0;
    let peak = densities
        .iter()
        .map(|(_, d)| *d)
        .fold(0.0f64, f64::max)
        .max(f64::MIN_POSITIVE);

    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale {
        domain: (lo, hi),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, peak),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    let points: Vec<String> = densities
        .iter()
        .map(|(x, d)| format!("{:.1},{:.1}", x_scale.map(*x), y_scale.map(*d)))
        .collect();
    canvas.elements.push(format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
        points.join(" "),
        PALETTE[0]
    ));

    canvas.draw_x_axis(
        &Scale {
            domain: (lo, hi),
            range: (lo, hi),
        },
        &axis_label(&opts, "xlabel", "Value"),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, peak),
            range: (0.0, peak),
        },
        &axis_label(&opts, "ylabel", "Density"),
    );
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}
