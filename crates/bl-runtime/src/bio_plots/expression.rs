//! Expression for BioLang plots.
//!
//! Split out of `plot/mod.rs` without changing behaviour: every figure
//! renders byte for byte as it did before.

use super::*;

#[derive(Clone, Copy, Debug)]
pub(super) enum HeatmapLinkage {
    Complete,
    Average,
    Single,
    WardD2,
}

impl HeatmapLinkage {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "complete" => Ok(Self::Complete),
            "average" | "upgma" => Ok(Self::Average),
            "single" => Ok(Self::Single),
            "ward" | "ward.d2" | "ward_d2" => Ok(Self::WardD2),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "clustered_heatmap() linkage must be complete, average, single, or ward.D2; got '{value}'"
                ),
                None,
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Average => "average",
            Self::Single => "single",
            Self::WardD2 => "ward.D2",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) enum HeatmapDistance {
    Euclidean,
    Manhattan,
}

impl HeatmapDistance {
    fn parse(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "euclidean" => Ok(Self::Euclidean),
            "manhattan" => Ok(Self::Manhattan),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "clustered_heatmap() distance must be euclidean or manhattan; got '{value}'"
                ),
                None,
            )),
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Euclidean => "euclidean",
            Self::Manhattan => "manhattan",
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct HeatmapMerge {
    left: usize,
    right: usize,
    height: f64,
}

#[derive(Clone, Debug)]
pub(super) struct HeatmapTree {
    merges: Vec<HeatmapMerge>,
    order: Vec<usize>,
}

pub(super) fn heatmap_observation_distance(
    left: &[f64],
    right: &[f64],
    method: HeatmapDistance,
) -> Option<f64> {
    let dimensions = left.len().min(right.len());
    let mut compared = 0usize;
    let mut total = 0.0;
    for (&x, &y) in left.iter().zip(right.iter()) {
        if x.is_finite() && y.is_finite() {
            let delta = (x - y).abs();
            total += match method {
                HeatmapDistance::Euclidean => delta * delta,
                HeatmapDistance::Manhattan => delta,
            };
            compared += 1;
        }
    }
    if compared == 0 {
        return None;
    }
    // Match base R dist(): scale pairwise-complete distances when values are missing.
    let scaled = total * dimensions as f64 / compared as f64;
    Some(match method {
        HeatmapDistance::Euclidean => scaled.sqrt(),
        HeatmapDistance::Manhattan => scaled,
    })
}

pub(super) fn hierarchical_heatmap_tree(
    data: &[Vec<f64>],
    distance_method: HeatmapDistance,
    linkage: HeatmapLinkage,
) -> Result<HeatmapTree> {
    let n = data.len();
    if n <= 1 {
        return Ok(HeatmapTree {
            merges: Vec::new(),
            order: (0..n).collect(),
        });
    }
    if matches!(linkage, HeatmapLinkage::WardD2)
        && !matches!(distance_method, HeatmapDistance::Euclidean)
    {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "clustered_heatmap() ward.D2 linkage requires euclidean distance",
            None,
        ));
    }

    let capacity = 2 * n - 1;
    // Reuse one of the n original slots for each merged cluster. A 2n-by-2n
    // grid stores the same active distances but costs about four times more.
    let mut distances = vec![vec![f64::NAN; n]; n];
    for i in 0..n {
        for j in (i + 1)..n {
            let d = heatmap_observation_distance(&data[i], &data[j], distance_method)
                .ok_or_else(|| {
                    BioLangError::runtime(
                        ErrorKind::TypeError,
                        format!(
                            "clustered_heatmap() cannot compute a distance between observations {i} and {j}: no finite values overlap"
                        ),
                        None,
                    )
                })?;
            distances[i][j] = d;
            distances[j][i] = d;
        }
    }

    let mut active_slots: Vec<usize> = (0..n).collect();
    let mut node_for_slot: Vec<usize> = (0..n).collect();
    let mut sizes = vec![1usize; capacity];
    let mut last_height = vec![f64::NEG_INFINITY; capacity];
    let mut min_leaf: Vec<usize> = (0..capacity).collect();
    let mut merges = Vec::with_capacity(n - 1);

    for step in 0..(n - 1) {
        let mut best: Option<(usize, usize, f64, (usize, usize))> = None;
        for ai in 0..active_slots.len() {
            for bi in (ai + 1)..active_slots.len() {
                let a_slot = active_slots[ai];
                let b_slot = active_slots[bi];
                let d = distances[a_slot][b_slot];
                if !d.is_finite() {
                    continue;
                }
                let a_node = node_for_slot[a_slot];
                let b_node = node_for_slot[b_slot];
                let pair = (a_node.min(b_node), a_node.max(b_node));
                let replace = match best {
                    None => true,
                    Some((_, _, old_d, old_pair)) => d < old_d || (d == old_d && pair < old_pair),
                };
                if replace {
                    best = Some((a_slot, b_slot, d, pair));
                }
            }
        }
        let (a_slot, b_slot, height, _) = best.ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "clustered_heatmap() hierarchy contains no finite cluster distance",
                None,
            )
        })?;
        let a = node_for_slot[a_slot];
        let b = node_for_slot[b_slot];
        let merged = n + step;

        // Match hclust's visible branch rotation: tighter subtree left;
        // singleton tightest; input order resolves a tie.
        let (left, right) = if last_height[a] < last_height[b]
            || (last_height[a] == last_height[b] && min_leaf[a] <= min_leaf[b])
        {
            (a, b)
        } else {
            (b, a)
        };
        merges.push(HeatmapMerge {
            left,
            right,
            height,
        });
        sizes[merged] = sizes[a] + sizes[b];
        last_height[merged] = height;
        min_leaf[merged] = min_leaf[a].min(min_leaf[b]);

        for &other_slot in &active_slots {
            if other_slot == a_slot || other_slot == b_slot {
                continue;
            }
            let other = node_for_slot[other_slot];
            let da = distances[a_slot][other_slot];
            let db = distances[b_slot][other_slot];
            let updated = match linkage {
                HeatmapLinkage::Complete => da.max(db),
                HeatmapLinkage::Single => da.min(db),
                HeatmapLinkage::Average => {
                    (sizes[a] as f64 * da + sizes[b] as f64 * db) / (sizes[a] + sizes[b]) as f64
                }
                HeatmapLinkage::WardD2 => {
                    let sa = sizes[a] as f64;
                    let sb = sizes[b] as f64;
                    let so = sizes[other] as f64;
                    (((so + sa) * da * da + (so + sb) * db * db - so * height * height)
                        / (sa + sb + so))
                        .max(0.0)
                        .sqrt()
                }
            };
            distances[a_slot][other_slot] = updated;
            distances[other_slot][a_slot] = updated;
        }
        node_for_slot[a_slot] = merged;
        active_slots.retain(|&slot| slot != b_slot);
    }

    fn append_leaves(
        node: usize,
        leaf_count: usize,
        merges: &[HeatmapMerge],
        out: &mut Vec<usize>,
    ) {
        if node < leaf_count {
            out.push(node);
        } else {
            let merge = &merges[node - leaf_count];
            append_leaves(merge.left, leaf_count, merges, out);
            append_leaves(merge.right, leaf_count, merges, out);
        }
    }
    let mut order = Vec::with_capacity(n);
    append_leaves(node_for_slot[active_slots[0]], n, &merges, &mut order);
    Ok(HeatmapTree { merges, order })
}

pub(super) fn draw_row_dendrogram(
    canvas: &mut SvgCanvas,
    tree: &HeatmapTree,
    heatmap_left: f64,
    heatmap_top: f64,
    cell_height: f64,
    dendrogram_width: f64,
) {
    let n = tree.order.len();
    if n < 2 || dendrogram_width <= 0.0 {
        return;
    }
    let mut x = vec![heatmap_left; 2 * n - 1];
    let mut y = vec![0.0; 2 * n - 1];
    for (position, &leaf) in tree.order.iter().enumerate() {
        y[leaf] = heatmap_top + (position as f64 + 0.5) * cell_height;
    }
    let max_height = tree
        .merges
        .iter()
        .map(|merge| merge.height)
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    for (index, merge) in tree.merges.iter().enumerate() {
        let node = n + index;
        x[node] = heatmap_left - dendrogram_width * merge.height / max_height;
        y[node] = 0.5 * (y[merge.left] + y[merge.right]);
        canvas.add_line(
            x[node],
            y[merge.left],
            x[node],
            y[merge.right],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[node],
            y[merge.left],
            x[merge.left],
            y[merge.left],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[node],
            y[merge.right],
            x[merge.right],
            y[merge.right],
            canvas.theme.axis_colour,
            1.0,
        );
    }
}

