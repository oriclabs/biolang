//! Spec for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(crate) const PLOT_SPEC_SCHEMA: &str = "biolang.plot.spec/v1";

#[derive(Clone, Debug)]
pub(super) struct CartesianPoint {
    pub(super) source_row: usize,
    pub(super) x: f64,
    pub(super) y: f64,
    pub(super) lower: Option<f64>,
    pub(super) upper: Option<f64>,
}

#[derive(Clone, Debug)]
pub(super) struct CartesianSeries {
    pub(super) name: String,
    pub(super) colour: String,
    pub(super) points: Vec<CartesianPoint>,
}

#[derive(Clone, Debug)]
pub(super) struct CartesianPlotSpec {
    pub(super) kind: String,
    pub(super) width: f64,
    pub(super) height: f64,
    pub(super) theme: String,
    pub(super) title: String,
    pub(super) subtitle: String,
    pub(super) caption: String,
    pub(super) x_label: String,
    pub(super) y_label: String,
    pub(super) x_domain: (f64, f64),
    pub(super) y_domain: (f64, f64),
    pub(super) series: Vec<CartesianSeries>,
    pub(super) dropped_non_finite: usize,
    pub(super) x_column: String,
    pub(super) y_columns: Vec<String>,
    pub(super) lower_column: Option<String>,
    pub(super) upper_column: Option<String>,
}

pub(super) fn interval_column(
    opts: &HashMap<String, Value>,
    primary: &str,
    alias: &str,
) -> Option<String> {
    opts.get(primary)
        .or_else(|| opts.get(alias))
        .and_then(Value::as_str)
        .map(str::to_string)
}

pub(super) fn build_cartesian_plot_spec(
    table: &Table,
    opts: &HashMap<String, Value>,
    who: &str,
) -> Result<CartesianPlotSpec> {
    if table.num_cols() < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() requires table with at least 2 columns"),
            None,
        ));
    }
    let kind = get_opt_str(opts, "type", "scatter").to_ascii_lowercase();
    if !matches!(
        kind.as_str(),
        "scatter" | "line" | "errorbar" | "confidence"
    ) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "{who}() specification type '{kind}' is unsupported; expected scatter/line/errorbar/confidence"
            ),
            None,
        ));
    }
    let x_column = get_opt_str(opts, "x", &table.columns[0]).to_string();
    let y_columns = series_columns(opts, &table.columns[1]);
    let lower_column = interval_column(opts, "ymin", "lower");
    let upper_column = interval_column(opts, "ymax", "upper");
    if matches!(kind.as_str(), "errorbar" | "confidence")
        && (lower_column.is_none() || upper_column.is_none())
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() type '{kind}' requires ymin/ymax (or lower/upper) column names"),
            None,
        ));
    }

    let xs = extract_table_col(table, &x_column)?;
    let lowers = lower_column
        .as_deref()
        .map(|column| extract_table_col(table, column))
        .transpose()?;
    let uppers = upper_column
        .as_deref()
        .map(|column| extract_table_col(table, column))
        .transpose()?;
    let mut dropped_non_finite = 0usize;
    let mut series = Vec::with_capacity(y_columns.len());
    for (series_index, column) in y_columns.iter().enumerate() {
        let ys = extract_table_col(table, column)?;
        let mut points = Vec::with_capacity(xs.len().min(ys.len()));
        for row in 0..xs.len().min(ys.len()) {
            let x = xs[row];
            let y = ys[row];
            let lower = lowers.as_ref().and_then(|values| values.get(row)).copied();
            let upper = uppers.as_ref().and_then(|values| values.get(row)).copied();
            let interval_is_valid = match (lower, upper) {
                (Some(lo), Some(hi)) => lo.is_finite() && hi.is_finite() && lo <= hi,
                (None, None) => true,
                _ => false,
            };
            if !x.is_finite() || !y.is_finite() || !interval_is_valid {
                dropped_non_finite += 1;
                continue;
            }
            points.push(CartesianPoint {
                source_row: row,
                x,
                y,
                lower,
                upper,
            });
        }
        series.push(CartesianSeries {
            name: column.clone(),
            colour: PALETTE[series_index % PALETTE.len()].to_string(),
            points,
        });
    }
    if series.iter().all(|item| item.points.is_empty()) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{who}() received no complete finite observations"),
            None,
        ));
    }

    let mut x_values = Vec::new();
    let mut y_values = Vec::new();
    for item in &series {
        for point in &item.points {
            x_values.push(point.x);
            y_values.push(point.lower.unwrap_or(point.y));
            y_values.push(point.upper.unwrap_or(point.y));
        }
    }
    let default_y_label = if y_columns.len() == 1 {
        y_columns[0].as_str()
    } else {
        ""
    };
    Ok(CartesianPlotSpec {
        kind,
        width: get_opt_f64(opts, "width", 800.0).max(1.0),
        height: get_opt_f64(opts, "height", 600.0).max(1.0),
        theme: get_opt_str(opts, "theme", "biolang").to_string(),
        title: get_opt_str(opts, "title", "").to_string(),
        subtitle: get_opt_str(opts, "subtitle", "").to_string(),
        caption: get_opt_str(opts, "caption", "").to_string(),
        x_label: axis_label(opts, "xlabel", &x_column),
        y_label: axis_label(opts, "ylabel", default_y_label),
        x_domain: col_range(&x_values),
        y_domain: col_range(&y_values),
        series,
        dropped_non_finite,
        x_column,
        y_columns,
        lower_column,
        upper_column,
    })
}

