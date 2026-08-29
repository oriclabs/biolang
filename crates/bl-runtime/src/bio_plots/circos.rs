//! Circos for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

#[derive(Clone, Debug)]
pub(super) struct CircosChromosome {
    source_row: usize,
    name: String,
    start: f64,
    end: f64,
    angle_start: f64,
    angle_end: f64,
    color: String,
    label_drawn: bool,
}

#[derive(Clone, Debug)]
pub(super) struct CircosTrackMeta {
    index: usize,
    name: String,
    kind: String,
    radial_inner: f64,
    radial_outer: f64,
    value_min: f64,
    value_max: f64,
}

#[derive(Clone, Debug)]
pub(super) struct CircosTrackMark {
    track_index: usize,
    point_index: usize,
    source_row: usize,
    chromosome_index: usize,
    chromosome: String,
    start: f64,
    end: f64,
    value: f64,
    angle_start: f64,
    angle_end: f64,
    radial_inner: f64,
    radial_outer: f64,
    color: String,
    label: Option<String>,
    label_drawn: bool,
    clipped: bool,
}

#[derive(Clone, Debug)]
pub(super) struct CircosLink {
    link_index: usize,
    source_row: usize,
    source_chromosome_index: usize,
    source_chromosome: String,
    source_start: f64,
    source_end: f64,
    target_chromosome_index: usize,
    target_chromosome: String,
    target_start: f64,
    target_end: f64,
    source_angle_start: f64,
    source_angle_end: f64,
    target_angle_start: f64,
    target_angle_end: f64,
    weight: f64,
    stroke_width: f64,
    color: String,
    label: Option<String>,
    label_drawn: bool,
}

pub(super) fn table_column_alias(table: &Table, names: &[&str]) -> Option<usize> {
    names.iter().find_map(|name| table.col_index(name))
}

pub(super) fn finite_table_number(
    row: &[Value],
    column: usize,
    family: &str,
    field: &str,
) -> Result<f64> {
    let value = row[column].as_float().ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{family}() column '{field}' must be numeric"),
            None,
        )
    })?;
    if !value.is_finite() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("{family}() column '{field}' must be finite"),
            None,
        ));
    }
    Ok(value)
}

pub(super) fn required_circos_column(
    table: &Table,
    names: &[&str],
    description: &str,
) -> Result<usize> {
    table_column_alias(table, names).ok_or_else(|| {
        BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "circos() {description} is missing column '{}'",
                names.join("' or '")
            ),
            None,
        )
    })
}

pub(super) fn circos_chromosome_angle(chromosome: &CircosChromosome, position: f64) -> f64 {
    chromosome.angle_start
        + (position - chromosome.start) / (chromosome.end - chromosome.start)
            * (chromosome.angle_end - chromosome.angle_start)
}

pub(super) fn normalized_track_kind(kind: &str) -> Result<String> {
    let normalized = match kind.to_ascii_lowercase().as_str() {
        "bar" => "bar",
        "line" | "coverage" => "line",
        "point" | "points" | "variant" | "variants" => "point",
        "heatmap" | "tile" | "tiles" => "heatmap",
        "cnv" | "copy_number" | "copy-number" => "cnv",
        "gene" | "genes" | "peak" | "peaks" | "interval" | "intervals" => "interval",
        other => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "circos() unknown track type '{other}', expected bar/line/point/heatmap/cnv/interval"
                ),
                None,
            ))
        }
    };
    Ok(normalized.to_string())
}

pub(super) fn circos_track_records(value: Option<&Value>) -> Result<Vec<(String, String, Table)>> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let mut tracks = Vec::new();
    let parse_track = |fallback_name: String, item: &Value| -> Result<(String, String, Table)> {
        let Value::Record(record) = item else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() each tracks entry must be a Record with data and type",
                None,
            ));
        };
        let data = match record.get("data") {
            Some(Value::Table(table)) => table.clone(),
            _ => {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() each track needs a data Table",
                    None,
                ))
            }
        };
        let name = record
            .get("name")
            .or_else(|| record.get("label"))
            .and_then(Value::as_str)
            .unwrap_or(&fallback_name)
            .to_string();
        let kind = normalized_track_kind(
            record
                .get("type")
                .or_else(|| record.get("track"))
                .and_then(Value::as_str)
                .unwrap_or("bar"),
        )?;
        Ok((name, kind, data))
    };
    match value {
        Value::List(items) => {
            for (index, item) in items.iter().enumerate() {
                tracks.push(parse_track(format!("track {}", index + 1), item)?);
            }
        }
        Value::Record(record) => {
            let mut names = record.keys().cloned().collect::<Vec<_>>();
            names.sort();
            for name in names {
                let item = record.get(&name).unwrap();
                match item {
                    Value::Table(table) => tracks.push((name, "bar".into(), table.clone())),
                    Value::Record(_) => tracks.push(parse_track(name, item)?),
                    _ => {
                        return Err(BioLangError::runtime(
                            ErrorKind::TypeError,
                            "circos() named tracks must contain Tables or track Records",
                            None,
                        ))
                    }
                }
            }
        }
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() tracks must be a List or Record",
                None,
            ))
        }
    }
    Ok(tracks)
}

