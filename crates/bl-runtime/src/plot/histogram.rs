//! Histogram for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(super) const HISTOGRAM_SCHEMA: &str = "biolang.plot.geometry/v1";

pub(super) const MAX_HISTOGRAM_BINS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HistogramClosure {
    Left,
    Right,
}

impl HistogramClosure {
    fn name(self) -> &'static str {
        match self {
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct HistogramGeometry {
    pub(crate) edges: Vec<f64>,
    pub(crate) counts: Vec<usize>,
    method: String,
    closure: HistogramClosure,
    include_lowest: bool,
    n_total: usize,
    n_finite: usize,
    n_included: usize,
    dropped_invalid: usize,
    dropped_non_finite: usize,
    dropped_outside: usize,
}

pub(super) fn histogram_bool_option(
    opts: &HashMap<String, Value>,
    key: &str,
    default: bool,
) -> bool {
    match opts.get(key) {
        Some(Value::Bool(value)) => *value,
        _ => default,
    }
}

pub(super) fn histogram_values(
    value: &Value,
    who: &str,
) -> Result<(Vec<f64>, usize, usize, usize)> {
    let items = match value {
        Value::List(items) => items,
        _ => {
            return Err(BioLangError::type_error(
                format!("{who}() requires List of numbers"),
                None,
            ))
        }
    };

    let mut values = Vec::with_capacity(items.len());
    let mut invalid = 0usize;
    let mut non_finite = 0usize;
    for item in items.iter() {
        let parsed = match item {
            Value::Int(value) => Some(*value as f64),
            Value::Float(value) => Some(*value),
            Value::Str(value) => value.parse::<f64>().ok(),
            _ => None,
        };
        match parsed {
            Some(value) if value.is_finite() => values.push(value),
            Some(_) => non_finite += 1,
            None => invalid += 1,
        }
    }
    if values.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{who}() received no finite numeric values - check the input and missing-value encoding"
            ),
            None,
        ));
    }
    Ok((values, items.len(), invalid, non_finite))
}

pub(super) fn histogram_bin_count(value: f64, option: &str) -> Result<usize> {
    if !value.is_finite() || value < 1.0 || value.fract() != 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("histogram() option '{option}' must be a positive whole number"),
            None,
        ));
    }
    let bins = value as usize;
    if bins > MAX_HISTOGRAM_BINS {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("histogram() requested {bins} bins; the safety limit is {MAX_HISTOGRAM_BINS}"),
            None,
        ));
    }
    Ok(bins)
}

pub(super) fn histogram_quantile(sorted: &[f64], probability: f64) -> f64 {
    if sorted.len() == 1 {
        return sorted[0];
    }
    let position = probability.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted[lower] + fraction * (sorted[upper] - sorted[lower])
}

pub(super) fn histogram_automatic_bin_count(values: &[f64], method: &str) -> usize {
    let n = values.len().max(1) as f64;
    let sturges = (n.log2() + 1.0).ceil().max(1.0) as usize;
    let (lo, hi) = col_range(values);
    let span = hi - lo;
    if span <= f64::EPSILON {
        return 1;
    }

    let width = match method {
        "freedman-diaconis" => {
            let mut sorted = values.to_vec();
            sorted.sort_by(|a, b| a.total_cmp(b));
            let iqr = histogram_quantile(&sorted, 0.75) - histogram_quantile(&sorted, 0.25);
            2.0 * iqr / n.cbrt()
        }
        "scott" => {
            let mean = values.iter().sum::<f64>() / n;
            let variance = if values.len() > 1 {
                values
                    .iter()
                    .map(|value| (value - mean).powi(2))
                    .sum::<f64>()
                    / (n - 1.0)
            } else {
                0.0
            };
            3.5 * variance.sqrt() / n.cbrt()
        }
        _ => return sturges,
    };
    if !width.is_finite() || width <= f64::EPSILON {
        sturges
    } else {
        ((span / width).ceil() as usize).clamp(1, MAX_HISTOGRAM_BINS)
    }
}

