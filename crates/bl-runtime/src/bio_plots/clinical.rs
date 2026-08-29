//! Clinical for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct SurvivalStep {
    time: f64,
    n_risk: usize,
    n_event: usize,
    n_censor: usize,
    survival: f64,
    std_error: f64,
}

#[derive(Clone, Debug)]
pub(super) struct SurvivalGroup {
    name: String,
    sample_count: usize,
    event_count: usize,
    censor_count: usize,
    median_survival: Option<f64>,
    steps: Vec<SurvivalStep>,
}

pub(super) fn kaplan_meier_groups(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<Vec<SurvivalGroup>> {
    let times = extract_table_col(table, get_opt_str(opts, "time", "time"))?;
    let events = extract_table_col(table, get_opt_str(opts, "event", "event"))?;
    if times.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "kaplan_meier() requires at least one observation",
            None,
        ));
    }
    let labels = if let Some(Value::Str(column)) = opts.get("group") {
        extract_str_col(table, column)?
    } else {
        vec!["All".into(); times.len()]
    };
    let mut group_names = Vec::<String>::new();
    let mut grouped = Vec::<Vec<(f64, bool)>>::new();
    let mut lookup = HashMap::<String, usize>::new();
    for ((&time, &event), label) in times.iter().zip(&events).zip(labels) {
        if !time.is_finite() || time < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "kaplan_meier() times must be finite and non-negative",
                None,
            ));
        }
        if !event.is_finite() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "kaplan_meier() event values must be finite",
                None,
            ));
        }
        let next = group_names.len();
        let index = *lookup.entry(label.clone()).or_insert_with(|| {
            group_names.push(label);
            grouped.push(Vec::new());
            next
        });
        grouped[index].push((time, event >= 1.0));
    }
    let mut result = Vec::with_capacity(grouped.len());
    for (name, mut observations) in group_names.into_iter().zip(grouped) {
        observations.sort_by(|left, right| left.0.total_cmp(&right.0));
        let sample_count = observations.len();
        let mut at_risk = sample_count;
        let mut survival = 1.0;
        let mut greenwood_sum = 0.0;
        let mut steps = vec![SurvivalStep {
            time: 0.0,
            n_risk: sample_count,
            n_event: 0,
            n_censor: 0,
            survival,
            std_error: 0.0,
        }];
        let mut event_count = 0usize;
        let mut censor_count = 0usize;
        let mut median_survival = None;
        let mut index = 0usize;
        while index < observations.len() {
            let time = observations[index].0;
            let mut events_at_time = 0usize;
            let mut censored_at_time = 0usize;
            while index < observations.len() && observations[index].0 == time {
                if observations[index].1 {
                    events_at_time += 1;
                } else {
                    censored_at_time += 1;
                }
                index += 1;
            }
            if events_at_time > 0 {
                survival *= 1.0 - events_at_time as f64 / at_risk as f64;
                if at_risk > events_at_time {
                    greenwood_sum += events_at_time as f64
                        / (at_risk as f64 * (at_risk - events_at_time) as f64);
                }
                if median_survival.is_none() && survival <= 0.5 {
                    median_survival = Some(time);
                }
            }
            event_count += events_at_time;
            censor_count += censored_at_time;
            steps.push(SurvivalStep {
                time,
                n_risk: at_risk,
                n_event: events_at_time,
                n_censor: censored_at_time,
                survival,
                std_error: if survival > 0.0 {
                    survival * greenwood_sum.sqrt()
                } else {
                    0.0
                },
            });
            at_risk -= events_at_time + censored_at_time;
        }
        result.push(SurvivalGroup {
            name,
            sample_count,
            event_count,
            censor_count,
            median_survival,
            steps,
        });
    }
    Ok(result)
}

// ── 9. forest_plot ──────────────────────────────────────────────

