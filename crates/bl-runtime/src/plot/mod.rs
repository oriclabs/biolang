use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Arity, Table, Value};
use std::collections::HashMap;

mod theme;
pub(crate) use theme::*;

mod canvas;
pub(crate) use canvas::*;

mod histogram;
pub(crate) use histogram::*;

mod heatmap;
pub(crate) use heatmap::*;

mod spec;
pub(crate) use spec::*;

mod raster;
pub(crate) use raster::*;

mod differential;
pub(crate) use differential::*;

mod distribution;
pub(crate) use distribution::*;

pub fn plot_builtin_list() -> Vec<(&'static str, Arity)> {
    vec![
        ("plot", Arity::Range(1, 2)),
        ("plot_spec", Arity::Range(1, 2)),
        ("render_plot", Arity::Range(1, 2)),
        ("plot_grid", Arity::Range(1, 2)),
        ("heatmap", Arity::Range(1, 2)),
        ("mosaic_plot", Arity::Range(1, 2)),
        ("mosaic_data", Arity::Range(1, 2)),
        ("histogram", Arity::Range(1, 2)),
        ("histogram_data", Arity::Range(1, 2)),
        ("boxplot_data", Arity::Range(1, 2)),
        ("ecdf_data", Arity::Range(1, 2)),
        ("normal_qq_data", Arity::Range(1, 2)),
        ("violin_data", Arity::Range(1, 2)),
        ("linear_fit_data", Arity::Range(2, 3)),
        ("categorical_data", Arity::Exact(1)),
        ("missingness_data", Arity::Range(1, 2)),
        ("ecdf_plot", Arity::Range(1, 2)),
        ("density_plot", Arity::Range(1, 2)),
        ("volcano", Arity::Range(1, 2)),
        ("ma_plot", Arity::Range(1, 2)),
        ("save_svg", Arity::Range(2, 3)),
        ("save_plot", Arity::Range(2, 3)),
        ("save_png", Arity::Range(2, 3)),
        ("genome_track", Arity::Range(1, 2)),
    ]
}

pub fn is_plot_builtin(name: &str) -> bool {
    matches!(
        name,
        "plot"
            | "plot_spec"
            | "render_plot"
            | "plot_grid"
            | "heatmap"
            | "mosaic_plot"
            | "mosaic_data"
            | "histogram"
            | "histogram_data"
            | "boxplot_data"
            | "ecdf_data"
            | "normal_qq_data"
            | "violin_data"
            | "linear_fit_data"
            | "categorical_data"
            | "missingness_data"
            | "ecdf_plot"
            | "density_plot"
            | "volcano"
            | "ma_plot"
            | "save_svg"
            | "save_plot"
            | "save_png"
            | "genome_track"
    )
}

/// Normalize single-Record calling convention for plot functions.
/// `func({data: table, title: "..."})` → `func(table, {title: "..."})`
/// `func({values: [...], bins: 8})` → `func([...], {bins: 8})`
fn normalize_plot_args(args: Vec<Value>) -> Vec<Value> {
    if args.len() == 1 {
        if let Value::Record(ref map) = args[0] {
            // Try "data" first, then "values" as the primary data key
            for key in &["data", "values"] {
                if let Some(data) = map.get(*key) {
                    let mut opts = map.as_ref().clone();
                    opts.remove(*key);
                    if opts.is_empty() {
                        return vec![data.clone()];
                    }
                    return vec![data.clone(), Value::Record(opts.into())];
                }
            }
        }
    }
    args
}

pub fn call_plot_builtin(name: &str, args: Vec<Value>) -> Result<Value> {
    // A plot specification deliberately contains a `data` table. It is the
    // object render_plot() consumes, not the single-record convenience form
    // that normalize_plot_args() expands for ordinary plotting calls.
    let args = if name == "render_plot" {
        args
    } else {
        normalize_plot_args(args)
    };
    match name {
        "plot" => builtin_plot(args),
        "plot_spec" => builtin_plot_spec(args),
        "render_plot" => builtin_render_plot(args),
        "plot_grid" => builtin_plot_grid(args),
        "heatmap" => builtin_heatmap(args),
        "mosaic_plot" => builtin_mosaic_plot(args),
        "mosaic_data" => builtin_mosaic_data(args),
        "histogram" => builtin_histogram(args),
        "histogram_data" => builtin_histogram_data(args),
        "boxplot_data" => builtin_boxplot_data(args),
        "ecdf_data" => builtin_ecdf_data(args),
        "normal_qq_data" => builtin_normal_qq_data(args),
        "violin_data" => builtin_violin_data(args),
        "linear_fit_data" => builtin_linear_fit_data(args),
        "categorical_data" => builtin_categorical_data(args),
        "missingness_data" => builtin_missingness_data(args),
        "ecdf_plot" => builtin_ecdf_plot(args),
        "density_plot" => builtin_density_plot(args),
        "volcano" => builtin_volcano(args),
        "ma_plot" => builtin_ma_plot(args),
        "save_svg" | "save_plot" => builtin_save_svg(args),
        "save_png" => builtin_save_png(args),
        "genome_track" => builtin_genome_track(args),
        _ => Err(BioLangError::runtime(
            ErrorKind::NameError,
            format!("unknown plot builtin '{name}'"),
            None,
        )),
    }
}

// ── SVG Infrastructure ──────────────────────────────────────────

pub(crate) fn get_opt_str<'a>(
    opts: &'a HashMap<String, Value>,
    key: &str,
    default: &'a str,
) -> &'a str {
    opts.get(key).and_then(|v| v.as_str()).unwrap_or(default)
}

/// The label for an axis: the caller's `xlabel`/`ylabel` when they gave one,
/// otherwise whatever the plot worked out for itself.
///
/// The worked-out defaults ("Value", "Count", a column name) are fine for a
/// quick look at data and useless in a figure meant to teach or to publish,
/// where the whole job of an axis is to say what was measured and in what unit.
/// Every plot here that draws an axis honours these two options.
pub(crate) fn axis_label(opts: &HashMap<String, Value>, key: &str, default: &str) -> String {
    opts.get(key)
        .and_then(|value| value.as_str())
        .unwrap_or(default)
        .to_string()
}

pub(crate) fn get_opt_f64(opts: &HashMap<String, Value>, key: &str, default: f64) -> f64 {
    opts.get(key).and_then(|v| v.as_float()).unwrap_or(default)
}

pub(crate) fn parse_options(args: &[Value]) -> HashMap<String, Value> {
    if args.len() > 1 {
        if let Value::Record(map) = &args[1] {
            return (map).as_ref().clone();
        }
    }
    HashMap::new()
}

pub(crate) fn extract_table_col(table: &Table, col: &str) -> Result<Vec<f64>> {
    let idx = table.col_index(col).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{col}' not found"),
            None,
        )
    })?;
    let mut vals = Vec::with_capacity(table.num_rows());
    for row in &table.rows {
        match &row[idx] {
            Value::Int(n) => vals.push(*n as f64),
            Value::Float(f) => vals.push(*f),
            Value::Str(s) => vals.push(s.parse::<f64>().unwrap_or(f64::NAN)),
            _ => vals.push(f64::NAN),
        }
    }
    Ok(vals)
}

/// A column as the text a reader would see, for labelling a category axis.
fn column_labels(table: &Table, col: &str) -> Vec<String> {
    let Some(idx) = table.col_index(col) else {
        return Vec::new();
    };
    table
        .rows
        .iter()
        .map(|row| match &row[idx] {
            Value::Str(s) => s.to_string(),
            Value::Int(n) => n.to_string(),
            Value::Float(f) => format!("{f}"),
            other => format!("{other:?}"),
        })
        .collect()
}

pub(crate) fn col_range(vals: &[f64]) -> (f64, f64) {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &v in vals {
        if v.is_finite() {
            if v < min {
                min = v;
            }
            if v > max {
                max = v;
            }
        }
    }
    if min > max {
        (0.0, 1.0)
    } else {
        (min, max)
    }
}

pub(crate) fn require_table<'a>(val: &'a Value, func: &str) -> Result<&'a Table> {
    match val {
        Value::Table(t) => Ok(t),
        other => Err(BioLangError::type_error(
            format!("{func}() requires Table, got {}", other.type_of()),
            None,
        )),
    }
}

// ── Builtins ────────────────────────────────────────────────────

/// The y columns a plot draws: one, or several when `y` is given a list.
///
/// Several series on one pair of axes is the case that every hand-drawn figure
/// in the statistics book needed and no builtin could express. Drawing them
/// separately and placing them side by side is not the same picture: each panel
/// gets its own scale, so the comparison the figure exists to make is the one
/// thing it cannot show.
fn series_columns(opts: &HashMap<String, Value>, fallback: &str) -> Vec<String> {
    let named: Vec<String> = match opts.get("y") {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect(),
        Some(value) => value.as_str().map(str::to_string).into_iter().collect(),
        None => Vec::new(),
    };
    if named.is_empty() {
        vec![fallback.to_string()]
    } else {
        named
    }
}

fn is_plot_grid_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA)
            && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "plot-grid"))
}

