//! Differential for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(super) fn render_volcano_svg(
    fcs: &[f64],
    pvals: &[f64],
    fc_thresh: f64,
    p_thresh: f64,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let fc_col = get_opt_str(opts, "fc", "log2fc");
    let p_col = get_opt_str(opts, "p", "pvalue");
    let neg_log_p: Vec<f64> = pvals
        .iter()
        .map(|&p| if p > 0.0 { -(p.log10()) } else { 0.0 })
        .collect();
    let (x_min, x_max) = col_range(fcs);
    let x_abs = x_min.abs().max(x_max.abs());
    let (_, y_max) = col_range(&neg_log_p);
    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale {
        domain: (-x_abs, x_abs),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (0.0, y_max),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let neg_log_p_thresh = -(p_thresh.log10());
    canvas.add_line(
        x_scale.map(-fc_thresh),
        canvas.margin.top,
        x_scale.map(-fc_thresh),
        canvas.margin.top + canvas.plot_height(),
        "#ccc",
        1.0,
    );
    canvas.add_line(
        x_scale.map(fc_thresh),
        canvas.margin.top,
        x_scale.map(fc_thresh),
        canvas.margin.top + canvas.plot_height(),
        "#ccc",
        1.0,
    );
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(neg_log_p_thresh),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(neg_log_p_thresh),
        "#ccc",
        1.0,
    );
    let renderable = (0..fcs.len().min(neg_log_p.len()))
        .filter(|&index| fcs[index].is_finite() && neg_log_p[index].is_finite())
        .collect::<Vec<_>>();
    let raster = raster_choice(opts, "volcano", renderable.len())?;
    let points: Vec<(f64, f64, &str)> = renderable
        .iter()
        .map(|&index| {
            let colour = if neg_log_p[index] > neg_log_p_thresh && fcs[index].abs() > fc_thresh {
                if fcs[index] > 0.0 {
                    "#e15759"
                } else {
                    "#4e79a7"
                }
            } else {
                "#999"
            };
            (
                x_scale.map(fcs[index]),
                y_scale.map(neg_log_p[index]),
                colour,
            )
        })
        .collect();
    let area = canvas.point_area();
    canvas.add_scatter(&points, 3.0, area, raster);
    canvas.draw_x_axis(
        &Scale {
            domain: (-x_abs, x_abs),
            range: (-x_abs, x_abs),
        },
        &axis_label(opts, "xlabel", &format!("log2(FC) [{fc_col}]")),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (0.0, y_max),
            range: (0.0, y_max),
        },
        &axis_label(opts, "ylabel", &format!("-log10(p) [{p_col}]")),
    );
    canvas.draw_title("Volcano Plot");
    canvas.set_accessible_description(format!(
        "Volcano plot with {} rendered of {} rows; fold-change threshold {fc_thresh} and p-value threshold {p_thresh}.",
        renderable.len(),
        fcs.len().min(pvals.len())
    ));
    Ok(canvas.render())
}

pub(super) fn render_ma_svg(
    a_vals: &[f64],
    m_vals: &[f64],
    m_threshold: f64,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let a_col = get_opt_str(opts, "a", "baseMean");
    let m_col = get_opt_str(opts, "m", "log2fc");
    let a_log: Vec<f64> = a_vals
        .iter()
        .map(|&value| if value > 0.0 { value.log2() } else { 0.0 })
        .collect();
    let (x_min, x_max) = col_range(&a_log);
    let (y_min, y_max) = col_range(m_vals);
    let y_abs = y_min.abs().max(y_max.abs());
    let mut canvas = SvgCanvas::new(width, height);
    let x_scale = Scale {
        domain: (x_min, x_max),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (-y_abs, y_abs),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(0.0),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(0.0),
        "#ccc",
        1.0,
    );
    let renderable = (0..a_log.len().min(m_vals.len()))
        .filter(|&index| a_log[index].is_finite() && m_vals[index].is_finite())
        .collect::<Vec<_>>();
    let raster = raster_choice(opts, "ma_plot", renderable.len())?;
    let points: Vec<(f64, f64, &str)> = renderable
        .iter()
        .map(|&index| {
            let colour = if m_vals[index].abs() > m_threshold {
                "#e15759"
            } else {
                "#999"
            };
            (
                x_scale.map(a_log[index]),
                y_scale.map(m_vals[index]),
                colour,
            )
        })
        .collect();
    let area = canvas.point_area();
    canvas.add_scatter(&points, 3.0, area, raster);
    canvas.draw_x_axis(
        &Scale {
            domain: (x_min, x_max),
            range: (x_min, x_max),
        },
        &axis_label(opts, "xlabel", &format!("A (log2 {a_col})")),
    );
    canvas.draw_y_axis(
        &Scale {
            domain: (-y_abs, y_abs),
            range: (-y_abs, y_abs),
        },
        &axis_label(opts, "ylabel", &format!("M ({m_col})")),
    );
    canvas.draw_title("MA Plot");
    canvas.set_accessible_description(format!(
        "MA plot with {} rendered of {} rows; absolute log2 fold-change threshold {m_threshold}.",
        renderable.len(),
        a_vals.len().min(m_vals.len())
    ));
    Ok(canvas.render())
}