pub(super) fn survival_plot_spec_value(
    groups: &[SurvivalGroup],
    opts: &HashMap<String, Value>,
) -> Value {
    let mut rows = Vec::new();
    let mut summaries = Vec::new();
    for (group_index, group) in groups.iter().enumerate() {
        for (step_index, step) in group.steps.iter().enumerate() {
            rows.push(vec![
                Value::Int(group_index as i64),
                Value::Str(group.name.clone()),
                Value::Int(step_index as i64),
                Value::Float(step.time),
                Value::Int(step.n_risk as i64),
                Value::Int(step.n_event as i64),
                Value::Int(step.n_censor as i64),
                Value::Float(step.survival),
                Value::Float(step.std_error),
            ]);
        }
        summaries.push(vec![
            Value::Int(group_index as i64),
            Value::Str(group.name.clone()),
            Value::Int(group.sample_count as i64),
            Value::Int(group.event_count as i64),
            Value::Int(group.censor_count as i64),
            group
                .median_survival
                .map(Value::Float)
                .unwrap_or(Value::Nil),
        ]);
    }
    let title = get_opt_str(opts, "title", "Kaplan-Meier");
    let options = HashMap::from([
        ("title".into(), Value::Str(title.into())),
        (
            "subtitle".into(),
            Value::Str(get_opt_str(opts, "subtitle", "").into()),
        ),
        (
            "caption".into(),
            Value::Str(get_opt_str(opts, "caption", "").into()),
        ),
        (
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        (
            "xlabel".into(),
            Value::Str(get_opt_str(opts, "xlabel", "Time").into()),
        ),
        (
            "ylabel".into(),
            Value::Str(get_opt_str(opts, "ylabel", "Survival probability").into()),
        ),
        (
            "censor_marks".into(),
            Value::Bool(
                opts.get("censor_marks")
                    .and_then(Value::as_bool)
                    .unwrap_or(true),
            ),
        ),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 640.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 440.0)),
        ),
    ]);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("survival".into())),
            ("plot".into(), Value::Str("kaplan_meier".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "group_index",
                        "group",
                        "step_index",
                        "time",
                        "n_risk",
                        "n_event",
                        "n_censor",
                        "survival",
                        "std_error",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "groups".into(),
                Value::Table(Table::new(
                    [
                        "group_index",
                        "group",
                        "sample_count",
                        "event_count",
                        "censor_count",
                        "median_survival",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    summaries,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("kaplan_meier".into())),
                        ("estimator".into(), Value::Str("product-limit".into())),
                        (
                            "tie_policy".into(),
                            Value::Str("events-and-censors-at-each-distinct-time".into()),
                        ),
                        ("standard_error".into(), Value::Str("Greenwood".into())),
                        (
                            "samples".into(),
                            Value::Int(
                                groups.iter().map(|group| group.sample_count).sum::<usize>() as i64,
                            ),
                        ),
                        ("groups".into(), Value::Int(groups.len() as i64)),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

pub(super) fn render_survival_svg(
    groups: &[SurvivalGroup],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    if groups.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() survival specification has no groups",
            None,
        ));
    }
    let width = get_opt_f64(opts, "width", 640.0);
    let height = get_opt_f64(opts, "height", 440.0);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    canvas.margin.left = 62.0_f64.min(width * 0.23);
    canvas.margin.right = if groups.len() > 1 {
        130.0_f64.min(width * 0.30)
    } else {
        20.0
    };
    canvas.margin.top = if subtitle.is_empty() { 52.0 } else { 70.0 };
    canvas.margin.bottom = if caption.is_empty() { 52.0 } else { 70.0 };
    let tmax = groups
        .iter()
        .flat_map(|group| group.steps.iter().map(|step| step.time))
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    let xs = Scale {
        domain: (0.0, tmax),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let ys = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let censor_marks = opts
        .get("censor_marks")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    for (group_index, group) in groups.iter().enumerate() {
        let colour = PALETTE[group_index % PALETTE.len()];
        let mut previous_survival = 1.0;
        let mut path = format!("M {:.2} {:.2}", xs.map(0.0), ys.map(previous_survival));
        let mut censor_path = String::new();
        for step in group.steps.iter().skip(1) {
            path.push_str(&format!(" H {:.2}", xs.map(step.time)));
            if step.survival != previous_survival {
                path.push_str(&format!(" V {:.2}", ys.map(step.survival)));
            }
            if censor_marks && step.n_censor > 0 {
                let x = xs.map(step.time);
                let y = ys.map(step.survival);
                censor_path.push_str(&format!(
                    " M {:.2} {:.2} H {:.2} M {:.2} {:.2} V {:.2}",
                    x - 4.0,
                    y,
                    x + 4.0,
                    x,
                    y - 4.0,
                    y + 4.0
                ));
            }
            previous_survival = step.survival;
        }
        path.push_str(&format!(" H {:.2}", xs.map(tmax)));
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{colour}" stroke-width="2" />"#
        ));
        if !censor_path.is_empty() {
            canvas.elements.push(format!(
                r#"<path d="{censor_path}" fill="none" stroke="{colour}" stroke-width="1.5" />"#
            ));
        }
        if groups.len() > 1 {
            let legend_x = canvas.margin.left + canvas.plot_width() + 12.0;
            let legend_y = canvas.margin.top + 16.0 + group_index as f64 * 20.0;
            canvas.add_line(legend_x, legend_y, legend_x + 18.0, legend_y, colour, 2.0);
            canvas.add_text(legend_x + 24.0, legend_y + 4.0, &group.name, "start", 10.0);
        }
    }
    canvas.draw_x_axis(
        &Scale {
            domain: (0.0, tmax),
            range: (0.0, tmax),
        },
        get_opt_str(opts, "xlabel", "Time"),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, 1.0),
            range: (0.0, 1.0),
        },
        get_opt_str(opts, "ylabel", "Survival probability"),
    );
    canvas.draw_title(get_opt_str(opts, "title", "Kaplan-Meier"));
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Kaplan-Meier product-limit curves for {} group(s), including censor marks.",
        groups.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_survival_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "survival")
    )
}