pub(super) fn histogram_equal_edges(values: &[f64], bins: usize) -> Vec<f64> {
    let (mut lo, mut hi) = col_range(values);
    if (hi - lo).abs() < f64::EPSILON {
        let padding = (lo.abs() * 0.01).max(0.5);
        lo -= padding;
        hi += padding;
    }
    let width = (hi - lo) / bins as f64;
    (0..=bins)
        .map(|index| {
            if index == bins {
                hi
            } else {
                lo + index as f64 * width
            }
        })
        .collect()
}

/// ggplot2's `bin_breaks_bins()`, which is not an equal split of the range.
///
/// `bins = n` in ggplot2 uses a width of `range / (n - 1)` and a boundary of
/// half a width, so the first bin is centred on the minimum and the outer
/// edges sit half a bin beyond the data. Cutting `[min, max]` into `n` equal
/// parts instead — the matplotlib and `hist(breaks = n)` reading — gives
/// different bar widths and different counts from the same `bins` value.
pub(crate) fn histogram_ggplot_edges(values: &[f64], bins: usize) -> Vec<f64> {
    let (mut lo, mut hi) = col_range(values);
    if (hi - lo).abs() < f64::EPSILON {
        let padding = (lo.abs() * 0.01).max(0.5);
        lo -= padding;
        hi += padding;
    }
    if bins < 2 {
        return vec![lo, hi];
    }
    let width = (hi - lo) / (bins - 1) as f64;
    let boundary = width / 2.0;
    // find_origin(): the boundary-aligned edge at or below the minimum.
    let origin = boundary + ((lo - boundary) / width).floor() * width;
    // ggplot2 nudges the upper limit so an exact multiple does not add a bin.
    let limit = hi + (1.0 - 1e-8) * width;
    let breaks = (((limit - origin) / width).floor() as i64 + 1).max(2) as usize;
    (0..breaks)
        .map(|index| origin + index as f64 * width)
        .collect()
}

pub(super) fn histogram_explicit_edges(items: &[Value]) -> Result<Vec<f64>> {
    let mut edges = Vec::with_capacity(items.len());
    for item in items {
        let edge = match item {
            Value::Int(value) => *value as f64,
            Value::Float(value) => *value,
            Value::Str(value) => value.parse::<f64>().map_err(|_| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "histogram() explicit breaks must all be numeric",
                    None,
                )
            })?,
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "histogram() explicit breaks must all be numeric",
                    None,
                ))
            }
        };
        if !edge.is_finite() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "histogram() explicit breaks must be finite",
                None,
            ));
        }
        edges.push(edge);
    }
    if edges.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "histogram() explicit breaks require at least two edges",
            None,
        ));
    }
    if edges.len() - 1 > MAX_HISTOGRAM_BINS {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("histogram() explicit breaks exceed the {MAX_HISTOGRAM_BINS}-bin limit"),
            None,
        ));
    }
    if edges.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "histogram() explicit breaks must be strictly increasing",
            None,
        ));
    }
    Ok(edges)
}

