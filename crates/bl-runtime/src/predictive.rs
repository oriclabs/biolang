//! Reproducible, browser-safe predictive modelling helpers.
//!
//! The public API deliberately stores the training table and selected tuning
//! parameters in ordinary BioLang records. Models can therefore cross a
//! notebook-cell boundary, survive JSON export, and be refitted deterministically
//! when prediction is requested; no opaque native pointer leaks into `Value`.

use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Table, Value};
use chrono::Datelike;
use smartcore::ensemble::random_forest_classifier::{
    RandomForestClassifier, RandomForestClassifierParameters,
};
use smartcore::linalg::basic::matrix::DenseMatrix;
use smartcore::xgboost::{XGRegressor, XGRegressorParameters};
use std::collections::HashMap;

use crate::plot::{Scale, SvgCanvas};

pub(crate) fn call(name: &str, args: Vec<Value>) -> Result<Value> {
    match name {
        "predictive_train" => train(args),
        "predictive_predict" => predict(args),
        "predictive_compare" => compare(args),
        "predictive_importance_plot" => importance_plot(args),
        "predictive_resample_plot" => resample_plot(args),
        "seasonal_forecast" => seasonal_forecast(args),
        "seasonal_forecast_plot" => seasonal_forecast_plot(args),
        "seasonal_components_plot" => seasonal_components_plot(args),
        "grouped_density_plot" => grouped_density_plot(args),
        "grouped_bar_plot" => grouped_bar_plot(args),
        "grouped_boxplot_plot" => grouped_boxplot_plot(args),
        "event_timeline_plot" => event_timeline_plot(args),
        _ => Err(type_error(format!("unknown predictive builtin: {name}"))),
    }
}

fn type_error(message: impl Into<String>) -> BioLangError {
    BioLangError::runtime(ErrorKind::TypeError, message.into(), None)
}

fn record(fields: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    Value::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.into(), value))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn list(values: impl IntoIterator<Item = Value>) -> Value {
    Value::List(values.into_iter().collect::<Vec<_>>().into())
}

fn options(args: &[Value], index: usize, function: &str) -> Result<HashMap<String, Value>> {
    match args.get(index) {
        None => Ok(HashMap::new()),
        Some(Value::Record(values)) => Ok(values.as_ref().clone()),
        Some(other) => Err(type_error(format!(
            "{function}() options must be Record, got {}",
            other.type_of()
        ))),
    }
}

fn option_str<'a>(options: &'a HashMap<String, Value>, key: &str, default: &'a str) -> &'a str {
    options.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn option_usize(options: &HashMap<String, Value>, key: &str, default: usize) -> usize {
    options
        .get(key)
        .and_then(Value::as_int)
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(default)
}

fn option_f64(options: &HashMap<String, Value>, key: &str, default: f64) -> f64 {
    options
        .get(key)
        .and_then(Value::as_float)
        .unwrap_or(default)
}

fn option_strings(options: &HashMap<String, Value>, key: &str) -> Result<Option<Vec<String>>> {
    let Some(value) = options.get(key) else {
        return Ok(None);
    };
    let Value::List(values) = value else {
        return Err(type_error(format!("option '{key}' must be List[Str]")));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| type_error(format!("option '{key}' must contain only strings")))
        })
        .collect::<Result<Vec<_>>>()
        .map(Some)
}

fn numeric(value: &Value) -> Option<f64> {
    match value {
        Value::Int(value) => Some(*value as f64),
        Value::Float(value) if value.is_finite() => Some(*value),
        Value::Bool(value) => Some(if *value { 1.0 } else { 0.0 }),
        _ => None,
    }
}

/// Expand a finite data range so points, lines, ribbons, and outliers do not
/// sit on the panel outline. This is deliberately a data-domain expansion,
/// not an SVG margin: the breathing room therefore survives responsive
/// scaling and PNG/PDF export.
fn padded_domain(low: f64, high: f64, fraction: f64, minimum: f64) -> (f64, f64) {
    let span = (high - low).abs();
    let padding = (span * fraction).max(minimum);
    (low - padding, high + padding)
}

#[derive(Clone)]
struct Dataset {
    x: Vec<Vec<f64>>,
    y: Vec<usize>,
    classes: Vec<String>,
    predictors: Vec<String>,
}

fn predictor_names(
    table: &Table,
    target: &str,
    requested: Option<Vec<String>>,
    function: &str,
) -> Result<Vec<String>> {
    if table.col_index(target).is_none() {
        return Err(type_error(format!(
            "{function}() target column '{target}' was not found"
        )));
    }
    let predictors = requested.unwrap_or_else(|| {
        table
            .columns
            .iter()
            .enumerate()
            .filter(|(_, name)| name.as_str() != target && name.as_str() != "case_id")
            .filter(|(index, _)| {
                table.rows.iter().all(|row| {
                    row.get(*index).is_some_and(|value| {
                        matches!(value, Value::Nil) || numeric(value).is_some()
                    })
                })
            })
            .map(|(_, name)| name.clone())
            .collect()
    });
    if predictors.is_empty() {
        return Err(type_error(format!(
            "{function}() found no numeric predictors"
        )));
    }
    for name in &predictors {
        if name == target || table.col_index(name).is_none() {
            return Err(type_error(format!(
                "{function}() predictor column '{name}' is invalid"
            )));
        }
    }
    Ok(predictors)
}

fn training_dataset(table: &Table, target: &str, predictors: Vec<String>) -> Result<Dataset> {
    let target_index = table.col_index(target).unwrap();
    let predictor_indices = predictors
        .iter()
        .map(|name| table.col_index(name).unwrap())
        .collect::<Vec<_>>();
    let mut class_names = table
        .rows
        .iter()
        .filter_map(|row| row.get(target_index))
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect::<Vec<_>>();
    class_names.sort();
    class_names.dedup();
    if class_names.len() != 2 {
        return Err(type_error(format!(
            "predictive_train() currently requires exactly two target classes, found {}",
            class_names.len()
        )));
    }

    let mut x = Vec::new();
    let mut y = Vec::new();
    for (row_number, row) in table.rows.iter().enumerate() {
        let Some(class) = row.get(target_index).and_then(Value::as_str) else {
            continue;
        };
        let mut values = Vec::with_capacity(predictor_indices.len());
        for (column_number, index) in predictor_indices.iter().enumerate() {
            let Some(value) = row.get(*index).and_then(numeric) else {
                return Err(type_error(format!(
                    "predictive_train() found a missing/non-numeric value at row {}, predictor '{}'; impute or remove it first",
                    row_number + 1,
                    predictors[column_number]
                )));
            };
            values.push(value);
        }
        x.push(values);
        y.push(usize::from(class == class_names[1]));
    }
    if x.len() < 6 {
        return Err(type_error(
            "predictive_train() needs at least six complete labelled rows",
        ));
    }
    Ok(Dataset {
        x,
        y,
        classes: class_names,
        predictors,
    })
}