fn plot_grid_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let Value::List(items) = value else {
        return Err(BioLangError::type_error(
            format!(
                "plot_grid() requires a List of plots, got {}",
                value.type_of()
            ),
            None,
        ));
    };
    if items.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() requires at least one plot",
            None,
        ));
    }
    let columns = get_opt_f64(opts, "columns", (items.len() as f64).sqrt().ceil()) as usize;
    if columns == 0 || columns > items.len().max(1) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() columns must be an integer between 1 and the panel count",
            None,
        ));
    }
    if get_opt_f64(opts, "columns", columns as f64).fract() != 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() columns must be an integer",
            None,
        ));
    }
    let rows = items.len().div_ceil(columns);
    let gap = get_opt_f64(opts, "gap", 18.0);
    let panel_width = get_opt_f64(opts, "panel_width", 420.0);
    let panel_height = get_opt_f64(opts, "panel_height", 330.0);
    let title = get_opt_str(opts, "title", "");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let shared_xlabel = get_opt_str(opts, "shared_xlabel", "");
    let shared_ylabel = get_opt_str(opts, "shared_ylabel", "");
    let header = if title.is_empty() {
        16.0
    } else if subtitle.is_empty() {
        46.0
    } else {
        64.0
    };
    let footer = 14.0
        + if shared_xlabel.is_empty() { 0.0 } else { 24.0 }
        + if caption.is_empty() { 0.0 } else { 18.0 };
    let legend_width = if opts.contains_key("legend") {
        140.0
    } else {
        0.0
    };
    let calculated_width = 20.0
        + columns as f64 * panel_width
        + columns.saturating_sub(1) as f64 * gap
        + legend_width
        + 20.0;
    let calculated_height =
        header + rows as f64 * panel_height + rows.saturating_sub(1) as f64 * gap + footer;
    let width = get_opt_f64(opts, "width", calculated_width);
    let height = get_opt_f64(opts, "height", calculated_height);
    if ![gap, panel_width, panel_height, width, height]
        .iter()
        .all(|value| value.is_finite() && *value > 0.0)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() dimensions must be finite and positive",
            None,
        ));
    }
    let labels = match opts.get("panel_labels") {
        Some(Value::List(labels)) if labels.len() == items.len() => labels
            .iter()
            .map(|label| {
                label.as_str().map(str::to_string).ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "plot_grid() panel_labels must contain strings",
                        None,
                    )
                })
            })
            .collect::<Result<Vec<_>>>()?,
        Some(Value::List(_)) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "plot_grid() panel_labels length must equal the panel count",
                None,
            ))
        }
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "plot_grid() panel_labels must be a List",
                None,
            ))
        }
        None => (0..items.len()).map(spreadsheet_panel_tag).collect(),
    };
    let mut panel_rows = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let mut svg = match item {
            Value::Str(svg) => svg.to_string(),
            Value::Record(_) => match builtin_render_plot(vec![item.clone()])? {
                Value::Str(svg) => svg.to_string(),
                other => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "plot_grid() PlotSpec rendered as {}, expected SVG",
                            other.type_of()
                        ),
                        None,
                    ))
                }
            },
            other => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "plot_grid() panel {} is {}, expected SVG or PlotSpec",
                        index + 1,
                        other.type_of()
                    ),
                    None,
                ))
            }
        };
        safe_nested_svg(&svg)?;
        if !shared_xlabel.is_empty() {
            svg = without_child_axis_title(&svg, "x");
        }
        if !shared_ylabel.is_empty() {
            svg = without_child_axis_title(&svg, "y");
        }
        let (source_width, source_height) = svg_dimensions(&svg)?;
        let row = index / columns;
        let column = index % columns;
        panel_rows.push(vec![
            Value::Int(index as i64),
            Value::Int(row as i64),
            Value::Int(column as i64),
            Value::Str(labels[index].clone().into()),
            Value::Float(20.0 + column as f64 * (panel_width + gap)),
            Value::Float(header + row as f64 * (panel_height + gap)),
            Value::Float(panel_width),
            Value::Float(panel_height),
            Value::Float(source_width),
            Value::Float(source_height),
            Value::Str(svg.into()),
        ]);
    }
    let legend = match opts.get("legend") {
        None => Table::new(vec!["label".into(), "color".into()], Vec::new()),
        Some(Value::Table(table)) => {
            let label = table.col_index("label").ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "plot_grid() legend needs label and color columns",
                    None,
                )
            })?;
            let color = table
                .col_index("color")
                .or_else(|| table.col_index("colour"))
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "plot_grid() legend needs label and color columns",
                        None,
                    )
                })?;
            let mut rows = Vec::new();
            for row in &table.rows {
                let label = row[label].as_str().ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "plot_grid() legend labels must be strings",
                        None,
                    )
                })?;
                let color = row[color]
                    .as_str()
                    .filter(|color| valid_spec_colour(color))
                    .ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "plot_grid() legend colors must be #rrggbb",
                            None,
                        )
                    })?;
                rows.push(vec![Value::Str(label.into()), Value::Str(color.into())]);
            }
            Table::new(vec!["label".into(), "color".into()], rows)
        }
        Some(_) => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "plot_grid() legend must be a Table with label and color columns",
                None,
            ))
        }
    };
    Ok(Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str("figure-composition".into())),
            ("plot".into(), Value::Str("plot-grid".into())),
            ("title".into(), Value::Str(title.into())),
            ("subtitle".into(), Value::Str(subtitle.into())),
            ("caption".into(), Value::Str(caption.into())),
            ("shared_xlabel".into(), Value::Str(shared_xlabel.into())),
            ("shared_ylabel".into(), Value::Str(shared_ylabel.into())),
            (
                "panels".into(),
                Value::Table(Table::new(
                    vec![
                        "panel_index".into(),
                        "row".into(),
                        "column".into(),
                        "tag".into(),
                        "x".into(),
                        "y".into(),
                        "width".into(),
                        "height".into(),
                        "source_width".into(),
                        "source_height".into(),
                        "svg".into(),
                    ],
                    panel_rows,
                )),
            ),
            ("legend".into(), Value::Table(legend)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("width".into(), Value::Float(width)),
                        ("height".into(), Value::Float(height)),
                        ("columns".into(), Value::Int(columns as i64)),
                        ("rows".into(), Value::Int(rows as i64)),
                        ("gap".into(), Value::Float(gap)),
                        ("panel_width".into(), Value::Float(panel_width)),
                        ("panel_height".into(), Value::Float(panel_height)),
                        ("header".into(), Value::Float(header)),
                        ("footer".into(), Value::Float(footer)),
                        ("legend_width".into(), Value::Float(legend_width)),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "publication").into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("panel_count".into(), Value::Int(items.len() as i64)),
                        ("layout".into(), Value::Str("equal_cells".into())),
                        ("child_svg_frozen".into(), Value::Bool(true)),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

fn render_plot_grid_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let Value::Record(map) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a plot-grid PlotSpec",
            None,
        ));
    };
    if !is_plot_grid_spec(value) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a biolang.plot.spec/v1 plot-grid Record",
            None,
        ));
    }
    let panels = match map.get("panels") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panels must be a Table",
                None,
            ))
        }
    };
    let legend = match map.get("legend") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid legend must be a Table",
                None,
            ))
        }
    };
    let options = match map.get("options") {
        Some(Value::Record(options)) => options,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid options must be a Record",
                None,
            ))
        }
    };
    for column in [
        "panel_index",
        "row",
        "column",
        "tag",
        "x",
        "y",
        "width",
        "height",
        "source_width",
        "source_height",
        "svg",
    ] {
        if panels.col_index(column).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() plot-grid panels are missing '{column}'"),
                None,
            ));
        }
    }
    let width = options
        .get("width")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid width is invalid",
                None,
            )
        })?;
    let height = options
        .get("height")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite() && *value > 0.0)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid height is invalid",
                None,
            )
        })?;
    let theme = PlotTheme::from_name(
        options
            .get("theme")
            .and_then(Value::as_str)
            .unwrap_or("publication"),
    );
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    let index_column = panels.col_index("panel_index").unwrap();
    let x_column = panels.col_index("x").unwrap();
    let y_column = panels.col_index("y").unwrap();
    let width_column = panels.col_index("width").unwrap();
    let height_column = panels.col_index("height").unwrap();
    let source_width_column = panels.col_index("source_width").unwrap();
    let source_height_column = panels.col_index("source_height").unwrap();
    let tag_column = panels.col_index("tag").unwrap();
    let svg_column = panels.col_index("svg").unwrap();
    for (index, row) in panels.rows.iter().enumerate() {
        if row[index_column].as_float() != Some(index as f64) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panel indexes are inconsistent",
                None,
            ));
        }
        let values = [
            x_column,
            y_column,
            width_column,
            height_column,
            source_width_column,
            source_height_column,
        ]
        .map(|column| row[column].as_float().unwrap_or(f64::NAN));
        if values
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
            || values[0] + values[2] > width + 1e-8
            || values[1] + values[3] > height + 1e-8
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panel geometry is inconsistent",
                None,
            ));
        }
        let svg = row[svg_column].as_str().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid panel SVG must be a string",
                None,
            )
        })?;
        safe_nested_svg(svg)?;
        let measured = svg_dimensions(svg)?;
        if (measured.0 - values[4]).abs() > 1e-8 || (measured.1 - values[5]).abs() > 1e-8 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid source dimensions were altered",
                None,
            ));
        }
        canvas.elements.push(format!(
            r#"<svg x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" viewBox="0 0 {:.2} {:.2}" preserveAspectRatio="xMidYMid meet" data-panel-index="{index}">{svg}</svg>"#,
            values[0], values[1], values[2], values[3], values[4], values[5]
        ));
        let tag = row[tag_column].as_str().unwrap_or("");
        canvas.add_text_styled(
            values[0] + 6.0,
            values[1] + 18.0,
            tag,
            "start",
            15.0,
            "bold",
            theme.text_colour,
        );
    }
    let title = map.get("title").and_then(Value::as_str).unwrap_or("");
    let subtitle = map.get("subtitle").and_then(Value::as_str).unwrap_or("");
    let caption = map.get("caption").and_then(Value::as_str).unwrap_or("");
    let shared_xlabel = map
        .get("shared_xlabel")
        .and_then(Value::as_str)
        .unwrap_or("");
    let shared_ylabel = map
        .get("shared_ylabel")
        .and_then(Value::as_str)
        .unwrap_or("");
    canvas.margin.left = 20.0;
    canvas.margin.right = 20.0;
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    if !shared_xlabel.is_empty() {
        canvas.add_text(
            width / 2.0,
            height - if caption.is_empty() { 8.0 } else { 22.0 },
            shared_xlabel,
            "middle",
            theme.axis_title_size,
        );
    }
    if !shared_ylabel.is_empty() {
        canvas.add_text_rotated(
            12.0,
            height / 2.0,
            shared_ylabel,
            -90.0,
            "middle",
            theme.axis_title_size,
        );
    }
    if !legend.rows.is_empty() {
        let label_column = legend.col_index("label").ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid legend is missing label",
                None,
            )
        })?;
        let color_column = legend.col_index("color").ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() plot-grid legend is missing color",
                None,
            )
        })?;
        let legend_width = options
            .get("legend_width")
            .and_then(Value::as_float)
            .unwrap_or(140.0);
        let mut y = options
            .get("header")
            .and_then(Value::as_float)
            .unwrap_or(48.0)
            + 12.0;
        let x = width - legend_width + 12.0;
        for row in &legend.rows {
            let label = row[label_column].as_str().unwrap_or("");
            let color = row[color_column]
                .as_str()
                .filter(|color| valid_spec_colour(color))
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "render_plot() plot-grid legend color is invalid",
                        None,
                    )
                })?;
            canvas.add_rect(x, y - 9.0, 10.0, 10.0, color);
            canvas.add_text(x + 15.0, y, label, "start", theme.legend_size);
            y += 17.0;
        }
    }
    canvas.set_accessible_description(format!(
        "Multi-panel BioLang figure containing {} panels and {} shared legend entries.",
        panels.rows.len(),
        legend.rows.len()
    ));
    let svg = canvas.render();
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    match format.as_str() {
        "spec" | "data" => Ok(value.clone()),
        "svg" | "raw" => Ok(Value::Str(svg.into())),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, title).into())),
        #[cfg(feature = "native")]
        "ascii" => render_svg_terminal(&svg, 100, 32, TerminalPlotStyle::Ascii)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => render_svg_terminal(&svg, 100, 32, TerminalPlotStyle::Braille)
            .map(Value::Str)
            .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal plot-grid output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() unknown plot-grid format '{format}'"),
            None,
        )),
    }
}