pub(crate) fn render_survival_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_survival_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 survival Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "group_index",
        "group",
        "step_index",
        "time",
        "n_risk",
        "n_event",
        "n_censor",
        "survival",
        "std_error",
    ] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() survival data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| data.col_index(name).unwrap();
    let mut groups = Vec::<SurvivalGroup>::new();
    for row in &data.rows {
        let number = |name: &str| -> Result<f64> {
            row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() survival field '{name}' must be numeric"),
                    None,
                )
            })
        };
        let group_index =
            frozen_nonnegative_integer(&row[column("group_index")], "survival", "group_index")?;
        if group_index > groups.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival group_index values must be contiguous",
                None,
            ));
        }
        if group_index == groups.len() {
            groups.push(SurvivalGroup {
                name: format!("{}", row[column("group")]),
                sample_count: 0,
                event_count: 0,
                censor_count: 0,
                median_survival: None,
                steps: Vec::new(),
            });
        }
        let group = &mut groups[group_index];
        if frozen_nonnegative_integer(&row[column("step_index")], "survival", "step_index")?
            != group.steps.len()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival step_index values must be contiguous within each group",
                None,
            ));
        }
        let step = SurvivalStep {
            time: number("time")?,
            n_risk: frozen_nonnegative_integer(&row[column("n_risk")], "survival", "n_risk")?,
            n_event: frozen_nonnegative_integer(&row[column("n_event")], "survival", "n_event")?,
            n_censor: frozen_nonnegative_integer(&row[column("n_censor")], "survival", "n_censor")?,
            survival: number("survival")?,
            std_error: number("std_error")?,
        };
        if !step.time.is_finite()
            || step.time < 0.0
            || !step.survival.is_finite()
            || !(0.0..=1.0).contains(&step.survival)
            || !step.std_error.is_finite()
            || step.std_error < 0.0
            || group.steps.last().is_some_and(|previous| {
                step.time < previous.time || step.survival > previous.survival
            })
            || step.n_event + step.n_censor > step.n_risk
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival steps must have finite non-decreasing times and non-increasing probabilities in [0, 1]",
                None,
            ));
        }
        if let Some(previous) = group.steps.last() {
            let expected_risk = previous
                .n_risk
                .saturating_sub(previous.n_event + previous.n_censor);
            if step.n_risk != expected_risk {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() survival risk sets do not follow the preceding event/censor counts",
                    None,
                ));
            }
            let expected_survival = if step.n_risk == 0 {
                previous.survival
            } else {
                previous.survival * (1.0 - step.n_event as f64 / step.n_risk as f64)
            };
            if (step.survival - expected_survival).abs() > 1e-10 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() survival probability does not match its frozen risk/event counts",
                    None,
                ));
            }
        } else if step.time != 0.0
            || step.n_event != 0
            || step.n_censor != 0
            || step.survival != 1.0
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() survival curves must begin at time 0 with probability 1",
                None,
            ));
        }
        group.sample_count = group.sample_count.max(step.n_risk);
        group.event_count += step.n_event;
        group.censor_count += step.n_censor;
        if group.median_survival.is_none() && step.survival <= 0.5 {
            group.median_survival = Some(step.time);
        }
        group.steps.push(step);
    }
    if groups.is_empty() || groups.iter().any(|group| group.steps.is_empty()) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() survival specification has no complete curve",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "survival")?;
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_survival_svg(&groups, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Kaplan-Meier");
    finish_frozen_bio_plot(value, render_options, title, "survival", svg)
}

pub(super) fn builtin_kaplan_meier(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "kaplan_meier")?;
    let opts = parse_options(&args);
    let groups = kaplan_meier_groups(table, &opts)?;
    let specification = survival_plot_spec_value(&groups, &opts);
    render_survival_plot_spec_value(&specification, &opts)
}

// ── 10. roc_curve ───────────────────────────────────────────────

#[derive(Clone, Debug)]
pub(super) struct ForestInterval {
    source_row: usize,
    label: String,
    estimate: f64,
    lower: f64,
    upper: f64,
    weight: f64,
}