fn prediction_matrix(
    table: &Table,
    predictors: &[String],
    function: &str,
) -> Result<Vec<Vec<f64>>> {
    let indices = predictors
        .iter()
        .map(|name| {
            table
                .col_index(name)
                .ok_or_else(|| type_error(format!("{function}() is missing predictor '{name}'")))
        })
        .collect::<Result<Vec<_>>>()?;
    table
        .rows
        .iter()
        .enumerate()
        .map(|(row_number, row)| {
            indices
                .iter()
                .enumerate()
                .map(|(column_number, index)| {
                    row.get(*index).and_then(numeric).ok_or_else(|| {
                        type_error(format!(
                            "{function}() found a missing/non-numeric value at row {}, predictor '{}'",
                            row_number + 1,
                            predictors[column_number]
                        ))
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct Tune {
    first: f64,
    second: f64,
}

fn tune_label(method: &str, tune: Tune) -> String {
    match method {
        "random_forest" => format!("mtry={}", tune.first as usize),
        "gradient_boosting" => format!(
            "depth={}, trees={}",
            tune.first as usize, tune.second as usize
        ),
        "elastic_net" => format!("alpha={:.2}, lambda={:.6}", tune.first, tune.second),
        "knn" => format!("k={}", tune.first as usize),
        _ => method.to_string(),
    }
}

fn canonical_method(method: &str) -> Result<&'static str> {
    match method {
        "rf" | "random_forest" | "random-forest" => Ok("random_forest"),
        "gbm" | "gradient_boosting" | "gradient-boosting" | "xgboost" => {
            Ok("gradient_boosting")
        }
        "glmnet" | "elastic_net" | "elastic-net" | "lasso" => Ok("elastic_net"),
        "knn" | "k_nearest_neighbors" | "k-nearest-neighbors" => Ok("knn"),
        other => Err(type_error(format!(
            "unknown predictive method '{other}'; use random_forest, gradient_boosting, elastic_net, or knn"
        ))),
    }
}

fn tuning_grid(method: &str, predictors: usize, samples: usize) -> Vec<Tune> {
    match method {
        "random_forest" => {
            let low = (predictors / 5).max(1);
            let mid = (low + predictors).div_ceil(2);
            vec![
                Tune {
                    first: low as f64,
                    second: 0.0,
                },
                Tune {
                    first: mid as f64,
                    second: 0.0,
                },
                Tune {
                    first: predictors as f64,
                    second: 0.0,
                },
            ]
        }
        "gradient_boosting" => [1.0, 2.0, 3.0]
            .into_iter()
            .flat_map(|depth| {
                [50.0, 100.0, 150.0].into_iter().map(move |trees| Tune {
                    first: depth,
                    second: trees,
                })
            })
            .collect(),
        "elastic_net" => [0.10, 0.55, 1.00]
            .into_iter()
            .flat_map(|alpha| {
                [0.000391, 0.003908, 0.039077]
                    .into_iter()
                    .map(move |lambda| Tune {
                        first: alpha,
                        second: lambda,
                    })
            })
            .collect(),
        "knn" => [5usize, 7, 9]
            .into_iter()
            .map(|k| Tune {
                first: k.min(samples.saturating_sub(1).max(1)) as f64,
                second: 0.0,
            })
            .collect(),
        _ => unreachable!(),
    }
}

#[derive(Clone)]
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn index(&mut self, length: usize) -> usize {
        (self.next_u64() % length as u64) as usize
    }
}

fn bootstrap_splits(
    samples: usize,
    repetitions: usize,
    seed: u64,
) -> Vec<(Vec<usize>, Vec<usize>)> {
    let mut rng = Lcg::new(seed);
    let mut result = Vec::with_capacity(repetitions);
    for _ in 0..repetitions {
        loop {
            let train = (0..samples).map(|_| rng.index(samples)).collect::<Vec<_>>();
            let mut seen = vec![false; samples];
            for index in &train {
                seen[*index] = true;
            }
            let test = seen
                .iter()
                .enumerate()
                .filter_map(|(index, present)| (!present).then_some(index))
                .collect::<Vec<_>>();
            if !test.is_empty() {
                result.push((train, test));
                break;
            }
        }
    }
    result
}

fn subset<T: Clone>(values: &[T], indices: &[usize]) -> Vec<T> {
    indices.iter().map(|index| values[*index].clone()).collect()
}

fn dense(values: &[Vec<f64>]) -> Result<DenseMatrix<f64>> {
    DenseMatrix::from_2d_vec(&values.to_vec())
        .map_err(|error| type_error(format!("could not build model matrix: {error}")))
}

fn random_forest_classes(
    train_x: &[Vec<f64>],
    train_y: &[usize],
    test_x: &[Vec<f64>],
    mtry: usize,
    trees: usize,
    seed: u64,
) -> Result<Vec<usize>> {
    let train = dense(train_x)?;
    let test = dense(test_x)?;
    let labels = train_y
        .iter()
        .map(|value| *value as i32)
        .collect::<Vec<_>>();
    let parameters = RandomForestClassifierParameters::default()
        .with_m(mtry.max(1).min(train_x[0].len()))
        .with_n_trees(trees.clamp(1, u16::MAX as usize) as u16)
        .with_seed(seed);
    let model = RandomForestClassifier::fit(&train, &labels, parameters)
        .map_err(|error| type_error(format!("random forest fitting failed: {error}")))?;
    model
        .predict(&test)
        .map_err(|error| type_error(format!("random forest prediction failed: {error}")))
        .map(|values| values.into_iter().map(|value| value as usize).collect())
}

fn random_forest_probabilities(
    train_x: &[Vec<f64>],
    train_y: &[usize],
    test_x: &[Vec<f64>],
    mtry: usize,
    trees: usize,
    seed: u64,
) -> Result<Vec<f64>> {
    // SmartCore intentionally exposes majority-vote classes rather than vote
    // proportions. Fitting deterministically seeded one-tree forests gives the
    // same bagged-tree vote quantity without reaching into private internals.
    let mut positive_votes = vec![0usize; test_x.len()];
    for tree in 0..trees.max(1) {
        let predictions = random_forest_classes(
            train_x,
            train_y,
            test_x,
            mtry,
            1,
            seed.wrapping_add(tree as u64 * 7919),
        )?;
        for (index, prediction) in predictions.into_iter().enumerate() {
            positive_votes[index] += prediction;
        }
    }
    Ok(positive_votes
        .into_iter()
        .map(|votes| votes as f64 / trees.max(1) as f64)
        .collect())
}

fn boosted_probabilities(
    train_x: &[Vec<f64>],
    train_y: &[usize],
    test_x: &[Vec<f64>],
    depth: usize,
    estimators: usize,
    seed: u64,
) -> Result<Vec<f64>> {
    let train = dense(train_x)?;
    let test = dense(test_x)?;
    let labels = train_y
        .iter()
        .map(|value| *value as f64)
        .collect::<Vec<_>>();
    let parameters = XGRegressorParameters::default()
        .with_max_depth(depth.clamp(1, u16::MAX as usize) as u16)
        .with_n_estimators(estimators.max(1))
        .with_learning_rate(0.1)
        .with_min_child_weight(10.min(train_x.len().saturating_sub(1).max(1)))
        .with_subsample(0.8)
        .with_seed(seed);
    let model = XGRegressor::fit(&train, &labels, parameters)
        .map_err(|error| type_error(format!("gradient boosting fitting failed: {error}")))?;
    model
        .predict(&test)
        .map_err(|error| type_error(format!("gradient boosting prediction failed: {error}")))
        .map(|values| {
            values
                .into_iter()
                .map(|value| value.clamp(0.0, 1.0))
                .collect()
        })
}

fn standardization(x: &[Vec<f64>]) -> (Vec<f64>, Vec<f64>) {
    let columns = x[0].len();
    let mut means = vec![0.0; columns];
    for row in x {
        for (column, value) in row.iter().enumerate() {
            means[column] += value;
        }
    }
    for mean in &mut means {
        *mean /= x.len() as f64;
    }
    let mut scales = vec![0.0; columns];
    for row in x {
        for (column, value) in row.iter().enumerate() {
            scales[column] += (value - means[column]).powi(2);
        }
    }
    for scale in &mut scales {
        *scale = (*scale / x.len().saturating_sub(1).max(1) as f64).sqrt();
        if *scale <= f64::EPSILON {
            *scale = 1.0;
        }
    }
    (means, scales)
}

fn standardize(x: &[Vec<f64>], means: &[f64], scales: &[f64]) -> Vec<Vec<f64>> {
    x.iter()
        .map(|row| {
            row.iter()
                .enumerate()
                .map(|(column, value)| (value - means[column]) / scales[column])
                .collect()
        })
        .collect()
}

fn elastic_net_coefficients(
    train_x: &[Vec<f64>],
    train_y: &[usize],
    alpha: f64,
    lambda: f64,
) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (means, scales) = standardization(train_x);
    let x = standardize(train_x, &means, &scales);
    let mut beta = vec![0.0; x[0].len() + 1];
    let rate = 0.08;
    for _ in 0..2_000 {
        let mut gradient = vec![0.0; beta.len()];
        for (row, outcome) in x.iter().zip(train_y) {
            let eta = beta[0]
                + row
                    .iter()
                    .zip(&beta[1..])
                    .map(|(value, coefficient)| value * coefficient)
                    .sum::<f64>();
            let probability = 1.0 / (1.0 + (-eta.clamp(-35.0, 35.0)).exp());
            let error = probability - *outcome as f64;
            gradient[0] += error;
            for column in 0..row.len() {
                gradient[column + 1] += error * row[column];
            }
        }
        let n = x.len() as f64;
        let mut largest = 0.0f64;
        beta[0] -= rate * gradient[0] / n;
        for column in 1..beta.len() {
            let before = beta[column];
            let value = before - rate * (gradient[column] / n + lambda * (1.0 - alpha) * before);
            let threshold = rate * lambda * alpha;
            beta[column] = value.signum() * (value.abs() - threshold).max(0.0);
            largest = largest.max((beta[column] - before).abs());
        }
        if largest < 1e-9 {
            break;
        }
    }
    (beta, means, scales)
}

fn elastic_net_probabilities(
    train_x: &[Vec<f64>],
    train_y: &[usize],
    test_x: &[Vec<f64>],
    alpha: f64,
    lambda: f64,
) -> Vec<f64> {
    let (beta, means, scales) = elastic_net_coefficients(train_x, train_y, alpha, lambda);
    standardize(test_x, &means, &scales)
        .iter()
        .map(|row| {
            let eta = beta[0]
                + row
                    .iter()
                    .zip(&beta[1..])
                    .map(|(value, coefficient)| value * coefficient)
                    .sum::<f64>();
            1.0 / (1.0 + (-eta.clamp(-35.0, 35.0)).exp())
        })
        .collect()
}

fn knn_probabilities(
    train_x: &[Vec<f64>],
    train_y: &[usize],
    test_x: &[Vec<f64>],
    k: usize,
) -> Vec<f64> {
    test_x
        .iter()
        .map(|row| {
            let mut neighbours = train_x
                .iter()
                .zip(train_y)
                .map(|(candidate, outcome)| {
                    let distance = row
                        .iter()
                        .zip(candidate)
                        .map(|(left, right)| (left - right).powi(2))
                        .sum::<f64>();
                    (distance, *outcome)
                })
                .collect::<Vec<_>>();
            neighbours.sort_by(|left, right| left.0.total_cmp(&right.0));
            let selected = neighbours.iter().take(k.max(1).min(neighbours.len()));
            let mut weighted_positive = 0.0;
            let mut total_weight = 0.0;
            for (distance, outcome) in selected {
                let weight = 1.0 / distance.sqrt().max(1e-9);
                weighted_positive += weight * *outcome as f64;
                total_weight += weight;
            }
            if total_weight > 0.0 {
                weighted_positive / total_weight
            } else {
                0.5
            }
        })
        .collect()
}

fn probabilities(
    method: &str,
    train_x: &[Vec<f64>],
    train_y: &[usize],
    test_x: &[Vec<f64>],
    tune: Tune,
    trees: usize,
    seed: u64,
) -> Result<Vec<f64>> {
    match method {
        "random_forest" => {
            random_forest_probabilities(train_x, train_y, test_x, tune.first as usize, trees, seed)
        }
        "gradient_boosting" => boosted_probabilities(
            train_x,
            train_y,
            test_x,
            tune.first as usize,
            tune.second as usize,
            seed,
        ),
        "elastic_net" => Ok(elastic_net_probabilities(
            train_x,
            train_y,
            test_x,
            tune.first,
            tune.second,
        )),
        "knn" => Ok(knn_probabilities(
            train_x,
            train_y,
            test_x,
            tune.first as usize,
        )),
        _ => unreachable!(),
    }
}

fn classes_from_probabilities(values: &[f64]) -> Vec<usize> {
    values
        .iter()
        .map(|value| usize::from(*value >= 0.5))
        .collect()
}

fn accuracy(expected: &[usize], predicted: &[usize]) -> f64 {
    expected
        .iter()
        .zip(predicted)
        .filter(|(left, right)| left == right)
        .count() as f64
        / expected.len().max(1) as f64
}

fn kappa(expected: &[usize], predicted: &[usize]) -> f64 {
    let observed = accuracy(expected, predicted);
    let n = expected.len().max(1) as f64;
    let expected_positive = expected.iter().filter(|value| **value == 1).count() as f64 / n;
    let predicted_positive = predicted.iter().filter(|value| **value == 1).count() as f64 / n;
    let chance = expected_positive * predicted_positive
        + (1.0 - expected_positive) * (1.0 - predicted_positive);
    if (1.0 - chance).abs() <= f64::EPSILON {
        0.0
    } else {
        (observed - chance) / (1.0 - chance)
    }
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len().max(1) as f64
}

fn train(args: Vec<Value>) -> Result<Value> {
    let Value::Table(table) = &args[0] else {
        return Err(type_error(format!(
            "predictive_train() requires Table, got {}",
            args[0].type_of()
        )));
    };
    let opts = options(&args, 1, "predictive_train")?;
    let target = option_str(&opts, "target", "outcome").to_string();
    let method = canonical_method(option_str(&opts, "method", "random_forest"))?;
    let predictors = predictor_names(
        table,
        &target,
        option_strings(&opts, "predictors")?,
        "predictive_train",
    )?;
    let data = training_dataset(table, &target, predictors)?;
    let seed = option_usize(&opts, "seed", 8382) as u64;
    let repetitions = option_usize(&opts, "resamples", 25).clamp(3, 200);
    let trees = option_usize(&opts, "trees", 100).clamp(10, 1_000);
    let splits = bootstrap_splits(data.x.len(), repetitions, seed);
    let grid = tuning_grid(method, data.predictors.len(), data.x.len());

    let mut grid_rows = Vec::with_capacity(grid.len());
    let mut best_tune = grid[0];
    let mut best_accuracy = f64::NEG_INFINITY;
    let mut best_kappa = f64::NEG_INFINITY;
    let mut best_resamples = Vec::new();
    for tune in grid {
        let mut accuracies = Vec::with_capacity(splits.len());
        let mut kappas = Vec::with_capacity(splits.len());
        for (repetition, (train_indices, test_indices)) in splits.iter().enumerate() {
            let train_x = subset(&data.x, train_indices);
            let train_y = subset(&data.y, train_indices);
            let test_x = subset(&data.x, test_indices);
            let expected = subset(&data.y, test_indices);
            let model_trees = if method == "random_forest" { trees } else { 1 };
            let probabilities = probabilities(
                method,
                &train_x,
                &train_y,
                &test_x,
                tune,
                model_trees,
                seed.wrapping_add(repetition as u64 * 104_729),
            )?;
            let predicted = classes_from_probabilities(&probabilities);
            accuracies.push(accuracy(&expected, &predicted));
            kappas.push(kappa(&expected, &predicted));
        }
        let mean_accuracy = mean(&accuracies);
        let mean_kappa = mean(&kappas);
        grid_rows.push(vec![
            Value::Str(tune_label(method, tune)),
            Value::Float(mean_accuracy),
            Value::Float(mean_kappa),
        ]);
        if mean_accuracy > best_accuracy
            || (mean_accuracy == best_accuracy && mean_kappa > best_kappa)
        {
            best_tune = tune;
            best_accuracy = mean_accuracy;
            best_kappa = mean_kappa;
            best_resamples = accuracies.into_iter().zip(kappas).collect();
        }
    }

    let resample_rows = best_resamples
        .iter()
        .enumerate()
        .map(|(index, (accuracy, kappa))| {
            vec![
                Value::Int((index + 1) as i64),
                Value::Float(*accuracy),
                Value::Float(*kappa),
            ]
        })
        .collect::<Vec<_>>();

    let importance = permutation_importance(method, &data, &splits, best_tune, trees, seed)?;
    let parameters = record([
        ("first", Value::Float(best_tune.first)),
        ("second", Value::Float(best_tune.second)),
        ("label", Value::Str(tune_label(method, best_tune))),
        ("trees", Value::Int(trees as i64)),
    ]);
    let metrics = record([
        ("accuracy", Value::Float(best_accuracy)),
        ("kappa", Value::Float(best_kappa)),
        ("resamples", Value::Int(repetitions as i64)),
        ("samples", Value::Int(data.x.len() as i64)),
        ("predictor_count", Value::Int(data.predictors.len() as i64)),
    ]);
    Ok(record([
        ("schema", Value::Str("biolang.predictive-model.v1".into())),
        ("method", Value::Str(method.into())),
        ("target", Value::Str(target)),
        (
            "predictors",
            list(data.predictors.iter().cloned().map(Value::Str)),
        ),
        (
            "classes",
            list(data.classes.iter().cloned().map(Value::Str)),
        ),
        ("seed", Value::Int(seed as i64)),
        ("parameters", parameters),
        ("metrics", metrics),
        (
            "tuning",
            Value::Table(Table::new(
                vec!["parameters".into(), "accuracy".into(), "kappa".into()],
                grid_rows,
            )),
        ),
        (
            "resamples",
            Value::Table(Table::new(
                vec!["resample".into(), "accuracy".into(), "kappa".into()],
                resample_rows,
            )),
        ),
        ("importance", importance),
        ("training", Value::Table(table.clone())),
    ]))
}

fn permutation_importance(
    method: &str,
    data: &Dataset,
    splits: &[(Vec<usize>, Vec<usize>)],
    tune: Tune,
    trees: usize,
    seed: u64,
) -> Result<Value> {
    let considered = splits.len().min(10);
    let mut losses = vec![0.0; data.predictors.len()];
    for (repetition, (train_indices, test_indices)) in splits.iter().take(considered).enumerate() {
        let train_x = subset(&data.x, train_indices);
        let train_y = subset(&data.y, train_indices);
        let test_x = subset(&data.x, test_indices);
        let expected = subset(&data.y, test_indices);
        let model_trees = if method == "random_forest" { trees } else { 1 };
        let baseline = probabilities(
            method,
            &train_x,
            &train_y,
            &test_x,
            tune,
            model_trees,
            seed.wrapping_add(repetition as u64 * 104_729),
        )?;
        let baseline_accuracy = accuracy(&expected, &classes_from_probabilities(&baseline));
        for column in 0..data.predictors.len() {
            let mut permuted = test_x.clone();
            if permuted.len() > 1 {
                let values = permuted.iter().map(|row| row[column]).collect::<Vec<_>>();
                let shift = 1 + (repetition + column) % (values.len() - 1);
                for row in 0..permuted.len() {
                    permuted[row][column] = values[(row + shift) % values.len()];
                }
            }
            let changed = probabilities(
                method,
                &train_x,
                &train_y,
                &permuted,
                tune,
                model_trees,
                seed.wrapping_add(repetition as u64 * 104_729),
            )?;
            losses[column] += (baseline_accuracy
                - accuracy(&expected, &classes_from_probabilities(&changed)))
            .max(0.0);
        }
    }
    let maximum = losses.iter().copied().fold(0.0, f64::max);
    let mut rows = data
        .predictors
        .iter()
        .cloned()
        .zip(losses)
        .map(|(predictor, loss)| {
            let scaled = if maximum > 0.0 {
                100.0 * loss / maximum
            } else {
                0.0
            };
            vec![Value::Str(predictor), Value::Float(scaled)]
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        numeric(&right[1])
            .unwrap_or(0.0)
            .total_cmp(&numeric(&left[1]).unwrap_or(0.0))
    });
    Ok(Value::Table(Table::new(
        vec!["predictor".into(), "importance".into()],
        rows,
    )))
}

fn model_fields<'a>(value: &'a Value, function: &str) -> Result<&'a HashMap<String, Value>> {
    let Value::Record(fields) = value else {
        return Err(type_error(format!(
            "{function}() model must be Record, got {}",
            value.type_of()
        )));
    };
    if fields.get("schema").and_then(Value::as_str) != Some("biolang.predictive-model.v1") {
        return Err(type_error(format!(
            "{function}() requires a model returned by predictive_train()"
        )));
    }
    Ok(fields)
}

fn model_string(fields: &HashMap<String, Value>, key: &str, function: &str) -> Result<String> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| type_error(format!("{function}() model has no valid '{key}' field")))
}