pub(super) fn draw_column_dendrogram(
    canvas: &mut SvgCanvas,
    tree: &HeatmapTree,
    heatmap_left: f64,
    heatmap_top: f64,
    cell_width: f64,
    dendrogram_height: f64,
) {
    let n = tree.order.len();
    if n < 2 || dendrogram_height <= 0.0 {
        return;
    }
    let mut x = vec![0.0; 2 * n - 1];
    let mut y = vec![heatmap_top; 2 * n - 1];
    for (position, &leaf) in tree.order.iter().enumerate() {
        x[leaf] = heatmap_left + (position as f64 + 0.5) * cell_width;
    }
    let max_height = tree
        .merges
        .iter()
        .map(|merge| merge.height)
        .fold(0.0, f64::max)
        .max(f64::EPSILON);
    for (index, merge) in tree.merges.iter().enumerate() {
        let node = n + index;
        x[node] = 0.5 * (x[merge.left] + x[merge.right]);
        y[node] = heatmap_top - dendrogram_height * merge.height / max_height;
        canvas.add_line(
            x[merge.left],
            y[node],
            x[merge.right],
            y[node],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[merge.left],
            y[node],
            x[merge.left],
            y[merge.left],
            canvas.theme.axis_colour,
            1.0,
        );
        canvas.add_line(
            x[merge.right],
            y[node],
            x[merge.right],
            y[merge.right],
            canvas.theme.axis_colour,
            1.0,
        );
    }
}

pub(super) fn hidden_heatmap_order(
    opts: &HashMap<String, Value>,
    key: &str,
    size: usize,
) -> Result<Option<Vec<usize>>> {
    let Some(value) = opts.get(key) else {
        return Ok(None);
    };
    let Value::List(items) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal option '{key}' must be a List"),
            None,
        ));
    };
    let mut order = Vec::with_capacity(items.len());
    for item in items.iter() {
        let index = item.as_float().map(|value| value as usize).ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal option '{key}' must contain indices"),
                None,
            )
        })?;
        order.push(index);
    }
    let mut sorted = order.clone();
    sorted.sort_unstable();
    if order.len() != size || sorted != (0..size).collect::<Vec<_>>() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal option '{key}' is not a permutation"),
            None,
        ));
    }
    Ok(Some(order))
}

pub(super) fn hidden_heatmap_tree(
    opts: &HashMap<String, Value>,
    key: &str,
    order: &[usize],
) -> Result<Option<HeatmapTree>> {
    let Some(value) = opts.get(key) else {
        return Ok(None);
    };
    let Value::Table(table) = value else {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal option '{key}' must be a Table"),
            None,
        ));
    };
    for required in ["left", "right", "height"] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal tree '{key}' is missing '{required}'"),
                None,
            ));
        }
    }
    if table.num_rows() != order.len().saturating_sub(1) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!("clustered_heatmap() internal tree '{key}' has the wrong merge count"),
            None,
        ));
    }
    let left = table.col_index("left").unwrap();
    let right = table.col_index("right").unwrap();
    let height = table.col_index("height").unwrap();
    let mut merges = Vec::with_capacity(table.num_rows());
    for (step, row) in table.rows.iter().enumerate() {
        let left_node = row[left]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "clustered_heatmap() internal tree '{key}' contains a non-numeric node"
                    ),
                    None,
                )
            })?;
        let right_node = row[right]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!(
                        "clustered_heatmap() internal tree '{key}' contains a non-numeric node"
                    ),
                    None,
                )
            })?;
        let merge_height = row[height].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal tree '{key}' contains a non-numeric height"),
                None,
            )
        })?;
        let available_nodes = order.len() + step;
        if left_node >= available_nodes
            || right_node >= available_nodes
            || !merge_height.is_finite()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("clustered_heatmap() internal tree '{key}' contains an invalid merge"),
                None,
            ));
        }
        merges.push(HeatmapMerge {
            left: left_node,
            right: right_node,
            height: merge_height,
        });
    }
    Ok(Some(HeatmapTree {
        merges,
        order: order.to_vec(),
    }))
}

pub(super) fn heatmap_tree_table(tree: Option<&HeatmapTree>) -> Value {
    Value::Table(Table::new(
        ["left", "right", "height"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        tree.map(|tree| {
            tree.merges
                .iter()
                .map(|merge| {
                    vec![
                        Value::Int(merge.left as i64),
                        Value::Int(merge.right as i64),
                        Value::Float(merge.height),
                    ]
                })
                .collect()
        })
        .unwrap_or_default(),
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn clustered_heatmap_spec_value(
    data: &[Vec<f64>],
    row_names: &[String],
    col_names: &[String],
    row_order: &[usize],
    col_order: &[usize],
    row_tree: Option<&HeatmapTree>,
    column_tree: Option<&HeatmapTree>,
    value_min: f64,
    value_max: f64,
    scale_min: f64,
    scale_max: f64,
    use_diverging: bool,
    opts: &HashMap<String, Value>,
) -> Value {
    let row_position = row_order
        .iter()
        .enumerate()
        .map(|(display, &source)| (source, display))
        .collect::<HashMap<_, _>>();
    let col_position = col_order
        .iter()
        .enumerate()
        .map(|(display, &source)| (source, display))
        .collect::<HashMap<_, _>>();
    let cells = data
        .iter()
        .enumerate()
        .flat_map(|(source_row, row)| {
            let row_position = &row_position;
            let col_position = &col_position;
            row.iter().enumerate().map(move |(source_col, &value)| {
                vec![
                    Value::Int(source_row as i64),
                    Value::Int(row_position[&source_row] as i64),
                    Value::Int(source_col as i64),
                    Value::Int(col_position[&source_col] as i64),
                    Value::Float(value),
                ]
            })
        })
        .collect();
    let rows = row_names
        .iter()
        .enumerate()
        .map(|(source, label)| {
            vec![
                Value::Int(source as i64),
                Value::Int(row_position[&source] as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let columns = col_names
        .iter()
        .enumerate()
        .map(|(source, label)| {
            vec![
                Value::Int(source as i64),
                Value::Int(col_position[&source] as i64),
                Value::Str(label.clone()),
            ]
        })
        .collect();
    let non_finite = data
        .iter()
        .flat_map(|row| row.iter())
        .filter(|value| !value.is_finite())
        .count();
    let options = HashMap::from([
        ("plot".into(), Value::Str("clustered_heatmap".into())),
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", "Clustered Heatmap").into()),
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
            "theme".into(),
            Value::Str(get_opt_str(opts, "theme", "").into()),
        ),
        (
            "order".into(),
            Value::Str(get_opt_str(opts, "order", "nearest").into()),
        ),
        (
            "linkage".into(),
            Value::Str(get_opt_str(opts, "linkage", "complete").into()),
        ),
        (
            "distance".into(),
            Value::Str(get_opt_str(opts, "distance", "euclidean").into()),
        ),
        (
            "dendrogram".into(),
            Value::Str(
                get_opt_str(
                    opts,
                    "dendrogram",
                    if row_tree.is_some() { "both" } else { "none" },
                )
                .into(),
            ),
        ),
        (
            "chars".into(),
            Value::Str(get_opt_str(opts, "chars", " ░▒▓█").into()),
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
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("heatmap".into())),
            ("plot".into(), Value::Str("clustered_heatmap".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "Clustered Heatmap").into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "source_row",
                        "display_row",
                        "source_col",
                        "display_col",
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
                    ["source_row", "display_row", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    rows,
                )),
            ),
            (
                "columns".into(),
                Value::Table(Table::new(
                    ["source_col", "display_col", "label"]
                        .into_iter()
                        .map(str::to_string)
                        .collect(),
                    columns,
                )),
            ),
            ("row_merges".into(), heatmap_tree_table(row_tree)),
            ("column_merges".into(), heatmap_tree_table(column_tree)),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("clustered_heatmap".into())),
                        ("input_rows".into(), Value::Int(data.len() as i64)),
                        (
                            "input_columns".into(),
                            Value::Int(data.first().map(Vec::len).unwrap_or(0) as i64),
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
                            "{non_finite} clustered-heatmap cells are non-finite"
                        ))]
                    }
                    .into(),
                ),
            ),
        ])
        .into(),
    )
}

