//! UMAP and FeaturePlot expose their supplied geometry as a replayable plot spec.
//!
//! The embedding is not recomputed here: source rows, coordinates, groups,
//! feature values and publication draw order must survive the round trip.

use bl_core::value::{Table, Value};
use bl_runtime::bio_plots::call_bio_plots_builtin;
use bl_runtime::plot::call_plot_builtin;
use std::collections::HashMap;

fn points(count: usize, feature: bool) -> Value {
    let rows = (0..count)
        .map(|index| {
            let mut row = HashMap::from([
                ("x".into(), Value::Float(index as f64 * 0.25)),
                ("y".into(), Value::Float((index % 7) as f64 - 3.0)),
                ("cluster".into(), Value::Str(format!("c{}", index % 3))),
                ("cell".into(), Value::Str(format!("cell-{index}"))),
            ]);
            if feature {
                row.insert(
                    "LYZ".into(),
                    Value::Float(((count - index) % count.max(1)) as f64),
                );
            }
            Value::Record(row.into())
        })
        .collect::<Vec<_>>();
    Value::List(rows.into())
}

fn opts(pairs: Vec<(&str, Value)>) -> Value {
    Value::Record(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn spec(points: Value, extra: Vec<(&str, Value)>) -> Value {
    let mut pairs = extra;
    pairs.push(("format", Value::Str("spec".into())));
    call_bio_plots_builtin("umap_plot", vec![points, opts(pairs)]).expect("embedding spec")
}

fn record(value: &Value) -> &HashMap<String, Value> {
    let Value::Record(record) = value else {
        panic!("expected Record, got {}", value.type_of())
    };
    record
}

#[test]
fn umap_spec_is_versioned_and_keeps_every_source_coordinate() {
    let value = spec(
        points(12, false),
        vec![
            ("title", Value::Str("PBMC UMAP".into())),
            ("color", Value::Str("cluster".into())),
            ("label_col", Value::Str("cell".into())),
        ],
    );
    let map = record(&value);
    assert!(
        matches!(map.get("schema"), Some(Value::Str(schema)) if schema == "biolang.plot.spec/v1")
    );
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "embedding"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("spec data is not a Table")
    };
    assert_eq!(
        data.columns,
        [
            "source_row",
            "draw_rank",
            "x",
            "y",
            "group",
            "label",
            "feature"
        ]
    );
    assert_eq!(data.num_rows(), 12);
    assert_eq!(data.rows[7][0], Value::Int(7));
    assert_eq!(data.rows[7][2], Value::Float(1.75));
    assert_eq!(data.rows[7][3], Value::Float(-3.0));
    assert_eq!(data.rows[7][4], Value::Str("c1".into()));
    assert_eq!(data.rows[7][5], Value::Str("cell-7".into()));
    assert_eq!(data.rows[7][6], Value::Nil);
}

#[test]
fn feature_spec_freezes_resolved_cutoffs_and_publication_draw_order() {
    let value = spec(
        points(8, true),
        vec![
            ("theme", Value::Str("publication".into())),
            ("feature", Value::Str("LYZ".into())),
            ("feature_label", Value::Str("LYZ expression".into())),
            ("min_cutoff", Value::Str("q25".into())),
            ("max_cutoff", Value::Str("q75".into())),
        ],
    );
    let map = record(&value);
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("spec options are not a Record")
    };
    assert_eq!(options.get("min_cutoff"), Some(&Value::Float(1.75)));
    assert_eq!(options.get("max_cutoff"), Some(&Value::Float(5.25)));
    assert_eq!(
        options.get("feature_label"),
        Some(&Value::Str("LYZ expression".into()))
    );
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("spec data is not a Table")
    };
    // Values are 0 for source row 0, then 7..1. Publication draws low first.
    let ranks = data
        .rows
        .iter()
        .map(|row| row[1].as_int().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(ranks, vec![0, 7, 6, 5, 4, 3, 2, 1]);
}

#[test]
fn render_plot_replays_umap_and_feature_specs_exactly() {
    for with_feature in [false, true] {
        let mut common = vec![
            ("theme", Value::Str("publication".into())),
            ("title", Value::Str("PBMC embedding".into())),
            ("subtitle", Value::Str("same geometry".into())),
            ("caption", Value::Str("replay fixture".into())),
            ("color", Value::Str("cluster".into())),
            ("point_size", Value::Float(2.5)),
            ("width", Value::Int(680)),
            ("height", Value::Int(480)),
            ("raster", Value::Bool(false)),
        ];
        if with_feature {
            common.extend([
                ("feature", Value::Str("LYZ".into())),
                ("min_cutoff", Value::Str("q10".into())),
                ("max_cutoff", Value::Str("q90".into())),
            ]);
        }
        let direct = call_bio_plots_builtin(
            if with_feature {
                "feature_plot"
            } else {
                "umap_plot"
            },
            vec![points(40, with_feature), opts(common.clone())],
        )
        .unwrap();
        let specification = spec(points(40, with_feature), common);
        let replay = call_plot_builtin("render_plot", vec![specification]).unwrap();
        assert_eq!(direct, replay, "embedding spec replay changed the SVG");
    }
}

#[test]
fn embedding_spec_can_render_standalone_html_canvas() {
    let specification = spec(points(20, false), vec![]);
    let rendered = call_plot_builtin(
        "render_plot",
        vec![
            specification,
            opts(vec![("format", Value::Str("html".into()))]),
        ],
    )
    .unwrap();
    let Value::Str(html) = rendered else {
        panic!("expected HTML string")
    };
    assert!(html.contains("<canvas"));
    assert!(html.contains("<svg"));
    assert!(html.contains("Use canvas"));
}

#[test]
fn dense_embedding_spec_records_and_replays_the_raster_decision() {
    let specification = spec(points(20_000, false), vec![]);
    let map = record(&specification);
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("spec options are not a Record")
    };
    assert_eq!(options.get("raster"), Some(&Value::Bool(true)));
    let Value::Str(svg) = call_plot_builtin("render_plot", vec![specification]).unwrap() else {
        panic!("expected SVG")
    };
    assert!(svg.contains("<image"));
    // Group legend swatches remain vector; the 20,000 data marks do not.
    assert!(svg.matches("<circle").count() <= 3);
    assert!(svg.len() < 400_000);
}

#[test]
fn spec_discloses_non_finite_coordinates_and_rejects_broken_data() {
    let data = Value::Table(Table::new(
        vec!["x".into(), "y".into(), "cluster".into()],
        vec![
            vec![Value::Float(0.0), Value::Float(1.0), Value::Str("a".into())],
            vec![
                Value::Float(f64::NAN),
                Value::Float(2.0),
                Value::Str("b".into()),
            ],
        ],
    ));
    let specification = spec(data, vec![]);
    let map = record(&specification);
    let Some(Value::Record(provenance)) = map.get("provenance") else {
        panic!("missing provenance")
    };
    assert_eq!(
        provenance.get("non_finite_coordinates"),
        Some(&Value::Int(1))
    );
    assert_eq!(provenance.get("rendered_points"), Some(&Value::Int(1)));
    assert!(matches!(map.get("warnings"), Some(Value::List(items)) if items.len() == 1));
    let Value::Str(svg) = call_plot_builtin("render_plot", vec![specification.clone()]).unwrap()
    else {
        panic!("expected SVG")
    };
    assert!(!svg.contains("NaN"));
    assert!(!svg.contains("inf"));

    let mut broken = record(&specification).clone();
    broken.insert("data".into(), Value::Table(Table::empty()));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken.into())]).is_err());
}