pub(super) fn circos_spec_value(value: &Value, opts: &HashMap<String, Value>) -> Result<Value> {
    let Value::Record(input) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() requires {segments: Table, links?: Table, tracks?: List}",
            None,
        ));
    };
    let segments = match input.get("segments").or_else(|| input.get("chromosomes")) {
        Some(Value::Table(table)) if !table.rows.is_empty() => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() requires a non-empty segments Table",
                None,
            ))
        }
    };
    let chrom_column =
        required_circos_column(segments, &["chrom", "chr", "chromosome"], "segments")?;
    let end_column = required_circos_column(segments, &["end", "length", "size"], "segments")?;
    let start_column = table_column_alias(segments, &["start"]);
    let color_column = table_column_alias(segments, &["color", "colour"]);
    let width = get_opt_f64(opts, "width", 700.0);
    let height = get_opt_f64(opts, "height", 700.0);
    if !width.is_finite() || !height.is_finite() || width < 160.0 || height < 160.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() width and height must be finite and at least 160",
            None,
        ));
    }
    let gap_degrees = get_opt_f64(opts, "gap_degrees", 2.0);
    let start_degrees = get_opt_f64(opts, "start_degrees", -90.0);
    if !gap_degrees.is_finite() || gap_degrees < 0.0 || !start_degrees.is_finite() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() gap_degrees must be non-negative and angles must be finite",
            None,
        ));
    }
    let mut chromosomes = Vec::<CircosChromosome>::new();
    let mut seen = HashSet::<String>::new();
    for (source_row, row) in segments.rows.iter().enumerate() {
        let name = row[chrom_column]
            .as_str()
            .filter(|name| !name.trim().is_empty())
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() chromosome names must be non-empty strings",
                    None,
                )
            })?
            .to_string();
        if !seen.insert(name.clone()) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("circos() chromosome '{name}' appears more than once"),
                None,
            ));
        }
        let start = match start_column {
            Some(column) => finite_table_number(row, column, "circos", "start")?,
            None => 0.0,
        };
        let end = finite_table_number(row, end_column, "circos", "end")?;
        if start < 0.0 || end <= start {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() chromosome intervals require 0 <= start < end",
                None,
            ));
        }
        let color = color_column
            .and_then(|column| row[column].as_str())
            .unwrap_or(PALETTE[source_row % PALETTE.len()])
            .to_string();
        if !valid_bio_hex_color(&color) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() segment colors must be #rrggbb",
                None,
            ));
        }
        chromosomes.push(CircosChromosome {
            source_row,
            name,
            start,
            end,
            angle_start: 0.0,
            angle_end: 0.0,
            color,
            label_drawn: false,
        });
    }
    let gap = gap_degrees.to_radians();
    let available = std::f64::consts::TAU - gap * chromosomes.len() as f64;
    if available <= 0.05 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() chromosome gaps leave no drawable circle",
            None,
        ));
    }
    let total_size = chromosomes
        .iter()
        .map(|chromosome| chromosome.end - chromosome.start)
        .sum::<f64>();
    let mut angle = start_degrees.to_radians();
    let label_radius = width.min(height) * 0.39;
    let max_labels = get_opt_usize(opts, "max_labels", chromosomes.len());
    for (index, chromosome) in chromosomes.iter_mut().enumerate() {
        let sweep = (chromosome.end - chromosome.start) / total_size * available;
        chromosome.angle_start = angle;
        chromosome.angle_end = angle + sweep;
        chromosome.label_drawn = index < max_labels
            && sweep * label_radius
                >= estimate_text_width(&chromosome.name, plot_theme(opts).legend_size) + 4.0;
        angle += sweep + gap;
    }
    let lookup = chromosomes
        .iter()
        .enumerate()
        .map(|(index, chromosome)| (chromosome.name.clone(), index))
        .collect::<HashMap<_, _>>();

    let raw_tracks = circos_track_records(input.get("tracks"))?;
    let track_band = get_opt_f64(opts, "track_width", 0.075);
    let track_gap = get_opt_f64(opts, "track_gap", 0.018);
    if !track_band.is_finite()
        || !track_gap.is_finite()
        || track_band <= 0.0
        || track_gap < 0.0
        || 0.86 - raw_tracks.len() as f64 * (track_band + track_gap) < 0.18
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos() track widths leave too little room for the chord layer",
            None,
        ));
    }
    let mut track_meta = Vec::<CircosTrackMeta>::new();
    let mut track_marks = Vec::<CircosTrackMark>::new();
    let mut label_budget = get_opt_usize(opts, "max_track_labels", 24);
    for (track_index, (name, kind, table)) in raw_tracks.iter().enumerate() {
        let track_chrom_column = required_circos_column(
            table,
            &["chrom", "chr", "chromosome"],
            &format!("track '{name}'"),
        )?;
        let point_column = table_column_alias(table, &["pos", "position"]);
        let mark_start_column = table_column_alias(table, &["start"])
            .or(point_column)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' needs start or position"),
                    None,
                )
            })?;
        let mark_end_column = table_column_alias(table, &["end"]);
        let value_column =
            table_column_alias(table, &["value", "score", "coverage", "depth", "log2ratio"]);
        let label_column = table_column_alias(table, &["label", "name", "gene"]);
        let mark_color_column = table_column_alias(table, &["color", "colour"]);
        let mut raw = Vec::<(
            usize,
            usize,
            f64,
            f64,
            f64,
            Option<String>,
            Option<String>,
            bool,
        )>::new();
        for (source_row, row) in table.rows.iter().enumerate() {
            let chromosome_name = row[track_chrom_column].as_str().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' chromosome must be a string"),
                    None,
                )
            })?;
            let chromosome_index = *lookup.get(chromosome_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "circos() track '{name}' references unknown chromosome '{chromosome_name}'"
                    ),
                    None,
                )
            })?;
            let chromosome = &chromosomes[chromosome_index];
            let original_start = finite_table_number(row, mark_start_column, "circos", "start")?;
            let original_end = match mark_end_column {
                Some(column) => finite_table_number(row, column, "circos", "end")?,
                None => original_start,
            };
            if original_start < 0.0 || original_end < original_start {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' requires 0 <= start <= end"),
                    None,
                ));
            }
            if original_end < chromosome.start || original_start > chromosome.end {
                continue;
            }
            let start = original_start.max(chromosome.start).min(chromosome.end);
            let end = original_end.max(chromosome.start).min(chromosome.end);
            let value = match value_column {
                Some(column) => finite_table_number(row, column, "circos", "value")?,
                None => 1.0,
            };
            let label = label_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            let color = mark_color_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            if color
                .as_deref()
                .is_some_and(|color| !valid_bio_hex_color(color))
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() track '{name}' colors must be #rrggbb"),
                    None,
                ));
            }
            raw.push((
                source_row,
                chromosome_index,
                start,
                end,
                value,
                label,
                color,
                start != original_start || end != original_end,
            ));
        }
        raw.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.2.total_cmp(&right.2))
                .then_with(|| left.3.total_cmp(&right.3))
                .then_with(|| left.0.cmp(&right.0))
        });
        let value_min = raw.iter().map(|row| row.4).fold(f64::INFINITY, f64::min);
        let value_max = raw
            .iter()
            .map(|row| row.4)
            .fold(f64::NEG_INFINITY, f64::max);
        let (value_min, value_max) = if raw.is_empty() {
            (0.0, 1.0)
        } else if (value_max - value_min).abs() < f64::EPSILON {
            (value_min.min(0.0), value_max.max(1.0))
        } else {
            (value_min, value_max)
        };
        let radial_outer = 0.86 - track_index as f64 * (track_band + track_gap);
        let radial_inner = radial_outer - track_band;
        track_meta.push(CircosTrackMeta {
            index: track_index,
            name: name.clone(),
            kind: kind.clone(),
            radial_inner,
            radial_outer,
            value_min,
            value_max,
        });
        for (
            point_index,
            (source_row, chromosome_index, start, end, value, label, explicit_color, clipped),
        ) in raw.into_iter().enumerate()
        {
            let chromosome = &chromosomes[chromosome_index];
            let normalized = ((value - value_min) / (value_max - value_min)).clamp(0.0, 1.0);
            let color = explicit_color.unwrap_or_else(|| match kind.as_str() {
                "cnv" => {
                    let magnitude = value_min.abs().max(value_max.abs()).max(f64::EPSILON);
                    publication_diverging_color((value / magnitude + 1.0) / 2.0)
                }
                "heatmap" | "bar" => publication_sequential_color(normalized),
                "line" => PALETTE[track_index % PALETTE.len()].to_string(),
                _ => PALETTE[track_index % PALETTE.len()].to_string(),
            });
            let radial_value = radial_inner + normalized * (radial_outer - radial_inner);
            let enough_arc = (circos_chromosome_angle(chromosome, end)
                - circos_chromosome_angle(chromosome, start))
            .abs()
                * label_radius
                >= label
                    .as_deref()
                    .map(|text| estimate_text_width(text, 8.0) + 3.0)
                    .unwrap_or(f64::INFINITY);
            let label_drawn =
                label.is_some() && label_budget > 0 && (kind == "point" || enough_arc);
            if label_drawn {
                label_budget -= 1;
            }
            track_marks.push(CircosTrackMark {
                track_index,
                point_index,
                source_row,
                chromosome_index,
                chromosome: chromosome.name.clone(),
                start,
                end,
                value,
                angle_start: circos_chromosome_angle(chromosome, start),
                angle_end: circos_chromosome_angle(chromosome, end),
                radial_inner,
                radial_outer: if kind == "bar" || kind == "line" || kind == "point" {
                    radial_value
                } else {
                    radial_outer
                },
                color,
                label,
                label_drawn,
                clipped,
            });
        }
    }

    let mut links = Vec::<CircosLink>::new();
    if let Some(link_value) = input.get("links") {
        let Value::Table(table) = link_value else {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "circos() links must be a Table",
                None,
            ));
        };
        let source_chrom_column = required_circos_column(
            table,
            &["source_chr", "source_chrom", "chrom1", "from_chr"],
            "links",
        )?;
        let target_chrom_column = required_circos_column(
            table,
            &["target_chr", "target_chrom", "chrom2", "to_chr"],
            "links",
        )?;
        let source_start_column = required_circos_column(
            table,
            &["source_start", "source_pos", "pos1", "from_pos"],
            "links",
        )?;
        let target_start_column = required_circos_column(
            table,
            &["target_start", "target_pos", "pos2", "to_pos"],
            "links",
        )?;
        let source_end_column = table_column_alias(table, &["source_end", "end1"]);
        let target_end_column = table_column_alias(table, &["target_end", "end2"]);
        let weight_column = table_column_alias(table, &["weight", "value", "count", "score"]);
        let link_color_column = table_column_alias(table, &["color", "colour"]);
        let link_label_column = table_column_alias(table, &["label", "name"]);
        let mut raw_links = Vec::<(
            usize,
            usize,
            String,
            f64,
            f64,
            usize,
            String,
            f64,
            f64,
            f64,
            Option<String>,
            Option<String>,
        )>::new();
        for (source_row, row) in table.rows.iter().enumerate() {
            let source_name = row[source_chrom_column].as_str().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link chromosome must be a string",
                    None,
                )
            })?;
            let target_name = row[target_chrom_column].as_str().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link chromosome must be a string",
                    None,
                )
            })?;
            let source_index = *lookup.get(source_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() link references unknown chromosome '{source_name}'"),
                    None,
                )
            })?;
            let target_index = *lookup.get(target_name).ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("circos() link references unknown chromosome '{target_name}'"),
                    None,
                )
            })?;
            let source_start =
                finite_table_number(row, source_start_column, "circos", "source_start")?;
            let target_start =
                finite_table_number(row, target_start_column, "circos", "target_start")?;
            let source_end = match source_end_column {
                Some(column) => finite_table_number(row, column, "circos", "source_end")?,
                None => source_start,
            };
            let target_end = match target_end_column {
                Some(column) => finite_table_number(row, column, "circos", "target_end")?,
                None => target_start,
            };
            let source_chromosome = &chromosomes[source_index];
            let target_chromosome = &chromosomes[target_index];
            if source_start < source_chromosome.start
                || source_end > source_chromosome.end
                || source_end < source_start
                || target_start < target_chromosome.start
                || target_end > target_chromosome.end
                || target_end < target_start
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link endpoints must lie inside their chromosome intervals",
                    None,
                ));
            }
            let weight = match weight_column {
                Some(column) => finite_table_number(row, column, "circos", "weight")?,
                None => 1.0,
            };
            if weight < 0.0 {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link weights must be non-negative",
                    None,
                ));
            }
            let color = link_color_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            if color
                .as_deref()
                .is_some_and(|color| !valid_bio_hex_color(color))
            {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "circos() link colors must be #rrggbb",
                    None,
                ));
            }
            let label = link_label_column
                .and_then(|column| row[column].as_str())
                .map(str::to_string);
            raw_links.push((
                source_row,
                source_index,
                source_name.to_string(),
                source_start,
                source_end,
                target_index,
                target_name.to_string(),
                target_start,
                target_end,
                weight,
                color,
                label,
            ));
        }
        raw_links.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| left.3.total_cmp(&right.3))
                .then_with(|| left.5.cmp(&right.5))
                .then_with(|| left.7.total_cmp(&right.7))
                .then_with(|| left.0.cmp(&right.0))
        });
        let max_weight = raw_links
            .iter()
            .map(|row| row.9)
            .fold(0.0f64, f64::max)
            .max(1.0);
        let mut ranked = (0..raw_links.len()).collect::<Vec<_>>();
        ranked.sort_by(|left, right| {
            raw_links[*right]
                .9
                .total_cmp(&raw_links[*left].9)
                .then_with(|| raw_links[*left].0.cmp(&raw_links[*right].0))
        });
        let labelled = ranked
            .into_iter()
            .take(get_opt_usize(opts, "max_link_labels", 12))
            .collect::<HashSet<_>>();
        for (link_index, row) in raw_links.into_iter().enumerate() {
            let source_chromosome = &chromosomes[row.1];
            let target_chromosome = &chromosomes[row.5];
            let strength = (row.9 / max_weight).sqrt();
            links.push(CircosLink {
                link_index,
                source_row: row.0,
                source_chromosome_index: row.1,
                source_chromosome: row.2,
                source_start: row.3,
                source_end: row.4,
                target_chromosome_index: row.5,
                target_chromosome: row.6,
                target_start: row.7,
                target_end: row.8,
                source_angle_start: circos_chromosome_angle(source_chromosome, row.3),
                source_angle_end: circos_chromosome_angle(source_chromosome, row.4),
                target_angle_start: circos_chromosome_angle(target_chromosome, row.7),
                target_angle_end: circos_chromosome_angle(target_chromosome, row.8),
                weight: row.9,
                stroke_width: 0.75 + (strength * 3.0 * 4.0).round() / 4.0,
                color: row.10.unwrap_or_else(|| chromosomes[row.1].color.clone()),
                label_drawn: row.11.is_some() && labelled.contains(&link_index),
                label: row.11,
            });
        }
    }

    let segment_table = Value::Table(Table::new(
        vec![
            "chromosome_index".into(),
            "source_row".into(),
            "chromosome".into(),
            "start".into(),
            "end".into(),
            "size".into(),
            "angle_start".into(),
            "angle_end".into(),
            "color".into(),
            "label_drawn".into(),
        ],
        chromosomes
            .iter()
            .enumerate()
            .map(|(index, chromosome)| {
                vec![
                    Value::Int(index as i64),
                    Value::Int(chromosome.source_row as i64),
                    Value::Str(chromosome.name.clone().into()),
                    Value::Float(chromosome.start),
                    Value::Float(chromosome.end),
                    Value::Float(chromosome.end - chromosome.start),
                    Value::Float(chromosome.angle_start),
                    Value::Float(chromosome.angle_end),
                    Value::Str(chromosome.color.clone().into()),
                    Value::Bool(chromosome.label_drawn),
                ]
            })
            .collect(),
    ));
    let track_metadata = Value::Table(Table::new(
        vec![
            "track_index".into(),
            "name".into(),
            "type".into(),
            "radial_inner".into(),
            "radial_outer".into(),
            "value_min".into(),
            "value_max".into(),
        ],
        track_meta
            .iter()
            .map(|track| {
                vec![
                    Value::Int(track.index as i64),
                    Value::Str(track.name.clone().into()),
                    Value::Str(track.kind.clone().into()),
                    Value::Float(track.radial_inner),
                    Value::Float(track.radial_outer),
                    Value::Float(track.value_min),
                    Value::Float(track.value_max),
                ]
            })
            .collect(),
    ));
    let track_table = Value::Table(Table::new(
        vec![
            "track_index".into(),
            "point_index".into(),
            "source_row".into(),
            "chromosome_index".into(),
            "chromosome".into(),
            "start".into(),
            "end".into(),
            "value".into(),
            "angle_start".into(),
            "angle_end".into(),
            "radial_inner".into(),
            "radial_outer".into(),
            "color".into(),
            "label".into(),
            "label_drawn".into(),
            "clipped".into(),
        ],
        track_marks
            .iter()
            .map(|mark| {
                vec![
                    Value::Int(mark.track_index as i64),
                    Value::Int(mark.point_index as i64),
                    Value::Int(mark.source_row as i64),
                    Value::Int(mark.chromosome_index as i64),
                    Value::Str(mark.chromosome.clone().into()),
                    Value::Float(mark.start),
                    Value::Float(mark.end),
                    Value::Float(mark.value),
                    Value::Float(mark.angle_start),
                    Value::Float(mark.angle_end),
                    Value::Float(mark.radial_inner),
                    Value::Float(mark.radial_outer),
                    Value::Str(mark.color.clone().into()),
                    mark.label
                        .as_ref()
                        .map(|label| Value::Str(label.clone().into()))
                        .unwrap_or(Value::Nil),
                    Value::Bool(mark.label_drawn),
                    Value::Bool(mark.clipped),
                ]
            })
            .collect(),
    ));
    let link_table = Value::Table(Table::new(
        vec![
            "link_index".into(),
            "source_row".into(),
            "source_chromosome_index".into(),
            "source_chromosome".into(),
            "source_start".into(),
            "source_end".into(),
            "target_chromosome_index".into(),
            "target_chromosome".into(),
            "target_start".into(),
            "target_end".into(),
            "source_angle_start".into(),
            "source_angle_end".into(),
            "target_angle_start".into(),
            "target_angle_end".into(),
            "weight".into(),
            "stroke_width".into(),
            "color".into(),
            "label".into(),
            "label_drawn".into(),
        ],
        links
            .iter()
            .map(|link| {
                vec![
                    Value::Int(link.link_index as i64),
                    Value::Int(link.source_row as i64),
                    Value::Int(link.source_chromosome_index as i64),
                    Value::Str(link.source_chromosome.clone().into()),
                    Value::Float(link.source_start),
                    Value::Float(link.source_end),
                    Value::Int(link.target_chromosome_index as i64),
                    Value::Str(link.target_chromosome.clone().into()),
                    Value::Float(link.target_start),
                    Value::Float(link.target_end),
                    Value::Float(link.source_angle_start),
                    Value::Float(link.source_angle_end),
                    Value::Float(link.target_angle_start),
                    Value::Float(link.target_angle_end),
                    Value::Float(link.weight),
                    Value::Float(link.stroke_width),
                    Value::Str(link.color.clone().into()),
                    link.label
                        .as_ref()
                        .map(|label| Value::Str(label.clone().into()))
                        .unwrap_or(Value::Nil),
                    Value::Bool(link.label_drawn),
                ]
            })
            .collect(),
    ));
    let title = get_opt_str(opts, "title", "Circular genome");
    let subtitle = get_opt_str(opts, "subtitle", "");
    let caption = get_opt_str(opts, "caption", "");
    Ok(Value::Record(
        HashMap::from([
            ("schema".into(), Value::Str("biolang.plot.spec/v1".into())),
            ("kind".into(), Value::Str("circular-genome".into())),
            ("plot".into(), Value::Str("circos".into())),
            ("title".into(), Value::Str(title.into())),
            ("subtitle".into(), Value::Str(subtitle.into())),
            ("caption".into(), Value::Str(caption.into())),
            ("segments".into(), segment_table),
            ("track_metadata".into(), track_metadata),
            ("tracks".into(), track_table),
            ("links".into(), link_table),
            (
                "options".into(),
                Value::Record(
                    HashMap::from([
                        ("width".into(), Value::Float(width)),
                        ("height".into(), Value::Float(height)),
                        (
                            "theme".into(),
                            Value::Str(get_opt_str(opts, "theme", "biolang").into()),
                        ),
                        ("gap_radians".into(), Value::Float(gap)),
                        (
                            "start_angle".into(),
                            Value::Float(start_degrees.to_radians()),
                        ),
                        ("available_angle".into(), Value::Float(available)),
                        ("outer_radius".into(), Value::Float(1.0)),
                        ("chromosome_inner_radius".into(), Value::Float(0.89)),
                        (
                            "link_radius".into(),
                            Value::Float(
                                track_meta
                                    .last()
                                    .map(|track| track.radial_inner - 0.035)
                                    .unwrap_or(0.82),
                            ),
                        ),
                        ("dense_links".into(), Value::Bool(links.len() > 200)),
                        ("dense_tracks".into(), Value::Bool(track_marks.len() > 500)),
                    ])
                    .into(),
                ),
            ),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        (
                            "chromosome_order".into(),
                            Value::List(
                                chromosomes
                                    .iter()
                                    .map(|chromosome| Value::Str(chromosome.name.clone().into()))
                                    .collect::<Vec<_>>()
                                    .into(),
                            ),
                        ),
                        (
                            "segment_rows".into(),
                            Value::Int(segments.rows.len() as i64),
                        ),
                        ("track_count".into(), Value::Int(track_meta.len() as i64)),
                        ("track_marks".into(), Value::Int(track_marks.len() as i64)),
                        ("link_count".into(), Value::Int(links.len() as i64)),
                        (
                            "geometry".into(),
                            Value::Str("length_weighted_gapped_circle".into()),
                        ),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    ))
}

pub(super) fn polar_point(cx: f64, cy: f64, radius: f64, angle: f64) -> (f64, f64) {
    (cx + radius * angle.cos(), cy + radius * angle.sin())
}

pub(super) fn annular_path(
    cx: f64,
    cy: f64,
    inner: f64,
    outer: f64,
    start: f64,
    end: f64,
) -> String {
    let (x1, y1) = polar_point(cx, cy, outer, start);
    let (x2, y2) = polar_point(cx, cy, outer, end);
    let (x3, y3) = polar_point(cx, cy, inner, end);
    let (x4, y4) = polar_point(cx, cy, inner, start);
    let large = usize::from((end - start).abs() > std::f64::consts::PI);
    format!(
        "M{x1:.2},{y1:.2}A{outer:.2},{outer:.2} 0 {large} 1 {x2:.2},{y2:.2}L{x3:.2},{y3:.2}A{inner:.2},{inner:.2} 0 {large} 0 {x4:.2},{y4:.2}Z"
    )
}

pub(super) fn render_circos_svg(
    specification: &HashMap<String, Value>,
    opts: &HashMap<String, Value>,
) -> Result<String> {
    let segments = match specification.get("segments") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let track_metadata = match specification.get("track_metadata") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let tracks = match specification.get("tracks") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let links = match specification.get("links") {
        Some(Value::Table(table)) => table,
        _ => unreachable!("validated before render"),
    };
    let options = frozen_spec_options(specification, opts, "circos")?;
    let width = get_opt_f64(&options, "width", 700.0);
    let height = get_opt_f64(&options, "height", 700.0);
    let title = specification
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Circular genome");
    let subtitle = specification
        .get("subtitle")
        .and_then(Value::as_str)
        .unwrap_or("");
    let caption = specification
        .get("caption")
        .and_then(Value::as_str)
        .unwrap_or("");
    let theme = plot_theme(&options);
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    canvas.margin.top = if title.is_empty() {
        20.0
    } else if subtitle.is_empty() {
        48.0
    } else {
        66.0
    };
    canvas.margin.bottom = if caption.is_empty() { 18.0 } else { 34.0 };
    canvas.margin.left = 20.0;
    canvas.margin.right = if track_metadata.rows.is_empty() {
        20.0
    } else {
        118.0_f64.min(width * 0.24)
    };
    let available_width = width - canvas.margin.left - canvas.margin.right;
    let available_height = height - canvas.margin.top - canvas.margin.bottom;
    let base_radius = available_width.min(available_height) * 0.46;
    let cx = canvas.margin.left + available_width / 2.0;
    let cy = canvas.margin.top + available_height / 2.0;
    let segment_columns = |name: &str| segments.col_index(name).unwrap();
    for row in &segments.rows {
        let start = row[segment_columns("angle_start")].as_float().unwrap();
        let end = row[segment_columns("angle_end")].as_float().unwrap();
        let color = row[segment_columns("color")].as_str().unwrap();
        let path = annular_path(cx, cy, base_radius * 0.89, base_radius, start, end);
        canvas.elements.push(format!(
            r##"<path d="{path}" fill="{color}" stroke="#ffffff" stroke-width="0.8" data-circos-layer="ideogram" />"##
        ));
        if row[segment_columns("label_drawn")].is_truthy() {
            let middle = (start + end) / 2.0;
            let (x, y) = polar_point(cx, cy, base_radius + 14.0, middle);
            canvas.add_text(
                x,
                y + 3.0,
                row[segment_columns("chromosome")].as_str().unwrap(),
                "middle",
                theme.legend_size,
            );
        }
    }
    let meta_type = track_metadata.col_index("type").unwrap();
    let meta_name = track_metadata.col_index("name").unwrap();
    let track_index_column = tracks.col_index("track_index").unwrap();
    let track_angle_start = tracks.col_index("angle_start").unwrap();
    let track_angle_end = tracks.col_index("angle_end").unwrap();
    let track_radial_inner = tracks.col_index("radial_inner").unwrap();
    let track_radial_outer = tracks.col_index("radial_outer").unwrap();
    let track_color = tracks.col_index("color").unwrap();
    let track_label = tracks.col_index("label").unwrap();
    let track_label_drawn = tracks.col_index("label_drawn").unwrap();
    let dense_tracks = get_opt_f64(&options, "dense_tracks", 0.0) > 0.0
        || matches!(options.get("dense_tracks"), Some(Value::Bool(true)));
    let mut dense_paths = BTreeMap::<(usize, String), String>::new();
    let mut line_paths = BTreeMap::<(usize, usize, String), String>::new();
    for row in &tracks.rows {
        let index = row[track_index_column].as_float().unwrap() as usize;
        let kind = track_metadata.rows[index][meta_type].as_str().unwrap();
        let start = row[track_angle_start].as_float().unwrap();
        let end = row[track_angle_end].as_float().unwrap();
        let inner = row[track_radial_inner].as_float().unwrap() * base_radius;
        let outer = row[track_radial_outer].as_float().unwrap() * base_radius;
        let color = row[track_color].as_str().unwrap();
        match kind {
            "point" => {
                let (x, y) = polar_point(cx, cy, outer, start);
                if dense_tracks {
                    dense_paths
                        .entry((index, color.to_string()))
                        .or_default()
                        .push_str(&format!("M{:.2},{:.2}h0.01", x, y));
                } else {
                    canvas.add_circle(x, y, 2.4, color);
                }
            }
            "line" => {
                let middle = (start + end) / 2.0;
                let (x, y) = polar_point(cx, cy, outer, middle);
                if !dense_tracks {
                    canvas.add_circle(x, y, 1.8, color);
                }
                let chromosome = row[tracks.col_index("chromosome_index").unwrap()]
                    .as_float()
                    .unwrap() as usize;
                let path = line_paths
                    .entry((index, chromosome, color.to_string()))
                    .or_default();
                path.push_str(if path.is_empty() { "M" } else { "L" });
                path.push_str(&format!("{x:.2},{y:.2}"));
            }
            _ => {
                let visible_end = if (end - start).abs() < 1e-8 {
                    start + 0.002
                } else {
                    end
                };
                let path = annular_path(cx, cy, inner, outer.max(inner + 1.0), start, visible_end);
                if dense_tracks {
                    dense_paths
                        .entry((index, color.to_string()))
                        .or_default()
                        .push_str(&path);
                } else {
                    canvas.elements.push(format!(
                        r#"<path d="{path}" fill="{color}" opacity="0.88" data-circos-layer="track" />"#
                    ));
                }
            }
        }
        if row[track_label_drawn].is_truthy() {
            if let Some(label) = row[track_label].as_str() {
                let (x, y) = polar_point(cx, cy, outer + 4.0, (start + end) / 2.0);
                canvas.add_text(x, y, label, "middle", 7.5);
            }
        }
    }
    for ((_, color), path) in dense_paths {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="{color}" stroke="{color}" stroke-width="3" opacity="0.82" data-circos-layer="dense-track" />"#
        ));
    }
    for ((_, _, color), path) in line_paths {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="1.5" opacity="0.9" data-circos-layer="line-track" />"#
        ));
    }
    let link_radius = get_opt_f64(&options, "link_radius", 0.82) * base_radius;
    let link_columns = |name: &str| links.col_index(name).unwrap();
    let dense_links = matches!(options.get("dense_links"), Some(Value::Bool(true)));
    let mut link_paths = BTreeMap::<(String, i64), String>::new();
    for row in &links.rows {
        let source_start = row[link_columns("source_angle_start")].as_float().unwrap();
        let source_end = row[link_columns("source_angle_end")].as_float().unwrap();
        let target_start = row[link_columns("target_angle_start")].as_float().unwrap();
        let target_end = row[link_columns("target_angle_end")].as_float().unwrap();
        let source = (source_start + source_end) / 2.0;
        let target = (target_start + target_end) / 2.0;
        let (sx, sy) = polar_point(cx, cy, link_radius, source);
        let (tx, ty) = polar_point(cx, cy, link_radius, target);
        let color = row[link_columns("color")].as_str().unwrap();
        let stroke = row[link_columns("stroke_width")].as_float().unwrap();
        let path = format!("M{sx:.2},{sy:.2}Q{cx:.2},{cy:.2} {tx:.2},{ty:.2}");
        if dense_links {
            link_paths
                .entry((color.to_string(), (stroke * 4.0).round() as i64))
                .or_default()
                .push_str(&path);
        } else if (source_end - source_start).abs() > 1e-8
            || (target_end - target_start).abs() > 1e-8
        {
            let (s2x, s2y) = polar_point(cx, cy, link_radius, source_end);
            let (t2x, t2y) = polar_point(cx, cy, link_radius, target_end);
            let ribbon = format!(
                "M{sx:.2},{sy:.2}Q{cx:.2},{cy:.2} {tx:.2},{ty:.2}L{t2x:.2},{t2y:.2}Q{cx:.2},{cy:.2} {s2x:.2},{s2y:.2}Z"
            );
            canvas.elements.push(format!(
                r#"<path d="{ribbon}" fill="{color}" opacity="0.25" stroke="{color}" stroke-width="0.5" data-circos-layer="ribbon" />"#
            ));
        } else {
            canvas.elements.push(format!(
                r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{stroke:.2}" opacity="0.38" data-circos-layer="link" />"#
            ));
        }
    }
    for ((color, bucket), path) in link_paths {
        canvas.elements.push(format!(
            r#"<path d="{path}" fill="none" stroke="{color}" stroke-width="{:.2}" opacity="0.25" data-circos-layer="dense-links" />"#,
            bucket as f64 / 4.0
        ));
    }
    if !track_metadata.rows.is_empty() {
        let legend_x = width - canvas.margin.right + 10.0;
        let mut legend_y = canvas.margin.top + 6.0;
        for (index, row) in track_metadata.rows.iter().enumerate() {
            let color = PALETTE[index % PALETTE.len()];
            canvas.add_rect(legend_x, legend_y - 8.0, 9.0, 9.0, color);
            canvas.add_text(
                legend_x + 14.0,
                legend_y,
                row[meta_name].as_str().unwrap(),
                "start",
                theme.legend_size,
            );
            legend_y += 16.0;
        }
    }
    canvas.draw_title(title);
    canvas.draw_subtitle(subtitle);
    canvas.draw_caption(caption);
    canvas.set_accessible_description(format!(
        "Circular genome plot with {} chromosomes, {} tracks, {} track marks and {} genomic links.",
        segments.rows.len(),
        track_metadata.rows.len(),
        tracks.rows.len(),
        links.rows.len()
    ));
    Ok(canvas.render())
}