pub(super) fn forest_intervals(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<Vec<ForestInterval>> {
    let labels = extract_str_col(table, get_opt_str(opts, "label", "label"))?;
    let estimates = extract_table_col(table, get_opt_str(opts, "estimate", "estimate"))?;
    let lowers = extract_table_col(table, get_opt_str(opts, "lower", "lower"))?;
    let uppers = extract_table_col(table, get_opt_str(opts, "upper", "upper"))?;
    let weights = if let Some(Value::Str(column)) = opts.get("weight") {
        extract_table_col(table, column)?
    } else {
        vec![1.0; labels.len()]
    };
    if labels.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "forest_plot() requires at least one interval",
            None,
        ));
    }
    let log_scale = get_opt_str(opts, "scale", "linear").eq_ignore_ascii_case("log");
    if !log_scale && !get_opt_str(opts, "scale", "linear").eq_ignore_ascii_case("linear") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "forest_plot() scale must be linear or log",
            None,
        ));
    }
    labels
        .into_iter()
        .zip(estimates)
        .zip(lowers)
        .zip(uppers)
        .zip(weights)
        .enumerate()
        .map(|(source_row, ((((label, estimate), lower), upper), weight))| {
            if !estimate.is_finite()
                || !lower.is_finite()
                || !upper.is_finite()
                || !weight.is_finite()
                || weight <= 0.0
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "forest_plot() estimates, interval bounds and weights must be finite; weights must be positive",
                    None,
                ));
            }
            if lower > estimate || estimate > upper {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "forest_plot() row {source_row} must satisfy lower <= estimate <= upper"
                    ),
                    None,
                ));
            }
            if log_scale && lower <= 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "forest_plot() log scale requires positive estimates and interval bounds",
                    None,
                ));
            }
            Ok(ForestInterval {
                source_row,
                label,
                estimate,
                lower,
                upper,
                weight,
            })
        })
        .collect()
}

pub(super) fn forest_domain(
    intervals: &[ForestInterval],
    reference: f64,
    log_scale: bool,
) -> ((f64, f64), (f64, f64)) {
    let transform = |value: f64| if log_scale { value.ln() } else { value };
    let raw_min = intervals
        .iter()
        .map(|interval| interval.lower)
        .fold(reference, f64::min);
    let raw_max = intervals
        .iter()
        .map(|interval| interval.upper)
        .fold(reference, f64::max);
    let transformed_min = transform(raw_min);
    let transformed_max = transform(raw_max);
    let padding = ((transformed_max - transformed_min).abs() * 0.06).max(0.1);
    (
        (raw_min, raw_max),
        (transformed_min - padding, transformed_max + padding),
    )
}

