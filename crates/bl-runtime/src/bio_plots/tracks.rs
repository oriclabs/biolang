//! Tracks for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct GenomeFeatureDatum {
    source_row: usize,
    chromosome: Option<String>,
    original_start: f64,
    original_end: f64,
    start: f64,
    end: f64,
    name: Option<String>,
    strand: String,
    lane: usize,
    label_drawn: bool,
}

pub(super) fn optional_string_cell(
    row: &[Value],
    column: Option<usize>,
    family: &str,
) -> Result<Option<String>> {
    match column.map(|index| &row[index]) {
        None | Some(Value::Nil) => Ok(None),
        Some(Value::Str(value)) => Ok((!value.is_empty()).then(|| value.clone())),
        Some(_) => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{family}() optional text columns must contain strings or nil"),
            None,
        )),
    }
}

pub(super) fn assign_interval_lanes<T>(
    items: &mut [T],
    start: impl Fn(&T) -> f64,
    end: impl Fn(&T) -> f64,
    set_lane: impl Fn(&mut T, usize),
) -> usize {
    let mut lane_ends: Vec<f64> = Vec::new();
    for item in items {
        let item_start = start(item);
        let lane = lane_ends
            .iter()
            .position(|lane_end| *lane_end <= item_start)
            .unwrap_or_else(|| {
                lane_ends.push(f64::NEG_INFINITY);
                lane_ends.len() - 1
            });
        lane_ends[lane] = end(item);
        set_lane(item, lane);
    }
    lane_ends.len().max(1)
}

pub(super) fn genome_track_spec_value(
    value: &Value,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let table = require_table_bp(value, "genome_track")?;
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() empty data",
            None,
        ));
    }
    let start_column = table.col_index("start").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'start' not found", None)
    })?;
    let end_column = table.col_index("end").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'end' not found", None)
    })?;
    let chromosome_column = table
        .col_index("chrom")
        .or_else(|| table.col_index("chromosome"));
    let name_column = table.col_index(get_opt_str(opts, "label", "name"));
    let strand_column = table.col_index("strand");
    let mut features = Vec::with_capacity(table.num_rows());
    for (source_row, row) in table.rows.iter().enumerate() {
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        if !start.is_finite() || !end.is_finite() || start < 0.0 || end <= start {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "genome_track() start/end must be finite, non-negative, and end > start",
                None,
            ));
        }
        let chromosome = optional_string_cell(row, chromosome_column, "genome_track")?;
        let name = optional_string_cell(row, name_column, "genome_track")?;
        let strand = optional_string_cell(row, strand_column, "genome_track")?.unwrap_or_default();
        if !matches!(strand.as_str(), "" | "+" | "-" | ".") {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "genome_track() strand must be '+', '-', '.', or nil",
                None,
            ));
        }
        features.push(GenomeFeatureDatum {
            source_row,
            chromosome,
            original_start: start,
            original_end: end,
            start,
            end,
            name,
            strand,
            lane: 0,
            label_drawn: false,
        });
    }
    let input_rows = features.len();
    let requested_chromosome = opts.get("chromosome").and_then(Value::as_str);
    if let Some(chromosome) = requested_chromosome {
        if chromosome.trim().is_empty()
            || features.iter().all(|feature| feature.chromosome.is_none())
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "genome_track() chromosome filtering needs chromosome data",
                None,
            ));
        }
        features.retain(|feature| feature.chromosome.as_deref() == Some(chromosome));
    }
    let rows_with_chromosomes = features
        .iter()
        .filter(|feature| feature.chromosome.is_some())
        .count();
    let chromosomes = features
        .iter()
        .filter_map(|feature| feature.chromosome.as_deref())
        .collect::<HashSet<_>>();
    if rows_with_chromosomes != 0 && rows_with_chromosomes != features.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() chromosome values must be present for every row or omitted from every row",
            None,
        ));
    }
    if chromosomes.len() > 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() draws one region; select one chromosome with {chromosome: \"chr1\"}",
            None,
        ));
    }
    let selected_chromosome = chromosomes.into_iter().next().map(str::to_string);
    let region_start = opts.get("region_start").and_then(Value::as_float);
    let region_end = opts.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|number| !number.is_finite() || number < 0.0)
        || region_end.is_some_and(|number| !number.is_finite() || number < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() region_start/region_end must be finite, non-negative and increasing",
            None,
        ));
    }
    let mut clipped_rows = 0usize;
    features = features
        .into_iter()
        .filter_map(|mut feature| {
            if region_start.is_some_and(|start| feature.end <= start)
                || region_end.is_some_and(|end| feature.start >= end)
            {
                return None;
            }
            feature.start = region_start.map_or(feature.start, |start| feature.start.max(start));
            feature.end = region_end.map_or(feature.end, |end| feature.end.min(end));
            if feature.start != feature.original_start || feature.end != feature.original_end {
                clipped_rows += 1;
            }
            (feature.end > feature.start).then_some(feature)
        })
        .collect();
    if features.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "genome_track() no features after chromosome/region filtering",
            None,
        ));
    }
    features.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let lane_count = assign_interval_lanes(
        &mut features,
        |feature| feature.start,
        |feature| feature.end,
        |feature, lane| feature.lane = lane,
    );
    let domain_start = region_start.unwrap_or_else(|| {
        features
            .iter()
            .map(|feature| feature.start)
            .fold(f64::INFINITY, f64::min)
    });
    let domain_end = region_end.unwrap_or_else(|| {
        features
            .iter()
            .map(|feature| feature.end)
            .fold(f64::NEG_INFINITY, f64::max)
    });
    let width = get_opt_f64(opts, "width", 1000.0);
    let show_labels = opts
        .get("show_labels")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let max_labels = get_opt_usize(opts, "max_labels", 100);
    if show_labels && max_labels > 0 {
        let span = domain_end - domain_start;
        let plot_width = (width - 80.0).max(1.0);
        let mut lane_right = vec![f64::NEG_INFINITY; lane_count];
        let mut drawn = 0usize;
        for feature in &mut features {
            let Some(name) = feature.name.as_deref() else {
                continue;
            };
            if drawn >= max_labels {
                break;
            }
            let x = (feature.start - domain_start) / span * plot_width;
            let right = x + estimate_text_width(name, 9.0) + 6.0;
            if x >= lane_right[feature.lane] {
                feature.label_drawn = true;
                lane_right[feature.lane] = right;
                drawn += 1;
            }
        }
    }
    let retained_rows = features.len();
    let rows = features
        .iter()
        .enumerate()
        .map(|(index, feature)| {
            vec![
                Value::Int(index as i64),
                Value::Int(feature.source_row as i64),
                feature
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(feature.original_start),
                Value::Float(feature.original_end),
                Value::Float(feature.start),
                Value::Float(feature.end),
                Value::Float(feature.end - feature.start),
                feature.name.clone().map(Value::Str).unwrap_or(Value::Nil),
                if feature.strand.is_empty() {
                    Value::Nil
                } else {
                    Value::Str(feature.strand.clone())
                },
                Value::Int(feature.lane as i64),
                Value::Str(PALETTE[feature.lane % PALETTE.len()].into()),
                Value::Bool(feature.label_drawn),
                Value::Bool(
                    feature.start != feature.original_start || feature.end != feature.original_end,
                ),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Genome Track");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("genome_track".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "feature_index",
                        "source_row",
                        "chromosome",
                        "original_start",
                        "original_end",
                        "start",
                        "end",
                        "length",
                        "name",
                        "strand",
                        "lane",
                        "color",
                        "label_drawn",
                        "clipped",
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
                        ("width".into(), Value::Float(width)),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 300.0)),
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
                            Value::Str(get_opt_str(opts, "xlabel", "Genomic position").into()),
                        ),
                        (
                            "chromosome".into(),
                            selected_chromosome.map(Value::Str).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_start".into(),
                            region_start.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_end".into(),
                            region_end.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        ("domain_start".into(), Value::Float(domain_start)),
                        ("domain_end".into(), Value::Float(domain_end)),
                        ("lane_count".into(), Value::Int(lane_count as i64)),
                        ("show_labels".into(), Value::Bool(show_labels)),
                        ("max_labels".into(), Value::Int(max_labels as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("genome_track".into())),
                        ("input_rows".into(), Value::Int(input_rows as i64)),
                        ("retained_rows".into(), Value::Int(retained_rows as i64)),
                        ("clipped_rows".into(), Value::Int(clipped_rows as i64)),
                        (
                            "lane_rule".into(),
                            Value::Str("greedy_first_non_overlapping_lane".into()),
                        ),
                        (
                            "row_order".into(),
                            Value::Str("start_end_source_row".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "warnings".into(),
                Value::List(
                    if retained_rows == input_rows && clipped_rows == 0 {
                        Vec::new()
                    } else {
                        vec![Value::Str(format!(
                            "{retained_rows} features retained from {input_rows}; {clipped_rows} clipped to the requested region"
                        ))]
                    }
                    .into(),
                ),
            ),
        ])
        .into(),
    ))
}

pub(super) fn render_genome_track_svg(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 1000.0);
    let height = get_opt_f64(opts, "height", 300.0);
    let title = get_opt_str(opts, "title", "Genome Track");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let xlabel = get_opt_str(opts, "xlabel", "Genomic position");
    let domain = (
        get_opt_f64(opts, "domain_start", 0.0),
        get_opt_f64(opts, "domain_end", 1.0),
    );
    let lane_count = get_opt_usize(opts, "lane_count", 1).max(1);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[domain.0, domain.1],
        &[0.0, lane_count as f64],
        xlabel,
        "",
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let lane_step = (canvas.plot_height() / lane_count as f64).max(1.0);
    let feature_height = (lane_step * 0.48).clamp(4.0, 16.0);
    let backbone_y = canvas.margin.top + canvas.plot_height();
    canvas.add_line(
        canvas.margin.left,
        backbone_y,
        canvas.margin.left + canvas.plot_width(),
        backbone_y,
        "#b8bcc4",
        1.5,
    );
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let name_column = table.col_index("name").unwrap();
    let strand_column = table.col_index("strand").unwrap();
    let lane_column = table.col_index("lane").unwrap();
    let color_column = table.col_index("color").unwrap();
    let label_column = table.col_index("label_drawn").unwrap();
    let dense = table.num_rows() > 200;
    let mut dense_paths: BTreeMap<String, String> = BTreeMap::new();
    let mut arrow_path = String::new();
    for row in &table.rows {
        let start = row[start_column].as_float().unwrap();
        let end = row[end_column].as_float().unwrap();
        let lane = row[lane_column].as_float().unwrap() as usize;
        let color = row[color_column].as_str().unwrap();
        let x1 = x_scale.map(start);
        let x2 = x_scale.map(end);
        let y = canvas.margin.top + lane as f64 * lane_step + (lane_step - feature_height) / 2.0;
        let feature_width = (x2 - x1).max(1.5);
        if dense {
            dense_paths
                .entry(color.into())
                .or_default()
                .push_str(&format!(
                    "M{x1:.2},{y:.2}h{feature_width:.2}v{feature_height:.2}h-{feature_width:.2}Z"
                ));
        } else {
            canvas.add_rect(x1, y, feature_width, feature_height, color);
        }
        let strand = row[strand_column].as_str().unwrap_or("");
        if matches!(strand, "+" | "-") {
            let tip = if strand == "+" { x2 - 1.0 } else { x1 + 1.0 };
            let inward = if strand == "+" { -1.0 } else { 1.0 };
            let mid = y + feature_height / 2.0;
            arrow_path.push_str(&format!(
                "M{:.2},{:.2}L{tip:.2},{mid:.2}L{:.2},{:.2}",
                tip + inward * 5.0,
                mid - feature_height * 0.28,
                tip + inward * 5.0,
                mid + feature_height * 0.28
            ));
        }
        if row[label_column].as_bool().unwrap_or(false) {
            if let Some(name) = row[name_column].as_str() {
                canvas.add_text(x1, y - 3.0, name, "start", 9.0);
            }
        }
    }
    for (color, path) in dense_paths {
        canvas
            .elements
            .push(format!(r#"<path d="{path}" fill="{color}" />"#));
    }
    if !arrow_path.is_empty() {
        canvas.elements.push(format!(
            r##"<path d="{arrow_path}" fill="none" stroke="#ffffff" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" />"##
        ));
    }
    canvas.draw_x_axis(
        &Scale {
            domain,
            range: domain,
        },
        xlabel,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Genome annotation track with {} features arranged across {lane_count} non-overlapping lanes.",
        table.num_rows()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_genome_track_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "genome_track"))
}

