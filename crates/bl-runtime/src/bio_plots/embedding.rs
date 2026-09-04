//! Embedding for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(super) fn nn_order(data: &[Vec<f64>]) -> Vec<usize> {
    let n = data.len();
    if n == 0 {
        return vec![];
    }
    let dist = |a: &[f64], b: &[f64]| -> f64 {
        let mut squared = 0.0;
        let mut compared = 0usize;
        for (&x, &y) in a.iter().zip(b.iter()) {
            if x.is_finite() && y.is_finite() {
                squared += (x - y).powi(2);
                compared += 1;
            }
        }
        if compared == 0 {
            f64::INFINITY
        } else {
            squared.sqrt()
        }
    };
    let mut visited = vec![false; n];
    let mut order = Vec::with_capacity(n);
    let mut cur = 0;
    visited[0] = true;
    order.push(0);
    for _ in 1..n {
        let mut best = None;
        let mut best_d = f64::INFINITY;
        for j in 0..n {
            if !visited[j] {
                let d = dist(&data[cur], &data[j]);
                if best.is_none() || d < best_d {
                    best_d = d;
                    best = Some(j);
                }
            }
        }
        let best = best.expect("an unvisited heatmap item must remain");
        visited[best] = true;
        order.push(best);
        cur = best;
    }
    order
}

// ── 12. pca_plot ────────────────────────────────────────────────

pub(super) fn render_pca_scores_svg(
    pc1: &[f64],
    pc2: &[f64],
    labels: Option<&[String]>,
    row_names: Option<&[String]>,
    pct1: f64,
    pct2: f64,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    // ggplot2 continuous scales expand five percent on either side. Besides
    // matching plotPCA(), this keeps point labels away from the panel border.
    let expand = |domain: (f64, f64)| {
        let span = (domain.1 - domain.0).abs();
        let padding = if span <= f64::EPSILON {
            domain.0.abs().max(1.0) * 0.05
        } else {
            span * 0.05
        };
        (domain.0 - padding, domain.1 + padding)
    };
    let xr = expand(col_range(pc1));
    let yr = expand(col_range(pc2));
    let w = get_opt_f64(opts, "width", 600.0);
    let h = get_opt_f64(opts, "height", 400.0);
    let title = get_opt_str(opts, "title", "PCA Plot").to_string();
    let mut canvas = themed_canvas(w, h, opts);
    let finite_rows = (0..pc1.len().min(pc2.len()))
        .filter(|&index| pc1[index].is_finite() && pc2[index].is_finite())
        .collect::<Vec<_>>();
    let mut colour_map: HashMap<String, usize> = HashMap::new();
    let mut next_colour = 0;
    if let Some(values) = labels {
        for &index in &finite_rows {
            if !colour_map.contains_key(&values[index]) {
                colour_map.insert(values[index].clone(), next_colour);
                next_colour += 1;
            }
        }
    }
    let colours = if get_opt_str(opts, "palette", "").eq_ignore_ascii_case("ggplot") {
        hue_palette(next_colour.max(1))
    } else {
        (0..next_colour.max(1))
            .map(|index| PALETTE[index % PALETTE.len()].to_string())
            .collect::<Vec<_>>()
    };
    let mut legend_entries = Vec::new();
    if let Some(values) = labels {
        for &index in &finite_rows {
            if !legend_entries.contains(&values[index]) {
                legend_entries.push(values[index].clone());
            }
        }
        let legend_title = get_opt_str(opts, "legend_title", "group");
        let widest = legend_entries
            .iter()
            .map(|entry| estimate_text_width(entry, 10.0))
            .chain(std::iter::once(estimate_text_width(legend_title, 10.0)))
            .fold(0.0, f64::max);
        canvas.margin.right = canvas
            .margin
            .right
            .max((widest + 42.0).clamp(92.0, w * 0.28));
    }
    let x_scale = Scale {
        domain: xr,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: yr,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    let mut points: Vec<(f64, f64, &str)> = Vec::with_capacity(finite_rows.len());
    for &index in &finite_rows {
        let colour_index = labels.map(|values| colour_map[&values[index]]).unwrap_or(0);
        points.push((
            x_scale.map(pc1[index]),
            y_scale.map(pc2[index]),
            colours[colour_index].as_str(),
        ));
    }
    let raster = raster_choice(opts, "pca_plot", finite_rows.len())?;
    let area = canvas.point_area();
    canvas.add_scatter(
        &points,
        get_opt_f64(opts, "point_radius", 4.0).clamp(1.0, 12.0),
        area,
        raster,
    );
    if let Some(names) = row_names {
        for &index in &finite_rows {
            canvas.add_text(
                x_scale.map(pc1[index]) + 6.0,
                y_scale.map(pc2[index]) - 4.0,
                &names[index],
                "start",
                8.0,
            );
        }
    }
    if labels.is_some() {
        let lx = canvas.margin.left + canvas.plot_width() + 22.0;
        let legend_title = get_opt_str(opts, "legend_title", "group");
        canvas.add_text(
            lx - 4.0,
            canvas.margin.top + 8.0,
            legend_title,
            "start",
            10.0,
        );
        for (index, name) in legend_entries.iter().enumerate() {
            let ly = canvas.margin.top + 26.0 + index as f64 * 18.0;
            canvas.add_circle(lx, ly, 4.0, &colours[index]);
            canvas.add_text(lx + 8.0, ly + 4.0, name, "start", 10.0);
        }
    }
    let x_label = opts
        .get("x_label")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("PC1 ({pct1:.1}%)"));
    let y_label = opts
        .get("y_label")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("PC2 ({pct2:.1}%)"));
    canvas.draw_x_axis(
        &Scale {
            domain: xr,
            range: xr,
        },
        &x_label,
    );
    canvas.draw_y_axis(
        &Scale {
            domain: yr,
            range: yr,
        },
        &y_label,
    );
    if opts
        .get("panel_border")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        canvas.draw_panel_border();
    }
    canvas.draw_title(&title);
    canvas.draw_subtitle(get_opt_str(opts, "subtitle", ""));
    canvas.draw_caption(get_opt_str(opts, "caption", ""));
    canvas.set_accessible_description(format!(
        "PCA score plot with {} rendered of {} observations; PC1 explains {pct1:.1}% and PC2 explains {pct2:.1}% of total variance.",
        finite_rows.len(),
        pc1.len().min(pc2.len())
    ));
    Ok(canvas.render())
}