pub(crate) fn histogram_geometry(args: &[Value], who: &str) -> Result<HistogramGeometry> {
    let opts = parse_options(args);
    let (values, n_total, dropped_invalid, dropped_non_finite) = histogram_values(&args[0], who)?;
    let closure = match opts.get("closed").and_then(Value::as_str) {
        Some("left") => HistogramClosure::Left,
        Some("right") => HistogramClosure::Right,
        Some(other) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("histogram() option 'closed' must be 'left' or 'right', got '{other}'"),
                None,
            ))
        }
        None if histogram_bool_option(&opts, "right", false) => HistogramClosure::Right,
        None => HistogramClosure::Left,
    };
    let include_lowest = histogram_bool_option(&opts, "include_lowest", true);

    let (edges, method) = match opts.get("breaks") {
        Some(Value::List(items)) => (histogram_explicit_edges(items)?, "explicit".to_string()),
        Some(Value::Int(value)) => {
            let bins = histogram_bin_count(*value as f64, "breaks")?;
            (
                histogram_equal_edges(&values, bins),
                format!("equal-width:{bins}"),
            )
        }
        Some(Value::Float(value)) => {
            let bins = histogram_bin_count(*value, "breaks")?;
            (
                histogram_equal_edges(&values, bins),
                format!("equal-width:{bins}"),
            )
        }
        Some(Value::Str(value)) => {
            let method = match value.to_ascii_lowercase().as_str() {
                "sturges" => "sturges",
                "fd" | "freedman-diaconis" | "freedman_diaconis" => "freedman-diaconis",
                "scott" => "scott",
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "histogram() unknown break rule '{value}'; use 'sturges', 'freedman-diaconis', 'scott', a bin count, or an explicit List"
                        ),
                        None,
                    ))
                }
            };
            let bins = histogram_automatic_bin_count(&values, method);
            (
                histogram_equal_edges(&values, bins),
                format!("{method}:equal-width:{bins}"),
            )
        }
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "histogram() option 'breaks' must be a rule name, bin count, or List of edges",
                None,
            ))
        }
        None => {
            let bins = match opts.get("bins") {
                Some(Value::Int(value)) => histogram_bin_count(*value as f64, "bins")?,
                Some(Value::Float(value)) => histogram_bin_count(*value, "bins")?,
                Some(_) => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "histogram() option 'bins' must be a positive whole number",
                        None,
                    ))
                }
                None => 20,
            };
            // ggplot2's reading of `bins`; `span` keeps the equal split of
            // the range that matplotlib and `hist(breaks = n)` use.
            match opts
                .get("bin_rule")
                .and_then(Value::as_str)
                .unwrap_or("ggplot")
            {
                "span" => (
                    histogram_equal_edges(&values, bins),
                    format!("equal-width:{bins}"),
                ),
                "ggplot" | "ggplot2" => (
                    histogram_ggplot_edges(&values, bins),
                    format!("ggplot:{bins}"),
                ),
                other => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                        "histogram() option 'bin_rule' must be 'span' or 'ggplot', got '{other}'"
                    ),
                        None,
                    ))
                }
            }
        }
    };

    let bins = edges.len() - 1;
    let first = edges[0];
    let last = edges[bins];
    let mut counts = vec![0usize; bins];
    let mut dropped_outside = 0usize;
    for value in &values {
        let index = match closure {
            HistogramClosure::Left => {
                if *value < first || *value > last || (*value == last && !include_lowest) {
                    None
                } else if *value == last {
                    Some(bins - 1)
                } else {
                    let upper = edges.partition_point(|edge| *edge <= *value);
                    Some(upper.saturating_sub(1).min(bins - 1))
                }
            }
            HistogramClosure::Right => {
                if *value < first || *value > last || (*value == first && !include_lowest) {
                    None
                } else if *value == first {
                    Some(0)
                } else {
                    let lower = edges.partition_point(|edge| *edge < *value);
                    Some(lower.saturating_sub(1).min(bins - 1))
                }
            }
        };
        if let Some(index) = index {
            counts[index] += 1;
        } else {
            dropped_outside += 1;
        }
    }
    let n_included = counts.iter().sum();

    Ok(HistogramGeometry {
        edges,
        counts,
        method,
        closure,
        include_lowest,
        n_total,
        n_finite: values.len(),
        n_included,
        dropped_invalid,
        dropped_non_finite,
        dropped_outside,
    })
}

