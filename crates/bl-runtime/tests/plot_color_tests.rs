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