pub(super) fn pca_plot_spec_value(
    pc1: &[f64],
    pc2: &[f64],
    labels: Option<&[String]>,
    row_names: Option<&[String]>,
    pct1: f64,
    pct2: f64,
    input_features: usize,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let point_count = pc1.len().min(pc2.len());
    let rendered_points = (0..point_count)
        .filter(|&index| pc1[index].is_finite() && pc2[index].is_finite())
        .count();
    let raster = raster_choice(opts, "pca_plot", rendered_points)?;
    let rows = (0..point_count)
        .map(|index| {
            vec![
                Value::Int(index as i64),
                Value::Float(pc1[index]),
                Value::Float(pc2[index]),
                labels
                    .and_then(|values| values.get(index))
                    .map(|value| Value::Str(value.clone()))
                    .unwrap_or(Value::Nil),
                row_names
                    .and_then(|values| values.get(index))
                    .map(|value| Value::Str(value.clone()))
                    .unwrap_or(Value::Nil),
            ]
        })
        .collect::<Vec<_>>();
    let data = Value::Table(Table::new(
        ["source_row", "pc1", "pc2", "group", "label"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        rows,
    ));
    let non_finite_coordinates = point_count - rendered_points;
    let warnings = if non_finite_coordinates == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{non_finite_coordinates} observations have non-finite PCA scores"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("pca".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "PCA Plot").into()),
            ),
            ("data".into(), data),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 600.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 400.0)),
                        ),
                        (
                            "title".into(),
                            Value::Str(get_opt_str(opts, "title", "PCA Plot").into()),
                        ),
                        ("pc1_variance_percent".into(), Value::Float(pct1)),
                        ("pc2_variance_percent".into(), Value::Float(pct2)),
                        ("has_groups".into(), Value::Bool(labels.is_some())),
                        ("has_labels".into(), Value::Bool(row_names.is_some())),
                        ("raster".into(), Value::Bool(raster.enabled)),
                        ("raster_scale".into(), Value::Float(raster.scale)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("pca_plot".into())),
                        ("input_rows".into(), Value::Int(point_count as i64)),
                        ("input_features".into(), Value::Int(input_features as i64)),
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

pub(crate) fn is_pca_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "pca")
    )
}

pub(crate) fn render_pca_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_pca_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 PCA Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in ["source_row", "pc1", "pc2", "group", "label"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() PCA data is missing '{required}'"),
                None,
            ));
        }
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA specification field 'options' must be Record",
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
    let pc1 = extract_table_col(table, "pc1")?;
    let pc2 = extract_table_col(table, "pc2")?;
    let groups = if options.get("has_groups").is_some_and(Value::is_truthy) {
        Some(extract_str_col(table, "group")?)
    } else {
        None
    };
    let labels = if options.get("has_labels").is_some_and(Value::is_truthy) {
        Some(extract_str_col(table, "label")?)
    } else {
        None
    };
    let pct1 = options
        .get("pc1_variance_percent")
        .and_then(Value::as_float)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA options are missing numeric 'pc1_variance_percent'",
                None,
            )
        })?;
    let pct2 = options
        .get("pc2_variance_percent")
        .and_then(Value::as_float)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() PCA options are missing numeric 'pc2_variance_percent'",
                None,
            )
        })?;
    let svg = render_pca_scores_svg(
        &pc1,
        &pc2,
        groups.as_deref(),
        labels.as_deref(),
        pct1,
        pct2,
        &options,
    )?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("PCA Plot");
    match format.as_str() {
        "svg" | "raw" => Ok(Value::Str(svg)),
        "html" | "canvas" => Ok(Value::Str(crate::plot::standalone_plot_html(&svg, title))),
        #[cfg(feature = "native")]
        "ascii" => {
            crate::plot::render_svg_terminal(&svg, 80, 24, crate::plot::TerminalPlotStyle::Ascii)
                .map(Value::Str)
                .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None))
        }
        #[cfg(feature = "native")]
        "unicode" | "braille" => {
            crate::plot::render_svg_terminal(&svg, 80, 24, crate::plot::TerminalPlotStyle::Braille)
                .map(Value::Str)
                .map_err(|error| BioLangError::runtime(ErrorKind::TypeError, error, None))
        }
        #[cfg(not(feature = "native"))]
        "ascii" | "unicode" | "braille" => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() terminal PCA output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown PCA format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