pub(super) fn forest_plot_spec_value(
    intervals: &[ForestInterval],
    opts: &HashMap<String, Value>,
) -> Value {
    let scale = get_opt_str(opts, "scale", "linear").to_ascii_lowercase();
    let reference_default = if scale == "log" { 1.0 } else { 0.0 };
    let reference = get_opt_f64(opts, "reference", reference_default);
    let (raw_domain, display_domain) = forest_domain(intervals, reference, scale.as_str() == "log");
    let title = get_opt_str(opts, "title", "Forest Plot");
    let rows = intervals
        .iter()
        .enumerate()
        .map(|(display_row, interval)| {
            vec![
                Value::Int(display_row as i64),
                Value::Int(interval.source_row as i64),
                Value::Str(interval.label.clone()),
                Value::Float(interval.estimate),
                Value::Float(interval.lower),
                Value::Float(interval.upper),
                Value::Float(interval.weight),
            ]
        })
        .collect();
    let height_default = (intervals.len() as f64 * 32.0 + 110.0).clamp(220.0, 1200.0);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("forest".into())),
            ("plot".into(), Value::Str("forest_plot".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "display_row",
                        "source_row",
                        "label",
                        "estimate",
                        "lower",
                        "upper",
                        "weight",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Effect size").into()),
                        ),
                        ("scale".into(), Value::Str(scale)),
                        ("reference".into(), Value::Float(reference)),
                        ("raw_min".into(), Value::Float(raw_domain.0)),
                        ("raw_max".into(), Value::Float(raw_domain.1)),
                        ("display_min".into(), Value::Float(display_domain.0)),
                        ("display_max".into(), Value::Float(display_domain.1)),
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 680.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", height_default)),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("forest_plot".into())),
                        ("intervals".into(), Value::Int(intervals.len() as i64)),
                        (
                            "marker_area".into(),
                            Value::Str("proportional-to-weight".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

pub(super) fn render_forest_svg(
    intervals: &[ForestInterval],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    if intervals.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() forest specification has no intervals",
            None,
        ));
    }
    let width = get_opt_f64(opts, "width", 680.0);
    let height_default = (intervals.len() as f64 * 32.0 + 110.0).clamp(220.0, 1200.0);
    let height = get_opt_f64(opts, "height", height_default);
    let scale = get_opt_str(opts, "scale", "linear");
    let log_scale = scale == "log";
    let reference = get_opt_f64(opts, "reference", if log_scale { 1.0 } else { 0.0 });
    if !reference.is_finite() || (log_scale && reference <= 0.0) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() forest reference must be finite and positive on a log scale",
            None,
        ));
    }
    let transform = |value: f64| if log_scale { value.ln() } else { value };
    let display_domain = (
        get_opt_f64(opts, "display_min", f64::NAN),
        get_opt_f64(opts, "display_max", f64::NAN),
    );
    if !display_domain.0.is_finite()
        || !display_domain.1.is_finite()
        || display_domain.0 >= display_domain.1
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() forest display domain must be finite and increasing",
            None,
        ));
    }
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let widest_label = intervals
        .iter()
        .map(|interval| estimate_text_width(&interval.label, theme.tick_size))
        .fold(0.0, f64::max);
    canvas.margin.left = (widest_label + 18.0).clamp(82.0, width * 0.38);
    canvas.margin.right = 20.0;
    canvas.margin.top = if subtitle.is_empty() { 54.0 } else { 72.0 };
    canvas.margin.bottom = if caption.is_empty() { 54.0 } else { 72.0 };
    let xs = Scale {
        domain: display_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let row_height = canvas.plot_height() / intervals.len() as f64;
    canvas.add_line(
        xs.map(transform(reference)),
        canvas.margin.top,
        xs.map(transform(reference)),
        canvas.margin.top + canvas.plot_height(),
        theme.grid_colour,
        1.2,
    );
    let max_weight = intervals
        .iter()
        .map(|interval| interval.weight)
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    for (row, interval) in intervals.iter().enumerate() {
        let y = canvas.margin.top + (row as f64 + 0.5) * row_height;
        canvas.add_line(
            xs.map(transform(interval.lower)),
            y,
            xs.map(transform(interval.upper)),
            y,
            PALETTE[0],
            2.0,
        );
        canvas.add_line(
            xs.map(transform(interval.lower)),
            y - 4.0,
            xs.map(transform(interval.lower)),
            y + 4.0,
            PALETTE[0],
            1.2,
        );
        canvas.add_line(
            xs.map(transform(interval.upper)),
            y - 4.0,
            xs.map(transform(interval.upper)),
            y + 4.0,
            PALETTE[0],
            1.2,
        );
        let radius = 3.5 + 4.5 * (interval.weight / max_weight).sqrt();
        canvas.add_circle(xs.map(transform(interval.estimate)), y, radius, PALETTE[0]);
        canvas.add_text(
            canvas.margin.left - 8.0,
            y + theme.tick_size * 0.35,
            &interval.label,
            "end",
            theme.tick_size,
        );
    }
    if log_scale {
        let y = canvas.margin.top + canvas.plot_height();
        canvas.add_line(
            canvas.margin.left,
            y,
            canvas.margin.left + canvas.plot_width(),
            y,
            theme.axis_colour,
            1.0,
        );
        let divisions = if width < 400.0 { 2 } else { 4 };
        let mut ticks = (0..=divisions)
            .map(|index| {
                display_domain.0
                    + (display_domain.1 - display_domain.0) * index as f64 / divisions as f64
            })
            .collect::<Vec<_>>();
        ticks.push(transform(reference));
        ticks.sort_by(f64::total_cmp);
        ticks.dedup_by(|left, right| (*left - *right).abs() < 1e-8);
        if width < 400.0 {
            let reference_tick = transform(reference);
            let reference_x = xs.map(reference_tick);
            ticks.retain(|tick| {
                (*tick - reference_tick).abs() < 1e-8 || (xs.map(*tick) - reference_x).abs() >= 42.0
            });
        }
        for tick in ticks {
            let x = xs.map(tick);
            canvas.add_line(x, y, x, y + 5.0, theme.axis_colour, 1.0);
            canvas.add_text(
                x,
                y + 18.0,
                &format!("{:.2}", tick.exp()),
                "middle",
                theme.tick_size,
            );
        }
        canvas.add_text(
            canvas.margin.left + canvas.plot_width() / 2.0,
            height - 12.0,
            get_opt_str(opts, "xlabel", "Effect size"),
            "middle",
            theme.axis_title_size,
        );
    } else {
        canvas.draw_x_axis(
            &Scale {
                domain: display_domain,
                range: display_domain,
            },
            get_opt_str(opts, "xlabel", "Effect size"),
        );
    }
    canvas.draw_title(get_opt_str(opts, "title", "Forest Plot"));
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Forest plot of {} estimates and confidence intervals; marker area is proportional to weight and the reference is {reference}.",
        intervals.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_forest_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "forest")
    )
}