pub(super) fn optional_number(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(Value::as_float)
        .filter(|number| number.is_finite())
}

pub(super) fn plot_spec_to_value(spec: &CartesianPlotSpec) -> Value {
    let mut rows = Vec::new();
    for item in &spec.series {
        for point in &item.points {
            rows.push(vec![
                Value::Int(point.source_row as i64),
                Value::Str(item.name.clone().into()),
                Value::Str(item.colour.clone().into()),
                Value::Float(point.x),
                Value::Float(point.y),
                point.lower.map(Value::Float).unwrap_or(Value::Nil),
                point.upper.map(Value::Float).unwrap_or(Value::Nil),
            ]);
        }
    }
    let data = Value::Table(Table::new(
        vec![
            "source_row".into(),
            "series".into(),
            "colour".into(),
            "x".into(),
            "y".into(),
            "lower".into(),
            "upper".into(),
        ],
        rows,
    ));
    let provenance = Value::Record(
        HashMap::from([
            ("x_column".into(), Value::Str(spec.x_column.clone().into())),
            (
                "y_columns".into(),
                Value::List(
                    spec.y_columns
                        .iter()
                        .map(|name| Value::Str(name.clone().into()))
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            (
                "lower_column".into(),
                spec.lower_column
                    .as_ref()
                    .map(|name| Value::Str(name.clone().into()))
                    .unwrap_or(Value::Nil),
            ),
            (
                "upper_column".into(),
                spec.upper_column
                    .as_ref()
                    .map(|name| Value::Str(name.clone().into()))
                    .unwrap_or(Value::Nil),
            ),
        ])
        .into(),
    );
    Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str(PLOT_SPEC_SCHEMA.into())),
            ("kind".into(), Value::Str(spec.kind.clone().into())),
            ("width".into(), Value::Float(spec.width)),
            ("height".into(), Value::Float(spec.height)),
            ("theme".into(), Value::Str(spec.theme.clone().into())),
            ("title".into(), Value::Str(spec.title.clone().into())),
            ("subtitle".into(), Value::Str(spec.subtitle.clone().into())),
            ("caption".into(), Value::Str(spec.caption.clone().into())),
            ("xlabel".into(), Value::Str(spec.x_label.clone().into())),
            ("ylabel".into(), Value::Str(spec.y_label.clone().into())),
            (
                "x_domain".into(),
                Value::List(
                    vec![Value::Float(spec.x_domain.0), Value::Float(spec.x_domain.1)].into(),
                ),
            ),
            (
                "y_domain".into(),
                Value::List(
                    vec![Value::Float(spec.y_domain.0), Value::Float(spec.y_domain.1)].into(),
                ),
            ),
            ("data".into(), data),
            (
                "dropped_non_finite".into(),
                Value::Int(spec.dropped_non_finite as i64),
            ),
            ("provenance".into(), provenance),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

pub(super) fn required_record_string(map: &HashMap<String, Value>, key: &str) -> Result<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() specification field '{key}' must be Str"),
                None,
            )
        })
}

pub(super) fn valid_spec_colour(colour: &str) -> bool {
    colour.len() == 7
        && colour.starts_with('#')
        && colour[1..]
            .chars()
            .all(|character| character.is_ascii_hexdigit())
}