pub(super) fn builtin_pca_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    // umap_plot calls this `color_col`, this one calls it `group_col`, and
    // everyone types `color`. Accept all three rather than silently colouring
    // nothing when the caller guesses a sibling function's spelling.
    let group_col = ["group_col", "color_col", "color"]
        .iter()
        .map(|key| get_opt_str(&opts, key, ""))
        .find(|value| !value.is_empty())
        .unwrap_or("")
        .to_string();
    let show_labels = opts
        .get("labels")
        .and_then(|v| match v {
            Value::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);

    if opts
        .get("precomputed")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let Value::Table(table) = &args[0] else {
            return Err(BioLangError::type_error(
                "pca_plot() precomputed scores require a Table",
                None,
            ));
        };
        let pc1_col = get_opt_str(&opts, "pc1_col", "PC1");
        let pc2_col = get_opt_str(&opts, "pc2_col", "PC2");
        let pc1 = extract_table_col(table, pc1_col)?;
        let pc2 = extract_table_col(table, pc2_col)?;
        let labels = if !group_col.is_empty() && table.col_index(&group_col).is_some() {
            extract_str_col(table, &group_col).ok()
        } else {
            None
        };
        let row_names = if show_labels {
            let label_col = get_opt_str(&opts, "label_col", "");
            if label_col.is_empty() {
                None
            } else {
                Some(extract_str_col(table, label_col)?)
            }
        } else {
            None
        };
        let pct1 = get_opt_f64(&opts, "pc1_variance_percent", 0.0);
        let pct2 = get_opt_f64(&opts, "pc2_variance_percent", 0.0);
        if fmt == "svg" {
            return render_pca_scores_svg(
                &pc1,
                &pc2,
                labels.as_deref(),
                row_names.as_deref(),
                pct1,
                pct2,
                &opts,
            )
            .map(Value::Str);
        }
        let spec = pca_plot_spec_value(
            &pc1,
            &pc2,
            labels.as_deref(),
            row_names.as_deref(),
            pct1,
            pct2,
            2,
            &opts,
        )?;
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_pca_plot_spec_value(&spec, &opts);
    }

    // Extract numeric matrix and optional group labels
    let (data, nrow, ncol, labels, row_names) = match &args[0] {
        Value::Table(table) => {
            if table.num_rows() < 2 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "pca_plot() needs >= 2 rows",
                    None,
                ));
            }
            // Find numeric columns (exclude group_col)
            let mut num_cols: Vec<String> = Vec::new();
            for col in &table.columns {
                if col == &group_col {
                    continue;
                }
                let Some(index) = table.col_index(col) else {
                    continue;
                };
                let numeric = table.rows.iter().all(|row| match &row[index] {
                    Value::Int(_) | Value::Float(_) => true,
                    Value::Str(value) => value.parse::<f64>().is_ok(),
                    _ => false,
                });
                if numeric {
                    num_cols.push(col.clone());
                }
            }
            if num_cols.len() < 2 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "pca_plot() needs >= 2 numeric columns",
                    None,
                ));
            }
            let nrow = table.num_rows();
            let ncol = num_cols.len();
            let mut data = vec![0.0; nrow * ncol];
            for (ci, col) in num_cols.iter().enumerate() {
                let vals = extract_table_col(table, col)?;
                if vals.iter().any(|value| !value.is_finite()) {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!("pca_plot() numeric column '{col}' contains non-finite values"),
                        None,
                    ));
                }
                for (ri, &v) in vals.iter().enumerate() {
                    data[ri * ncol + ci] = v;
                }
            }
            let lbls = if !group_col.is_empty() && table.col_index(&group_col).is_some() {
                extract_str_col(table, &group_col).ok()
            } else {
                None
            };
            // Use first column as row names if it's a string column
            let rn: Option<Vec<String>> = if show_labels {
                table.columns.iter().find_map(|c| {
                    if c == &group_col {
                        return None;
                    }
                    let idx = table.col_index(c)?;
                    if matches!(&table.rows[0][idx], Value::Str(_)) {
                        Some(
                            table
                                .rows
                                .iter()
                                .map(|r| match &r[idx] {
                                    Value::Str(s) => s.clone(),
                                    other => format!("{other}"),
                                })
                                .collect(),
                        )
                    } else {
                        None
                    }
                })
            } else {
                None
            };
            (data, nrow, ncol, lbls, rn)
        }
        Value::Matrix(m) => {
            if m.ncol < 2 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "pca_plot() needs >= 2 columns",
                    None,
                ));
            }
            (m.data.clone(), m.nrow, m.ncol, None, m.row_names.clone())
        }
        _ => {
            return Err(BioLangError::type_error(
                "pca_plot() requires Table or Matrix",
                None,
            ))
        }
    };
    if data.iter().any(|value| !value.is_finite()) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "pca_plot() requires finite numeric values",
            None,
        ));
    }
    if nrow < 2 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "pca_plot() needs >= 2 rows",
            None,
        ));
    }

    // --- PCA via power iteration ---
    // 1. Center columns
    let mut centered = data.clone();
    for ci in 0..ncol {
        let mean = (0..nrow).map(|r| centered[r * ncol + ci]).sum::<f64>() / nrow as f64;
        for r in 0..nrow {
            centered[r * ncol + ci] -= mean;
        }
    }
    // 2. Compute covariance matrix (ncol x ncol)
    let mut cov = vec![0.0; ncol * ncol];
    for i in 0..ncol {
        for j in i..ncol {
            let val: f64 = (0..nrow)
                .map(|r| centered[r * ncol + i] * centered[r * ncol + j])
                .sum::<f64>()
                / (nrow - 1) as f64;
            cov[i * ncol + j] = val;
            cov[j * ncol + i] = val;
        }
    }
    // 3. Power iteration for top eigenvector
    let power_iter = |cov: &[f64], ncol: usize, deflate_vec: Option<&[f64]>| -> Vec<f64> {
        let mut v: Vec<f64> = (0..ncol)
            .map(|i| if i == 0 { 1.0 } else { 0.5 / (i as f64 + 1.0) })
            .collect();
        let norm = v.iter().map(|x| x * x).sum::<f64>().sqrt();
        for x in &mut v {
            *x /= norm;
        }
        let mut work_cov = cov.to_vec();
        if let Some(dv) = deflate_vec {
            // Deflate: C' = C - lambda * v * v^T (approximate lambda via Rayleigh quotient)
            let mut av = vec![0.0; ncol];
            for i in 0..ncol {
                av[i] = (0..ncol).map(|j| cov[i * ncol + j] * dv[j]).sum::<f64>();
            }
            let lambda: f64 = (0..ncol).map(|i| dv[i] * av[i]).sum();
            for i in 0..ncol {
                for j in 0..ncol {
                    work_cov[i * ncol + j] -= lambda * dv[i] * dv[j];
                }
            }
        }
        for _ in 0..200 {
            let mut new_v = vec![0.0; ncol];
            for i in 0..ncol {
                new_v[i] = (0..ncol)
                    .map(|j| work_cov[i * ncol + j] * v[j])
                    .sum::<f64>();
            }
            let n = new_v.iter().map(|x| x * x).sum::<f64>().sqrt();
            if n < 1e-15 {
                break;
            }
            for x in &mut new_v {
                *x /= n;
            }
            v = new_v;
        }
        v
    };
    let ev1 = power_iter(&cov, ncol, None);
    let ev2 = power_iter(&cov, ncol, Some(&ev1));
    // 4. Project data onto PC1 and PC2
    let pc1: Vec<f64> = (0..nrow)
        .map(|r| (0..ncol).map(|c| centered[r * ncol + c] * ev1[c]).sum())
        .collect();
    let pc2: Vec<f64> = (0..nrow)
        .map(|r| (0..ncol).map(|c| centered[r * ncol + c] * ev2[c]).sum())
        .collect();

    // Compute variance explained
    let total_var: f64 = (0..ncol).map(|i| cov[i * ncol + i]).sum();
    let var1: f64 = (0..ncol)
        .map(|i| {
            let a: f64 = (0..ncol).map(|j| cov[i * ncol + j] * ev1[j]).sum();
            a * ev1[i]
        })
        .sum();
    let var2: f64 = (0..ncol)
        .map(|i| {
            let a: f64 = (0..ncol).map(|j| cov[i * ncol + j] * ev2[j]).sum();
            a * ev2[i]
        })
        .sum();
    let pct1 = if total_var > 0.0 {
        var1 / total_var * 100.0
    } else {
        0.0
    };
    let pct2 = if total_var > 0.0 {
        var2 / total_var * 100.0
    } else {
        0.0
    };

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = pca_plot_spec_value(
            &pc1,
            &pc2,
            labels.as_deref(),
            if show_labels {
                row_names.as_deref()
            } else {
                None
            },
            pct1,
            pct2,
            ncol,
            &opts,
        )?;
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_pca_plot_spec_value(&spec, &opts);
    }

    if fmt == "svg" {
        return render_pca_scores_svg(
            &pc1,
            &pc2,
            labels.as_deref(),
            if show_labels {
                row_names.as_deref()
            } else {
                None
            },
            pct1,
            pct2,
            &opts,
        )
        .map(Value::Str);
    }

    let xr = col_range(&pc1);
    let yr = col_range(&pc2);

    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for j in 0..pc1.len() {
        chart.put(pc1[j], pc2[j], xr, yr, '●');
    }
    write_output(&chart.render(&format!("PCA Plot (PC1: {pct1:.1}%, PC2: {pct2:.1}%)")));
    Ok(Value::Nil)
}