pub(crate) fn render_genome_track_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_genome_track_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 genome-track Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genome-track specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "feature_index",
        "source_row",
        "chromosome",
        "original_start",
        "original_end",
        "start",
        "end",
        "length",
        "name",
        "strand",
        "lane",
        "color",
        "label_drawn",
        "clipped",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() genome-track data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genome-track specification has no features",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "genome-track")?;
    let domain_start = options
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let domain_end = options
        .get("domain_end")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let lane_count = options
        .get("lane_count")
        .map(|value| frozen_nonnegative_integer(value, "genome-track", "lane_count"))
        .transpose()?
        .unwrap_or(0);
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_start < 0.0
        || domain_end <= domain_start
        || lane_count == 0
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genome-track frozen domain or lane count is malformed",
            None,
        ));
    }
    let index_column = table.col_index("feature_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let original_start_column = table.col_index("original_start").unwrap();
    let original_end_column = table.col_index("original_end").unwrap();
    let chromosome_column = table.col_index("chromosome").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let length_column = table.col_index("length").unwrap();
    let name_column = table.col_index("name").unwrap();
    let strand_column = table.col_index("strand").unwrap();
    let lane_column = table.col_index("lane").unwrap();
    let color_column = table.col_index("color").unwrap();
    let label_column = table.col_index("label_drawn").unwrap();
    let clipped_column = table.col_index("clipped").unwrap();
    let selected_chromosome = options.get("chromosome").and_then(Value::as_str);
    let region_start = options.get("region_start").and_then(Value::as_float);
    let region_end = options.get("region_end").and_then(Value::as_float);
    let mut previous: Option<(f64, f64, usize)> = None;
    let mut lane_ends = vec![f64::NEG_INFINITY; lane_count];
    let mut highest_lane = 0usize;
    for (expected, row) in table.rows.iter().enumerate() {
        let feature_index =
            frozen_nonnegative_integer(&row[index_column], "genome-track", "feature_index")?;
        let source = frozen_nonnegative_integer(&row[source_column], "genome-track", "source_row")?;
        let original_start = row[original_start_column].as_float().unwrap_or(f64::NAN);
        let original_end = row[original_end_column].as_float().unwrap_or(f64::NAN);
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let length = row[length_column].as_float().unwrap_or(f64::NAN);
        let lane = frozen_nonnegative_integer(&row[lane_column], "genome-track", "lane")?;
        let chromosome = match &row[chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() genome-track chromosome is malformed",
                    None,
                ))
            }
        };
        let name_ok = matches!(&row[name_column], Value::Nil | Value::Str(_));
        let expected_lane = lane_ends
            .iter()
            .position(|lane_end| *lane_end <= start)
            .unwrap_or(lane_count);
        let strand_ok = matches!(&row[strand_column], Value::Nil)
            || matches!(row[strand_column].as_str(), Some("+" | "-" | "."));
        let key = (start, end, source);
        let clipped = start != original_start || end != original_end;
        let expected_start = region_start.map_or(original_start, |lower| original_start.max(lower));
        let expected_end = region_end.map_or(original_end, |upper| original_end.min(upper));
        if feature_index != expected
            || !original_start.is_finite()
            || !original_end.is_finite()
            || !start.is_finite()
            || !end.is_finite()
            || original_start < 0.0
            || original_end <= original_start
            || start < domain_start
            || end > domain_end
            || end <= start
            || (start - expected_start).abs() > 1e-10
            || (end - expected_end).abs() > 1e-10
            || (length - (end - start)).abs() > 1e-10
            || lane >= lane_count
            || lane != expected_lane
            || !strand_ok
            || !name_ok
            || (row[label_column].is_truthy() && row[name_column].as_str().is_none())
            || selected_chromosome != chromosome
            || !row[color_column].as_str().is_some_and(valid_bio_hex_color)
            || !matches!(row[label_column], Value::Bool(_))
            || !matches!(row[clipped_column], Value::Bool(_))
            || row[clipped_column].is_truthy() != clipped
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() genome-track frozen geometry is inconsistent",
                None,
            ));
        }
        lane_ends[lane] = end;
        highest_lane = highest_lane.max(lane);
        previous = Some(key);
    }
    if highest_lane + 1 != lane_count {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() genome-track frozen lane count is inconsistent",
            None,
        ));
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_genome_track_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Genome Track");
    finish_frozen_bio_plot(value, render_options, title, "genome-track", svg)
}

pub(crate) fn builtin_genome_track(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = genome_track_spec_value(&args[0], &opts)?;
    render_genome_track_plot_spec_value(&specification, &opts)
}

#[cfg(any())]
pub(super) fn builtin_lollipop_legacy(args: Vec<Value>) -> Result<Value> {
    let table = require_table_bp(&args[0], "lollipop")?;
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let positions = extract_table_col(table, get_opt_str(&opts, "pos", "position"))?;
    let labels = extract_str_col(table, get_opt_str(&opts, "label", "label")).ok();
    let heights = extract_table_col(table, "count")
        .or_else(|_| extract_table_col(table, "height"))
        .ok();

    let xr = col_range(&positions);
    let ys = heights.as_ref().map(|h| col_range(h)).unwrap_or((0.0, 1.0));
    let yr = (0.0, ys.1 * 1.1);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };
        let y_scale = Scale {
            domain: yr,
            range: (c.margin.top + c.plot_height(), c.margin.top),
        };
        // Domain bar
        let bar_y = c.margin.top + c.plot_height();
        c.add_rect(c.margin.left, bar_y - 8.0, c.plot_width(), 16.0, "#eee");
        for i in 0..positions.len() {
            let x = xs.map(positions[i]);
            let height_val = heights.as_ref().map(|h| h[i]).unwrap_or(1.0);
            let y = y_scale.map(height_val);
            c.add_line(x, bar_y, x, y, "#333", 1.5);
            c.add_circle(x, y, 5.0, PALETTE[i % PALETTE.len()]);
            if let Some(ref lbls) = labels {
                c.add_text(x, y - 8.0, &lbls[i], "middle", 8.0);
            }
        }
        let dx = Scale {
            domain: xr,
            range: xr,
        };
        c.draw_x_axis(&dx, "Position");
        c.draw_title("Lollipop Plot");
        return Ok(Value::Str(c.render()));
    }

    let width = get_opt_usize(&opts, "width", 60);
    let height = get_opt_usize(&opts, "height", 12);
    let mut chart = AsciiChart::new(width, height);
    for i in 0..positions.len() {
        let h = heights.as_ref().map(|hv| hv[i]).unwrap_or(1.0);
        chart.put(positions[i], h, xr, yr, '●');
        // Draw stem
        let (gx, gy) = chart.map(positions[i], h, xr, yr);
        let (_, base_y) = chart.map(positions[i], 0.0, xr, yr);
        for y in gy..base_y {
            chart.grid[y][gx] = '│';
        }
        chart.grid[gy][gx] = '●';
    }
    write_output(&chart.render("Lollipop Plot"));
    Ok(Value::Nil)
}

// ── 19. circos ──────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub(super) struct LollipopDatum {
    source_row: usize,
    position: f64,
    height: f64,
    label: Option<String>,
    label_lane: usize,
    label_drawn: bool,
}

pub(super) fn lollipop_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let table = require_table_bp(value, "lollipop")?;
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "lollipop() empty data",
            None,
        ));
    }
    let position_name = get_opt_str(opts, "pos", "position");
    let position_column = table.col_index(position_name).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("column '{position_name}' not found"),
            None,
        )
    })?;
    let height_column = table
        .col_index("count")
        .or_else(|| table.col_index("height"));
    let label_column = table.col_index(get_opt_str(opts, "label", "label"));
    let mut data = Vec::with_capacity(table.num_rows());
    for (source_row, row) in table.rows.iter().enumerate() {
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let height = height_column
            .and_then(|column| row[column].as_float())
            .unwrap_or(1.0);
        if !position.is_finite() || position < 0.0 || !height.is_finite() || height < 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "lollipop() positions and heights must be finite and non-negative",
                None,
            ));
        }
        data.push(LollipopDatum {
            source_row,
            position,
            height,
            label: optional_string_cell(row, label_column, "lollipop")?,
            label_lane: 0,
            label_drawn: false,
        });
    }
    data.sort_by(|left, right| {
        left.position
            .total_cmp(&right.position)
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let observed_start = data.first().unwrap().position;
    let observed_end = data.last().unwrap().position;
    let mut domain_start = opts
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or_else(|| {
            if opts.contains_key("length") {
                0.0
            } else {
                observed_start
            }
        });
    let mut domain_end = opts
        .get("domain_end")
        .and_then(Value::as_float)
        .or_else(|| opts.get("length").and_then(Value::as_float))
        .unwrap_or(observed_end);
    if domain_end == domain_start {
        domain_start = (domain_start - 0.5).max(0.0);
        domain_end += 0.5;
    }
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_start < 0.0
        || domain_end <= domain_start
        || observed_start < domain_start
        || observed_end > domain_end
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "lollipop() domain must be finite, increasing, and contain every position",
            None,
        ));
    }
    let show_labels = opts
        .get("show_labels")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let max_labels = get_opt_usize(opts, "max_labels", 80);
    if show_labels {
        let width = get_opt_f64(opts, "width", 800.0);
        let plot_width = (width - 80.0).max(1.0);
        let span = domain_end - domain_start;
        let mut lane_right = [f64::NEG_INFINITY; 2];
        let mut drawn = 0usize;
        for datum in &mut data {
            let Some(label) = datum.label.as_deref() else {
                continue;
            };
            if drawn >= max_labels {
                break;
            }
            let x = (datum.position - domain_start) / span * plot_width;
            let half = estimate_text_width(label, 8.0) / 2.0 + 4.0;
            if let Some(lane) = lane_right.iter().position(|right| x - half >= *right) {
                datum.label_lane = lane;
                datum.label_drawn = true;
                lane_right[lane] = x + half;
                drawn += 1;
            }
        }
    }
    let y_max = data
        .iter()
        .map(|datum| datum.height)
        .fold(0.0, f64::max)
        .max(1.0);
    let rows = data
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                Value::Float(datum.position),
                Value::Float(datum.height),
                datum.label.clone().map(Value::Str).unwrap_or(Value::Nil),
                Value::Int(datum.label_lane as i64),
                Value::Bool(datum.label_drawn),
                Value::Str(PALETTE[index % PALETTE.len()].into()),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Lollipop Plot");
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("lollipop".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "position",
                        "height",
                        "label",
                        "label_lane",
                        "label_drawn",
                        "color",
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
                            Value::Float(get_opt_f64(opts, "width", 800.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 320.0)),
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
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "Count / value").into()),
                        ),
                        ("domain_start".into(), Value::Float(domain_start)),
                        ("domain_end".into(), Value::Float(domain_end)),
                        ("y_max".into(), Value::Float(y_max)),
                        ("show_labels".into(), Value::Bool(show_labels)),
                        ("max_labels".into(), Value::Int(max_labels as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("lollipop".into())),
                        ("input_rows".into(), Value::Int(data.len() as i64)),
                        ("row_order".into(), Value::Str("position_source_row".into())),
                        (
                            "label_rule".into(),
                            Value::Str("two_lane_first_fit_with_limit".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::new().into())),
        ])
        .into(),
    ))
}

