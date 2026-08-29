//! Genomic for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

pub(super) fn manhattan_plot_spec_value(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let position_column = get_opt_str(opts, "pos", "pos");
    let p_column = get_opt_str(opts, "p", "pvalue");
    let label_column = opts.get("label").and_then(Value::as_str);
    let highlight_column = opts.get("highlight").and_then(Value::as_str);
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let positions = extract_table_col(table, position_column)?;
    let p_values = extract_table_col(table, p_column)?;
    let threshold = get_opt_f64(opts, "threshold", 5e-8);
    if !threshold.is_finite() || !(0.0..=1.0).contains(&threshold) || threshold == 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "manhattan() threshold must be finite and within (0, 1]",
            None,
        ));
    }
    if p_values
        .iter()
        .any(|p| !p.is_finite() || *p <= 0.0 || *p > 1.0)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "manhattan() p-values must be finite and within (0, 1]",
            None,
        ));
    }
    let (genome_positions, spans) = checked_genome_layout(&chromosomes, &positions, "manhattan")?;
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span.index))
        .collect::<HashMap<_, _>>();
    let label_index = label_column.and_then(|name| table.col_index(name));
    if label_column.is_some() && label_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", label_column.unwrap()),
            None,
        ));
    }
    let highlight_index = highlight_column.and_then(|name| table.col_index(name));
    if highlight_column.is_some() && highlight_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", highlight_column.unwrap()),
            None,
        ));
    }
    let negative_log_p = p_values
        .iter()
        .map(|value| -value.log10())
        .collect::<Vec<_>>();
    let raster = raster_choice(opts, "manhattan", p_values.len())?;
    let thin = thin_requested(opts, "manhattan")?;
    let rows = (0..p_values.len())
        .map(|index| {
            vec![
                Value::Int(index as i64),
                Value::Int(span_lookup[chromosomes[index].as_str()] as i64),
                Value::Str(chromosomes[index].clone()),
                Value::Float(positions[index]),
                Value::Float(genome_positions[index]),
                Value::Float(p_values[index]),
                Value::Float(negative_log_p[index]),
                Value::Bool(p_values[index] <= threshold),
                Value::Bool(
                    highlight_index.is_some_and(|column| table.rows[index][column].is_truthy()),
                ),
                label_index
                    .map(|column| Value::Str(format!("{}", table.rows[index][column])))
                    .unwrap_or(Value::Nil),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Manhattan Plot");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("manhattan".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "position",
                        "genome_position",
                        "p_value",
                        "neg_log10_p",
                        "significant",
                        "highlighted",
                        "label",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 1200.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 400.0)),
                        ),
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
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Chromosome").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "-log10(p)").into()),
                        ),
                        ("threshold".into(), Value::Float(threshold)),
                        ("threshold_y".into(), Value::Float(-threshold.log10())),
                        ("raster".into(), Value::Bool(raster.enabled)),
                        ("raster_scale".into(), Value::Float(raster.scale)),
                        ("thin".into(), Value::Bool(thin)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("manhattan".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "chromosome_order".into(),
                            Value::Str("first_observed".into()),
                        ),
                        ("chromosome_gap_fraction".into(), Value::Float(0.02)),
                        ("p_transform".into(), Value::Str("-log10".into())),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("position_column".into(), Value::Str(position_column.into())),
                        ("p_column".into(), Value::Str(p_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

pub(super) fn chromosome_spans_from_spec(
    map: &HashMap<String, Value>,
    family: &str,
) -> Result<Vec<ChromosomeSpan>> {
    let table = match map.get("chromosomes") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} specification field 'chromosomes' must be Table"),
                None,
            ))
        }
    };
    for required in ["chromosome_index", "chromosome", "offset", "length"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} chromosomes are missing '{required}'"),
                None,
            ));
        }
    }
    let index_column = table.col_index("chromosome_index").unwrap();
    let name_column = table.col_index("chromosome").unwrap();
    let offset_column = table.col_index("offset").unwrap();
    let length_column = table.col_index("length").unwrap();
    let mut spans: Vec<ChromosomeSpan> = Vec::with_capacity(table.num_rows());
    for (expected, row) in table.rows.iter().enumerate() {
        let index = frozen_nonnegative_integer(&row[index_column], family, "chromosome_index")?;
        let name = row[name_column].as_str().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} chromosome must be Str"),
                None,
            )
        })?;
        let offset = row[offset_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        if index != expected
            || name.is_empty()
            || !offset.is_finite()
            || !length.is_finite()
            || length <= 0.0
            || (expected == 0 && offset.abs() > 1e-10)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() {family} chromosome layout is malformed"),
                None,
            ));
        }
        if let Some(previous) = spans.last() {
            let expected_offset = previous.offset + previous.length * 1.02;
            if (offset - expected_offset).abs() > 1e-8 * expected_offset.abs().max(1.0) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() {family} chromosome spans must use the frozen 2% gap"),
                    None,
                ));
            }
        }
        for (column, expected_value) in [
            ("start", offset),
            ("end", offset + length),
            ("midpoint", offset + length / 2.0),
        ] {
            if let Some(column_index) = table.col_index(column) {
                let observed = row[column_index].as_float().unwrap_or(f64::NAN);
                if !observed.is_finite()
                    || (observed - expected_value).abs() > 1e-8 * expected_value.abs().max(1.0)
                {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "render_plot() {family} chromosome field '{column}' is inconsistent"
                        ),
                        None,
                    ));
                }
            }
        }
        spans.push(ChromosomeSpan {
            index,
            name: name.into(),
            offset,
            length,
        });
    }
    if spans.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("render_plot() {family} needs chromosomes"),
            None,
        ));
    }
    Ok(spans)
}

