//! Renderer-neutral contingency-table mosaic plots.

use crate::plot::{
    get_opt_f64, get_opt_str, plot_theme, publication_diverging_color, standalone_plot_html,
    SvgCanvas, PALETTE, PLOT_SPEC_SCHEMA,
};
#[cfg(feature = "native")]
use crate::plot::{render_svg_terminal, TerminalPlotStyle};
use bl_core::error::{BioLangError, ErrorKind, Result};
use bl_core::value::{Table, Value};
use std::collections::{BTreeSet, HashMap};

fn shown(value: &Value) -> String {
    match value {
        Value::Str(value) => value.to_string(),
        Value::Int(value) => value.to_string(),
        Value::Float(value) => format!("{value}"),
        Value::Bool(value) => value.to_string(),
        Value::Nil => "missing".to_string(),
        other => format!("{other}"),
    }
}

fn labels_option(
    opts: &HashMap<String, Value>,
    key: &str,
    expected: usize,
) -> Result<Option<Vec<String>>> {
    let Some(value) = opts.get(key) else {
        return Ok(None);
    };
    let Value::List(values) = value else {
        return Err(BioLangError::type_error(
            format!("mosaic_plot() option '{key}' must be a List"),
            None,
        ));
    };
    if values.len() != expected {
        return Err(BioLangError::type_error(
            format!(
                "mosaic_plot() option '{key}' needs {expected} labels, got {}",
                values.len()
            ),
            None,
        ));
    }
    Ok(Some(values.iter().map(shown).collect()))
}