fn model_strings(
    fields: &HashMap<String, Value>,
    key: &str,
    function: &str,
) -> Result<Vec<String>> {
    let Some(Value::List(values)) = fields.get(key) else {
        return Err(type_error(format!(
            "{function}() model has no valid '{key}' field"
        )));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| type_error(format!("{function}() model field '{key}' is malformed")))
        })
        .collect()
}

fn model_tune(fields: &HashMap<String, Value>, function: &str) -> Result<(Tune, usize)> {
    let Some(Value::Record(parameters)) = fields.get("parameters") else {
        return Err(type_error(format!(
            "{function}() model has no valid parameters"
        )));
    };
    Ok((
        Tune {
            first: parameters
                .get("first")
                .and_then(Value::as_float)
                .unwrap_or(0.0),
            second: parameters
                .get("second")
                .and_then(Value::as_float)
                .unwrap_or(0.0),
        },
        parameters
            .get("trees")
            .and_then(Value::as_int)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(100),
    ))
}

fn predict(args: Vec<Value>) -> Result<Value> {
    let fields = model_fields(&args[0], "predictive_predict")?;
    let Value::Table(new_data) = &args[1] else {
        return Err(type_error(format!(
            "predictive_predict() new data must be Table, got {}",
            args[1].type_of()
        )));
    };
    let opts = options(&args, 2, "predictive_predict")?;
    let output = option_str(&opts, "type", "class");
    if !matches!(output, "class" | "prob" | "both") {
        return Err(type_error(format!(
            "predictive_predict() option 'type' must be class, prob, or both, got '{output}'"
        )));
    }
    let method = model_string(fields, "method", "predictive_predict")?;
    let target = model_string(fields, "target", "predictive_predict")?;
    let predictors = model_strings(fields, "predictors", "predictive_predict")?;
    let classes = model_strings(fields, "classes", "predictive_predict")?;
    let Some(Value::Table(training)) = fields.get("training") else {
        return Err(type_error(
            "predictive_predict() model has no training table",
        ));
    };
    let training = training_dataset(&training, &target, predictors.clone())?;
    let test_x = prediction_matrix(new_data, &predictors, "predictive_predict")?;
    let (tune, trees) = model_tune(fields, "predictive_predict")?;
    let seed = fields.get("seed").and_then(Value::as_int).unwrap_or(8382) as u64;
    let probabilities = probabilities(
        &method,
        &training.x,
        &training.y,
        &test_x,
        tune,
        trees,
        seed,
    )?;
    let predictions = classes_from_probabilities(&probabilities);
    if output == "class" {
        return Ok(list(
            predictions
                .into_iter()
                .map(|value| Value::Str(classes[value].clone())),
        ));
    }
    let mut columns = Vec::new();
    if output == "both" {
        columns.push("prediction".to_string());
    }
    columns.extend(classes.iter().cloned());
    let rows = predictions
        .into_iter()
        .zip(probabilities)
        .map(|(prediction, positive)| {
            let mut row = Vec::new();
            if output == "both" {
                row.push(Value::Str(classes[prediction].clone()));
            }
            row.push(Value::Float(1.0 - positive));
            row.push(Value::Float(positive));
            row
        })
        .collect();
    Ok(Value::Table(Table::new(columns, rows)))
}

fn compare(args: Vec<Value>) -> Result<Value> {
    let Value::List(models) = &args[0] else {
        return Err(type_error(format!(
            "predictive_compare() requires List[model], got {}",
            args[0].type_of()
        )));
    };
    if models.is_empty() {
        return Err(type_error(
            "predictive_compare() requires at least one model",
        ));
    }
    let mut summary_rows = Vec::new();
    let mut resample_rows = Vec::new();
    let mut winner = None::<(String, f64)>;
    for model in models.iter() {
        let fields = model_fields(model, "predictive_compare")?;
        let method = model_string(fields, "method", "predictive_compare")?;
        let Some(Value::Record(metrics)) = fields.get("metrics") else {
            return Err(type_error("predictive_compare() model metrics are missing"));
        };
        let accuracy = metrics
            .get("accuracy")
            .and_then(Value::as_float)
            .unwrap_or(f64::NAN);
        let kappa = metrics
            .get("kappa")
            .and_then(Value::as_float)
            .unwrap_or(f64::NAN);
        summary_rows.push(vec![
            Value::Str(method.clone()),
            Value::Float(accuracy),
            Value::Float(kappa),
        ]);
        if winner.as_ref().is_none_or(|(_, best)| accuracy > *best) {
            winner = Some((method.clone(), accuracy));
        }
        let Some(Value::Table(resamples)) = fields.get("resamples") else {
            continue;
        };
        let accuracy_index = resamples.col_index("accuracy").unwrap();
        let kappa_index = resamples.col_index("kappa").unwrap();
        for (index, row) in resamples.rows.iter().enumerate() {
            resample_rows.push(vec![
                Value::Str(method.clone()),
                Value::Int((index + 1) as i64),
                row[accuracy_index].clone(),
                row[kappa_index].clone(),
            ]);
        }
    }
    summary_rows.sort_by(|left, right| {
        numeric(&right[1])
            .unwrap_or(f64::NEG_INFINITY)
            .total_cmp(&numeric(&left[1]).unwrap_or(f64::NEG_INFINITY))
    });
    Ok(record([
        (
            "summary",
            Value::Table(Table::new(
                vec!["model".into(), "accuracy".into(), "kappa".into()],
                summary_rows,
            )),
        ),
        (
            "resamples",
            Value::Table(Table::new(
                vec![
                    "model".into(),
                    "resample".into(),
                    "accuracy".into(),
                    "kappa".into(),
                ],
                resample_rows,
            )),
        ),
        (
            "best_model",
            Value::Str(winner.map(|value| value.0).unwrap_or_default()),
        ),
    ]))
}

fn plot_options(args: &[Value], index: usize, function: &str) -> Result<HashMap<String, Value>> {
    options(args, index, function)
}

fn importance_plot(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Record(model) => match model.get("importance") {
            Some(Value::Table(table)) => table,
            _ => {
                return Err(type_error(
                    "predictive_importance_plot() model has no importance table",
                ))
            }
        },
        Value::Table(table) => table,
        other => {
            return Err(type_error(format!(
                "predictive_importance_plot() requires model or Table, got {}",
                other.type_of()
            )))
        }
    };
    let opts = plot_options(&args, 1, "predictive_importance_plot")?;
    let predictor_index = table
        .col_index("predictor")
        .ok_or_else(|| type_error("importance table needs 'predictor'"))?;
    let importance_index = table
        .col_index("importance")
        .ok_or_else(|| type_error("importance table needs 'importance'"))?;
    let mut values = table
        .rows
        .iter()
        .filter_map(|row| {
            Some((
                row.get(predictor_index)?.as_str()?.to_owned(),
                numeric(row.get(importance_index)?)?,
            ))
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| right.1.total_cmp(&left.1));
    if values.is_empty() {
        return Err(type_error("importance table has no drawable rows"));
    }
    let width = option_f64(&opts, "width", 720.0).max(420.0);
    let height = option_f64(&opts, "height", 480.0).max(300.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 150.0;
    canvas.margin.right = 30.0;
    canvas.margin.top = 58.0;
    canvas.margin.bottom = 65.0;
    let maximum = values
        .iter()
        .map(|(_, value)| *value)
        .fold(0.0, f64::max)
        .max(1.0);
    let tick_domain = (0.0, maximum);
    let scale = Scale {
        domain: padded_domain(0.0, maximum, 0.065, 1.0),
        range: (canvas.margin.left, width - canvas.margin.right),
    };
    let plot_top = canvas.margin.top;
    let plot_bottom = canvas.margin.top + canvas.plot_height();
    let plot_left = canvas.margin.left;
    let plot_width = canvas.plot_width();
    canvas.add_stroked_rect(
        plot_left,
        plot_top,
        plot_width,
        canvas.plot_height(),
        "none",
        canvas.theme.axis_colour,
        canvas.theme.axis_width,
    );
    // R's varImp plot has a zero reference line inside the expanded panel.
    let zero_x = scale.map(0.0);
    canvas.add_line(
        zero_x,
        plot_top,
        zero_x,
        plot_bottom,
        canvas.theme.axis_colour,
        canvas.theme.axis_width,
    );
    let step = canvas.plot_height() / values.len() as f64;
    for (index, (name, value)) in values.iter().enumerate() {
        let y = canvas.margin.top + step * (index as f64 + 0.5);
        canvas.add_line(scale.map(0.0), y, scale.map(*value), y, "#222222", 1.0);
        canvas.add_circle_with_opacity(scale.map(*value), y, 3.5, "#0072B2", 1.0);
        canvas.add_text(canvas.margin.left - 8.0, y + 4.0, name, "end", 11.0);
    }
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Variable importance"),
    );
    canvas.draw_x_axis_with_tick_domain(&scale, tick_domain, "Importance");
    // Match R's classic boxed axis: top ticks mirror the bottom ticks but do
    // not repeat their labels.
    for tick in (Scale {
        domain: tick_domain,
        range: tick_domain,
    })
    .nice_ticks(5)
    {
        let x = scale.map(tick);
        canvas.add_line(
            x,
            plot_top,
            x,
            plot_top - 6.0,
            canvas.theme.axis_colour,
            canvas.theme.axis_width,
        );
    }
    Ok(Value::Str(canvas.render()))
}

fn resample_plot(args: Vec<Value>) -> Result<Value> {
    let table = match &args[0] {
        Value::Record(value) => match value.get("resamples") {
            Some(Value::Table(table)) => table,
            _ => {
                return Err(type_error(
                    "predictive_resample_plot() comparison has no resamples table",
                ))
            }
        },
        Value::Table(table) => table,
        other => {
            return Err(type_error(format!(
                "predictive_resample_plot() requires comparison or Table, got {}",
                other.type_of()
            )))
        }
    };
    let opts = plot_options(&args, 1, "predictive_resample_plot")?;
    let model_index = table
        .col_index("model")
        .ok_or_else(|| type_error("resamples table needs 'model'"))?;
    let accuracy_index = table
        .col_index("accuracy")
        .ok_or_else(|| type_error("resamples table needs 'accuracy'"))?;
    let kappa_index = table
        .col_index("kappa")
        .ok_or_else(|| type_error("resamples table needs 'kappa'"))?;
    let mut groups = HashMap::<String, (Vec<f64>, Vec<f64>)>::new();
    for row in &table.rows {
        let Some(name) = row.get(model_index).and_then(Value::as_str) else {
            continue;
        };
        let Some(accuracy) = row.get(accuracy_index).and_then(numeric) else {
            continue;
        };
        let Some(kappa) = row.get(kappa_index).and_then(numeric) else {
            continue;
        };
        let entry = groups.entry(name.to_string()).or_default();
        entry.0.push(accuracy);
        entry.1.push(kappa);
    }
    let mut groups = groups.into_iter().collect::<Vec<_>>();
    let model_order = |name: &str| match name {
        "random_forest" => 0,
        "elastic_net" => 1,
        "gradient_boosting" => 2,
        "knn" => 3,
        _ => 4,
    };
    groups.sort_by_key(|item| (model_order(&item.0), item.0.clone()));
    if groups.is_empty() {
        return Err(type_error("resamples table has no drawable rows"));
    }
    let width = option_f64(&opts, "width", 760.0).max(520.0);
    let height = option_f64(&opts, "height", 520.0).max(360.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 105.0;
    canvas.margin.right = 25.0;
    canvas.margin.top = 78.0;
    canvas.margin.bottom = 55.0;
    let gap = 0.0;
    let panel_width = (canvas.plot_width() - gap) / 2.0;
    let panel_height = canvas.plot_height();
    // caret's bwplot.resamples() uses lattice's default same-scale relation
    // across conditioned metric panels. Accuracy and Kappa must therefore map
    // an equal value to the same horizontal position in their respective panel.
    let all_metrics = groups
        .iter()
        .flat_map(|(_, values)| values.0.iter().chain(values.1.iter()).copied())
        .collect::<Vec<_>>();
    let metric_low = all_metrics.iter().copied().fold(f64::INFINITY, f64::min);
    let metric_high = all_metrics
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    let metric_span = (metric_high - metric_low).max(0.1);
    let shared_domain = (
        metric_low - metric_span * 0.10,
        metric_high + metric_span * 0.10,
    );
    for (panel, (label, column)) in [("Accuracy", 0usize), ("Kappa", 1usize)]
        .into_iter()
        .enumerate()
    {
        let left = canvas.margin.left + panel as f64 * (panel_width + gap);
        let top = canvas.margin.top;
        let x_scale = Scale {
            domain: shared_domain,
            range: (left, left + panel_width),
        };
        let x_ticks = x_scale.nice_ticks(5);
        let vertical = x_ticks
            .iter()
            .map(|tick| x_scale.map(*tick))
            .collect::<Vec<_>>();
        draw_panel_grid(
            &mut canvas,
            left,
            top,
            panel_width,
            panel_height,
            &vertical,
            &[],
        );
        canvas.add_rect(left, top - 27.0, panel_width, 27.0, "#F2F2F2");
        canvas.add_stroked_rect(left, top - 27.0, panel_width, 27.0, "none", "#333333", 1.0);
        canvas.add_text(left + panel_width / 2.0, top - 8.0, label, "middle", 13.0);
        let outer_top = top - 27.0;
        let bottom = top + panel_height;
        let decimals = crate::plot::tick_decimals(&x_ticks);
        for tick in &x_ticks {
            let x = x_scale.map(*tick);
            canvas.add_line(x, bottom, x, bottom + 6.0, "#333333", 1.0);
            canvas.add_line(x, outer_top, x, outer_top - 6.0, "#333333", 1.0);
            if panel == 0 {
                canvas.add_text(
                    x,
                    bottom + 20.0,
                    &format!("{tick:.decimals$}"),
                    "middle",
                    10.0,
                );
            } else {
                canvas.add_text(
                    x,
                    outer_top - 10.0,
                    &format!("{tick:.decimals$}"),
                    "middle",
                    10.0,
                );
            }
        }
        let step = panel_height / groups.len() as f64;
        for (index, (name, values)) in groups.iter().enumerate() {
            let values = if column == 0 { &values.0 } else { &values.1 };
            let Some(summary) = lattice_box_summary(values.clone()) else {
                continue;
            };
            let y = top + step * (index as f64 + 0.5);
            canvas.add_dashed_line(
                x_scale.map(summary.lower_whisker),
                y,
                x_scale.map(summary.upper_whisker),
                y,
                "#0072B2",
                1.1,
                6.0,
            );
            canvas.add_dashed_line(
                x_scale.map(summary.lower_whisker),
                y - step * 0.19,
                x_scale.map(summary.lower_whisker),
                y + step * 0.19,
                "#0072B2",
                1.1,
                6.0,
            );
            canvas.add_dashed_line(
                x_scale.map(summary.upper_whisker),
                y - step * 0.19,
                x_scale.map(summary.upper_whisker),
                y + step * 0.19,
                "#0072B2",
                1.1,
                6.0,
            );
            canvas.add_stroked_rect(
                x_scale.map(summary.q1),
                y - step * 0.19,
                (x_scale.map(summary.q3) - x_scale.map(summary.q1))
                    .abs()
                    .max(1.0),
                step * 0.38,
                "#ffffff",
                "#0072B2",
                1.1,
            );
            // lattice panel.bwplot's solid black `box.dot` is the median
            // (blist.stats[, 3]), not the arithmetic mean or a second marker.
            canvas.add_circle_with_opacity(x_scale.map(summary.median), y, 3.8, "#000000", 1.0);
            for outlier in summary.outliers {
                canvas.add_stroked_circle(x_scale.map(outlier), y, 3.1, "#ffffff", "#0072B2", 1.0);
            }
            if panel == 0 {
                let display = match name.as_str() {
                    "random_forest" => "rf",
                    "elastic_net" => "glmnet",
                    "gradient_boosting" => "gbm",
                    "knn" => "knn",
                    other => other,
                };
                canvas.add_text(left - 9.0, y + 4.0, display, "end", 11.0);
            }
        }
    }
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Bootstrap model comparison"),
    );
    Ok(Value::Str(canvas.render()))
}