pub(super) fn render_manhattan_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let genome_positions = extract_table_col(table, "genome_position")?;
    let negative_log_p = extract_table_col(table, "neg_log10_p")?;
    let chromosome_indices = extract_table_col(table, "chromosome_index")?;
    let highlighted_column = table.col_index("highlighted");
    let width = get_opt_f64(opts, "width", 1200.0);
    let height = get_opt_f64(opts, "height", 400.0);
    let title = get_opt_str(opts, "title", "Manhattan Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Chromosome");
    let y_label = get_opt_str(opts, "ylabel", "-log10(p)");
    let threshold_y = get_opt_f64(opts, "threshold_y", -5e-8f64.log10());
    let genome_end = spans.last().unwrap().end();
    let x_padding = (genome_end * 0.01).max(1e-9);
    let x_domain = (-x_padding, genome_end + x_padding);
    let y_max = negative_log_p
        .iter()
        .copied()
        .fold(threshold_y.max(1.0), f64::max)
        * 1.05;
    let y_domain = (0.0, y_max);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[0.0, y_max],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(threshold_y),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(threshold_y),
        "#d62728",
        1.1,
    );
    let raster = crate::plot::RasterChoice {
        enabled: opts.get("raster").is_some_and(Value::is_truthy),
        scale: get_opt_f64(opts, "raster_scale", 2.0),
    };
    let points = (0..negative_log_p.len())
        .map(|index| {
            let chromosome_index = chromosome_indices[index] as usize;
            (
                x_scale.map(genome_positions[index]),
                y_scale.map(negative_log_p[index]),
                PALETTE[chromosome_index % PALETTE.len()],
            )
        })
        .collect::<Vec<_>>();
    let area = canvas.point_area();
    let thin = opts.get("thin").is_some_and(Value::is_truthy);
    let drawn = if thin {
        let coordinates = points.iter().map(|&(x, y, _)| (x, y)).collect::<Vec<_>>();
        let grid = if raster.enabled { raster.scale } else { 1.0 };
        let kept = thin_to_pixel_grid(&coordinates, area, grid, &negative_log_p);
        let survivors = kept.iter().map(|&index| points[index]).collect::<Vec<_>>();
        canvas.add_scatter(&survivors, 2.5, area, raster);
        survivors.len()
    } else {
        canvas.add_scatter(&points, 2.5, area, raster);
        points.len()
    };
    if let Some(column) = highlighted_column {
        for (index, row) in table.rows.iter().enumerate() {
            if row[column].is_truthy() {
                canvas.add_circle(points[index].0, points[index].1, 4.5, "#d62728");
            }
        }
    }
    draw_genome_axis(&mut canvas, spans, x_domain, x_label);
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let significant = negative_log_p
        .iter()
        .filter(|value| **value >= threshold_y)
        .count();
    if drawn < points.len() {
        canvas.set_accessible_description(format!(
            "Manhattan plot, thinned to one variant per pixel: {drawn} of {} variants drawn, the most significant in each pixel. Point density does not indicate variant count. {significant} variants meet the significance threshold.",
            points.len()
        ));
        canvas.add_text(
            canvas.margin.left,
            canvas.height - 6.0,
            &format!(
                "thinned: {drawn} of {} variants drawn (most significant per pixel)",
                points.len()
            ),
            "start",
            9.0,
        );
    } else {
        canvas.set_accessible_description(format!(
            "Manhattan plot with {} variants across {} chromosomes; {significant} meet the significance threshold.",
            points.len(), spans.len()
        ));
    }
    Ok(canvas.render())
}

pub(crate) fn is_manhattan_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "manhattan"))
}

pub(crate) fn render_manhattan_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_manhattan_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 Manhattan Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() Manhattan specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "chromosome_index",
        "chromosome",
        "position",
        "genome_position",
        "p_value",
        "neg_log10_p",
        "significant",
        "highlighted",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() Manhattan data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() Manhattan specification has no variants",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "Manhattan")?;
    let mut options = frozen_spec_options(map, render_options, "Manhattan")?;
    let threshold = get_opt_f64(&options, "threshold", f64::NAN);
    let threshold_y = get_opt_f64(&options, "threshold_y", f64::NAN);
    let chromosome_column = table.col_index("chromosome_index").unwrap();
    let chromosome_name_column = table.col_index("chromosome").unwrap();
    let position_column = table.col_index("position").unwrap();
    let genome_column = table.col_index("genome_position").unwrap();
    let p_column = table.col_index("p_value").unwrap();
    let transformed_column = table.col_index("neg_log10_p").unwrap();
    let significant_column = table.col_index("significant").unwrap();
    let highlighted_column = table.col_index("highlighted").unwrap();
    for (expected, row) in table.rows.iter().enumerate() {
        if frozen_nonnegative_integer(
            &row[table.col_index("source_row").unwrap()],
            "Manhattan",
            "source_row",
        )? != expected
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() Manhattan source rows must be contiguous",
                None,
            ));
        }
        let chromosome_index =
            frozen_nonnegative_integer(&row[chromosome_column], "Manhattan", "chromosome_index")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let genome_position = row[genome_column].as_float().unwrap_or(f64::NAN);
        let p_value = row[p_column].as_float().unwrap_or(f64::NAN);
        let transformed = row[transformed_column].as_float().unwrap_or(f64::NAN);
        if chromosome_index >= spans.len()
            || !position.is_finite()
            || position < 0.0
            || position > spans[chromosome_index].length
            || (genome_position - (spans[chromosome_index].offset + position)).abs() > 1e-8
            || !p_value.is_finite()
            || p_value <= 0.0
            || p_value > 1.0
            || (transformed + p_value.log10()).abs() > 1e-10
            || !matches!(row[significant_column], Value::Bool(_))
            || row[significant_column].is_truthy() != (p_value <= threshold)
            || !matches!(row[highlighted_column], Value::Bool(_))
            || row[chromosome_name_column].as_str() != Some(spans[chromosome_index].name.as_str())
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() Manhattan frozen geometry is inconsistent",
                None,
            ));
        }
    }
    if !threshold.is_finite()
        || !threshold_y.is_finite()
        || (threshold_y + threshold.log10()).abs() > 1e-10
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() Manhattan threshold metadata is inconsistent",
            None,
        ));
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    if let Some(width) = render_options.get("width") {
        options.insert("width".into(), width.clone());
    }
    if let Some(height) = render_options.get("height") {
        options.insert("height".into(), height.clone());
    }
    let svg = render_manhattan_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Manhattan Plot");
    finish_frozen_bio_plot(value, render_options, title, "Manhattan", svg)
}

pub(super) fn builtin_manhattan(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "manhattan")?;
    let opts = parse_options(&args);
    let specification = manhattan_plot_spec_value(table, &opts)?;
    render_manhattan_plot_spec_value(&specification, &opts)
}

// ── 2. qq_plot ──────────────────────────────────────────────────