pub(super) fn render_lollipop_svg(table: &Table, opts: &HashMap<String, Value>) -> Result<String> {
    let width = get_opt_f64(opts, "width", 800.0);
    let height = get_opt_f64(opts, "height", 320.0);
    let title = get_opt_str(opts, "title", "Lollipop Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let xlabel = get_opt_str(opts, "xlabel", "Position");
    let ylabel = get_opt_str(opts, "ylabel", "Count / value");
    let x_domain = (
        get_opt_f64(opts, "domain_start", 0.0),
        get_opt_f64(opts, "domain_end", 1.0),
    );
    let y_domain = (0.0, get_opt_f64(opts, "y_max", 1.0) * 1.12);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[x_domain.0, x_domain.1],
        &[0.0, y_domain.1],
        xlabel,
        ylabel,
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
    let baseline = y_scale.map(0.0);
    canvas.add_rect(
        canvas.margin.left,
        baseline - 5.0,
        canvas.plot_width(),
        10.0,
        "#e2e5e9",
    );
    let position_column = table.col_index("position").unwrap();
    let height_column = table.col_index("height").unwrap();
    let label_column = table.col_index("label").unwrap();
    let label_lane_column = table.col_index("label_lane").unwrap();
    let label_drawn_column = table.col_index("label_drawn").unwrap();
    let color_column = table.col_index("color").unwrap();
    let mut stems = String::new();
    let mut dense_points: BTreeMap<String, String> = BTreeMap::new();
    let dense = table.num_rows() > 200;
    for row in &table.rows {
        let x = x_scale.map(row[position_column].as_float().unwrap());
        let y = y_scale.map(row[height_column].as_float().unwrap());
        let color = row[color_column].as_str().unwrap();
        stems.push_str(&format!("M{x:.2},{baseline:.2}V{y:.2}"));
        if dense {
            dense_points
                .entry(color.into())
                .or_default()
                .push_str(&format!(
                    "M{:.2},{y:.2}a4,4 0 1,0 8,0a4,4 0 1,0 -8,0",
                    x - 4.0
                ));
        } else {
            canvas.add_circle(x, y, 5.0, color);
        }
        if row[label_drawn_column].as_bool().unwrap_or(false) {
            if let Some(label) = row[label_column].as_str() {
                let lane = row[label_lane_column].as_float().unwrap_or(0.0);
                canvas.add_text(x, y - 8.0 - lane * 11.0, label, "middle", 8.0);
            }
        }
    }
    canvas.elements.push(format!(
        r##"<path d="{stems}" fill="none" stroke="#343a40" stroke-width="1.4" />"##
    ));
    for (color, path) in dense_points {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="{color}" fill-opacity="0.78" />"#
        ));
    }
    canvas.draw_x_axis(
        &Scale {
            domain: x_domain,
            range: x_domain,
        },
        xlabel,
    );
    canvas.draw_y_axis(
        &Scale {
            domain: y_domain,
            range: y_domain,
        },
        ylabel,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Lollipop plot with {} observations across positions {:.3} to {:.3}.",
        table.num_rows(),
        x_domain.0,
        x_domain.1
    ));
    Ok(canvas.render())
}

pub(crate) fn is_lollipop_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "lollipop"))
}

pub(crate) fn render_lollipop_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_lollipop_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 lollipop Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() lollipop specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "position",
        "height",
        "label",
        "label_lane",
        "label_drawn",
        "color",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() lollipop data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() lollipop specification has no observations",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "lollipop")?;
    let domain_start = options
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let domain_end = options
        .get("domain_end")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let y_max = options
        .get("y_max")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_end <= domain_start
        || !y_max.is_finite()
        || y_max <= 0.0
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() lollipop frozen domains are malformed",
            None,
        ));
    }
    let index_column = table.col_index("point_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let position_column = table.col_index("position").unwrap();
    let height_column = table.col_index("height").unwrap();
    let label_lane_column = table.col_index("label_lane").unwrap();
    let label_drawn_column = table.col_index("label_drawn").unwrap();
    let color_column = table.col_index("color").unwrap();
    let label_value_column = table.col_index("label").unwrap();
    let mut previous: Option<(f64, usize)> = None;
    let mut observed_y_max = 1.0f64;
    for (expected, row) in table.rows.iter().enumerate() {
        let index = frozen_nonnegative_integer(&row[index_column], "lollipop", "point_index")?;
        let source = frozen_nonnegative_integer(&row[source_column], "lollipop", "source_row")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let height = row[height_column].as_float().unwrap_or(f64::NAN);
        let label_lane =
            frozen_nonnegative_integer(&row[label_lane_column], "lollipop", "label_lane")?;
        let key = (position, source);
        if index != expected
            || !position.is_finite()
            || position < domain_start
            || position > domain_end
            || !height.is_finite()
            || height < 0.0
            || height > y_max
            || label_lane > 1
            || !matches!(&row[label_value_column], Value::Nil | Value::Str(_))
            || (row[label_drawn_column].is_truthy() && row[label_value_column].as_str().is_none())
            || !matches!(row[label_drawn_column], Value::Bool(_))
            || !row[color_column].as_str().is_some_and(valid_bio_hex_color)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() lollipop frozen geometry is inconsistent",
                None,
            ));
        }
        observed_y_max = observed_y_max.max(height);
        previous = Some(key);
    }
    if (observed_y_max - y_max).abs() > 1e-10 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() lollipop frozen y maximum is inconsistent",
            None,
        ));
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_lollipop_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Lollipop Plot");
    finish_frozen_bio_plot(value, render_options, title, "lollipop", svg)
}

pub(super) fn builtin_lollipop(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = lollipop_spec_value(&args[0], &opts)?;
    render_lollipop_plot_spec_value(&specification, &opts)
}