fn solve(mut matrix: Vec<Vec<f64>>, mut right: Vec<f64>) -> Option<Vec<f64>> {
    let n = right.len();
    for column in 0..n {
        let pivot = (column..n).max_by(|left, right_index| {
            matrix[*left][column]
                .abs()
                .total_cmp(&matrix[*right_index][column].abs())
        })?;
        if matrix[pivot][column].abs() < 1e-12 {
            return None;
        }
        matrix.swap(column, pivot);
        right.swap(column, pivot);
        let diagonal = matrix[column][column];
        for item in &mut matrix[column][column..] {
            *item /= diagonal;
        }
        right[column] /= diagonal;
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for item in column..n {
                matrix[row][item] -= factor * matrix[column][item];
            }
            right[row] -= factor * right[column];
        }
    }
    Some(right)
}

fn regression_coefficients(design: &[Vec<f64>], response: &[f64], ridge: f64) -> Result<Vec<f64>> {
    let p = design[0].len();
    let mut xtx = vec![vec![0.0; p]; p];
    let mut xty = vec![0.0; p];
    for (row, outcome) in design.iter().zip(response) {
        for left in 0..p {
            xty[left] += row[left] * outcome;
            for right in 0..p {
                xtx[left][right] += row[left] * row[right];
            }
        }
    }
    for (index, row) in xtx.iter_mut().enumerate().skip(1) {
        row[index] += ridge;
    }
    solve(xtx, xty).ok_or_else(|| type_error("seasonal_forecast() design matrix is singular"))
}

fn forecast_basis(t: f64, trend_scale: f64, period: f64, order: usize) -> Vec<f64> {
    let scaled = t / trend_scale.max(1.0);
    let mut row = vec![1.0, scaled];
    for change in [0.25, 0.50, 0.75] {
        row.push((scaled - change).max(0.0));
    }
    for harmonic in 1..=order {
        let angle = std::f64::consts::TAU * harmonic as f64 * t / period;
        row.push(angle.sin());
        row.push(angle.cos());
    }
    row
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn shift_iso_date(date: &str, days: i64) -> Result<String> {
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| type_error(format!("seasonal_forecast() cannot parse date '{date}'")))?;
    Ok((parsed + chrono::Duration::days(days))
        .format("%Y-%m-%d")
        .to_string())
}

fn date_offset_days(date: &str, origin: chrono::NaiveDate) -> Result<f64> {
    let parsed = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .map_err(|_| type_error(format!("seasonal_forecast() cannot parse date '{date}'")))?;
    Ok((parsed - origin).num_days() as f64)
}

fn seasonal_forecast(args: Vec<Value>) -> Result<Value> {
    let Value::Table(table) = &args[0] else {
        return Err(type_error(format!(
            "seasonal_forecast() requires Table, got {}",
            args[0].type_of()
        )));
    };
    let opts = options(&args, 1, "seasonal_forecast")?;
    let date_name = option_str(&opts, "date", "ds").to_string();
    let value_name = option_str(&opts, "value", "y").to_string();
    let date_index = table.col_index(&date_name).ok_or_else(|| {
        type_error(format!(
            "seasonal_forecast() missing date column '{date_name}'"
        ))
    })?;
    let value_index = table.col_index(&value_name).ok_or_else(|| {
        type_error(format!(
            "seasonal_forecast() missing value column '{value_name}'"
        ))
    })?;
    let mut dates = Vec::new();
    let mut response = Vec::new();
    for row in &table.rows {
        let Some(date) = row.get(date_index).and_then(Value::as_str) else {
            continue;
        };
        let Some(value) = row.get(value_index).and_then(numeric) else {
            continue;
        };
        dates.push(date.to_string());
        response.push(value);
    }
    if response.len() < 2 * 52 {
        return Err(type_error(
            "seasonal_forecast() needs at least two seasonal cycles (104 complete weekly observations)",
        ));
    }
    let date_period = opts.get("period_days").and_then(numeric);
    let period = date_period
        .unwrap_or_else(|| option_f64(&opts, "period", 52.1775))
        .max(2.0);
    let order = option_usize(&opts, "fourier_order", 10).clamp(1, 30);
    let future_periods = option_usize(&opts, "periods", 260).clamp(1, 10_000);
    let frequency_days = option_usize(&opts, "frequency_days", 7).clamp(1, 365) as i64;
    let confidence = option_f64(&opts, "confidence", 0.95).clamp(0.50, 0.999);
    let origin = chrono::NaiveDate::parse_from_str(&dates[0], "%Y-%m-%d").map_err(|_| {
        type_error(format!(
            "seasonal_forecast() cannot parse date '{}'",
            dates[0]
        ))
    })?;
    let basis_times = if date_period.is_some() {
        dates
            .iter()
            .map(|date| date_offset_days(date, origin))
            .collect::<Result<Vec<_>>>()?
    } else {
        (0..response.len()).map(|index| index as f64).collect()
    };
    let trend_scale = basis_times.last().copied().unwrap_or(1.0).max(1.0);
    let design = basis_times
        .iter()
        .map(|time| forecast_basis(*time, trend_scale, period, order))
        .collect::<Vec<_>>();
    let coefficients = regression_coefficients(&design, &response, 0.01)?;
    let fitted = design
        .iter()
        .map(|row| dot(row, &coefficients))
        .collect::<Vec<_>>();
    let residual_sd = (response
        .iter()
        .zip(&fitted)
        .map(|(observed, fitted)| (observed - fitted).powi(2))
        .sum::<f64>()
        / response.len().saturating_sub(coefficients.len()).max(1) as f64)
        .sqrt();
    let mut absolute_residuals = response
        .iter()
        .zip(&fitted)
        .map(|(observed, fitted)| (observed - fitted).abs())
        .collect::<Vec<_>>();
    absolute_residuals.sort_by(f64::total_cmp);
    let middle = absolute_residuals.len() / 2;
    let median_absolute_residual = if absolute_residuals.len() % 2 == 0 {
        (absolute_residuals[middle - 1] + absolute_residuals[middle]) / 2.0
    } else {
        absolute_residuals[middle]
    };
    let interval_scale = (1.4826 * median_absolute_residual).max(f64::EPSILON);
    // 1.96 is exact enough for the default teaching interval. For other
    // confidence levels use the runtime's established inverse-normal
    // approximation through a compact Acklam implementation.
    let z = inverse_normal(0.5 + confidence / 2.0);
    let last_date = dates.last().unwrap().clone();
    let mut forecast_rows = Vec::with_capacity(response.len() + future_periods);
    let mut component_rows = Vec::with_capacity(response.len() + future_periods);
    for index in 0..(response.len() + future_periods) {
        let future_step = index.saturating_sub(response.len()) + 1;
        let time = if index < response.len() {
            basis_times[index]
        } else if date_period.is_some() {
            basis_times.last().copied().unwrap_or(0.0) + frequency_days as f64 * future_step as f64
        } else {
            index as f64
        };
        let basis = forecast_basis(time, trend_scale, period, order);
        let yhat = dot(&basis, &coefficients);
        let horizon = index.saturating_sub(response.len()) as f64;
        // Median absolute residuals keep a handful of outbreak/reporting
        // spikes from inflating every interval. The 1.4826 consistency factor
        // estimates sigma under normal errors; modest horizon growth then
        // acknowledges that extrapolation becomes less certain.
        let interval = z * interval_scale * (1.0 + 0.05 * horizon / response.len() as f64).sqrt();
        let date = if index < dates.len() {
            dates[index].clone()
        } else {
            shift_iso_date(&last_date, frequency_days * future_step as i64)?
        };
        let observed = response.get(index).copied();
        forecast_rows.push(vec![
            Value::Str(date.clone()),
            observed.map(Value::Float).unwrap_or(Value::Nil),
            Value::Float(yhat),
            Value::Float(yhat - interval),
            Value::Float(yhat + interval),
        ]);
        let trend = dot(&basis[..5], &coefficients[..5]);
        // Prophet's trend component shows a narrow uncertainty shadow rather
        // than reusing the much wider observation interval. Keep it subtle in
        // history and let it widen gradually through the forecast horizon.
        let future_fraction = horizon / future_periods.max(1) as f64;
        let trend_interval = interval * (0.025 + 0.18 * future_fraction.min(1.0));
        component_rows.push(vec![
            Value::Str(date),
            Value::Float(trend),
            Value::Float(trend - trend_interval),
            Value::Float(trend + trend_interval),
            Value::Float(yhat - trend),
        ]);
    }
    Ok(record([
        ("schema", Value::Str("biolang.seasonal-forecast.v1".into())),
        ("date", Value::Str(date_name)),
        ("value", Value::Str(value_name)),
        ("period", Value::Float(period)),
        (
            "period_unit",
            Value::Str(
                if date_period.is_some() {
                    "days"
                } else {
                    "observations"
                }
                .into(),
            ),
        ),
        ("fourier_order", Value::Int(order as i64)),
        ("history_rows", Value::Int(response.len() as i64)),
        ("future_rows", Value::Int(future_periods as i64)),
        ("residual_sd", Value::Float(residual_sd)),
        ("interval_scale_mad", Value::Float(interval_scale)),
        (
            "forecast",
            Value::Table(Table::new(
                vec![
                    "date".into(),
                    "observed".into(),
                    "yhat".into(),
                    "yhat_lower".into(),
                    "yhat_upper".into(),
                ],
                forecast_rows,
            )),
        ),
        (
            "components",
            Value::Table(Table::new(
                vec![
                    "date".into(),
                    "trend".into(),
                    "trend_lower".into(),
                    "trend_upper".into(),
                    "yearly".into(),
                ],
                component_rows,
            )),
        ),
    ]))
}

fn inverse_normal(probability: f64) -> f64 {
    // Peter J. Acklam's rational approximation, written from the published
    // coefficients. Maximum absolute error is well below plot resolution.
    const A: [f64; 6] = [
        -39.6968302866538,
        220.946098424521,
        -275.928510446969,
        138.357751867269,
        -30.6647980661472,
        2.50662827745924,
    ];
    const B: [f64; 5] = [
        -54.4760987982241,
        161.585836858041,
        -155.698979859887,
        66.8013118877197,
        -13.2806815528857,
    ];
    const C: [f64; 6] = [
        -0.00778489400243029,
        -0.322396458041136,
        -2.40075827716184,
        -2.54973253934373,
        4.37466414146497,
        2.93816398269878,
    ];
    const D: [f64; 4] = [
        0.00778469570904146,
        0.32246712907004,
        2.445134137143,
        3.75440866190742,
    ];
    let p = probability.clamp(1e-12, 1.0 - 1e-12);
    if p < 0.02425 {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p > 0.97575 {
        -inverse_normal(1.0 - p)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
}

fn forecast_record<'a>(value: &'a Value, function: &str) -> Result<&'a HashMap<String, Value>> {
    let Value::Record(fields) = value else {
        return Err(type_error(format!(
            "{function}() requires forecast Record, got {}",
            value.type_of()
        )));
    };
    if fields.get("schema").and_then(Value::as_str) != Some("biolang.seasonal-forecast.v1") {
        return Err(type_error(format!(
            "{function}() requires a result returned by seasonal_forecast()"
        )));
    }
    Ok(fields)
}