pub(super) fn genetic_qq_plot_spec_value(
    values: Vec<f64>,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let input_count = values.len();
    let mut p_values = values
        .into_iter()
        .filter(|value| value.is_finite() && *value > 0.0 && *value <= 1.0)
        .collect::<Vec<_>>();
    p_values.sort_by(|left, right| left.total_cmp(right));
    if p_values.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "qq_plot() needs p-values within (0, 1]",
            None,
        ));
    }
    let confidence = get_opt_f64(opts, "confidence", 0.95);
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "qq_plot() confidence must be within (0, 1)",
            None,
        ));
    }
    // Exact beta-order-statistic intervals are intentionally opt-in. At
    // genome-wide scale, evaluating two beta quantiles per test can dominate
    // the entire plot; the p-value positions and lambda GC remain O(n log n).
    let envelope = bio_plot_bool_option(opts, "envelope", false, "qq_plot")?;
    let count = p_values.len();
    let tail = (1.0 - confidence) / 2.0;
    let mut rows = Vec::with_capacity(count);
    for (index, &p_value) in p_values.iter().enumerate() {
        let rank = index + 1;
        let expected_p = (rank as f64 - 0.5) / count as f64;
        let (envelope_lower, envelope_upper) = if envelope {
            let beta_alpha = rank as f64;
            let beta_beta = (count - rank + 1) as f64;
            let lower_p = beta_quantile(tail, beta_alpha, beta_beta).max(f64::MIN_POSITIVE);
            let upper_p = beta_quantile(1.0 - tail, beta_alpha, beta_beta).max(f64::MIN_POSITIVE);
            (-upper_p.log10(), -lower_p.log10())
        } else {
            let expected = -expected_p.log10();
            (expected, expected)
        };
        rows.push(vec![
            Value::Int(rank as i64),
            Value::Float(p_value),
            Value::Float(expected_p),
            Value::Float(-expected_p.log10()),
            Value::Float(-p_value.log10()),
            Value::Float(envelope_lower),
            Value::Float(envelope_upper),
        ]);
    }
    let mut chi_square = p_values
        .iter()
        .map(|p_value| {
            let z = bl_core::bio_core::stats_ops::normal_quantile(p_value / 2.0);
            z * z
        })
        .collect::<Vec<_>>();
    chi_square.sort_by(f64::total_cmp);
    let lambda_gc = quantile_type7(&chi_square, 0.5) / 0.454_936_423_119_572_7;
    let raster = raster_choice(opts, "qq_plot", count)?;
    let title = get_opt_str(opts, "title", "Genetic Q-Q Plot");
    let dropped = input_count - count;
    let warnings = if dropped == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{dropped} values outside finite (0, 1] were excluded"
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("genetic_qq".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "rank",
                        "p_value",
                        "expected_p",
                        "expected_neg_log10_p",
                        "observed_neg_log10_p",
                        "envelope_lower",
                        "envelope_upper",
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
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 600.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 600.0)),
                        ),
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
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Expected -log10(p)").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "Observed -log10(p)").into()),
                        ),
                        ("confidence".into(), Value::Float(confidence)),
                        ("envelope".into(), Value::Bool(envelope)),
                        ("lambda_gc".into(), Value::Float(lambda_gc)),
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
                        ("builtin".into(), Value::Str("qq_plot".into())),
                        ("input_values".into(), Value::Int(input_count as i64)),
                        ("retained_values".into(), Value::Int(count as i64)),
                        ("dropped_values".into(), Value::Int(dropped as i64)),
                        (
                            "expected_positions".into(),
                            Value::Str("(rank - 0.5) / n".into()),
                        ),
                        (
                            "envelope_distribution".into(),
                            Value::Str("beta_order_statistic".into()),
                        ),
                        (
                            "lambda_gc_denominator".into(),
                            Value::Float(0.454_936_423_119_572_7),
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

pub(super) fn render_genetic_qq_svg(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let expected = extract_table_col(table, "expected_neg_log10_p")?;
    let observed = extract_table_col(table, "observed_neg_log10_p")?;
    let envelope_lower = extract_table_col(table, "envelope_lower")?;
    let envelope_upper = extract_table_col(table, "envelope_upper")?;
    let width = get_opt_f64(opts, "width", 600.0);
    let height = get_opt_f64(opts, "height", 600.0);
    let title = get_opt_str(opts, "title", "Genetic Q-Q Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Expected -log10(p)");
    let y_label = get_opt_str(opts, "ylabel", "Observed -log10(p)");
    let max_value = expected
        .iter()
        .chain(observed.iter())
        .chain(envelope_upper.iter())
        .copied()
        .fold(1.0, f64::max)
        * 1.05;
    let domain = (0.0, max_value);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.fit_cartesian_layout(
        &[0.0, max_value],
        &[0.0, max_value],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    if opts.get("envelope").is_some_and(Value::is_truthy) {
        let mut polygon = expected
            .iter()
            .zip(envelope_upper.iter())
            .map(|(x, y)| format!("{:.2},{:.2}", x_scale.map(*x), y_scale.map(*y)))
            .collect::<Vec<_>>();
        polygon.extend(
            expected
                .iter()
                .zip(envelope_lower.iter())
                .rev()
                .map(|(x, y)| format!("{:.2},{:.2}", x_scale.map(*x), y_scale.map(*y))),
        );
        canvas.elements.push(format!(
            r##"<polygon points="{}" fill="#4e79a7" fill-opacity="0.14" stroke="none" />"##,
            polygon.join(" ")
        ));
    }
    canvas.add_line(
        x_scale.map(0.0),
        y_scale.map(0.0),
        x_scale.map(max_value),
        y_scale.map(max_value),
        "#6b7280",
        1.0,
    );
    let raster = crate::plot::RasterChoice {
        enabled: opts.get("raster").is_some_and(Value::is_truthy),
        scale: get_opt_f64(opts, "raster_scale", 2.0),
    };
    let points = expected
        .iter()
        .zip(observed.iter())
        .map(|(x, y)| (x_scale.map(*x), y_scale.map(*y), PALETTE[0]))
        .collect::<Vec<_>>();
    let area = canvas.point_area();
    canvas.add_scatter(&points, 3.0, area, raster);
    canvas.draw_x_axis(
        &Scale {
            domain,
            range: domain,
        },
        x_label,
    );
    canvas.draw_y_axis(
        &Scale {
            domain,
            range: domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let lambda = get_opt_f64(opts, "lambda_gc", f64::NAN);
    canvas.add_text(
        canvas.margin.left + canvas.plot_width() - 6.0,
        canvas.margin.top + 18.0,
        &format!("λGC = {lambda:.3}"),
        "end",
        canvas.theme.legend_size,
    );
    let envelope_description = if opts.get("envelope").is_some_and(Value::is_truthy) {
        "an exact beta order-statistic confidence envelope"
    } else {
        "no confidence envelope"
    };
    canvas.set_accessible_description(format!(
        "Genetic p-value Q-Q plot with {} tests, {envelope_description}, and genomic inflation factor lambda GC {lambda:.3}.",
        points.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_genetic_qq_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "genetic_qq"))
}

pub(crate) fn render_genetic_qq_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_genetic_qq_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 genetic Q-Q Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genetic Q-Q specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "rank",
        "p_value",
        "expected_p",
        "expected_neg_log10_p",
        "observed_neg_log10_p",
        "envelope_lower",
        "envelope_upper",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() genetic Q-Q data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genetic Q-Q specification has no p-values",
            None,
        ));
    }
    let rank_column = table.col_index("rank").unwrap();
    let p_column = table.col_index("p_value").unwrap();
    let expected_p_column = table.col_index("expected_p").unwrap();
    let expected_column = table.col_index("expected_neg_log10_p").unwrap();
    let observed_column = table.col_index("observed_neg_log10_p").unwrap();
    let lower_column = table.col_index("envelope_lower").unwrap();
    let upper_column = table.col_index("envelope_upper").unwrap();
    let count = table.num_rows();
    let options = frozen_spec_options(map, render_options, "genetic Q-Q")?;
    let confidence = get_opt_f64(&options, "confidence", f64::NAN);
    let envelope = options.get("envelope").is_some_and(Value::is_truthy);
    if !confidence.is_finite() || confidence <= 0.0 || confidence >= 1.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genetic Q-Q confidence must be within (0, 1)",
            None,
        ));
    }
    let tail = (1.0 - confidence) / 2.0;
    let mut previous_p = 0.0;
    let mut p_values = Vec::with_capacity(count);
    for (index, row) in table.rows.iter().enumerate() {
        let rank = frozen_nonnegative_integer(&row[rank_column], "genetic Q-Q", "rank")?;
        let p_value = row[p_column].as_float().unwrap_or(f64::NAN);
        let expected_p = row[expected_p_column].as_float().unwrap_or(f64::NAN);
        let expected = row[expected_column].as_float().unwrap_or(f64::NAN);
        let observed = row[observed_column].as_float().unwrap_or(f64::NAN);
        let lower = row[lower_column].as_float().unwrap_or(f64::NAN);
        let upper = row[upper_column].as_float().unwrap_or(f64::NAN);
        let frozen_expected_p = (index as f64 + 0.5) / count as f64;
        let (frozen_lower, frozen_upper) = if envelope {
            let rank = index + 1;
            let lower_p =
                beta_quantile(tail, rank as f64, (count - rank + 1) as f64).max(f64::MIN_POSITIVE);
            let upper_p = beta_quantile(1.0 - tail, rank as f64, (count - rank + 1) as f64)
                .max(f64::MIN_POSITIVE);
            (-upper_p.log10(), -lower_p.log10())
        } else {
            let expected = -frozen_expected_p.log10();
            (expected, expected)
        };
        if rank != index + 1
            || !p_value.is_finite()
            || p_value <= 0.0
            || p_value > 1.0
            || (index > 0 && p_value < previous_p)
            || (expected_p - frozen_expected_p).abs() > 1e-12
            || (expected + expected_p.log10()).abs() > 1e-10
            || (observed + p_value.log10()).abs() > 1e-10
            || !lower.is_finite()
            || !upper.is_finite()
            || lower > upper
            || (lower - frozen_lower).abs() > 2e-9
            || (upper - frozen_upper).abs() > 2e-9
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genetic Q-Q frozen geometry is inconsistent",
                None,
            ));
        }
        previous_p = p_value;
        p_values.push(p_value);
    }
    let lambda = get_opt_f64(&options, "lambda_gc", f64::NAN);
    let mut chi_square = p_values
        .iter()
        .map(|p_value| {
            let z = bl_core::bio_core::stats_ops::normal_quantile(p_value / 2.0);
            z * z
        })
        .collect::<Vec<_>>();
    chi_square.sort_by(f64::total_cmp);
    let frozen_lambda = quantile_type7(&chi_square, 0.5) / 0.454_936_423_119_572_7;
    if !lambda.is_finite() || lambda < 0.0 || (lambda - frozen_lambda).abs() > 2e-9 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genetic Q-Q lambda_gc must be finite and non-negative",
            None,
        ));
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_genetic_qq_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Genetic Q-Q Plot");
    finish_frozen_bio_plot(value, render_options, title, "genetic Q-Q", svg)
}