pub(super) fn builtin_hic_map(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let heat_chars: Vec<char> = get_opt_str(&opts, "chars", " ░▒▓█").chars().collect();

    let (_names, data) = match &args[0] {
        Value::Matrix(m) => {
            let names = m
                .row_names
                .clone()
                .unwrap_or_else(|| (0..m.nrow).map(|i| format!("{i}")).collect());
            let mut data = vec![vec![0.0; m.ncol]; m.nrow];
            for r in 0..m.nrow {
                for c in 0..m.ncol {
                    data[r][c] = m.data[r * m.ncol + c];
                }
            }
            (names, data)
        }
        Value::Table(table) => {
            let mut cols_data: Vec<Vec<f64>> = Vec::new();
            for col in &table.columns {
                cols_data.push(extract_table_col(table, col)?);
            }
            let (nrows, ncols) = (table.num_rows(), table.num_cols());
            let mut t = vec![vec![0.0; ncols]; nrows];
            for c in 0..ncols {
                for r in 0..nrows {
                    t[r][c] = cols_data[c][r];
                }
            }
            let names: Vec<String> = (0..nrows).map(|i| format!("{i}")).collect();
            (names, t)
        }
        _ => {
            return Err(BioLangError::type_error(
                "hic_map() requires Matrix or Table",
                None,
            ))
        }
    };

    let n = data.len();
    let all: Vec<f64> = data
        .iter()
        .flat_map(|r| r.iter().copied())
        .filter(|v| v.is_finite() && *v > 0.0)
        .collect();
    let (vmin, vmax) = if all.is_empty() {
        (0.0, 1.0)
    } else {
        col_range(&all)
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 600.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = themed_canvas(w, h, &opts);
        c.margin.left = 50.0;
        c.margin.bottom = 50.0;
        let cell = (c.plot_width() / n as f64).min(c.plot_height() / n as f64);
        for r in 0..n {
            for col in r..n {
                let v = data[r][col];
                let t = if (vmax - vmin).abs() < f64::EPSILON {
                    0.5
                } else {
                    ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
                };
                let color = sequential_color(t);
                c.add_rect(
                    c.margin.left + col as f64 * cell,
                    c.margin.top + r as f64 * cell,
                    cell,
                    cell,
                    &color,
                );
            }
        }
        finish_themed_canvas(&mut c, &opts, "Hi-C Contact Map");
        return Ok(Value::Str(c.render()));
    }

    let nlevels = heat_chars.len();
    let mut out = String::from("  Hi-C Contact Map\n");
    for r in 0..n {
        out.push_str("  ");
        for col in 0..n {
            if col < r {
                out.push_str("  ");
                continue;
            }
            let v = data[r][col];
            let t = if (vmax - vmin).abs() < f64::EPSILON {
                0.5
            } else {
                ((v - vmin) / (vmax - vmin)).clamp(0.0, 1.0)
            };
            out.push(heat_chars[(t * (nlevels - 1) as f64).round() as usize]);
            out.push(' ');
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 21. sashimi ─────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub(super) struct SashimiCoverageDatum {
    source_row: usize,
    chromosome: Option<String>,
    position: f64,
    depth: f64,
}

#[derive(Clone, Debug)]
pub(super) struct SashimiJunctionDatum {
    source_row: usize,
    chromosome: Option<String>,
    start: f64,
    end: f64,
    count: f64,
    strand: String,
    lane: usize,
    arc_fraction: f64,
    stroke_width: f64,
    label_drawn: bool,
}

pub(super) fn sashimi_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let (coverage_table, junction_table) = match value {
        Value::Record(map) => {
            let coverage = match map.get("coverage") {
                None | Some(Value::Nil) => None,
                Some(Value::Table(table)) => Some(table),
                Some(_) => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "sashimi() coverage must be a Table",
                        None,
                    ))
                }
            };
            let junctions = match map.get("junctions") {
                Some(Value::Table(table)) => table,
                _ => {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "sashimi() needs a junctions Table",
                        None,
                    ))
                }
            };
            (coverage, junctions)
        }
        Value::Table(table) => (None, table),
        other => {
            return Err(BioLangError::type_error(
                format!(
                    "sashimi() requires Record or Table, got {}",
                    other.type_of()
                ),
                None,
            ))
        }
    };
    if junction_table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() junctions data is empty",
            None,
        ));
    }
    let start_column = junction_table.col_index("start").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'start' not found", None)
    })?;
    let end_column = junction_table.col_index("end").ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "column 'end' not found", None)
    })?;
    let count_column = junction_table.col_index("count");
    let junction_chromosome_column = junction_table
        .col_index("chrom")
        .or_else(|| junction_table.col_index("chromosome"));
    let strand_column = junction_table.col_index("strand");
    let mut junctions = Vec::with_capacity(junction_table.num_rows());
    for (source_row, row) in junction_table.rows.iter().enumerate() {
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let count = count_column
            .and_then(|column| row[column].as_float())
            .unwrap_or(1.0);
        if !start.is_finite()
            || !end.is_finite()
            || !count.is_finite()
            || start < 0.0
            || end <= start
            || count < 0.0
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sashimi() junction start/end/count must be finite and non-negative, with end > start",
                None,
            ));
        }
        let chromosome = optional_string_cell(row, junction_chromosome_column, "sashimi")?;
        let strand = optional_string_cell(row, strand_column, "sashimi")?.unwrap_or_default();
        if !matches!(strand.as_str(), "" | "+" | "-" | ".") {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sashimi() strand must be '+', '-', '.', or nil",
                None,
            ));
        }
        junctions.push(SashimiJunctionDatum {
            source_row,
            chromosome,
            start,
            end,
            count,
            strand,
            lane: 0,
            arc_fraction: 0.0,
            stroke_width: 0.0,
            label_drawn: false,
        });
    }
    let mut coverage = Vec::new();
    if let Some(table) = coverage_table {
        let position_column = table
            .col_index("pos")
            .or_else(|| table.col_index("position"))
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "sashimi() coverage needs a 'pos' or 'position' column",
                    None,
                )
            })?;
        let depth_column = table.col_index("depth").ok_or_else(|| {
            BioLangError::runtime(ErrorKind::TypeError, "column 'depth' not found", None)
        })?;
        let chromosome_column = table
            .col_index("chrom")
            .or_else(|| table.col_index("chromosome"));
        for (source_row, row) in table.rows.iter().enumerate() {
            let position = row[position_column].as_float().unwrap_or(f64::NAN);
            let depth = row[depth_column].as_float().unwrap_or(f64::NAN);
            if !position.is_finite() || position < 0.0 || !depth.is_finite() || depth < 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "sashimi() coverage positions and depths must be finite and non-negative",
                    None,
                ));
            }
            coverage.push(SashimiCoverageDatum {
                source_row,
                chromosome: optional_string_cell(row, chromosome_column, "sashimi")?,
                position,
                depth,
            });
        }
    }
    let input_junctions = junctions.len();
    let input_coverage = coverage.len();
    let requested_chromosome = opts.get("chromosome").and_then(Value::as_str);
    if let Some(chromosome) = requested_chromosome {
        if chromosome.trim().is_empty()
            || (junctions.iter().all(|datum| datum.chromosome.is_none())
                && coverage.iter().all(|datum| datum.chromosome.is_none()))
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "sashimi() chromosome filtering needs chromosome data",
                None,
            ));
        }
        junctions.retain(|datum| {
            datum
                .chromosome
                .as_deref()
                .is_none_or(|value| value == chromosome)
        });
        coverage.retain(|datum| {
            datum
                .chromosome
                .as_deref()
                .is_none_or(|value| value == chromosome)
        });
    }
    for (present, total, family) in [
        (
            junctions
                .iter()
                .filter(|datum| datum.chromosome.is_some())
                .count(),
            junctions.len(),
            "junction",
        ),
        (
            coverage
                .iter()
                .filter(|datum| datum.chromosome.is_some())
                .count(),
            coverage.len(),
            "coverage",
        ),
    ] {
        if present != 0 && present != total {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("sashimi() {family} chromosome values must be complete or omitted"),
                None,
            ));
        }
    }
    let chromosomes = junctions
        .iter()
        .filter_map(|datum| datum.chromosome.as_deref())
        .chain(
            coverage
                .iter()
                .filter_map(|datum| datum.chromosome.as_deref()),
        )
        .collect::<HashSet<_>>();
    if chromosomes.len() > 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() draws one region; select one chromosome with {chromosome: \"chr1\"}",
            None,
        ));
    }
    let selected_chromosome = chromosomes.into_iter().next().map(str::to_string);
    let region_start = opts.get("region_start").and_then(Value::as_float);
    let region_end = opts.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|number| !number.is_finite() || number < 0.0)
        || region_end.is_some_and(|number| !number.is_finite() || number < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() region_start/region_end must be finite, non-negative and increasing",
            None,
        ));
    }
    junctions.retain(|datum| {
        region_start.is_none_or(|start| datum.start >= start)
            && region_end.is_none_or(|end| datum.end <= end)
    });
    coverage.retain(|datum| {
        region_start.is_none_or(|start| datum.position >= start)
            && region_end.is_none_or(|end| datum.position <= end)
    });
    if junctions.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "sashimi() no complete junctions remain in the requested region",
            None,
        ));
    }
    junctions.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    coverage.sort_by(|left, right| {
        left.position
            .total_cmp(&right.position)
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let lane_count = assign_interval_lanes(
        &mut junctions,
        |datum| datum.start,
        |datum| datum.end,
        |datum, lane| datum.lane = lane,
    );
    let max_count = junctions
        .iter()
        .map(|datum| datum.count)
        .fold(0.0, f64::max)
        .max(1.0);
    for datum in &mut junctions {
        let strength = (datum.count / max_count).sqrt();
        datum.arc_fraction = 0.35 + 0.65 * strength;
        datum.stroke_width = 1.0 + (strength * 12.0).round() / 4.0;
    }
    let max_labels = get_opt_usize(opts, "max_labels", 60);
    let show_labels = opts
        .get("show_labels")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    if show_labels && max_labels > 0 {
        let mut by_count: Vec<usize> = (0..junctions.len()).collect();
        by_count.sort_by(|left, right| {
            junctions[*right]
                .count
                .total_cmp(&junctions[*left].count)
                .then_with(|| {
                    junctions[*left]
                        .source_row
                        .cmp(&junctions[*right].source_row)
                })
        });
        for index in by_count.into_iter().take(max_labels) {
            junctions[index].label_drawn = true;
        }
    }
    let data_start = junctions
        .iter()
        .map(|datum| datum.start)
        .chain(coverage.iter().map(|datum| datum.position))
        .fold(f64::INFINITY, f64::min);
    let data_end = junctions
        .iter()
        .map(|datum| datum.end)
        .chain(coverage.iter().map(|datum| datum.position))
        .fold(f64::NEG_INFINITY, f64::max);
    let domain_start = region_start.unwrap_or(data_start);
    let domain_end = region_end.unwrap_or(data_end);
    let max_depth = coverage
        .iter()
        .map(|datum| datum.depth)
        .fold(0.0, f64::max)
        .max(1.0);
    let coverage_rows = coverage
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                datum
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(datum.position),
                Value::Float(datum.depth),
            ]
        })
        .collect();
    let junction_rows = junctions
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                datum
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(datum.start),
                Value::Float(datum.end),
                Value::Float(datum.end - datum.start),
                Value::Float(datum.count),
                if datum.strand.is_empty() {
                    Value::Nil
                } else {
                    Value::Str(datum.strand.clone())
                },
                Value::Int(datum.lane as i64),
                Value::Float(datum.arc_fraction),
                Value::Float(datum.stroke_width),
                Value::Str(PALETTE[datum.lane % PALETTE.len()].into()),
                Value::Bool(datum.label_drawn),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Sashimi Plot");
    let mut warnings = Vec::new();
    if junctions.len() != input_junctions || coverage.len() != input_coverage {
        warnings.push(Value::Str(format!(
            "{} of {input_junctions} complete junctions and {} of {input_coverage} coverage points retained",
            junctions.len(),
            coverage.len()
        )));
    }
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("sashimi".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "coverage".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "chromosome",
                        "position",
                        "depth",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    coverage_rows,
                )),
            ),
            (
                "junctions".into(),
                Value::Table(Table::new(
                    [
                        "junction_index",
                        "source_row",
                        "chromosome",
                        "start",
                        "end",
                        "span",
                        "count",
                        "strand",
                        "lane",
                        "arc_fraction",
                        "stroke_width",
                        "color",
                        "label_drawn",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    junction_rows,
                )),
            ),
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
                            Value::Str(get_opt_str(opts, "xlabel", "Genomic position").into()),
                        ),
                        (
                            "chromosome".into(),
                            selected_chromosome.map(Value::Str).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_start".into(),
                            region_start.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_end".into(),
                            region_end.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        ("domain_start".into(), Value::Float(domain_start)),
                        ("domain_end".into(), Value::Float(domain_end)),
                        ("max_count".into(), Value::Float(max_count)),
                        ("max_depth".into(), Value::Float(max_depth)),
                        ("lane_count".into(), Value::Int(lane_count as i64)),
                        ("show_labels".into(), Value::Bool(show_labels)),
                        ("max_labels".into(), Value::Int(max_labels as i64)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("sashimi".into())),
                        (
                            "junction_order".into(),
                            Value::Str("start_end_source_row".into()),
                        ),
                        (
                            "coverage_order".into(),
                            Value::Str("position_source_row".into()),
                        ),
                        (
                            "lane_rule".into(),
                            Value::Str("greedy_first_non_overlapping_lane".into()),
                        ),
                        (
                            "region_rule".into(),
                            Value::Str("complete_junctions_points_inclusive".into()),
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

pub(super) fn render_sashimi_svg(
    coverage: &Table,
    junctions: &Table,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let width = get_opt_f64(opts, "width", 900.0);
    let height = get_opt_f64(opts, "height", 380.0);
    let title = get_opt_str(opts, "title", "Sashimi Plot");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let xlabel = get_opt_str(opts, "xlabel", "Genomic position");
    let domain = (
        get_opt_f64(opts, "domain_start", 0.0),
        get_opt_f64(opts, "domain_end", 1.0),
    );
    let max_depth = get_opt_f64(opts, "max_depth", 1.0);
    let lane_count = get_opt_usize(opts, "lane_count", 1).max(1);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[domain.0, domain.1],
        &[0.0, max_depth],
        xlabel,
        "",
        title,
        subtitle,
        caption,
        0.0,
    );
    let x_scale = Scale {
        domain,
        range: (canvas.margin.left, canvas.margin.left + canvas.plot_width()),
    };
    let panel_top = canvas.margin.top;
    let coverage_height = canvas.plot_height() * 0.43;
    let separator_y = panel_top + coverage_height + 12.0;
    let arc_base = canvas.margin.top + canvas.plot_height() - 6.0;
    let arc_height = (arc_base - separator_y - 8.0).max(10.0);
    canvas.add_line(
        canvas.margin.left,
        separator_y,
        canvas.margin.left + canvas.plot_width(),
        separator_y,
        "#c5c9d0",
        1.0,
    );
    if coverage.num_rows() > 0 {
        let position_column = coverage.col_index("position").unwrap();
        let depth_column = coverage.col_index("depth").unwrap();
        let mut area = String::new();
        let mut line = String::new();
        for (index, row) in coverage.rows.iter().enumerate() {
            let x = x_scale.map(row[position_column].as_float().unwrap());
            let depth = row[depth_column].as_float().unwrap();
            let y = panel_top + coverage_height - depth / max_depth * coverage_height;
            if index == 0 {
                area.push_str(&format!(
                    "M{x:.2},{:.2}L{x:.2},{y:.2}",
                    panel_top + coverage_height
                ));
                line.push_str(&format!("M{x:.2},{y:.2}"));
            } else {
                area.push_str(&format!("L{x:.2},{y:.2}"));
                line.push_str(&format!("L{x:.2},{y:.2}"));
            }
        }
        let last_x = x_scale.map(
            coverage.rows.last().unwrap()[position_column]
                .as_float()
                .unwrap(),
        );
        area.push_str(&format!("L{last_x:.2},{:.2}Z", panel_top + coverage_height));
        canvas.elements.push(format!(
            r##"<path d="{area}" fill="#9aa6b2" fill-opacity="0.35" stroke="none" />"##
        ));
        canvas.elements.push(format!(
            r##"<path d="{line}" fill="none" stroke="#5f6b76" stroke-width="1.3" stroke-linejoin="round" />"##
        ));
        canvas.add_text(
            canvas.margin.left + 4.0,
            panel_top + 12.0,
            &format!("Coverage (max {max_depth:.1})"),
            "start",
            9.0,
        );
    }
    let start_column = junctions.col_index("start").unwrap();
    let end_column = junctions.col_index("end").unwrap();
    let count_column = junctions.col_index("count").unwrap();
    let lane_column = junctions.col_index("lane").unwrap();
    let fraction_column = junctions.col_index("arc_fraction").unwrap();
    let stroke_column = junctions.col_index("stroke_width").unwrap();
    let color_column = junctions.col_index("color").unwrap();
    let label_column = junctions.col_index("label_drawn").unwrap();
    let mut paths: BTreeMap<(String, i64), String> = BTreeMap::new();
    for row in &junctions.rows {
        let x1 = x_scale.map(row[start_column].as_float().unwrap());
        let x2 = x_scale.map(row[end_column].as_float().unwrap());
        let lane = row[lane_column].as_float().unwrap() as usize;
        let fraction = row[fraction_column].as_float().unwrap();
        let stroke_width = row[stroke_column].as_float().unwrap();
        let color = row[color_column].as_str().unwrap();
        let lane_fraction = (lane + 1) as f64 / lane_count as f64;
        let apex_y = arc_base - arc_height * lane_fraction * fraction;
        let mid_x = (x1 + x2) / 2.0;
        paths
            .entry((color.into(), (stroke_width * 100.0).round() as i64))
            .or_default()
            .push_str(&format!(
                "M{x1:.2},{arc_base:.2}Q{mid_x:.2},{apex_y:.2} {x2:.2},{arc_base:.2}"
            ));
        if row[label_column].as_bool().unwrap_or(false) {
            canvas.add_text(
                mid_x,
                apex_y - 4.0,
                &format!("{:.0}", row[count_column].as_float().unwrap()),
                "middle",
                8.5,
            );
        }
    }
    for ((color, stroke_key), path) in paths {
        let stroke_width = stroke_key as f64 / 100.0;
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{stroke_width:.2}" stroke-opacity="0.82" />"#
        ));
    }
    canvas.add_text(
        canvas.margin.left + 4.0,
        separator_y + 13.0,
        "Splice junction reads",
        "start",
        9.0,
    );
    canvas.draw_x_axis(
        &Scale {
            domain,
            range: domain,
        },
        xlabel,
    );
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Sashimi plot with {} coverage points and {} splice junctions across {lane_count} non-overlapping arc lanes.",
        coverage.num_rows(),
        junctions.num_rows()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_sashimi_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "sashimi"))
}