fn builtin_plot_grid(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let spec = plot_grid_spec_value(&args[0], &opts)?;
    render_plot_grid_spec_value(&spec, &opts)
}

fn render_plot_spec_value(
    spec: &CartesianPlotSpec,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let format = get_opt_str(opts, "format", "svg").to_ascii_lowercase();
    if format == "spec" || format == "data" {
        return Ok(plot_spec_to_value(spec));
    }
    // Counted across every series: they share the plot area, so they share the
    // one raster, and the threshold is about how many marks land in it.
    let point_count = spec
        .series
        .iter()
        .map(|series| series.points.len())
        .sum::<usize>();
    let raster = raster_choice(opts, "plot", point_count)?;
    let svg = render_cartesian_plot_spec(spec, raster)?;
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg.into())),
        // The terminal preview rasterises through resvg, which the browser
        // build deliberately leaves out. Guard the arms rather than the
        // function so a WASM caller asking for one gets a real message.
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
            format!(
                "render_plot() format '{format}' needs the native build; this runtime can emit svg/html/spec"
            ),
            None,
        )),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, &spec.title).into())),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

fn builtin_plot_spec(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let table = require_table(&args[0], "plot_spec")?;
    Ok(plot_spec_to_value(&build_cartesian_plot_spec(
        table,
        &opts,
        "plot_spec",
    )?))
}

fn builtin_mosaic_data(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    crate::mosaic_plot::specification(require_table(&args[0], "mosaic_data")?, &opts)
}

fn builtin_mosaic_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification =
        crate::mosaic_plot::specification(require_table(&args[0], "mosaic_plot")?, &opts)?;
    if matches!(
        get_opt_str(&opts, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        Ok(specification)
    } else {
        crate::mosaic_plot::render(&specification, &opts)
    }
}

fn builtin_render_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let map = match &args[0] {
        Value::Record(map) => map,
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "render_plot() requires plot specification Record, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    if !matches!(map.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() requires schema '{PLOT_SPEC_SCHEMA}'"),
            None,
        ));
    }
    let kind = map.get("kind").and_then(Value::as_str).unwrap_or("");
    let plot = map.get("plot").and_then(Value::as_str).unwrap_or("");
    match (kind, plot) {
        (_, "plot-grid") => render_plot_grid_spec_value(&args[0], &opts),
        ("manhattan", _) => crate::bio_plots::render_manhattan_plot_spec_value(&args[0], &opts),
        ("genetic_qq", _) => crate::bio_plots::render_genetic_qq_plot_spec_value(&args[0], &opts),
        ("rainfall", _) => crate::bio_plots::render_rainfall_plot_spec_value(&args[0], &opts),
        ("ideogram", _) => crate::bio_plots::render_ideogram_plot_spec_value(&args[0], &opts),
        ("cnv", _) => crate::bio_plots::render_cnv_plot_spec_value(&args[0], &opts),
        ("coverage_track", _) => {
            crate::bio_plots::render_coverage_track_plot_spec_value(&args[0], &opts)
        }
        ("genome_track", _) => {
            crate::bio_plots::render_genome_track_plot_spec_value(&args[0], &opts)
        }
        ("lollipop", _) => crate::bio_plots::render_lollipop_plot_spec_value(&args[0], &opts),
        ("sashimi", _) => crate::bio_plots::render_sashimi_plot_spec_value(&args[0], &opts),
        (_, "circos") => crate::bio_plots::render_circos_plot_spec_value(&args[0], &opts),
        ("survival", _) => crate::bio_plots::render_survival_plot_spec_value(&args[0], &opts),
        ("forest", _) => crate::bio_plots::render_forest_plot_spec_value(&args[0], &opts),
        ("roc", _) => crate::bio_plots::render_roc_plot_spec_value(&args[0], &opts),
        ("heatmap", "clustered_heatmap") => {
            crate::bio_plots::render_clustered_heatmap_spec_value(&args[0], &opts)
        }
        ("heatmap", _) => render_heatmap_plot_spec_value(&args[0], &opts),
        ("mosaic", _) => crate::mosaic_plot::render(&args[0], &opts),
        ("violin", _) => crate::bio_plots::render_violin_plot_spec_value(&args[0], &opts),
        ("dot_plot", _) => crate::bio_plots::render_dot_plot_spec_value(&args[0], &opts),
        ("embedding", _) => crate::bio_plots::render_embedding_plot_spec_value(&args[0], &opts),
        ("pca", _) => crate::bio_plots::render_pca_plot_spec_value(&args[0], &opts),
        ("differential_expression", _) => render_differential_plot_spec_value(&args[0], &opts),
        ("scatter" | "line" | "errorbar" | "confidence", _) => {
            let spec = plot_spec_from_value(&args[0])?;
            render_plot_spec_value(&spec, &opts)
        }
        ("", _) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() specification field 'kind' must be Str",
            None,
        )),
        (unknown, _) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() unknown plot kind '{unknown}'"),
            None,
        )),
    }
}

fn builtin_plot(args: Vec<Value>) -> Result<Value> {
    // Handle Record with x/y lists: plot({x: [...], y: [...], title: "..."})
    let args = if args.len() == 1 {
        if let Value::Record(ref map) = args[0] {
            if map.contains_key("x") && map.contains_key("y") {
                if let (Value::List(xv), Value::List(yv)) = (&map["x"], &map["y"]) {
                    let rows: Vec<Vec<Value>> = xv
                        .iter()
                        .zip(yv.iter())
                        .map(|(x, y)| vec![x.clone(), y.clone()])
                        .collect();
                    let table = Value::Table(Table::new(vec!["x".into(), "y".into()], rows));
                    let mut opts = map.as_ref().clone();
                    opts.remove("x");
                    opts.remove("y");
                    if opts.is_empty() {
                        vec![table]
                    } else {
                        vec![table, Value::Record(opts.into())]
                    }
                } else {
                    args
                }
            } else {
                args
            }
        } else {
            args
        }
    } else {
        args
    };

    let opts = parse_options(&args);
    let plot_type = get_opt_str(&opts, "type", "scatter").to_string();
    let width = get_opt_f64(&opts, "width", 800.0);
    let height = get_opt_f64(&opts, "height", 600.0);
    let title = get_opt_str(&opts, "title", "").to_string();

    let table = require_table(&args[0], "plot")?;

    if table.num_cols() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot() requires table with at least 2 columns",
            None,
        ));
    }

    // These plot families now have one renderer-neutral specification. The
    // SVG, terminal preview and standalone HTML/canvas fallback all originate
    // from this object; none of those display paths recomputes statistics.
    if matches!(
        plot_type.to_ascii_lowercase().as_str(),
        "scatter" | "line" | "errorbar" | "confidence"
    ) {
        let spec = build_cartesian_plot_spec(table, &opts, "plot")?;
        return render_plot_spec_value(&spec, &opts);
    }

    let x_col = get_opt_str(&opts, "x", &table.columns[0]).to_string();
    let y_cols = series_columns(&opts, &table.columns[1]);

    let xs = extract_table_col(table, &x_col)?;
    let mut series: Vec<Vec<f64>> = Vec::with_capacity(y_cols.len());
    for column in &y_cols {
        series.push(extract_table_col(table, column)?);
    }

    let (x_min, x_max) = col_range(&xs);
    // One vertical scale across every series, so the comparison is honest.
    let (mut y_min, mut y_max) = series.iter().fold((f64::MAX, f64::MIN), |(lo, hi), ys| {
        let (series_lo, series_hi) = col_range(ys);
        (lo.min(series_lo), hi.max(series_hi))
    });
    if plot_type == "box" {
        // Every numeric column becomes a group. The former scale came only
        // from the default y column, so a wider first or later column could be
        // clipped even though its geometry was still drawn.
        (y_min, y_max) = table.columns.iter().try_fold(
            (f64::INFINITY, f64::NEG_INFINITY),
            |(low, high), column| {
                let values = extract_table_col(table, column)?;
                let (column_low, column_high) = col_range(&values);
                Ok::<_, BioLangError>((low.min(column_low), high.max(column_high)))
            },
        )?;
    }
    if plot_type == "bar" {
        // A bar says "this much", and the reader takes its length as the
        // quantity. Starting the axis at the smallest value instead of zero
        // draws counts of 100 and 104 as a bar of nothing beside a full-height
        // one -- the best-known way to mislead with a chart, and it was the
        // default here. A bar chart's axis includes zero.
        y_min = y_min.min(0.0);
        y_max = y_max.max(0.0);
    }

    let mut canvas = SvgCanvas::new(width, height);
    // No x_scale here: bar positions come from the category layout below and
    // box positions from the column index, so only the vertical scale is
    // shared. The scatter and line arms that needed one now build their own
    // spec and return above.
    let y_scale = Scale {
        domain: (y_min, y_max),
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };

    match plot_type.as_str() {
        // scatter, line, errorbar and confidence never reach this match: they
        // are built as a CartesianPlotSpec and returned above. Only the
        // families that have no spec yet are rendered directly here.
        "bar" => {
            // Grouped: one cluster per x position, one bar per series inside it.
            let group_w = canvas.plot_width() / xs.len() as f64;
            let bar_w = group_w * 0.8 / series.len() as f64;
            let baseline = y_scale.map(0.0f64.max(y_min));
            for (index, ys) in series.iter().enumerate() {
                // A single series keeps its old per-bar colouring, which is what
                // a bar chart of categories wants; several series colour by
                // series instead, because that is what the legend names.
                for (i, &y) in ys.iter().enumerate() {
                    if !y.is_finite() {
                        continue;
                    }
                    let bx = canvas.margin.left
                        + group_w * i as f64
                        + group_w * 0.1
                        + bar_w * index as f64;
                    let by = y_scale.map(y);
                    let bh = (baseline - by).abs();
                    let top = by.min(baseline);
                    let colour = if series.len() == 1 {
                        PALETTE[i % PALETTE.len()]
                    } else {
                        PALETTE[index % PALETTE.len()]
                    };
                    canvas.add_rect(bx, top, bar_w, bh, colour);
                }
            }
            draw_legend(&mut canvas, &y_cols);
        }
        "box" => {
            // Box plot per numeric column
            for (ci, col) in table.columns.iter().enumerate() {
                let vals = extract_table_col(table, col)?;
                if !vals.iter().any(|value| value.is_finite()) {
                    continue;
                }
                // The renderer consumes the same inspectable geometry exposed
                // by boxplot_data(), including its type-7 quartiles and Tukey
                // whisker coefficient. No summary statistic is recalculated in
                // screen coordinates.
                let geometry = box_geometry(col, &vals, "type7", 1.5);

                let bx = canvas.margin.left
                    + (ci as f64 + 0.2) * canvas.plot_width() / table.num_cols() as f64;
                let bw = canvas.plot_width() / table.num_cols() as f64 * 0.6;

                canvas.add_rect(
                    bx,
                    y_scale.map(geometry.q3),
                    bw,
                    (y_scale.map(geometry.q1) - y_scale.map(geometry.q3)).abs(),
                    PALETTE[ci % PALETTE.len()],
                );
                canvas.add_line(
                    bx,
                    y_scale.map(geometry.median),
                    bx + bw,
                    y_scale.map(geometry.median),
                    "#333",
                    2.0,
                );
                canvas.add_line(
                    bx + bw / 2.0,
                    y_scale.map(geometry.q3),
                    bx + bw / 2.0,
                    y_scale.map(geometry.whisker_high),
                    "#333",
                    1.0,
                );
                canvas.add_line(
                    bx + bw / 2.0,
                    y_scale.map(geometry.q1),
                    bx + bw / 2.0,
                    y_scale.map(geometry.whisker_low),
                    "#333",
                    1.0,
                );
                for (_, value) in &geometry.outliers {
                    canvas.add_circle(bx + bw / 2.0, y_scale.map(*value), 3.0, "#333");
                }
            }
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("plot() unknown type '{plot_type}', expected scatter/line/bar/box"),
                None,
            ));
        }
    }

    let d_x_scale = Scale {
        domain: (x_min, x_max),
        range: (x_min, x_max),
    };
    let d_y_scale = Scale {
        domain: (y_min, y_max),
        range: (y_min, y_max),
    };
    // With several series the legend names them, so the default y label would
    // be one column's name standing for all of them. Better to say nothing.
    let default_ylabel = if y_cols.len() == 1 { &y_cols[0] } else { "" };
    if plot_type == "bar" {
        canvas.draw_category_axis(
            &column_labels(table, &x_col),
            &axis_label(&opts, "xlabel", &x_col),
        );
    } else {
        canvas.draw_x_axis(&d_x_scale, &axis_label(&opts, "xlabel", &x_col));
    }
    canvas.draw_y_axis(&d_y_scale, &axis_label(&opts, "ylabel", default_ylabel));
    if !title.is_empty() {
        canvas.draw_title(&title);
    }

    Ok(Value::Str(canvas.render()))
}