fn forecast_columns(table: &Table) -> Result<(usize, usize, usize, usize)> {
    Ok((
        table
            .col_index("observed")
            .ok_or_else(|| type_error("forecast table missing observed"))?,
        table
            .col_index("yhat")
            .ok_or_else(|| type_error("forecast table missing yhat"))?,
        table
            .col_index("yhat_lower")
            .ok_or_else(|| type_error("forecast table missing yhat_lower"))?,
        table
            .col_index("yhat_upper")
            .ok_or_else(|| type_error("forecast table missing yhat_upper"))?,
    ))
}

fn forecast_date_coordinates(
    table: &Table,
) -> Result<(Vec<f64>, chrono::NaiveDate, chrono::NaiveDate)> {
    let date_index = table
        .col_index("date")
        .ok_or_else(|| type_error("forecast table missing date"))?;
    let dates = table
        .rows
        .iter()
        .map(|row| {
            let date = row
                .get(date_index)
                .and_then(Value::as_str)
                .ok_or_else(|| type_error("forecast table has an invalid date"))?;
            chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map_err(|_| type_error(format!("forecast table cannot parse date '{date}'")))
        })
        .collect::<Result<Vec<_>>>()?;
    let origin = *dates
        .first()
        .ok_or_else(|| type_error("forecast table has no rows"))?;
    let last = *dates.last().unwrap();
    let coordinates = dates
        .iter()
        .map(|date| (*date - origin).num_days() as f64)
        .collect();
    Ok((coordinates, origin, last))
}

fn year_ticks(origin: chrono::NaiveDate, last: chrono::NaiveDate) -> Vec<(f64, String)> {
    let span = last.year() - origin.year();
    let step = if span > 12 {
        5
    } else if span > 6 {
        2
    } else {
        1
    };
    let mut year = origin.year();
    while year % step != 0 {
        year += 1;
    }
    let mut ticks = Vec::new();
    while year <= last.year() {
        if let Some(date) = chrono::NaiveDate::from_ymd_opt(year, 1, 1) {
            if date >= origin && date <= last {
                ticks.push(((date - origin).num_days() as f64, year.to_string()));
            }
        }
        year += step;
    }
    ticks
}

fn draw_date_axis(
    canvas: &mut SvgCanvas,
    scale: &Scale,
    origin: chrono::NaiveDate,
    last: chrono::NaiveDate,
    label: &str,
) {
    let y = canvas.margin.top + canvas.plot_height();
    canvas.add_line(
        canvas.margin.left,
        y,
        canvas.margin.left + canvas.plot_width(),
        y,
        canvas.theme.axis_colour,
        canvas.theme.axis_width,
    );
    for (value, text) in year_ticks(origin, last) {
        let x = scale.map(value);
        canvas.add_line(
            x,
            y,
            x,
            y + 5.0,
            canvas.theme.axis_colour,
            canvas.theme.axis_width,
        );
        canvas.add_text(x, y + 19.0, &text, "middle", canvas.theme.tick_size);
    }
    canvas.add_axis_title(
        canvas.margin.left + canvas.plot_width() / 2.0,
        canvas.height - 6.0,
        label,
        "x",
        None,
    );
}

fn seasonal_forecast_plot(args: Vec<Value>) -> Result<Value> {
    let fields = forecast_record(&args[0], "seasonal_forecast_plot")?;
    let Some(Value::Table(table)) = fields.get("forecast") else {
        return Err(type_error("seasonal forecast has no forecast table"));
    };
    let opts = options(&args, 1, "seasonal_forecast_plot")?;
    let (observed_index, yhat_index, lower_index, upper_index) = forecast_columns(table)?;
    let (date_coordinates, date_origin, last_date) = forecast_date_coordinates(table)?;
    let values = table
        .rows
        .iter()
        .flat_map(|row| {
            [
                row.get(observed_index).and_then(numeric),
                row.get(lower_index).and_then(numeric),
                row.get(upper_index).and_then(numeric),
            ]
            .into_iter()
            .flatten()
        })
        .collect::<Vec<_>>();
    let y_min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let y_max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let width = option_f64(&opts, "width", 860.0).max(520.0);
    let height = option_f64(&opts, "height", 500.0).max(340.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 75.0;
    canvas.margin.right = 25.0;
    canvas.margin.top = 58.0;
    canvas.margin.bottom = 70.0;
    let last_coordinate = date_coordinates.last().copied().unwrap_or(1.0);
    let x_scale = Scale {
        domain: padded_domain(0.0, last_coordinate, 0.018, 1.0),
        range: (canvas.margin.left, width - canvas.margin.right),
    };
    let (y_low, y_high) = padded_domain(y_min, y_max, 0.045, 1.0);
    let y_scale = Scale {
        domain: (y_low, y_high),
        range: (height - canvas.margin.bottom, canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    let upper = table
        .rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            Some((
                x_scale.map(date_coordinates[index]),
                y_scale.map(numeric(row.get(upper_index)?)?),
            ))
        })
        .collect::<Vec<_>>();
    let lower = table
        .rows
        .iter()
        .enumerate()
        .rev()
        .filter_map(|(index, row)| {
            Some((
                x_scale.map(date_coordinates[index]),
                y_scale.map(numeric(row.get(lower_index)?)?),
            ))
        })
        .collect::<Vec<_>>();
    let mut polygon = upper;
    polygon.extend(lower);
    canvas.add_polygon_with_opacity(&polygon, "#9ECAE1", 0.55);
    let fitted = table
        .rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| {
            Some((
                x_scale.map(date_coordinates[index]),
                y_scale.map(numeric(row.get(yhat_index)?)?),
            ))
        })
        .collect::<Vec<_>>();
    canvas.add_polyline(&fitted, "#2C7FB8", 1.5);
    for (index, row) in table.rows.iter().enumerate() {
        if let Some(observed) = row.get(observed_index).and_then(numeric) {
            canvas.add_circle_with_opacity(
                x_scale.map(date_coordinates[index]),
                y_scale.map(observed),
                2.2,
                "#111111",
                0.78,
            );
        }
    }
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Seasonal forecast"),
    );
    draw_date_axis(
        &mut canvas,
        &x_scale,
        date_origin,
        last_date,
        option_str(&opts, "x_label", "Time"),
    );
    canvas.draw_y_axis(&y_scale, option_str(&opts, "y_label", "Value"));
    Ok(Value::Str(canvas.render()))
}

fn seasonal_components_plot(args: Vec<Value>) -> Result<Value> {
    let fields = forecast_record(&args[0], "seasonal_components_plot")?;
    let Some(Value::Table(table)) = fields.get("components") else {
        return Err(type_error("seasonal forecast has no components table"));
    };
    let opts = options(&args, 1, "seasonal_components_plot")?;
    let trend_index = table
        .col_index("trend")
        .ok_or_else(|| type_error("components table missing trend"))?;
    let yearly_index = table
        .col_index("yearly")
        .ok_or_else(|| type_error("components table missing yearly"))?;
    let trend_lower_index = table.col_index("trend_lower");
    let trend_upper_index = table.col_index("trend_upper");
    let width = option_f64(&opts, "width", 860.0).max(520.0);
    let height = option_f64(&opts, "height", 620.0).max(420.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 78.0;
    canvas.margin.right = 25.0;
    canvas.margin.top = 52.0;
    canvas.margin.bottom = 55.0;
    let gap = 55.0;
    let panel_height = (canvas.plot_height() - gap) / 2.0;
    let plot_left = canvas.margin.left;
    let plot_width = canvas.plot_width();
    let (date_coordinates, origin, last_date) = forecast_date_coordinates(table)?;
    let trend_values = table
        .rows
        .iter()
        .filter_map(|row| row.get(trend_index).and_then(numeric))
        .collect::<Vec<_>>();
    let trend_low = trend_lower_index
        .and_then(|index| {
            table
                .rows
                .iter()
                .filter_map(|row| row.get(index).and_then(numeric))
                .min_by(f64::total_cmp)
        })
        .unwrap_or_else(|| trend_values.iter().copied().fold(f64::INFINITY, f64::min));
    let trend_high = trend_upper_index
        .and_then(|index| {
            table
                .rows
                .iter()
                .filter_map(|row| row.get(index).and_then(numeric))
                .max_by(f64::total_cmp)
        })
        .unwrap_or_else(|| {
            trend_values
                .iter()
                .copied()
                .fold(f64::NEG_INFINITY, f64::max)
        });
    let trend_padding = ((trend_high - trend_low) * 0.05).max(1.0);
    let trend_top = canvas.margin.top;
    let last_coordinate = date_coordinates.last().copied().unwrap_or(1.0);
    let trend_x = Scale {
        domain: padded_domain(0.0, last_coordinate, 0.018, 1.0),
        range: (canvas.margin.left, width - canvas.margin.right),
    };
    let trend_y = Scale {
        domain: (trend_low - trend_padding, trend_high + trend_padding),
        range: (trend_top + panel_height, trend_top),
    };
    let trend_date_ticks = year_ticks(origin, last_date);
    let trend_vertical = trend_date_ticks
        .iter()
        .map(|(value, _)| trend_x.map(*value))
        .collect::<Vec<_>>();
    let trend_ticks = trend_y.nice_ticks(4);
    let trend_horizontal = trend_ticks
        .iter()
        .map(|value| trend_y.map(*value))
        .collect::<Vec<_>>();
    draw_panel_grid(
        &mut canvas,
        plot_left,
        trend_top,
        plot_width,
        panel_height,
        &trend_vertical,
        &trend_horizontal,
    );
    if let (Some(lower_index), Some(upper_index)) = (trend_lower_index, trend_upper_index) {
        let mut band = table
            .rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                Some((
                    trend_x.map(date_coordinates[index]),
                    trend_y.map(numeric(row.get(upper_index)?)?),
                ))
            })
            .collect::<Vec<_>>();
        band.extend(
            table
                .rows
                .iter()
                .enumerate()
                .rev()
                .filter_map(|(index, row)| {
                    Some((
                        trend_x.map(date_coordinates[index]),
                        trend_y.map(numeric(row.get(lower_index)?)?),
                    ))
                }),
        );
        canvas.add_polygon_with_opacity(&band, "#9ECAE1", 0.30);
    }
    let trend_points = trend_values
        .iter()
        .enumerate()
        .map(|(index, value)| (trend_x.map(date_coordinates[index]), trend_y.map(*value)))
        .collect::<Vec<_>>();
    canvas.add_polyline(&trend_points, "#0072B2", 1.5);
    for (value, label) in &trend_date_ticks {
        canvas.add_text(
            trend_x.map(*value),
            trend_top + panel_height + 17.0,
            label,
            "middle",
            canvas.theme.tick_size,
        );
    }
    for value in &trend_ticks {
        canvas.add_text(
            canvas.margin.left - 7.0,
            trend_y.map(*value) + 4.0,
            &format!("{value:.0}"),
            "end",
            canvas.theme.tick_size,
        );
    }
    canvas.add_axis_title(
        15.0,
        trend_top + panel_height / 2.0,
        "trend",
        "y",
        Some(-90.0),
    );
    canvas.add_axis_title(
        canvas.margin.left + canvas.plot_width() / 2.0,
        trend_top + panel_height + 36.0,
        "ds",
        "x",
        None,
    );

    let date_index = table.col_index("date").unwrap();
    let mut yearly_sums = vec![0.0; 366];
    let mut yearly_counts = vec![0usize; 366];
    for row in &table.rows {
        let (Some(date), Some(value)) = (
            row.get(date_index).and_then(Value::as_str),
            row.get(yearly_index).and_then(numeric),
        ) else {
            continue;
        };
        let Ok(date) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
            continue;
        };
        let day = date.ordinal0().min(365) as usize;
        yearly_sums[day] += value;
        yearly_counts[day] += 1;
    }
    let yearly_values = yearly_sums
        .iter()
        .zip(&yearly_counts)
        .enumerate()
        .filter_map(|(day, (sum, count))| {
            (*count > 0).then_some((day as f64, *sum / *count as f64))
        })
        .collect::<Vec<_>>();
    let yearly_low = yearly_values
        .iter()
        .map(|(_, value)| *value)
        .fold(f64::INFINITY, f64::min);
    let yearly_high = yearly_values
        .iter()
        .map(|(_, value)| *value)
        .fold(f64::NEG_INFINITY, f64::max);
    let yearly_padding = ((yearly_high - yearly_low) * 0.05).max(1.0);
    let yearly_top = trend_top + panel_height + gap;
    let yearly_x = Scale {
        domain: padded_domain(0.0, 365.0, 0.018, 1.0),
        range: (canvas.margin.left, width - canvas.margin.right),
    };
    let yearly_y = Scale {
        domain: (yearly_low - yearly_padding, yearly_high + yearly_padding),
        range: (yearly_top + panel_height, yearly_top),
    };
    let seasonal_ticks = [
        (0.0, "January 01"),
        (90.0, "April 01"),
        (181.0, "July 01"),
        (273.0, "October 01"),
        (365.0, "January 01"),
    ];
    let yearly_vertical = seasonal_ticks
        .iter()
        .map(|(value, _)| yearly_x.map(*value))
        .collect::<Vec<_>>();
    let yearly_ticks = yearly_y.nice_ticks(4);
    let yearly_horizontal = yearly_ticks
        .iter()
        .map(|value| yearly_y.map(*value))
        .collect::<Vec<_>>();
    draw_panel_grid(
        &mut canvas,
        plot_left,
        yearly_top,
        plot_width,
        panel_height,
        &yearly_vertical,
        &yearly_horizontal,
    );
    let yearly_points = yearly_values
        .iter()
        .map(|(day, value)| (yearly_x.map(*day), yearly_y.map(*value)))
        .collect::<Vec<_>>();
    canvas.add_polyline(&yearly_points, "#0072B2", 1.5);
    for (value, label) in seasonal_ticks {
        canvas.add_text(
            yearly_x.map(value),
            yearly_top + panel_height + 17.0,
            label,
            "middle",
            canvas.theme.tick_size,
        );
    }
    for value in &yearly_ticks {
        canvas.add_text(
            canvas.margin.left - 7.0,
            yearly_y.map(*value) + 4.0,
            &format!("{value:.0}"),
            "end",
            canvas.theme.tick_size,
        );
    }
    canvas.add_axis_title(
        15.0,
        yearly_top + panel_height / 2.0,
        "yearly",
        "y",
        Some(-90.0),
    );
    canvas.add_axis_title(
        canvas.margin.left + canvas.plot_width() / 2.0,
        height - 5.0,
        "Day of year",
        "x",
        None,
    );
    canvas.draw_title(
        opts.get("title")
            .and_then(Value::as_str)
            .unwrap_or("Forecast components"),
    );
    Ok(Value::Str(canvas.render()))
}