pub(crate) fn render_forest_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_forest_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 forest Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() forest specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "display_row",
        "source_row",
        "label",
        "estimate",
        "lower",
        "upper",
        "weight",
    ] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() forest data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| data.col_index(name).unwrap();
    let options = frozen_spec_options(map, render_options, "forest")?;
    let log_scale = get_opt_str(&options, "scale", "linear") == "log";
    let mut intervals = Vec::with_capacity(data.num_rows());
    for (expected_row, row) in data.rows.iter().enumerate() {
        let number = |name: &str| -> Result<f64> {
            row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() forest field '{name}' must be numeric"),
                    None,
                )
            })
        };
        if frozen_nonnegative_integer(&row[column("display_row")], "forest", "display_row")?
            != expected_row
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() forest display_row values must be contiguous and ordered",
                None,
            ));
        }
        let interval = ForestInterval {
            source_row: frozen_nonnegative_integer(
                &row[column("source_row")],
                "forest",
                "source_row",
            )?,
            label: format!("{}", row[column("label")]),
            estimate: number("estimate")?,
            lower: number("lower")?,
            upper: number("upper")?,
            weight: number("weight")?,
        };
        if !interval.estimate.is_finite()
            || !interval.lower.is_finite()
            || !interval.upper.is_finite()
            || !interval.weight.is_finite()
            || interval.weight <= 0.0
            || interval.lower > interval.estimate
            || interval.estimate > interval.upper
            || (log_scale && interval.lower <= 0.0)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() forest intervals must be finite, ordered, positive-weight, and positive on a log scale",
                None,
            ));
        }
        intervals.push(interval);
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_forest_svg(&intervals, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Forest Plot");
    finish_frozen_bio_plot(value, render_options, title, "forest", svg)
}

pub(super) fn builtin_forest_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "forest_plot")?;
    let opts = parse_options(&args);
    let intervals = forest_intervals(table, &opts)?;
    let scale = get_opt_str(&opts, "scale", "linear");
    let reference = get_opt_f64(&opts, "reference", if scale == "log" { 1.0 } else { 0.0 });
    if !reference.is_finite() || (scale == "log" && reference <= 0.0) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "forest_plot() reference must be finite and positive on a log scale",
            None,
        ));
    }
    let specification = forest_plot_spec_value(&intervals, &opts);
    render_forest_plot_spec_value(&specification, &opts)
}

// ── 11. clustered_heatmap ───────────────────────────────────────

#[derive(Clone, Debug)]
pub(super) struct RocPoint {
    threshold: Option<f64>,
    fpr: f64,
    tpr: f64,
    tp: Option<usize>,
    fp: Option<usize>,
    tn: Option<usize>,
    fn_count: Option<usize>,
}

pub(super) fn roc_geometry(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<(Vec<RocPoint>, f64, String, usize)> {
    let precomputed = table.col_index("fpr").is_some() && table.col_index("tpr").is_some();
    let (points, observations) = if precomputed {
        let fprs = extract_table_col(table, "fpr")?;
        let tprs = extract_table_col(table, "tpr")?;
        if fprs.is_empty() || fprs.len() != tprs.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() precomputed fpr and tpr columns must be non-empty and equal length",
                None,
            ));
        }
        let mut points = Vec::with_capacity(fprs.len());
        for (index, (&fpr, &tpr)) in fprs.iter().zip(&tprs).enumerate() {
            if !fpr.is_finite()
                || !tpr.is_finite()
                || !(0.0..=1.0).contains(&fpr)
                || !(0.0..=1.0).contains(&tpr)
                || points
                    .last()
                    .is_some_and(|previous: &RocPoint| fpr < previous.fpr || tpr < previous.tpr)
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "roc_curve() precomputed point {index} must be finite, within [0, 1], and monotone in fpr/tpr"
                    ),
                    None,
                ));
            }
            points.push(RocPoint {
                threshold: None,
                fpr,
                tpr,
                tp: None,
                fp: None,
                tn: None,
                fn_count: None,
            });
        }
        (points, fprs.len())
    } else {
        let scores = extract_table_col(table, get_opt_str(opts, "score", "score"))?;
        let labels = extract_table_col(table, get_opt_str(opts, "label", "label"))?;
        if scores.is_empty() || scores.len() != labels.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() score and label columns must be non-empty and equal length",
                None,
            ));
        }
        if scores.iter().any(|value| !value.is_finite())
            || labels.iter().any(|value| !value.is_finite())
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() scores and labels must be finite",
                None,
            ));
        }
        let positives = labels.iter().filter(|&&label| label >= 1.0).count();
        let negatives = labels.len() - positives;
        if positives == 0 || negatives == 0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "roc_curve() requires at least one positive and one negative observation",
                None,
            ));
        }
        let mut order = (0..scores.len()).collect::<Vec<_>>();
        order.sort_by(|&left, &right| scores[right].total_cmp(&scores[left]));
        let mut tp = 0usize;
        let mut fp = 0usize;
        let mut points = vec![RocPoint {
            threshold: None,
            fpr: 0.0,
            tpr: 0.0,
            tp: Some(0),
            fp: Some(0),
            tn: Some(negatives),
            fn_count: Some(positives),
        }];
        let mut index = 0usize;
        while index < order.len() {
            let threshold = scores[order[index]];
            while index < order.len() && scores[order[index]] == threshold {
                if labels[order[index]] >= 1.0 {
                    tp += 1;
                } else {
                    fp += 1;
                }
                index += 1;
            }
            points.push(RocPoint {
                threshold: Some(threshold),
                fpr: fp as f64 / negatives as f64,
                tpr: tp as f64 / positives as f64,
                tp: Some(tp),
                fp: Some(fp),
                tn: Some(negatives - fp),
                fn_count: Some(positives - tp),
            });
        }
        (points, scores.len())
    };
    let fprs = points.iter().map(|point| point.fpr).collect::<Vec<_>>();
    let tprs = points.iter().map(|point| point.tpr).collect::<Vec<_>>();
    let auc_override = opts.get("auc").and_then(Value::as_float);
    let auc = auc_override.unwrap_or_else(|| trapz_auc(&fprs, &tprs));
    if !auc.is_finite() || !(0.0..=1.0).contains(&auc) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "roc_curve() auc must be finite and within [0, 1]",
            None,
        ));
    }
    Ok((
        points,
        auc,
        if auc_override.is_some() {
            "option".into()
        } else {
            "trapezoidal".into()
        },
        observations,
    ))
}