pub(crate) fn is_circos_plot_spec(value: &Value) -> bool {
    matches!(value, Value::Record(map)
        if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == "biolang.plot.spec/v1")
            && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "circos"))
}

pub(crate) fn render_circos_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let Value::Record(map) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a circos PlotSpec",
            None,
        ));
    };
    if !is_circos_plot_spec(value) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() requires a biolang.plot.spec/v1 circos Record",
            None,
        ));
    }
    let table = |name: &str| -> Result<&Table> {
        match map.get(name) {
            Some(Value::Table(table)) => Ok(table),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() circos field '{name}' must be a Table"),
                None,
            )),
        }
    };
    let segments = table("segments")?;
    let track_metadata = table("track_metadata")?;
    let tracks = table("tracks")?;
    let links = table("links")?;
    let require_columns = |table: &Table, family: &str, columns: &[&str]| -> Result<()> {
        for column in columns {
            if table.col_index(column).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() circos {family} is missing '{column}'"),
                    None,
                ));
            }
        }
        Ok(())
    };
    require_columns(
        segments,
        "segments",
        &[
            "chromosome_index",
            "source_row",
            "chromosome",
            "start",
            "end",
            "size",
            "angle_start",
            "angle_end",
            "color",
            "label_drawn",
        ],
    )?;
    require_columns(
        track_metadata,
        "track metadata",
        &[
            "track_index",
            "name",
            "type",
            "radial_inner",
            "radial_outer",
            "value_min",
            "value_max",
        ],
    )?;
    require_columns(
        tracks,
        "tracks",
        &[
            "track_index",
            "point_index",
            "source_row",
            "chromosome_index",
            "chromosome",
            "start",
            "end",
            "value",
            "angle_start",
            "angle_end",
            "radial_inner",
            "radial_outer",
            "color",
            "label",
            "label_drawn",
            "clipped",
        ],
    )?;
    require_columns(
        links,
        "links",
        &[
            "link_index",
            "source_row",
            "source_chromosome_index",
            "source_chromosome",
            "source_start",
            "source_end",
            "target_chromosome_index",
            "target_chromosome",
            "target_start",
            "target_end",
            "source_angle_start",
            "source_angle_end",
            "target_angle_start",
            "target_angle_end",
            "weight",
            "stroke_width",
            "color",
            "label",
            "label_drawn",
        ],
    )?;
    let options = match map.get("options") {
        Some(Value::Record(options)) => options,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos options must be a Record",
                None,
            ))
        }
    };
    let gap = options
        .get("gap_radians")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos gap_radians is invalid",
                None,
            )
        })?;
    let start_angle = options
        .get("start_angle")
        .and_then(Value::as_float)
        .filter(|value| value.is_finite())
        .ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos start_angle is invalid",
                None,
            )
        })?;
    let mut chromosome_geometry = Vec::<CircosChromosome>::new();
    let mut expected_angle = start_angle;
    let mut total_size = 0.0;
    for row in &segments.rows {
        let size = row[segments.col_index("size").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        if !size.is_finite() || size <= 0.0 {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos chromosome sizes are invalid",
                None,
            ));
        }
        total_size += size;
    }
    let available = std::f64::consts::TAU - gap * segments.rows.len() as f64;
    for (index, row) in segments.rows.iter().enumerate() {
        let frozen_index = frozen_nonnegative_integer(
            &row[segments.col_index("chromosome_index").unwrap()],
            "circos",
            "chromosome_index",
        )?;
        let source_row = frozen_nonnegative_integer(
            &row[segments.col_index("source_row").unwrap()],
            "circos",
            "source_row",
        )?;
        let name = row[segments.col_index("chromosome").unwrap()]
            .as_str()
            .unwrap_or("");
        let start = row[segments.col_index("start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let end = row[segments.col_index("end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let size = row[segments.col_index("size").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_start = row[segments.col_index("angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_end = row[segments.col_index("angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let expected_end = expected_angle + size / total_size * available;
        let color = row[segments.col_index("color").unwrap()]
            .as_str()
            .unwrap_or("");
        if frozen_index != index
            || name.is_empty()
            || !start.is_finite()
            || !end.is_finite()
            || start < 0.0
            || (end - start - size).abs() > 1e-8
            || (angle_start - expected_angle).abs() > 1e-10
            || (angle_end - expected_end).abs() > 1e-10
            || !valid_bio_hex_color(color)
            || !matches!(
                row[segments.col_index("label_drawn").unwrap()],
                Value::Bool(_)
            )
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos chromosome geometry is inconsistent",
                None,
            ));
        }
        chromosome_geometry.push(CircosChromosome {
            source_row,
            name: name.to_string(),
            start,
            end,
            angle_start,
            angle_end,
            color: color.to_string(),
            label_drawn: row[segments.col_index("label_drawn").unwrap()].is_truthy(),
        });
        expected_angle = expected_end + gap;
    }
    let mut frozen_track_metadata = Vec::<(String, f64, f64, f64, f64)>::new();
    for (index, row) in track_metadata.rows.iter().enumerate() {
        let frozen_index = frozen_nonnegative_integer(
            &row[track_metadata.col_index("track_index").unwrap()],
            "circos",
            "track_index",
        )?;
        let kind = row[track_metadata.col_index("type").unwrap()]
            .as_str()
            .unwrap_or("");
        let inner = row[track_metadata.col_index("radial_inner").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let outer = row[track_metadata.col_index("radial_outer").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let value_min = row[track_metadata.col_index("value_min").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let value_max = row[track_metadata.col_index("value_max").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        if frozen_index != index
            || !matches!(normalized_track_kind(kind), Ok(ref normalized) if normalized == kind)
            || !inner.is_finite()
            || !outer.is_finite()
            || !value_min.is_finite()
            || !value_max.is_finite()
            || inner < 0.0
            || outer <= inner
            || outer > 0.89
            || value_max <= value_min
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos track metadata is inconsistent",
                None,
            ));
        }
        frozen_track_metadata.push((kind.to_string(), inner, outer, value_min, value_max));
    }
    let mut point_counts = vec![0usize; track_metadata.rows.len()];
    for row in &tracks.rows {
        let track_index = frozen_nonnegative_integer(
            &row[tracks.col_index("track_index").unwrap()],
            "circos",
            "track_index",
        )?;
        let point_index = frozen_nonnegative_integer(
            &row[tracks.col_index("point_index").unwrap()],
            "circos",
            "point_index",
        )?;
        let chromosome_index = frozen_nonnegative_integer(
            &row[tracks.col_index("chromosome_index").unwrap()],
            "circos",
            "chromosome_index",
        )?;
        if track_index >= track_metadata.rows.len()
            || chromosome_index >= chromosome_geometry.len()
            || point_index != point_counts[track_index]
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos track indexes are inconsistent",
                None,
            ));
        }
        point_counts[track_index] += 1;
        let chromosome = &chromosome_geometry[chromosome_index];
        let start = row[tracks.col_index("start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let end = row[tracks.col_index("end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_start = row[tracks.col_index("angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let angle_end = row[tracks.col_index("angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let value = row[tracks.col_index("value").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let radial_inner = row[tracks.col_index("radial_inner").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let radial_outer = row[tracks.col_index("radial_outer").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let (kind, metadata_inner, metadata_outer, value_min, value_max) =
            &frozen_track_metadata[track_index];
        let normalized = ((value - value_min) / (value_max - value_min)).clamp(0.0, 1.0);
        let expected_outer = if matches!(kind.as_str(), "bar" | "line" | "point") {
            metadata_inner + normalized * (metadata_outer - metadata_inner)
        } else {
            *metadata_outer
        };
        let color = row[tracks.col_index("color").unwrap()]
            .as_str()
            .unwrap_or("");
        if row[tracks.col_index("chromosome").unwrap()].as_str() != Some(chromosome.name.as_str())
            || start < chromosome.start
            || end > chromosome.end
            || end < start
            || !value.is_finite()
            || (angle_start - circos_chromosome_angle(chromosome, start)).abs() > 1e-10
            || (angle_end - circos_chromosome_angle(chromosome, end)).abs() > 1e-10
            || (radial_inner - metadata_inner).abs() > 1e-10
            || (radial_outer - expected_outer).abs() > 1e-10
            || !valid_bio_hex_color(color)
            || !matches!(
                row[tracks.col_index("label_drawn").unwrap()],
                Value::Bool(_)
            )
            || !matches!(row[tracks.col_index("clipped").unwrap()], Value::Bool(_))
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos track geometry is inconsistent",
                None,
            ));
        }
    }
    let max_link_weight = links
        .rows
        .iter()
        .filter_map(|row| row[links.col_index("weight").unwrap()].as_float())
        .fold(0.0f64, f64::max)
        .max(1.0);
    for (index, row) in links.rows.iter().enumerate() {
        let frozen_index = frozen_nonnegative_integer(
            &row[links.col_index("link_index").unwrap()],
            "circos",
            "link_index",
        )?;
        let source_index = frozen_nonnegative_integer(
            &row[links.col_index("source_chromosome_index").unwrap()],
            "circos",
            "source_chromosome_index",
        )?;
        let target_index = frozen_nonnegative_integer(
            &row[links.col_index("target_chromosome_index").unwrap()],
            "circos",
            "target_chromosome_index",
        )?;
        if frozen_index != index
            || source_index >= chromosome_geometry.len()
            || target_index >= chromosome_geometry.len()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos link indexes are inconsistent",
                None,
            ));
        }
        let source = &chromosome_geometry[source_index];
        let target = &chromosome_geometry[target_index];
        let source_start = row[links.col_index("source_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let source_end = row[links.col_index("source_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let target_start = row[links.col_index("target_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let target_end = row[links.col_index("target_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_source_angle = row[links.col_index("source_angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_source_angle_end = row[links.col_index("source_angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_target_angle = row[links.col_index("target_angle_start").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let frozen_target_angle_end = row[links.col_index("target_angle_end").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let weight = row[links.col_index("weight").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let stroke_width = row[links.col_index("stroke_width").unwrap()]
            .as_float()
            .unwrap_or(f64::NAN);
        let expected_stroke_width =
            0.75 + ((weight / max_link_weight).sqrt() * 3.0 * 4.0).round() / 4.0;
        if source_start < source.start
            || source_end > source.end
            || source_end < source_start
            || target_start < target.start
            || target_end > target.end
            || target_end < target_start
            || (frozen_source_angle - circos_chromosome_angle(source, source_start)).abs() > 1e-10
            || (frozen_source_angle_end - circos_chromosome_angle(source, source_end)).abs() > 1e-10
            || (frozen_target_angle - circos_chromosome_angle(target, target_start)).abs() > 1e-10
            || (frozen_target_angle_end - circos_chromosome_angle(target, target_end)).abs() > 1e-10
            || !weight.is_finite()
            || weight < 0.0
            || !stroke_width.is_finite()
            || (stroke_width - expected_stroke_width).abs() > 1e-10
            || !row[links.col_index("color").unwrap()]
                .as_str()
                .is_some_and(valid_bio_hex_color)
            || !matches!(row[links.col_index("label_drawn").unwrap()], Value::Bool(_))
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() circos link geometry is inconsistent",
                None,
            ));
        }
    }
    if matches!(
        get_opt_str(render_options, "format", "svg")
            .to_ascii_lowercase()
            .as_str(),
        "spec" | "data"
    ) {
        return Ok(value.clone());
    }
    let svg = render_circos_svg(map, render_options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Circular genome");
    finish_frozen_bio_plot(value, render_options, title, "circos", svg)
}

pub(super) fn builtin_circos(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    // Keep the historical `circos(segments_table, options?)` form working.
    // The structured Record is the canonical multi-track input, but a bare
    // segment table remains a useful and documented shorthand for the outer
    // chromosome ring.
    let input = match &args[0] {
        Value::Table(_) => {
            Value::Record(HashMap::from([("segments".into(), args[0].clone())]).into())
        }
        value => value.clone(),
    };
    let specification = circos_spec_value(&input, &opts)?;
    render_circos_plot_spec_value(&specification, &opts)
}

#[cfg(any())]
pub(super) fn builtin_circos_legacy(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();

    let (segments, links) = match &args[0] {
        Value::Record(map) => {
            let seg = map.get("segments").cloned().unwrap_or(Value::Nil);
            let lnk = map.get("links").cloned().unwrap_or(Value::Nil);
            (seg, lnk)
        }
        Value::Table(_) => (args[0].clone(), Value::Nil),
        _ => {
            return Err(BioLangError::type_error(
                "circos() requires Record with 'segments' and 'links' Tables",
                None,
            ))
        }
    };

    let seg_table = match &segments {
        Value::Table(t) => t,
        _ => {
            return Err(BioLangError::type_error(
                "circos() 'segments' must be a Table",
                None,
            ))
        }
    };
    let chroms = extract_str_col(seg_table, "chrom")?;
    let ends = extract_table_col(seg_table, "end")?;

    // Compute chrom sizes
    let mut chrom_sizes: Vec<(String, f64)> = Vec::new();
    let mut seen: HashMap<String, f64> = HashMap::new();
    for (i, c) in chroms.iter().enumerate() {
        let e = seen.entry(c.clone()).or_insert(0.0);
        if ends[i] > *e {
            *e = ends[i];
        }
    }
    let mut chrom_order: Vec<String> = Vec::new();
    for c in &chroms {
        if !chrom_order.contains(c) {
            chrom_order.push(c.clone());
        }
    }
    for c in &chrom_order {
        chrom_sizes.push((c.clone(), *seen.get(c).unwrap_or(&1.0)));
    }
    let total_size: f64 = chrom_sizes.iter().map(|(_, s)| s).sum();

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 500.0);
        let h = get_opt_f64(&opts, "height", 500.0);
        let mut c = SvgCanvas::new(w, h);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let r = w.min(h) * 0.38;
        let r_inner = r * 0.85;
        let mut angle = 0.0f64;
        let mut chrom_angles: HashMap<String, (f64, f64)> = HashMap::new();
        for (name, size) in &chrom_sizes {
            let sweep = (*size / total_size) * 2.0 * std::f64::consts::PI;
            let a1 = angle;
            let a2 = angle + sweep;
            chrom_angles.insert(name.clone(), (a1, a2));
            // Draw arc segment
            let ci = chrom_order.iter().position(|c| c == name).unwrap_or(0);
            let color = PALETTE[ci % PALETTE.len()];
            let (x1, y1) = (cx + r * a1.cos(), cy + r * a1.sin());
            let (x2, y2) = (cx + r * a2.cos(), cy + r * a2.sin());
            let (x3, y3) = (cx + r_inner * a2.cos(), cy + r_inner * a2.sin());
            let (x4, y4) = (cx + r_inner * a1.cos(), cy + r_inner * a1.sin());
            let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
            c.elements.push(format!(
                r#"<path d="M {x1:.1},{y1:.1} A {r:.1},{r:.1} 0 {large} 1 {x2:.1},{y2:.1} L {x3:.1},{y3:.1} A {ri:.1},{ri:.1} 0 {large} 0 {x4:.1},{y4:.1} Z" fill="{color}" opacity="0.7" />"#,
                ri = r_inner
            ));
            let mid_a = (a1 + a2) / 2.0;
            let lx = cx + (r + 15.0) * mid_a.cos();
            let ly = cy + (r + 15.0) * mid_a.sin();
            c.add_text(lx, ly, name, "middle", 9.0);
            angle = a2 + 0.02; // small gap
        }
        // Draw links as bezier curves
        if let Value::Table(link_table) = &links {
            if let (Ok(c1), Ok(s1), Ok(c2), Ok(s2)) = (
                extract_str_col(link_table, "chrom1"),
                extract_table_col(link_table, "pos1"),
                extract_str_col(link_table, "chrom2"),
                extract_table_col(link_table, "pos2"),
            ) {
                for i in 0..c1.len() {
                    if let (Some(&(a1s, a1e)), Some(&(a2s, a2e))) =
                        (chrom_angles.get(&c1[i]), chrom_angles.get(&c2[i]))
                    {
                        let sz1 = seen.get(&c1[i]).copied().unwrap_or(1.0);
                        let sz2 = seen.get(&c2[i]).copied().unwrap_or(1.0);
                        let ang1 = a1s + (s1[i] / sz1) * (a1e - a1s);
                        let ang2 = a2s + (s2[i] / sz2) * (a2e - a2s);
                        let (px1, py1) = (cx + r_inner * ang1.cos(), cy + r_inner * ang1.sin());
                        let (px2, py2) = (cx + r_inner * ang2.cos(), cy + r_inner * ang2.sin());
                        c.elements.push(format!(
                            r#"<path d="M {px1:.1},{py1:.1} Q {cx:.1},{cy:.1} {px2:.1},{py2:.1}" fill="none" stroke="{}" opacity="0.4" stroke-width="1.5" />"#,
                            PALETTE[i % PALETTE.len()]
                        ));
                    }
                }
            }
        }
        c.draw_title("Circos");
        return Ok(Value::Str(c.render()));
    }

    // ASCII: linear summary
    let bar_w = get_opt_usize(&opts, "width", 50);
    let max_name = chrom_sizes.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let mut out = String::from("  Circos (linear view)\n");
    for (name, size) in &chrom_sizes {
        let len = (size / total_size * bar_w as f64).round() as usize;
        let bar: String = "█".repeat(len.max(1));
        out.push_str(&format!("  {:>w$}  {bar}\n", name, w = max_name));
    }
    if let Value::Table(link_table) = &links {
        if let (Ok(c1), Ok(s1), Ok(c2), Ok(s2)) = (
            extract_str_col(link_table, "chrom1"),
            extract_table_col(link_table, "pos1"),
            extract_str_col(link_table, "chrom2"),
            extract_table_col(link_table, "pos2"),
        ) {
            out.push_str(&format!("  Links ({}):\n", c1.len()));
            for i in 0..c1.len().min(10) {
                out.push_str(&format!(
                    "    {}:{:.0} → {}:{:.0}\n",
                    c1[i], s1[i], c2[i], s2[i]
                ));
            }
        }
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── 20. hic_map ─────────────────────────────────────────────────

pub(super) fn builtin_circos_plot(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let title = get_opt_str(&opts, "title", "Circos Plot").to_string();
    let track_type = get_opt_str(&opts, "track", "bar").to_string();

    // Default human chromosome sizes (in bp)
    let default_chroms: Vec<(&str, f64)> = vec![
        ("chr1", 248956422.0),
        ("chr2", 242193529.0),
        ("chr3", 198295559.0),
        ("chr4", 190214555.0),
        ("chr5", 181538259.0),
        ("chr6", 170805979.0),
        ("chr7", 159345973.0),
        ("chr8", 145138636.0),
        ("chr9", 138394717.0),
        ("chr10", 133797422.0),
        ("chr11", 135086622.0),
        ("chr12", 133275309.0),
        ("chr13", 114364328.0),
        ("chr14", 107043718.0),
        ("chr15", 101991189.0),
        ("chr16", 90338345.0),
        ("chr17", 83257441.0),
        ("chr18", 80373285.0),
        ("chr19", 58617616.0),
        ("chr20", 64444167.0),
        ("chr21", 46709983.0),
        ("chr22", 50818468.0),
        ("chrX", 156040895.0),
        ("chrY", 57227415.0),
    ];

    // Extract data points: List of Records with {chrom, start, end, value}
    let data_points: Vec<(String, f64, f64, f64)> = match &args[0] {
        Value::List(items) => items
            .iter()
            .filter_map(|item| {
                if let Value::Record(map) = item {
                    let chrom = map
                        .get("chrom")
                        .or(map.get("chr"))
                        .map(|v| format!("{v}"))?;
                    let start = map
                        .get("start")
                        .or(map.get("pos"))
                        .and_then(|v| v.as_float())
                        .unwrap_or(0.0);
                    let end = map
                        .get("end")
                        .and_then(|v| v.as_float())
                        .unwrap_or(start + 1.0);
                    let value = map
                        .get("value")
                        .or(map.get("score"))
                        .and_then(|v| v.as_float())
                        .unwrap_or(1.0);
                    Some((chrom, start, end, value))
                } else {
                    None
                }
            })
            .collect(),
        Value::Table(table) => {
            let chroms = extract_str_col(
                table,
                if table.col_index("chrom").is_some() {
                    "chrom"
                } else {
                    "chr"
                },
            )?;
            let starts = extract_table_col(
                table,
                if table.col_index("start").is_some() {
                    "start"
                } else {
                    "pos"
                },
            )?;
            let ends = extract_table_col(table, "end")
                .unwrap_or_else(|_| starts.iter().map(|&s| s + 1.0).collect());
            let values = extract_table_col(table, "value")
                .or_else(|_| extract_table_col(table, "score"))
                .unwrap_or_else(|_| vec![1.0; chroms.len()]);
            chroms
                .into_iter()
                .zip(starts.into_iter().zip(ends.into_iter().zip(values)))
                .map(|(c, (s, (e, v)))| (c, s, e, v))
                .collect()
        }
        _ => {
            return Err(BioLangError::type_error(
                "circos_plot() requires List of Records or Table",
                None,
            ))
        }
    };

    // Determine chromosome set
    let mut chrom_order: Vec<String> = Vec::new();
    for (c, _, _, _) in &data_points {
        if !chrom_order.contains(c) {
            chrom_order.push(c.clone());
        }
    }
    let default_order: Vec<&str> = default_chroms.iter().map(|(n, _)| *n).collect();
    chrom_order.sort_by_key(|c| default_order.iter().position(|&d| d == c).unwrap_or(999));

    let chrom_sizes: Vec<(String, f64)> = chrom_order
        .iter()
        .map(|c| {
            let default_size = default_chroms
                .iter()
                .find(|(n, _)| *n == c.as_str())
                .map(|(_, s)| *s);
            let data_max = data_points
                .iter()
                .filter(|(dc, _, _, _)| dc == c)
                .map(|(_, _, e, _)| *e)
                .fold(0.0f64, f64::max);
            (c.clone(), default_size.unwrap_or(data_max * 1.1))
        })
        .collect();
    let total_size: f64 = chrom_sizes.iter().map(|(_, s)| s).sum();
    if total_size <= 0.0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "circos_plot() no data to plot",
            None,
        ));
    }

    let val_range = {
        let vals: Vec<f64> = data_points.iter().map(|(_, _, _, v)| *v).collect();
        if vals.is_empty() {
            (0.0, 1.0)
        } else {
            col_range(&vals)
        }
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 500.0);
        let h = w;
        let mut c = SvgCanvas::new(w, h);
        let (cx, cy) = (w / 2.0, h / 2.0);
        let r_outer = w.min(h) * 0.40;
        let r_inner = r_outer * 0.82;
        let r_track_outer = r_inner * 0.95;
        let r_track_inner = r_inner * 0.65;
        let gap = 0.01;
        let total_gap = gap * chrom_order.len() as f64;
        let usable_angle = 2.0 * std::f64::consts::PI - total_gap;

        let mut chrom_angles: HashMap<String, (f64, f64)> = HashMap::new();
        let mut angle = 0.0f64;
        for (ci, (name, size)) in chrom_sizes.iter().enumerate() {
            let sweep = (*size / total_size) * usable_angle;
            let a1 = angle;
            let a2 = angle + sweep;
            chrom_angles.insert(name.clone(), (a1, a2));

            let color = PALETTE[ci % PALETTE.len()];
            let (x1, y1) = (cx + r_outer * a1.cos(), cy + r_outer * a1.sin());
            let (x2, y2) = (cx + r_outer * a2.cos(), cy + r_outer * a2.sin());
            let (x3, y3) = (cx + r_inner * a2.cos(), cy + r_inner * a2.sin());
            let (x4, y4) = (cx + r_inner * a1.cos(), cy + r_inner * a1.sin());
            let large = if sweep > std::f64::consts::PI { 1 } else { 0 };
            c.elements.push(format!(
                r#"<path d="M {x1:.1},{y1:.1} A {ro:.1},{ro:.1} 0 {large} 1 {x2:.1},{y2:.1} L {x3:.1},{y3:.1} A {ri:.1},{ri:.1} 0 {large} 0 {x4:.1},{y4:.1} Z" fill="{color}" opacity="0.5" stroke="white" stroke-width="0.5" />"#,
                ro = r_outer, ri = r_inner
            ));

            let mid_a = (a1 + a2) / 2.0;
            let lx = cx + (r_outer + 14.0) * mid_a.cos();
            let ly = cy + (r_outer + 14.0) * mid_a.sin();
            let label = name.strip_prefix("chr").unwrap_or(name);
            c.add_text(lx, ly + 4.0, label, "middle", 8.0);

            angle = a2 + gap;
        }

        // Draw data track
        for (chrom, start, end, value) in &data_points {
            if let Some(&(a1, a2)) = chrom_angles.get(chrom) {
                let chrom_size = chrom_sizes
                    .iter()
                    .find(|(n, _)| n == chrom)
                    .map(|(_, s)| *s)
                    .unwrap_or(1.0);
                let ang_start = a1 + (*start / chrom_size) * (a2 - a1);
                let ang_end = a1 + (*end / chrom_size) * (a2 - a1);
                let t = if (val_range.1 - val_range.0).abs() < f64::EPSILON {
                    0.5
                } else {
                    (value - val_range.0) / (val_range.1 - val_range.0)
                };
                let t = t.clamp(0.0, 1.0);

                match track_type.as_str() {
                    "scatter" => {
                        let r_pt = r_track_inner + t * (r_track_outer - r_track_inner);
                        let mid_ang = (ang_start + ang_end) / 2.0;
                        let px = cx + r_pt * mid_ang.cos();
                        let py = cy + r_pt * mid_ang.sin();
                        c.add_circle(px, py, 2.0, "#e15759");
                    }
                    "line" => {
                        let r_pt = r_track_inner + t * (r_track_outer - r_track_inner);
                        let mid_ang = (ang_start + ang_end) / 2.0;
                        let px = cx + r_pt * mid_ang.cos();
                        let py = cy + r_pt * mid_ang.sin();
                        let base_x = cx + r_track_inner * mid_ang.cos();
                        let base_y = cy + r_track_inner * mid_ang.sin();
                        c.add_line(base_x, base_y, px, py, "#4e79a7", 1.0);
                    }
                    _ => {
                        let r_bar = r_track_inner + t * (r_track_outer - r_track_inner);
                        let (bx1, by1) = (
                            cx + r_track_inner * ang_start.cos(),
                            cy + r_track_inner * ang_start.sin(),
                        );
                        let (bx2, by2) =
                            (cx + r_bar * ang_start.cos(), cy + r_bar * ang_start.sin());
                        let (bx3, by3) = (cx + r_bar * ang_end.cos(), cy + r_bar * ang_end.sin());
                        let (bx4, by4) = (
                            cx + r_track_inner * ang_end.cos(),
                            cy + r_track_inner * ang_end.sin(),
                        );
                        let large_bar = if (ang_end - ang_start) > std::f64::consts::PI {
                            1
                        } else {
                            0
                        };
                        let color = sequential_color(t);
                        c.elements.push(format!(
                            r#"<path d="M {bx1:.1},{by1:.1} L {bx2:.1},{by2:.1} A {r_bar:.1},{r_bar:.1} 0 {large_bar} 1 {bx3:.1},{by3:.1} L {bx4:.1},{by4:.1} A {ri:.1},{ri:.1} 0 {large_bar} 0 {bx1:.1},{by1:.1} Z" fill="{color}" opacity="0.8" />"#,
                            ri = r_track_inner
                        ));
                    }
                }
            }
        }

        c.elements.push(format!(
            r##"<circle cx="{cx:.1}" cy="{cy:.1}" r="{:.1}" fill="none" stroke="#ddd" stroke-width="0.5" />"##,
            r_track_inner
        ));

        c.draw_title(&title);
        return Ok(Value::Str(c.render()));
    }

    // ASCII fallback
    let bar_w = get_opt_usize(&opts, "width", 50);
    let max_name = chrom_sizes.iter().map(|(n, _)| n.len()).max().unwrap_or(4);
    let mut out = format!("  {title} (linear view)\n");
    for (name, size) in &chrom_sizes {
        let len = (size / total_size * bar_w as f64).round() as usize;
        let n_pts = data_points.iter().filter(|(c, _, _, _)| c == name).count();
        let bar: String = "█".repeat(len.max(1));
        out.push_str(&format!(
            "  {:>w$}  {bar}  ({n_pts} points)\n",
            name,
            w = max_name
        ));
    }
    write_output(&out);
    Ok(Value::Nil)
}

// ── umap_plot ────────────────────────────────────────────────────