pub(super) fn builtin_qq_plot(args: Vec<Value>) -> Result<Value> {
    let values = nums_from_value(&args[0], "qq_plot")?;
    let opts = parse_options(&args);
    let specification = genetic_qq_plot_spec_value(values, &opts)?;
    render_genetic_qq_plot_spec_value(&specification, &opts)
}

// ── 3. ideogram ─────────────────────────────────────────────────

pub(super) fn cytoband_class(stain: &str) -> &'static str {
    let stain = stain.to_ascii_lowercase();
    if stain.contains("acen") {
        "acen"
    } else if stain.contains("gpos100") {
        "gpos100"
    } else if stain.contains("gpos75") {
        "gpos75"
    } else if stain.contains("gpos50") {
        "gpos50"
    } else if stain.contains("gpos25") {
        "gpos25"
    } else if stain.contains("gvar") {
        "gvar"
    } else if stain.contains("stalk") {
        "stalk"
    } else if stain.contains("gneg") {
        "gneg"
    } else {
        "unknown"
    }
}

pub(super) fn cytoband_color(class: &str) -> &'static str {
    match class {
        "acen" => "#ef4444",
        "gpos100" => "#111827",
        "gpos75" => "#52525b",
        "gpos50" => "#9297a1",
        "gpos25" => "#d1d5db",
        "gvar" => "#c4b5fd",
        "stalk" => "#60a5fa",
        "gneg" => "#f8fafc",
        _ => "#e5e7eb",
    }
}