pub(crate) fn render_sashimi_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_sashimi_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 sashimi Record",
                None,
            ))
        }
    };
    let coverage = match map.get("coverage") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi specification field 'coverage' must be Table",
                None,
            ))
        }
    };
    let junctions = match map.get("junctions") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi specification field 'junctions' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "chromosome",
        "position",
        "depth",
    ] {
        if coverage.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() sashimi coverage is missing '{required}'"),
                None,
            ));
        }
    }
    for required in [
        "junction_index",
        "source_row",
        "chromosome",
        "start",
        "end",
        "span",
        "count",
        "strand",
        "lane",
        "arc_fraction",
        "stroke_width",
        "color",
        "label_drawn",
    ] {
        if junctions.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() sashimi junctions are missing '{required}'"),
                None,
            ));
        }
    }
    if junctions.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() sashimi specification has no junctions",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "sashimi")?;
    let domain_start = options
        .get("domain_start")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let domain_end = options
        .get("domain_end")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let max_count = options
        .get("max_count")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let max_depth = options
        .get("max_depth")
        .and_then(Value::as_float)
        .unwrap_or(f64::NAN);
    let lane_count = options
        .get("lane_count")
        .map(|value| frozen_nonnegative_integer(value, "sashimi", "lane_count"))
        .transpose()?
        .unwrap_or(0);
    if !domain_start.is_finite()
        || !domain_end.is_finite()
        || domain_start < 0.0
        || domain_end <= domain_start
        || !max_count.is_finite()
        || max_count <= 0.0
        || !max_depth.is_finite()
        || max_depth <= 0.0
        || lane_count == 0
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() sashimi frozen domains are malformed",
            None,
        ));
    }
    let point_index_column = coverage.col_index("point_index").unwrap();
    let point_source_column = coverage.col_index("source_row").unwrap();
    let point_chromosome_column = coverage.col_index("chromosome").unwrap();
    let position_column = coverage.col_index("position").unwrap();
    let depth_column = coverage.col_index("depth").unwrap();
    let selected_chromosome = options.get("chromosome").and_then(Value::as_str);
    let mut previous_point: Option<(f64, usize)> = None;
    let mut observed_max_depth = 1.0f64;
    for (expected, row) in coverage.rows.iter().enumerate() {
        let index = frozen_nonnegative_integer(&row[point_index_column], "sashimi", "point_index")?;
        let source =
            frozen_nonnegative_integer(&row[point_source_column], "sashimi", "source_row")?;
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let depth = row[depth_column].as_float().unwrap_or(f64::NAN);
        let chromosome = match &row[point_chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() sashimi coverage chromosome is malformed",
                    None,
                ))
            }
        };
        let key = (position, source);
        if index != expected
            || !position.is_finite()
            || position < domain_start
            || position > domain_end
            || !depth.is_finite()
            || depth < 0.0
            || depth > max_depth
            || chromosome.is_some_and(|name| selected_chromosome != Some(name))
            || previous_point.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi frozen coverage is inconsistent",
                None,
            ));
        }
        observed_max_depth = observed_max_depth.max(depth);
        previous_point = Some(key);
    }
    let junction_index_column = junctions.col_index("junction_index").unwrap();
    let source_column = junctions.col_index("source_row").unwrap();
    let chromosome_column = junctions.col_index("chromosome").unwrap();
    let start_column = junctions.col_index("start").unwrap();
    let end_column = junctions.col_index("end").unwrap();
    let span_column = junctions.col_index("span").unwrap();
    let count_column = junctions.col_index("count").unwrap();
    let strand_column = junctions.col_index("strand").unwrap();
    let lane_column = junctions.col_index("lane").unwrap();
    let fraction_column = junctions.col_index("arc_fraction").unwrap();
    let stroke_column = junctions.col_index("stroke_width").unwrap();
    let color_column = junctions.col_index("color").unwrap();
    let label_column = junctions.col_index("label_drawn").unwrap();
    let mut previous_junction: Option<(f64, f64, usize)> = None;
    let mut lane_ends = vec![f64::NEG_INFINITY; lane_count];
    let mut observed_max_count = 1.0f64;
    let mut highest_lane = 0usize;
    for (expected, row) in junctions.rows.iter().enumerate() {
        let index =
            frozen_nonnegative_integer(&row[junction_index_column], "sashimi", "junction_index")?;
        let source = frozen_nonnegative_integer(&row[source_column], "sashimi", "source_row")?;
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let span = row[span_column].as_float().unwrap_or(f64::NAN);
        let count = row[count_column].as_float().unwrap_or(f64::NAN);
        let lane = frozen_nonnegative_integer(&row[lane_column], "sashimi", "lane")?;
        let chromosome = match &row[chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() sashimi junction chromosome is malformed",
                    None,
                ))
            }
        };
        let strand_ok = matches!(&row[strand_column], Value::Nil)
            || matches!(row[strand_column].as_str(), Some("+" | "-" | "."));
        let fraction = row[fraction_column].as_float().unwrap_or(f64::NAN);
        let stroke = row[stroke_column].as_float().unwrap_or(f64::NAN);
        let expected_lane = lane_ends
            .iter()
            .position(|lane_end| *lane_end <= start)
            .unwrap_or(lane_count);
        let expected_strength = (count / max_count).sqrt();
        let expected_fraction = 0.35 + 0.65 * expected_strength;
        let expected_stroke = 1.0 + (expected_strength * 12.0).round() / 4.0;
        let key = (start, end, source);
        if index != expected
            || !start.is_finite()
            || !end.is_finite()
            || start < domain_start
            || end > domain_end
            || end <= start
            || (span - (end - start)).abs() > 1e-10
            || !count.is_finite()
            || count < 0.0
            || count > max_count
            || selected_chromosome != chromosome
            || !strand_ok
            || lane >= lane_count
            || lane != expected_lane
            || (fraction - expected_fraction).abs() > 1e-10
            || (stroke - expected_stroke).abs() > 1e-10
            || !row[color_column].as_str().is_some_and(valid_bio_hex_color)
            || !matches!(row[label_column], Value::Bool(_))
            || previous_junction.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() sashimi frozen junction geometry is inconsistent",
                None,
            ));
        }
        lane_ends[lane] = end;
        observed_max_count = observed_max_count.max(count);
        highest_lane = highest_lane.max(lane);
        previous_junction = Some(key);
    }
    if (observed_max_count - max_count).abs() > 1e-10
        || (observed_max_depth - max_depth).abs() > 1e-10
        || highest_lane + 1 != lane_count
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() sashimi frozen maxima or lane count are inconsistent",
            None,
        ));
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_sashimi_svg(coverage, junctions, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Sashimi Plot");
    finish_frozen_bio_plot(value, render_options, title, "sashimi", svg)
}

pub(super) fn builtin_sashimi(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = sashimi_spec_value(&args[0], &opts)?;
    render_sashimi_plot_spec_value(&specification, &opts)
}