pub(super) fn differential_plot_spec_value(
    plot_kind: &str,
    raw_x: &[f64],
    raw_y: &[f64],
    labels: &[String],
    x_column: &str,
    y_column: &str,
    fc_threshold: f64,
    p_threshold: Option<f64>,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let point_count = raw_x.len().min(raw_y.len());
    let transformed_y = if plot_kind == "volcano" {
        raw_y
            .iter()
            .map(|&value| if value > 0.0 { -(value.log10()) } else { 0.0 })
            .collect::<Vec<_>>()
    } else {
        raw_y.to_vec()
    };
    let transformed_x = if plot_kind == "ma" {
        raw_x
            .iter()
            .map(|&value| if value > 0.0 { value.log2() } else { 0.0 })
            .collect::<Vec<_>>()
    } else {
        raw_x.to_vec()
    };
    let rendered_points = (0..point_count)
        .filter(|&index| transformed_x[index].is_finite() && transformed_y[index].is_finite())
        .count();
    let raster = raster_choice(
        opts,
        if plot_kind == "volcano" {
            "volcano"
        } else {
            "ma_plot"
        },
        rendered_points,
    )?;
    let neg_log_p_threshold = p_threshold.map(|value| -(value.log10()));
    let rows = (0..point_count)
        .map(|index| {
            let status = if !transformed_x[index].is_finite() || !transformed_y[index].is_finite() {
                "not_rendered"
            } else if plot_kind == "volcano" {
                if transformed_y[index] > neg_log_p_threshold.unwrap_or(f64::INFINITY)
                    && raw_x[index].abs() > fc_threshold
                {
                    if raw_x[index] > 0.0 {
                        "up"
                    } else {
                        "down"
                    }
                } else {
                    "not_significant"
                }
            } else if raw_y[index].abs() > fc_threshold {
                "changed"
            } else {
                "not_changed"
            };
            vec![
                Value::Int(index as i64),
                Value::Float(raw_x[index]),
                Value::Float(raw_y[index]),
                Value::Float(transformed_x[index]),
                Value::Float(transformed_y[index]),
                labels
                    .get(index)
                    .filter(|value| !value.is_empty())
                    .map(|value| Value::Str(value.clone()))
                    .unwrap_or(Value::Nil),
                Value::Str(status.into()),
            ]
        })
        .collect::<Vec<_>>();
    let title = if plot_kind == "volcano" {
        "Volcano Plot"
    } else {
        "MA Plot"
    };
    let mut spec_options = HashMap::from([
        ("plot".into(), Value::Str(plot_kind.into())),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 800.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 600.0)),
        ),
        ("x_column".into(), Value::Str(x_column.into())),
        ("y_column".into(), Value::Str(y_column.into())),
        ("fold_change_threshold".into(), Value::Float(fc_threshold)),
        ("raster".into(), Value::Bool(raster.enabled)),
        ("raster_scale".into(), Value::Float(raster.scale)),
    ]);
    if let Some(value) = p_threshold {
        spec_options.insert("p_value_threshold".into(), Value::Float(value));
    }
    let default_x_label = if plot_kind == "volcano" {
        format!("log2(FC) [{x_column}]")
    } else {
        format!("A (log2 {x_column})")
    };
    let default_y_label = if plot_kind == "volcano" {
        format!("-log10(p) [{y_column}]")
    } else {
        format!("M ({y_column})")
    };
    spec_options.insert(
        "xlabel".into(),
        Value::Str(axis_label(opts, "xlabel", &default_x_label)),
    );
    spec_options.insert(
        "ylabel".into(),
        Value::Str(axis_label(opts, "ylabel", &default_y_label)),
    );
    spec_options.insert(
        if plot_kind == "volcano" { "fc" } else { "a" }.into(),
        Value::Str(x_column.into()),
    );
    spec_options.insert(
        if plot_kind == "volcano" { "p" } else { "m" }.into(),
        Value::Str(y_column.into()),
    );
    let non_finite_coordinates = point_count - rendered_points;
    let warnings = if non_finite_coordinates == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{non_finite_coordinates} rows have non-finite plot coordinates"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str("differential_expression".into())),
            ("plot".into(), Value::Str(plot_kind.into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    ["source_row", "raw_x", "raw_y", "x", "y", "label", "status"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    rows,
                )),
            ),
            ("options".into(), Value::Record(spec_options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "builtin".into(),
                            Value::Str(if plot_kind == "volcano" {
                                "volcano".into()
                            } else {
                                "ma_plot".into()
                            }),
                        ),
                        ("input_rows".into(), Value::Int(point_count as i64)),
                        ("rendered_points".into(), Value::Int(rendered_points as i64)),
                        (
                            "non_finite_coordinates".into(),
                            Value::Int(non_finite_coordinates as i64),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(warnings.into())),
        ])
        .into(),
    ))
}

