//! Distribution for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct ViolinShape {
    name: String,
    sample_count: usize,
    bandwidth: f64,
    median: f64,
    input_min: f64,
    input_max: f64,
    points: Vec<(f64, f64)>,
}

pub(super) fn violin_shape(name: String, values: &[f64], steps: usize) -> ViolinShape {
    let bandwidth = silverman_bandwidth(values);
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let (input_min, input_max) = col_range(values);
    ViolinShape {
        name,
        sample_count: values.len(),
        bandwidth,
        median: quantile_type7(&sorted, 0.5),
        input_min,
        input_max,
        points: gaussian_kde(values, bandwidth, steps),
    }
}

fn trimmed_violin_shape(name: String, values: &[f64], steps: usize) -> ViolinShape {
    let mut shape = violin_shape(name, values, steps);
    shape.points = gaussian_kde_between(
        values,
        shape.bandwidth,
        steps,
        shape.input_min,
        shape.input_max,
    );
    shape
}

pub(super) fn render_legacy_violin_svg(
    shapes: &[ViolinShape],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let w = get_opt_f64(opts, "width", 600.0);
    let h = get_opt_f64(opts, "height", 400.0);
    let mut c = themed_canvas(w, h, opts);
    c.margin.bottom = 60.0;
    let global_min = shapes
        .iter()
        .map(|shape| shape.input_min)
        .fold(f64::INFINITY, f64::min);
    let global_max = shapes
        .iter()
        .map(|shape| shape.input_max)
        .fold(f64::NEG_INFINITY, f64::max);
    let ys = Scale {
        domain: (global_min, global_max),
        range: (c.margin.top + c.plot_height(), c.margin.top),
    };
    let group_w = c.plot_width() / shapes.len() as f64;
    for (gi, shape) in shapes.iter().enumerate() {
        let max_d = shape
            .points
            .iter()
            .map(|point| point.1)
            .fold(0.0_f64, f64::max);
        let cx = c.margin.left + (gi as f64 + 0.5) * group_w;
        let half_w = group_w * 0.4;
        let mut points_l = String::new();
        let mut points_r = String::new();
        for &(value, density) in &shape.points {
            let y = ys.map(value);
            let dx = if max_d > 0.0 {
                density / max_d * half_w
            } else {
                0.0
            };
            points_l.push_str(&format!("{:.1},{:.1} ", cx - dx, y));
            points_r.push_str(&format!("{:.1},{:.1} ", cx + dx, y));
        }
        let all_points = format!(
            "{points_l}{}",
            points_r
                .split_whitespace()
                .rev()
                .collect::<Vec<_>>()
                .join(" ")
        );
        c.elements.push(format!(
            r#"<polygon points="{all_points}" fill="{}" opacity="0.6" />"#,
            PALETTE[gi % PALETTE.len()]
        ));
        c.add_text(
            cx,
            c.margin.top + c.plot_height() + 18.0,
            &shape.name,
            "middle",
            10.0,
        );
    }
    c.draw_y_axis(
        &Scale {
            domain: (global_min, global_max),
            range: (global_min, global_max),
        },
        get_opt_str(opts, "ylab", "Value"),
    );
    finish_themed_canvas(&mut c, opts, "Violin Plot");
    c.set_accessible_description(format!(
        "Violin plot with {} groups; each shape is a frozen Gaussian kernel-density grid.",
        shapes.len()
    ));
    Ok(c.render())
}

