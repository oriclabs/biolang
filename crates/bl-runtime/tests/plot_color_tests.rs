//! Cluster colouring must work however the points are handed over.
//!
//! umap_plot auto-detected a cluster column for Value::Table only. The same
//! data as a List of Records - which its extraction handles perfectly well -
//! came back with every point one colour and no error. On PBMC3k that was one
//! colour for eleven clusters.
//!
//! It also accepted the option only as `color_col`, so `{ color: "cluster" }`
//! was silently ignored, and the sibling pca_plot spells it `group_col`.

use bl_core::value::Value;
use bl_runtime::bio_plots::call_bio_plots_builtin;
use std::collections::{HashMap, HashSet};

fn points_as_records(n_clusters: i64) -> Value {
    let mut rows = Vec::new();
    for cluster in 0..n_clusters {
        for k in 0..4 {
            let mut record = HashMap::new();
            record.insert(
                "x".to_string(),
                Value::Float(cluster as f64 * 2.0 + k as f64 * 0.2),
            );
            record.insert(
                "y".to_string(),
                Value::Float((cluster % 4) as f64 + k as f64 * 0.2),
            );
            record.insert("cluster".to_string(), Value::Int(cluster));
            rows.push(Value::Record(record.into()));
        }
    }
    Value::List(rows.into())
}

fn distinct_fills(svg: &str) -> HashSet<String> {
    svg.split("fill=\"")
        .skip(1)
        .filter_map(|part| part.find('"').map(|end| part[..end].to_string()))
        .filter(|value| value.starts_with('#'))
        .collect()
}

fn render(points: Value, opts: Vec<(&str, &str)>) -> String {
    let mut map = HashMap::new();
    for (key, value) in opts {
        map.insert(key.to_string(), Value::Str(value.to_string()));
    }
    match call_bio_plots_builtin("umap_plot", vec![points, Value::Record(map.into())]) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("expected an SVG string, got {other:?}"),
    }
}

#[test]
fn a_list_of_records_gets_its_clusters_coloured() {
    // No colour option at all: the cluster column must be found by name, as it
    // already was for a Table.
    let svg = render(points_as_records(9), vec![("title", "auto")]);
    assert!(
        distinct_fills(&svg).len() >= 9,
        "nine clusters rendered with {} colours",
        distinct_fills(&svg).len()
    );
}

#[test]
fn the_color_option_is_honoured_under_either_name() {
    for key in ["color", "color_col"] {
        let svg = render(points_as_records(6), vec![(key, "cluster")]);
        assert!(
            distinct_fills(&svg).len() >= 6,
            "`{key}` gave {} colours for six clusters",
            distinct_fills(&svg).len()
        );
    }
}

#[test]
fn more_clusters_than_the_old_palette_still_get_distinct_colours() {
    let svg = render(points_as_records(14), vec![("color", "cluster")]);
    let fills = distinct_fills(&svg);
    assert!(
        fills.len() >= 14,
        "fourteen clusters rendered with {} colours: {fills:?}",
        fills.len()
    );
}

// ── continuous colouring (feature_plot) ─────────────────────────────────────
//
// Seurat's FeaturePlot: colour the embedding by one gene's expression. This is
// how marker-based annotation is taught, and BioLang had no equivalent.

fn points_with_feature(n: i64) -> Value {
    let mut rows = Vec::new();
    for i in 0..n {
        let mut record = HashMap::new();
        record.insert("x".to_string(), Value::Float(i as f64));
        record.insert("y".to_string(), Value::Float((i % 5) as f64));
        record.insert("cluster".to_string(), Value::Int(i % 3));
        // A gradient, so a continuous scale has something to show.
        record.insert("LYZ".to_string(), Value::Float(i as f64 / n as f64 * 6.0));
        rows.push(Value::Record(record.into()));
    }
    Value::List(rows.into())
}

#[test]
fn a_feature_gets_a_continuous_scale_not_palette_colours() {
    let svg = render(points_with_feature(60), vec![("feature", "LYZ")]);
    let fills = distinct_fills(&svg);
    // A categorical palette tops out at 24; a gradient over 60 points gives far
    // more, and none of them need be palette entries.
    assert!(
        fills.len() > 24,
        "expected a continuous gradient, got {} colours",
        fills.len()
    );
}

#[test]
fn the_feature_scale_is_labelled() {
    let svg = render(points_with_feature(40), vec![("feature", "LYZ")]);
    assert!(svg.contains("LYZ"), "colour bar carries no feature name");
    assert!(
        svg.matches("<rect").count() > 10,
        "no colour bar drawn: a reader cannot tell high from low"
    );
}

#[test]
fn a_feature_overrides_cluster_colouring() {
    // Both a cluster column and a feature are present. The feature wins, and
    // the two legends must not both be drawn.
    let svg = render(
        points_with_feature(40),
        vec![("feature", "LYZ"), ("color", "cluster")],
    );
    assert!(
        distinct_fills(&svg).len() > 24,
        "cluster colouring took precedence over the requested feature"
    );
}

#[test]
fn feature_plot_is_the_same_renderer_as_umap_plot() {
    let by_alias = match call_bio_plots_builtin(
        "feature_plot",
        vec![points_with_feature(30), {
            let mut m = HashMap::new();
            m.insert("feature".to_string(), Value::Str("LYZ".to_string()));
            Value::Record(m.into())
        }],
    ) {
        Ok(Value::Str(s)) => s,
        other => panic!("feature_plot returned {other:?}"),
    };
    let by_option = render(points_with_feature(30), vec![("feature", "LYZ")]);
    assert_eq!(
        by_alias, by_option,
        "feature_plot and umap_plot(feature:) must not drift apart"
    );
}