pub(super) fn record_domain(map: &HashMap<String, Value>, key: &str) -> Result<(f64, f64)> {
    let values = match map.get(key) {
        Some(Value::List(values)) if values.len() == 2 => values,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() specification field '{key}' must contain two numbers"),
                None,
            ))
        }
    };
    match (values[0].as_float(), values[1].as_float()) {
        (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() => Ok((lo, hi)),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() specification field '{key}' contains a non-finite value"),
            None,
        )),
    }
}

pub(super) fn plot_spec_from_value(value: &Value) -> Result<CartesianPlotSpec> {
    let map = match value {
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
    let kind = required_record_string(map, "kind")?;
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() specification field 'data' must be Table",
                None,
            ))
        }
    };
    let indexes = ["source_row", "series", "colour", "x", "y", "lower", "upper"].map(|column| {
        table.col_index(column).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() specification data is missing '{column}'"),
                None,
            )
        })
    });
    let [source_index, series_index, colour_index, x_index, y_index, lower_index, upper_index] = [
        indexes[0].clone()?,
        indexes[1].clone()?,
        indexes[2].clone()?,
        indexes[3].clone()?,
        indexes[4].clone()?,
        indexes[5].clone()?,
        indexes[6].clone()?,
    ];
    let mut series: Vec<CartesianSeries> = Vec::new();
    for row in &table.rows {
        let name = row[series_index].as_str().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data series must be Str",
                None,
            )
        })?;
        let colour = row[colour_index]
            .as_str()
            .unwrap_or(PALETTE[series.len() % PALETTE.len()]);
        if !valid_spec_colour(colour) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data colour must be a #rrggbb value",
                None,
            ));
        }
        let x = optional_number(row.get(x_index)).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data x must be finite",
                None,
            )
        })?;
        let y = optional_number(row.get(y_index)).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() data y must be finite",
                None,
            )
        })?;
        let source_row = row[source_index].as_float().unwrap_or(0.0).max(0.0) as usize;
        let position = series.iter().position(|item| item.name == name);
        let target = match position {
            Some(index) => &mut series[index],
            None => {
                series.push(CartesianSeries {
                    name: name.to_string(),
                    colour: colour.to_string(),
                    points: Vec::new(),
                });
                series.last_mut().unwrap()
            }
        };
        target.points.push(CartesianPoint {
            source_row,
            x,
            y,
            lower: optional_number(row.get(lower_index)),
            upper: optional_number(row.get(upper_index)),
        });
    }
    if series.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() specification contains no marks",
            None,
        ));
    }
    let provenance = match map.get("provenance") {
        Some(Value::Record(record)) => Some(record),
        _ => None,
    };
    let x_column = provenance
        .and_then(|record| record.get("x_column"))
        .and_then(Value::as_str)
        .unwrap_or("x")
        .to_string();
    let y_columns = series.iter().map(|item| item.name.clone()).collect();
    Ok(CartesianPlotSpec {
        kind,
        width: optional_number(map.get("width")).unwrap_or(800.0).max(1.0),
        height: optional_number(map.get("height")).unwrap_or(600.0).max(1.0),
        theme: map
            .get("theme")
            .and_then(Value::as_str)
            .unwrap_or("biolang")
            .to_string(),
        title: required_record_string(map, "title")?,
        subtitle: map
            .get("subtitle")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        caption: map
            .get("caption")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string(),
        x_label: required_record_string(map, "xlabel")?,
        y_label: required_record_string(map, "ylabel")?,
        x_domain: record_domain(map, "x_domain")?,
        y_domain: record_domain(map, "y_domain")?,
        series,
        dropped_non_finite: optional_number(map.get("dropped_non_finite"))
            .unwrap_or(0.0)
            .max(0.0) as usize,
        x_column,
        y_columns,
        lower_column: None,
        upper_column: None,
    })
}