/// Freeze contingency counts and every rectangle boundary into a replayable
/// plot specification. Rows occupy horizontal space by row total; cells split
/// each row vertically, making every rectangle's area observed / grand total.
pub(crate) fn specification(table: &Table, opts: &HashMap<String, Value>) -> Result<Value> {
    if table.num_rows() == 0 || table.num_cols() == 0 {
        return Err(BioLangError::type_error(
            "mosaic_plot() requires a non-empty contingency Table",
            None,
        ));
    }
    let requested_label = opts
        .get("row")
        .and_then(Value::as_str)
        .map(|name| {
            table.col_index(name).ok_or_else(|| {
                BioLangError::type_error(
                    format!("mosaic_plot() row label column '{name}' was not found"),
                    None,
                )
            })
        })
        .transpose()?;
    let inferred_label = (table.num_cols() > 1
        && table
            .rows
            .iter()
            .all(|row| matches!(row.first(), Some(Value::Str(_)))))
    .then_some(0);
    let label_column = requested_label.or(inferred_label);
    let value_columns = (0..table.num_cols())
        .filter(|column| Some(*column) != label_column)
        .collect::<Vec<_>>();
    if value_columns.is_empty() {
        return Err(BioLangError::type_error(
            "mosaic_plot() requires at least one numeric count column",
            None,
        ));
    }

    let default_rows = table
        .rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            label_column
                .map(|column| shown(&row[column]))
                .unwrap_or_else(|| (index + 1).to_string())
        })
        .collect::<Vec<_>>();
    let row_labels = labels_option(opts, "row_labels", table.num_rows())?.unwrap_or(default_rows);
    let default_columns = value_columns
        .iter()
        .map(|column| table.columns[*column].clone())
        .collect::<Vec<_>>();
    let column_labels =
        labels_option(opts, "col_labels", value_columns.len())?.unwrap_or(default_columns);

    let mut counts = vec![vec![0.0; value_columns.len()]; table.num_rows()];
    let mut missing_cells = 0usize;
    for (row_index, row) in table.rows.iter().enumerate() {
        for (column_index, source_column) in value_columns.iter().enumerate() {
            let count = match &row[*source_column] {
                Value::Int(value) => *value as f64,
                Value::Float(value) => *value,
                Value::Str(value) => value.parse().unwrap_or(f64::NAN),
                _ => f64::NAN,
            };
            if !count.is_finite() {
                missing_cells += 1;
                continue;
            }
            if count < 0.0 {
                return Err(BioLangError::type_error(
                    format!(
                        "mosaic_plot() counts must be non-negative; row {}, column '{}' is {count}",
                        row_index + 1,
                        table.columns[*source_column]
                    ),
                    None,
                ));
            }
            counts[row_index][column_index] = count;
        }
    }
    let row_totals = counts
        .iter()
        .map(|row| row.iter().sum::<f64>())
        .collect::<Vec<_>>();
    let column_totals = (0..value_columns.len())
        .map(|column| counts.iter().map(|row| row[column]).sum::<f64>())
        .collect::<Vec<_>>();
    let total = row_totals.iter().sum::<f64>();
    if total <= 0.0 {
        return Err(BioLangError::type_error(
            "mosaic_plot() requires at least one positive count",
            None,
        ));
    }

    let shade = match opts.get("shade") {
        None | Some(Value::Bool(false)) => "column",
        Some(Value::Bool(true)) => "residual",
        Some(Value::Str(value)) => match value.to_ascii_lowercase().as_str() {
            "residual" | "pearson" => "residual",
            "column" | "category" | "none" => "column",
            _ => {
                return Err(BioLangError::type_error(
                    "mosaic_plot() option 'shade' must be Bool, 'column', or 'residual'",
                    None,
                ))
            }
        },
        Some(_) => {
            return Err(BioLangError::type_error(
                "mosaic_plot() option 'shade' must be Bool, 'column', or 'residual'",
                None,
            ))
        }
    };
    let view = get_opt_str(opts, "view", "count").to_ascii_lowercase();
    if !matches!(view.as_str(), "count" | "row" | "column" | "total") {
        return Err(BioLangError::type_error(
            "mosaic_plot() option 'view' must be count, row, column, or total",
            None,
        ));
    }

    let mut rows = Vec::with_capacity(table.num_rows() * value_columns.len());
    let mut x0 = 0.0;
    for row_index in 0..table.num_rows() {
        let x1 = x0 + row_totals[row_index] / total;
        let mut y0 = 0.0;
        for column_index in 0..value_columns.len() {
            let observed = counts[row_index][column_index];
            let y1 = if row_totals[row_index] == 0.0 {
                y0
            } else {
                y0 + observed / row_totals[row_index]
            };
            let expected = row_totals[row_index] * column_totals[column_index] / total;
            let residual = if expected == 0.0 {
                0.0
            } else {
                (observed - expected) / expected.sqrt()
            };
            let fill = if shade == "residual" {
                publication_diverging_color((residual.clamp(-4.0, 4.0) + 4.0) / 8.0)
            } else {
                PALETTE[column_index % PALETTE.len()].to_string()
            };
            rows.push(vec![
                Value::Int(row_index as i64),
                Value::Int(column_index as i64),
                Value::Str(row_labels[row_index].clone().into()),
                Value::Str(column_labels[column_index].clone().into()),
                Value::Float(observed),
                Value::Float(row_totals[row_index]),
                Value::Float(column_totals[column_index]),
                Value::Float(expected),
                Value::Float(residual),
                Value::Float(observed / total),
                Value::Float(if row_totals[row_index] == 0.0 {
                    0.0
                } else {
                    observed / row_totals[row_index]
                }),
                Value::Float(if column_totals[column_index] == 0.0 {
                    0.0
                } else {
                    observed / column_totals[column_index]
                }),
                Value::Float(x0),
                Value::Float(x1),
                Value::Float(y0),
                Value::Float(y1),
                Value::Str(fill.into()),
            ]);
            y0 = y1;
        }
        x0 = x1;
    }
    let data = Table::new(
        [
            "row_index",
            "column_index",
            "row_label",
            "column_label",
            "observed",
            "row_total",
            "column_total",
            "expected",
            "pearson_residual",
            "total_proportion",
            "row_proportion",
            "column_proportion",
            "x0",
            "x1",
            "y0",
            "y1",
            "fill",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        rows,
    );
    let warning_values = if missing_cells == 0 {
        Vec::new()
    } else {
        vec![Value::Str(
            format!("{missing_cells} missing or non-finite cells were treated as zero").into(),
        )]
    };
    let show_values = opts
        .get("show_values")
        .and_then(Value::as_bool)
        .unwrap_or(data.num_rows() <= 20);
    Ok(Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str("mosaic".into())),
            ("plot".into(), Value::Str("mosaic".into())),
            ("data".into(), Value::Table(data)),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "Mosaic plot").into()),
            ),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 800.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 520.0)),
                        ),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "publication").into()),
                        ),
                        (
                            "title".into(),
                            Value::Str(get_opt_str(opts, "title", "Mosaic plot").into()),
                        ),
                        (
                            "subtitle".into(),
                            Value::Str(
                                get_opt_str(
                                    opts,
                                    "subtitle",
                                    "Rectangle area is proportional to observed count",
                                )
                                .into(),
                            ),
                        ),
                        (
                            "caption".into(),
                            Value::Str(get_opt_str(opts, "caption", "").into()),
                        ),
                        ("view".into(), Value::Str(view.into())),
                        ("shade".into(), Value::Str(shade.into())),
                        ("show_values".into(), Value::Bool(show_values)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("mosaic_plot".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "input_columns".into(),
                            Value::Int(value_columns.len() as i64),
                        ),
                        ("missing_cells".into(), Value::Int(missing_cells as i64)),
                        ("grand_total".into(), Value::Float(total)),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(warning_values.into())),
        ])
        .into(),
    ))
}