#[cfg(any())]
pub(super) fn builtin_sashimi_legacy(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    // Input: Record{coverage: Table(pos, depth), junctions: Table(start, end, count)}
    // or just a Table of junctions
    let (cov_data, junctions) = match &args[0] {
        Value::Record(map) => {
            let cov = map.get("coverage").and_then(|v| {
                if let Value::Table(t) = v {
                    Some(t)
                } else {
                    None
                }
            });
            let junc = map.get("junctions").and_then(|v| {
                if let Value::Table(t) = v {
                    Some(t)
                } else {
                    None
                }
            });
            (cov.cloned(), junc.cloned())
        }
        Value::Table(t) => (None, Some(t.clone())),
        _ => {
            return Err(BioLangError::type_error(
                "sashimi() requires Record or Table",
                None,
            ))
        }
    };

    let junc_table = junctions.as_ref().ok_or_else(|| {
        BioLangError::runtime(ErrorKind::TypeError, "sashimi() needs junctions data", None)
    })?;
    let j_starts = extract_table_col(junc_table, "start")?;
    let j_ends = extract_table_col(junc_table, "end")?;
    let j_counts =
        extract_table_col(junc_table, "count").unwrap_or_else(|_| vec![1.0; j_starts.len()]);

    let mut all_pos: Vec<f64> = Vec::new();
    all_pos.extend(&j_starts);
    all_pos.extend(&j_ends);
    if let Some(ref ct) = cov_data {
        if let Ok(ps) = extract_table_col(ct, "pos") {
            all_pos.extend(&ps);
        }
    }
    let xr = col_range(&all_pos);

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 300.0);
        let mut c = SvgCanvas::new(w, h);
        let xs = Scale {
            domain: xr,
            range: (c.margin.left, c.margin.left + c.plot_width()),
        };

        // Coverage area
        if let Some(ref ct) = cov_data {
            if let (Ok(ps), Ok(ds)) = (extract_table_col(ct, "pos"), extract_table_col(ct, "depth"))
            {
                let max_d = ds.iter().cloned().fold(0.0f64, f64::max).max(1.0);
                let cov_h = c.plot_height() * 0.5;
                let base_y = c.margin.top + cov_h;
                let mut pts = format!("{:.1},{:.1} ", xs.map(ps[0]), base_y);
                for i in 0..ps.len() {
                    let y = base_y - (ds[i] / max_d) * cov_h;
                    pts.push_str(&format!("{:.1},{:.1} ", xs.map(ps[i]), y));
                }
                pts.push_str(&format!("{:.1},{:.1}", xs.map(*ps.last().unwrap()), base_y));
                c.elements.push(format!(
                    r##"<polygon points="{pts}" fill="#ccc" opacity="0.5" />"##
                ));
            }
        }

        // Junction arcs
        let max_count = j_counts.iter().cloned().fold(0.0f64, f64::max).max(1.0);
        let arc_base = c.margin.top + c.plot_height() * 0.55;
        for i in 0..j_starts.len() {
            let x1 = xs.map(j_starts[i]);
            let x2 = xs.map(j_ends[i]);
            let mid_x = (x1 + x2) / 2.0;
            let arc_h = (j_counts[i] / max_count) * c.plot_height() * 0.35;
            c.elements.push(format!(
                r#"<path d="M {x1:.1},{arc_base:.1} Q {mid_x:.1},{:.1} {x2:.1},{arc_base:.1}" fill="none" stroke="{}" stroke-width="{:.1}" />"#,
                arc_base - arc_h, PALETTE[i % PALETTE.len()], (j_counts[i] / max_count * 3.0).max(1.0)
            ));
            c.add_text(
                mid_x,
                arc_base - arc_h - 5.0,
                &format!("{:.0}", j_counts[i]),
                "middle",
                9.0,
            );
        }

        let dx = Scale {
            domain: xr,
            range: xr,
        };
        c.draw_x_axis(&dx, "Position");
        c.draw_title("Sashimi Plot");
        return Ok(Value::Str(c.render()));
    }

    // ASCII
    let width = get_opt_usize(&opts, "width", 60);
    let mut out = String::from("  Sashimi Plot\n");

    // Coverage sparkline if available
    if let Some(ref ct) = cov_data {
        if let (Ok(ps), Ok(ds)) = (extract_table_col(ct, "pos"), extract_table_col(ct, "depth")) {
            let mut bins = vec![0.0; width];
            let mut counts = vec![0usize; width];
            let span = xr.1 - xr.0;
            for i in 0..ps.len() {
                let b = ((ps[i] - xr.0) / span * width as f64) as usize;
                let b = b.min(width - 1);
                bins[b] += ds[i];
                counts[b] += 1;
            }
            for i in 0..width {
                if counts[i] > 0 {
                    bins[i] /= counts[i] as f64;
                }
            }
            out.push_str(&format!("  Depth: {}\n", spark_str(&bins)));
        }
    }

    // Junction list
    out.push_str(&format!("  Junctions ({}):\n", j_starts.len()));
    for i in 0..j_starts.len().min(15) {
        out.push_str(&format!(
            "    {:.0}─{:.0} ({:.0} reads) ⌒\n",
            j_starts[i], j_ends[i], j_counts[i]
        ));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 22. volcano_plot ────────────────────────────────────────────

pub(super) fn builtin_alignment_view(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "Multiple Sequence Alignment").to_string();
    let color_by = get_opt_str(&opts, "color_by", "nucleotide").to_string();
    let start_pos = get_opt_f64(&opts, "start", 0.0) as usize;
    let end_pos_opt: Option<usize> = opts
        .get("end")
        .and_then(|v| v.as_float())
        .map(|f| f as usize);

    // Extract sequences: List of Records with {id, sequence}
    let (ids, sequences): (Vec<String>, Vec<String>) = match &args[0] {
        Value::List(items) => {
            let mut ids = Vec::new();
            let mut seqs = Vec::new();
            for item in items.iter() {
                match item {
                    Value::Record(map) => {
                        let id = map
                            .get("id")
                            .map(|v| format!("{v}"))
                            .unwrap_or_else(|| format!("seq{}", ids.len() + 1));
                        let seq = map
                            .get("sequence")
                            .or(map.get("seq"))
                            .map(|v| match v {
                                Value::Str(s) => s.clone(),
                                Value::DNA(s) | Value::RNA(s) | Value::Protein(s) => s.data.clone(),
                                _ => format!("{v}"),
                            })
                            .unwrap_or_default();
                        ids.push(id);
                        seqs.push(seq);
                    }
                    Value::Str(s) => {
                        ids.push(format!("seq{}", ids.len() + 1));
                        seqs.push(s.clone());
                    }
                    Value::DNA(s) | Value::RNA(s) | Value::Protein(s) => {
                        ids.push(format!("seq{}", ids.len() + 1));
                        seqs.push(s.data.clone());
                    }
                    _ => {}
                }
            }
            (ids, seqs)
        }
        Value::Table(table) => {
            let id_col = table
                .col_index("id")
                .or(table.col_index("name"))
                .unwrap_or(0);
            let seq_col = table.col_index("sequence").or(table.col_index("seq"));
            if seq_col.is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "alignment_view() needs 'sequence' column",
                    None,
                ));
            }
            let seq_idx = seq_col.unwrap();
            let ids: Vec<String> = table
                .rows
                .iter()
                .map(|r| match &r[id_col] {
                    Value::Str(s) => s.clone(),
                    other => format!("{other}"),
                })
                .collect();
            let seqs: Vec<String> = table
                .rows
                .iter()
                .map(|r| match &r[seq_idx] {
                    Value::Str(s) => s.clone(),
                    Value::DNA(s) | Value::RNA(s) | Value::Protein(s) => s.data.clone(),
                    other => format!("{other}"),
                })
                .collect();
            (ids, seqs)
        }
        _ => {
            return Err(BioLangError::type_error(
                "alignment_view() requires List of Records or Table",
                None,
            ))
        }
    };
    if sequences.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "alignment_view() empty input",
            None,
        ));
    }

    let max_len = sequences.iter().map(|s| s.len()).max().unwrap_or(0);
    let end_pos = end_pos_opt.unwrap_or(max_len).min(max_len);
    let start = start_pos.min(end_pos);
    let display_len = (end_pos - start).min(100); // limit to 100 positions

    // Compute consensus
    let mut consensus = String::new();
    for pos in start..(start + display_len) {
        let mut counts: HashMap<char, usize> = HashMap::new();
        for seq in &sequences {
            if let Some(ch) = seq.chars().nth(pos) {
                *counts.entry(ch.to_ascii_uppercase()).or_insert(0) += 1;
            }
        }
        let top = counts
            .iter()
            .max_by_key(|(_, &c)| c)
            .map(|(&ch, _)| ch)
            .unwrap_or('-');
        consensus.push(top);
    }

    // Compute conservation scores (fraction matching consensus)
    let conservation: Vec<f64> = (0..display_len)
        .map(|di| {
            let pos = start + di;
            let cons_char = consensus.chars().nth(di).unwrap_or('-');
            let matches = sequences
                .iter()
                .filter(|s| s.chars().nth(pos).map(|c| c.to_ascii_uppercase()) == Some(cons_char))
                .count();
            matches as f64 / sequences.len() as f64
        })
        .collect();

    fn nuc_color(ch: char) -> &'static str {
        match ch.to_ascii_uppercase() {
            'A' => "#4caf50",
            'T' | 'U' => "#f44336",
            'C' => "#2196f3",
            'G' => "#ff9800",
            '-' => "#ffffff",
            _ => "#cccccc",
        }
    }

    fn conservation_color(score: f64) -> String {
        let t = score.clamp(0.0, 1.0);
        let r = (255.0 * (1.0 - t * 0.7)) as u8;
        let g = (255.0 * (1.0 - t * 0.4)) as u8;
        let b = (255.0 * (1.0 - t * 0.1)) as u8;
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    if fmt == "svg" {
        let cell_w = 12.0;
        let cell_h = 16.0;
        let label_w = ids.iter().map(|s| s.len()).max().unwrap_or(4) as f64 * 7.0 + 10.0;
        let pos_h = 20.0;
        let cons_h = 18.0;
        let w_auto = label_w + display_len as f64 * cell_w + 40.0;
        let h_auto = pos_h + (sequences.len() + 1) as f64 * cell_h + cons_h + 60.0;
        let w = get_opt_f64(&opts, "width", w_auto.min(1200.0));
        let h = get_opt_f64(&opts, "height", h_auto.min(800.0));
        let mut c = SvgCanvas::new(w, h);
        c.margin.left = label_w;
        c.margin.top = 50.0;

        let actual_cell_w = (c.plot_width() / display_len as f64).min(cell_w);

        // Position numbers at top (every 10th)
        for di in 0..display_len {
            let pos = start + di;
            if pos % 10 == 0 {
                let x = c.margin.left + di as f64 * actual_cell_w + actual_cell_w / 2.0;
                c.add_text(x, c.margin.top - 5.0, &pos.to_string(), "middle", 8.0);
            }
        }

        // Sequence rows
        for (si, seq) in sequences.iter().enumerate() {
            let y = c.margin.top + si as f64 * cell_h;
            c.add_text(c.margin.left - 5.0, y + cell_h * 0.7, &ids[si], "end", 9.0);
            for di in 0..display_len {
                let pos = start + di;
                let ch = seq.chars().nth(pos).unwrap_or('-');
                let x = c.margin.left + di as f64 * actual_cell_w;
                let fill = if color_by == "conservation" {
                    conservation_color(conservation[di])
                } else {
                    nuc_color(ch).to_string()
                };
                c.elements.push(format!(
                    r#"<rect x="{x:.1}" y="{y:.1}" width="{actual_cell_w:.1}" height="{:.1}" fill="{fill}" opacity="0.6" />"#,
                    cell_h - 1.0
                ));
                if actual_cell_w >= 8.0 {
                    c.elements.push(format!(
                        r##"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{:.0}" font-family="monospace" fill="#333">{}</text>"##,
                        x + actual_cell_w / 2.0, y + cell_h * 0.7, (actual_cell_w * 0.8).min(11.0), ch
                    ));
                }
            }
        }

        // Consensus row
        let cons_y = c.margin.top + sequences.len() as f64 * cell_h + 5.0;
        c.add_text(
            c.margin.left - 5.0,
            cons_y + cell_h * 0.7,
            "Consensus",
            "end",
            9.0,
        );
        for (di, cons_ch) in consensus.chars().enumerate() {
            let x = c.margin.left + di as f64 * actual_cell_w;
            let bg = if conservation[di] > 0.8 {
                "#333"
            } else if conservation[di] > 0.5 {
                "#888"
            } else {
                "#ccc"
            };
            c.elements.push(format!(
                r#"<rect x="{x:.1}" y="{cons_y:.1}" width="{actual_cell_w:.1}" height="{:.1}" fill="{bg}" />"#,
                cell_h - 1.0
            ));
            if actual_cell_w >= 8.0 {
                c.elements.push(format!(
                    r#"<text x="{:.1}" y="{:.1}" text-anchor="middle" font-size="{:.0}" font-family="monospace" fill="white" font-weight="bold">{}</text>"#,
                    x + actual_cell_w / 2.0, cons_y + cell_h * 0.7, (actual_cell_w * 0.8).min(11.0), cons_ch
                ));
            }
        }

        if end_pos - start > 100 {
            c.add_text(
                c.margin.left + c.plot_width() + 5.0,
                c.margin.top + 20.0,
                "...",
                "start",
                12.0,
            );
        }

        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let max_id = ids.iter().map(|s| s.len()).max().unwrap_or(4);
    let term_w = get_opt_usize(&opts, "width", 80);
    let show_len = display_len.min(term_w.saturating_sub(max_id + 3));
    let mut out = format!("  {title}\n");
    out.push_str(&format!("  {:>w$}  ", "", w = max_id));
    for di in 0..show_len {
        let pos = start + di;
        if pos % 10 == 0 {
            out.push('|');
        } else {
            out.push(' ');
        }
    }
    out.push('\n');
    for (si, seq) in sequences.iter().enumerate() {
        out.push_str(&format!("  {:>w$}  ", ids[si], w = max_id));
        for di in 0..show_len {
            let pos = start + di;
            out.push(seq.chars().nth(pos).unwrap_or('-'));
        }
        out.push('\n');
    }
    out.push_str(&format!("  {:>w$}  ", "Cons", w = max_id));
    for ch in consensus.chars().take(show_len) {
        out.push(ch);
    }
    out.push('\n');
    out.push_str(&format!("  {:>w$}  ", "", w = max_id));
    for &cv in conservation.iter().take(show_len) {
        out.push(if cv > 0.9 {
            '*'
        } else if cv > 0.5 {
            ':'
        } else if cv > 0.3 {
            '.'
        } else {
            ' '
        });
    }
    out.push('\n');
    write_output(&out);
    Ok(Value::Nil)
}

// ── 25. circos_plot ─────────────────────────────────────────────

#[derive(Clone, Debug)]
pub(super) struct CoverageDatum {
    source_row: usize,
    chromosome: Option<String>,
    original_start: f64,
    original_end: f64,
    start: f64,
    end: f64,
    value: f64,
    geometry: &'static str,
}

pub(super) fn coverage_value_from_record(map: &HashMap<String, Value>) -> Option<f64> {
    ["value", "coverage", "signal", "score"]
        .into_iter()
        .find_map(|key| map.get(key).and_then(Value::as_float))
}

pub(super) fn coverage_data(
    value: &Value,
    opts: &HashMap<String, Value>,
) -> Result<Vec<CoverageDatum>> {
    match value {
        Value::Table(table) => {
            let position_column = opts
                .get("pos")
                .and_then(Value::as_str)
                .and_then(|name| table.col_index(name))
                .or_else(|| table.col_index("pos"))
                .or_else(|| table.col_index("position"));
            let start_column = table.col_index(get_opt_str(opts, "start", "start"));
            let end_column = table.col_index(get_opt_str(opts, "end", "end"));
            if position_column.is_none() && (start_column.is_none() || end_column.is_none()) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "coverage_track() Table needs pos/position or both start and end columns",
                    None,
                ));
            }
            let value_column = if let Some(name) = opts.get("value").and_then(Value::as_str) {
                table.col_index(name)
            } else {
                ["value", "coverage", "signal", "score"]
                    .into_iter()
                    .find_map(|name| table.col_index(name))
            }
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "coverage_track() Table needs value, coverage, signal or score",
                    None,
                )
            })?;
            let chromosome_column = table.col_index(get_opt_str(opts, "chrom", "chrom"));
            table
                .rows
                .iter()
                .enumerate()
                .map(|(source_row, row)| {
                    let chromosome = chromosome_column
                        .map(|column| {
                            row[column]
                                .as_str()
                                .filter(|name| !name.trim().is_empty())
                                .map(str::to_string)
                                .ok_or_else(|| {
                                    BioLangError::runtime(
                                        ErrorKind::TypeError,
                                        "coverage_track() chromosome values must be non-empty Str",
                                        None,
                                    )
                                })
                        })
                        .transpose()?;
                    let (start, end, geometry) = if let Some(column) = position_column {
                        let position = row[column].as_float().unwrap_or(f64::NAN);
                        (position, position, "point")
                    } else {
                        (
                            row[start_column.unwrap()].as_float().unwrap_or(f64::NAN),
                            row[end_column.unwrap()].as_float().unwrap_or(f64::NAN),
                            "interval",
                        )
                    };
                    let value = row[value_column].as_float().unwrap_or(f64::NAN);
                    Ok(CoverageDatum {
                        source_row,
                        chromosome,
                        original_start: start,
                        original_end: end,
                        start,
                        end,
                        value,
                        geometry,
                    })
                })
                .collect()
        }
        Value::List(items) => items
            .iter()
            .enumerate()
            .map(|(source_row, item)| {
                let map = match item {
                    Value::Record(map) => map,
                    _ => {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            "coverage_track() List items must be Records",
                            None,
                        ))
                    }
                };
                let chromosome = map
                    .get("chrom")
                    .or_else(|| map.get("chromosome"))
                    .map(|value| {
                        value
                            .as_str()
                            .filter(|name| !name.trim().is_empty())
                            .map(str::to_string)
                            .ok_or_else(|| {
                                BioLangError::runtime(
                                    ErrorKind::TypeError,
                                    "coverage_track() chromosome values must be non-empty Str",
                                    None,
                                )
                            })
                    })
                    .transpose()?;
                let (start, end, geometry) = if let Some(position) = map
                    .get("pos")
                    .or_else(|| map.get("position"))
                    .and_then(Value::as_float)
                {
                    (position, position, "point")
                } else {
                    let start = map.get("start").and_then(Value::as_float).ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "coverage_track() record needs pos/position or numeric start",
                            None,
                        )
                    })?;
                    let end = map.get("end").and_then(Value::as_float).ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "coverage_track() interval record needs numeric end",
                            None,
                        )
                    })?;
                    (start, end, "interval")
                };
                let value = coverage_value_from_record(map).ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        "coverage_track() record needs numeric value, coverage, signal or score",
                        None,
                    )
                })?;
                Ok(CoverageDatum {
                    source_row,
                    chromosome,
                    original_start: start,
                    original_end: end,
                    start,
                    end,
                    value,
                    geometry,
                })
            })
            .collect(),
        _ => Err(BioLangError::type_error(
            "coverage_track() requires Table or List of Records",
            None,
        )),
    }
}