fn value_list(value: &Value, function: &str, name: &str) -> Result<Vec<Value>> {
    match value {
        Value::List(values) => Ok(values.iter().cloned().collect()),
        other => Err(type_error(format!(
            "{function}() {name} must be List, got {}",
            other.type_of()
        ))),
    }
}

fn category(value: &Value, missing: &str) -> Option<String> {
    match value {
        Value::Nil => Some(missing.to_string()),
        Value::Str(value) => Some(value.clone()),
        Value::Int(value) => Some(value.to_string()),
        Value::Float(value) if value.is_finite() => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn ordered_levels(values: &[Value], missing: &str) -> Vec<String> {
    let mut levels = values
        .iter()
        .filter(|value| !matches!(value, Value::Nil))
        .filter_map(|value| category(value, missing))
        .collect::<Vec<_>>();
    levels.sort();
    levels.dedup();
    if values.iter().any(|value| matches!(value, Value::Nil)) {
        levels.push(missing.to_string());
    }
    levels
}

fn colours(
    options: &HashMap<String, Value>,
    levels: &[String],
    missing: &str,
) -> Result<Vec<String>> {
    let supplied = option_strings(options, "colors")?;
    let defaults = crate::plot::hue_palette(
        levels
            .iter()
            .filter(|level| level.as_str() != missing)
            .count(),
    );
    let mut observed = 0usize;
    Ok(levels
        .iter()
        .map(|level| {
            if level == missing {
                "#7F7F7F".to_string()
            } else {
                let colour = supplied
                    .as_ref()
                    .and_then(|values| values.get(observed % values.len().max(1)))
                    .cloned()
                    .unwrap_or_else(|| defaults[observed % defaults.len().max(1)].clone());
                observed += 1;
                colour
            }
        })
        .collect())
}

fn legend(
    canvas: &mut SvgCanvas,
    levels: &[String],
    colours: &[String],
    title: &str,
    x: f64,
    top: f64,
) {
    canvas.add_text(x, top, title, "start", 12.0);
    for (index, (level, colour)) in levels.iter().zip(colours).enumerate() {
        let y = top + 23.0 + index as f64 * 23.0;
        canvas.add_rect(x, y - 11.0, 13.0, 13.0, colour);
        canvas.add_text(x + 20.0, y, level, "start", 11.0);
    }
}

fn draw_panel_grid(
    canvas: &mut SvgCanvas,
    left: f64,
    top: f64,
    width: f64,
    height: f64,
    vertical: &[f64],
    horizontal: &[f64],
) {
    canvas.add_rect(left, top, width, height, canvas.theme.panel_colour);
    if canvas.theme.grid_width > 0.0 {
        for x in vertical {
            canvas.add_line(
                *x,
                top,
                *x,
                top + height,
                canvas.theme.grid_colour,
                canvas.theme.grid_width,
            );
        }
        for y in horizontal {
            canvas.add_line(
                left,
                *y,
                left + width,
                *y,
                canvas.theme.grid_colour,
                canvas.theme.grid_width,
            );
        }
    }
    canvas.add_stroked_rect(
        left,
        top,
        width,
        height,
        "none",
        canvas.theme.axis_colour,
        canvas.theme.axis_width,
    );
}

#[derive(Debug)]
struct BoxSummary {
    lower_whisker: f64,
    q1: f64,
    median: f64,
    q3: f64,
    upper_whisker: f64,
    outliers: Vec<f64>,
}

fn box_summary(mut values: Vec<f64>) -> Option<BoxSummary> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let value_at = |probability: f64| {
        let position = probability * (values.len() - 1) as f64;
        let low = position.floor() as usize;
        let high = position.ceil() as usize;
        values[low] + (values[high] - values[low]) * (position - low as f64)
    };
    let q1 = value_at(0.25);
    let median = value_at(0.5);
    let q3 = value_at(0.75);
    let iqr = q3 - q1;
    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;
    let lower_whisker = values
        .iter()
        .copied()
        .find(|value| *value >= lower_fence)
        .unwrap_or(values[0]);
    let upper_whisker = values
        .iter()
        .copied()
        .rev()
        .find(|value| *value <= upper_fence)
        .unwrap_or(*values.last().unwrap());
    let outliers = values
        .into_iter()
        .filter(|value| *value < lower_whisker || *value > upper_whisker)
        .collect();
    Some(BoxSummary {
        lower_whisker,
        q1,
        median,
        q3,
        upper_whisker,
        outliers,
    })
}

/// Reproduce the summaries used by lattice::panel.bwplot: R's
/// boxplot.stats() is based on fivenum() hinges rather than type-7 quartiles.
fn lattice_box_summary(mut values: Vec<f64>) -> Option<BoxSummary> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(f64::total_cmp);
    let n = values.len() as f64;
    let n4 = ((n + 3.0) / 2.0).floor() / 2.0;
    let value_at = |one_based: f64| {
        let low = one_based.floor().max(1.0) as usize - 1;
        let high = one_based.ceil().max(1.0) as usize - 1;
        (values[low] + values[high]) / 2.0
    };
    let q1 = value_at(n4);
    let median = value_at((n + 1.0) / 2.0);
    let q3 = value_at(n + 1.0 - n4);
    let iqr = q3 - q1;
    let lower_fence = q1 - 1.5 * iqr;
    let upper_fence = q3 + 1.5 * iqr;
    let lower_whisker = values
        .iter()
        .copied()
        .find(|value| *value >= lower_fence)
        .unwrap_or(values[0]);
    let upper_whisker = values
        .iter()
        .copied()
        .rev()
        .find(|value| *value <= upper_fence)
        .unwrap_or(*values.last().unwrap());
    let outliers = values
        .into_iter()
        .filter(|value| *value < lower_fence || *value > upper_fence)
        .collect();
    Some(BoxSummary {
        lower_whisker,
        q1,
        median,
        q3,
        upper_whisker,
        outliers,
    })
}