// ── 13. oncoprint ───────────────────────────────────────────────

pub(super) fn expand_equal_aspect_domains(
    x: (f64, f64),
    y: (f64, f64),
    panel_width: f64,
    panel_height: f64,
) -> ((f64, f64), (f64, f64)) {
    let x_span = (x.1 - x.0).abs().max(1e-12);
    let y_span = (y.1 - y.0).abs().max(1e-12);
    let panel_ratio = panel_width.max(1.0) / panel_height.max(1.0);
    let domain_ratio = x_span / y_span;
    if domain_ratio < panel_ratio {
        let wanted = y_span * panel_ratio;
        let centre = (x.0 + x.1) / 2.0;
        ((centre - wanted / 2.0, centre + wanted / 2.0), y)
    } else {
        let wanted = x_span / panel_ratio;
        let centre = (y.0 + y.1) / 2.0;
        (x, (centre - wanted / 2.0, centre + wanted / 2.0))
    }
}

pub(super) fn finite_quantile(values: &[f64], probability: f64) -> Option<f64> {
    let mut finite = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect::<Vec<_>>();
    if finite.is_empty() {
        return None;
    }
    finite.sort_by(f64::total_cmp);
    Some(quantile_type7(&finite, probability.clamp(0.0, 1.0)))
}

/// FeaturePlot-compatible cutoff: a number, or `q05`/`q95` for a quantile.
pub(super) fn feature_cutoff(
    opts: &HashMap<String, Value>,
    key: &str,
    values: &[f64],
    fallback: f64,
) -> Result<f64> {
    match opts.get(key) {
        None => Ok(fallback),
        Some(value) if value.as_float().is_some_and(f64::is_finite) => {
            Ok(value.as_float().unwrap())
        }
        Some(Value::Str(value)) => {
            let lower = value.to_ascii_lowercase();
            let Some(percent) = lower.strip_prefix('q') else {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "umap_plot() option '{key}' must be a number or quantile such as 'q05'"
                    ),
                    None,
                ));
            };
            let probability = percent.parse::<f64>().map_err(|_| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("umap_plot() option '{key}' has an invalid quantile '{value}'"),
                    None,
                )
            })? / 100.0;
            if !(0.0..=1.0).contains(&probability) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("umap_plot() option '{key}' quantile must be between q00 and q100"),
                    None,
                ));
            }
            finite_quantile(values, probability).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("umap_plot() cannot apply '{key}' to an empty feature"),
                    None,
                )
            })
        }
        Some(_) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("umap_plot() option '{key}' must be a number or quantile such as 'q05'"),
            None,
        )),
    }
}

pub(super) fn group_legend_reserve(
    groups: &[String],
    height: f64,
    theme: PlotTheme,
) -> (usize, f64) {
    if groups.is_empty() || !theme.is_adaptive() {
        return (
            groups.len().max(1),
            if groups.is_empty() { 0.0 } else { 120.0 },
        );
    }
    let rows = (((height - 110.0).max(36.0) / 18.0).floor() as usize).max(1);
    let columns = groups.len().div_ceil(rows);
    let mut width = 0.0;
    for column in 0..columns {
        let start = column * rows;
        let end = (start + rows).min(groups.len());
        let widest = groups[start..end]
            .iter()
            .map(|label| estimate_text_width(label, theme.legend_size))
            .fold(0.0, f64::max);
        width += (widest + 34.0).clamp(76.0, 150.0);
    }
    (rows, width + 8.0)
}