// ── elbow / scree plot ──────────────────────────────────────────────────────

fn elbow(arg: Value) -> String {
    match call_bio_plots_builtin("elbow_plot", vec![arg]) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("elbow_plot returned {other:?}"),
    }
}

fn ratios() -> Vec<f64> {
    vec![0.178, 0.069, 0.049, 0.018, 0.013, 0.009]
}

fn ratio_list() -> Value {
    Value::List(
        ratios()
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    )
}

#[test]
fn elbow_plot_draws_one_point_per_component() {
    let svg = elbow(ratio_list());
    assert_eq!(
        svg.matches("<circle").count(),
        ratios().len(),
        "expected a marker per component"
    );
    assert!(
        svg.contains("<polyline"),
        "no line connecting the components"
    );
}

#[test]
fn elbow_plot_accepts_the_record_sc_pca_returns() {
    // Passing the pca result straight through is what a reader tries first.
    let mut record = HashMap::new();
    record.insert("explained_variance_ratio".to_string(), ratio_list());
    record.insert("components".to_string(), Value::Int(6));
    let svg = elbow(Value::Record(record.into()));
    assert_eq!(svg.matches("<circle").count(), ratios().len());
}

#[test]
fn elbow_plot_is_anchored_at_zero() {
    // A scree plot on a truncated axis exaggerates the elbow, which is the one
    // thing it exists to show. The y axis must reach 0.
    let svg = elbow(ratio_list());
    assert!(
        svg.contains(">0<") || svg.contains(">0.00<") || svg.contains(">0.0<"),
        "y axis does not reach zero, so the elbow is exaggerated"
    );
}

#[test]
fn elbow_plot_rejects_input_it_cannot_read() {
    assert!(call_bio_plots_builtin("elbow_plot", vec![Value::Int(3)]).is_err());
    assert!(
        call_bio_plots_builtin("elbow_plot", vec![Value::List(Vec::new().into())]).is_err(),
        "an empty list should error rather than render a blank chart"
    );
}

// ── violin plot ─────────────────────────────────────────────────────────────
//
// A boxplot draws five numbers and cannot show bimodality. A violin draws the
// density, and the test below is that it actually distinguishes the two shapes
// rather than drawing a plausible blob.

fn violin(rows: Vec<(&str, f64)>) -> String {
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(group, value)| {
            let mut record = HashMap::new();
            record.insert("group".to_string(), Value::Str(group.to_string()));
            record.insert("value".to_string(), Value::Float(value));
            Value::Record(record.into())
        })
        .collect();
    match call_bio_plots_builtin("violin_plot", vec![Value::List(items.into())]) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("violin_plot returned {other:?}"),
    }
}

/// Count bulges down one side of a violin outline: one for a unimodal
/// distribution, two for a bimodal one.
fn width_peaks(svg: &str, index: usize) -> usize {
    let polygon = svg
        .split("<polygon points=\"")
        .nth(index + 1)
        .and_then(|part| part.find('"').map(|end| &part[..end]))
        .expect("polygon");
    let xs: Vec<f64> = polygon
        .split_whitespace()
        .filter_map(|t| t.split(',').next().and_then(|v| v.parse::<f64>().ok()))
        .collect();
    let right = &xs[..xs.len() / 2];
    let base = right.iter().cloned().fold(f64::MAX, f64::min);
    let widths: Vec<f64> = right.iter().map(|x| x - base).collect();
    let tallest = widths.iter().cloned().fold(f64::MIN, f64::max);
    (1..widths.len().saturating_sub(1))
        .filter(|&i| {
            widths[i] > widths[i - 1] && widths[i] >= widths[i + 1] && widths[i] > 0.4 * tallest
        })
        .count()
}

#[test]
fn a_violin_shows_bimodality_a_boxplot_would_hide() {
    let mut rows: Vec<(&str, f64)> = Vec::new();
    // Unimodal: a single tight cluster.
    for i in 0..200 {
        rows.push(("uni", 10.0 + ((i % 20) as f64 - 10.0) * 0.15));
    }
    // Bimodal: two clusters far apart.
    for i in 0..100 {
        rows.push(("bi", 6.0 + ((i % 10) as f64 - 5.0) * 0.1));
        rows.push(("bi", 13.0 + ((i % 10) as f64 - 5.0) * 0.1));
    }
    let svg = violin(rows);
    assert_eq!(svg.matches("<polygon").count(), 2, "expected two violins");
    assert_eq!(
        width_peaks(&svg, 0),
        1,
        "unimodal group drew more than one bulge"
    );
    assert_eq!(
        width_peaks(&svg, 1),
        2,
        "bimodal group did not draw two bulges"
    );
}

#[test]
fn each_violin_carries_a_median_line() {
    let rows: Vec<(&str, f64)> = (0..40).map(|i| ("a", i as f64)).collect();
    let svg = violin(rows);
    assert!(svg.contains("#333333"), "no median marked");
}

#[test]
fn violin_groups_keep_a_stable_order() {
    // Hash order would reshuffle the axis between runs.
    let rows: Vec<(&str, f64)> = (0..30)
        .map(|i| (["z", "a", "m"][i % 3], i as f64))
        .collect();
    let svg = violin(rows.clone());
    let first = svg.find("z").zip(svg.find('a')).zip(svg.find('m'));
    assert!(first.is_some());
    assert_eq!(svg, violin(rows), "same input rendered differently twice");
}

#[test]
fn violin_plot_rejects_input_it_cannot_read() {
    assert!(call_bio_plots_builtin("violin_plot", vec![Value::Int(1)]).is_err());
}