pub(super) fn render_long_violin_svg(
    shapes: &[ViolinShape],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let theme = plot_theme(opts);
    let seurat_theme = get_opt_str(opts, "theme", "") == "seurat";
    let ggplot_like = matches!(theme.kind, PlotThemeKind::Ggplot | PlotThemeKind::Classic);
    let legend_enabled = match opts.get("legend") {
        None => (ggplot_like || seurat_theme) && shapes.len() > 1,
        Some(Value::Bool(value)) => *value && shapes.len() > 1,
        Some(other) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "violin_plot() option 'legend' must be Bool, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    let legend_title = get_opt_str(opts, "legend_title", "group").to_string();
    let value_col = get_opt_str(opts, "value_label", "value").to_string();
    let x_label = get_opt_str(opts, "xlab", "").to_string();
    let title = get_opt_str(opts, "title", "Distribution").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let width = get_opt_f64(opts, "width", 640.0);
    let height = get_opt_f64(opts, "height", 420.0);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let lo = shapes
        .iter()
        .filter_map(|shape| shape.points.first().map(|point| point.0))
        .fold(f64::INFINITY, f64::min);
    let hi = shapes
        .iter()
        .filter_map(|shape| shape.points.last().map(|point| point.0))
        .fold(f64::NEG_INFINITY, f64::max);

    if theme.is_adaptive() {
        let provisional = Scale {
            domain: (lo, hi),
            range: (1.0, 0.0),
        };
        let widest_tick = provisional
            .nice_ticks(5)
            .iter()
            .map(|tick| estimate_text_width(&format!("{tick:.2}"), theme.tick_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_tick + 42.0).clamp(54.0, width * 0.30);
        canvas.margin.right = 18.0_f64.min(width * 0.08);
        canvas.margin.top = if subtitle.is_empty() { 48.0 } else { 66.0 };
        let provisional_slot =
            (width - canvas.margin.left - canvas.margin.right).max(1.0) / shapes.len() as f64;
        let widest_group = shapes
            .iter()
            .map(|shape| estimate_text_width(&shape.name, theme.tick_size))
            .fold(0.0, f64::max);
        let rotate_labels = widest_group > provisional_slot * 0.86;
        let label_reserve = if rotate_labels {
            (widest_group * 0.72 + 12.0).clamp(32.0, height * 0.28)
        } else {
            28.0
        };
        canvas.margin.bottom = label_reserve
            + if x_label.is_empty() { 12.0 } else { 30.0 }
            + if caption.is_empty() { 0.0 } else { 16.0 };
    }
    if legend_enabled {
        let widest = shapes
            .iter()
            .map(|shape| estimate_text_width(&shape.name, theme.legend_size))
            .fold(
                estimate_text_width(&legend_title, theme.legend_size),
                f64::max,
            );
        canvas.margin.right += (55.0 + widest).clamp(105.0, 220.0);
    }

    let y_scale = Scale {
        domain: (lo, hi),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let slot = canvas.plot_width() / shapes.len() as f64;
    if theme.is_adaptive() {
        let left = canvas.margin.left;
        let right = left + canvas.plot_width();
        let top = canvas.margin.top;
        let bottom = top + canvas.plot_height();
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for tick in y_scale.nice_ticks(5) {
            let y = y_scale.map(tick);
            canvas.add_line(left, y, right, y, theme.grid_colour, theme.grid_width);
        }
        canvas.add_line(
            left,
            bottom,
            right,
            bottom,
            theme.axis_colour,
            theme.axis_width,
        );
    }
    let widest_group = shapes
        .iter()
        .map(|shape| estimate_text_width(&shape.name, theme.tick_size))
        .fold(0.0, f64::max);
    let rotate_labels = theme.is_adaptive() && widest_group > slot * 0.86;
    let shared_peak = shapes
        .iter()
        .flat_map(|shape| shape.points.iter().map(|point| point.1))
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    let ggplot_colours = ggplot_like.then(|| hue_palette(shapes.len()));

    for (gi, shape) in shapes.iter().enumerate() {
        let centre = canvas.margin.left + slot * (gi as f64 + 0.5);
        let peak = if ggplot_like {
            shared_peak
        } else {
            shape
                .points
                .iter()
                .map(|point| point.1)
                .fold(f64::MIN, f64::max)
                .max(1e-9)
        };
        let half = slot * 0.42;
        let mut outline = Vec::with_capacity(shape.points.len() * 2);
        for &(value, density) in &shape.points {
            outline.push(format!(
                "{:.1},{:.1}",
                centre + density / peak * half,
                y_scale.map(value)
            ));
        }
        for &(value, density) in shape.points.iter().rev() {
            outline.push(format!(
                "{:.1},{:.1}",
                centre - density / peak * half,
                y_scale.map(value)
            ));
        }
        let colour = if seurat_theme {
            SEURAT_PALETTE[gi % SEURAT_PALETTE.len()]
        } else if let Some(colours) = &ggplot_colours {
            colours[gi].as_str()
        } else {
            PALETTE[gi % PALETTE.len()]
        };
        canvas.elements.push(format!(
            r#"<polygon points="{}" fill="{}" opacity="0.65" stroke="{}" stroke-width="1" />"#,
            outline.join(" "),
            colour,
            if seurat_theme { "#333333" } else { colour }
        ));
        let my = y_scale.map(shape.median);
        canvas.add_line(
            centre - half * 0.5,
            my,
            centre + half * 0.5,
            my,
            "#333333",
            2.0,
        );
        let label_y = canvas.margin.top + canvas.plot_height() + 16.0;
        if rotate_labels {
            canvas.add_text_rotated(
                centre,
                label_y - 3.0,
                &shape.name,
                40.0,
                "start",
                theme.tick_size,
            );
        } else {
            canvas.add_text(
                centre,
                label_y,
                &shape.name,
                "middle",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }
    canvas.draw_y_axis(&y_scale, &value_col);
    if !x_label.is_empty() {
        canvas.add_text(
            canvas.margin.left + canvas.plot_width() / 2.0,
            height - 8.0,
            &x_label,
            "middle",
            theme.axis_title_size,
        );
    }
    if legend_enabled {
        let colours = ggplot_colours.clone().unwrap_or_else(|| {
            (0..shapes.len())
                .map(|index| {
                    if seurat_theme {
                        SEURAT_PALETTE[index % SEURAT_PALETTE.len()].to_string()
                    } else {
                        PALETTE[index % PALETTE.len()].to_string()
                    }
                })
                .collect()
        });
        let legend_x = width - canvas.margin.right + 22.0;
        canvas.add_text(
            legend_x,
            canvas.margin.top + 5.0,
            &legend_title,
            "start",
            theme.legend_size,
        );
        for (index, shape) in shapes.iter().enumerate() {
            let y = canvas.margin.top + 27.0 + 24.0 * index as f64;
            canvas.add_stroked_rect(
                legend_x,
                y - 10.0,
                13.0,
                13.0,
                &colours[index],
                "#333333",
                0.8,
            );
            canvas.add_text(
                legend_x + 21.0,
                y + 1.0,
                &shape.name,
                "start",
                theme.legend_size,
            );
        }
    }
    canvas.set_accessible_description(format!(
        "Violin plot of {value_col} for {} groups; each shape is a Gaussian kernel density and each horizontal mark is the group median.",
        shapes.len()
    ));
    if theme.is_adaptive() {
        canvas.draw_title(&title);
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    } else {
        canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    }
    Ok(canvas.render())
}

pub(super) fn violin_plot_spec_value(
    shapes: &[ViolinShape],
    variant: &str,
    value_label: &str,
    opts: &HashMap<String, Value>,
) -> Value {
    let data_rows = shapes
        .iter()
        .enumerate()
        .flat_map(|(group_index, shape)| {
            shape
                .points
                .iter()
                .enumerate()
                .map(move |(grid_index, &(value, density))| {
                    vec![
                        Value::Int(group_index as i64),
                        Value::Str(shape.name.clone()),
                        Value::Int(grid_index as i64),
                        Value::Float(value),
                        Value::Float(density),
                    ]
                })
        })
        .collect();
    let group_rows = shapes
        .iter()
        .enumerate()
        .map(|(index, shape)| {
            vec![
                Value::Int(index as i64),
                Value::Str(shape.name.clone()),
                Value::Int(shape.sample_count as i64),
                Value::Float(shape.bandwidth),
                Value::Float(shape.median),
                Value::Float(shape.input_min),
                Value::Float(shape.input_max),
            ]
        })
        .collect();
    let (default_title, default_width, default_height) = if variant == "wide" {
        ("Violin Plot", 600.0, 400.0)
    } else {
        ("Distribution", 640.0, 420.0)
    };
    let theme = plot_theme(opts);
    let legend_default = variant == "long"
        && shapes.len() > 1
        && (matches!(theme.kind, PlotThemeKind::Ggplot | PlotThemeKind::Classic)
            || get_opt_str(opts, "theme", "") == "seurat");
    let options = HashMap::from([
        ("variant".into(), Value::Str(variant.into())),
        ("value_label".into(), Value::Str(value_label.into())),
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", default_title).into()),
        ),
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
            "ylab".into(),
            Value::Str(get_opt_str(opts, "ylab", "Value").into()),
        ),
        (
            "xlab".into(),
            Value::Str(get_opt_str(opts, "xlab", "").into()),
        ),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", default_width)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", default_height)),
        ),
        (
            "legend".into(),
            Value::Bool(
                opts.get("legend")
                    .and_then(Value::as_bool)
                    .unwrap_or(legend_default),
            ),
        ),
        (
            "legend_title".into(),
            Value::Str(get_opt_str(opts, "legend_title", "group").into()),
        ),
    ]);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("violin".into())),
            ("plot".into(), Value::Str(variant.into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", default_title).into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    ["group_index", "group", "grid_index", "value", "density"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    data_rows,
                )),
            ),
            (
                "groups".into(),
                Value::Table(Table::new(
                    [
                        "group_index",
                        "group",
                        "sample_count",
                        "bandwidth",
                        "median",
                        "input_min",
                        "input_max",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    group_rows,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "builtin".into(),
                            Value::Str(if variant == "wide" {
                                "violin".into()
                            } else {
                                "violin_plot".into()
                            }),
                        ),
                        ("groups".into(), Value::Int(shapes.len() as i64)),
                        (
                            "samples".into(),
                            Value::Int(
                                shapes.iter().map(|shape| shape.sample_count).sum::<usize>() as i64,
                            ),
                        ),
                        (
                            "grid_points".into(),
                            Value::Int(
                                shapes.iter().map(|shape| shape.points.len()).sum::<usize>() as i64,
                            ),
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

pub(crate) fn is_violin_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "violin")
    )
}

pub(crate) fn render_violin_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_violin_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 violin Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin specification field 'data' must be Table",
                None,
            ))
        }
    };
    let summaries = match map.get("groups") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin specification field 'groups' must be Table",
                None,
            ))
        }
    };
    for required in ["group_index", "group", "grid_index", "value", "density"] {
        if data.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() violin data is missing '{required}'"),
                None,
            ));
        }
    }
    for required in [
        "group_index",
        "group",
        "sample_count",
        "bandwidth",
        "median",
        "input_min",
        "input_max",
    ] {
        if summaries.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() violin groups are missing '{required}'"),
                None,
            ));
        }
    }
    if summaries.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() violin specification has no groups",
            None,
        ));
    }
    let mut shapes = Vec::with_capacity(summaries.num_rows());
    let summary_index = |name: &str| summaries.col_index(name).unwrap();
    for (expected, row) in summaries.rows.iter().enumerate() {
        let index = row[summary_index("group_index")]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() violin group_index must be numeric",
                    None,
                )
            })?;
        if index != expected {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin group_index values must be contiguous and ordered",
                None,
            ));
        }
        let number = |name: &str| -> Result<f64> {
            row[summary_index(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() violin group field '{name}' must be numeric"),
                    None,
                )
            })
        };
        shapes.push(ViolinShape {
            name: format!("{}", row[summary_index("group")]),
            sample_count: number("sample_count")? as usize,
            bandwidth: number("bandwidth")?,
            median: number("median")?,
            input_min: number("input_min")?,
            input_max: number("input_max")?,
            points: Vec::new(),
        });
    }
    let group_index = data.col_index("group_index").unwrap();
    let grid_index = data.col_index("grid_index").unwrap();
    let value_index = data.col_index("value").unwrap();
    let density_index = data.col_index("density").unwrap();
    for row in &data.rows {
        let group = row[group_index]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() violin data group_index must be numeric",
                    None,
                )
            })?;
        let shape = shapes.get_mut(group).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin data references an unknown group_index",
                None,
            )
        })?;
        let expected_grid = shape.points.len();
        let actual_grid = row[grid_index]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() violin grid_index must be numeric",
                    None,
                )
            })?;
        if expected_grid != actual_grid {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin grid_index values must be contiguous within each group",
                None,
            ));
        }
        let grid_value = row[value_index].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin grid values must be numeric",
                None,
            )
        })?;
        let density = row[density_index].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin densities must be numeric",
                None,
            )
        })?;
        if !grid_value.is_finite() || !density.is_finite() || density < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin grid values must be finite and densities non-negative",
                None,
            ));
        }
        shape.points.push((grid_value, density));
    }
    if shapes.iter().any(|shape| shape.points.len() < 2) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() violin specification needs at least two grid points per group",
            None,
        ));
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() violin specification field 'options' must be Record",
                None,
            ))
        }
    };
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in ["width", "height"] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    let variant = get_opt_str(&options, "variant", "long");
    let svg = match variant {
        "wide" => render_legacy_violin_svg(&shapes, &options)?,
        "long" => render_long_violin_svg(&shapes, &options)?,
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() unknown violin variant '{other}'"),
                None,
            ))
        }
    };
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Violin Plot");
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(crate::plot::standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => crate::plot::render_svg_terminal(
            &svg,
            80,
            24,
            crate::plot::TerminalPlotStyle::Ascii,
        )
        .map(Value::Str)
        .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => crate::plot::render_svg_terminal(
            &svg,
            80,
            24,
            crate::plot::TerminalPlotStyle::Braille,
        )
        .map(Value::Str)
        .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal violin output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown violin format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

pub(super) fn builtin_violin(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    // Collect named groups of numeric data
    let groups: Vec<(String, Vec<f64>)> = match &args[0] {
        Value::List(_) => {
            let vals = nums_from_value(&args[0], "violin")?;
            vec![("data".to_string(), vals)]
        }
        Value::Table(table) => table
            .columns
            .iter()
            .map(|col| {
                let vals = extract_table_col(table, col).unwrap_or_default();
                let finite: Vec<f64> = vals.into_iter().filter(|v| v.is_finite()).collect();
                (col.clone(), finite)
            })
            .filter(|(_, v)| !v.is_empty())
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "violin() requires List or Table",
                None,
            ))
        }
    };
    if groups.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin() no data",
            None,
        ));
    }

    let shapes = groups
        .iter()
        .map(|(name, values)| violin_shape(name.clone(), values, 50))
        .collect::<Vec<_>>();

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec =
            violin_plot_spec_value(&shapes, "wide", get_opt_str(&opts, "ylab", "Value"), &opts);
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_violin_plot_spec_value(&spec, &opts);
    }

    if fmt == "svg" {
        return render_legacy_violin_svg(&shapes, &opts).map(Value::Str);
    }

    // ASCII: horizontal violin per group
    let bar_w = get_opt_usize(&opts, "width", 40);
    let mut out = String::from("  Violin Plot\n");
    let max_label = groups.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    for (name, vals) in &groups {
        let bw = silverman_bw(vals);
        let (_, kde_d) = kde(vals, bw, bar_w);
        let max_d = kde_d.iter().cloned().fold(0.0f64, f64::max);
        let bars: String = kde_d
            .iter()
            .map(|&d| {
                let t = if max_d > 0.0 { d / max_d } else { 0.0 };
                let idx = (t * 7.0).round() as usize;
                ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][idx.min(7)]
            })
            .collect();
        out.push_str(&format!("  {:>w$}  {bars}\n", name, w = max_label));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 7. density ──────────────────────────────────────────────────