pub(crate) fn is_clustered_heatmap_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "heatmap")
                && matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "clustered_heatmap")
    )
}

pub(crate) fn render_clustered_heatmap_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_clustered_heatmap_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 clustered-heatmap Record",
                None,
            ))
        }
    };
    let table_field = |name: &str| -> Result<&Table> {
        match map.get(name) {
            Some(Value::Table(table)) => Ok(table),
            _ => Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "render_plot() clustered-heatmap specification field '{name}' must be Table"
                ),
                None,
            )),
        }
    };
    let cells = table_field("data")?;
    let rows = table_field("rows")?;
    let columns = table_field("columns")?;
    let row_merges = table_field("row_merges")?;
    let column_merges = table_field("column_merges")?;
    for required in [
        "source_row",
        "display_row",
        "source_col",
        "display_col",
        "value",
    ] {
        if cells.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() clustered-heatmap data is missing '{required}'"),
                None,
            ));
        }
    }
    if rows.num_rows() == 0 || columns.num_rows() == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() clustered-heatmap specification is empty",
            None,
        ));
    }
    for (table, source, display) in [
        (rows, "source_row", "display_row"),
        (columns, "source_col", "display_col"),
    ] {
        for required in [source, display, "label"] {
            if table.col_index(required).is_none() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() clustered-heatmap metadata is missing '{required}'"),
                    None,
                ));
            }
        }
    }
    let ordered_metadata =
        |table: &Table, source: &str, display: &str| -> Result<(Vec<String>, Vec<usize>)> {
            let source_index = table.col_index(source).unwrap();
            let display_index = table.col_index(display).unwrap();
            let label_index = table.col_index("label").unwrap();
            let mut labels = vec![String::new(); table.num_rows()];
            let mut order = vec![usize::MAX; table.num_rows()];
            for row in &table.rows {
                let source_value = row[source_index]
                    .as_float()
                    .map(|value| value as usize)
                    .ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "render_plot() clustered-heatmap source indices must be numeric",
                            None,
                        )
                    })?;
                let display_value = row[display_index]
                    .as_float()
                    .map(|value| value as usize)
                    .ok_or_else(|| {
                        BioLangError::runtime(
                            ErrorKind::TypeError,
                            "render_plot() clustered-heatmap display indices must be numeric",
                            None,
                        )
                    })?;
                if source_value >= labels.len()
                    || display_value >= order.len()
                    || order[display_value] != usize::MAX
                {
                    return Err(BioLangError::runtime(
                        ErrorKind::TypeError,
                        "render_plot() clustered-heatmap metadata indices are invalid",
                        None,
                    ));
                }
                labels[source_value] = format!("{}", row[label_index]);
                order[display_value] = source_value;
            }
            if order.contains(&usize::MAX) {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() clustered-heatmap metadata is incomplete",
                    None,
                ));
            }
            Ok((labels, order))
        };
    let (row_labels, row_order) = ordered_metadata(rows, "source_row", "display_row")?;
    let (col_labels, col_order) = ordered_metadata(columns, "source_col", "display_col")?;
    if cells.num_rows() != row_labels.len() * col_labels.len() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() clustered-heatmap data must contain one cell per row and column",
            None,
        ));
    }
    let sr = cells.col_index("source_row").unwrap();
    let sc = cells.col_index("source_col").unwrap();
    let vi = cells.col_index("value").unwrap();
    let mut matrix = vec![vec![f64::NAN; col_labels.len()]; row_labels.len()];
    for (expected, row) in cells.rows.iter().enumerate() {
        let source_row = row[sr]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() clustered-heatmap source_row must be numeric",
                    None,
                )
            })?;
        let source_col = row[sc]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() clustered-heatmap source_col must be numeric",
                    None,
                )
            })?;
        if source_row >= matrix.len()
            || source_col >= col_labels.len()
            || expected != source_row * col_labels.len() + source_col
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap cells must be ordered by source row and column",
                None,
            ));
        }
        matrix[source_row][source_col] = row[vi].as_float().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap values must be numeric",
                None,
            )
        })?;
    }
    let mut table_columns = Vec::with_capacity(col_labels.len() + 1);
    table_columns.push("gene".to_string());
    table_columns.extend(col_labels.iter().cloned());
    let table_rows = matrix
        .iter()
        .enumerate()
        .map(|(index, values)| {
            let mut row = Vec::with_capacity(values.len() + 1);
            row.push(Value::Str(row_labels[index].clone()));
            row.extend(values.iter().map(|value| Value::Float(*value)));
            row
        })
        .collect();
    let input = Value::Table(Table::new(table_columns, table_rows));
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap specification field 'options' must be Record",
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
    options.insert("format".into(), Value::Str("svg".into()));
    options.insert(
        "__row_order".into(),
        Value::List(
            row_order
                .iter()
                .map(|index| Value::Int(*index as i64))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    options.insert(
        "__col_order".into(),
        Value::List(
            col_order
                .iter()
                .map(|index| Value::Int(*index as i64))
                .collect::<Vec<_>>()
                .into(),
        ),
    );
    options.insert("__row_tree".into(), Value::Table(row_merges.clone()));
    options.insert("__column_tree".into(), Value::Table(column_merges.clone()));
    for key in ["scale_min", "scale_max", "diverging"] {
        let value = options.get(key).cloned().ok_or_else(|| {
            BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() clustered-heatmap options are missing '{key}'"),
                None,
            )
        })?;
        options.insert(format!("__{key}"), value);
    }
    let svg = match builtin_clustered_heatmap(vec![input, Value::Record(options.into())])? {
        Value::Str(svg) => svg,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() clustered-heatmap renderer did not return SVG",
                None,
            ))
        }
    };
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Clustered Heatmap");
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
            "render_plot() terminal clustered-heatmap output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown clustered-heatmap format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