pub(super) fn render_cartesian_plot_spec(
    spec: &CartesianPlotSpec,
    raster: RasterChoice,
) -> Result<String> {
    let mut canvas =
        SvgCanvas::with_theme(spec.width, spec.height, PlotTheme::from_name(&spec.theme));
    let point_count = spec
        .series
        .iter()
        .map(|series| series.points.len())
        .sum::<usize>();
    canvas.set_accessible_description(format!(
        "{} plot with {} series and {} rendered marks; {} non-finite rows were excluded.",
        spec.kind,
        spec.series.len(),
        point_count,
        spec.dropped_non_finite
    ));
    let domain_x = Scale {
        domain: spec.x_domain,
        range: spec.x_domain,
    };
    let domain_y = Scale {
        domain: spec.y_domain,
        range: spec.y_domain,
    };
    let series_names = spec
        .series
        .iter()
        .map(|item| item.name.clone())
        .collect::<Vec<_>>();
    canvas.fit_cartesian_layout(
        &domain_x.nice_ticks(5),
        &domain_y.nice_ticks(5),
        &spec.x_label,
        &spec.y_label,
        &spec.title,
        &spec.subtitle,
        &spec.caption,
        legend_reserve_width(canvas.theme, &series_names),
    );
    canvas.draw_cartesian_grid(&domain_x, &domain_y);
    let x_scale = Scale {
        domain: spec.x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: spec.y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    for item in &spec.series {
        match spec.kind.as_str() {
            "scatter" => {
                let points: Vec<(f64, f64, &str)> = item
                    .points
                    .iter()
                    .map(|point| {
                        (
                            x_scale.map(point.x),
                            y_scale.map(point.y),
                            item.colour.as_str(),
                        )
                    })
                    .collect();
                let area = canvas.point_area();
                canvas.add_scatter(&points, 4.0, area, raster);
            }
            "line" => {
                let points = item
                    .points
                    .iter()
                    .map(|point| format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(point.y)))
                    .collect::<Vec<_>>();
                if !points.is_empty() {
                    canvas.elements.push(format!(
                        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
                        points.join(" "),
                        item.colour
                    ));
                }
            }
            "errorbar" => {
                for point in &item.points {
                    let (Some(lower), Some(upper)) = (point.lower, point.upper) else {
                        continue;
                    };
                    let x = x_scale.map(point.x);
                    let top = y_scale.map(upper);
                    let bottom = y_scale.map(lower);
                    canvas.add_line(x, top, x, bottom, &item.colour, 1.5);
                    canvas.add_line(x - 5.0, top, x + 5.0, top, &item.colour, 1.5);
                    canvas.add_line(x - 5.0, bottom, x + 5.0, bottom, &item.colour, 1.5);
                    canvas.add_circle(x, y_scale.map(point.y), 3.5, &item.colour);
                }
            }
            "confidence" => {
                let upper = item.points.iter().filter_map(|point| {
                    point.upper.map(|value| {
                        format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(value))
                    })
                });
                let lower = item.points.iter().rev().filter_map(|point| {
                    point.lower.map(|value| {
                        format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(value))
                    })
                });
                let band = upper.chain(lower).collect::<Vec<_>>();
                if band.len() >= 4 {
                    canvas.elements.push(format!(
                        r#"<polygon points="{}" fill="{}" fill-opacity="0.18" stroke="none" />"#,
                        band.join(" "),
                        item.colour
                    ));
                }
                let centre = item
                    .points
                    .iter()
                    .map(|point| format!("{:.1},{:.1}", x_scale.map(point.x), y_scale.map(point.y)))
                    .collect::<Vec<_>>();
                if !centre.is_empty() {
                    canvas.elements.push(format!(
                        r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="2" />"#,
                        centre.join(" "),
                        item.colour
                    ));
                }
            }
            other => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() unsupported specification kind '{other}'"),
                    None,
                ))
            }
        }
    }
    draw_legend(&mut canvas, &series_names);
    canvas.draw_x_axis(
        &Scale {
            domain: spec.x_domain,
            range: spec.x_domain,
        },
        &spec.x_label,
    );
    canvas.draw_y_axis(
        &Scale {
            domain: spec.y_domain,
            range: spec.y_domain,
        },
        &spec.y_label,
    );
    if !spec.title.is_empty() {
        canvas.draw_title(&spec.title);
    }
    canvas.draw_subtitle(&spec.subtitle);
    canvas.draw_caption(&spec.caption);
    Ok(canvas.render())
}