pub(super) fn ideogram_plot_spec_value(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let start_column = get_opt_str(opts, "start", "start");
    let end_column = get_opt_str(opts, "end", "end");
    let stain_column = if let Some(column) = opts.get("stain").and_then(Value::as_str) {
        Some(column)
    } else if table.col_index("stain").is_some() {
        Some("stain")
    } else if table.col_index("gieStain").is_some() {
        Some("gieStain")
    } else {
        None
    };
    let band_column = if let Some(column) = opts.get("band").and_then(Value::as_str) {
        Some(column)
    } else if table.col_index("band").is_some() {
        Some("band")
    } else if table.col_index("name").is_some() {
        Some("name")
    } else {
        None
    };
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let starts = extract_table_col(table, start_column)?;
    let ends = extract_table_col(table, end_column)?;
    if chromosomes.is_empty()
        || starts.len() != chromosomes.len()
        || ends.len() != chromosomes.len()
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ideogram() requires equally sized, non-empty chromosome/start/end columns",
            None,
        ));
    }
    if starts
        .iter()
        .zip(&ends)
        .any(|(&start, &end)| !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ideogram() intervals must be finite, non-negative and have end > start",
            None,
        ));
    }
    let (_, spans) = checked_genome_layout(&chromosomes, &ends, "ideogram")?;
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span.index))
        .collect::<HashMap<_, _>>();
    let stain_index = stain_column.and_then(|column| table.col_index(column));
    if stain_column.is_some() && stain_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", stain_column.unwrap()),
            None,
        ));
    }
    let band_index = band_column.and_then(|column| table.col_index(column));
    if band_column.is_some() && band_index.is_none() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{}' not found", band_column.unwrap()),
            None,
        ));
    }
    if stain_index.is_some_and(|column| {
        table
            .rows
            .iter()
            .any(|row| !matches!(row[column], Value::Str(_)))
    }) || band_index.is_some_and(|column| {
        table
            .rows
            .iter()
            .any(|row| !matches!(row[column], Value::Str(_)))
    }) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "ideogram() stain and band values must be Str",
            None,
        ));
    }
    let mut order = (0..table.num_rows()).collect::<Vec<_>>();
    order.sort_by(|&left, &right| {
        span_lookup[chromosomes[left].as_str()]
            .cmp(&span_lookup[chromosomes[right].as_str()])
            .then_with(|| starts[left].total_cmp(&starts[right]))
            .then_with(|| ends[left].total_cmp(&ends[right]))
            .then_with(|| left.cmp(&right))
    });
    let rows = order
        .into_iter()
        .map(|source_row| {
            let stain = stain_index
                .and_then(|column| table.rows[source_row][column].as_str())
                .unwrap_or("gneg");
            let band = band_index
                .and_then(|column| table.rows[source_row][column].as_str())
                .unwrap_or("");
            vec![
                Value::Int(source_row as i64),
                Value::Int(span_lookup[chromosomes[source_row].as_str()] as i64),
                Value::Str(chromosomes[source_row].clone()),
                Value::Float(starts[source_row]),
                Value::Float(ends[source_row]),
                Value::Float(ends[source_row] - starts[source_row]),
                Value::Str(band.into()),
                Value::Str(stain.into()),
                Value::Str(cytoband_class(stain).into()),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Chromosome Ideogram");
    let default_height = (spans.len() as f64 * 28.0 + 105.0).max(180.0);
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("ideogram".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "start",
                        "end",
                        "length",
                        "band",
                        "stain",
                        "stain_class",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 900.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", default_height)),
                        ),
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
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Position").into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("ideogram".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "chromosome_order".into(),
                            Value::Str("first_observed".into()),
                        ),
                        (
                            "band_order".into(),
                            Value::Str("chromosome_start_end_source_row".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open".into()),
                        ),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("start_column".into(), Value::Str(start_column.into())),
                        ("end_column".into(), Value::Str(end_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

pub(super) fn render_ideogram_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 900.0);
    let height = get_opt_f64(
        opts,
        "height",
        (spans.len() as f64 * 28.0 + 105.0).max(180.0),
    );
    let title = get_opt_str(opts, "title", "Chromosome Ideogram");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Position");
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.left = 78.0;
    canvas.margin.right = 24.0;
    canvas.margin.top = if subtitle.is_empty() { 48.0 } else { 68.0 };
    canvas.margin.bottom = if caption.is_empty() { 43.0 } else { 62.0 };
    let local_maximum = spans.iter().map(|span| span.length).fold(1.0, f64::max);
    let x_scale = Scale {
        domain: (0.0, local_maximum),
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let row_height = (canvas.plot_height() / spans.len() as f64)
        .min(24.0)
        .max(10.0);
    let band_height = (row_height * 0.62).clamp(8.0, 15.0);
    let chromosome_column = table.col_index("chromosome_index").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let class_column = table.col_index("stain_class").unwrap();
    let clip_rectangles = spans
        .iter()
        .map(|span| {
            let y = canvas.margin.top
                + span.index as f64 * row_height
                + (row_height - band_height) / 2.0;
            format!(
                r#"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" />"#,
                x_scale.map(0.0),
                y,
                (x_scale.map(span.length) - x_scale.map(0.0)).max(1.0),
                band_height,
                band_height / 2.0
            )
        })
        .collect::<String>();
    canvas.elements.push(format!(
        r#"<defs><clipPath id="biolang-cytoband-clip">{clip_rectangles}</clipPath></defs>"#
    ));
    let mut paths = HashMap::<String, String>::new();
    for row in &table.rows {
        let chromosome_index = row[chromosome_column].as_float().unwrap_or(0.0) as usize;
        let start = row[start_column].as_float().unwrap_or(0.0);
        let end = row[end_column].as_float().unwrap_or(0.0);
        let class = row[class_column].as_str().unwrap_or("unknown");
        let y = canvas.margin.top
            + chromosome_index as f64 * row_height
            + (row_height - band_height) / 2.0;
        let x1 = x_scale.map(start);
        let x2 = x_scale.map(end);
        paths.entry(class.into()).or_default().push_str(&format!(
            "M{:.2},{:.2}H{:.2}V{:.2}H{:.2}Z",
            x1,
            y,
            x2,
            y + band_height,
            x1
        ));
    }
    for class in [
        "gneg", "gpos25", "gpos50", "gpos75", "gpos100", "acen", "gvar", "stalk", "unknown",
    ] {
        if let Some(path) = paths.get(class) {
            canvas.elements.push(format!(
                r##"<path d="{path}" fill="{}" stroke="none" clip-path="url(#biolang-cytoband-clip)" />"##,
                cytoband_color(class)
            ));
        }
    }
    for span in spans {
        let y =
            canvas.margin.top + span.index as f64 * row_height + (row_height - band_height) / 2.0;
        canvas.add_text(
            canvas.margin.left - 9.0,
            y + band_height / 2.0 + 4.0,
            &span.name,
            "end",
            canvas.theme.tick_size,
        );
        canvas.elements.push(format!(
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" rx="{:.2}" fill="none" stroke="#475569" stroke-width="0.8" />"##,
            x_scale.map(0.0),
            y,
            (x_scale.map(span.length) - x_scale.map(0.0)).max(1.0),
            band_height,
            band_height / 2.0
        ));
    }
    let axis = Scale {
        domain: (0.0, local_maximum),
        range: (0.0, local_maximum),
    };
    canvas.draw_x_axis(&axis, x_label);
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Chromosome ideogram with {} cytobands across {} chromosomes; chromosome lengths share one coordinate scale.",
        table.num_rows(), spans.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_ideogram_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "ideogram"))
}

pub(crate) fn render_ideogram_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_ideogram_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 ideogram Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ideogram specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "chromosome_index",
        "chromosome",
        "start",
        "end",
        "length",
        "stain",
        "stain_class",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() ideogram data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() ideogram specification has no bands",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "ideogram")?;
    let source_column = table.col_index("source_row").unwrap();
    let index_column = table.col_index("chromosome_index").unwrap();
    let name_column = table.col_index("chromosome").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let length_column = table.col_index("length").unwrap();
    let stain_column = table.col_index("stain").unwrap();
    let class_column = table.col_index("stain_class").unwrap();
    let mut previous: Option<(usize, f64, f64, usize)> = None;
    for row in &table.rows {
        let source = frozen_nonnegative_integer(&row[source_column], "ideogram", "source_row")?;
        let index = frozen_nonnegative_integer(&row[index_column], "ideogram", "chromosome_index")?;
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        let name = row[name_column].as_str().unwrap_or("");
        let stain = row[stain_column].as_str().unwrap_or("");
        let class = row[class_column].as_str().unwrap_or("");
        let key = (index, start, end, source);
        if index >= spans.len()
            || name != spans[index].name
            || !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || end <= start
            || end > spans[index].length + 1e-8
            || (length - (end - start)).abs() > 1e-10
            || class != cytoband_class(stain)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() ideogram frozen geometry is inconsistent",
                None,
            ));
        }
        previous = Some(key);
    }
    let options = frozen_spec_options(map, render_options, "ideogram")?;
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_ideogram_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Chromosome Ideogram");
    finish_frozen_bio_plot(value, render_options, title, "ideogram", svg)
}

pub(super) fn builtin_ideogram(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "ideogram")?;
    let opts = parse_options(&args);
    let specification = ideogram_plot_spec_value(table, &opts)?;
    render_ideogram_plot_spec_value(&specification, &opts)
}

// ── 4. rainfall ─────────────────────────────────────────────────