pub(super) fn builtin_clustered_heatmap(args: Vec<Value>) -> Result<Value> {
    let opts = parse_options(&args);
    let fmt = get_opt_str(&opts, "format", "svg").to_string();
    let heat_chars: Vec<char> = get_opt_str(&opts, "chars", " ░▒▓█").chars().collect();
    let title = get_opt_str(&opts, "title", "Clustered Heatmap").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let legend_title = get_opt_str(&opts, "legend_title", "value").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let order_method = get_opt_str(&opts, "order", "nearest").to_ascii_lowercase();
    let hierarchical = match order_method.as_str() {
        "nearest" | "nn" => false,
        "hierarchical" | "hclust" => true,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                "clustered_heatmap() order must be nearest or hierarchical; got '{order_method}'"
            ),
                None,
            ))
        }
    };
    let linkage = HeatmapLinkage::parse(get_opt_str(&opts, "linkage", "complete"))?;
    let distance = HeatmapDistance::parse(get_opt_str(&opts, "distance", "euclidean"))?;
    let dendrogram_mode = get_opt_str(
        &opts,
        "dendrogram",
        if hierarchical { "both" } else { "none" },
    )
    .to_ascii_lowercase();
    let (draw_row_tree, draw_column_tree) = match dendrogram_mode.as_str() {
        "both" => (true, true),
        "row" | "rows" => (true, false),
        "column" | "columns" | "col" | "cols" => (false, true),
        "none" => (false, false),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!(
                    "clustered_heatmap() dendrogram must be both, row, column, or none; got '{dendrogram_mode}'"
                ),
                None,
            ))
        }
    };
    if !hierarchical && (draw_row_tree || draw_column_tree) {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "clustered_heatmap() dendrograms require order: \"hierarchical\"",
            None,
        ));
    }

    let (mut row_names, mut col_names, data) = match &args[0] {
        Value::Table(table) => {
            let mut numeric_names = Vec::new();
            let mut cols_data: Vec<Vec<f64>> = Vec::new();
            for col in &table.columns {
                if let Ok(values) = extract_table_col(table, col) {
                    // extract_table_col represents non-numeric strings as
                    // NaN. A gene annotation column therefore parses without
                    // an error but is not a numeric heatmap dimension.
                    if values.iter().any(|value| value.is_finite()) {
                        numeric_names.push(col.clone());
                        cols_data.push(values);
                    }
                }
            }
            if cols_data.is_empty() {
                return Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    "clustered_heatmap() table contains no numeric columns",
                    None,
                ));
            }
            let (nrows, ncols) = (table.num_rows(), cols_data.len());
            let mut t = vec![vec![0.0; ncols]; nrows];
            for c in 0..ncols {
                for r in 0..nrows {
                    t[r][c] = cols_data[c][r];
                }
            }
            let label_column = ["gene", "feature", "name", "marker"]
                .iter()
                .find(|name| table.col_index(name).is_some());
            let rn = label_column
                .and_then(|name| extract_str_col(table, name).ok())
                .unwrap_or_else(|| (0..nrows).map(|i| format!("row{i}")).collect());
            (rn, numeric_names, t)
        }
        Value::Matrix(m) => {
            let rn = m
                .row_names
                .clone()
                .unwrap_or_else(|| (0..m.nrow).map(|i| format!("row{i}")).collect());
            let cn = m
                .col_names
                .clone()
                .unwrap_or_else(|| (0..m.ncol).map(|i| format!("col{i}")).collect());
            let mut data = vec![vec![0.0; m.ncol]; m.nrow];
            for r in 0..m.nrow {
                for c in 0..m.ncol {
                    data[r][c] = m.data[r * m.ncol + c];
                }
            }
            (rn, cn, data)
        }
        _ => {
            return Err(BioLangError::type_error(
                "clustered_heatmap() requires Table or Matrix",
                None,
            ))
        }
    };
    let option_labels = |key: &str| -> Option<Vec<String>> {
        match opts.get(key) {
            Some(Value::List(items)) => Some(items.iter().map(|item| format!("{item}")).collect()),
            _ => None,
        }
    };
    if let Some(labels) = option_labels("row_labels") {
        for (index, label) in labels.into_iter().enumerate().take(row_names.len()) {
            row_names[index] = label;
        }
    }
    if let Some(labels) = option_labels("col_labels") {
        for (index, label) in labels.into_iter().enumerate().take(col_names.len()) {
            col_names[index] = label;
        }
    }
    let nrows = data.len();
    let ncols = if nrows > 0 { data[0].len() } else { 0 };
    if nrows == 0 || ncols == 0 {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "clustered_heatmap() received empty data",
            None,
        ));
    }
    let col_data: Vec<Vec<f64>> = (0..ncols)
        .map(|c| (0..nrows).map(|r| data[r][c]).collect())
        .collect();
    let frozen_row_order = hidden_heatmap_order(&opts, "__row_order", nrows)?;
    let frozen_col_order = hidden_heatmap_order(&opts, "__col_order", ncols)?;
    let row_tree = if hierarchical {
        if let Some(order) = frozen_row_order.as_deref() {
            match hidden_heatmap_tree(&opts, "__row_tree", order)? {
                Some(tree) => Some(tree),
                None => Some(hierarchical_heatmap_tree(&data, distance, linkage)?),
            }
        } else {
            Some(hierarchical_heatmap_tree(&data, distance, linkage)?)
        }
    } else {
        None
    };
    let column_tree = if hierarchical {
        if let Some(order) = frozen_col_order.as_deref() {
            match hidden_heatmap_tree(&opts, "__column_tree", order)? {
                Some(tree) => Some(tree),
                None => Some(hierarchical_heatmap_tree(&col_data, distance, linkage)?),
            }
        } else {
            Some(hierarchical_heatmap_tree(&col_data, distance, linkage)?)
        }
    } else {
        None
    };
    let row_order = frozen_row_order.unwrap_or_else(|| {
        row_tree
            .as_ref()
            .map(|tree| tree.order.clone())
            .unwrap_or_else(|| nn_order(&data))
    });
    let col_order = frozen_col_order.unwrap_or_else(|| {
        column_tree
            .as_ref()
            .map(|tree| tree.order.clone())
            .unwrap_or_else(|| nn_order(&col_data))
    });
    let all: Vec<f64> = data
        .iter()
        .flat_map(|r| r.iter().copied())
        .filter(|v| v.is_finite())
        .collect();
    let (vmin, vmax) = if all.is_empty() {
        (0.0, 1.0)
    } else {
        col_range(&all)
    };
    let requested_centre = opts.get("center").and_then(Value::as_float);
    let use_diverging = opts
        .get("__diverging")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            publication_theme && (requested_centre.is_some() || (vmin < 0.0 && vmax > 0.0))
        });
    let frozen_scale = opts
        .get("__scale_min")
        .and_then(Value::as_float)
        .zip(opts.get("__scale_max").and_then(Value::as_float));
    let (scale_min, scale_max) = if let Some(domain) = frozen_scale {
        domain
    } else if use_diverging {
        let centre = requested_centre.unwrap_or(0.0);
        let radius = (vmin - centre)
            .abs()
            .max((vmax - centre).abs())
            .max(f64::EPSILON);
        (centre - radius, centre + radius)
    } else {
        (vmin, vmax)
    };

    if matches!(fmt.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = clustered_heatmap_spec_value(
            &data,
            &row_names,
            &col_names,
            &row_order,
            &col_order,
            row_tree.as_ref(),
            column_tree.as_ref(),
            vmin,
            vmax,
            scale_min,
            scale_max,
            use_diverging,
            &opts,
        );
        if matches!(fmt.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_clustered_heatmap_spec_value(&spec, &opts);
    }
    let colour = |t: f64| {
        if publication_theme {
            if use_diverging {
                publication_diverging_color(t)
            } else {
                publication_sequential_color(t)
            }
        } else {
            sequential_color(t)
        }
    };

    if fmt == "svg" {
        let w = get_opt_f64(&opts, "width", 800.0);
        let h = get_opt_f64(&opts, "height", 600.0);
        let mut c = SvgCanvas::with_theme(w, h, theme);
        let row_dendrogram_width = if draw_row_tree {
            (w * 0.12).clamp(28.0, 80.0).min(w * 0.18)
        } else {
            0.0
        };
        let column_dendrogram_height = if draw_column_tree {
            (h * 0.12).clamp(28.0, 70.0).min(h * 0.18)
        } else {
            0.0
        };
        if theme.is_adaptive() {
            let widest_row = row_names
                .iter()
                .map(|label| estimate_text_width(label, theme.tick_size))
                .fold(0.0, f64::max);
            let widest_col = col_names
                .iter()
                .map(|label| estimate_text_width(label, theme.tick_size))
                .fold(0.0, f64::max);
            let legend_label = [scale_min, 0.5 * (scale_min + scale_max), scale_max]
                .iter()
                .map(|value| estimate_text_width(&format!("{value:.2}"), theme.legend_size))
                .fold(0.0, f64::max);
            let label_margin = (widest_row + 12.0).clamp(52.0, w * 0.27);
            c.margin.left =
                (label_margin + row_dendrogram_width + if draw_row_tree { 8.0 } else { 0.0 })
                    .min(w * 0.43);
            c.margin.right = (42.0
                + legend_label.max(estimate_text_width(&legend_title, theme.legend_size)))
            .clamp(78.0, w * 0.31);
            c.margin.top = if title.is_empty() {
                20.0
            } else if subtitle.is_empty() {
                48.0
            } else {
                66.0
            } + column_dendrogram_height
                + if draw_column_tree { 7.0 } else { 0.0 };
            c.margin.bottom = (widest_col * 0.72 + 18.0).clamp(48.0, h * 0.28)
                + if caption.is_empty() { 0.0 } else { 18.0 };
        } else {
            c.margin.left = 80.0 + row_dendrogram_width + if draw_row_tree { 8.0 } else { 0.0 };
            c.margin.top += column_dendrogram_height + if draw_column_tree { 7.0 } else { 0.0 };
            c.margin.bottom = 60.0;
        }
        let cw = c.plot_width() / ncols as f64;
        let ch = c.plot_height() / nrows as f64;
        for (ri, &row_i) in row_order.iter().enumerate() {
            for (ci, &col_i) in col_order.iter().enumerate() {
                let v = data[row_i][col_i];
                let t = if (scale_max - scale_min).abs() < f64::EPSILON {
                    0.5
                } else {
                    (v - scale_min) / (scale_max - scale_min)
                };
                let x = c.margin.left + ci as f64 * cw;
                let y = c.margin.top + ri as f64 * ch;
                c.add_rect(x, y, cw, ch, &colour(t));
                if theme.is_adaptive() && cw.min(ch) >= 4.0 {
                    c.elements.push(format!(
                        r#"<rect x="{x:.1}" y="{y:.1}" width="{cw:.1}" height="{ch:.1}" fill="none" stroke="{}" stroke-width="0.5" />"#,
                        theme.grid_colour
                    ));
                }
            }
        }
        if hierarchical {
            let row_heights = row_tree
                .as_ref()
                .map(|tree| {
                    tree.merges
                        .iter()
                        .map(|merge| format!("{:.12}", merge.height))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let column_heights = column_tree
                .as_ref()
                .map(|tree| {
                    tree.merges
                        .iter()
                        .map(|merge| format!("{:.12}", merge.height))
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            let row_order_metadata = row_order
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",");
            let column_order_metadata = col_order
                .iter()
                .map(usize::to_string)
                .collect::<Vec<_>>()
                .join(",");
            c.elements.push(format!(
                r#"<g data-biolang-clustering="hierarchical" data-distance="{}" data-linkage="{}" data-dendrogram="{}" data-row-order="{}" data-row-heights="{}" data-column-order="{}" data-column-heights="{}">"#,
                distance.name(),
                linkage.name(),
                dendrogram_mode,
                row_order_metadata,
                row_heights,
                column_order_metadata,
                column_heights
            ));
            let heatmap_left = c.margin.left;
            let heatmap_top = c.margin.top;
            if draw_row_tree {
                if let Some(tree) = &row_tree {
                    draw_row_dendrogram(
                        &mut c,
                        tree,
                        heatmap_left,
                        heatmap_top,
                        ch,
                        row_dendrogram_width,
                    );
                }
            }
            if draw_column_tree {
                if let Some(tree) = &column_tree {
                    draw_column_dendrogram(
                        &mut c,
                        tree,
                        heatmap_left,
                        heatmap_top,
                        cw,
                        column_dendrogram_height,
                    );
                }
            }
            c.elements.push("</g>".to_string());
        }
        let row_step = if theme.is_adaptive() {
            (10.0 / ch.max(1.0)).ceil().max(1.0) as usize
        } else {
            1
        };
        for (ri, &row_i) in row_order.iter().enumerate().step_by(row_step) {
            c.add_text(
                c.margin.left - row_dendrogram_width - if draw_row_tree { 7.0 } else { 3.0 },
                c.margin.top + (ri as f64 + 0.5) * ch + 4.0,
                &row_names[row_i],
                "end",
                if theme.is_adaptive() {
                    theme.tick_size
                } else {
                    9.0
                },
            );
        }
        if theme.is_adaptive() {
            let col_step = (10.0 / cw.max(1.0)).ceil().max(1.0) as usize;
            let label_y = c.margin.top + c.plot_height() + 10.0;
            for (ci, &col_i) in col_order.iter().enumerate().step_by(col_step) {
                c.add_text_rotated(
                    c.margin.left + (ci as f64 + 0.5) * cw,
                    label_y,
                    &col_names[col_i],
                    45.0,
                    "start",
                    theme.tick_size,
                );
            }

            let legend_x = c.margin.left + c.plot_width() + 14.0;
            let legend_top = c.margin.top;
            let legend_height = c.plot_height().min(180.0);
            c.add_text(
                legend_x,
                legend_top - 8.0,
                &legend_title,
                "start",
                theme.legend_size,
            );
            for step in 0..40 {
                let t = 1.0 - step as f64 / 39.0;
                c.add_rect(
                    legend_x,
                    legend_top + step as f64 * legend_height / 40.0,
                    12.0,
                    legend_height / 40.0 + 0.5,
                    &colour(t),
                );
            }
            for (value, y) in [
                (scale_max, legend_top + 4.0),
                (
                    (scale_min + scale_max) / 2.0,
                    legend_top + legend_height / 2.0 + 3.0,
                ),
                (scale_min, legend_top + legend_height + 3.0),
            ] {
                c.add_text(
                    legend_x + 17.0,
                    y,
                    &format!("{value:.2}"),
                    "start",
                    theme.legend_size,
                );
            }
        }
        c.set_accessible_description(if hierarchical {
            format!(
                "Heatmap with {nrows} rows and {ncols} columns, hierarchically ordered using {} distance and {} linkage; dendrogram display: {}.",
                distance.name(),
                linkage.name(),
                dendrogram_mode
            )
        } else {
            format!(
                "Heatmap with {nrows} rows and {ncols} columns, ordered by deterministic nearest-neighbour traversal from the first row and first column."
            )
        });
        c.draw_title(&title);
        if theme.is_adaptive() {
            c.draw_subtitle(&subtitle);
            c.draw_caption(&caption);
        }
        return Ok(Value::Str(c.render()));
    }

    let max_rl = row_names.iter().map(|s| s.len()).max().unwrap_or(0);
    let nlevels = heat_chars.len();
    let mut out = format!("  {title}\n");
    if hierarchical {
        out.push_str(&format!(
            "  order: hierarchical; distance: {}; linkage: {}\n",
            distance.name(),
            linkage.name()
        ));
    }
    out.push_str(&format!("  {:>w$}  ", "", w = max_rl));
    for &ci in &col_order {
        out.push_str(&format!(
            "{} ",
            &col_names[ci].chars().take(2).collect::<String>()
        ));
    }
    out.push('\n');
    for &ri in &row_order {
        out.push_str(&format!("  {:>w$}  ", row_names[ri], w = max_rl));
        for &ci in &col_order {
            let t = if (vmax - vmin).abs() < f64::EPSILON {
                0.5
            } else {
                (data[ri][ci] - vmin) / (vmax - vmin)
            };
            out.push(
                heat_chars[(t * (nlevels - 1) as f64)
                    .round()
                    .clamp(0.0, (nlevels - 1) as f64) as usize],
            );
            out.push_str("  ");
        }
        out.push('\n');
    }
    write_output(&out);
    Ok(Value::Nil)
}

/// Genes against clusters, where a dot's size is how many cells express the
/// gene and its colour is how strongly.
///
/// Seurat's DotPlot, and the figure that actually settles cell-type calls. A
/// feature plot shows one gene at a time and a heatmap of means hides how many
/// cells are behind each one - which matters, because a gene blazing in 5% of a
/// cluster and one steady across 90% give the same mean and mean opposite
/// things. Encoding both is the whole point, so both are drawn: area for
/// detection rate, colour for level.
///
/// Colour is the mean expression z-scored per gene across clusters, as Seurat
/// scales it. Without that a housekeeping gene at high absolute expression
/// washes out every marker on the plot; with it, each row says where that gene
/// is relatively highest, which is the question being asked.
pub(super) fn render_dot_plot_geometry_svg(
    gene_names: &[String],
    cluster_names: &[String],
    detected: &[Vec<f64>],
    scaled: &[Vec<f64>],
    opts: &HashMap<String, Value>,
) -> Result<String> {
    const CLIP: f64 = 2.5;
    let title = get_opt_str(opts, "title", "Marker expression").to_string();
    let subtitle = get_opt_str(opts, "subtitle", "").to_string();
    let caption = get_opt_str(opts, "caption", "").to_string();
    let theme = plot_theme(opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let cell = get_opt_f64(opts, "cell", 26.0);
    let requested_width = get_opt_f64(opts, "width", 0.0);
    let requested_height = get_opt_f64(opts, "height", 0.0);
    let widest_gene = gene_names
        .iter()
        .map(|gene| estimate_text_width(gene, theme.tick_size))
        .fold(0.0, f64::max);
    let widest_cluster = cluster_names
        .iter()
        .map(|cluster| estimate_text_width(cluster, theme.tick_size))
        .fold(0.0, f64::max);
    let adaptive_left = (widest_gene + 14.0).clamp(64.0, 180.0);
    let adaptive_top =
        (54.0 + if subtitle.is_empty() { 0.0 } else { 18.0 } + widest_cluster * 0.68)
            .clamp(72.0, 132.0);
    let adaptive_right = 112.0;
    let adaptive_bottom = if caption.is_empty() { 16.0 } else { 32.0 };
    let width = if requested_width > 0.0 {
        requested_width
    } else if theme.is_adaptive() {
        adaptive_left + cell * cluster_names.len() as f64 + adaptive_right
    } else {
        180.0 + cell * cluster_names.len() as f64 + 120.0
    };
    let height = if requested_height > 0.0 {
        requested_height
    } else if theme.is_adaptive() {
        adaptive_top + cell * gene_names.len() as f64 + adaptive_bottom
    } else {
        90.0 + cell * gene_names.len() as f64
    };
    let mut canvas = SvgCanvas::with_theme(width, height, theme);
    if theme.is_adaptive() {
        canvas.margin.left = adaptive_left.min(width * 0.34);
        canvas.margin.right = adaptive_right.min(width * 0.36);
        canvas.margin.top = adaptive_top.min(height * 0.38);
        canvas.margin.bottom = adaptive_bottom.min(height * 0.15);
    }
    let left = if theme.is_adaptive() {
        canvas.margin.left
    } else {
        130.0
    };
    let top = if theme.is_adaptive() {
        canvas.margin.top
    } else {
        60.0
    };
    let cell_x = if theme.is_adaptive() {
        canvas.plot_width() / cluster_names.len().max(1) as f64
    } else {
        cell
    };
    let cell_y = if theme.is_adaptive() {
        canvas.plot_height() / gene_names.len().max(1) as f64
    } else {
        cell
    };
    let radius_max = if theme.is_adaptive() {
        (cell_x.min(cell_y) * 0.40).min(12.0)
    } else {
        cell * 0.42
    };

    if theme.is_adaptive() {
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for column in 0..=cluster_names.len() {
            let x = left + cell_x * column as f64;
            canvas.add_line(
                x,
                top,
                x,
                top + canvas.plot_height(),
                theme.grid_colour,
                theme.grid_width,
            );
        }
        for row in 0..=gene_names.len() {
            let y = top + cell_y * row as f64;
            canvas.add_line(
                left,
                y,
                left + canvas.plot_width(),
                y,
                theme.grid_colour,
                theme.grid_width,
            );
        }
    }
    for (column, cluster) in cluster_names.iter().enumerate() {
        let x = left + cell_x * (column as f64 + 0.5);
        canvas.add_text_rotated(
            x,
            top - 10.0,
            cluster,
            -45.0,
            "start",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
    }
    for (row, gene) in gene_names.iter().enumerate() {
        let y = top + cell_y * (row as f64 + 0.5);
        canvas.add_text(
            left - 8.0,
            y + 3.0,
            gene,
            "end",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
        for column in 0..cluster_names.len() {
            let fraction = detected[row][column];
            if fraction <= 0.0 {
                continue;
            }
            let x = left + cell_x * (column as f64 + 0.5);
            let radius = radius_max * fraction.sqrt();
            let t = (scaled[row][column] + CLIP) / (2.0 * CLIP);
            let colour = if publication_theme {
                publication_diverging_color(t)
            } else {
                sequential_color(t)
            };
            canvas.add_circle(x, y, radius, &colour);
        }
    }
    let legend_x =
        left + if theme.is_adaptive() {
            canvas.plot_width()
        } else {
            cell * cluster_names.len() as f64
        } + if theme.is_adaptive() { 16.0 } else { 24.0 };
    let legend_size = if theme.is_adaptive() {
        theme.legend_size
    } else {
        9.0
    };
    canvas.add_text(legend_x, top + 4.0, "% detected", "start", legend_size);
    for (i, fraction) in [0.25_f64, 0.5, 1.0].iter().enumerate() {
        let y = top + 22.0 + i as f64 * if theme.is_adaptive() { 25.0 } else { 18.0 };
        canvas.add_circle(legend_x + 8.0, y, radius_max * fraction.sqrt(), "#888888");
        canvas.add_text(
            legend_x + 22.0,
            y + 3.0,
            &format!("{:.0}%", fraction * 100.0),
            "start",
            8.0,
        );
    }
    let bar_top = top + if theme.is_adaptive() { 112.0 } else { 90.0 };
    canvas.add_text(legend_x, bar_top - 6.0, "z-score", "start", legend_size);
    for step in 0..24 {
        let t = 1.0 - step as f64 / 23.0;
        let colour = if publication_theme {
            publication_diverging_color(t)
        } else {
            sequential_color(t)
        };
        canvas.add_rect(legend_x, bar_top + step as f64 * 3.0, 10.0, 3.4, &colour);
    }
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 6.0,
        &format!("{CLIP:.1}"),
        "start",
        8.0,
    );
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 72.0,
        &format!("{:.1}", -CLIP),
        "start",
        8.0,
    );
    canvas.set_accessible_description(format!(
        "Single-cell dot plot for {} genes across {} clusters. Dot area encodes the percentage of detected cells and colour encodes per-gene z-scored mean expression.",
        gene_names.len(),
        cluster_names.len()
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

pub(super) fn dot_plot_spec_value(
    gene_names: &[String],
    cluster_names: &[String],
    means: &[Vec<f64>],
    detected: &[Vec<f64>],
    scaled: &[Vec<f64>],
    input_cells: usize,
    opts: &HashMap<String, Value>,
) -> Value {
    let rows = gene_names
        .iter()
        .enumerate()
        .flat_map(|(gene_index, gene)| {
            cluster_names
                .iter()
                .enumerate()
                .map(move |(cluster_index, cluster)| {
                    vec![
                        Value::Int(gene_index as i64),
                        Value::Str(gene.clone()),
                        Value::Int(cluster_index as i64),
                        Value::Str(cluster.clone()),
                        Value::Float(means[gene_index][cluster_index]),
                        Value::Float(detected[gene_index][cluster_index]),
                        Value::Float(scaled[gene_index][cluster_index]),
                    ]
                })
        })
        .collect();
    let options = HashMap::from([
        (
            "title".into(),
            Value::Str(get_opt_str(opts, "title", "Marker expression").into()),
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
        ("cell".into(), Value::Float(get_opt_f64(opts, "cell", 26.0))),
        (
            "width".into(),
            Value::Float(get_opt_f64(opts, "width", 0.0)),
        ),
        (
            "height".into(),
            Value::Float(get_opt_f64(opts, "height", 0.0)),
        ),
        ("z_score_clip".into(), Value::Float(2.5)),
    ]);
    Value::Record(
        HashMap::from([
            (
                "schema".into(),
                Value::Str(crate::plot::PLOT_SPEC_SCHEMA.into()),
            ),
            ("kind".into(), Value::Str("dot_plot".into())),
            (
                "title".into(),
                Value::Str(get_opt_str(opts, "title", "Marker expression").into()),
            ),
            (
                "data".into(),
                Value::Table(Table::new(
                    [
                        "gene_index",
                        "gene",
                        "cluster_index",
                        "cluster",
                        "mean_expression",
                        "detected_fraction",
                        "scaled_expression",
                    ]
                    .into_iter()
                    .map(str::to_string)
                    .collect(),
                    rows,
                )),
            ),
            ("options".into(), Value::Record(options.into())),
            (
                "provenance".into(),
                Value::Record(
                    HashMap::from([
                        ("builtin".into(), Value::Str("dot_plot".into())),
                        ("input_cells".into(), Value::Int(input_cells as i64)),
                        ("genes".into(), Value::Int(gene_names.len() as i64)),
                        ("clusters".into(), Value::Int(cluster_names.len() as i64)),
                    ])
                    .into(),
                ),
            ),
            ("warnings".into(), Value::List(Vec::<Value>::new().into())),
        ])
        .into(),
    )
}

pub(crate) fn is_dot_plot_spec(value: &Value) -> bool {
    matches!(
        value,
        Value::Record(map)
            if matches!(map.get("schema"), Some(Value::Str(schema)) if schema == crate::plot::PLOT_SPEC_SCHEMA)
                && matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "dot_plot")
    )
}

pub(crate) fn render_dot_plot_spec_value(
    value: &Value,
    render_options: &HashMap<String, Value>,
) -> Result<Value> {
    let map = match value {
        Value::Record(map) if is_dot_plot_spec(value) => map,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() requires a biolang.plot.spec/v1 dot-plot Record",
                None,
            ))
        }
    };
    let table = match map.get("data") {
        Some(Value::Table(table)) => table,
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot specification field 'data' must be Table",
                None,
            ))
        }
    };
    for required in [
        "gene_index",
        "gene",
        "cluster_index",
        "cluster",
        "mean_expression",
        "detected_fraction",
        "scaled_expression",
    ] {
        if table.col_index(required).is_none() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                format!("render_plot() dot-plot data is missing '{required}'"),
                None,
            ));
        }
    }
    let column = |name: &str| table.col_index(name).unwrap();
    let mut gene_names = Vec::<String>::new();
    let mut cluster_names = Vec::<String>::new();
    for row in &table.rows {
        let gene = row[column("gene_index")]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() dot-plot gene_index must be numeric",
                    None,
                )
            })?;
        let cluster = row[column("cluster_index")]
            .as_float()
            .map(|value| value as usize)
            .ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    "render_plot() dot-plot cluster_index must be numeric",
                    None,
                )
            })?;
        if gene == gene_names.len() {
            gene_names.push(format!("{}", row[column("gene")]));
        } else if gene > gene_names.len() {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot gene indices must be contiguous",
                None,
            ));
        }
        if gene == 0 && cluster == cluster_names.len() {
            cluster_names.push(format!("{}", row[column("cluster")]));
        }
    }
    if gene_names.is_empty() || cluster_names.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() dot-plot specification is empty",
            None,
        ));
    }
    let expected = gene_names.len() * cluster_names.len();
    if table.num_rows() != expected {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "render_plot() dot-plot data must contain one row per gene-cluster pair",
            None,
        ));
    }
    let mut means = vec![vec![0.0; cluster_names.len()]; gene_names.len()];
    let mut detected = means.clone();
    let mut scaled = means.clone();
    for (expected_row, row) in table.rows.iter().enumerate() {
        let gene = row[column("gene_index")].as_float().unwrap() as usize;
        let cluster = row[column("cluster_index")].as_float().unwrap() as usize;
        if expected_row != gene * cluster_names.len() + cluster
            || gene >= gene_names.len()
            || cluster >= cluster_names.len()
        {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot rows must be ordered by gene and cluster index",
                None,
            ));
        }
        let number = |name: &str| -> Result<f64> {
            let value = row[column(name)].as_float().ok_or_else(|| {
                BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() dot-plot field '{name}' must be numeric"),
                    None,
                )
            })?;
            if value.is_finite() {
                Ok(value)
            } else {
                Err(BioLangError::runtime(
                    ErrorKind::TypeError,
                    format!("render_plot() dot-plot field '{name}' must be finite"),
                    None,
                ))
            }
        };
        means[gene][cluster] = number("mean_expression")?;
        detected[gene][cluster] = number("detected_fraction")?;
        scaled[gene][cluster] = number("scaled_expression")?;
        if !(0.0..=1.0).contains(&detected[gene][cluster]) {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot detected_fraction must lie between zero and one",
                None,
            ));
        }
    }
    let mut options = match map.get("options") {
        Some(Value::Record(options)) => options.as_ref().clone(),
        _ => {
            return Err(BioLangError::runtime(
                ErrorKind::TypeError,
                "render_plot() dot-plot specification field 'options' must be Record",
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
    let svg =
        render_dot_plot_geometry_svg(&gene_names, &cluster_names, &detected, &scaled, &options)?;
    let title = map
        .get("title")
        .and_then(Value::as_str)
        .unwrap_or("Dot plot");
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
            "render_plot() terminal dot-plot output needs the native build",
            None,
        )),
        _ => Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "render_plot() unknown dot-plot format '{format}', expected svg/ascii/unicode/html/spec"
            ),
            None,
        )),
    }
}