pub(super) fn histogram_geometry_value(geometry: &HistogramGeometry) -> Value {
    let mut cumulative = 0usize;
    let rows = geometry
        .counts
        .iter()
        .enumerate()
        .map(|(index, count)| {
            cumulative += count;
            let left = geometry.edges[index];
            let right = geometry.edges[index + 1];
            let width = right - left;
            let density = if geometry.n_included == 0 || width <= 0.0 {
                0.0
            } else {
                *count as f64 / (geometry.n_included as f64 * width)
            };
            let cumulative_fraction = if geometry.n_included == 0 {
                0.0
            } else {
                cumulative as f64 / geometry.n_included as f64
            };
            let left_closed = geometry.closure == HistogramClosure::Left
                || (index == 0 && geometry.include_lowest);
            let right_closed = geometry.closure == HistogramClosure::Right
                || (index + 1 == geometry.counts.len() && geometry.include_lowest);
            vec![
                Value::Int(index as i64),
                Value::Float(left),
                Value::Float(right),
                Value::Bool(left_closed),
                Value::Bool(right_closed),
                Value::Int(*count as i64),
                Value::Float(density),
                Value::Int(cumulative as i64),
                Value::Float(cumulative_fraction),
            ]
        })
        .collect();
    let table = Table::new(
        [
            "bin",
            "left",
            "right",
            "left_closed",
            "right_closed",
            "count",
            "density",
            "cumulative_count",
            "cumulative_fraction",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        rows,
    );
    let record = HashMap::from([
        ("schema".into(), Value::Str(HISTOGRAM_SCHEMA.into())),
        ("kind".into(), Value::Str("histogram".into())),
        ("method".into(), Value::Str(geometry.method.clone())),
        ("closure".into(), Value::Str(geometry.closure.name().into())),
        (
            "include_lowest".into(),
            Value::Bool(geometry.include_lowest),
        ),
        ("n_total".into(), Value::Int(geometry.n_total as i64)),
        ("n_finite".into(), Value::Int(geometry.n_finite as i64)),
        ("n_included".into(), Value::Int(geometry.n_included as i64)),
        (
            "dropped_invalid".into(),
            Value::Int(geometry.dropped_invalid as i64),
        ),
        (
            "dropped_non_finite".into(),
            Value::Int(geometry.dropped_non_finite as i64),
        ),
        (
            "dropped_outside".into(),
            Value::Int(geometry.dropped_outside as i64),
        ),
        ("bins".into(), Value::Table(table)),
    ]);
    Value::Record(record.into())
}

pub(super) fn builtin_histogram_data(args: Vec<Value>) -> Result<Value> {
    let geometry = histogram_geometry(&args, "histogram_data")?;
    Ok(histogram_geometry_value(&geometry))
}

pub(super) fn builtin_histogram(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Histogram").to_string();
    let geometry = histogram_geometry(&args, "histogram")?;
    let max_count = geometry.counts.iter().copied().max().unwrap_or(0).max(1);

    let theme = stats_plot_theme(&opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let x_scale = Scale {
        domain: (geometry.edges[0], *geometry.edges.last().unwrap()),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, max_count as f64),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // The default theme keeps its historical bare panel; a named theme draws
    // the panel and grid it implies.
    if !matches!(theme.kind, PlotThemeKind::Legacy) {
        canvas.draw_cartesian_grid(&x_scale, &y_scale);
    }

    // ggplot2 `geom_histogram()` fills with grey35 and draws no border, so its
    // bars abut instead of being separated by a gap.
    let ggplot_like = matches!(theme.kind, PlotThemeKind::Ggplot);
    let bar_fill = if ggplot_like { "#595959" } else { PALETTE[0] };
    let bar_gap = if ggplot_like { 0.0 } else { 1.0 };
    for (index, count) in geometry.counts.iter().enumerate() {
        let x = x_scale.map(geometry.edges[index]);
        let right = x_scale.map(geometry.edges[index + 1]);
        let y = y_scale.map(*count as f64);
        let height = canvas.margin.top + canvas.plot_height() - y;
        canvas.add_rect(x, y, (right - x - bar_gap).max(0.0), height, bar_fill);
    }

    let data_x_scale = Scale {
        domain: x_scale.domain,
        range: x_scale.domain,
    };
    let data_y_scale = Scale {
        domain: (0.0, max_count as f64),
        range: (0.0, max_count as f64),
    };
    canvas.draw_x_axis(&data_x_scale, &axis_label(&opts, "xlabel", "Value"));
    canvas.draw_y_axis(&data_y_scale, &axis_label(&opts, "ylabel", "Count"));
    canvas.draw_title(&title);

    Ok(Value::Str(canvas.render()))
}