pub(super) fn roc_plot_spec_value(
    points: &[RocPoint],
    auc: f64,
    auc_source: &str,
    observations: usize,
    opts: &HashMap<String, Value>,
) -> Value {
    let rows = points
        .iter()
        .enumerate()
        .map(|(index, point)| {
            vec![
                Value::Int(index as i64),
                point.threshold.map(Value::Float).unwrap_or(Value::Nil),
                Value::Float(point.fpr),
                Value::Float(point.tpr),
                point
                    .tp
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
                point
                    .fp
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
                point
                    .tn
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
                point
                    .fn_count
                    .map(|value| Value::Int(value as i64))
                    .unwrap_or(Value::Nil),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "ROC Curve");
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("roc".into())),
            ("plot".into(), Value::Str("roc_curve".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "threshold",
                        "fpr",
                        "tpr",
                        "tp",
                        "fp",
                        "tn",
                        "fn",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("title".into(), Value::Str(title.into())),
                        (
                            "subtitle".into(),
                            Value::Str(get_opt_str(opts, "subtitle", "").into()),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "False positive rate").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "True positive rate").into()),
                        ),
                        ("auc".into(), Value::Float(auc)),
                        ("auc_source".into(), Value::Str(auc_source.into())),
                        (
                            "show_auc".into(),
                            Value::Bool(
                                opts.get("show_auc")
                                    .and_then(Value::as_bool)
                                    .unwrap_or(true),
                            ),
                        ),
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 560.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 520.0)),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("roc_curve".into())),
                        (
                            "input".into(),
                            Value::Str(
                                if points.iter().all(|point| point.tp.is_none()) {
                                    "precomputed"
                                } else {
                                    "raw-scores"
                                }
                                .into(),
                            ),
                        ),
                        ("observations".into(), Value::Int(observations as i64)),
                        (
                            "tie_policy".into(),
                            Value::Str("simultaneous-at-distinct-score-threshold".into()),
                        ),
                        ("auc_method".into(), Value::Str(auc_source.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

pub(super) fn render_roc_svg(points: &[RocPoint], opts: &HashMap<String, Value>) -> Result<String> {
    if points.len() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() ROC specification needs at least two points",
            None,
        ));
    }
    let width = get_opt_f64(opts, "width", 560.0);
    let height = get_opt_f64(opts, "height", 520.0);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    canvas.margin.left = 62.0_f64.min(width * 0.22);
    canvas.margin.right = 22.0;
    canvas.margin.top = if subtitle.is_empty() { 58.0 } else { 76.0 };
    canvas.margin.bottom = if caption.is_empty() { 56.0 } else { 72.0 };
    let xs = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let ys = Scale {
        domain: (0.0, 1.0),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.add_line(
        xs.map(0.0),
        ys.map(0.0),
        xs.map(1.0),
        ys.map(1.0),
        theme.grid_colour,
        1.2,
    );
    let mut area = vec![format!("{:.2},{:.2}", xs.map(points[0].fpr), ys.map(0.0))];
    area.extend(
        points
            .iter()
            .map(|point| format!("{:.2},{:.2}", xs.map(point.fpr), ys.map(point.tpr))),
    );
    area.push(format!(
        "{:.2},{:.2}",
        xs.map(points.last().unwrap().fpr),
        ys.map(0.0)
    ));
    canvas.elements.push(format!(
        r#"<polygon points="{}" fill="{}" opacity="0.16" />"#,
        area.join(" "),
        PALETTE[0]
    ));
    let line = points
        .iter()
        .map(|point| format!("{:.2},{:.2}", xs.map(point.fpr), ys.map(point.tpr)))
        .collect::<Vec<_>>()
        .join(" ");
    canvas.elements.push(format!(
        r#"<polyline points="{line}" fill="none" stroke="{}" stroke-width="2.2" />"#,
        PALETTE[0]
    ));
    let axis = Scale {
        domain: (0.0, 1.0),
        range: (0.0, 1.0),
    };
    canvas.draw_x_axis(&axis, get_opt_str(opts, "xlabel", "False positive rate"));
    canvas.draw_y_axis(&axis, get_opt_str(opts, "ylabel", "True positive rate"));
    let auc = get_opt_f64(opts, "auc", f64::NAN);
    if !auc.is_finite() || !(0.0..=1.0).contains(&auc) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() ROC auc must be finite and within [0, 1]",
            None,
        ));
    }
    if opts
        .get("show_auc")
        .and_then(Value::as_bool)
        .unwrap_or(true)
    {
        canvas.add_text(
            canvas.margin.left + canvas.plot_width() - 6.0,
            canvas.margin.top + 18.0,
            &format!("AUC = {auc:.3}"),
            "end",
            theme.axis_title_size,
        );
    }
    canvas.draw_title(get_opt_str(opts, "title", "ROC Curve"));
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Receiver operating characteristic curve with {} frozen threshold points and trapezoidal area {auc:.4}.",
        points.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_roc_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "roc")
    )
}