pub(super) fn rainfall_plot_spec_value(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<Option<Value>> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let position_column = get_opt_str(opts, "pos", "pos");
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let positions = extract_table_col(table, position_column)?;
    let (_, spans) = checked_genome_layout(&chromosomes, &positions, "rainfall")?;
    let duplicate_floor = get_opt_f64(opts, "duplicate_floor", 1.0);
    if !duplicate_floor.is_finite() || duplicate_floor <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "rainfall() duplicate_floor must be finite and positive",
            None,
        ));
    }
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span))
        .collect::<HashMap<_, _>>();
    let mut indices = (0..positions.len()).collect::<Vec<_>>();
    indices.sort_by(|left, right| {
        span_lookup[chromosomes[*left].as_str()]
            .index
            .cmp(&span_lookup[chromosomes[*right].as_str()].index)
            .then(positions[*left].total_cmp(&positions[*right]))
            .then(left.cmp(right))
    });
    let mut rows = Vec::new();
    for pair in indices.windows(2) {
        let previous = pair[0];
        let current = pair[1];
        if chromosomes[previous] != chromosomes[current] {
            continue;
        }
        let distance = positions[current] - positions[previous];
        let plotted_distance = distance.max(duplicate_floor);
        let span = span_lookup[chromosomes[current].as_str()];
        rows.push(vec![
            Value::Int(rows.len() as i64),
            Value::Int(current as i64),
            Value::Int(span.index as i64),
            Value::Str(chromosomes[current].clone()),
            Value::Float(positions[current]),
            Value::Float(positions[previous]),
            Value::Float(span.offset + positions[current]),
            Value::Float(distance),
            Value::Float(plotted_distance),
            Value::Float(plotted_distance.log10()),
            Value::Bool(distance == 0.0),
        ]);
    }
    if rows.is_empty() {
        return Ok(None);
    }
    let raster = raster_choice(opts, "rainfall", rows.len())?;
    let short_distance = get_opt_f64(opts, "short_distance", 1_000.0);
    let medium_distance = get_opt_f64(opts, "medium_distance", 100_000.0);
    if !short_distance.is_finite()
        || !medium_distance.is_finite()
        || short_distance <= 0.0
        || medium_distance <= short_distance
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "rainfall() distance colour boundaries must be finite, positive, and short_distance < medium_distance",
            None,
        ));
    }
    let title = get_opt_str(opts, "title", "Rainfall Plot");
    Ok(Some(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("rainfall".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "position",
                        "previous_position",
                        "genome_position",
                        "distance",
                        "plotted_distance",
                        "log10_distance",
                        "duplicate_position",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 1000.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 400.0)),
                        ),
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
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Chromosome").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "log10(distance in bp)").into()),
                        ),
                        ("duplicate_floor".into(), Value::Float(duplicate_floor)),
                        ("short_distance".into(), Value::Float(short_distance)),
                        ("medium_distance".into(), Value::Float(medium_distance)),
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
                        ("builtin".into(), Value::Str("rainfall".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "distance_points".into(),
                            Value::Int((table.num_rows() - spans.len()) as i64),
                        ),
                        (
                            "sort".into(),
                            Value::Str(
                                "first_observed_chromosome_then_position_then_source_row".into(),
                            ),
                        ),
                        (
                            "distance_scope".into(),
                            Value::Str("within_chromosome".into()),
                        ),
                        ("distance_transform".into(), Value::Str("log10".into())),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("position_column".into(), Value::Str(position_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )))
}

pub(super) fn render_rainfall_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let genome_positions = extract_table_col(table, "genome_position")?;
    let log_distance = extract_table_col(table, "log10_distance")?;
    let width = get_opt_f64(opts, "width", 1000.0);
    let height = get_opt_f64(opts, "height", 400.0);
    let title = get_opt_str(opts, "title", "Rainfall Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Chromosome");
    let y_label = get_opt_str(opts, "ylabel", "log10(distance in bp)");
    let genome_end = spans.last().unwrap().end();
    let x_padding = (genome_end * 0.01).max(1e-9);
    let x_domain = (-x_padding, genome_end + x_padding);
    let (mut y_min, mut y_max) = col_range(&log_distance);
    if (y_max - y_min).abs() < 1e-12 {
        y_min -= 0.5;
        y_max += 0.5;
    }
    let y_domain = (y_min, y_max);
    let theme = plot_theme(opts);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[y_min, y_max],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    let short_log = get_opt_f64(opts, "short_distance", 1_000.0).log10();
    let medium_log = get_opt_f64(opts, "medium_distance", 100_000.0).log10();
    let points = genome_positions
        .iter()
        .zip(log_distance.iter())
        .map(|(x, y)| {
            let colour = if *y < short_log {
                "#d62728"
            } else if *y < medium_log {
                "#e69f00"
            } else {
                "#2a9d8f"
            };
            (x_scale.map(*x), y_scale.map(*y), colour)
        })
        .collect::<Vec<_>>();
    let raster = crate::plot::RasterChoice {
        enabled: opts.get("raster").is_some_and(Value::is_truthy),
        scale: get_opt_f64(opts, "raster_scale", 2.0),
    };
    let area = canvas.point_area();
    canvas.add_scatter(&points, 2.5, area, raster);
    draw_genome_axis(&mut canvas, spans, x_domain, x_label);
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let duplicate_column = table.col_index("duplicate_position").unwrap();
    let duplicates = table
        .rows
        .iter()
        .filter(|row| row[duplicate_column].is_truthy())
        .count();
    canvas.set_accessible_description(format!(
        "Rainfall plot with {} within-chromosome inter-variant distances across {} chromosomes; {duplicates} duplicate-position distances use the declared display floor.",
        points.len(), spans.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_rainfall_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "rainfall"))
}