pub(super) fn builtin_density(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let bw_opt = opts.get("bandwidth").and_then(|v| v.as_float());

    let groups: Vec<(String, Vec<f64>)> = match &args[0] {
        Value::List(_) => vec![("data".to_string(), nums_from_value(&args[0], "density")?)],
        Value::Table(table) => table
            .columns
            .iter()
            .filter_map(|col| {
                let vals: Vec<f64> = extract_table_col(table, col)
                    .ok()?
                    .into_iter()
                    .filter(|v| v.is_finite())
                    .collect();
                if vals.is_empty() {
                    None
                } else {
                    Some((col.clone(), vals))
                }
            })
            .collect(),
        _ => {
            return Err(BioLangError::type_error(
                "density() requires List or Table",
                None,
            ))
        }
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 400.0);
        let mut c = themed_canvas(w, h, &opts);
        let mut global_xs: Vec<f64> = Vec::new();
        let mut global_ys: Vec<f64> = Vec::new();
        let mut curves: Vec<(Vec<f64>, Vec<f64>)> = Vec::new();
        for (_, vals) in &groups {
            let bw = bw_opt.unwrap_or_else(|| silverman_bw(vals));
            let (kx, ky) = kde(vals, bw, 100);
            global_xs.extend(&kx);
            global_ys.extend(&ky);
            curves.push((kx, ky));
        }
        let xr = col_range(&global_xs);
        let yr = (0.0, col_range(&global_ys).1 * 1.1);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let ys = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        for (gi, (kx, ky)) in curves.iter().enumerate() {
            let baseline = ys.map(0.0);
            let mut points = String::new();
            points.push_str(&format!("{:.1},{:.1} ", xs.map(kx[0]), baseline));
            for i in 0..kx.len() {
                points.push_str(&format!("{:.1},{:.1} ", xs.map(kx[i]), ys.map(ky[i])));
            }
            points.push_str(&format!(
                "{:.1},{:.1}",
                xs.map(*kx.last().unwrap()),
                baseline
            ));
            c.elements.push(format!(
                r#"<polygon points="{points}" fill="{}" opacity="0.4" />"#,
                PALETTE[gi % PALETTE.len()]
            ));
        }
        let dx = Scale {
            domain: xr,
            range: xr,
        };
        let dy = Scale {
            domain: yr,
            range: yr,
        };
        c.draw_x_axis(&dx, get_opt_str(&opts, "xlab", "Value"));
        c.draw_y_axis(&dy, get_opt_str(&opts, "ylab", "Density"));
        finish_themed_canvas(&mut c, &opts, "Density");
        return Ok(Value::Str(c.render()));
    }

    // ASCII: histogram-style density
    let width = get_opt_usize(&opts, "width", 60);
    let _height = get_opt_usize(&opts, "height", 16);
    let mut out = String::from("  Density\n");
    for (name, vals) in groups.iter() {
        let bw = bw_opt.unwrap_or_else(|| silverman_bw(vals));
        let (_kx, ky) = kde(vals, bw, width);
        let max_y = ky.iter().cloned().fold(0.0f64, f64::max);
        let bars: String = ky
            .iter()
            .map(|&y| {
                let t = if max_y > 0.0 { y / max_y } else { 0.0 };
                let idx = (t * 7.0).round() as usize;
                ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'][idx.min(7)]
            })
            .collect();
        if groups.len() > 1 {
            out.push_str(&format!("  {name}:\n"));
        }
        out.push_str(&format!("  {bars}\n"));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 8. kaplan_meier ─────────────────────────────────────────────

/// Scatter plot for UMAP/PCA/t-SNE embeddings.
/// data: Table with columns x, y, and optionally color/label/cluster
/// options: Record{title?, width?, height?, color_col?, label_col?, format?}
/// Scree / elbow plot: variance explained by each principal component.
///
/// The figure you read to choose how many components to keep - it flattens
/// where the components stop carrying structure. Accepts either the list of
/// ratios or the whole record `sc_pca` returns, because passing that record
/// straight through is what a reader will try first.
pub(super) fn builtin_elbow_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);

    let values: Vec<f64> = match &args[0] {
        Value::List(items) => items.iter().filter_map(|v| v.as_float()).collect(),
        Value::Record(map) => map
            .get("explained_variance_ratio")
            .or_else(|| map.get("explained_variance"))
            .map(|v| match v {
                Value::List(items) => items.iter().filter_map(|x| x.as_float()).collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default(),
        _ => return Err(BioLangError::type_error(
            "elbow_plot() requires a List of variance ratios, or the Record returned by sc_pca()",
            None,
        )),
    };

    if values.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "elbow_plot() found no variance values",
            None,
        ));
    }

    let title = get_opt_str(&opts, "title", "Scree plot").to_string();
    let width = get_opt_f64(&opts, "width", 600.0);
    let height = get_opt_f64(&opts, "height", 400.0);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(&opts));

    let option_numbers = |key: &str| -> Result<Option<Vec<f64>>> {
        let Some(value) = opts.get(key) else {
            return Ok(None);
        };
        let Value::List(items) = value else {
            return Err(BioLangError::type_error(
                format!("elbow_plot() option '{key}' must be List"),
                None,
            ));
        };
        items
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value.as_float().filter(|value| value.is_finite()).ok_or_else(|| {
                    BioLangError::type_error(
                        format!(
                            "elbow_plot() option '{key}' must contain finite numbers; index {index} is {}",
                            value.type_of()
                        ),
                        None,
                    )
                })
            })
            .collect::<Result<Vec<_>>>()
            .map(Some)
    };
    let option_labels = |key: &str| -> Result<Option<Vec<String>>> {
        let Some(value) = opts.get(key) else {
            return Ok(None);
        };
        let Value::List(items) = value else {
            return Err(BioLangError::type_error(
                format!("elbow_plot() option '{key}' must be List"),
                None,
            ));
        };
        items
            .iter()
            .enumerate()
            .map(|(index, value)| {
                value.as_str().map(str::to_string).ok_or_else(|| {
                    BioLangError::type_error(
                        format!(
                            "elbow_plot() option '{key}' must contain strings; index {index} is {}",
                            value.type_of()
                        ),
                        None,
                    )
                })
            })
            .collect::<Result<Vec<_>>>()
            .map(Some)
    };
    let x_breaks = option_numbers("x_breaks")?;
    let x_labels = option_labels("x_labels")?;
    if let (Some(breaks), Some(labels)) = (&x_breaks, &x_labels) {
        if breaks.len() != labels.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "elbow_plot() x_breaks and x_labels must have equal length",
                None,
            ));
        }
    }
    let y_breaks = option_numbers("y_breaks")?;

    let highest = values.iter().cloned().fold(f64::MIN, f64::max);
    let x_scale = Scale {
        domain: (0.5, values.len() as f64 + 0.5),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        // Always anchored at zero: a scree plot read on a truncated axis
        // exaggerates the elbow, which is the one thing it exists to show.
        domain: (0.0, highest * 1.1 + 1e-9),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    let points: Vec<String> = values
        .iter()
        .enumerate()
        .map(|(i, v)| format!("{:.1},{:.1}", x_scale.map(i as f64 + 1.0), y_scale.map(*v)))
        .collect();
    let colour = get_opt_str(&opts, "color", PALETTE[0]);
    let line_width = get_opt_f64(&opts, "line_width", 2.0).max(0.0);
    canvas.elements.push(format!(
        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="{:.2}" />"#,
        points.join(" "),
        colour,
        line_width
    ));
    let open_points = get_opt_str(&opts, "point_style", "filled") == "open";
    let point_radius = get_opt_f64(&opts, "point_radius", 4.0).max(0.0);
    for (i, v) in values.iter().enumerate() {
        let x = x_scale.map(i as f64 + 1.0);
        let y = y_scale.map(*v);
        if open_points {
            canvas.add_stroked_circle(x, y, point_radius, "#FFFFFF", colour, 1.0);
        } else {
            canvas.add_circle(x, y, point_radius, colour);
        }
    }

    let x_label = get_opt_str(&opts, "x_label", "component");
    let y_label = get_opt_str(&opts, "y_label", "variance explained");
    if let Some(breaks) = &x_breaks {
        let ticks = breaks
            .iter()
            .enumerate()
            .map(|(index, value)| {
                (
                    *value,
                    x_labels
                        .as_ref()
                        .and_then(|labels| labels.get(index).cloned())
                        .unwrap_or_else(|| format!("{value}")),
                )
            })
            .collect::<Vec<_>>();
        canvas.draw_x_axis_with_ticks(&x_scale, &ticks, x_label);
    } else {
        canvas.draw_x_axis(&x_scale, x_label);
    }
    if let Some(breaks) = &y_breaks {
        let ticks = breaks
            .iter()
            .map(|value| (*value, format!("{value}")))
            .collect::<Vec<_>>();
        canvas.draw_y_axis_with_ticks(&y_scale, &ticks, y_label);
    } else {
        canvas.draw_y_axis(&y_scale, y_label);
    }
    if !title.is_empty() {
        canvas.draw_title(&title);
    }

    Ok(Value::Str(canvas.render()))
}