fn grouped_density_plot(args: Vec<Value>) -> Result<Value> {
    let values = value_list(&args[0], "grouped_density_plot", "values")?;
    let groups = value_list(&args[1], "grouped_density_plot", "groups")?;
    if values.len() != groups.len() {
        return Err(type_error(
            "grouped_density_plot() values and groups must have equal length",
        ));
    }
    let opts = options(&args, 2, "grouped_density_plot")?;
    let missing = option_str(&opts, "missing_label", "NA");
    let levels = ordered_levels(&groups, missing);
    let palette = colours(&opts, &levels, missing)?;
    let grouped = levels
        .iter()
        .map(|level| {
            values
                .iter()
                .zip(&groups)
                .filter_map(|(value, group)| {
                    (category(group, missing).as_deref() == Some(level.as_str()))
                        .then(|| numeric(value))
                        .flatten()
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let pooled = grouped.iter().flatten().copied().collect::<Vec<_>>();
    if pooled.len() < 2 {
        return Err(type_error(
            "grouped_density_plot() has too few numeric values",
        ));
    }
    let minimum = pooled.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = pooled.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let points = option_usize(&opts, "points", 240).clamp(50, 1_000);
    let x_span = (maximum - minimum).max(1.0);
    let x_start = minimum - x_span * 0.05;
    let x_end = maximum + x_span * 0.05;
    let curves = grouped
        .iter()
        .map(|samples| {
            if samples.is_empty() {
                return vec![(x_start, 0.0); points];
            }
            let sample_mean = samples.iter().sum::<f64>() / samples.len() as f64;
            let sd = (samples
                .iter()
                .map(|value| (value - sample_mean).powi(2))
                .sum::<f64>()
                / samples.len().saturating_sub(1).max(1) as f64)
                .sqrt();
            let bandwidth = (1.06 * sd * (samples.len() as f64).powf(-0.2))
                .max(x_span / 100.0)
                .max(1e-6);
            (0..points)
                .map(|index| {
                    let x = x_start + (x_end - x_start) * index as f64 / (points - 1) as f64;
                    let density = samples
                        .iter()
                        .map(|sample| {
                            let z = (x - sample) / bandwidth;
                            (-0.5 * z * z).exp()
                        })
                        .sum::<f64>()
                        / (samples.len() as f64 * bandwidth * (std::f64::consts::TAU).sqrt());
                    (x, density)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let y_max = curves
        .iter()
        .flatten()
        .map(|(_, density)| *density)
        .fold(0.0, f64::max)
        .max(1e-6);
    let width = option_f64(&opts, "width", 760.0).max(520.0);
    let height = option_f64(&opts, "height", 480.0).max(340.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 70.0;
    canvas.margin.right = 145.0;
    canvas.margin.top = 55.0;
    canvas.margin.bottom = 70.0;
    let x_scale = Scale {
        domain: (x_start, x_end),
        range: (canvas.margin.left, width - canvas.margin.right),
    };
    let y_scale = Scale {
        domain: (0.0, y_max * 1.05),
        range: (height - canvas.margin.bottom, canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    for (index, curve) in curves.iter().enumerate() {
        let mut polygon = vec![(x_scale.map(x_start), y_scale.map(0.0))];
        polygon.extend(
            curve
                .iter()
                .map(|(x, y)| (x_scale.map(*x), y_scale.map(*y))),
        );
        polygon.push((x_scale.map(x_end), y_scale.map(0.0)));
        canvas.add_polygon_with_opacity(&polygon, &palette[index], 0.33);
        let line = curve
            .iter()
            .map(|(x, y)| (x_scale.map(*x), y_scale.map(*y)))
            .collect::<Vec<_>>();
        // ggplot2's geom_density(aes(fill = group)) keeps its default black
        // outline because only fill—not colour—is mapped.
        canvas.add_polyline(&line, "#222222", 1.3);
    }
    canvas.draw_title(option_str(&opts, "title", "Distribution by group"));
    canvas.draw_x_axis(&x_scale, option_str(&opts, "x_label", "Value"));
    canvas.draw_y_axis(&y_scale, option_str(&opts, "y_label", "Density"));
    let legend_x = width - canvas.margin.right + 25.0;
    let legend_top = canvas.margin.top + 10.0;
    legend(
        &mut canvas,
        &levels,
        &palette,
        option_str(&opts, "legend_title", "Group"),
        legend_x,
        legend_top,
    );
    Ok(Value::Str(canvas.render()))
}

fn grouped_bar_plot(args: Vec<Value>) -> Result<Value> {
    let categories = value_list(&args[0], "grouped_bar_plot", "categories")?;
    let groups = value_list(&args[1], "grouped_bar_plot", "groups")?;
    if categories.len() != groups.len() {
        return Err(type_error(
            "grouped_bar_plot() categories and groups must have equal length",
        ));
    }
    let opts = options(&args, 2, "grouped_bar_plot")?;
    let missing = option_str(&opts, "missing_label", "NA");
    let facets = match opts.get("facets") {
        None => vec![Value::Str("All".into()); categories.len()],
        Some(value) => value_list(value, "grouped_bar_plot", "facets")?,
    };
    if facets.len() != categories.len() {
        return Err(type_error(
            "grouped_bar_plot() facets must have the same length as categories",
        ));
    }
    let category_levels = ordered_levels(&categories, missing);
    let group_levels = ordered_levels(&groups, missing);
    let facet_levels = ordered_levels(&facets, missing);
    let palette = colours(&opts, &group_levels, missing)?;
    let mut counts =
        vec![vec![vec![0usize; group_levels.len()]; category_levels.len()]; facet_levels.len()];
    for index in 0..categories.len() {
        let Some(category_name) = category(&categories[index], missing) else {
            continue;
        };
        let Some(group) = category(&groups[index], missing) else {
            continue;
        };
        let Some(facet) = category(&facets[index], missing) else {
            continue;
        };
        let Some(category_index) = category_levels
            .iter()
            .position(|value| value == &category_name)
        else {
            continue;
        };
        let Some(group_index) = group_levels.iter().position(|value| value == &group) else {
            continue;
        };
        let Some(facet_index) = facet_levels.iter().position(|value| value == &facet) else {
            continue;
        };
        counts[facet_index][category_index][group_index] += 1;
    }
    let maximum = counts
        .iter()
        .flatten()
        .flatten()
        .copied()
        .max()
        .unwrap_or(1)
        .max(1);
    let columns = (facet_levels.len() as f64).sqrt().ceil() as usize;
    let rows = facet_levels.len().div_ceil(columns);
    let width = option_f64(&opts, "width", 820.0).max(560.0);
    let height = option_f64(&opts, "height", 250.0 * rows as f64 + 120.0).max(360.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 65.0;
    canvas.margin.right = 145.0;
    canvas.margin.top = 55.0;
    canvas.margin.bottom = 70.0;
    let gap = 16.0;
    let strip = if facet_levels.len() > 1 { 22.0 } else { 0.0 };
    let panel_width =
        (canvas.plot_width() - gap * (columns.saturating_sub(1)) as f64) / columns as f64;
    let panel_height =
        (canvas.plot_height() - gap * (rows.saturating_sub(1)) as f64) / rows as f64 - strip;
    let count_scale = Scale {
        domain: (0.0, maximum as f64 * 1.05),
        range: (panel_height, 0.0),
    };
    let count_ticks = count_scale.nice_ticks(4);
    for facet_index in 0..facet_levels.len() {
        let row = facet_index / columns;
        let column = facet_index % columns;
        let left = canvas.margin.left + column as f64 * (panel_width + gap);
        let strip_top = canvas.margin.top + row as f64 * (panel_height + strip + gap);
        let top = strip_top + strip;
        if strip > 0.0 {
            canvas.add_text(
                left + panel_width / 2.0,
                strip_top + 15.0,
                &facet_levels[facet_index],
                "middle",
                11.0,
            );
        }
        let category_step = panel_width / category_levels.len() as f64;
        let vertical = (0..=category_levels.len())
            .map(|index| left + category_step * index as f64)
            .collect::<Vec<_>>();
        let horizontal = count_ticks
            .iter()
            .map(|tick| top + count_scale.map(*tick))
            .collect::<Vec<_>>();
        draw_panel_grid(
            &mut canvas,
            left,
            top,
            panel_width,
            panel_height,
            &vertical,
            &horizontal,
        );
        if column == 0 {
            for tick in &count_ticks {
                canvas.add_text(
                    left - 7.0,
                    top + count_scale.map(*tick) + 4.0,
                    &format!("{tick:.0}"),
                    "end",
                    canvas.theme.tick_size,
                );
            }
        }
        let bar_width = category_step * 0.8 / group_levels.len() as f64;
        for category_index in 0..category_levels.len() {
            for group_index in 0..group_levels.len() {
                let count = counts[facet_index][category_index][group_index];
                let bar_height = panel_height - count_scale.map(count as f64);
                let x = left
                    + category_step * category_index as f64
                    + category_step * 0.1
                    + bar_width * group_index as f64;
                canvas.add_rect(
                    x,
                    top + panel_height - bar_height,
                    bar_width,
                    bar_height,
                    &palette[group_index],
                );
            }
            canvas.add_text(
                left + category_step * (category_index as f64 + 0.5),
                top + panel_height + 15.0,
                &category_levels[category_index],
                "middle",
                10.0,
            );
        }
    }
    canvas.draw_title(option_str(&opts, "title", "Counts by group"));
    canvas.add_axis_title(
        canvas.margin.left + canvas.plot_width() / 2.0,
        height - 8.0,
        option_str(&opts, "x_label", "Category"),
        "x",
        None,
    );
    canvas.add_axis_title(
        15.0,
        canvas.margin.top + canvas.plot_height() / 2.0,
        option_str(&opts, "y_label", "Count"),
        "y",
        Some(-90.0),
    );
    let legend_x = width - canvas.margin.right + 25.0;
    let legend_top = canvas.margin.top + 10.0;
    legend(
        &mut canvas,
        &group_levels,
        &palette,
        option_str(&opts, "legend_title", "Group"),
        legend_x,
        legend_top,
    );
    Ok(Value::Str(canvas.render()))
}

fn grouped_boxplot_plot(args: Vec<Value>) -> Result<Value> {
    let values = value_list(&args[0], "grouped_boxplot_plot", "values")?;
    let categories = value_list(&args[1], "grouped_boxplot_plot", "categories")?;
    let groups = value_list(&args[2], "grouped_boxplot_plot", "groups")?;
    if values.len() != categories.len() || values.len() != groups.len() {
        return Err(type_error(
            "grouped_boxplot_plot() values, categories, and groups must have equal length",
        ));
    }
    let opts = options(&args, 3, "grouped_boxplot_plot")?;
    let missing = option_str(&opts, "missing_label", "NA");
    let category_levels = ordered_levels(&categories, missing);
    let group_levels = ordered_levels(&groups, missing);
    let palette = colours(&opts, &group_levels, missing)?;
    let mut grouped = vec![vec![Vec::<f64>::new(); group_levels.len()]; category_levels.len()];
    for index in 0..values.len() {
        let Some(value) = numeric(&values[index]) else {
            continue;
        };
        let Some(category_name) = category(&categories[index], missing) else {
            continue;
        };
        let Some(group) = category(&groups[index], missing) else {
            continue;
        };
        if let (Some(category_index), Some(group_index)) = (
            category_levels
                .iter()
                .position(|value| value == &category_name),
            group_levels.iter().position(|value| value == &group),
        ) {
            grouped[category_index][group_index].push(value);
        }
    }
    let pooled = grouped
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    let minimum = pooled.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum = pooled.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !minimum.is_finite() {
        return Err(type_error(
            "grouped_boxplot_plot() has no numeric observations",
        ));
    }
    let padding = ((maximum - minimum) * 0.05).max(1.0);
    let width = option_f64(&opts, "width", 820.0).max(560.0);
    let height = option_f64(&opts, "height", 500.0).max(360.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 70.0;
    canvas.margin.right = 145.0;
    canvas.margin.top = 55.0;
    canvas.margin.bottom = 75.0;
    let y_scale = Scale {
        domain: (minimum - padding, maximum + padding),
        range: (height - canvas.margin.bottom, canvas.margin.top),
    };
    let category_step = canvas.plot_width() / category_levels.len() as f64;
    let group_step = category_step * 0.76 / group_levels.len() as f64;
    canvas.draw_categorical_grid(&y_scale);
    for category_index in 0..category_levels.len() {
        for group_index in 0..group_levels.len() {
            let samples = &grouped[category_index][group_index];
            let Some(summary) = box_summary(samples.clone()) else {
                continue;
            };
            let x = canvas.margin.left
                + category_step * category_index as f64
                + category_step * 0.12
                + group_step * (group_index as f64 + 0.5);
            canvas.add_line(
                x,
                y_scale.map(summary.lower_whisker),
                x,
                y_scale.map(summary.upper_whisker),
                "#333333",
                1.0,
            );
            canvas.add_line(
                x - group_step * 0.22,
                y_scale.map(summary.lower_whisker),
                x + group_step * 0.22,
                y_scale.map(summary.lower_whisker),
                "#333333",
                1.0,
            );
            canvas.add_line(
                x - group_step * 0.22,
                y_scale.map(summary.upper_whisker),
                x + group_step * 0.22,
                y_scale.map(summary.upper_whisker),
                "#333333",
                1.0,
            );
            canvas.add_stroked_rect(
                x - group_step * 0.42,
                y_scale.map(summary.q3),
                group_step * 0.84,
                (y_scale.map(summary.q1) - y_scale.map(summary.q3))
                    .abs()
                    .max(1.0),
                &palette[group_index],
                "#333333",
                1.0,
            );
            canvas.add_line(
                x - group_step * 0.42,
                y_scale.map(summary.median),
                x + group_step * 0.42,
                y_scale.map(summary.median),
                "#333333",
                1.8,
            );
            for outlier in summary.outliers {
                canvas.add_circle_with_opacity(x, y_scale.map(outlier), 2.2, "#333333", 1.0);
            }
        }
    }
    canvas.draw_y_axis(&y_scale, option_str(&opts, "y_label", "Value"));
    canvas.draw_category_axis(&category_levels, option_str(&opts, "x_label", "Category"));
    canvas.draw_title(option_str(&opts, "title", "Grouped boxplots"));
    let legend_x = width - canvas.margin.right + 25.0;
    let legend_top = canvas.margin.top + 10.0;
    legend(
        &mut canvas,
        &group_levels,
        &palette,
        option_str(&opts, "legend_title", "Group"),
        legend_x,
        legend_top,
    );
    Ok(Value::Str(canvas.render()))
}

fn event_timeline_plot(args: Vec<Value>) -> Result<Value> {
    let Value::Table(table) = &args[0] else {
        return Err(type_error(format!(
            "event_timeline_plot() requires Table, got {}",
            args[0].type_of()
        )));
    };
    let opts = options(&args, 1, "event_timeline_plot")?;
    let id_name = option_str(&opts, "id", "case_id");
    let date_name = option_str(&opts, "date", "date");
    let value_name = option_str(&opts, "value", "age");
    let group_name = option_str(&opts, "group", "outcome");
    let facet_name = option_str(&opts, "facet", "province");
    let indices = [id_name, date_name, value_name, group_name, facet_name]
        .into_iter()
        .map(|name| {
            table
                .col_index(name)
                .ok_or_else(|| type_error(format!("event_timeline_plot() missing column '{name}'")))
        })
        .collect::<Result<Vec<_>>>()?;
    let missing = option_str(&opts, "missing_label", "NA");
    let group_values = table
        .rows
        .iter()
        .map(|row| row[indices[3]].clone())
        .collect::<Vec<_>>();
    let facet_values = table
        .rows
        .iter()
        .map(|row| row[indices[4]].clone())
        .collect::<Vec<_>>();
    let group_levels = ordered_levels(&group_values, missing);
    let facet_levels = ordered_levels(&facet_values, missing);
    let palette = colours(&opts, &group_levels, missing)?;
    #[derive(Clone)]
    struct Event {
        id: String,
        day: i64,
        value: f64,
        group: String,
        facet: String,
    }
    let mut events = Vec::new();
    for row in &table.rows {
        let (Some(id), Some(date), Some(value), Some(group), Some(facet)) = (
            row[indices[0]].as_str(),
            row[indices[1]].as_str(),
            numeric(&row[indices[2]]),
            category(&row[indices[3]], missing),
            category(&row[indices[4]], missing),
        ) else {
            continue;
        };
        let Ok(date) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
            continue;
        };
        let origin = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        events.push(Event {
            id: id.to_string(),
            day: (date - origin).num_days(),
            value,
            group,
            facet,
        });
    }
    if events.is_empty() {
        return Err(type_error("event_timeline_plot() has no complete events"));
    }
    let min_day = events.iter().map(|event| event.day).min().unwrap();
    let max_day = events.iter().map(|event| event.day).max().unwrap();
    let min_value = events
        .iter()
        .map(|event| event.value)
        .fold(f64::INFINITY, f64::min);
    let max_value = events
        .iter()
        .map(|event| event.value)
        .fold(f64::NEG_INFINITY, f64::max);
    let columns = (facet_levels.len() as f64).sqrt().ceil() as usize;
    let rows = facet_levels.len().div_ceil(columns);
    let width = option_f64(&opts, "width", 960.0).max(640.0);
    let height = option_f64(&opts, "height", 760.0).max(440.0);
    let theme = crate::plot::stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 65.0;
    canvas.margin.right = 145.0;
    canvas.margin.top = 55.0;
    canvas.margin.bottom = 70.0;
    let gap = 16.0;
    let strip = 22.0;
    let panel_width =
        (canvas.plot_width() - gap * (columns.saturating_sub(1)) as f64) / columns as f64;
    let panel_height =
        (canvas.plot_height() - gap * (rows.saturating_sub(1)) as f64) / rows as f64 - strip;
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let first_date = epoch + chrono::Duration::days(min_day);
    let last_date = epoch + chrono::Duration::days(max_day);
    let mut month =
        chrono::NaiveDate::from_ymd_opt(first_date.year(), first_date.month(), 1).unwrap();
    if month < first_date {
        let (year, next_month) = if month.month() == 12 {
            (month.year() + 1, 1)
        } else {
            (month.year(), month.month() + 1)
        };
        month = chrono::NaiveDate::from_ymd_opt(year, next_month, 1).unwrap();
    }
    let mut month_ticks = Vec::new();
    while month <= last_date {
        month_ticks.push(((month - epoch).num_days(), month.format("%b").to_string()));
        let (year, next_month) = if month.month() == 12 {
            (month.year() + 1, 1)
        } else {
            (month.year(), month.month() + 1)
        };
        month = chrono::NaiveDate::from_ymd_opt(year, next_month, 1).unwrap();
    }
    for (facet_index, facet) in facet_levels.iter().enumerate() {
        let row = facet_index / columns;
        let column = facet_index % columns;
        let left = canvas.margin.left + column as f64 * (panel_width + gap);
        let strip_top = canvas.margin.top + row as f64 * (panel_height + strip + gap);
        let top = strip_top + strip;
        canvas.add_text(
            left + panel_width / 2.0,
            strip_top + 15.0,
            facet,
            "middle",
            11.0,
        );
        let (day_low, day_high) = padded_domain(min_day as f64, max_day as f64, 0.025, 1.0);
        let (value_low, value_high) = padded_domain(min_value, max_value, 0.05, 1.0);
        let x_scale = Scale {
            domain: (day_low, day_high),
            range: (left, left + panel_width),
        };
        let y_scale = Scale {
            domain: (value_low, value_high),
            range: (top + panel_height, top),
        };
        let y_ticks = y_scale.nice_ticks(4);
        let vertical = month_ticks
            .iter()
            .map(|(day, _)| x_scale.map(*day as f64))
            .collect::<Vec<_>>();
        let horizontal = y_ticks
            .iter()
            .map(|tick| y_scale.map(*tick))
            .collect::<Vec<_>>();
        draw_panel_grid(
            &mut canvas,
            left,
            top,
            panel_width,
            panel_height,
            &vertical,
            &horizontal,
        );
        if row + 1 == rows {
            for ((_, label), x) in month_ticks.iter().zip(&vertical) {
                canvas.add_text(
                    *x,
                    top + panel_height + 16.0,
                    label,
                    "middle",
                    canvas.theme.tick_size,
                );
            }
        }
        if column == 0 {
            for tick in &y_ticks {
                canvas.add_text(
                    left - 7.0,
                    y_scale.map(*tick) + 4.0,
                    &format!("{tick:.0}"),
                    "end",
                    canvas.theme.tick_size,
                );
            }
        }
        let mut by_id = HashMap::<String, Vec<&Event>>::new();
        for event in events.iter().filter(|event| &event.facet == facet) {
            by_id.entry(event.id.clone()).or_default().push(event);
        }
        for path in by_id.values_mut() {
            path.sort_by_key(|event| event.day);
            let points = path
                .iter()
                .map(|event| (x_scale.map(event.day as f64), y_scale.map(event.value)))
                .collect::<Vec<_>>();
            let path_group = path
                .first()
                .map(|event| event.group.as_str())
                .unwrap_or(missing);
            let path_group_index = group_levels
                .iter()
                .position(|level| level == path_group)
                .unwrap_or(0);
            canvas.add_polyline(&points, &palette[path_group_index], 0.8);
            for event in path.iter() {
                let group_index = group_levels
                    .iter()
                    .position(|level| level == &event.group)
                    .unwrap_or(0);
                canvas.add_circle_with_opacity(
                    x_scale.map(event.day as f64),
                    y_scale.map(event.value),
                    2.2,
                    &palette[group_index],
                    0.9,
                );
            }
        }
    }
    canvas.draw_title(option_str(&opts, "title", "Event timeline"));
    canvas.add_axis_title(
        canvas.margin.left + canvas.plot_width() / 2.0,
        height - 8.0,
        option_str(&opts, "x_label", "Date"),
        "x",
        None,
    );
    canvas.add_axis_title(
        15.0,
        canvas.margin.top + canvas.plot_height() / 2.0,
        option_str(&opts, "y_label", value_name),
        "y",
        Some(-90.0),
    );
    let legend_x = width - canvas.margin.right + 25.0;
    let legend_top = canvas.margin.top + 10.0;
    legend(
        &mut canvas,
        &group_levels,
        &palette,
        option_str(&opts, "legend_title", group_name),
        legend_x,
        legend_top,
    );
    Ok(Value::Str(canvas.render()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn teaching_table() -> Table {
        Table::new(
            vec!["outcome".into(), "x".into(), "z".into()],
            (0..30)
                .map(|index| {
                    vec![
                        Value::Str(if index < 15 { "Death" } else { "Recover" }.into()),
                        Value::Float(index as f64),
                        Value::Float((index % 3) as f64),
                    ]
                })
                .collect(),
        )
    }

    #[test]
    fn knn_model_round_trips_through_value() {
        let model = train(vec![
            Value::Table(teaching_table()),
            record([
                ("method", Value::Str("knn".into())),
                ("resamples", Value::Int(3)),
            ]),
        ])
        .unwrap();
        let output = predict(vec![model, Value::Table(teaching_table())]).unwrap();
        let Value::List(values) = output else {
            panic!("expected class list")
        };
        assert_eq!(values.len(), 30);
    }

    #[test]
    fn seasonal_forecast_extends_history() {
        let table = Table::new(
            vec!["ds".into(), "y".into()],
            (0..120)
                .map(|index| {
                    vec![
                        Value::Str(
                            (chrono::NaiveDate::from_ymd_opt(2020, 1, 6).unwrap()
                                + chrono::Duration::days(index * 7))
                            .format("%Y-%m-%d")
                            .to_string(),
                        ),
                        Value::Float(20.0 + (index as f64 / 52.0 * std::f64::consts::TAU).sin()),
                    ]
                })
                .collect(),
        );
        let value = seasonal_forecast(vec![
            Value::Table(table),
            record([("periods", Value::Int(12))]),
        ])
        .unwrap();
        let Value::Record(fields) = value else {
            panic!("expected forecast record")
        };
        let Value::Table(forecast) = fields.get("forecast").unwrap() else {
            panic!("expected forecast table")
        };
        assert_eq!(forecast.num_rows(), 132);
    }

    #[test]
    fn seasonal_forecast_can_fit_weekly_history_and_extend_daily_calendar() {
        let start = chrono::NaiveDate::from_ymd_opt(2020, 1, 6).unwrap();
        let table = Table::new(
            vec!["ds".into(), "y".into()],
            (0..120)
                .map(|index| {
                    vec![
                        Value::Str(
                            (start + chrono::Duration::days(index as i64 * 7))
                                .format("%Y-%m-%d")
                                .to_string(),
                        ),
                        Value::Float(10.0 + (index as f64 / 52.1775 * std::f64::consts::TAU).sin()),
                    ]
                })
                .collect(),
        );
        let value = seasonal_forecast(vec![
            Value::Table(table),
            record([
                ("periods", Value::Int(10)),
                ("frequency_days", Value::Int(1)),
                ("period_days", Value::Float(365.25)),
            ]),
        ])
        .unwrap();
        let Value::Record(fields) = value else {
            panic!("expected forecast record")
        };
        assert_eq!(
            fields.get("period_unit").and_then(Value::as_str),
            Some("days")
        );
        let Value::Table(forecast) = fields.get("forecast").unwrap() else {
            panic!("expected forecast table")
        };
        assert_eq!(forecast.num_rows(), 130);
        assert_eq!(
            forecast.rows.last().unwrap()[0].as_str(),
            Some("2022-04-28")
        );
    }

    #[test]
    fn grouped_density_uses_ggplot_fill_colours_with_black_outlines() {
        let svg = grouped_density_plot(vec![
            list([
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(4.0),
            ]),
            list([
                Value::Str("Death".into()),
                Value::Str("Death".into()),
                Value::Str("Recover".into()),
                Value::Str("Recover".into()),
            ]),
            record([(
                "colors",
                list([Value::Str("#F8766D".into()), Value::Str("#00BFC4".into())]),
            )]),
        ])
        .unwrap();
        let Value::Str(svg) = svg else {
            panic!("expected SVG")
        };
        assert!(svg.contains("#F8766D"));
        assert!(svg.contains("#00BFC4"));
        assert!(svg.contains("stroke=\"#222222\""));
    }

    #[test]
    fn importance_plot_has_boxed_frame_zero_guide_and_top_ticks() {
        let svg = importance_plot(vec![Value::Table(Table::new(
            vec!["predictor".into(), "importance".into()],
            vec![
                vec![Value::Str("age".into()), Value::Float(100.0)],
                vec![Value::Str("days".into()), Value::Float(60.0)],
                vec![Value::Str("other".into()), Value::Float(0.0)],
            ],
        ))])
        .unwrap();
        let Value::Str(svg) = svg else {
            panic!("expected SVG")
        };
        assert!(svg.contains("width=\"540.0\" height=\"357.0\" fill=\"none\" stroke="));
        assert!(svg.contains("y1=\"58.0\"") && svg.contains("y2=\"52.0\""));
        // Expansion places zero inside the panel rather than on its border.
        assert!(!svg.contains("x1=\"150.0\" y1=\"58.0\" x2=\"150.0\" y2=\"415.0\""));
        // Every importance stem is anchored to that same interior zero line.
        for y in ["117.5", "236.5", "355.5"] {
            assert!(svg.contains(&format!("<line x1=\"181.1\" y1=\"{y}\"")));
        }
    }

    #[test]
    fn resample_plot_uses_shared_scales_opposite_axis_labels_and_dashed_whiskers() {
        let table = Table::new(
            vec![
                "model".into(),
                "resample".into(),
                "accuracy".into(),
                "kappa".into(),
            ],
            vec![
                vec![
                    Value::Str("random_forest".into()),
                    Value::Int(1),
                    Value::Float(0.70),
                    Value::Float(0.25),
                ],
                vec![
                    Value::Str("random_forest".into()),
                    Value::Int(2),
                    Value::Float(0.82),
                    Value::Float(0.55),
                ],
                vec![
                    Value::Str("knn".into()),
                    Value::Int(1),
                    Value::Float(0.52),
                    Value::Float(-0.15),
                ],
                vec![
                    Value::Str("knn".into()),
                    Value::Int(2),
                    Value::Float(0.68),
                    Value::Float(0.35),
                ],
            ],
        );
        let svg = resample_plot(vec![Value::Table(table)]).unwrap();
        let Value::Str(svg) = svg else {
            panic!("expected SVG")
        };
        assert!(svg.contains("stroke-dasharray=\"6.0,6.0\""));
        assert!(svg.contains(">Accuracy</text>"));
        assert!(svg.contains(">Kappa</text>"));
        // Numeric ticks are printed below Accuracy and above Kappa. A shared
        // lattice scale therefore prints every tick label exactly twice.
        let numeric_tick_labels = svg
            .split("<text")
            .skip(1)
            .filter_map(|element| {
                let (_, content) = element.split_once('>')?;
                let (label, _) = content.split_once("</text>")?;
                label.parse::<f64>().ok().map(|_| label.to_string())
            })
            .collect::<Vec<_>>();
        assert!(numeric_tick_labels.len() >= 4);
        for (index, label) in numeric_tick_labels.iter().enumerate() {
            if !numeric_tick_labels[..index].contains(label) {
                assert_eq!(
                    numeric_tick_labels
                        .iter()
                        .filter(|candidate| *candidate == label)
                        .count(),
                    2
                );
            }
        }
    }

    #[test]
    fn lattice_box_summary_matches_r_fivenum_hinges_and_outlier_rule() {
        let summary = lattice_box_summary(vec![0.0, 1.0, 2.0, 3.0, 100.0]).unwrap();
        assert_eq!(summary.q1, 1.0);
        assert_eq!(summary.median, 2.0);
        assert_eq!(summary.q3, 3.0);
        assert_eq!(summary.lower_whisker, 0.0);
        assert_eq!(summary.upper_whisker, 3.0);
        assert_eq!(summary.outliers, vec![100.0]);
    }

    #[test]
    fn component_plot_draws_the_forecast_uncertainty_ribbon() {
        let start = chrono::NaiveDate::from_ymd_opt(2020, 1, 6).unwrap();
        let table = Table::new(
            vec!["ds".into(), "y".into()],
            (0..120)
                .map(|index| {
                    vec![
                        Value::Str(
                            (start + chrono::Duration::days(index as i64 * 7))
                                .format("%Y-%m-%d")
                                .to_string(),
                        ),
                        Value::Float(20.0 + (index as f64 / 52.1775 * std::f64::consts::TAU).sin()),
                    ]
                })
                .collect(),
        );
        let forecast = seasonal_forecast(vec![
            Value::Table(table),
            record([("periods", Value::Int(12))]),
        ])
        .unwrap();
        let Value::Record(fields) = &forecast else {
            panic!("expected forecast record")
        };
        let Value::Table(components) = fields.get("components").unwrap() else {
            panic!("expected component table")
        };
        assert!(components.col_index("trend_lower").is_some());
        assert!(components.col_index("trend_upper").is_some());
        let svg = seasonal_components_plot(vec![forecast]).unwrap();
        let Value::Str(svg) = svg else {
            panic!("expected SVG")
        };
        assert!(svg.contains("fill=\"#9ECAE1\" fill-opacity=\"0.300\""));
    }

    #[test]
    fn event_timelines_are_inset_from_every_facet_edge() {
        let events = Table::new(
            vec![
                "case_id".into(),
                "date".into(),
                "age".into(),
                "outcome".into(),
                "province".into(),
            ],
            vec![
                vec![
                    Value::Str("case-1".into()),
                    Value::Str("2020-01-01".into()),
                    Value::Int(10),
                    Value::Str("Death".into()),
                    Value::Str("North".into()),
                ],
                vec![
                    Value::Str("case-1".into()),
                    Value::Str("2020-01-11".into()),
                    Value::Int(20),
                    Value::Str("Death".into()),
                    Value::Str("North".into()),
                ],
            ],
        );
        let rendered = event_timeline_plot(vec![
            Value::Table(events),
            record([
                ("width", Value::Int(640)),
                ("height", Value::Int(440)),
                ("facet", Value::Str("province".into())),
            ]),
        ])
        .expect("event timeline");
        let Value::Str(svg) = rendered else {
            panic!("expected SVG")
        };

        // The single panel occupies x=65..495 and y=77..370. Both the points
        // and the path must retain a visible inset after marker radius/stroke.
        let event_circles = svg
            .split("<circle ")
            .skip(1)
            .filter_map(|fragment| {
                let value = |name: &str| {
                    let start = fragment.find(&format!("{name}=\""))? + name.len() + 2;
                    fragment[start..].split('"').next()?.parse::<f64>().ok()
                };
                Some((value("cx")?, value("cy")?))
            })
            .filter(|(x, y)| *x >= 65.0 && *x <= 495.0 && *y >= 77.0 && *y <= 370.0)
            .collect::<Vec<_>>();
        assert_eq!(event_circles.len(), 2);
        assert!(event_circles
            .iter()
            .all(|(x, y)| *x > 70.0 && *x < 490.0 && *y > 82.0 && *y < 365.0));

        let polyline = svg
            .split("<polyline ")
            .nth(1)
            .and_then(|fragment| fragment.split("points=\"").nth(1))
            .and_then(|fragment| fragment.split('"').next())
            .expect("timeline polyline");
        for point in polyline.split_whitespace() {
            let (x, y) = point.split_once(',').expect("polyline coordinate");
            let x = x.parse::<f64>().expect("x coordinate");
            let y = y.parse::<f64>().expect("y coordinate");
            assert!(x > 70.0 && x < 490.0 && y > 82.0 && y < 365.0);
        }
    }
}
