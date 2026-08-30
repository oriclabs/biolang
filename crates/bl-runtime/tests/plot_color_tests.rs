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

#[test]
fn seurat_theme_uses_r_familiar_discrete_colours() {
    let svg = render(
        points_as_records(4),
        vec![("color", "cluster"), ("theme", "seurat")],
    );
    assert!(svg.contains("#f8766d"), "first ggplot-like hue is absent");
    assert!(svg.contains("#7cae00"), "second ggplot-like hue is absent");
}

#[test]
fn publication_theme_adds_adaptive_presentation_without_changing_the_default() {
    let legacy = render(points_as_records(4), vec![("color", "cluster")]);
    let publication = render(
        points_as_records(4),
        vec![("color", "cluster"), ("theme", "publication")],
    );
    assert!(legacy.contains("data-biolang-theme=\"biolang\""));
    assert!(publication.contains("data-biolang-theme=\"publication\""));
    assert!(publication.contains("Arial, Helvetica, sans-serif"));
    assert!(
        publication.contains("#e5e7eb"),
        "publication grid is absent"
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
fn seurat_feature_theme_runs_from_light_grey_to_blue() {
    let svg = render(
        points_with_feature(40),
        vec![("feature", "LYZ"), ("theme", "seurat")],
    );
    assert!(svg.contains("#d3d3d3"), "low-expression grey is absent");
    assert!(svg.contains("#0000ff"), "high-expression blue is absent");
}

#[test]
fn publication_feature_plot_uses_a_perceptually_ordered_ramp() {
    let svg = render(
        points_with_feature(40),
        vec![("feature", "LYZ"), ("theme", "publication")],
    );
    assert!(
        svg.contains("#440154"),
        "low end of publication ramp is absent"
    );
    assert!(
        svg.contains("#fde725"),
        "high end of publication ramp is absent"
    );
}

#[test]
fn publication_feature_plot_accepts_quantile_cutoffs() {
    let mut map = HashMap::new();
    map.insert("feature".to_string(), Value::Str("LYZ".to_string()));
    map.insert("theme".to_string(), Value::Str("publication".to_string()));
    map.insert("min_cutoff".to_string(), Value::Str("q25".to_string()));
    map.insert("max_cutoff".to_string(), Value::Str("q75".to_string()));
    let svg = match call_bio_plots_builtin(
        "feature_plot",
        vec![points_with_feature(40), Value::Record(map.into())],
    ) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("feature_plot returned {other:?}"),
    };
    // Values are 0, .15, ... 5.85, so type-7 q25/q75 are 1.46 and 4.39.
    assert!(
        svg.contains(">1.46<"),
        "lower quantile is not on the colour key"
    );
    assert!(
        svg.contains(">4.39<"),
        "upper quantile is not on the colour key"
    );
}

#[test]
fn publication_plot_renders_subtitle_and_caption() {
    let mut map = HashMap::new();
    map.insert("theme".to_string(), Value::Str("publication".to_string()));
    map.insert(
        "subtitle".to_string(),
        Value::Str("PBMC subset".to_string()),
    );
    map.insert(
        "caption".to_string(),
        Value::Str("BioLang analysis".to_string()),
    );
    let svg = match call_bio_plots_builtin(
        "umap_plot",
        vec![points_as_records(3), Value::Record(map.into())],
    ) {
        Ok(Value::Str(svg)) => svg,
        other => panic!("umap_plot returned {other:?}"),
    };
    assert!(svg.contains(">PBMC subset<"));
    assert!(svg.contains(">BioLang analysis<"));
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
    violin_with_options(rows, Vec::new())
}

fn violin_with_options(rows: Vec<(&str, f64)>, opts: Vec<(&str, Value)>) -> String {
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(group, value)| {
            let mut record = HashMap::new();
            record.insert("group".to_string(), Value::Str(group.to_string()));
            record.insert("value".to_string(), Value::Float(value));
            Value::Record(record.into())
        })
        .collect();
    let mut args = vec![Value::List(items.into())];
    if !opts.is_empty() {
        args.push(Value::Record(
            opts.into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect::<HashMap<_, _>>()
                .into(),
        ));
    }
    match call_bio_plots_builtin("violin_plot", args) {
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
    // SVG coordinates are rounded to a tenth of a pixel. At the 512-point
    // grid used by ggplot2-compatible violins that can turn one smooth summit
    // into several one-pixel teeth. Smooth only for this visual-shape test;
    // the renderer keeps the full density grid.
    let radius = 6usize;
    let smoothed: Vec<f64> = (0..widths.len())
        .map(|i| {
            let start = i.saturating_sub(radius);
            let end = (i + radius + 1).min(widths.len());
            widths[start..end].iter().sum::<f64>() / (end - start) as f64
        })
        .collect();
    let tallest = smoothed.iter().cloned().fold(f64::MIN, f64::max);
    (1..smoothed.len().saturating_sub(1))
        .filter(|&i| {
            smoothed[i] > smoothed[i - 1]
                && smoothed[i] >= smoothed[i + 1]
                && smoothed[i] > 0.4 * tallest
        })
        .count()
}

#[test]
fn a_violin_shows_bimodality_a_boxplot_would_hide() {
    let mut rows: Vec<(&str, f64)> = Vec::new();
    // Unimodal: counts fall away from a single centre. Cycling i % 20 would
    // spread the sample evenly instead, and the density of an even spread is a
    // plateau -- flat to within a part in 10,000, so which point on top counts
    // as the maximum comes down to how the polygon coordinates round.
    for offset in -10..=10i32 {
        for _ in 0..(11 - offset.abs()) * 2 {
            rows.push(("uni", 10.0 + f64::from(offset) * 0.1));
        }
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
fn ggplot_violin_uses_r_group_fills() {
    let rows: Vec<(&str, f64)> = (0..30)
        .map(|i| (["BRCA", "OV", "UCEC"][i % 3], i as f64))
        .collect();
    let svg = violin_with_options(rows, vec![("theme", Value::Str("ggplot".into()))]);
    for colour in ["#f8766d", "#00ba38", "#619cff"] {
        assert!(
            svg.contains(&format!("fill=\"{colour}\"")),
            "ggplot violin should contain R hue {colour}"
        );
    }
    assert!(
        svg.contains(">group</text>"),
        "mapped group fills should include a discrete legend"
    );
}

#[test]
fn violin_plot_rejects_input_it_cannot_read() {
    assert!(call_bio_plots_builtin("violin_plot", vec![Value::Int(1)]).is_err());
}

#[test]
fn publication_violin_survives_notebook_and_journal_widths() {
    let rows = vec![
        ("untreated sample", 1.0),
        ("untreated sample", 1.2),
        ("untreated sample", 1.4),
        ("treated sample", 2.1),
        ("treated sample", 2.4),
        ("treated sample", 2.8),
    ];
    for width in [321_i64, 680, 800] {
        let svg = violin_with_options(
            rows.clone(),
            vec![
                ("theme", Value::Str("publication".to_string())),
                ("width", Value::Int(width)),
                ("height", Value::Int(360)),
                ("title", Value::Str("Expression distribution".to_string())),
                ("subtitle", Value::Str("Two study groups".to_string())),
                ("caption", Value::Str("Median shown by a line".to_string())),
            ],
        );
        assert!(svg.contains(&format!("width=\"{width}\"")));
        assert!(svg.contains("data-biolang-theme=\"publication\""));
        assert!(svg.contains(">untreated sample<"));
        assert!(svg.contains(">treated sample<"));
        assert!(svg.contains(">Two study groups<"));
        assert!(svg.contains(">Median shown by a line<"));
        assert!(svg.contains("#e5e7eb"), "horizontal guides are absent");
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }
}

#[test]
fn publication_theme_does_not_change_violin_density_sampling() {
    let rows: Vec<(&str, f64)> = (0..40).map(|i| ("sample", (i % 11) as f64)).collect();
    let legacy = violin(rows.clone());
    let publication =
        violin_with_options(rows, vec![("theme", Value::Str("publication".to_string()))]);
    let points = |svg: &str| {
        svg.split("<polygon points=\"")
            .nth(1)
            .and_then(|part| {
                part.find('"')
                    .map(|end| part[..end].split_whitespace().count())
            })
            .expect("violin polygon")
    };
    assert_eq!(points(&legacy), points(&publication));
    assert_eq!(
        points(&legacy),
        1024,
        "the 512-point KDE should be mirrored"
    );
}