pub(super) fn builtin_dot_plot(args: Vec<Value>) -> Result<Value> {
    let opts: HashMap<String, Value> = match args.get(2) {
        Some(Value::Record(map)) => map.as_ref().clone(),
        _ => HashMap::new(),
    };

    let (n_cells, n_genes, columns) = crate::singlecell::expression_columns(&args[0], "dot_plot")?;

    let labels: Vec<String> = match &args[1] {
        Value::List(items) => items.iter().map(|v| format!("{v}")).collect(),
        _ => {
            return Err(BioLangError::type_error(
                "dot_plot() requires a List of cluster labels, one per cell",
                None,
            ))
        }
    };
    if labels.len() != n_cells {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            format!(
                "dot_plot(): {} cluster labels for {n_cells} cells",
                labels.len()
            ),
            None,
        ));
    }

    let gene_names: Vec<String> = match opts.get("genes") {
        Some(Value::List(items)) => items.iter().map(|v| format!("{v}")).collect(),
        _ => (0..n_genes).map(|g| format!("gene{g}")).collect(),
    };

    // Which genes to draw. Named features keep the caller's order, because a
    // dot plot is usually read as a story - lineage by lineage.
    let selected: Vec<usize> = match opts.get("features").or_else(|| opts.get("markers")) {
        Some(Value::List(items)) => items
            .iter()
            .filter_map(|item| match item {
                Value::Int(i) if (*i as usize) < n_genes => Some(*i as usize),
                other => {
                    let wanted = format!("{other}");
                    gene_names.iter().position(|name| *name == wanted)
                }
            })
            .collect(),
        _ => (0..n_genes).collect(),
    };
    if selected.is_empty() {
        return Err(BioLangError::runtime(
            ErrorKind::TypeError,
            "dot_plot() found none of the requested features".to_string(),
            None,
        ));
    }

    let mut cluster_order: Vec<String> = Vec::new();
    let mut members: HashMap<String, Vec<usize>> = HashMap::new();
    for (cell, label) in labels.iter().enumerate() {
        if !members.contains_key(label) {
            cluster_order.push(label.clone());
        }
        members.entry(label.clone()).or_default().push(cell);
    }

    // mean expression and detection rate, per gene per cluster.
    let mut means = vec![vec![0.0_f64; cluster_order.len()]; selected.len()];
    let mut detected = vec![vec![0.0_f64; cluster_order.len()]; selected.len()];
    for (row, &gene) in selected.iter().enumerate() {
        let values = &columns[gene];
        for (column, cluster) in cluster_order.iter().enumerate() {
            let cells = &members[cluster];
            if cells.is_empty() {
                continue;
            }
            let mut total = 0.0;
            let mut expressing = 0usize;
            for &cell in cells {
                let value = values[cell];
                total += value;
                if value > 0.0 {
                    expressing += 1;
                }
            }
            means[row][column] = total / cells.len() as f64;
            detected[row][column] = expressing as f64 / cells.len() as f64;
        }
    }

    // z-score each gene across clusters, clipped as Seurat clips it so one
    // extreme cluster cannot flatten the rest of the row.
    const CLIP: f64 = 2.5;
    let scaled: Vec<Vec<f64>> = means
        .iter()
        .map(|row| {
            let n = row.len() as f64;
            let mean = row.iter().sum::<f64>() / n;
            let sd = (row.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n).sqrt();
            row.iter()
                .map(|v| {
                    if sd > 1e-12 {
                        ((v - mean) / sd).clamp(-CLIP, CLIP)
                    } else {
                        0.0
                    }
                })
                .collect()
        })
        .collect();

    let selected_gene_names = selected
        .iter()
        .map(|&gene| gene_names[gene].clone())
        .collect::<Vec<_>>();
    let format = get_opt_str(&opts, "format", "svg").to_ascii_lowercase();
    if matches!(format.as_str(), "spec" | "data" | "html" | "canvas") {
        let spec = dot_plot_spec_value(
            &selected_gene_names,
            &cluster_order,
            &means,
            &detected,
            &scaled,
            n_cells,
            &opts,
        );
        if matches!(format.as_str(), "spec" | "data") {
            return Ok(spec);
        }
        return render_dot_plot_spec_value(&spec, &opts);
    }
    if matches!(format.as_str(), "svg" | "raw") {
        return render_dot_plot_geometry_svg(
            &selected_gene_names,
            &cluster_order,
            &detected,
            &scaled,
            &opts,
        )
        .map(Value::Str);
    }

    let title = get_opt_str(&opts, "title", "Marker expression").to_string();
    let subtitle = get_opt_str(&opts, "subtitle", "").to_string();
    let caption = get_opt_str(&opts, "caption", "").to_string();
    let theme = plot_theme(&opts);
    let publication_theme = theme.kind == PlotThemeKind::Publication;
    let cell = get_opt_f64(&opts, "cell", 26.0);
    let requested_width = get_opt_f64(&opts, "width", 0.0);
    let requested_height = get_opt_f64(&opts, "height", 0.0);
    let widest_gene = selected
        .iter()
        .map(|&gene| estimate_text_width(&gene_names[gene], theme.tick_size))
        .fold(0.0, f64::max);
    let widest_cluster = cluster_order
        .iter()
        .map(|cluster| estimate_text_width(cluster, theme.tick_size))
        .fold(0.0, f64::max);
    let adaptive_left = (widest_gene + 14.0).clamp(64.0, 180.0);
    let adaptive_top =
        (54.0 + if subtitle.is_empty() { 0.0 } else { 18.0 } + widest_cluster * 0.68)
            .clamp(72.0, 132.0);
    let adaptive_right = 112.0;
    let adaptive_bottom = if caption.is_empty() { 16.0 } else { 32.0 };
    // Sized to the grid unless told otherwise: a fixed canvas either crushes 30
    // genes together or strands 3 in a corner.
    let width = if requested_width > 0.0 {
        requested_width
    } else if theme.is_adaptive() {
        adaptive_left + cell * cluster_order.len() as f64 + adaptive_right
    } else {
        180.0 + cell * cluster_order.len() as f64 + 120.0
    };
    let height = if requested_height > 0.0 {
        requested_height
    } else if theme.is_adaptive() {
        adaptive_top + cell * selected.len() as f64 + adaptive_bottom
    } else {
        90.0 + cell * selected.len() as f64
    };
    let mut canvas = SvgCanvas::with_theme(width, height, theme);

    if theme.is_adaptive() {
        canvas.margin.left = adaptive_left.min(width * 0.34);
        canvas.margin.right = adaptive_right.min(width * 0.36);
        canvas.margin.top = adaptive_top.min(height * 0.38);
        canvas.margin.bottom = adaptive_bottom.min(height * 0.15);
    }

    let left = if theme.is_adaptive() {
        canvas.margin.left
    } else {
        130.0
    };
    let top = if theme.is_adaptive() {
        canvas.margin.top
    } else {
        60.0
    };
    let cell_x = if theme.is_adaptive() {
        canvas.plot_width() / cluster_order.len().max(1) as f64
    } else {
        cell
    };
    let cell_y = if theme.is_adaptive() {
        canvas.plot_height() / selected.len().max(1) as f64
    } else {
        cell
    };
    let radius_max = if theme.is_adaptive() {
        // Sparse panels can have very large cells. Dot area still represents
        // detection within each cell, but the maximum mark must not expand to
        // fill it: that overwhelms labels and makes the size key collide.
        (cell_x.min(cell_y) * 0.40).min(12.0)
    } else {
        cell * 0.42
    };

    if theme.is_adaptive() {
        canvas.add_rect(
            left,
            top,
            canvas.plot_width(),
            canvas.plot_height(),
            theme.panel_colour,
        );
        for column in 0..=cluster_order.len() {
            let x = left + cell_x * column as f64;
            canvas.add_line(
                x,
                top,
                x,
                top + canvas.plot_height(),
                theme.grid_colour,
                theme.grid_width,
            );
        }
        for row in 0..=selected.len() {
            let y = top + cell_y * row as f64;
            canvas.add_line(
                left,
                y,
                left + canvas.plot_width(),
                y,
                theme.grid_colour,
                theme.grid_width,
            );
        }
    }

    for (column, cluster) in cluster_order.iter().enumerate() {
        let x = left + cell_x * (column as f64 + 0.5);
        canvas.add_text_rotated(
            x,
            top - 10.0,
            cluster,
            -45.0,
            "start",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
    }

    for (row, &gene) in selected.iter().enumerate() {
        let y = top + cell_y * (row as f64 + 0.5);
        canvas.add_text(
            left - 8.0,
            y + 3.0,
            &gene_names[gene],
            "end",
            if theme.is_adaptive() {
                theme.tick_size
            } else {
                9.0
            },
        );
        for column in 0..cluster_order.len() {
            let x = left + cell_x * (column as f64 + 0.5);
            let fraction = detected[row][column];
            if fraction <= 0.0 {
                continue;
            }
            // Area, not radius, tracks the fraction: a radius-linear dot at 50%
            // reads as a quarter of the ink, which under-sells every mid-range
            // gene on the plot.
            let radius = radius_max * fraction.sqrt();
            let t = (scaled[row][column] + CLIP) / (2.0 * CLIP);
            let colour = if publication_theme {
                publication_diverging_color(t)
            } else {
                sequential_color(t)
            };
            canvas.add_circle(x, y, radius, &colour);
        }
    }

    // Two legends, because the figure carries two encodings and a reader cannot
    // guess either.
    let legend_x =
        left + if theme.is_adaptive() {
            canvas.plot_width()
        } else {
            cell * cluster_order.len() as f64
        } + if theme.is_adaptive() { 16.0 } else { 24.0 };
    let legend_size = if theme.is_adaptive() {
        theme.legend_size
    } else {
        9.0
    };
    canvas.add_text(legend_x, top + 4.0, "% detected", "start", legend_size);
    for (i, fraction) in [0.25_f64, 0.5, 1.0].iter().enumerate() {
        let y = top + 22.0 + i as f64 * if theme.is_adaptive() { 25.0 } else { 18.0 };
        canvas.add_circle(legend_x + 8.0, y, radius_max * fraction.sqrt(), "#888888");
        canvas.add_text(
            legend_x + 22.0,
            y + 3.0,
            &format!("{:.0}%", fraction * 100.0),
            "start",
            8.0,
        );
    }
    let bar_top = top + if theme.is_adaptive() { 112.0 } else { 90.0 };
    canvas.add_text(legend_x, bar_top - 6.0, "z-score", "start", legend_size);
    for step in 0..24 {
        let t = 1.0 - step as f64 / 23.0;
        let colour = if publication_theme {
            publication_diverging_color(t)
        } else {
            sequential_color(t)
        };
        canvas.add_rect(legend_x, bar_top + step as f64 * 3.0, 10.0, 3.4, &colour);
    }
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 6.0,
        &format!("{CLIP:.1}"),
        "start",
        8.0,
    );
    canvas.add_text(
        legend_x + 14.0,
        bar_top + 72.0,
        &format!("{:.1}", -CLIP),
        "start",
        8.0,
    );

    canvas.set_accessible_description(format!(
        "Single-cell dot plot for {} genes across {} clusters. Dot area encodes the percentage of detected cells and colour encodes per-gene z-scored mean expression.",
        selected.len(),
        cluster_order.len()
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