pub(crate) fn standalone_plot_html(svg: &str, title: &str) -> String {
    let title = if title.trim().is_empty() {
        "BioLang plot"
    } else {
        title
    };
    let escaped_title = title
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");
    format!(
        r#"<!doctype html><html><head><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>{escaped_title}</title><style>body{{margin:0;padding:1rem;font-family:system-ui,sans-serif}}figure{{margin:0;overflow:auto}}svg,canvas{{max-width:100%;height:auto}}button{{margin:0 0 .5rem .35rem}}</style></head><body><figure id="bl-figure" aria-labelledby="bl-caption"><figcaption id="bl-caption" style="position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0,0,0,0)">{escaped_title}</figcaption><button type="button" id="bl-toggle" aria-controls="bl-svg bl-canvas" aria-pressed="false" disabled>Use canvas</button><button type="button" id="bl-download" aria-controls="bl-canvas" disabled>Download PNG</button>{svg}<canvas id="bl-canvas" hidden role="img" aria-label="{escaped_title} canvas fallback"></canvas></figure><script>(function(){{const f=document.getElementById('bl-figure'),s=f.querySelector('svg'),c=document.getElementById('bl-canvas'),t=document.getElementById('bl-toggle'),d=document.getElementById('bl-download');s.id='bl-svg';const v=s.viewBox.baseVal,w=v.width||+s.getAttribute('width')||800,h=v.height||+s.getAttribute('height')||600,scale=Math.min(devicePixelRatio||1,2);c.width=Math.round(w*scale);c.height=Math.round(h*scale);c.style.width=w+'px';const blob=new Blob([new XMLSerializer().serializeToString(s)],{{type:'image/svg+xml'}}),u=URL.createObjectURL(blob),i=new Image;i.onload=()=>{{const x=c.getContext('2d');x.setTransform(scale,0,0,scale,0,0);x.drawImage(i,0,0,w,h);URL.revokeObjectURL(u);t.disabled=false;d.disabled=false}};i.onerror=()=>URL.revokeObjectURL(u);i.src=u;t.onclick=()=>{{const show=c.hidden;c.hidden=!show;s.hidden=show;t.setAttribute('aria-pressed',String(show));t.textContent=show?'Use SVG':'Use canvas'}};d.onclick=()=>{{const a=document.createElement('a');a.download='biolang-plot.png';a.href=c.toDataURL('image/png');a.click()}}}})();</script></body></html>"#
    )
}

pub(super) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

pub(super) fn svg_dimensions(svg: &str) -> Result<(f64, f64)> {
    let opening = svg.find('>').map(|index| &svg[..=index]).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() received malformed SVG",
            None,
        )
    })?;
    let attribute = |name: &str| -> Option<f64> {
        let pattern = regex::Regex::new(&format!(r#"\b{name}="([0-9]+(?:\.[0-9]+)?)""#)).ok()?;
        pattern
            .captures(opening)
            .and_then(|capture| capture.get(1))
            .and_then(|value| value.as_str().parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value > 0.0)
    };
    if let (Some(width), Some(height)) = (attribute("width"), attribute("height")) {
        return Ok((width, height));
    }
    let viewbox = regex::Regex::new(
        r#"\bviewBox="[-+0-9.eE]+\s+[-+0-9.eE]+\s+([-+0-9.eE]+)\s+([-+0-9.eE]+)""#,
    )
    .unwrap();
    let Some(capture) = viewbox.captures(opening) else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() SVG needs numeric width/height or a viewBox",
            None,
        ));
    };
    let width = capture[1].parse::<f64>().unwrap_or(f64::NAN);
    let height = capture[2].parse::<f64>().unwrap_or(f64::NAN);
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() SVG viewBox must have positive dimensions",
            None,
        ));
    }
    Ok((width, height))
}

pub(super) fn safe_nested_svg(svg: &str) -> Result<()> {
    let trimmed = svg.trim();
    let lowered = trimmed.to_ascii_lowercase();
    if !trimmed.starts_with("<svg") || !trimmed.ends_with("</svg>") {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() accepts SVG plot strings or PlotSpec records",
            None,
        ));
    }
    if lowered.contains("<script")
        || lowered.contains("<foreignobject")
        || lowered.contains("javascript:")
        || regex::Regex::new(r#"\son[a-z]+\s*="#)
            .unwrap()
            .is_match(&lowered)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "plot_grid() refuses active content inside SVG panels",
            None,
        ));
    }
    Ok(())
}

pub(super) fn without_child_axis_title(svg: &str, axis: &str) -> String {
    let pattern = regex::Regex::new(&format!(
        r#"(?i)<text\b[^>]*\bdata-biolang-axis-title="{axis}"[^>]*>[^<]*</text>"#
    ))
    .unwrap();
    pattern.replace_all(svg, "").into_owned()
}

pub(super) fn spreadsheet_panel_tag(mut index: usize) -> String {
    let mut tag = String::new();
    loop {
        tag.insert(0, (b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    tag
}