pub(crate) fn render_rainfall_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_rainfall_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 rainfall Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() rainfall specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "chromosome_index",
        "chromosome",
        "position",
        "previous_position",
        "genome_position",
        "distance",
        "plotted_distance",
        "log10_distance",
        "duplicate_position",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() rainfall data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() rainfall specification has no distances",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "rainfall")?;
    let options = frozen_spec_options(map, render_options, "rainfall")?;
    let duplicate_floor = get_opt_f64(&options, "duplicate_floor", f64::NAN);
    let short_distance = get_opt_f64(&options, "short_distance", f64::NAN);
    let medium_distance = get_opt_f64(&options, "medium_distance", f64::NAN);
    if !duplicate_floor.is_finite()
        || duplicate_floor <= 0.0
        || !short_distance.is_finite()
        || !medium_distance.is_finite()
        || short_distance <= 0.0
        || medium_distance <= short_distance
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() rainfall distance floors and colour boundaries are invalid",
            None,
        ));
    }
    let point_column = table.col_index("point_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let chromosome_column = table.col_index("chromosome_index").unwrap();
    let chromosome_name_column = table.col_index("chromosome").unwrap();
    let position_column = table.col_index("position").unwrap();
    let previous_column = table.col_index("previous_position").unwrap();
    let genome_column = table.col_index("genome_position").unwrap();
    let distance_column = table.col_index("distance").unwrap();
    let plotted_column = table.col_index("plotted_distance").unwrap();
    let log_column = table.col_index("log10_distance").unwrap();
    let duplicate_column = table.col_index("duplicate_position").unwrap();
    let mut previous_key: Option<(usize, f64, usize)> = None;
    for (expected, row) in table.rows.iter().enumerate() {
        let chromosome_index =
            frozen_nonnegative_integer(&row[chromosome_column], "rainfall", "chromosome_index")?;
        let source_row = frozen_nonnegative_integer(&row[source_column], "rainfall", "source_row")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let previous = row[previous_column].as_float().unwrap_or(f64::NAN);
        let genome_position = row[genome_column].as_float().unwrap_or(f64::NAN);
        let distance = row[distance_column].as_float().unwrap_or(f64::NAN);
        let plotted = row[plotted_column].as_float().unwrap_or(f64::NAN);
        let log_distance = row[log_column].as_float().unwrap_or(f64::NAN);
        if frozen_nonnegative_integer(&row[point_column], "rainfall", "point_index")? != expected
            || chromosome_index >= spans.len()
            || !position.is_finite()
            || !previous.is_finite()
            || position < previous
            || (distance - (position - previous)).abs() > 1e-10
            || (plotted - distance.max(duplicate_floor)).abs() > 1e-10
            || (log_distance - plotted.log10()).abs() > 1e-10
            || !matches!(row[duplicate_column], Value::Bool(_))
            || row[duplicate_column].is_truthy() != (distance == 0.0)
            || (genome_position - (spans[chromosome_index].offset + position)).abs() > 1e-8
            || row[chromosome_name_column].as_str() != Some(spans[chromosome_index].name.as_str())
            || previous_key.is_some_and(
                |(previous_chromosome, previous_position, previous_source)| {
                    chromosome_index < previous_chromosome
                        || (chromosome_index == previous_chromosome
                            && (position < previous_position
                                || (position == previous_position && source_row < previous_source)))
                },
            )
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() rainfall frozen geometry is inconsistent",
                None,
            ));
        }
        previous_key = Some((chromosome_index, position, source_row));
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_rainfall_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Rainfall Plot");
    finish_frozen_bio_plot(value, render_options, title, "rainfall", svg)
}

pub(super) fn builtin_rainfall(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "rainfall")?;
    let opts = parse_options(&args);
    let Some(specification) = rainfall_plot_spec_value(table, &opts)? else {
        write_output("  (insufficient data for rainfall plot)\n");
        return Ok(Value::Nil);
    };
    render_rainfall_plot_spec_value(&specification, &opts)
}

// ── 5. cnv_plot ─────────────────────────────────────────────────

pub(super) fn cnv_state(ratio: f64, loss_threshold: f64, gain_threshold: f64) -> &'static str {
    if ratio > gain_threshold {
        "gain"
    } else if ratio < loss_threshold {
        "loss"
    } else {
        "neutral"
    }
}