// ── Heatmap color schemes ──────────────────────────────────────

fn builtin_save_svg(args: Vec<Value>) -> Result<Value> {
    let svg = match &args[0] {
        Value::Str(s) => s.to_string(),
        Value::Record(_) => match builtin_render_plot(vec![args[0].clone()])? {
            Value::Str(svg) if svg.trim_start().starts_with("<svg") => svg.to_string(),
            other => {
                return Err(BioLangError::type_error(
                    format!(
                        "save_svg() rendered PlotSpec as {}, expected SVG",
                        other.type_of()
                    ),
                    None,
                ))
            }
        },
        Value::Nil => return Err(BioLangError::type_error(
            "save_svg()/save_plot() received Nil — the plot function before the pipe likely failed or returned nothing".to_string(), None,
        )),
        other => return Err(BioLangError::type_error(
            format!("save_svg() requires SVG Str or PlotSpec Record, got {}", other.type_of()), None,
        )),
    };
    let path = match &args[1] {
        Value::Str(s) => s,
        other => {
            return Err(BioLangError::type_error(
                format!("save_svg() requires Str (path), got {}", other.type_of()),
                None,
            ))
        }
    };
    let opts = parse_options(&args[1..]);
    let profile = get_opt_str(&opts, "profile", "screen").to_ascii_lowercase();
    if !matches!(profile.as_str(), "screen" | "publication" | "journal") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "save_svg() profile must be screen or publication",
            None,
        ));
    }
    let mut output = svg;
    let svg_start = output.find("<svg").ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "save_svg() requires an SVG document or PlotSpec",
            None,
        )
    })?;
    if matches!(profile.as_str(), "publication" | "journal") {
        let font = match get_opt_str(&opts, "font", "sans")
            .to_ascii_lowercase()
            .as_str()
        {
            "sans" | "sans-serif" | "arial" | "helvetica" => "Arial,Helvetica,sans-serif",
            "serif" | "times" => "Times New Roman,Times,serif",
            "mono" | "monospace" => "Courier New,monospace",
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "save_svg() publication font must be sans, serif, or mono",
                    None,
                ))
            }
        };
        let metadata = format!(
            "<metadata>BioLang publication figure; vector text; font profile: {}</metadata><style>text{{font-family:{font}}}</style>",
            xml_escape(font)
        );
        let opening = output[svg_start..]
            .find('>')
            .map(|offset| svg_start + offset)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "save_svg() received malformed SVG",
                    None,
                )
            })?;
        output.insert_str(opening + 1, &metadata);
        output.insert_str(svg_start + 4, " data-biolang-export=\"publication\"");
    }
    let width_mm = opts.get("width_mm").and_then(Value::as_float);
    let height_mm = opts.get("height_mm").and_then(Value::as_float);
    match (width_mm, height_mm) {
        (None, None) => {}
        (Some(width_mm), Some(height_mm))
            if width_mm.is_finite()
                && height_mm.is_finite()
                && width_mm > 0.0
                && height_mm > 0.0 =>
        {
            let width_pattern = regex::Regex::new(r#"\bwidth="[^"]+""#).unwrap();
            let height_pattern = regex::Regex::new(r#"\bheight="[^"]+""#).unwrap();
            output = width_pattern
                .replacen(&output, 1, format!("width=\"{width_mm}mm\""))
                .into_owned();
            output = height_pattern
                .replacen(&output, 1, format!("height=\"{height_mm}mm\""))
                .into_owned();
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "save_svg() width_mm and height_mm must be supplied together as positive numbers",
                None,
            ))
        }
    }
    std::fs::write(path, output).map_err(|e| {
        BioLangError::runtime(
            ErrorKind::IOError,
            format!("save_svg() write failed: {e}"),
            None,
        )
    })?;
    Ok(Value::Str(path.clone()))
}

/// Character set used when an SVG plot is previewed in a terminal.
///
/// Braille keeps two-by-four subpixels in each character and is the most useful
/// interactive preview. ASCII is lower resolution but survives restricted
/// terminals and plain-text logs.
#[cfg(feature = "native")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TerminalPlotStyle {
    Braille,
    Ascii,
}

/// Rasterise a complete SVG document into a compact terminal preview.
///
/// This deliberately consumes the SVG that every plot builtin already
/// produces. It therefore cannot disagree with the saved figure about scales,
/// points, labels, or clipping. The result contains no ANSI escapes, so callers
/// may safely colour it or place it in a plain-text log.
#[cfg(feature = "native")]
pub fn render_svg_terminal(
    svg: &str,
    columns: usize,
    max_rows: usize,
    style: TerminalPlotStyle,
) -> std::result::Result<String, String> {
    use resvg::{tiny_skia, usvg};

    let options = usvg::Options {
        fontdb: svg_font_database(),
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options)
        .map_err(|error| format!("could not parse SVG: {error}"))?;
    let size = tree.size();
    let source_width = size.width().max(1.0);
    let source_height = size.height().max(1.0);

    let columns = columns.clamp(12, 160);
    let max_rows = max_rows.clamp(4, 60);
    // One character covers a 2x4 pixel cell in either style: Braille encodes
    // exactly that grid, and matching it for ASCII keeps both previews the same
    // shape at the same requested width.
    let cell_width = 2usize;
    let cell_height = 4usize;
    let target_width = (columns * cell_width) as f32;
    let target_height = (max_rows * cell_height) as f32;
    let scale = (target_width / source_width)
        .min(target_height / source_height)
        .max(0.001);
    let pixel_width = (source_width * scale).ceil().max(1.0) as u32;
    let pixel_height = (source_height * scale).ceil().max(1.0) as u32;
    let mut pixmap = tiny_skia::Pixmap::new(pixel_width, pixel_height)
        .ok_or_else(|| "could not allocate terminal plot raster".to_string())?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    // resvg stores premultiplied RGBA. Composite each channel over white before
    // measuring darkness so an untouched transparent pixel remains blank.
    let ink_at = |x: u32, y: u32| -> f32 {
        let Some(pixel) = pixmap.pixel(x, y) else {
            return 0.0;
        };
        let transparent = 255u16.saturating_sub(pixel.alpha() as u16);
        let red = (pixel.red() as u16 + transparent).min(255) as f32;
        let green = (pixel.green() as u16 + transparent).min(255) as f32;
        let blue = (pixel.blue() as u16 + transparent).min(255) as f32;
        255.0 - (0.2126 * red + 0.7152 * green + 0.0722 * blue)
    };

    let output_columns = (pixel_width as usize).div_ceil(cell_width);
    let output_rows = (pixel_height as usize).div_ceil(cell_height);
    let mut lines = Vec::with_capacity(output_rows);
    for row in 0..output_rows {
        let mut line = String::with_capacity(output_columns);
        for column in 0..output_columns {
            let x0 = column * cell_width;
            let y0 = row * cell_height;
            match style {
                TerminalPlotStyle::Braille => {
                    const DOTS: [[u8; 2]; 4] =
                        [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
                    let mut bits = 0u8;
                    for dy in 0..cell_height {
                        for dx in 0..cell_width {
                            if ink_at((x0 + dx) as u32, (y0 + dy) as u32) >= 28.0 {
                                bits |= DOTS[dy][dx];
                            }
                        }
                    }
                    line.push(char::from_u32(0x2800 + bits as u32).unwrap_or(' '));
                }
                TerminalPlotStyle::Ascii => {
                    const LEVELS: &[u8] = b" .:-=+*#%@";
                    let mut total = 0.0f32;
                    let mut peak = 0.0f32;
                    for dy in 0..cell_height {
                        for dx in 0..cell_width {
                            let ink = ink_at((x0 + dx) as u32, (y0 + dy) as u32);
                            total += ink;
                            peak = peak.max(ink);
                        }
                    }
                    let average = total / (cell_width * cell_height) as f32;
                    // Thin axes and lines would disappear under a pure average;
                    // retain their peak while still giving solid areas weight.
                    let density = (0.65 * peak + 0.35 * average) / 255.0;
                    let index = (density * (LEVELS.len() - 1) as f32).round() as usize;
                    line.push(LEVELS[index.min(LEVELS.len() - 1)] as char);
                }
            }
        }
        lines.push(line.trim_end().to_string());
    }
    while lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return Err("SVG rendered as an empty terminal preview".to_string());
    }
    Ok(lines.join("\n"))
}