pub(super) fn valid_bio_hex_color(color: &str) -> bool {
    color.len() == 7
        && color.starts_with('#')
        && color[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn coverage_track_spec_value(
    value: &Value,
    opts: &HashMap<String, Value>,
) -> Result<Value> {
    let mut data = coverage_data(value, opts)?;
    let input_rows = data.len();
    if data.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() empty data",
            None,
        ));
    }
    if data.iter().any(|datum| {
        !datum.start.is_finite()
            || !datum.end.is_finite()
            || !datum.value.is_finite()
            || datum.start < 0.0
            || datum.end < datum.start
            || (datum.geometry == "interval" && datum.end <= datum.start)
    }) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() coordinates and values must be finite, positions non-negative, and interval end > start",
            None,
        ));
    }
    let requested_chromosome = opts.get("chromosome").and_then(Value::as_str);
    if let Some(chromosome) = requested_chromosome {
        if chromosome.trim().is_empty() || data.iter().all(|datum| datum.chromosome.is_none()) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "coverage_track() chromosome filtering needs a non-empty name and chromosome data",
                None,
            ));
        }
        data.retain(|datum| datum.chromosome.as_deref() == Some(chromosome));
    }
    let observed_chromosomes = data
        .iter()
        .filter_map(|datum| datum.chromosome.as_deref())
        .collect::<HashSet<_>>();
    let rows_with_chromosomes = data
        .iter()
        .filter(|datum| datum.chromosome.is_some())
        .count();
    if rows_with_chromosomes != 0 && rows_with_chromosomes != data.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() chromosome values must be present for every row or omitted from every row",
            None,
        ));
    }
    if observed_chromosomes.len() > 1 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() draws one genomic region; select one chromosome with {chromosome: \"chr1\"}",
            None,
        ));
    }
    let selected_chromosome = observed_chromosomes.into_iter().next().map(str::to_string);
    let region_start = opts.get("region_start").and_then(Value::as_float);
    let region_end = opts.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|value| !value.is_finite() || value < 0.0)
        || region_end.is_some_and(|value| !value.is_finite() || value < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() region_start/region_end must be finite, non-negative and increasing",
            None,
        ));
    }
    let mut clipped_rows = 0usize;
    data = data
        .into_iter()
        .filter_map(|mut datum| {
            if datum.geometry == "point" {
                let keep = region_start.is_none_or(|start| datum.start >= start)
                    && region_end.is_none_or(|end| datum.start <= end);
                keep.then_some(datum)
            } else {
                if region_start.is_some_and(|start| datum.end <= start)
                    || region_end.is_some_and(|end| datum.start >= end)
                {
                    return None;
                }
                let clipped_start =
                    region_start.map_or(datum.start, |start| datum.start.max(start));
                let clipped_end = region_end.map_or(datum.end, |end| datum.end.min(end));
                if clipped_start != datum.start || clipped_end != datum.end {
                    clipped_rows += 1;
                }
                datum.start = clipped_start;
                datum.end = clipped_end;
                (datum.end > datum.start).then_some(datum)
            }
        })
        .collect();
    if data.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() no data after chromosome/region filtering",
            None,
        ));
    }
    data.sort_by(|left, right| {
        left.start
            .total_cmp(&right.start)
            .then_with(|| left.end.total_cmp(&right.end))
            .then_with(|| left.source_row.cmp(&right.source_row))
    });
    let retained_rows = data.len();
    let rows = data
        .iter()
        .enumerate()
        .map(|(index, datum)| {
            vec![
                Value::Int(index as i64),
                Value::Int(datum.source_row as i64),
                datum
                    .chromosome
                    .clone()
                    .map(Value::Str)
                    .unwrap_or(Value::Nil),
                Value::Float(datum.original_start),
                Value::Float(datum.original_end),
                Value::Float(datum.start),
                Value::Float(datum.end),
                Value::Float(if datum.geometry == "point" {
                    datum.start
                } else {
                    (datum.start + datum.end) / 2.0
                }),
                Value::Float(datum.value),
                Value::Str(datum.geometry.into()),
                Value::Bool(datum.start != datum.original_start || datum.end != datum.original_end),
            ]
        })
        .collect();
    let title = get_opt_str(opts, "title", "Coverage Track");
    let color = get_opt_str(opts, "color", "#4e79a7");
    if !valid_bio_hex_color(color) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "coverage_track() color must be a #rrggbb value",
            None,
        ));
    }
    let warnings = if input_rows == retained_rows && clipped_rows == 0 {
        Vec::new()
    } else {
        vec![Value::Str(format!(
            "{} rows retained from {input_rows}; {clipped_rows} overlapping intervals clipped to the requested region",
            retained_rows
        ))]
    };
    Ok(Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("coverage_track".into())),
            ("title".into(), Value::Str(title.into())),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "point_index",
                        "source_row",
                        "chromosome",
                        "original_start",
                        "original_end",
                        "start",
                        "end",
                        "position",
                        "value",
                        "geometry",
                        "clipped",
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
                            Value::Float(get_opt_f64(opts, "width", 900.0)),
                        ),
                        (
                            "height".into(),
                            Value::Float(get_opt_f64(opts, "height", 280.0)),
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
                            Value::Str(get_opt_str(opts, "xlabel", "Genomic position").into()),
                        ),
                        (
                            "ylabel".into(),
                            Value::Str(get_opt_str(opts, "ylabel", "Coverage / signal").into()),
                        ),
                        ("color".into(), Value::Str(color.into())),
                        (
                            "chromosome".into(),
                            selected_chromosome
                                .clone()
                                .map(Value::Str)
                                .unwrap_or(Value::Nil),
                        ),
                        (
                            "region_start".into(),
                            region_start.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                        (
                            "region_end".into(),
                            region_end.map(Value::Float).unwrap_or(Value::Nil),
                        ),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("coverage_track".into())),
                        ("input_rows".into(), Value::Int(input_rows as i64)),
                        ("retained_rows".into(), Value::Int(retained_rows as i64)),
                        ("clipped_rows".into(), Value::Int(clipped_rows as i64)),
                        (
                            "row_order".into(),
                            Value::Str("start_end_source_row".into()),
                        ),
                        (
                            "region_rule".into(),
                            Value::Str("points_inclusive_intervals_overlap_half_open".into()),
                        ),
                        (
                            "coordinate_convention".into(),
                            Value::Str("zero_based_half_open_for_intervals".into()),
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

pub(super) fn render_coverage_track_svg(
    table: &Table,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let starts = extract_table_col(table, "start")?;
    let ends = extract_table_col(table, "end")?;
    let positions = extract_table_col(table, "position")?;
    let values = extract_table_col(table, "value")?;
    let geometry_column = table.col_index("geometry").unwrap();
    let width = get_opt_f64(opts, "width", 900.0);
    let height = get_opt_f64(opts, "height", 280.0);
    let title = get_opt_str(opts, "title", "Coverage Track");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    let x_label = get_opt_str(opts, "xlabel", "Genomic position");
    let y_label = get_opt_str(opts, "ylabel", "Coverage / signal");
    let color = get_opt_str(opts, "color", "#4e79a7");
    let mut x_min = starts.iter().copied().fold(f64::INFINITY, f64::min);
    let mut x_max = ends.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if let Some(region_start) = opts.get("region_start").and_then(Value::as_float) {
        x_min = region_start;
    }
    if let Some(region_end) = opts.get("region_end").and_then(Value::as_float) {
        x_max = region_end;
    }
    if x_max <= x_min {
        x_min -= 0.5;
        x_max += 0.5;
    }
    let raw_min = values.iter().copied().fold(0.0, f64::min);
    let raw_max = values.iter().copied().fold(0.0, f64::max);
    let spread = (raw_max - raw_min).max(raw_max.abs() * 0.05).max(1.0);
    let y_domain = (
        if raw_min < 0.0 {
            raw_min - spread * 0.05
        } else {
            0.0
        },
        if raw_max > 0.0 {
            raw_max + spread * 0.05
        } else {
            0.0
        },
    );
    let y_domain = if y_domain.1 <= y_domain.0 {
        (y_domain.0 - 0.5, y_domain.1 + 0.5)
    } else {
        y_domain
    };
    let x_domain = (x_min, x_max);
    let mut canvas = SvgCanvas::with_theme(width, height, plot_theme(opts));
    canvas.fit_cartesian_layout(
        &[x_min, x_max],
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
    let baseline = y_scale.map(0.0);
    let mut interval_fill = String::new();
    let mut interval_line = String::new();
    let mut point_indexes = Vec::new();
    for (index, row) in table.rows.iter().enumerate() {
        if row[geometry_column].as_str() == Some("interval") {
            let x1 = x_scale.map(starts[index]);
            let x2 = x_scale.map(ends[index]);
            let y = y_scale.map(values[index]);
            interval_fill.push_str(&format!(
                "M{:.2},{:.2}V{:.2}H{:.2}V{:.2}Z",
                x1, baseline, y, x2, baseline
            ));
            interval_line.push_str(&format!("M{:.2},{:.2}H{:.2}", x1, y, x2));
        } else {
            point_indexes.push(index);
        }
    }
    if !interval_fill.is_empty() {
        canvas.elements.push(format!(
            r#"<path d="{interval_fill}" fill="{color}" fill-opacity="0.34" stroke="none" />"#
        ));
        canvas.elements.push(format!(
            r#"<path d="{interval_line}" fill="none" stroke="{color}" stroke-width="1.35" />"#
        ));
    }
    if !point_indexes.is_empty() {
        let mut area = format!(
            "M{:.2},{:.2}",
            x_scale.map(positions[point_indexes[0]]),
            baseline
        );
        let mut line = String::new();
        for &index in &point_indexes {
            let x = x_scale.map(positions[index]);
            let y = y_scale.map(values[index]);
            area.push_str(&format!("L{:.2},{:.2}", x, y));
            if line.is_empty() {
                line.push_str(&format!("M{:.2},{:.2}", x, y));
            } else {
                line.push_str(&format!("L{:.2},{:.2}", x, y));
            }
        }
        area.push_str(&format!(
            "L{:.2},{:.2}Z",
            x_scale.map(positions[*point_indexes.last().unwrap()]),
            baseline
        ));
        canvas.elements.push(format!(
            r#"<path d="{area}" fill="{color}" fill-opacity="0.26" stroke="none" />"#
        ));
        canvas.elements.push(format!(
            r#"<path d="{line}" fill="none" stroke="{color}" stroke-width="1.5" stroke-linejoin="round" />"#
        ));
        if point_indexes.len() == 1 {
            let index = point_indexes[0];
            canvas.add_circle(
                x_scale.map(positions[index]),
                y_scale.map(values[index]),
                2.8,
                color,
            );
        }
    }
    canvas.draw_x_axis(
        &Scale {
            domain: x_domain,
            range: x_domain,
        },
        x_label,
    );
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
    let interval_count = table
        .rows
        .iter()
        .filter(|row| row[geometry_column].as_str() == Some("interval"))
        .count();
    canvas.set_accessible_description(format!(
        "Coverage track with {} observations: {interval_count} intervals and {} point samples.",
        table.num_rows(),
        table.num_rows() - interval_count
    ));
    Ok(canvas.render())
}

pub(crate) fn is_coverage_track_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
            && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "coverage_track"))
}

