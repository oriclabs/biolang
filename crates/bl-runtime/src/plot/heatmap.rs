//! Heatmap for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(super) fn interpolate_viridis(t: f64) -> String {
    // Viridis: dark purple → teal → yellow (5-stop approximation)
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (68.0, 1.0, 84.0),    // 0.00 — dark purple
        (59.0, 82.0, 139.0),  // 0.25 — blue-purple
        (33.0, 145.0, 140.0), // 0.50 — teal
        (94.0, 201.0, 98.0),  // 0.75 — green
        (253.0, 231.0, 37.0), // 1.00 — yellow
    ];
    heatmap_interp_stops(t, &stops)
}

pub(super) fn interpolate_plasma(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (13.0, 8.0, 135.0),   // deep blue
        (126.0, 3.0, 168.0),  // purple
        (204.0, 71.0, 120.0), // pink
        (248.0, 149.0, 64.0), // orange
        (240.0, 249.0, 33.0), // yellow
    ];
    heatmap_interp_stops(t, &stops)
}

pub(super) fn interpolate_inferno(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (0.0, 0.0, 4.0),       // black
        (87.0, 16.0, 110.0),   // dark purple
        (188.0, 55.0, 84.0),   // red
        (249.0, 142.0, 9.0),   // orange
        (252.0, 255.0, 164.0), // light yellow
    ];
    heatmap_interp_stops(t, &stops)
}

pub(super) fn interpolate_rdbu(t: f64) -> String {
    // Diverging: blue (low) → white (mid) → red (high)
    let t = t.clamp(0.0, 1.0);
    let stops: [(f64, f64, f64); 5] = [
        (33.0, 102.0, 172.0),  // strong blue
        (146.0, 197.0, 222.0), // light blue
        (247.0, 247.0, 247.0), // white/near-white
        (239.0, 138.0, 98.0),  // light red
        (178.0, 24.0, 43.0),   // strong red
    ];
    heatmap_interp_stops(t, &stops)
}

pub(super) fn interpolate_blues(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (247.0 - t * 239.0) as u8;
    let g = (251.0 - t * 183.0) as u8;
    let b = (255.0 - t * 69.0) as u8;
    format!("rgb({r},{g},{b})")
}

pub(super) fn interpolate_reds(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (255.0 - t * 52.0) as u8;
    let g = (245.0 - t * 227.0) as u8;
    let b = (240.0 - t * 240.0) as u8;
    format!("rgb({r},{g},{b})")
}

pub(super) fn interpolate_greens(t: f64) -> String {
    let t = t.clamp(0.0, 1.0);
    let r = (247.0 - t * 247.0) as u8;
    let g = (252.0 - t * 102.0) as u8;
    let b = (245.0 - t * 200.0) as u8;
    format!("rgb({r},{g},{b})")
}

/// Linearly interpolate between N evenly-spaced color stops.
pub(super) fn heatmap_interp_stops(t: f64, stops: &[(f64, f64, f64)]) -> String {
    let n = stops.len();
    if n == 0 {
        return "rgb(128,128,128)".into();
    }
    if n == 1 {
        let (r, g, b) = stops[0];
        return format!("rgb({},{},{})", r as u8, g as u8, b as u8);
    }
    let t = t.clamp(0.0, 1.0);
    let seg = t * (n - 1) as f64;
    let i = (seg.floor() as usize).min(n - 2);
    let f = seg - i as f64;
    let (r0, g0, b0) = stops[i];
    let (r1, g1, b1) = stops[i + 1];
    let r = (r0 + f * (r1 - r0)) as u8;
    let g = (g0 + f * (g1 - g0)) as u8;
    let b = (b0 + f * (b1 - b0)) as u8;
    format!("rgb({r},{g},{b})")
}

pub(super) fn heatmap_color(t: f64, scheme: &str) -> String {
    match scheme {
        "viridis" => interpolate_viridis(t),
        "plasma" => interpolate_plasma(t),
        "inferno" => interpolate_inferno(t),
        "rdbu" => interpolate_rdbu(t),
        "blues" => interpolate_blues(t),
        "reds" => interpolate_reds(t),
        "greens" => interpolate_greens(t),
        _ => interpolate_viridis(t),
    }
}