pub(super) fn cnv_plot_spec_value(table: &Table, opts: &HashMap<String, Value>) -> Result<Value> {
    let chromosome_column = get_opt_str(opts, "chrom", "chrom");
    let start_column = get_opt_str(opts, "start", "start");
    let end_column = get_opt_str(opts, "end", "end");
    let ratio_column = get_opt_str(opts, "ratio", "log2ratio");
    let chromosomes = extract_str_col(table, chromosome_column)?;
    let starts = extract_table_col(table, start_column)?;
    let ends = extract_table_col(table, end_column)?;
    let ratios = extract_table_col(table, ratio_column)?;
    if chromosomes.is_empty()
        || starts.len() != chromosomes.len()
        || ends.len() != chromosomes.len()
        || ratios.len() != chromosomes.len()
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cnv_plot() requires equally sized, non-empty chromosome/start/end/ratio columns",
            None,
        ));
    }
    if starts
        .iter()
        .zip(&ends)
        .zip(&ratios)
        .any(|((&start, &end), &ratio)| {
            !start.is_finite()
                || !end.is_finite()
                || !ratio.is_finite()
                || start < 0.0
                || end <= start
        })
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cnv_plot() intervals and ratios must be finite, positions non-negative, and end > start",
            None,
        ));
    }
    let loss_threshold = get_opt_f64(opts, "loss_threshold", -0.2);
    let gain_threshold = get_opt_f64(opts, "gain_threshold", 0.2);
    if !loss_threshold.is_finite()
        || !gain_threshold.is_finite()
        || loss_threshold >= gain_threshold
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "cnv_plot() requires finite loss_threshold < gain_threshold",
            None,
        ));
    }
    let (_, spans) = checked_genome_layout(&chromosomes, &ends, "cnv_plot")?;
    let span_lookup = spans
        .iter()
        .map(|span| (span.name.as_str(), span))
        .collect::<HashMap<_, _>>();
    let mut order = (0..table.num_rows()).collect::<Vec<_>>();
    order.sort_by(|&left, &right| {
        span_lookup[chromosomes[left].as_str()]
            .index
            .cmp(&span_lookup[chromosomes[right].as_str()].index)
            .then_with(|| starts[left].total_cmp(&starts[right]))
            .then_with(|| ends[left].total_cmp(&ends[right]))
            .then_with(|| left.cmp(&right))
    });
    let rows = order
        .into_iter()
        .map(|source_row| {
            let span = span_lookup[chromosomes[source_row].as_str()];
            let genome_start = span.offset + starts[source_row];
            let genome_end = span.offset + ends[source_row];
            vec![
                Value::Int(source_row as i64),
                Value::Int(span.index as i64),
                Value::Str(chromosomes[source_row].clone()),
                Value::Float(starts[source_row]),
                Value::Float(ends[source_row]),
                Value::Float(ends[source_row] - starts[source_row]),
                Value::Float(genome_start),
                Value::Float(genome_end),
                Value::Float((genome_start + genome_end) / 2.0),
                Value::Float(ratios[source_row]),
                Value::Str(cnv_state(ratios[source_row], loss_threshold, gain_threshold).into()),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Copy-number Profile");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("cnv".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "chromosome_index",
                        "chromosome",
                        "start",
                        "end",
                        "length",
                        "genome_start",
                        "genome_end",
                        "genome_midpoint",
                        "log2ratio",
                        "state",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("chromosomes".into(), chromosome_table(&spans)),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "width".into(),
                            Value::Float(get_opt_f64(opts, "width", 1100.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 380.0)),
                        ),
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
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        (
                            "xlabel".into(),
                            Value::Str(get_opt_str(opts, "xlabel", "Chromosome").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "log2 ratio").into()),
                        ),
                        ("loss_threshold".into(), Value::Float(loss_threshold)),
                        ("gain_threshold".into(), Value::Float(gain_threshold)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("cnv_plot".into())),
                        ("input_rows".into(), Value::Int(table.num_rows() as i64)),
                        (
                            "chromosome_order".into(),
                            Value::Str("first_observed".into()),
                        ),
                        ("chromosome_gap_fraction".into(), Value::Float(0.02)),
                        (
                            "segment_order".into(),
                            Value::Str("chromosome_start_end_source_row".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open".into()),
                        ),
                        (
                            "chromosome_column".into(),
                            Value::Str(chromosome_column.into()),
                        ),
                        ("start_column".into(), Value::Str(start_column.into())),
                        ("end_column".into(), Value::Str(end_column.into())),
                        ("ratio_column".into(), Value::Str(ratio_column.into())),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

pub(super) fn render_cnv_svg(
    table: &Table,
    spans: &[ChromosomeSpan],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let genome_start = extract_table_col(table, "genome_start")?;
    let genome_end = extract_table_col(table, "genome_end")?;
    let ratios = extract_table_col(table, "log2ratio")?;
    let state_column = table.col_index("state").unwrap();
    let width = get_opt_f64(opts, "width", 1100.0);
    let height = get_opt_f64(opts, "height", 380.0);
    let title = get_opt_str(opts, "title", "Copy-number Profile");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Chromosome");
    let y_label = get_opt_str(opts, "ylabel", "log2 ratio");
    let maximum = ratios
        .iter()
        .copied()
        .map(f64::abs)
        .fold(0.5, f64::max)
        .max(get_opt_f64(opts, "loss_threshold", -0.2).abs())
        .max(get_opt_f64(opts, "gain_threshold", 0.2).abs());
    let y_domain = (-maximum * 1.12, maximum * 1.12);
    let final_end = spans.last().map(ChromosomeSpan::end).unwrap_or(1.0);
    let pad = final_end * 0.01;
    let x_domain = (-pad, final_end + pad);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[y_domain.0, 0.0, y_domain.1],
        x_label,
        y_label,
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain: x_domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let y_scale = Scale {
        domain: y_domain,
        range: (canvas.margin.top + canvas.plot_height(), canvas.margin.top),
    };
    canvas.draw_cartesian_grid(&x_scale, &y_scale);
    for span in spans.iter().filter(|span| span.index % 2 == 1) {
        canvas.elements.push(format!(
            r##"<rect x="{:.2}" y="{:.2}" width="{:.2}" height="{:.2}" fill="#64748b" fill-opacity="0.035" />"##,
            x_scale.map(span.start()),
            canvas.margin.top,
            (x_scale.map(span.end()) - x_scale.map(span.start())).max(0.0),
            canvas.plot_height()
        ));
    }
    canvas.add_line(
        canvas.margin.left,
        y_scale.map(0.0),
        canvas.margin.left + canvas.plot_width(),
        y_scale.map(0.0),
        "#475569",
        1.0,
    );
    let mut paths = HashMap::<String, String>::new();
    for (index, row) in table.rows.iter().enumerate() {
        let state = row[state_column].as_str().unwrap_or("neutral");
        paths.entry(state.into()).or_default().push_str(&format!(
            "M{:.2},{:.2}H{:.2}",
            x_scale.map(genome_start[index]),
            y_scale.map(ratios[index]),
            x_scale.map(genome_end[index])
        ));
    }
    for state in ["neutral", "loss", "gain"] {
        if let Some(path) = paths.get(state) {
            let color = match state {
                "gain" => "#d73027",
                "loss" => "#4575b4",
                _ => "#6b7280",
            };
            canvas.elements.push(format!(
                r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="2.2" stroke-linecap="butt" />"#
            ));
        }
    }
    draw_genome_axis(&mut canvas, spans, x_domain, x_label);
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        y_label,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    let gain_count = table
        .rows
        .iter()
        .filter(|row| row[state_column].as_str() == Some("gain"))
        .count();
    let loss_count = table
        .rows
        .iter()
        .filter(|row| row[state_column].as_str() == Some("loss"))
        .count();
    canvas.set_accessible_description(format!(
        "Copy-number profile with {} segments across {} chromosomes: {gain_count} gain and {loss_count} loss segments.",
        table.num_rows(), spans.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_cnv_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "cnv"))
}

pub(crate) fn render_cnv_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_cnv_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 CNV Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() CNV specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "source_row",
        "chromosome_index",
        "chromosome",
        "start",
        "end",
        "length",
        "genome_start",
        "genome_end",
        "genome_midpoint",
        "log2ratio",
        "state",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() CNV data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() CNV specification has no segments",
            None,
        ));
    }
    let spans = chromosome_spans_from_spec(map, "CNV")?;
    let options = frozen_spec_options(map, render_options, "CNV")?;
    let loss_threshold = get_opt_f64(&options, "loss_threshold", f64::NAN);
    let gain_threshold = get_opt_f64(&options, "gain_threshold", f64::NAN);
    if !loss_threshold.is_finite()
        || !gain_threshold.is_finite()
        || loss_threshold >= gain_threshold
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() CNV thresholds are inconsistent",
            None,
        ));
    }
    let source_column = table.col_index("source_row").unwrap();
    let index_column = table.col_index("chromosome_index").unwrap();
    let name_column = table.col_index("chromosome").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let length_column = table.col_index("length").unwrap();
    let genome_start_column = table.col_index("genome_start").unwrap();
    let genome_end_column = table.col_index("genome_end").unwrap();
    let midpoint_column = table.col_index("genome_midpoint").unwrap();
    let ratio_column = table.col_index("log2ratio").unwrap();
    let state_column = table.col_index("state").unwrap();
    let mut previous: Option<(usize, f64, f64, usize)> = None;
    for row in &table.rows {
        let source = frozen_nonnegative_integer(&row[source_column], "CNV", "source_row")?;
        let index = frozen_nonnegative_integer(&row[index_column], "CNV", "chromosome_index")?;
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        let genome_start = row[genome_start_column].as_float().unwrap_or(f64::NAN);
        let genome_end = row[genome_end_column].as_float().unwrap_or(f64::NAN);
        let midpoint = row[midpoint_column].as_float().unwrap_or(f64::NAN);
        let ratio = row[ratio_column].as_float().unwrap_or(f64::NAN);
        let state = row[state_column].as_str().unwrap_or("");
        let name = row[name_column].as_str().unwrap_or("");
        let key = (index, start, end, source);
        if index >= spans.len()
            || name != spans[index].name
            || !start.is_finite()
            || !end.is_finite()
            || !ratio.is_finite()
            || start < 0.0
            || end <= start
            || end > spans[index].length + 1e-8
            || (length - (end - start)).abs() > 1e-10
            || (genome_start - (spans[index].offset + start)).abs() > 1e-8
            || (genome_end - (spans[index].offset + end)).abs() > 1e-8
            || (midpoint - (genome_start + genome_end) / 2.0).abs() > 1e-8
            || state != cnv_state(ratio, loss_threshold, gain_threshold)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() CNV frozen geometry is inconsistent",
                None,
            ));
        }
        previous = Some(key);
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_cnv_svg(table, &spans, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Copy-number Profile");
    finish_frozen_bio_plot(value, render_options, title, "CNV", svg)
}

pub(super) fn builtin_cnv_plot(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "cnv_plot")?;
    let opts = parse_options(&args);
    let specification = cnv_plot_spec_value(table, &opts)?;
    render_cnv_plot_spec_value(&specification, &opts)
}

// ── 6. violin ───────────────────────────────────────────────────