fn field(table: &Table, name: &str) -> Result<usize> {
    table.col_index(name).ok_or_else(|| {
        BioLangError::type_error(
            format!("render_plot() mosaic data is missing '{name}'"),
            None,
        )
    })
}

pub(crate) fn render(value: &Value, render_opts: &HashMap<String, Value>) -> Result<Value> {
    let record = match value {
        Value::Record(record)
            if matches!(record.get("schema"), Some(Value::Str(schema)) if schema == PLOT_SPEC_SCHEMA)
                && matches!(record.get("kind"), Some(Value::Str(kind)) if kind == "mosaic") =>
        {
            record
        }
        _ => {
            return Err(BioLangError::type_error(
                "render_plot() requires a biolang.plot.spec/v1 mosaic Record",
                None,
            ))
        }
    };
    let data = match record.get("data") {
        Some(Value::Table(table)) if table.num_rows() > 0 => table,
        _ => {
            return Err(BioLangError::type_error(
                "render_plot() mosaic field 'data' must be a non-empty Table",
                None,
            ))
        }
    };
    let mut opts = match record.get("options") {
        Some(Value::Record(opts)) => opts.as_ref().clone(),
        _ => {
            return Err(BioLangError::type_error(
                "render_plot() mosaic field 'options' must be Record",
                None,
            ))
        }
    };
    let format = get_opt_str(render_opts, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    for name in ["width", "height"] {
        if let Some(override_value) = render_opts.get(name) {
            opts.insert(name.into(), override_value.clone());
        }
    }
    let width = get_opt_f64(&opts, "width", 800.0).max(320.0);
    let height = get_opt_f64(&opts, "height", 520.0).max(260.0);
    let title = get_opt_str(&opts, "title", "Mosaic plot");
    let subtitle = get_opt_str(&opts, "subtitle", "");
    let caption = get_opt_str(&opts, "caption", "");
    let view = get_opt_str(&opts, "view", "count");
    let shade = get_opt_str(&opts, "shade", "column");
    let show_values = opts.get("show_values").is_some_and(Value::is_truthy);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(&opts));
    canvas.margin.top = if subtitle.is_empty() { 48.0 } else { 64.0 };
    canvas.margin.left = 38.0;
    canvas.margin.right = 150.0;
    canvas.margin.bottom = 72.0;
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Mosaic contingency plot with {} cells. Rectangle area represents observed frequency; shading uses {shade} values.",
        data.num_rows()
    ));

    let ri = field(data, "row_index")?;
    let ci = field(data, "column_index")?;
    let rl = field(data, "row_label")?;
    let cl = field(data, "column_label")?;
    let oi = field(data, "observed")?;
    let ei = field(data, "expected")?;
    let pi = field(data, "pearson_residual")?;
    let row_pi = field(data, "row_proportion")?;
    let col_pi = field(data, "column_proportion")?;
    let total_pi = field(data, "total_proportion")?;
    let x0i = field(data, "x0")?;
    let x1i = field(data, "x1")?;
    let y0i = field(data, "y0")?;
    let y1i = field(data, "y1")?;
    let fi = field(data, "fill")?;
    let (plot_x, plot_y, plot_width, plot_height) = (
        canvas.margin.left,
        canvas.margin.top,
        canvas.plot_width(),
        canvas.plot_height(),
    );
    let escape = |text: &str| {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let mut seen_rows = BTreeSet::new();
    let mut seen_columns = BTreeSet::new();
    for row in &data.rows {
        let number = |index: usize| {
            row[index]
                .as_float()
                .filter(|value| value.is_finite())
                .ok_or_else(|| {
                    BioLangError::new(
                        ErrorKind::TypeError,
                        "render_plot() mosaic geometry must contain finite numbers",
                        None,
                    )
                })
        };
        let row_index = number(ri)? as usize;
        let column_index = number(ci)? as usize;
        let row_label = shown(&row[rl]);
        let column_label = shown(&row[cl]);
        let observed = number(oi)?;
        let expected = number(ei)?;
        let residual = number(pi)?;
        let (x0, x1, y0, y1) = (number(x0i)?, number(x1i)?, number(y0i)?, number(y1i)?);
        if !(0.0..=1.0).contains(&x0)
            || !(0.0..=1.0).contains(&x1)
            || !(0.0..=1.0).contains(&y0)
            || !(0.0..=1.0).contains(&y1)
            || x1 < x0
            || y1 < y0
        {
            return Err(BioLangError::type_error(
                "render_plot() mosaic coordinates must be ordered proportions from zero to one",
                None,
            ));
        }
        let x = plot_x + x0 * plot_width;
        let y = plot_y + (1.0 - y1) * plot_height;
        let cell_width = (x1 - x0) * plot_width;
        let cell_height = (y1 - y0) * plot_height;
        let fill = row[fi].as_str().unwrap_or("#cccccc");
        let tooltip = format!("{row_label} × {column_label}: observed {observed:.3}, expected {expected:.3}, Pearson residual {residual:.3}");
        canvas.elements.push(format!(
            r##"<rect data-biolang-mosaic-cell="true" x="{x:.2}" y="{y:.2}" width="{cell_width:.2}" height="{cell_height:.2}" fill="{}" stroke="#ffffff" stroke-width="1"><title>{}</title></rect>"##,
            escape(fill), escape(&tooltip)));
        if show_values && cell_width >= 34.0 && cell_height >= 22.0 {
            let label = match view {
                "row" => format!("{:.1}%", number(row_pi)? * 100.0),
                "column" => format!("{:.1}%", number(col_pi)? * 100.0),
                "total" => format!("{:.1}%", number(total_pi)? * 100.0),
                _ => format!("{observed:.0}"),
            };
            canvas.add_text(
                x + cell_width / 2.0,
                y + cell_height / 2.0 + 4.0,
                &label,
                "middle",
                10.0,
            );
        }
        if seen_rows.insert(row_index) {
            canvas.add_text(
                x + cell_width / 2.0,
                plot_y + plot_height + 20.0,
                &row_label,
                "middle",
                10.0,
            );
        }
        if shade != "residual" && seen_columns.insert(column_index) {
            let legend_y = plot_y + 8.0 + column_index as f64 * 22.0;
            canvas.add_rect(
                plot_x + plot_width + 18.0,
                legend_y - 10.0,
                12.0,
                12.0,
                fill,
            );
            canvas.add_text(
                plot_x + plot_width + 36.0,
                legend_y,
                &column_label,
                "start",
                10.0,
            );
        }
    }
    if shade == "residual" {
        canvas.add_text(
            plot_x + plot_width + 18.0,
            plot_y + 5.0,
            "Pearson residual",
            "start",
            10.0,
        );
        for (legend_index, residual) in [-4.0_f64, -2.0, 0.0, 2.0, 4.0].into_iter().enumerate() {
            let legend_y = plot_y + 27.0 + legend_index as f64 * 22.0;
            let colour = publication_diverging_color((residual + 4.0) / 8.0);
            canvas.add_rect(
                plot_x + plot_width + 18.0,
                legend_y - 10.0,
                12.0,
                12.0,
                &colour,
            );
            canvas.add_text(
                plot_x + plot_width + 36.0,
                legend_y,
                &format!("{residual:+.0}"),
                "start",
                10.0,
            );
        }
    }
    canvas.add_line(
        plot_x,
        plot_y,
        plot_x + plot_width,
        plot_y,
        canvas.theme.axis_colour,
        1.0,
    );
    canvas.add_line(
        plot_x,
        plot_y + plot_height,
        plot_x + plot_width,
        plot_y + plot_height,
        canvas.theme.axis_colour,
        1.0,
    );
    let svg = canvas.render();
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Ascii)
            .map(Value::Str)
            .map_err(|error| BioLangError::type_error(error, None)),
        #[cfg(feature = "native")]
        "unicode" | "braille" => render_svg_terminal(&svg, 80, 24, TerminalPlotStyle::Braille)
            .map(Value::Str)
            .map_err(|error| BioLangError::type_error(error, None)),
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::type_error(
            "render_plot() terminal mosaic output needs the native build",
            None,
        )),
        _ => Err(BioLangError::type_error(
            format!("render_plot() unknown mosaic format '{format}'"),
            None,
        )),
    }
}