pub(super) fn is_differential_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "differential_expression")
    )
}

pub(super) fn render_differential_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_differential_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 differential-expression Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() differential-expression field 'data' must be Table",
                None,
            ))
        }
    };
    for required in ["source_row", "raw_x", "raw_y", "x", "y", "label", "status"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() differential-expression data is missing '{required}'"),
                None,
            ));
        }
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() differential-expression field 'options' must be Record",
                None,
            ))
        }
    };
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in [
        "raster",
        "raster_threshold",
        "raster_scale",
        "width",
        "height",
    ] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    let raw_x = extract_table_col(table, "raw_x")?;
    let raw_y = extract_table_col(table, "raw_y")?;
    let plot_kind = map.get("plot").and_then(Value::as_str).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() differential-expression specification is missing 'plot'",
            None,
        )
    })?;
    let threshold = options
        .get("fold_change_threshold")
        .and_then(Value::as_float)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() differential-expression options are missing numeric 'fold_change_threshold'",
                None,
            )
        })?;
    let svg = match plot_kind {
        "volcano" => {
            let p_threshold = options
                .get("p_value_threshold")
                .and_then(Value::as_float)
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "render_plot() volcano options are missing numeric 'p_value_threshold'",
                        None,
                    )
                })?;
            render_volcano_svg(&raw_x, &raw_y, threshold, p_threshold, &options)?
        }
        "ma" => render_ma_svg(&raw_x, &raw_y, threshold, &options)?,
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() unknown differential-expression plot '{other}'"),
                None,
            ))
        }
    };
    let title = map.get("title").and_then(Value::as_str).unwrap_or("Plot");
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Ascii)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Braille)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal differential-expression output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown differential-expression format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

pub(super) fn extract_optional_plot_labels(table: &Table) -> Vec<String> {
    ["gene", "name", "id"]
        .iter()
        .find_map(|column| {
            let index = table.col_index(column)?;
            Some(
                table
                    .rows
                    .iter()
                    .map(|row| match &row[index] {
                        Value::Str(value) => value.clone(),
                        Value::Nil => String::new(),
                        other => format!("{other}"),
                    })
                    .collect(),
            )
        })
        .unwrap_or_else(|| vec![String::new(); table.num_rows()])
}

pub(super) fn builtin_volcano(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "volcano")?;
    let opts = parse_options(&args);
    let fc_col = get_opt_str(&opts, "fc", "log2fc").to_string();
    let p_col = get_opt_str(&opts, "p", "pvalue").to_string();
    let fc_thresh = get_opt_f64(&opts, "fc_threshold", 1.0);
    let p_thresh = get_opt_f64(&opts, "p_threshold", 0.05);
    let fcs = extract_table_col(table, &fc_col)?;
    let pvals = extract_table_col(table, &p_col)?;
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let labels = extract_optional_plot_labels(table);
        let spec = differential_plot_spec_value(
            "volcano",
            &fcs,
            &pvals,
            &labels,
            &fc_col,
            &p_col,
            fc_thresh,
            Some(p_thresh),
            &opts,
        )?;
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_differential_plot_spec_value(&spec, &opts);
    }
    render_volcano_svg(&fcs, &pvals, fc_thresh, p_thresh, &opts).map(Value::Str)
}

pub(super) fn builtin_ma_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table(&args[0], "ma_plot")?;
    let opts = parse_options(&args);
    let a_col = get_opt_str(&opts, "a", "baseMean").to_string();
    let m_col = get_opt_str(&opts, "m", "log2fc").to_string();

    let a_vals = extract_table_col(table, &a_col)?;
    let m_vals = extract_table_col(table, &m_col)?;

    // Preserve the legacy MA classification boundary exactly.
    const M_THRESHOLD: f64 = 1.0;
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let labels = extract_optional_plot_labels(table);
        let spec = differential_plot_spec_value(
            "ma",
            &a_vals,
            &m_vals,
            &labels,
            &a_col,
            &m_col,
            M_THRESHOLD,
            None,
            &opts,
        )?;
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_differential_plot_spec_value(&spec, &opts);
    }
    render_ma_svg(&a_vals, &m_vals, M_THRESHOLD, &opts).map(Value::Str)
}