fn builtin_genome_track(args: Vec<Value>) -> Result<Value> {
    crate::bio_plots::builtin_genome_track(args)
}

#[cfg(test)]
mod palette_tests {
    use super::{estimate_text_width, PlotTheme, Scale, SvgCanvas, PALETTE};
    #[cfg(feature = "native")]
    use super::{render_svg_terminal, TerminalPlotStyle};
    use std::collections::HashSet;

    // Callers index PALETTE modulo its length, so a plot with more groups than
    // colours draws two groups the same. At eight entries that bit constantly:
    // clustering 2700 PBMCs gave eleven groups and three of them reused colours
    // already spent, on the one figure the analysis is read from.

    #[test]
    fn palette_covers_a_realistic_cluster_count() {
        assert!(
            PALETTE.len() >= 20,
            "single-cell work routinely yields 15-30 clusters; palette has {}",
            PALETTE.len()
        );
    }

    #[cfg(feature = "native")]
    #[test]
    fn terminal_renderers_turn_svg_into_text_without_leaking_markup() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="80"><rect width="120" height="80" fill="white"/><line x1="5" y1="70" x2="115" y2="10" stroke="black" stroke-width="5"/></svg>"#;
        for style in [TerminalPlotStyle::Braille, TerminalPlotStyle::Ascii] {
            let preview = render_svg_terminal(svg, 40, 12, style).expect("terminal preview");
            assert!(!preview.trim().is_empty());
            assert!(!preview.contains("<svg"));
            assert!(preview.lines().count() <= 12);
            assert!(preview.lines().all(|line| line.chars().count() <= 40));
        }
    }

    #[test]
    fn palette_entries_are_distinct() {
        let unique: HashSet<&&str> = PALETTE.iter().collect();
        assert_eq!(
            unique.len(),
            PALETTE.len(),
            "palette repeats a colour, so two groups share one"
        );
    }

    #[test]
    fn axis_ticks_use_readable_one_two_five_steps() {
        let ticks = Scale {
            domain: (0.0, 7.0),
            range: (0.0, 700.0),
        }
        .nice_ticks(5);
        assert_eq!(ticks, vec![0.0, 2.0, 4.0, 6.0]);

        let crossing_zero = Scale {
            domain: (-3.0, 7.0),
            range: (0.0, 700.0),
        }
        .nice_ticks(5);
        assert_eq!(crossing_zero, vec![-2.0, 0.0, 2.0, 4.0, 6.0]);
    }

    #[test]
    fn axis_ticks_handle_small_constant_and_reversed_domains() {
        let small = Scale {
            domain: (0.0012, 0.0019),
            range: (0.0, 1.0),
        }
        .nice_ticks(5);
        assert_eq!(small.len(), 4);
        for (actual, expected) in small.iter().zip([0.0012, 0.0014, 0.0016, 0.0018]) {
            assert!((actual - expected).abs() < 1e-12);
        }
        assert_eq!(
            Scale {
                domain: (3.0, 3.0),
                range: (0.0, 1.0)
            }
            .nice_ticks(5),
            vec![3.0]
        );
        assert_eq!(
            Scale {
                domain: (7.0, 0.0),
                range: (0.0, 1.0)
            }
            .nice_ticks(5),
            vec![6.0, 4.0, 2.0, 0.0]
        );
    }

    #[test]
    fn palette_entries_are_well_formed_hex() {
        for colour in PALETTE {
            assert!(
                colour.len() == 7 && colour.starts_with('#'),
                "malformed colour: {colour}"
            );
            assert!(
                colour[1..].chars().all(|c| c.is_ascii_hexdigit()),
                "non-hex colour: {colour}"
            );
        }
    }

    #[test]
    fn the_original_eight_are_unchanged() {
        // Existing figures keep the colours they had.
        let original = [
            "#4e79a7", "#f28e2b", "#e15759", "#76b7b2", "#59a14f", "#edc948", "#b07aa1", "#ff9da7",
        ];
        assert_eq!(&PALETTE[..8], &original[..]);
    }

    #[test]
    fn rendered_svg_uses_the_title_as_its_accessible_label() {
        let mut canvas = SvgCanvas::new(320.0, 180.0);
        canvas.draw_title("A & B");
        canvas.set_accessible_description("Values < reference & finite");

        let svg = canvas.render();
        assert!(svg.contains("role=\"img\""));
        assert!(svg.contains("focusable=\"false\""));
        assert!(svg.contains("aria-label=\"A &amp; B\""));
        assert!(svg.contains("<title>A &amp; B</title>"));
        assert!(svg.contains("<desc>Values &lt; reference &amp; finite</desc>"));
    }

    #[test]
    fn rendered_svg_has_a_default_accessible_label() {
        let canvas = SvgCanvas::new(320.0, 180.0);
        let svg = canvas.render();
        assert!(svg.contains("aria-label=\"BioLang plot\""));
        assert!(svg.contains("<title>BioLang plot</title>"));
        assert!(svg.contains("<desc>BioLang data visualization.</desc>"));
    }

    #[test]
    fn publication_theme_is_opt_in_and_structurally_identified() {
        let legacy = SvgCanvas::new(320.0, 180.0).render();
        let publication =
            SvgCanvas::with_theme(320.0, 180.0, PlotTheme::from_name("publication")).render();
        assert!(legacy.contains("data-biolang-theme=\"biolang\""));
        assert!(publication.contains("data-biolang-theme=\"publication\""));
        assert!(!legacy.contains("Arial, Helvetica"));
    }

    #[test]
    fn adaptive_layout_reserves_room_for_wide_tick_labels() {
        let theme = PlotTheme::from_name("publication");
        let mut short = SvgCanvas::with_theme(500.0, 320.0, theme);
        short.fit_cartesian_layout(&[0.0, 1.0], &[0.0, 1.0], "x", "y", "t", "", "", 0.0);
        let mut wide = SvgCanvas::with_theme(500.0, 320.0, theme);
        wide.fit_cartesian_layout(
            &[0.0, 1.0],
            &[10_000_000.0, 90_000_000.0],
            "x",
            "y",
            "t",
            "",
            "",
            0.0,
        );
        assert!(wide.margin.left > short.margin.left);
    }

    #[test]
    fn text_measurement_distinguishes_narrow_and_wide_labels() {
        assert!(estimate_text_width("MMMM", 12.0) > estimate_text_width("iiii", 12.0) * 2.0);
    }
}

#[cfg(test)]
mod axis_label_tests {
    use super::{axis_label, call_plot_builtin};
    use bl_core::value::Value;
    use std::collections::HashMap;
    use std::sync::Arc;

    // The defaults ("Value", "Count") are what a histogram draws when nobody
    // says otherwise. They were previously the only thing it could draw: an
    // xlabel in the options record was accepted and silently ignored, so a
    // figure asking for "minutes until the next eruption" got "Value" and no
    // warning. Anything building teaching or publication figures had to
    // string-replace the rendered SVG afterwards.

    fn options(pairs: &[(&str, &str)]) -> HashMap<String, Value> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), Value::Str((*v).into())))
            .collect()
    }

    #[test]
    fn an_explicit_label_replaces_the_default() {
        let opts = options(&[("xlabel", "minutes until the next eruption")]);
        assert_eq!(
            axis_label(&opts, "xlabel", "Value"),
            "minutes until the next eruption"
        );
    }

    #[test]
    fn the_default_is_used_when_no_label_is_given() {
        assert_eq!(axis_label(&HashMap::new(), "xlabel", "Value"), "Value");
    }

    #[test]
    fn a_histogram_renders_the_labels_it_was_given() {
        let values = Value::List(Arc::new(
            (1..=40).map(|n| Value::Float(f64::from(n))).collect(),
        ));
        let mut opts = HashMap::new();
        opts.insert("xlabel".to_string(), Value::Str("waiting (minutes)".into()));
        opts.insert("ylabel".to_string(), Value::Str("eruptions".into()));

        let svg = match call_plot_builtin("histogram", vec![values, Value::Record(Arc::new(opts))])
            .expect("histogram renders")
        {
            Value::Str(svg) => svg.to_string(),
            other => panic!("histogram should return SVG, got {other:?}"),
        };

        assert!(
            svg.contains(">waiting (minutes)<"),
            "x label missing: {svg:.400}"
        );
        assert!(svg.contains(">eruptions<"), "y label missing");
        assert!(
            !svg.contains(">Value<"),
            "the default x label should be gone"
        );
        assert!(
            !svg.contains(">Count<"),
            "the default y label should be gone"
        );
    }

    #[test]
    fn a_histogram_leaves_headroom_above_the_tallest_bar() {
        let values = Value::List(Arc::new(
            [1.0, 1.0, 1.0, 2.0].into_iter().map(Value::Float).collect(),
        ));
        let opts = Value::Record(Arc::new(HashMap::from([
            ("bins".to_string(), Value::Int(2)),
            ("theme".to_string(), Value::Str("ggplot".into())),
        ])));
        let svg =
            match call_plot_builtin("histogram", vec![values, opts]).expect("histogram renders") {
                Value::Str(svg) => svg.to_string(),
                other => panic!("histogram should return SVG, got {other:?}"),
            };
        let fill = svg.find("fill=\"#595959\"").expect("histogram bar");
        let start = svg[..fill].rfind("<rect ").expect("bar rectangle");
        let bar = &svg[start..fill];
        let y = bar
            .split(" y=\"")
            .nth(1)
            .and_then(|value| value.split('"').next())
            .and_then(|value| value.parse::<f64>().ok())
            .expect("bar y coordinate");
        assert!(
            y > 40.0,
            "the tallest bar should sit below the 40px panel top: {bar}"
        );
    }
}

#[cfg(test)]
mod distribution_plot_tests {
    use super::{call_plot_builtin, silverman_bandwidth};
    use bl_core::value::Value;
    use std::collections::HashMap;
    use std::sync::Arc;

    // `ecdf_plot` and `density_plot` are the two ways of showing a distribution
    // that a histogram's bin width cannot distort: the ECDF has no parameter at
    // all, and a density has one that is stated rather than implied by where
    // the bin edges happened to fall.