pub(super) fn embedding_plot_spec_value(
    opts: &HashMap<String, Value>,
    xs: &[f64],
    ys: &[f64],
    groups: &[String],
    labels: &[String],
    feature_values: &[f64],
    has_feature: bool,
    feature_label: &str,
    feature_range: (f64, f64),
    publication_theme: bool,
) -> Result<Value> {
    let point_count = xs.len().min(ys.len());
    let renderable_points = (0..point_count)
        .filter(|&index| xs[index].is_finite() && ys[index].is_finite())
        .count();
    let raster = raster_choice(opts, "umap_plot", renderable_points)?;
    let mut draw_order: Vec<usize> = (0..point_count)
        .filter(|&index| xs[index].is_finite() && ys[index].is_finite())
        .collect();
    if has_feature && publication_theme {
        draw_order.sort_by(|&a, &b| {
            match (feature_values[a].is_finite(), feature_values[b].is_finite()) {
                (false, true) => std::cmp::Ordering::Less,
                (true, false) => std::cmp::Ordering::Greater,
                _ => feature_values[a].total_cmp(&feature_values[b]),
            }
        });
    }
    let mut draw_rank = vec![None; point_count];
    for (rank, &source_row) in draw_order.iter().enumerate() {
        draw_rank[source_row] = Some(rank);
    }

    let rows = (0..point_count)
        .map(|index| {
            vec![
                Value::Int(index as i64),
                draw_rank[index]
                    .map(|rank| Value::Int(rank as i64))
                    .unwrap_or(Value::Nil),
                Value::Float(xs[index]),
                Value::Float(ys[index]),
                Value::Str(groups.get(index).cloned().unwrap_or_default()),
                Value::Str(labels.get(index).cloned().unwrap_or_default()),
                if has_feature {
                    feature_values
                        .get(index)
                        .copied()
                        .filter(|value| value.is_finite())
                        .map(Value::Float)
                        .unwrap_or(Value::Nil)
                } else {
                    Value::Nil
                },
            ]
        })
        .collect::<Vec<_>>();
    let data = Value::Table(Table::new(
        [
            "source_row",
            "draw_rank",
            "x",
            "y",
            "group",
            "label",
            "feature",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        rows,
    ));

    let title = get_opt_str(opts, "title", "UMAP").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let theme_name = get_opt_str(opts, "theme", "biolang").to_string();
    let equal_aspect = match opts.get("aspect") {
        Some(Value::Str(value)) => value.eq_ignore_ascii_case("equal"),
        Some(Value::Bool(value)) => *value,
        _ => publication_theme,
    };
    let label_groups = opts
        .get("label_groups")
        .or_else(|| opts.get("label"))
        .is_some_and(Value::is_truthy);
    let mut replay_options = HashMap::from([
        ("title".into(), Value::Str(title.clone())),
        ("subtitle".into(), Value::Str(subtitle.clone())),
        ("caption".into(), Value::Str(caption.clone())),
        ("theme".into(), Value::Str(theme_name.clone())),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 600.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 450.0)),
        ),
        (
            "point_size".into(),
            Value::Float(get_opt_f64(opts, "point_size", 3.0).max(0.1)),
        ),
        (
            "aspect".into(),
            Value::Str(if equal_aspect { "equal" } else { "free" }.into()),
        ),
        ("label_groups".into(), Value::Bool(label_groups)),
        ("raster".into(), Value::Bool(raster.enabled)),
        ("raster_scale".into(), Value::Float(raster.scale)),
    ]);
    if groups.iter().any(|group| !group.is_empty()) {
        replay_options.insert("color_col".into(), Value::Str("group".into()));
    }
    if labels.iter().any(|label| !label.is_empty()) {
        replay_options.insert("label_col".into(), Value::Str("label".into()));
    }
    if has_feature {
        replay_options.insert("feature".into(), Value::Str("feature".into()));
        replay_options.insert(
            "feature_label".into(),
            Value::Str(feature_label.to_string()),
        );
        replay_options.insert("min_cutoff".into(), Value::Float(feature_range.0));
        replay_options.insert("max_cutoff".into(), Value::Float(feature_range.1));
    }
    for key in ["na_color", "xlab", "ylab"] {
        if let Some(value) = opts.get(key) {
            replay_options.insert(key.to_string(), value.clone());
        }
    }

    let non_finite_coordinates = xs
        .iter()
        .zip(ys.iter())
        .filter(|(x, y)| !x.is_finite() || !y.is_finite())
        .count();
    let provenance = Value::Record(
        HashMap::from([
            ("builtin".into(), Value::Str("umap_plot".into())),
            ("input_rows".into(), Value::Int(point_count as i64)),
            (
                "rendered_points".into(),
                Value::Int(renderable_points as i64),
            ),
            (
                "non_finite_coordinates".into(),
                Value::Int(non_finite_coordinates as i64),
            ),
            (
                "feature".into(),
                if has_feature {
                    Value::Str(feature_label.to_string())
                } else {
                    Value::Nil
                },
            ),
        ])
        .into(),
    );
    let warnings = if non_finite_coordinates == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{non_finite_coordinates} points have non-finite coordinates"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("embedding".into())),
            ("title".into(), Value::Str(title)),
            ("subtitle".into(), Value::Str(subtitle)),
            ("caption".into(), Value::Str(caption)),
            ("theme".into(), Value::Str(theme_name)),
            ("data".into(), data),
            ("options".into(), Value::Record(replay_options.into())),
            ("provenance".into(), provenance),
            ("warnings".into(), Value::List(warnings.into())),
        ])
        .into(),
    ))
}

pub(crate) fn is_embedding_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "embedding")
    )
}

pub(crate) fn render_embedding_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_embedding_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 embedding Record",
                None,
            ))
        }
    };
    let data = match map.get("data") {
        Some(Value::Table(table)) => Value::Table(table.clone()),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() embedding specification field 'data' must be Table",
                None,
            ))
        }
    };
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() embedding specification field 'options' must be Record",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "draw_rank",
        "x",
        "y",
        "group",
        "label",
        "feature",
    ] {
        let Value::Table(table) = &data else {
            unreachable!()
        };
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() embedding data is missing '{required}'"),
                None,
            ));
        }
    }

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
            options.insert(key.to_string(), override_value.clone());
        }
    }
    options.insert("format".into(), Value::Str("svg".into()));
    let svg = match builtin_umap_plot(vec![data, Value::Record(options.into())])? {
        Value::Str(svg) => svg,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() embedding renderer did not return SVG",
                None,
            ))
        }
    };
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Embedding");
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
            "render_plot() terminal embedding output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown embedding format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