pub(crate) fn render_coverage_track_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_coverage_track_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 coverage-track Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() coverage-track specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "point_index",
        "source_row",
        "chromosome",
        "original_start",
        "original_end",
        "start",
        "end",
        "position",
        "value",
        "geometry",
        "clipped",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() coverage-track data is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() coverage-track specification has no observations",
            None,
        ));
    }
    let options = frozen_spec_options(map, render_options, "coverage-track")?;
    let color = get_opt_str(&options, "color", "");
    if !valid_bio_hex_color(color) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() coverage-track color must be a #rrggbb value",
            None,
        ));
    }
    let point_column = table.col_index("point_index").unwrap();
    let source_column = table.col_index("source_row").unwrap();
    let chromosome_column = table.col_index("chromosome").unwrap();
    let original_start_column = table.col_index("original_start").unwrap();
    let original_end_column = table.col_index("original_end").unwrap();
    let start_column = table.col_index("start").unwrap();
    let end_column = table.col_index("end").unwrap();
    let position_column = table.col_index("position").unwrap();
    let value_column = table.col_index("value").unwrap();
    let geometry_column = table.col_index("geometry").unwrap();
    let clipped_column = table.col_index("clipped").unwrap();
    let selected_chromosome = options.get("chromosome").and_then(Value::as_str);
    let region_start = options.get("region_start").and_then(Value::as_float);
    let region_end = options.get("region_end").and_then(Value::as_float);
    if region_start.is_some_and(|number| !number.is_finite() || number < 0.0)
        || region_end.is_some_and(|number| !number.is_finite() || number < 0.0)
        || matches!((region_start, region_end), (Some(start), Some(end)) if end <= start)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() coverage-track region is malformed",
            None,
        ));
    }
    let mut previous: Option<(f64, f64, usize)> = None;
    for (expected, row) in table.rows.iter().enumerate() {
        let point_index =
            frozen_nonnegative_integer(&row[point_column], "coverage-track", "point_index")?;
        let source =
            frozen_nonnegative_integer(&row[source_column], "coverage-track", "source_row")?;
        let original_start = row[original_start_column].as_float().unwrap_or(f64::NAN);
        let original_end = row[original_end_column].as_float().unwrap_or(f64::NAN);
        let start = row[start_column].as_float().unwrap_or(f64::NAN);
        let end = row[end_column].as_float().unwrap_or(f64::NAN);
        let position = row[position_column].as_float().unwrap_or(f64::NAN);
        let datum = row[value_column].as_float().unwrap_or(f64::NAN);
        let geometry = row[geometry_column].as_str().unwrap_or("");
        let chromosome = match &row[chromosome_column] {
            Value::Nil => None,
            Value::Str(name) if !name.is_empty() => Some(name.as_str()),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() coverage-track chromosome is malformed",
                    None,
                ))
            }
        };
        let key = (start, end, source);
        let expected_position = if geometry == "point" {
            start
        } else {
            (start + end) / 2.0
        };
        let expected_start = if geometry == "point" {
            original_start
        } else {
            region_start.map_or(original_start, |lower| original_start.max(lower))
        };
        let expected_end = if geometry == "point" {
            original_end
        } else {
            region_end.map_or(original_end, |upper| original_end.min(upper))
        };
        let expected_clipped = start != original_start || end != original_end;
        if point_index != expected
            || !original_start.is_finite()
            || !original_end.is_finite()
            || !start.is_finite()
            || !end.is_finite()
            || !position.is_finite()
            || !datum.is_finite()
            || start < 0.0
            || end < start
            || !matches!(geometry, "point" | "interval")
            || (geometry == "point" && (end != start || original_end != original_start))
            || (geometry == "interval" && (end <= start || original_end <= original_start))
            || (start - expected_start).abs() > 1e-10
            || (end - expected_end).abs() > 1e-10
            || (position - expected_position).abs() > 1e-10
            || !matches!(row[clipped_column], Value::Bool(_))
            || row[clipped_column].is_truthy() != expected_clipped
            || selected_chromosome != chromosome
            || region_start.is_some_and(|lower| start < lower)
            || region_end.is_some_and(|upper| end > upper)
            || previous.is_some_and(|old| key < old)
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() coverage-track frozen geometry is inconsistent",
                None,
            ));
        }
        previous = Some(key);
    }
    let format = get_opt_str(render_options, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data") {
        return Ok(value.clone());
    }
    let svg = render_coverage_track_svg(table, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Coverage Track");
    finish_frozen_bio_plot(value, render_options, title, "coverage-track", svg)
}

/// Genome browser-style coverage track. Interval inputs retain and clip their
/// real spans; point inputs retain their sampled positions.
pub(super) fn builtin_coverage_track(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let specification = coverage_track_spec_value(&args[0], &opts)?;
    render_coverage_track_plot_spec_value(&specification, &opts)
}