    fn numbers(values: &[f64]) -> Value {
        Value::List(Arc::new(values.iter().copied().map(Value::Float).collect()))
    }

    fn options(pairs: &[(&str, Value)]) -> Value {
        let mut record = HashMap::new();
        for (key, value) in pairs {
            record.insert((*key).to_string(), value.clone());
        }
        Value::Record(Arc::new(record))
    }

    fn render(name: &str, args: Vec<Value>) -> String {
        match call_plot_builtin(name, args).expect("plot renders") {
            Value::Str(svg) => svg.to_string(),
            other => panic!("{name} should return SVG, got {other:?}"),
        }
    }

    /// Every case here is `bw.nrd0` from R 4.6.1, printed to twelve places. A
    /// density is only comparable with one drawn in R if the default smoothing
    /// agrees, so these are checked against the reference rather than against
    /// whatever the formula currently returns.
    fn assert_matches_r(label: &str, values: &[f64], expected: f64) {
        let got = silverman_bandwidth(values);
        assert!(
            (got - expected).abs() < 1e-11,
            "{label}: bw.nrd0 is {expected}, got {got}"
        );
    }

    #[test]
    fn the_default_bandwidth_is_the_one_r_picks() {
        assert_matches_r(
            "primes",
            &[2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0],
            5.124_406_997_583,
        );
    }

    #[test]
    fn one_far_out_value_does_not_widen_the_bandwidth() {
        // sd is 30.9 here and the IQR is 4.5. Taking the smaller is the whole
        // point of the rule: one outlier must not smooth the other nine values
        // into a single hill. It also pins the divisor -- R uses 1.34, and 1.349
        // (the exact interquartile range of a standard normal, and what the
        // comment in R's own source says) would give 1.894 instead.
        assert_matches_r(
            "heavy tail",
            &[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 100.0],
            1.906_997_944_138,
        );
    }

    #[test]
    fn a_tied_middle_half_falls_back_to_the_standard_deviation() {
        // The IQR is exactly zero here, so the rule's usual estimate is zero --
        // a bandwidth that would make the density a row of infinite spikes.
        assert_matches_r(
            "tied middle",
            &[0.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 5.0, 10.0],
            1.338_462_650_764,
        );
    }

    #[test]
    fn a_constant_column_still_gets_a_positive_bandwidth() {
        // sd and IQR are both zero. Nothing here is worth plotting, but the
        // bandwidth is a divisor and must not be zero. R uses the magnitude of
        // an observation, and then 1 when even that is zero.
        assert_matches_r("constant 7", &[7.0; 6], 4.402_610_848_261);
        assert_matches_r("constant 0", &[0.0; 6], 0.628_944_406_894);
    }