/// Violin plot: the shape of a distribution, per group.
///
/// The figure QC is read from - nFeature, nCount, percent.mt across samples.
/// A boxplot draws five numbers and so cannot show bimodality (Chapter 5 of the
/// MSMB companion measures exactly that blind spot); a violin draws the whole
/// density, which is the point.
///
/// Density is estimated with the same Gaussian KDE and `bw.nrd0` bandwidth
/// convention exposed by `violin_data()`. Like ggplot2's `geom_violin()`, the
/// density is trimmed to each group's observed range and ggplot/classic themes
/// use one shared area scale. The long-form input contract remains distinct
/// from `violin()`, which treats numeric table columns as groups.
pub(super) fn builtin_violin_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    let theme = plot_theme(&opts);
    let seurat_theme = get_opt_str(&opts, "theme", "") == "seurat";
    let value_col = ["value", "value_col", "y"]
        .iter()
        .map(|key| get_opt_str(&opts, key, ""))
        .find(|v| !v.is_empty())
        .unwrap_or("value")
        .to_string();
    let group_col = ["group", "group_col", "color", "x"]
        .iter()
        .map(|key| get_opt_str(&opts, key, ""))
        .find(|v| !v.is_empty())
        .unwrap_or("group")
        .to_string();

    // Collect values per group, preserving first-seen order so the axis is
    // stable across runs rather than following hash order.
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<f64>> = HashMap::new();
    let mut push = |group: String, value: f64| {
        if !value.is_finite() {
            return;
        }
        if !groups.contains_key(&group) {
            order.push(group.clone());
        }
        groups.entry(group).or_default().push(value);
    };

    match &args[0] {
        Value::List(items) => {
            for item in items.iter() {
                match item {
                    Value::Record(map) => {
                        if let Some(value) = map.get(&value_col).and_then(|v| v.as_float()) {
                            let group = map
                                .get(&group_col)
                                .map(|v| format!("{v}"))
                                .unwrap_or_else(|| "all".to_string());
                            push(group, value);
                        }
                    }
                    // A bare list of numbers is one unnamed group.
                    other => {
                        if let Some(value) = other.as_float() {
                            push("all".to_string(), value);
                        }
                    }
                }
            }
        }
        Value::Table(table) => {
            let values = extract_table_col(table, &value_col).unwrap_or_default();
            let labels = extract_str_col(table, &group_col)
                .unwrap_or_else(|_| vec!["all".to_string(); values.len()]);
            for (i, value) in values.iter().enumerate() {
                push(
                    labels.get(i).cloned().unwrap_or_else(|| "all".to_string()),
                    *value,
                );
            }
        }
        _ => {
            return Err(BioLangError::type_error(
                "violin_plot() requires a Table or List of Records",
                None,
            ))
        }
    }

    if order.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "violin_plot() found no values",
            None,
        ));
    }

    let shapes = order
        .iter()
        .map(|name| trimmed_violin_shape(name.clone(), &groups[name], 512))
        .collect::<Vec<_>>();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let mut spec_options = opts.clone();
        spec_options.insert("value_label".into(), Value::Str(value_col.clone()));
        spec_options
            .entry("legend_title".into())
            .or_insert_with(|| Value::Str(group_col.clone()));
        let spec = violin_plot_spec_value(&shapes, "long", &value_col, &spec_options);
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_violin_plot_spec_value(&spec, &opts);
    }
    if matches!(format.as_str(), "svg" | "raw") {
        let mut render_options = opts.clone();
        render_options.insert("value_label".into(), Value::Str(value_col.clone()));
        render_options
            .entry("legend_title".into())
            .or_insert_with(|| Value::Str(group_col.clone()));
        return render_long_violin_svg(&shapes, &render_options).map(Value::Str);
    }

    let title = get_opt_str(&opts, "title", "Distribution").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let width = get_opt_f64(&opts, "width", 640.0);
    let height = get_opt_f64(&opts, "height", 420.0);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);

    let lo = shapes
        .iter()
        .filter_map(|shape| shape.points.first().map(|point| point.0))
        .fold(f64::INFINITY, f64::min);
    let hi = shapes
        .iter()
        .filter_map(|shape| shape.points.last().map(|point| point.0))
        .fold(f64::NEG_INFINITY, f64::max);

    if theme.is_adaptive() {
        let provisional = Scale {
            domain: (lo, hi),
            range: (1.0, 0.0),
        };
        let widest_tick = provisional
            .nice_ticks(5)
            .iter()
            .map(|tick| estimate_text_width(&format!("{tick:.2}"), theme.tick_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_tick + 42.0).clamp(54.0, width * 0.30);
        canvas.margin.right = 18.0_f64.min(width * 0.08);
        canvas.margin.top = if subtitle.is_empty() { 48.0 } else { 66.0 };

        let provisional_slot =
            (width - canvas.margin.left - canvas.margin.right).max(1.0) / order.len() as f64;
        let widest_group = order
            .iter()
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let rotate_labels = widest_group > provisional_slot * 0.86;
        let label_reserve = if rotate_labels {
            (widest_group * 0.72 + 12.0).clamp(32.0, height * 0.28)
        } else {
            28.0
        };
        canvas.margin.bottom = label_reserve + if caption.is_empty() { 12.0 } else { 28.0 };
    }

    let y_scale = Scale {
        domain: (lo, hi),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    let slot = canvas.plot_width() / order.len() as f64;

    if theme.is_adaptive() {
        let left = canvas.margin.left;
        let right = left + canvas.plot_width();
        let top = canvas.margin.top;
        let bottom = top + canvas.plot_height();
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for tick in y_scale.nice_ticks(5) {
            let y = y_scale.map(tick);
            canvas.add_line(left, y, right, y, theme.grid_colour, theme.grid_width);
        }
        canvas.add_line(
            left,
            bottom,
            right,
            bottom,
            theme.axis_colour,
            theme.axis_width,
        );
    }

    let widest_group = order
        .iter()
        .map(|label| estimate_text_width(label, theme.tick_size))
        .fold(0.0, f64::max);
    let rotate_labels = theme.is_adaptive() && widest_group > slot * 0.86;
    let ggplot_like = matches!(theme.kind, PlotThemeKind::Ggplot | PlotThemeKind::Classic);
    let shared_peak = shapes
        .iter()
        .flat_map(|shape| shape.points.iter().map(|point| point.1))
        .fold(0.0_f64, f64::max)
        .max(1e-9);
    let ggplot_colours = ggplot_like.then(|| hue_palette(shapes.len()));

    for (gi, name) in order.iter().enumerate() {
        let values = &groups[name];
        let centre = canvas.margin.left + slot * (gi as f64 + 0.5);
        let shape = &shapes[gi].points;
        let peak = if ggplot_like {
            shared_peak
        } else {
            shape
                .iter()
                .map(|(_, density)| *density)
                .fold(f64::MIN, f64::max)
                .max(1e-9)
        };
        let half = slot * 0.42;

        // The same Gaussian KDE exposed by violin_data(), mirrored around the
        // group centre. Rendering changes width into pixels but never changes
        // the scientific grid or bandwidth.
        let mut outline: Vec<String> = Vec::new();
        for (value, density) in shape {
            outline.push(format!(
                "{:.1},{:.1}",
                centre + density / peak * half,
                y_scale.map(*value)
            ));
        }
        for (value, density) in shape.iter().rev() {
            outline.push(format!(
                "{:.1},{:.1}",
                centre - density / peak * half,
                y_scale.map(*value)
            ));
        }
        let colour = if seurat_theme {
            SEURAT_PALETTE[gi % SEURAT_PALETTE.len()]
        } else if let Some(colours) = &ggplot_colours {
            colours[gi].as_str()
        } else {
            PALETTE[gi % PALETTE.len()]
        };
        canvas.elements.push(format!(
            r#"<polygon points="{}" fill="{}" opacity="0.65" stroke="{}" stroke-width="1" />"#,
            outline.join(" "),
            colour,
            if seurat_theme { "#333333" } else { colour }
        ));

        // The median, so the violin still carries the summary a boxplot would.
        let mut sorted = values.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = quantile_type7(&sorted, 0.5);
        let my = y_scale.map(median);
        canvas.add_line(
            centre - half * 0.5,
            my,
            centre + half * 0.5,
            my,
            "#333333",
            2.0,
        );

        let label_y = canvas.margin.top + canvas.plot_height() + 16.0;
        if rotate_labels {
            canvas.add_text_rotated(centre, label_y - 3.0, name, 40.0, "start", theme.tick_size);
        } else {
            canvas.add_text(
                centre,
                label_y,
                name,
                "middle",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }

    canvas.draw_y_axis(&y_scale, &value_col);
    canvas.set_accessible_description(format!(
        "Violin plot of {value_col} for {} groups; each shape is a Gaussian kernel density and each horizontal mark is the group median.",
        order.len()
    ));
    if theme.is_adaptive() {
        canvas.draw_title(&title);
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    } else {
        canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    }
    Ok(Value::Str(canvas.render()))
}

/// One point per gene: mean expression against dispersion, with the genes
/// `highly_variable_genes` keeps drawn on top.
///
/// Seurat calls this VariableFeaturePlot and it is normally read as a sanity
/// check - "did feature selection pick the markers?" - but its real value is
/// showing *where* on the expression range the selection landed. The dispersion
/// of a count is mean-dependent, so a selection rule that ignores the trend
/// picks whatever is rarest. That bug was live in this runtime: ranking by
/// variance/mean^2 chose lncRNAs seen in two cells out of 2700 and dropped LYZ,
/// MS4A1 and GNLY entirely. On this figure it is unmistakable - every
/// highlighted point sits jammed against the left edge.
///
/// So the default y-axis is the raw dispersion against a log mean, which shows
/// the trend and the selection together. `y: "normalised"` gives the
/// bin-standardised value that is actually ranked, which is the flatter,
/// Seurat-shaped view.
pub(super) fn builtin_variable_feature_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let use_normalised = get_opt_str(&opts, "y", "dispersion").starts_with("norm");
    let n_selected = get_opt_usize(&opts, "n", 2000);
    let n_labels = get_opt_usize(&opts, "label", 10);

    // Gene names are optional; without them the points are still positioned
    // correctly, they just cannot be labelled.
    let gene_names: Vec<String> = match opts.get("genes") {
        Some(Value::List(items)) => items
            .iter()
            .map(|v| match v {
                Value::Str(s) => s.clone(),
                other => format!("{other}"),
            })
            .collect(),
        _ => Vec::new(),
    };

    // Either the matrix itself - in which case the same code that selects the
    // genes computes the coordinates - or a table of statistics computed
    // elsewhere.
    let is_records =
        matches!(&args[0], Value::List(items) if matches!(items.first(), Some(Value::Record(_))));

    struct Gene {
        name: String,
        mean: f64,
        dispersion: f64,
        selected: bool,
        rank_key: f64,
    }

    let genes: Vec<Gene> = if matches!(&args[0], Value::Table(_)) || is_records {
        let mean_col = get_opt_str(&opts, "mean_col", "mean").to_string();
        let disp_col = get_opt_str(&opts, "dispersion_col", "dispersion").to_string();
        let name_col = get_opt_str(&opts, "gene_col", "gene").to_string();

        let mut rows: Vec<Gene> = Vec::new();
        match &args[0] {
            Value::Table(table) => {
                let means = extract_table_col(table, &mean_col).unwrap_or_default();
                let disps = extract_table_col(table, &disp_col)
                    .or_else(|_| extract_table_col(table, "variance"))
                    .unwrap_or_default();
                let names = extract_str_col(table, &name_col)
                    .unwrap_or_else(|_| vec![String::new(); means.len()]);
                for i in 0..means.len().min(disps.len()) {
                    rows.push(Gene {
                        name: names.get(i).cloned().unwrap_or_default(),
                        mean: means[i],
                        dispersion: disps[i],
                        selected: false,
                        rank_key: disps[i],
                    });
                }
            }
            Value::List(items) => {
                for item in items.iter() {
                    if let Value::Record(map) = item {
                        let mean = map.get(&mean_col).and_then(|v| v.as_float());
                        let disp = map
                            .get(&disp_col)
                            .or_else(|| map.get("variance"))
                            .and_then(|v| v.as_float());
                        if let (Some(mean), Some(disp)) = (mean, disp) {
                            rows.push(Gene {
                                name: map
                                    .get(&name_col)
                                    .map(|v| format!("{v}"))
                                    .unwrap_or_default(),
                                mean,
                                dispersion: disp,
                                selected: map
                                    .get("variable")
                                    .map(|v| v.is_truthy())
                                    .unwrap_or(false),
                                rank_key: disp,
                            });
                        }
                    }
                }
            }
            _ => unreachable!("guarded by the match above"),
        }

        // An explicit highlight list wins over a `variable` column, and is how
        // you draw a selection that was made by something other than
        // highly_variable_genes.
        if let Some(Value::List(items)) = opts.get("highlight") {
            let wanted: HashSet<String> = items
                .iter()
                .map(|v| match v {
                    Value::Str(s) => s.clone(),
                    other => format!("{other}"),
                })
                .collect();
            for gene in rows.iter_mut() {
                gene.selected = wanted.contains(&gene.name);
            }
        } else if !rows.iter().any(|g| g.selected) && opts.contains_key("n") {
            // A table with no `variable` column and no highlight list carries no
            // selection, so only an explicit `n` asks for one. Defaulting to the
            // top 2000 would have highlighted every row of any smaller table -
            // a figure claiming that every gene is variable.
            let mut order: Vec<usize> = (0..rows.len()).collect();
            order.sort_by(|&a, &b| {
                rows[b]
                    .rank_key
                    .partial_cmp(&rows[a].rank_key)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for &i in order.iter().take(n_selected) {
                rows[i].selected = true;
            }
        }
        rows
    } else {
        let stats = crate::singlecell::hvg_statistics(&args[0], "variable_feature_plot")?;
        let chosen: HashSet<usize> = stats.select(n_selected).into_iter().collect();
        stats
            .expressed
            .iter()
            .enumerate()
            .map(|(position, &gene)| Gene {
                name: gene_names
                    .get(gene)
                    .cloned()
                    .unwrap_or_else(|| format!("gene{gene}")),
                mean: stats.means[gene],
                dispersion: if use_normalised {
                    stats.normalised[position]
                } else {
                    stats.dispersions[position]
                },
                selected: chosen.contains(&gene),
                rank_key: stats.normalised[position],
            })
            .collect()
    };

    if genes.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "variable_feature_plot() found no genes with a mean and a dispersion",
            None,
        ));
    }

    let title = get_opt_str(&opts, "title", "Variable features").to_string();
    let width = get_opt_f64(&opts, "width", 640.0);
    let height = get_opt_f64(&opts, "height", 460.0);
    let mut canvas = SvgCanvas::new(width, height);

    // Mean expression spans orders of magnitude, so on a linear axis every gene
    // collapses onto the left edge and the figure shows nothing.
    let xs: Vec<f64> = genes.iter().map(|g| g.mean.max(1e-6).log10()).collect();
    let ys: Vec<f64> = genes.iter().map(|g| g.dispersion).collect();
    let (x_lo, x_hi) = col_range(&xs);
    let (y_lo, y_hi) = col_range(&ys);
    let x_pad = (x_hi - x_lo) * 0.05 + 1e-3;
    let y_pad = (y_hi - y_lo) * 0.05 + 1e-3;

    let x_scale = Scale {
        domain: (x_lo - x_pad, x_hi + x_pad),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: (y_lo - y_pad, y_hi + y_pad),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    // Unselected first, so the selection is never buried under the cloud. The
    // two layers raster separately because they are drawn at different radii,
    // which still leaves two elements instead of tens of thousands.
    let raster = raster_choice(&opts, "variable_feature_plot", genes.len())?;
    let area = canvas.point_area();
    let background: Vec<(f64, f64, &str)> = genes
        .iter()
        .enumerate()
        .filter(|(_, gene)| !gene.selected)
        .map(|(i, _)| (x_scale.map(xs[i]), y_scale.map(ys[i]), "#bbbbbb"))
        .collect();
    canvas.add_scatter(&background, 1.6, area, raster);
    // Red on grey, as Seurat draws it - the selection has to read at a glance
    // against a cloud of tens of thousands of points.
    let selected: Vec<(f64, f64, &str)> = genes
        .iter()
        .enumerate()
        .filter(|(_, gene)| gene.selected)
        .map(|(i, _)| (x_scale.map(xs[i]), y_scale.map(ys[i]), PALETTE[2]))
        .collect();
    let n_variable = selected.len();
    canvas.add_scatter(&selected, 2.4, area, raster);

    // Label the strongest few. Any more and the labels cover the cloud they are
    // meant to explain.
    let mut ranked: Vec<usize> = (0..genes.len()).filter(|&i| genes[i].selected).collect();
    ranked.sort_by(|&a, &b| {
        genes[b]
            .rank_key
            .partial_cmp(&genes[a].rank_key)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    for &i in ranked.iter().take(n_labels) {
        if genes[i].name.is_empty() {
            continue;
        }
        canvas.add_text(
            x_scale.map(xs[i]) + 4.0,
            y_scale.map(ys[i]) - 4.0,
            &genes[i].name,
            "start",
            8.0,
        );
    }

    canvas.draw_x_axis(&x_scale, "log10 mean expression");
    canvas.draw_y_axis(
        &y_scale,
        if use_normalised {
            "standardised dispersion"
        } else {
            "dispersion (variance / mean)"
        },
    );
    canvas.add_text(width / 2.0, 22.0, &title, "middle", 14.0);
    canvas.add_text(
        canvas.margin.left,
        36.0,
        &format!("{n_variable} variable of {} genes", genes.len()),
        "start",
        10.0,
    );

    Ok(Value::Str(canvas.render()))
}