pub(super) fn builtin_umap_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "UMAP").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let seurat_theme = get_opt_str(&opts, "theme", "") == "seurat";
    let label_groups = opts
        .get("label_groups")
        .or_else(|| opts.get("label"))
        .is_some_and(Value::is_truthy);
    let point_radius = get_opt_f64(&opts, "point_size", 3.0).max(0.1);
    // `color` is what everyone reaches for; `color_col` is the documented name.
    // Accepting only the latter meant `umap_plot(pts, { color: "cluster" })` was
    // silently ignored and every point came out one colour.
    let color_col = {
        let explicit = get_opt_str(&opts, "color_col", "");
        if explicit.is_empty() {
            get_opt_str(&opts, "color", "").to_string()
        } else {
            explicit.to_string()
        }
    };

    const CLUSTER_COLUMNS: [&str; 5] = ["cluster", "group", "label", "color", "cell_type"];

    let color_col = if color_col.is_empty() {
        // Auto-detect a cluster column. This used to run for Table only, so the
        // same data passed as a List of Records - which the extraction below
        // handles perfectly well - rendered every cell in one colour with no
        // error. On PBMC3k that was 1 colour for 11 clusters, and nothing said
        // so.
        match &args[0] {
            Value::Table(t) => CLUSTER_COLUMNS
                .iter()
                .find(|&&c| t.col_index(c).is_some())
                .map(|&s| s.to_string())
                .unwrap_or_default(),
            Value::List(items) => items
                .iter()
                .find_map(|item| match item {
                    Value::Record(map) => CLUSTER_COLUMNS
                        .iter()
                        .find(|&&c| map.contains_key(c))
                        .map(|&s| s.to_string()),
                    _ => None,
                })
                .unwrap_or_default(),
            _ => String::new(),
        }
    } else {
        color_col
    };
    let label_col = get_opt_str(&opts, "label_col", "").to_string();

    // Extract x, y, color_labels, point_labels from Table or List<Record>
    let (xs, ys, color_labels, point_labels): (Vec<f64>, Vec<f64>, Vec<String>, Vec<String>) =
        match &args[0] {
            Value::Table(table) => {
                let x_col = ["x", "UMAP1", "umap1", "PC1", "pc1", "tSNE1", "tsne1"]
                    .iter()
                    .find(|&&c| table.col_index(c).is_some())
                    .copied()
                    .unwrap_or("x");
                let y_col = ["y", "UMAP2", "umap2", "PC2", "pc2", "tSNE2", "tsne2"]
                    .iter()
                    .find(|&&c| table.col_index(c).is_some())
                    .copied()
                    .unwrap_or("y");
                let xs = extract_table_col(table, x_col).unwrap_or_default();
                let ys = extract_table_col(table, y_col).unwrap_or_default();
                let cls = if !color_col.is_empty() {
                    extract_str_col(table, &color_col)
                        .unwrap_or_else(|_| vec![String::new(); xs.len()])
                } else {
                    vec![String::new(); xs.len()]
                };
                let lbls = if !label_col.is_empty() {
                    extract_str_col(table, &label_col)
                        .unwrap_or_else(|_| vec![String::new(); xs.len()])
                } else {
                    vec![String::new(); xs.len()]
                };
                (xs, ys, cls, lbls)
            }
            Value::List(items) => {
                let mut xs = Vec::new();
                let mut ys = Vec::new();
                let mut cls = Vec::new();
                let mut lbls = Vec::new();
                for item in items.iter() {
                    if let Value::Record(map) = item {
                        let x = map
                            .get("x")
                            .or(map.get("UMAP1"))
                            .or(map.get("umap1"))
                            .and_then(|v| v.as_float())
                            .unwrap_or(0.0);
                        let y = map
                            .get("y")
                            .or(map.get("UMAP2"))
                            .or(map.get("umap2"))
                            .and_then(|v| v.as_float())
                            .unwrap_or(0.0);
                        xs.push(x);
                        ys.push(y);
                        let cl = if !color_col.is_empty() {
                            map.get(&color_col)
                                .map(|v| format!("{v}"))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        cls.push(cl);
                        let lb = if !label_col.is_empty() {
                            map.get(&label_col)
                                .map(|v| format!("{v}"))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        lbls.push(lb);
                    }
                }
                (xs, ys, cls, lbls)
            }
            _ => {
                return Err(BioLangError::type_error(
                    "umap_plot() requires Table or List of Records",
                    None,
                ))
            }
        };

    if xs.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "umap_plot() empty data",
            None,
        ));
    }
    let rendered_point_count = xs
        .iter()
        .zip(ys.iter())
        .filter(|(x, y)| x.is_finite() && y.is_finite())
        .count();
    if rendered_point_count == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "umap_plot() has no finite coordinate pairs",
            None,
        ));
    }

    // Continuous colouring: one gene's expression across the same points.
    //
    // This is Seurat's FeaturePlot, and it is how marker-based annotation is
    // actually taught - colour the embedding by LYZ and the monocyte island
    // lights up. Handled here rather than in a separate builtin so it reuses the
    // extraction, scaling and canvas work above; feature_plot() is an alias.
    let feature_col = get_opt_str(&opts, "feature", "").to_string();
    // The colour key is labelled with the column name by default. `feature_label`
    // separates the two so a caller need not rename its data field just to get a
    // readable legend — building 737k records with a computed key to do that
    // cost more than drawing the plot.
    let feature_label = {
        let explicit = get_opt_str(&opts, "feature_label", "").to_string();
        if explicit.is_empty() {
            feature_col.clone()
        } else {
            explicit
        }
    };
    let feature_values: Vec<f64> = if feature_col.is_empty() {
        Vec::new()
    } else {
        match &args[0] {
            Value::Table(table) => extract_table_col(table, &feature_col).unwrap_or_default(),
            Value::List(items) => items
                .iter()
                .map(|item| match item {
                    Value::Record(map) => map
                        .get(&feature_col)
                        .and_then(|v| v.as_float())
                        .unwrap_or(f64::NAN),
                    _ => f64::NAN,
                })
                .collect(),
            _ => Vec::new(),
        }
    };
    // A partial column would silently mis-colour points, so take it only when
    // there is a value for every point.
    let has_feature =
        feature_values.len() == xs.len() && feature_values.iter().any(|value| value.is_finite());
    let raw_feature_range = if has_feature {
        col_range(&feature_values)
    } else {
        (0.0, 1.0)
    };
    let feature_range = if has_feature {
        let lo = feature_cutoff(&opts, "min_cutoff", &feature_values, raw_feature_range.0)?;
        let hi = feature_cutoff(&opts, "max_cutoff", &feature_values, raw_feature_range.1)?;
        if lo > hi {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "umap_plot() min_cutoff must not exceed max_cutoff",
                None,
            ));
        }
        (lo, hi)
    } else {
        raw_feature_range
    };

    // Build group -> color index mapping
    let mut group_order: Vec<String> = Vec::new();
    let mut group_map: HashMap<String, usize> = HashMap::new();
    for (index, cl) in color_labels.iter().enumerate() {
        if !xs[index].is_finite() || !ys[index].is_finite() {
            continue;
        }
        if !cl.is_empty() && !group_map.contains_key(cl) {
            group_map.insert(cl.clone(), group_order.len());
            group_order.push(cl.clone());
        }
    }
    // A feature scale takes over the legend area, so the two never compete.
    let has_groups = !group_order.is_empty() && !has_feature;

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = embedding_plot_spec_value(
            &opts,
            &xs,
            &ys,
            &color_labels,
            &point_labels,
            &feature_values,
            has_feature,
            &feature_label,
            feature_range,
            publication_theme,
        )?;
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_embedding_plot_spec_value(&spec, &opts);
    }

    let xr = col_range(&xs);
    let yr = col_range(&ys);
    let xpad = (xr.1 - xr.0) * 0.05 + 0.1;
    let ypad = (yr.1 - yr.0) * 0.05 + 0.1;
    let xr = (xr.0 - xpad, xr.1 + xpad);
    let yr = (yr.0 - ypad, yr.1 + ypad);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 450.0);
        let mut c = SvgCanvas::with_theme(w, h, theme);
        let (legend_rows, group_legend_w) = group_legend_reserve(&group_order, h, theme);
        let feature_legend_w = if has_feature {
            if theme.is_adaptive() {
                (estimate_text_width(&feature_label, theme.legend_size) + 54.0).clamp(92.0, 170.0)
            } else {
                120.0
            }
        } else {
            0.0
        };
        let legend_w = if has_feature {
            feature_legend_w
        } else if has_groups {
            group_legend_w
        } else {
            0.0
        };
        c.fit_cartesian_layout(
            &Scale {
                domain: xr,
                range: xr,
            }
            .nice_ticks(5),
            &Scale {
                domain: yr,
                range: yr,
            }
            .nice_ticks(5),
            "UMAP 1",
            "UMAP 2",
            &title,
            &subtitle,
            &caption,
            legend_w,
        );
        // Legacy figures reserved a strip inside the default panel. Adaptive
        // themes reserve a true outer margin, so data and grid share one panel.
        let plot_right = if theme.is_adaptive() {
            c.margin.left + c.plot_width()
        } else {
            c.margin.left + c.plot_width() - legend_w
        };
        let equal_aspect = match opts.get("aspect") {
            Some(Value::Str(value)) => value.eq_ignore_ascii_case("equal"),
            Some(Value::Bool(value)) => *value,
            _ => publication_theme,
        };
        let (plot_xr, plot_yr) = if equal_aspect {
            expand_equal_aspect_domains(xr, yr, plot_right - c.margin.left, c.plot_height())
        } else {
            (xr, yr)
        };
        let domain_x = Scale {
            domain: plot_xr,
            range: plot_xr,
        };
        let domain_y = Scale {
            domain: plot_yr,
            range: plot_yr,
        };
        c.draw_cartesian_grid(&domain_x, &domain_y);
        c.set_accessible_description(format!(
            "Embedding with {} points, {} groups{}.",
            rendered_point_count,
            group_order.len(),
            if has_feature {
                format!(", coloured by {feature_label}")
            } else {
                String::new()
            }
        ));
        let x_scale = Scale {
            domain: plot_xr,
            range: (c.margin.left, plot_right),
        };
        let y_scale = Scale {
            domain: plot_yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };

        // One <circle> per cell, or one embedded raster for the lot. Vector
        // points are better when there are few: they hover, they select, they
        // scale to any zoom. The threshold and the reasoning behind it live
        // with `raster_choice`, so every scatter switches over at the same
        // size and explains itself the same way.
        let raster = raster_choice(&opts, "umap_plot", rendered_point_count)?;

        let point_color = |i: usize| -> String {
            if has_feature {
                let (lo, hi) = feature_range;
                if !feature_values[i].is_finite() {
                    return get_opt_str(
                        &opts,
                        "na_color",
                        if publication_theme {
                            "#d9dde3"
                        } else {
                            "#b8b8b8"
                        },
                    )
                    .to_string();
                }
                // A column with no spread would divide by zero; paint it mid-scale.
                let t = if (hi - lo).abs() < 1e-12 {
                    0.5
                } else {
                    (feature_values[i] - lo) / (hi - lo)
                };
                if seurat_theme {
                    seurat_feature_color(t)
                } else if publication_theme {
                    publication_sequential_color(t)
                } else {
                    sequential_color(t)
                }
            } else if has_groups {
                let ci = group_map.get(&color_labels[i]).copied().unwrap_or(0);
                if seurat_theme {
                    SEURAT_PALETTE[ci % SEURAT_PALETTE.len()].to_string()
                } else {
                    PALETTE[ci % PALETTE.len()].to_string()
                }
            } else {
                "#4e79a7".to_string()
            }
        };

        // Low values first, high values last: highly expressed cells remain
        // visible instead of being covered by input-order low-expression dots.
        let mut draw_order = (0..xs.len())
            .filter(|&index| xs[index].is_finite() && ys[index].is_finite())
            .collect::<Vec<_>>();
        if has_feature && publication_theme {
            draw_order.sort_by(|&a, &b| {
                match (feature_values[a].is_finite(), feature_values[b].is_finite()) {
                    (false, true) => std::cmp::Ordering::Less,
                    (true, false) => std::cmp::Ordering::Greater,
                    _ => feature_values[a].total_cmp(&feature_values[b]),
                }
            });
        }
        let points: Vec<(f64, f64, String)> = draw_order
            .iter()
            .map(|&i| (x_scale.map(xs[i]), y_scale.map(ys[i]), point_color(i)))
            .collect();
        // Not point_area(): the legend takes the right-hand strip, so the
        // points stop at plot_right rather than the full plot width.
        c.add_scatter(
            &points,
            point_radius,
            (
                c.margin.left,
                c.margin.top,
                plot_right - c.margin.left,
                c.plot_height(),
            ),
            raster,
        );

        // Point labels stay vector in both modes - they are text, and there are
        // never many of them.
        for i in 0..xs.len() {
            if xs[i].is_finite() && ys[i].is_finite() && !point_labels[i].is_empty() {
                c.add_text(
                    x_scale.map(xs[i]) + 4.0,
                    y_scale.map(ys[i]) - 4.0,
                    &point_labels[i],
                    "start",
                    7.0,
                );
            }
        }

        // Colour bar for a continuous feature: without a scale the reader can
        // see where expression is high but not how high.
        if has_feature {
            let lx = plot_right + 14.0;
            let bar_top = c.margin.top + 14.0;
            let bar_height = 120.0_f64;
            let steps = 40;
            for step in 0..steps {
                // Drawn top-down, so the top of the bar is the maximum.
                let t = 1.0 - (step as f64 / (steps - 1) as f64);
                let y = bar_top + (step as f64 / steps as f64) * bar_height;
                c.add_rect(
                    lx,
                    y,
                    12.0,
                    bar_height / steps as f64 + 0.6,
                    &if seurat_theme {
                        seurat_feature_color(t)
                    } else if publication_theme {
                        publication_sequential_color(t)
                    } else {
                        sequential_color(t)
                    },
                );
            }
            let (lo, hi) = feature_range;
            c.add_text(lx + 16.0, bar_top + 8.0, &format!("{hi:.2}"), "start", 9.0);
            c.add_text(
                lx + 16.0,
                bar_top + bar_height,
                &format!("{lo:.2}"),
                "start",
                9.0,
            );
            c.add_text(lx, bar_top - 6.0, &feature_label, "start", 10.0);
        }

        // Legend for groups
        if has_groups {
            let lx = plot_right + 12.0;
            let legend_draw_width = if theme.is_adaptive() {
                (c.margin.right - 20.0).max(40.0)
            } else {
                group_legend_w
            };
            for (gi, gname) in group_order.iter().enumerate() {
                let column = gi / legend_rows;
                let row = gi % legend_rows;
                let column_x = lx
                    + column as f64
                        * (legend_draw_width / group_order.len().div_ceil(legend_rows) as f64);
                let ly = c.margin.top + 10.0 + row as f64 * 18.0;
                let color = if seurat_theme {
                    SEURAT_PALETTE[gi % SEURAT_PALETTE.len()]
                } else {
                    PALETTE[gi % PALETTE.len()]
                };
                c.add_circle(column_x + 5.0, ly + 4.0, 4.0, color);
                c.add_text(column_x + 13.0, ly + 8.0, gname, "start", theme.legend_size);
            }
        }

        // Seurat's `label = TRUE`: place one readable label at the group
        // centre. Medians are less sensitive than means to stray UMAP points.
        if has_groups && label_groups {
            for group in &group_order {
                let mut group_x: Vec<f64> = color_labels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| {
                        (label == group && xs[i].is_finite() && ys[i].is_finite()).then_some(xs[i])
                    })
                    .collect();
                let mut group_y: Vec<f64> = color_labels
                    .iter()
                    .enumerate()
                    .filter_map(|(i, label)| {
                        (label == group && xs[i].is_finite() && ys[i].is_finite()).then_some(ys[i])
                    })
                    .collect();
                if group_x.is_empty() {
                    continue;
                }
                group_x.sort_by(f64::total_cmp);
                group_y.sort_by(f64::total_cmp);
                let x = x_scale.map(group_x[group_x.len() / 2]);
                let y = y_scale.map(group_y[group_y.len() / 2]);
                let box_width = 8.0 + estimate_text_width(group, theme.legend_size);
                c.add_rect(x - box_width / 2.0, y - 10.0, box_width, 16.0, "#ffffff");
                c.add_text(x, y + 2.0, group, "middle", theme.legend_size);
            }
        }

        // Axis labels
        let dx = Scale {
            domain: plot_xr,
            range: plot_xr,
        };
        let dy = Scale {
            domain: plot_yr,
            range: plot_yr,
        };
        // Explicit labels win; otherwise infer from the title. Inference alone
        // labelled a counts-vs-genes QC scatter "Dim 1"/"Dim 2", because this
        // builtin now draws any continuous-valued scatter, not just embeddings.
        let x_override = get_opt_str(&opts, "xlab", "").to_string();
        let y_override = get_opt_str(&opts, "ylab", "").to_string();
        let title_lc = title.to_lowercase();
        let x_label = if title_lc.contains("umap") {
            "UMAP 1"
        } else if title_lc.contains("pca") {
            "PC 1"
        } else if title_lc.contains("tsne") || title_lc.contains("t-sne") {
            "t-SNE 1"
        } else {
            "Dim 1"
        };
        let y_label = if title_lc.contains("umap") {
            "UMAP 2"
        } else if title_lc.contains("pca") {
            "PC 2"
        } else if title_lc.contains("tsne") || title_lc.contains("t-sne") {
            "t-SNE 2"
        } else {
            "Dim 2"
        };
        let x_label = if x_override.is_empty() {
            x_label
        } else {
            x_override.as_str()
        };
        let y_label = if y_override.is_empty() {
            y_label
        } else {
            y_override.as_str()
        };
        c.draw_x_axis(&dx, x_label);
        c.draw_y_axis(&dy, y_label);
        c.draw_title(&title);
        c.draw_subtitle(&subtitle);
        c.draw_caption(&caption);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 20);
    let mut chart = AsciiChart::new(width, height);
    for i in 0..xs.len() {
        if !xs[i].is_finite() || !ys[i].is_finite() {
            continue;
        }
        let ch = if has_groups {
            let ci = group_map.get(&color_labels[i]).copied().unwrap_or(0);
            char::from_digit((ci % 10) as u32, 10).unwrap_or('*')
        } else {
            '*'
        };
        chart.put(xs[i], ys[i], xr, yr, ch);
    }
    let n = rendered_point_count;
    write_output(&chart.render(&format!("{title}  ({n} points)")));
    Ok(Value::Nil)
}

// ── coverage_track ───────────────────────────────────────────────