pub(crate) fn render_roc_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_roc_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 ROC Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "threshold",
        "fpr",
        "tpr",
        "tp",
        "fp",
        "tn",
        "fn",
    ] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() ROC data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| data.col_index(name).unwrap();
    let optional_count = |row: &[Value], name: &str| -> Result<Option<usize>> {
        match &row[column(name)] {
            Value::Nil => Ok(None),
            value => frozen_nonnegative_integer(value, "ROC", name).map(Some),
        }
    };
    let mut points = Vec::with_capacity(data.num_rows());
    for (expected, row) in data.rows.iter().enumerate() {
        let number = |name: &str| -> Result<f64> {
            row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() ROC field '{name}' must be numeric"),
                    None,
                )
            })
        };
        if frozen_nonnegative_integer(&row[column("point_index")], "ROC", "point_index")?
            != expected
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC point_index values must be contiguous and ordered",
                None,
            ));
        }
        let threshold = match &row[column("threshold")] {
            Value::Nil => None,
            value => Some(value.as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() ROC threshold must be numeric or Nil",
                    None,
                )
            })?),
        };
        let point = RocPoint {
            threshold,
            fpr: number("fpr")?,
            tpr: number("tpr")?,
            tp: optional_count(row, "tp")?,
            fp: optional_count(row, "fp")?,
            tn: optional_count(row, "tn")?,
            fn_count: optional_count(row, "fn")?,
        };
        if !point.fpr.is_finite()
            || !point.tpr.is_finite()
            || !(0.0..=1.0).contains(&point.fpr)
            || !(0.0..=1.0).contains(&point.tpr)
            || point
                .threshold
                .is_some_and(|threshold| !threshold.is_finite())
            || points.last().is_some_and(|previous: &RocPoint| {
                point.fpr < previous.fpr || point.tpr < previous.tpr
            })
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC points must be finite, monotone, and within [0, 1]",
                None,
            ));
        }
        points.push(point);
    }
    let raw_counts = points.iter().any(|point| point.tp.is_some());
    if raw_counts {
        if points.iter().any(|point| {
            point.tp.is_none()
                || point.fp.is_none()
                || point.tn.is_none()
                || point.fn_count.is_none()
        }) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC confusion counts must be present for every raw-score point",
                None,
            ));
        }
        let positives = points[0].tp.unwrap() + points[0].fn_count.unwrap();
        let negatives = points[0].fp.unwrap() + points[0].tn.unwrap();
        if positives == 0 || negatives == 0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC confusion counts require both classes",
                None,
            ));
        }
        for point in &points {
            let tp = point.tp.unwrap();
            let fp = point.fp.unwrap();
            if tp + point.fn_count.unwrap() != positives
                || fp + point.tn.unwrap() != negatives
                || (point.tpr - tp as f64 / positives as f64).abs() > 1e-10
                || (point.fpr - fp as f64 / negatives as f64).abs() > 1e-10
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() ROC rates do not match their frozen confusion counts",
                    None,
                ));
            }
        }
    }
    let options = frozen_spec_options(map, render_options, "ROC")?;
    let auc = get_opt_f64(&options, "auc", f64::NAN);
    if get_opt_str(&options, "auc_source", "trapezoidal") == "trapezoidal" {
        let fprs = points.iter().map(|point| point.fpr).collect::<Vec<_>>();
        let tprs = points.iter().map(|point| point.tpr).collect::<Vec<_>>();
        if !auc.is_finite() || (auc - trapz_auc(&fprs, &tprs)).abs() > 1e-10 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ROC auc does not match its frozen trapezoidal curve",
                None,
            ));
        }
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_roc_svg(&points, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("ROC Curve");
    finish_frozen_bio_plot(value, render_options, title, "ROC", svg)
}

pub(super) fn builtin_roc_curve(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "roc_curve")?;
    let opts = parse_options(&args);
    let (points, auc, auc_source, observations) = roc_geometry(table, &opts)?;
    let specification = roc_plot_spec_value(&points, auc, &auc_source, observations, &opts);
    render_roc_plot_spec_value(&specification, &opts)
}