    #[test]
    fn the_ecdf_is_drawn_as_steps_not_as_a_joined_line() {
        // Two segments per observation -- the riser at the value, then the flat
        // run to the next one. Joining the points directly would draw a
        // distribution that is smooth between observations, which is exactly
        // what an ECDF is not.
        let svg = render("ecdf_plot", vec![numbers(&[1.0, 2.0, 3.0, 4.0])]);
        let points = svg
            .split(r#"<polyline points=""#)
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("the step is drawn as a polyline");
        // Two points per observation -- the top of the riser and the end of the
        // flat run -- plus the corner it starts from on the axis.
        assert_eq!(
            points.split(' ').count(),
            9,
            "expected 2 vertices per observation: {points}"
        );
        // Consecutive pairs must share alternately an x then a y: that is what
        // makes it a staircase rather than a line joining the observations.
        let vertices: Vec<(&str, &str)> = points
            .split(' ')
            .map(|point| point.split_once(',').expect("x,y"))
            .collect();
        for pair in vertices.windows(2) {
            assert!(
                pair[0].0 == pair[1].0 || pair[0].1 == pair[1].1,
                "{:?} -> {:?} is a diagonal, not a step",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn the_ecdf_says_what_its_y_axis_means() {
        let svg = render("ecdf_plot", vec![numbers(&[1.0, 2.0, 3.0])]);
        assert!(svg.contains(">Proportion at or below<"), "{svg:.400}");

        let labelled = render(
            "ecdf_plot",
            vec![
                numbers(&[1.0, 2.0, 3.0]),
                options(&[("ylabel", Value::Str("fraction of samples".into()))]),
            ],
        );
        assert!(labelled.contains(">fraction of samples<"));
        assert!(!labelled.contains(">Proportion at or below<"));
    }

    #[test]
    fn an_explicit_bandwidth_changes_the_curve() {
        // The failure this guards against is the one that made every axis label
        // in this file wrong for so long: an option accepted, parsed, and then
        // not used. A bandwidth ten times wider must draw a different picture.
        let values = numbers(&[1.0, 2.0, 3.0, 8.0, 9.0, 10.0]);
        let narrow = render(
            "density_plot",
            vec![values.clone(), options(&[("bandwidth", Value::Float(0.2))])],
        );
        let wide = render(
            "density_plot",
            vec![values, options(&[("bandwidth", Value::Float(2.0))])],
        );
        assert_ne!(narrow, wide, "the bandwidth option was ignored");
    }

    #[test]
    fn a_list_with_no_numbers_in_it_is_an_error_rather_than_an_empty_plot() {
        let text = Value::List(Arc::new(vec![Value::Str("setosa".into())]));
        let error = call_plot_builtin("density_plot", vec![text])
            .expect_err("a column of species names is not a distribution");
        assert!(
            error.to_string().contains("no numeric values"),
            "unhelpful message: {error}"
        );
    }

    #[test]
    fn a_numeric_column_read_from_a_csv_still_plots() {
        // Columns arrive as text often enough that rejecting them would be the
        // wrong default -- this is what a `col()` on an unparsed CSV gives you.
        let as_text = Value::List(Arc::new(
            ["1.5", "2.5", "3.5", "4.5"]
                .iter()
                .map(|s| Value::Str((*s).into()))
                .collect(),
        ));
        assert!(render("ecdf_plot", vec![as_text]).starts_with("<svg"));
    }
}

#[cfg(test)]
mod series_and_box_tests {
    use super::{call_plot_builtin, quantile_type7};
    use bl_core::value::{Table, Value};
    use std::collections::HashMap;
    use std::sync::Arc;

    // Two things a plot could not do: draw more than one series on one pair of
    // axes, and agree with `quantile()` about where the quartiles are.

    fn table(columns: &[&str], rows: &[&[f64]]) -> Value {
        Value::Table(Table::new(
            columns.iter().map(|c| (*c).to_string()).collect(),
            rows.iter()
                .map(|row| row.iter().copied().map(Value::Float).collect())
                .collect(),
        ))
    }

    fn options(pairs: &[(&str, Value)]) -> Value {
        let mut record = HashMap::new();
        for (key, value) in pairs {
            record.insert((*key).to_string(), value.clone());
        }
        Value::Record(Arc::new(record))
    }

    fn strings(names: &[&str]) -> Value {
        Value::List(Arc::new(
            names.iter().map(|n| Value::Str((*n).into())).collect(),
        ))
    }

    fn render(args: Vec<Value>) -> String {
        match call_plot_builtin("plot", args).expect("plot renders") {
            Value::Str(svg) => svg.to_string(),
            other => panic!("plot should return SVG, got {other:?}"),
        }
    }

    fn wide() -> Value {
        table(
            &["x", "small", "large"],
            &[
                &[1.0, 1.0, 10.0],
                &[2.0, 2.0, 20.0],
                &[3.0, 3.0, 30.0],
                &[4.0, 4.0, 40.0],
            ],
        )
    }

    fn line_options(columns: &[&str]) -> Value {
        options(&[("type", Value::Str("line".into())), ("y", strings(columns))])
    }

    #[test]
    fn a_list_of_y_columns_draws_one_line_each() {
        let svg = render(vec![wide(), line_options(&["small", "large"])]);
        assert_eq!(
            svg.matches("<polyline").count(),
            2,
            "expected one polyline per series: {svg:.600}"
        );
    }

    #[test]
    fn the_series_share_one_vertical_scale() {
        // The reason to draw them together rather than side by side. If each
        // series were scaled to itself, "small" would occupy the same height
        // alone as it does beside a series ten times larger.
        let alone = render(vec![wide(), line_options(&["small"])]);
        let together = render(vec![wide(), line_options(&["small", "large"])]);
        let first_line = |svg: &str| -> String {
            svg.split(r#"<polyline points=""#)
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .expect("a polyline")
                .to_string()
        };
        assert_ne!(
            first_line(&alone),
            first_line(&together),
            "the small series was not rescaled to the shared range"
        );
    }

    #[test]
    fn a_legend_names_the_series_and_only_when_there_are_several() {
        let several = render(vec![wide(), line_options(&["small", "large"])]);
        assert!(several.contains(">small<"), "{several:.600}");
        assert!(several.contains(">large<"));
        // Once each, in the legend. Letting the y axis default to one of the
        // column names would label the whole scale after half the data.
        assert_eq!(several.matches(">small<").count(), 1, "{several:.600}");
        assert_eq!(several.matches(">large<").count(), 1);

        // One series is named by the y axis, so a legend would only repeat it.
        let single = render(vec![wide(), line_options(&["small"])]);
        assert_eq!(single.matches(">small<").count(), 1, "{single:.600}");
    }

    #[test]
    fn a_plain_string_y_still_selects_one_column() {
        let svg = render(vec![
            wide(),
            options(&[
                ("type", Value::Str("line".into())),
                ("y", Value::Str("large".into())),
            ]),
        ]);
        assert_eq!(svg.matches("<polyline").count(), 1);
        assert!(svg.contains(">large<"), "the y axis should name the column");
    }

    #[test]
    fn grouped_bars_draw_one_bar_per_series_per_category() {
        let svg = render(vec![
            wide(),
            options(&[
                ("type", Value::Str("bar".into())),
                ("y", strings(&["small", "large"])),
            ]),
        ]);
        // Four categories, two series, plus the background rect.
        assert_eq!(svg.matches("<rect").count(), 9, "{svg:.600}");

        // And side by side rather than on top of each other: eight bars at
        // eight distinct x positions. Drawing them at the same x hides one
        // series behind the other, which is the failure a grouped bar chart
        // exists to avoid.
        let mut positions: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .filter_map(|x| x.parse::<f64>().ok())
            .collect();
        positions.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        let distinct = {
            let mut copy = positions.clone();
            copy.dedup();
            copy.len()
        };
        assert_eq!(distinct, 8, "bars overlap: {positions:?}");

        // And the leftmost one clear of the y axis, which sits at the default
        // left margin of 60. A bar drawn on the axis reads as part of it.
        assert!(
            positions[0] > 60.0,
            "first bar sits on the axis: {positions:?}"
        );
    }

    #[test]
    fn a_missing_column_is_an_error_rather_than_a_silently_empty_plot() {
        let error = call_plot_builtin(
            "plot",
            vec![wide(), options(&[("y", strings(&["small", "enormous"]))])],
        )
        .expect_err("a column that is not in the table");
        assert!(
            error.to_string().contains("enormous"),
            "the message should name the column: {error}"
        );
    }

    #[test]
    fn the_quartiles_are_the_ones_quantile_reports() {
        // R 4.6.1: quantile(1:10) gives 3.25, 5.5, 7.75. The nearest-rank rule
        // the box plot used gives 3, 5.5 and 8 for the same ten values.
        let ten: Vec<f64> = (1..=10).map(f64::from).collect();
        for (p, expected) in [(0.25, 3.25), (0.5, 5.5), (0.75, 7.75)] {
            let got = quantile_type7(&ten, p);
            assert!(
                (got - expected).abs() < 1e-12,
                "quantile at {p} is {expected}, got {got}"
            );
        }
        // And on the primes, where the interpolation lands off a data point.
        let primes = [2.0, 3.0, 5.0, 7.0, 11.0, 13.0, 17.0, 19.0, 23.0, 29.0];
        assert!((quantile_type7(&primes, 0.75) - 18.5).abs() < 1e-12);
    }

    fn categories(names: &[&str], values: &[f64]) -> Value {
        Value::Table(Table::new(
            vec!["dept".to_string(), "rate".to_string()],
            names
                .iter()
                .zip(values.iter())
                .map(|(name, value)| vec![Value::Str((*name).into()), Value::Float(*value)])
                .collect(),
        ))
    }

    #[test]
    fn a_bar_chart_puts_the_category_names_under_the_bars() {
        // A category has no numeric position, so reading the column as numbers
        // gives NaN and an axis running 0.0 to 1.0 under bars that have nothing
        // to do with those numbers. That is what this used to draw.
        let svg = render(vec![
            categories(&["Biology", "Chemistry", "Physics"], &[0.62, 0.34, 0.51]),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        for department in ["Biology", "Chemistry", "Physics"] {
            assert!(
                svg.contains(&format!(">{department}<")),
                "{department} is missing: {svg:.700}"
            );
        }
        // And nothing else on that row: the tick labels under a bar chart are
        // the categories, not numbers interpolated from a column of NaN. The
        // default 600px canvas puts them at y=568.
        let under_the_axis: Vec<&str> = svg
            .split(r#" y="568.0""#)
            .skip(1)
            .filter_map(|fragment| fragment.split('>').nth(1))
            .filter_map(|text| text.split('<').next())
            .collect();
        assert_eq!(
            under_the_axis,
            vec!["Biology", "Chemistry", "Physics"],
            "the x tick labels are not the categories"
        );

        // Centred in its slot, not on the edge between two of them. With three
        // categories across the default 720px plot area, the first belongs at
        // 60 + 240/2 = 180.
        let biology_x = svg
            .split(r#"<text x=""#)
            .find(|fragment| fragment.contains(">Biology<"))
            .and_then(|fragment| fragment.split('"').next())
            .and_then(|x| x.parse::<f64>().ok())
            .expect("a Biology label with an x");
        assert!(
            (biology_x - 180.0).abs() < 1.0,
            "the first category label sits at {biology_x}, not centred at 180"
        );
    }

    #[test]
    fn a_scatter_keeps_its_numeric_axis() {
        // The x values are deliberately not round, so a numeric axis puts ticks
        // between them and a category axis would print the values themselves.
        let uneven = Value::Table(Table::new(
            vec!["x".to_string(), "y".to_string()],
            [0.0, 33.0, 67.0, 100.0]
                .iter()
                .map(|x| vec![Value::Float(*x), Value::Float(*x)])
                .collect(),
        ));
        let svg = render(vec![uneven, options(&[("x", Value::Str("x".into()))])]);
        assert!(
            svg.contains(">20<") || svg.contains(">20.0<"),
            "the x axis should be numbered, not labelled: {svg:.700}"
        );
        assert!(
            !svg.contains(">33<"),
            "the data values are being used as tick labels: {svg:.700}"
        );
    }

    #[test]
    fn too_many_categories_to_fit_are_thinned_rather_than_overlapped() {
        // Sixty labels across seven hundred pixels is unreadable mush. Fewer
        // labels is worse than all of them only when they fit.
        let names: Vec<String> = (1..=60).map(|n| format!("sample{n}")).collect();
        let refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let values: Vec<f64> = (1..=60).map(f64::from).collect();
        let svg = render(vec![
            categories(&refs, &values),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        let drawn = names
            .iter()
            .filter(|name| svg.contains(&format!(">{name}<")))
            .count();
        assert!(drawn > 0, "no labels at all");
        assert!(drawn < 60, "all 60 labels were drawn: they cannot fit");
        // Still thinned evenly rather than truncated: the last one is present.
        assert!(svg.contains(">sample1<"), "the first label is missing");
    }

    #[test]
    fn a_bar_chart_measures_from_zero() {
        // Two counts, one twice the other, must be drawn as one bar twice the
        // height of the other. Scaling to the data's own range instead makes
        // the smaller bar vanish entirely, which is how a 3% difference gets
        // published as a dramatic one.
        let svg = render(vec![
            categories(&["few", "many"], &[10.0, 20.0]),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        let heights: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|fragment| {
                fragment
                    .split(r#"height=""#)
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|h| h.parse::<f64>().ok())
            })
            .collect();
        assert_eq!(heights.len(), 2, "expected two bars: {svg:.700}");
        assert!(heights[0] > 1.0, "the smaller bar collapsed to {heights:?}");
        assert!(
            (heights[1] / heights[0] - 2.0).abs() < 0.05,
            "20 should be twice the height of 10, got {heights:?}"
        );
    }

    #[test]
    fn a_negative_bar_hangs_below_the_zero_line() {
        // Bars grow from zero in both directions. Growing them from the
        // smallest value instead flattens the negative one to nothing, and it
        // is the negative bar that usually carries the news.
        let svg = render(vec![
            categories(&["loss", "gain"], &[-5.0, 10.0]),
            options(&[("type", Value::Str("bar".into()))]),
        ]);
        let heights: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|fragment| {
                fragment
                    .split(r#"height=""#)
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|h| h.parse::<f64>().ok())
            })
            .collect();
        assert_eq!(heights.len(), 2, "expected two bars: {svg:.700}");
        assert!(
            heights[0] > 1.0,
            "the negative bar collapsed to {heights:?}"
        );
        assert!(
            (heights[1] / heights[0] - 2.0).abs() < 0.05,
            "10 should be twice the length of -5, got {heights:?}"
        );

        // Both bars must meet at the zero line: the negative one starts there
        // and hangs down, the positive one ends there. A rect placed at the far
        // end of its own length instead is the right size in the wrong place.
        let tops: Vec<f64> = svg
            .split(r#"<rect x=""#)
            .skip(1)
            .filter_map(|fragment| {
                fragment
                    .split(r#" y=""#)
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                    .and_then(|y| y.parse::<f64>().ok())
            })
            .collect();
        let zero_line = tops[1] + heights[1];
        assert!(
            (tops[0] - zero_line).abs() < 0.2,
            "the negative bar starts at {}, the zero line is at {zero_line}",
            tops[0]
        );
    }

    #[test]
    fn a_scatter_still_frames_the_data_it_has() {
        // The zero rule is for bars only. Forcing a scatter of values around
        // 1000 to include the origin would waste the whole plot area.
        let svg = render(vec![
            Value::Table(Table::new(
                vec!["x".to_string(), "y".to_string()],
                (0..4)
                    .map(|n| {
                        vec![
                            Value::Float(f64::from(n)),
                            Value::Float(1000.0 + f64::from(n)),
                        ]
                    })
                    .collect(),
            )),
            options(&[("type", Value::Str("scatter".into()))]),
        ]);
        assert!(
            !svg.contains(">0<") || svg.contains(">1000<"),
            "the y axis was stretched down to zero: {svg:.700}"
        );
        assert!(svg.contains(">1000<") || svg.contains(">1000.0<"));
    }

    #[test]
    fn a_box_plot_marks_the_points_beyond_the_whiskers() {
        // Tukey's rule is the reason to draw a box plot at all: the whisker
        // stops at the last value within 1.5 IQR of the box and everything past
        // it gets its own mark. Reaching to the extremes instead draws every
        // dataset as though it had no outliers.
        // Three of the columns run 1 to 7 with the last value replaced, so
        // each has a box from 2.75 to 6.25 and an IQR of 3.5, putting the fence
        // at 11.5. The odd values sit either side of it: 14 is past it and must
        // be marked, 10.5 is inside it and must not. A wider 3 IQR rule (fence
        // 16.75) marks neither; a narrower 1 IQR rule (fence 9.75) marks both.
        // Something merely enormous would pass whatever multiple was used.
        //
        // The fourth column puts values below the box instead of above it,
        // because the two whiskers are separate pieces of code. It runs
        // -6, -1, 3..8: the box is 2 to 6.25, the fence 6.375, so -6 is out and
        // -1 is not. Two low values rather than one, so that reaching down from
        // the wrong quartile is visible as an extra mark rather than landing on
        // the same answer by luck.
        let rows: [[f64; 4]; 8] = [
            [1.0, 1.0, 1.0, -6.0],
            [2.0, 2.0, 2.0, -1.0],
            [3.0, 3.0, 3.0, 3.0],
            [4.0, 4.0, 4.0, 4.0],
            [5.0, 5.0, 5.0, 5.0],
            [6.0, 6.0, 6.0, 6.0],
            [7.0, 7.0, 7.0, 7.0],
            [8.0, 10.5, 14.0, 8.0],
        ];
        let both = Value::Table(Table::new(
            vec![
                "clean".to_string(),
                "borderline".to_string(),
                "spiky".to_string(),
                "low".to_string(),
            ],
            rows.iter()
                .map(|row| row.iter().copied().map(Value::Float).collect())
                .collect(),
        ));
        let clean_only = table(
            &["clean", "also_clean"],
            &[
                &[1.0, 1.0],
                &[2.0, 2.0],
                &[3.0, 3.0],
                &[4.0, 4.0],
                &[5.0, 5.0],
                &[6.0, 6.0],
                &[7.0, 7.0],
                &[8.0, 8.0],
            ],
        );
        let box_opts = options(&[("type", Value::Str("box".into()))]);

        let plain = render(vec![clean_only, box_opts.clone()]);
        assert_eq!(
            plain.matches("<circle").count(),
            0,
            "nothing here is beyond a fence: {plain:.600}"
        );

        let flagged = render(vec![both, box_opts]);
        assert_eq!(
            flagged.matches("<circle").count(),
            2,
            "the 14 and the -6 should both be marked: {flagged:.600}"
        );

        // The median line is the most-read mark on a box plot, and nothing
        // above would notice it being drawn at the wrong height. The first
        // column runs 1 to 8, whose quartiles are 2.75, 4.5 and 6.25 -- the
        // median exactly halfway up the box -- so the line has to land on the
        // box's midpoint whatever the vertical scale turns out to be.
        let attribute = |fragment: &str, name: &str| -> f64 {
            fragment
                .split(&format!("{name}=\""))
                .nth(1)
                .and_then(|rest| rest.split('"').next())
                .and_then(|value| value.parse::<f64>().ok())
                .unwrap_or_else(|| panic!("no {name} in {fragment:.120}"))
        };
        let first_box = plain.split(r#"<rect x=""#).nth(1).expect("a box is drawn");
        let box_top = attribute(first_box, "y");
        let box_height = attribute(first_box, "height");
        let median_line = plain
            .split(r#"stroke-width="2""#)
            .next()
            .and_then(|before| before.rsplit("<line ").next())
            .expect("a median line");
        let median_y = attribute(median_line, "y1");
        assert!(
            (median_y - (box_top + box_height / 2.0)).abs() < 0.2,
            "median at {median_y}, box from {box_top} spanning {box_height}"
        );
    }
}

#[cfg(test)]
mod thinning_tests {
    use super::thin_to_pixel_grid;

    // The whole value of thinning rests on one promise: it removes overdraw,
    // never coverage. These check that promise directly, because the failure
    // mode -- a variant quietly disappearing from a GWAS figure -- is invisible
    // in the rendered output.

    const AREA: (f64, f64, f64, f64) = (0.0, 0.0, 100.0, 100.0);

    #[test]
    fn points_in_distinct_pixels_all_survive() {
        let points = [(1.0, 1.0), (2.0, 1.0), (3.0, 1.0), (1.0, 2.0)];
        let rank = [0.0; 4];
        assert_eq!(
            thin_to_pixel_grid(&points, AREA, 1.0, &rank),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn a_shared_pixel_keeps_the_highest_rank() {
        // All four land in pixel (5, 5) at scale 1; only the strongest signal
        // is worth the pixel, and in a Manhattan plot that is the smallest p.
        let points = [(5.1, 5.1), (5.9, 5.2), (5.4, 5.7), (5.2, 5.5)];
        let rank = [1.0, 9.0, 3.0, 2.0];
        assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank), vec![1]);
    }

    #[test]
    fn ties_resolve_by_input_order_not_hash_order() {
        // Equal ranks must not leave the survivor up to HashMap iteration, or
        // the same data would render differently between runs.
        let points = [(5.1, 5.1), (5.6, 5.6), (5.8, 5.2)];
        let rank = [4.0, 4.0, 4.0];
        for _ in 0..64 {
            assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank), vec![0]);
        }
    }

    #[test]
    fn a_finer_grid_keeps_more_points() {
        // Same data, four times the pixels: separations too small to see at
        // scale 1 become visible at scale 4, so nothing is merged.
        let points = [(5.05, 5.05), (5.55, 5.55), (5.80, 5.30)];
        let rank = [1.0, 2.0, 3.0];
        assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank).len(), 1);
        assert_eq!(thin_to_pixel_grid(&points, AREA, 4.0, &rank).len(), 3);
    }

    #[test]
    fn the_area_origin_is_subtracted_before_gridding() {
        // Cells are measured from the plot area, not the page. An offset panel
        // must thin the same way an unoffset one does.
        let flush = [(0.2, 0.2), (0.7, 0.7), (1.4, 0.3)];
        let offset: Vec<(f64, f64)> = flush.iter().map(|(x, y)| (x + 60.0, y + 40.0)).collect();
        let rank = [1.0, 2.0, 3.0];
        assert_eq!(
            thin_to_pixel_grid(&flush, (0.0, 0.0, 100.0, 100.0), 1.0, &rank),
            thin_to_pixel_grid(&offset, (60.0, 40.0, 100.0, 100.0), 1.0, &rank)
        );
    }

    #[test]
    fn non_finite_coordinates_are_dropped_rather_than_gridded() {
        // NaN would floor to a garbage cell and could evict a real point.
        let points = [(f64::NAN, 1.0), (1.0, f64::INFINITY), (2.0, 2.0)];
        let rank = [9.0, 9.0, 0.0];
        assert_eq!(thin_to_pixel_grid(&points, AREA, 1.0, &rank), vec![2]);
    }

    #[test]
    fn every_occupied_pixel_still_gets_a_point() {
        // The contract in one assertion: the set of occupied cells before and
        // after thinning is identical, so no pixel goes unpainted.
        use std::collections::HashSet;
        let points: Vec<(f64, f64)> = (0..5000)
            .map(|i| {
                let f = i as f64;
                ((f * 0.037) % 100.0, (f * 0.611) % 100.0)
            })
            .collect();
        let rank: Vec<f64> = (0..5000).map(|i| (i % 97) as f64).collect();
        let cell = |&(x, y): &(f64, f64)| (x.floor() as i64, y.floor() as i64);
        let before: HashSet<(i64, i64)> = points.iter().map(cell).collect();
        let kept = thin_to_pixel_grid(&points, AREA, 1.0, &rank);
        let after: HashSet<(i64, i64)> = kept.iter().map(|&i| cell(&points[i])).collect();
        assert_eq!(before, after);
        assert!(kept.len() < points.len(), "nothing was thinned at all");
    }
}

#[cfg(test)]
mod hue_palette_tests {
    use super::hue_palette;

    /// Reference values printed by R: `scales::hue_pal()(n)`.
    ///
    /// A fixed table is only right at the one `n` it was copied from, which is
    /// how a two-group plot could look correct while every other count was
    /// silently off-palette.
    #[test]
    fn hue_palette_matches_r_scales_hue_pal() {
        let expected: [&[&str]; 5] = [
            &["#f8766d", "#00bfc4"],
            &["#f8766d", "#00ba38", "#619cff"],
            &["#f8766d", "#7cae00", "#00bfc4", "#c77cff"],
            &["#f8766d", "#a3a500", "#00bf7d", "#00b0f6", "#e76bf3"],
            &[
                "#f8766d", "#b79f00", "#00ba38", "#00bfc4", "#619cff", "#f564e3",
            ],
        ];
        for (index, reference) in expected.iter().enumerate() {
            let count = index + 2;
            assert_eq!(
                hue_palette(count),
                *reference,
                "hue_pal({count}) disagrees with R"
            );
        }
    }

    #[test]
    fn hue_palette_always_returns_one_colour_per_group() {
        for count in 1..=24 {
            let palette = hue_palette(count);
            assert_eq!(palette.len(), count);
            assert!(palette
                .iter()
                .all(|colour| colour.len() == 7 && colour.starts_with('#')));
        }
    }
}

#[cfg(test)]
mod ggplot_binning_tests {
    use super::{histogram_equal_edges, histogram_ggplot_edges};

    /// `bins = n` means different things in ggplot2 and in matplotlib.
    ///
    /// ggplot2 centres the first bin on the minimum with width range/(n-1);
    /// an equal split of [min, max] uses width range/n starting at the
    /// minimum. Same `bins`, different bars.
    #[test]
    fn ggplot_bins_are_not_an_equal_split_of_the_range() {
        let values: Vec<f64> = (0..100).map(|value| value as f64).collect();
        let ggplot = histogram_ggplot_edges(&values, 30);
        let span = histogram_equal_edges(&values, 30);
        assert_eq!(ggplot.len(), 31, "ggplot rule must still yield 30 bins");
        assert_eq!(span.len(), 31);

        let ggplot_width = ggplot[1] - ggplot[0];
        assert!(
            (ggplot_width - 99.0 / 29.0).abs() < 1e-9,
            "width must be range/(bins - 1), got {ggplot_width}"
        );
        assert!(
            ggplot[0] < 0.0,
            "the first edge sits half a bin below the minimum, got {}",
            ggplot[0]
        );
        assert!(
            *ggplot.last().unwrap() > 99.0,
            "the last edge sits above the maximum"
        );
        assert!(
            span[0].abs() < 1e-9 && (span[30] - 99.0).abs() < 1e-9,
            "the span rule still runs edge to edge"
        );
    }

    #[test]
    fn ggplot_edges_are_evenly_spaced_and_cover_the_data() {
        let values = [12.88_f64, 20.1, 33.4, 47.9, 81.25];
        let edges = histogram_ggplot_edges(&values, 30);
        let width = edges[1] - edges[0];
        for pair in edges.windows(2) {
            assert!((pair[1] - pair[0] - width).abs() < 1e-9, "uneven bin width");
        }
        assert!(edges[0] <= 12.88 && *edges.last().unwrap() >= 81.25);
    }
}

#[cfg(test)]
mod text_metric_tests {

    use super::estimate_text_width;

    /// Advance widths straight out of Arial's `hmtx` table, which matches the
    /// Helvetica AFM values these three faces have shared since PostScript.
    #[test]
    fn widths_match_the_published_font_metrics() {
        for (character, per_mille) in [
            (' ', 278.0),
            ('.', 278.0),
            ('i', 222.0),
            ('m', 833.0),
            ('0', 556.0),
            ('A', 667.0),
            ('W', 944.0),
        ] {
            let width = estimate_text_width(&character.to_string(), 1000.0);
            assert!(
                (width - per_mille).abs() < 0.5,
                "{character:?} should advance {per_mille} per em, got {width}"
            );
        }
    }

    #[test]
    fn a_string_is_the_sum_of_its_glyphs_and_scales_with_size() {
        let label = "Height (cm)";
        let at_ten = estimate_text_width(label, 10.0);
        let at_twenty = estimate_text_width(label, 20.0);
        assert!(
            (at_twenty - 2.0 * at_ten).abs() < 1e-9,
            "width must be linear in size"
        );
        // 5.167 em from the real table; the character-class rule said 5.94.
        assert!(
            (at_ten - 51.67).abs() < 0.05,
            "expected 51.67px at size 10, got {at_ten}"
        );
    }

    #[test]
    fn characters_outside_the_table_still_get_a_width() {
        assert!(estimate_text_width("\u{4e2d}\u{6587}", 10.0) > 0.0);
        assert_eq!(
            estimate_text_width("\u{7}", 10.0),
            0.0,
            "control characters take no space"
        );
    }
}