/// Text color for readability: white on dark cells, black on light cells.
pub(super) fn heatmap_text_color(t: f64, scheme: &str) -> &'static str {
    match scheme {
        "rdbu" => {
            // mid-range is white/light, extremes are dark
            if !(0.25..=0.75).contains(&t) {
                "white"
            } else {
                "#333"
            }
        }
        "blues" | "greens" | "reds" => {
            if t > 0.6 {
                "white"
            } else {
                "#333"
            }
        }
        // viridis, plasma, inferno: dark at low end, bright at high end
        _ => {
            if t < 0.55 {
                "white"
            } else {
                "#333"
            }
        }
    }
}

/// Simple row clustering by sorting rows by their mean value.
pub(super) fn cluster_rows(
    row_data: &mut Vec<Vec<f64>>,
    row_labels: &mut Vec<String>,
) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..row_data.len()).collect();
    indices.sort_by(|&a, &b| {
        let mean_a: f64 = row_data[a]
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .sum::<f64>()
            / row_data[a].len().max(1) as f64;
        let mean_b: f64 = row_data[b]
            .iter()
            .copied()
            .filter(|v| v.is_finite())
            .sum::<f64>()
            / row_data[b].len().max(1) as f64;
        mean_a
            .partial_cmp(&mean_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let orig_rows = row_data.clone();
    let orig_labels = row_labels.clone();
    for (new_i, &old_i) in indices.iter().enumerate() {
        row_data[new_i] = orig_rows[old_i].clone();
        if old_i < orig_labels.len() {
            row_labels[new_i] = orig_labels[old_i].clone();
        }
    }
    indices
}

pub(super) fn render_heatmap_geometry_svg(
    row_data: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    scale_min: f64,
    scale_max: f64,
    use_diverging: bool,
    scheme_explicit: bool,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let title = get_opt_str(opts, "title", "Heatmap").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let legend_title = get_opt_str(opts, "legend_title", "value").to_string();
    let na_colour = get_opt_str(opts, "na_color", "#cccccc").to_string();
    let theme = plot_theme(opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let scheme = get_opt_str(opts, "colors", "viridis").to_string();
    let show_values = opts
        .get("show_values")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let do_cluster = opts
        .get("cluster")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let nrows = row_data.len();
    let ncols = row_data.first().map(Vec::len).unwrap_or(0);
    if nrows == 0 || ncols == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() heatmap specification is empty",
            None,
        ));
    }
    let cell_colour = |t: f64| {
        if publication_theme && !scheme_explicit {
            if use_diverging {
                publication_diverging_color(t)
            } else {
                publication_sequential_color(t)
            }
        } else {
            heatmap_color(t, &scheme)
        }
    };
    let max_row_label_len = row_labels.iter().map(String::len).max().unwrap_or(0);
    let left_margin = 40.0 + (max_row_label_len as f64 * 7.0).min(120.0);
    let legend_width = 60.0;
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    if theme.is_adaptive() {
        let widest_row = row_labels
            .iter()
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let widest_col = col_labels
            .iter()
            .take(ncols)
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let legend_label = [scale_min, (scale_min + scale_max) / 2.0, scale_max]
            .iter()
            .map(|value| estimate_text_width(&format!("{value:.2}"), theme.legend_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_row + 12.0).clamp(48.0, width * 0.31);
        canvas.margin.right = (42.0
            + legend_label.max(estimate_text_width(&legend_title, theme.legend_size)))
        .clamp(76.0, width * 0.31);
        canvas.margin.top = if title.is_empty() {
            20.0
        } else if subtitle.is_empty() {
            48.0
        } else {
            66.0
        };
        canvas.margin.bottom = (widest_col * 0.72 + 18.0).clamp(48.0, height * 0.28)
            + if caption.is_empty() { 0.0 } else { 18.0 };
    } else {
        canvas.margin.left = left_margin;
        canvas.margin.bottom = 70.0;
        canvas.margin.right = 20.0 + legend_width;
        canvas.margin.top = if title.is_empty() { 20.0 } else { 45.0 };
    }
    let plot_w = canvas.plot_width();
    let plot_h = canvas.plot_height();
    let cell_w = plot_w / ncols as f64;
    let cell_h = plot_h / nrows as f64;
    for (ri, row) in row_data.iter().enumerate() {
        for (ci, &value) in row.iter().enumerate() {
            let t = if (scale_max - scale_min).abs() < f64::EPSILON {
                0.5
            } else {
                (value - scale_min) / (scale_max - scale_min)
            };
            let colour = if value.is_finite() {
                cell_colour(t)
            } else {
                na_colour.clone()
            };
            let x = canvas.margin.left + ci as f64 * cell_w;
            let y = canvas.margin.top + ri as f64 * cell_h;
            canvas.add_rect(x, y, cell_w, cell_h, &colour);
            if !theme.is_adaptive() || cell_w.min(cell_h) >= 4.0 {
                canvas.elements.push(format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.5\" />",
                    x,
                    y,
                    cell_w,
                    cell_h,
                    if theme.is_adaptive() { theme.grid_colour } else { "#eee" }
                ));
            }
            if show_values && value.is_finite() {
                let text_colour = heatmap_text_color(t, &scheme);
                let label = if value.abs() >= 100.0 || value == 0.0 {
                    format!("{value:.0}")
                } else if value.abs() >= 1.0 {
                    format!("{value:.1}")
                } else {
                    format!("{value:.2}")
                };
                let font_size = (cell_w.min(cell_h) * 0.35).clamp(7.0, 14.0);
                canvas.elements.push(format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="central" font-size="{:.1}" font-family="{}" fill="{}">{}</text>"#,
                    x + cell_w / 2.0,
                    y + cell_h / 2.0,
                    font_size,
                    theme.font_family,
                    text_colour,
                    label.replace('&', "&amp;").replace('<', "&lt;")
                ));
            }
        }
    }
    let col_step = if theme.is_adaptive() {
        (10.0 / cell_w.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ci, label) in col_labels.iter().enumerate().step_by(col_step) {
        if ci < ncols {
            canvas.add_text_rotated(
                canvas.margin.left + (ci as f64 + 0.5) * cell_w,
                canvas.margin.top + plot_h + 10.0,
                label,
                45.0,
                "start",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }
    let row_step = if theme.is_adaptive() {
        (10.0 / cell_h.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ri, label) in row_labels.iter().enumerate().step_by(row_step) {
        if ri < nrows {
            canvas.add_text(
                canvas.margin.left - 6.0,
                canvas.margin.top + (ri as f64 + 0.5) * cell_h + 4.0,
                label,
                "end",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }
    let legend_x = canvas.margin.left + plot_w + 15.0;
    let legend_top = canvas.margin.top;
    let legend_h = plot_h.min(200.0);
    let legend_bar_w = 15.0;
    let legend_steps = 50usize;
    let step_h = legend_h / legend_steps as f64;
    if theme.is_adaptive() && !legend_title.is_empty() {
        canvas.add_text(
            legend_x,
            legend_top - 8.0,
            &legend_title,
            "start",
            theme.legend_size,
        );
    }
    for i in 0..legend_steps {
        let t = 1.0 - i as f64 / (legend_steps - 1) as f64;
        canvas.elements.push(format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" />"#,
            legend_x,
            legend_top + i as f64 * step_h,
            legend_bar_w,
            step_h + 0.5,
            cell_colour(t)
        ));
    }
    canvas.elements.push(format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#333\" stroke-width=\"0.5\" />",
        legend_x, legend_top, legend_bar_w, legend_h
    ));
    let label_x = legend_x + legend_bar_w + 5.0;
    canvas.add_text(
        label_x,
        legend_top + 4.0,
        &format!("{scale_max:.2}"),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h / 2.0 + 3.0,
        &format!("{:.2}", (scale_min + scale_max) / 2.0),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h + 3.0,
        &format!("{scale_min:.2}"),
        "start",
        9.0,
    );
    canvas.set_accessible_description(format!(
        "Heatmap with {nrows} rows and {ncols} columns. Rows are {}.",
        if do_cluster {
            "sorted by their mean value"
        } else {
            "shown in input order"
        }
    ));
    if !title.is_empty() {
        canvas.draw_title(&title);
    }
    if theme.is_adaptive() {
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    }
    Ok(canvas.render())
}

pub(super) fn heatmap_plot_spec_value(
    row_data: &[Vec<f64>],
    row_labels: &[String],
    col_labels: &[String],
    row_order: &[usize],
    value_min: f64,
    value_max: f64,
    scale_min: f64,
    scale_max: f64,
    use_diverging: bool,
    scheme_explicit: bool,
    opts: &HashMap<String, Value>,
) -> Value {
    let cells = row_data
        .iter()
        .enumerate()
        .flat_map(|(display_row, row)| {
            row.iter().enumerate().map(move |(display_col, &value)| {
                vec![
                    Value::Int(display_row as i64),
                    Value::Int(row_order[display_row] as i64),
                    Value::Int(display_col as i64),
                    Value::Int(display_col as i64),
                    Value::Float(value),
                ]
            })
        })
        .collect();
    let row_rows = row_labels
        .iter()
        .enumerate()
        .map(|(display_row, label)| {
            vec![
                Value::Int(display_row as i64),
                Value::Int(row_order[display_row] as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let col_rows = col_labels
        .iter()
        .enumerate()
        .map(|(display_col, label)| {
            vec![
                Value::Int(display_col as i64),
                Value::Int(display_col as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let non_finite = row_data
        .iter()
        .flat_map(|row| row.iter())
        .filter(|value| !value.is_finite())
        .count();
    let options = HashMap::from([
        ("plot".into(), Value::Str("heatmap".into())),
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", "Heatmap").into()),
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
            "legend_title".into(),
            Value::Str(get_opt_str(opts, "legend_title", "value").into()),
        ),
        (
            "na_color".into(),
            Value::Str(get_opt_str(opts, "na_color", "#cccccc").into()),
        ),
        (
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        (
            "colors".into(),
            Value::Str(get_opt_str(opts, "colors", "viridis").into()),
        ),
        ("colors_explicit".into(), Value::Bool(scheme_explicit)),
        (
            "show_values".into(),
            Value::Bool(
                opts.get("show_values")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
        (
            "cluster".into(),
            Value::Bool(
                opts.get("cluster")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            ),
        ),
        ("value_min".into(), Value::Float(value_min)),
        ("value_max".into(), Value::Float(value_max)),
        (
            "center".into(),
            opts.get("center").cloned().unwrap_or(Value::Nil),
        ),
        ("scale_min".into(), Value::Float(scale_min)),
        ("scale_max".into(), Value::Float(scale_max)),
        ("diverging".into(), Value::Bool(use_diverging)),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 800.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 600.0)),
        ),
    ]);
    Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str("heatmap".into())),
            ("plot".into(), Value::Str("heatmap".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "Heatmap").into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "display_row",
                        "source_row",
                        "display_col",
                        "source_col",
                        "value",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    cells,
                )),
            ),
            (
                "rows".into(),
                Value::Table(Table::new(
                    ["display_row", "source_row", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    row_rows,
                )),
            ),
            (
                "columns".into(),
                Value::Table(Table::new(
                    ["display_col", "source_col", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    col_rows,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("heatmap".into())),
                        ("input_rows".into(), Value::Int(row_data.len() as i64)),
                        (
                            "input_columns".into(),
                            Value::Int(row_data.first().map(Vec::len).unwrap_or(0) as i64),
                        ),
                        ("non_finite_cells".into(), Value::Int(non_finite as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "warnings".into(),
                Value::List(
                    if non_finite == 0 {
                        Vec::new()
                    } else {
                        vec![Value::Str(format!(
                            "{non_finite} heatmap cells are non-finite and use na_color"
                        ))]
                    }
                    .into(),
                ),
            ),
        ])
        .into(),
    )
}

pub(super) fn is_heatmap_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "heatmap")
                && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "heatmap")
    )
}

pub(super) fn render_heatmap_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_heatmap_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 heatmap Record",
                None,
            ))
        }
    };
    let table_field = |name: &str| -> Result<&Table> {
        match map.get(name) {
            Some(Value::Table(table)) => Ok(table),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() heatmap specification field '{name}' must be Table"),
                None,
            )),
        }
    };
    let cells = table_field("data")?;
    let rows = table_field("rows")?;
    let columns = table_field("columns")?;
    for required in [
        "display_row",
        "source_row",
        "display_col",
        "source_col",
        "value",
    ] {
        if cells.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() heatmap data is missing '{required}'"),
                None,
            ));
        }
    }
    for (table, axis, required) in [
        (rows, "row", ["display_row", "source_row", "label"]),
        (columns, "column", ["display_col", "source_col", "label"]),
    ] {
        for field in required {
            if table.col_index(field).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() heatmap {axis} metadata is missing '{field}'"),
                    None,
                ));
            }
        }
    }
    if rows.num_rows() == 0 || columns.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() heatmap specification is empty",
            None,
        ));
    }
    let labels = |table: &Table| {
        let index = table.col_index("label").unwrap();
        table
            .rows
            .iter()
            .map(|row| format!("{}", row[index]))
            .collect::<Vec<_>>()
    };
    let row_labels = labels(rows);
    let col_labels = labels(columns);
    let expected = rows.num_rows() * columns.num_rows();
    if cells.num_rows() != expected {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() heatmap data must contain one cell per displayed row and column",
            None,
        ));
    }
    let ri = cells.col_index("display_row").unwrap();
    let ci = cells.col_index("display_col").unwrap();
    let vi = cells.col_index("value").unwrap();
    let mut row_data = vec![vec![f64::NAN; columns.num_rows()]; rows.num_rows()];
    for (expected_index, row) in cells.rows.iter().enumerate() {
        let display_row = row[ri]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() heatmap display_row must be numeric",
                    None,
                )
            })?;
        let display_col = row[ci]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() heatmap display_col must be numeric",
                    None,
                )
            })?;
        if display_row >= rows.num_rows()
            || display_col >= columns.num_rows()
            || expected_index != display_row * columns.num_rows() + display_col
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() heatmap cells must be complete and ordered by display row and column",
                None,
            ));
        }
        row_data[display_row][display_col] = row[vi].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() heatmap cell values must be numeric",
                None,
            )
        })?;
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() heatmap specification field 'options' must be Record",
                None,
            ))
        }
    };
    let number = |name: &str| -> Result<f64> {
        options.get(name).and_then(Value::as_float).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() heatmap options are missing numeric '{name}'"),
                None,
            )
        })
    };
    let scale_min = number("scale_min")?;
    let scale_max = number("scale_max")?;
    let use_diverging = options.get("diverging").is_some_and(Value::is_truthy);
    let scheme_explicit = options.get("colors_explicit").is_some_and(Value::is_truthy);
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for key in ["width", "height"] {
        if let Some(override_value) = render_options.get(key) {
            options.insert(key.into(), override_value.clone());
        }
    }
    let svg = render_heatmap_geometry_svg(
        &row_data,
        &row_labels,
        &col_labels,
        scale_min,
        scale_max,
        use_diverging,
        scheme_explicit,
        &options,
    )?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Heatmap");
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
            "render_plot() terminal heatmap output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown heatmap format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

pub(super) fn builtin_heatmap(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "Heatmap").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let legend_title = get_opt_str(&opts, "legend_title", "value").to_string();
    let na_colour = get_opt_str(&opts, "na_color", "#cccccc").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let scheme_explicit = opts.contains_key("colors");
    let scheme = get_opt_str(&opts, "colors", "viridis").to_string();
    let show_values = opts
        .get("show_values")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let do_cluster = opts
        .get("cluster")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // User-supplied row/col labels
    let user_row_labels: Option<Vec<String>> = opts.get("row_labels").and_then(|v| {
        if let Value::List(items) = v {
            Some(items.iter().map(|i| format!("{i}")).collect())
        } else {
            None
        }
    });
    let user_col_labels: Option<Vec<String>> = opts.get("col_labels").and_then(|v| {
        if let Value::List(items) = v {
            Some(items.iter().map(|i| format!("{i}")).collect())
        } else {
            None
        }
    });

    // Extract data into row-major matrix: row_data[row][col]
    let (mut col_labels, mut row_data, mut row_labels) = match &args[0] {
        Value::Table(table) => {
            let cl = table.columns.clone();
            let mut rd: Vec<Vec<f64>> = Vec::with_capacity(table.num_rows());
            let mut rl: Vec<String> = Vec::with_capacity(table.num_rows());
            for (ri, row) in table.rows.iter().enumerate() {
                let mut rv = Vec::with_capacity(row.len());
                for val in row {
                    rv.push(match val {
                        Value::Int(n) => *n as f64,
                        Value::Float(f) => *f,
                        Value::Str(s) => s.parse::<f64>().unwrap_or(f64::NAN),
                        _ => f64::NAN,
                    });
                }
                rd.push(rv);
                rl.push(format!("{}", ri + 1));
            }
            (cl, rd, rl)
        }
        Value::Matrix(m) => {
            let cl = m
                .col_names
                .clone()
                .unwrap_or_else(|| (0..m.ncol).map(|i| format!("col{i}")).collect());
            let mut rd = Vec::with_capacity(m.nrow);
            let rl: Vec<String> = m
                .row_names
                .clone()
                .unwrap_or_else(|| (0..m.nrow).map(|i| format!("{}", i + 1)).collect());
            for r in 0..m.nrow {
                let row_start = r * m.ncol;
                rd.push(m.data[row_start..row_start + m.ncol].to_vec());
            }
            (cl, rd, rl)
        }
        Value::List(items) => {
            // List of Lists (matrix) or List of Records
            if items.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "heatmap() received empty list",
                    None,
                ));
            }
            match &items[0] {
                Value::List(_) => {
                    // List of Lists
                    let mut rd = Vec::with_capacity(items.len());
                    let mut max_cols = 0usize;
                    for item in items.iter() {
                        if let Value::List(row) = item {
                            let rv: Vec<f64> = row
                                .iter()
                                .map(|v| match v {
                                    Value::Int(n) => *n as f64,
                                    Value::Float(f) => *f,
                                    _ => f64::NAN,
                                })
                                .collect();
                            if rv.len() > max_cols {
                                max_cols = rv.len();
                            }
                            rd.push(rv);
                        } else {
                            return Err(BioLangError::type_error(
                                "heatmap() list items must all be Lists or Records",
                                None,
                            ));
                        }
                    }
                    let cl: Vec<String> = (0..max_cols).map(|i| format!("col{i}")).collect();
                    let rl: Vec<String> = (0..rd.len()).map(|i| format!("{}", i + 1)).collect();
                    (cl, rd, rl)
                }
                Value::Record(_) => {
                    // List of Records — collect all field names as columns
                    let mut all_keys = Vec::new();
                    let mut key_set = std::collections::HashSet::new();
                    for item in items.iter() {
                        if let Value::Record(map) = item {
                            for k in map.keys() {
                                if key_set.insert(k.clone()) {
                                    all_keys.push(k.clone());
                                }
                            }
                        }
                    }
                    let mut rd = Vec::with_capacity(items.len());
                    for item in items.iter() {
                        if let Value::Record(map) = item {
                            let rv: Vec<f64> = all_keys
                                .iter()
                                .map(|k| {
                                    map.get(k)
                                        .map(|v| match v {
                                            Value::Int(n) => *n as f64,
                                            Value::Float(f) => *f,
                                            _ => f64::NAN,
                                        })
                                        .unwrap_or(f64::NAN)
                                })
                                .collect();
                            rd.push(rv);
                        }
                    }
                    let rl: Vec<String> = (0..rd.len()).map(|i| format!("{}", i + 1)).collect();
                    (all_keys, rd, rl)
                }
                _ => {
                    return Err(BioLangError::type_error(
                        "heatmap() requires Table, Matrix, List of Lists, or List of Records",
                        None,
                    ))
                }
            }
        }
        _ => {
            return Err(BioLangError::type_error(
                "heatmap() requires Table, Matrix, List of Lists, or List of Records",
                None,
            ))
        }
    };

    // Apply user-supplied labels if given
    if let Some(ul) = user_row_labels {
        for (i, label) in ul.into_iter().enumerate() {
            if i < row_labels.len() {
                row_labels[i] = label;
            }
        }
    }
    if let Some(ul) = user_col_labels {
        col_labels = ul;
    }

    let nrows = row_data.len();
    let ncols = if nrows > 0 {
        row_data[0].len()
    } else {
        col_labels.len()
    };

    if nrows == 0 || ncols == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "heatmap() received empty data",
            None,
        ));
    }

    // Optional clustering (sort rows by mean)
    let row_order = if do_cluster {
        cluster_rows(&mut row_data, &mut row_labels)
    } else {
        (0..nrows).collect()
    };

    // Compute global min/max
    let mut all_vals = Vec::new();
    for row in &row_data {
        for &v in row {
            if v.is_finite() {
                all_vals.push(v);
            }
        }
    }
    let (vmin, vmax) = col_range(&all_vals);
    let requested_centre = opts.get("center").and_then(Value::as_float);
    let use_diverging = publication_theme
        && !scheme_explicit
        && (requested_centre.is_some() || (vmin < 0.0 && vmax > 0.0));
    let (scale_min, scale_max) = if use_diverging {
        let centre = requested_centre.unwrap_or(0.0);
        let radius = (vmin - centre)
            .abs()
            .max((vmax - centre).abs())
            .max(f64::EPSILON);
        (centre - radius, centre + radius)
    } else {
        (vmin, vmax)
    };

    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        if row_data.iter().any(|row| row.len() != ncols) || col_labels.len() < ncols {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "heatmap() inspectable output requires rectangular data and one label per column",
                None,
            ));
        }
        let spec = heatmap_plot_spec_value(
            &row_data,
            &row_labels,
            &col_labels[..ncols],
            &row_order,
            vmin,
            vmax,
            scale_min,
            scale_max,
            use_diverging,
            scheme_explicit,
            &opts,
        );
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_heatmap_plot_spec_value(&spec, &opts);
    }
    if matches!(format.as_str(), "svg" | "raw") {
        return render_heatmap_geometry_svg(
            &row_data,
            &row_labels,
            &col_labels,
            scale_min,
            scale_max,
            use_diverging,
            scheme_explicit,
            &opts,
        )
        .map(Value::Str);
    }
    let cell_colour = |t: f64| {
        if publication_theme && !scheme_explicit {
            if use_diverging {
                publication_diverging_color(t)
            } else {
                publication_sequential_color(t)
            }
        } else {
            heatmap_color(t, &scheme)
        }
    };

    // Compute margins based on label lengths
    let max_row_label_len = row_labels.iter().map(|s| s.len()).max().unwrap_or(0);
    let left_margin = 40.0 + (max_row_label_len as f64 * 7.0).min(120.0);
    let legend_width = 60.0;

    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    if theme.is_adaptive() {
        let widest_row = row_labels
            .iter()
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let widest_col = col_labels
            .iter()
            .take(ncols)
            .map(|label| estimate_text_width(label, theme.tick_size))
            .fold(0.0, f64::max);
        let legend_label = [scale_min, (scale_min + scale_max) / 2.0, scale_max]
            .iter()
            .map(|value| estimate_text_width(&format!("{value:.2}"), theme.legend_size))
            .fold(0.0, f64::max);
        canvas.margin.left = (widest_row + 12.0).clamp(48.0, width * 0.31);
        canvas.margin.right = (42.0
            + legend_label.max(estimate_text_width(&legend_title, theme.legend_size)))
        .clamp(76.0, width * 0.31);
        canvas.margin.top = if title.is_empty() {
            20.0
        } else if subtitle.is_empty() {
            48.0
        } else {
            66.0
        };
        canvas.margin.bottom = (widest_col * 0.72 + 18.0).clamp(48.0, height * 0.28)
            + if caption.is_empty() { 0.0 } else { 18.0 };
    } else {
        canvas.margin.left = left_margin;
        canvas.margin.bottom = 70.0;
        canvas.margin.right = 20.0 + legend_width;
        canvas.margin.top = if title.is_empty() { 20.0 } else { 45.0 };
    }

    let plot_w = canvas.plot_width();
    let plot_h = canvas.plot_height();
    let cell_w = plot_w / ncols as f64;
    let cell_h = plot_h / nrows as f64;

    // Draw cells
    for (ri, row) in row_data.iter().enumerate() {
        for (ci, &v) in row.iter().enumerate() {
            let t = if (scale_max - scale_min).abs() < f64::EPSILON {
                0.5
            } else {
                (v - scale_min) / (scale_max - scale_min)
            };
            let color = if v.is_finite() {
                cell_colour(t)
            } else {
                na_colour.clone()
            };
            let x = canvas.margin.left + ci as f64 * cell_w;
            let y = canvas.margin.top + ri as f64 * cell_h;
            canvas.add_rect(x, y, cell_w, cell_h, &color);

            // Cell border for visual separation
            if !theme.is_adaptive() || cell_w.min(cell_h) >= 4.0 {
                canvas.elements.push(format!(
                    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"{}\" stroke-width=\"0.5\" />",
                    x,
                    y,
                    cell_w,
                    cell_h,
                    if theme.is_adaptive() { theme.grid_colour } else { "#eee" }
                ));
            }

            // Show numeric value in cell
            if show_values && v.is_finite() {
                let txt_color = heatmap_text_color(t, &scheme);
                let label = if v.abs() >= 100.0 || v == 0.0 {
                    format!("{:.0}", v)
                } else if v.abs() >= 1.0 {
                    format!("{:.1}", v)
                } else {
                    format!("{:.2}", v)
                };
                let font_size = (cell_w.min(cell_h) * 0.35).clamp(7.0, 14.0);
                canvas.elements.push(format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="central" font-size="{:.1}" font-family="{}" fill="{}">{}</text>"#,
                    x + cell_w / 2.0, y + cell_h / 2.0, font_size,
                    theme.font_family, txt_color,
                    label.replace('&', "&amp;").replace('<', "&lt;")
                ));
            }
        }
    }

    // Column labels (rotated at bottom)
    let col_step = if theme.is_adaptive() {
        (10.0 / cell_w.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ci, col) in col_labels.iter().enumerate().step_by(col_step) {
        if ci < ncols {
            let x = canvas.margin.left + (ci as f64 + 0.5) * cell_w;
            let y = canvas.margin.top + plot_h + 10.0;
            canvas.add_text_rotated(
                x,
                y,
                col,
                45.0,
                "start",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }

    // Row labels (on the left)
    let row_step = if theme.is_adaptive() {
        (10.0 / cell_h.max(1.0)).ceil().max(1.0) as usize
    } else {
        1
    };
    for (ri, label) in row_labels.iter().enumerate().step_by(row_step) {
        if ri < nrows {
            let y = canvas.margin.top + (ri as f64 + 0.5) * cell_h + 4.0;
            canvas.add_text(
                canvas.margin.left - 6.0,
                y,
                label,
                "end",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    10.0
                },
            );
        }
    }

    // Color legend / scale bar (right side)
    let legend_x = canvas.margin.left + plot_w + 15.0;
    let legend_top = canvas.margin.top;
    let legend_h = plot_h.min(200.0);
    let legend_bar_w = 15.0;
    let legend_steps = 50usize;
    let step_h = legend_h / legend_steps as f64;
    if theme.is_adaptive() && !legend_title.is_empty() {
        canvas.add_text(
            legend_x,
            legend_top - 8.0,
            &legend_title,
            "start",
            theme.legend_size,
        );
    }
    for i in 0..legend_steps {
        let t = 1.0 - (i as f64 / (legend_steps - 1) as f64); // top = max
        let color = cell_colour(t);
        let y = legend_top + i as f64 * step_h;
        canvas.elements.push(format!(
            r#"<rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="{}" />"#,
            legend_x,
            y,
            legend_bar_w,
            step_h + 0.5,
            color
        ));
    }
    // Legend border
    canvas.elements.push(format!(
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" fill=\"none\" stroke=\"#333\" stroke-width=\"0.5\" />",
        legend_x, legend_top, legend_bar_w, legend_h
    ));
    // Legend tick labels
    let label_x = legend_x + legend_bar_w + 5.0;
    canvas.add_text(
        label_x,
        legend_top + 4.0,
        &format!("{scale_max:.2}"),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h / 2.0 + 3.0,
        &format!("{:.2}", (scale_min + scale_max) / 2.0),
        "start",
        9.0,
    );
    canvas.add_text(
        label_x,
        legend_top + legend_h + 3.0,
        &format!("{scale_min:.2}"),
        "start",
        9.0,
    );

    // Title
    canvas.set_accessible_description(format!(
        "Heatmap with {nrows} rows and {ncols} columns. Rows are {}.",
        if do_cluster {
            "sorted by their mean value"
        } else {
            "shown in input order"
        }
    ));
    if !title.is_empty() {
        canvas.draw_title(&title);
    }
    if theme.is_adaptive() {
        canvas.draw_subtitle(&subtitle);
        canvas.draw_caption(&caption);
    }

    Ok(Value::Str(canvas.render()))
}
