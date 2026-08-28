//! PCA, volcano and MA plots expose resolved scientific geometry for replay.

use bl_core::value::{Table, Value};
use bl_runtime::bio_plots::call_bio_plots_builtin;
use bl_runtime::plot::call_plot_builtin;
use std::collections::HashMap;

fn opts(pairs: Vec<(&str, Value)>) -> Value {
    Value::Record(
        pairs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn record(value: &Value) -> &HashMap<String, Value> {
    let Value::Record(record) = value else {
        panic!("expected Record, got {}", value.type_of())
    };
    record
}

fn pca_input() -> Value {
    Value::Table(Table::new(
        vec![
            "sample".into(),
            "group".into(),
            "g1".into(),
            "g2".into(),
            "g3".into(),
        ],
        vec![
            vec![
                Value::Str("s1".into()),
                Value::Str("control".into()),
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(3.0),
            ],
            vec![
                Value::Str("s2".into()),
                Value::Str("control".into()),
                Value::Float(1.2),
                Value::Float(1.8),
                Value::Float(3.2),
            ],
            vec![
                Value::Str("s3".into()),
                Value::Str("treated".into()),
                Value::Float(5.0),
                Value::Float(4.0),
                Value::Float(1.0),
            ],
            vec![
                Value::Str("s4".into()),
                Value::Str("treated".into()),
                Value::Float(5.3),
                Value::Float(3.7),
                Value::Float(0.8),
            ],
        ],
    ))
}

fn differential_input(count: usize) -> Value {
    Value::Table(Table::new(
        vec!["gene".into(), "mean".into(), "effect".into(), "q".into()],
        (0..count)
            .map(|index| {
                vec![
                    Value::Str(format!("gene-{index}")),
                    Value::Float(index as f64 + 1.0),
                    Value::Float((index % 9) as f64 - 4.0),
                    Value::Float(10f64.powf(-((index % 8) as f64 + 1.0))),
                ]
            })
            .collect(),
    ))
}

#[test]
fn pca_spec_keeps_scores_groups_labels_and_variance() {
    let specification = call_bio_plots_builtin(
        "pca_plot",
        vec![
            pca_input(),
            opts(vec![
                ("format", Value::Str("spec".into())),
                ("group_col", Value::Str("group".into())),
                ("labels", Value::Bool(true)),
            ]),
        ],
    )
    .unwrap();
    let map = record(&specification);
    assert!(
        matches!(map.get("schema"), Some(Value::Str(value)) if value == "biolang.plot.spec/v1")
    );
    assert!(matches!(map.get("kind"), Some(Value::Str(value)) if value == "pca"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("PCA data is not a Table")
    };
    assert_eq!(data.columns, ["source_row", "pc1", "pc2", "group", "label"]);
    assert_eq!(data.num_rows(), 4);
    assert!(data
        .rows
        .iter()
        .all(|row| row[1].as_float().is_some_and(f64::is_finite)
            && row[2].as_float().is_some_and(f64::is_finite)));
    assert_eq!(data.rows[2][3], Value::Str("treated".into()));
    assert_eq!(data.rows[2][4], Value::Str("s3".into()));
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("PCA options are not a Record")
    };
    assert!(options
        .get("pc1_variance_percent")
        .and_then(Value::as_float)
        .is_some_and(|value| value > 0.0));
    assert!(options
        .get("pc2_variance_percent")
        .and_then(Value::as_float)
        .is_some());
    assert_eq!(options.get("has_groups"), Some(&Value::Bool(true)));
    assert_eq!(options.get("has_labels"), Some(&Value::Bool(true)));
}

#[test]
fn pca_rejects_an_empty_labelled_table_without_panicking() {
    assert!(call_bio_plots_builtin(
        "pca_plot",
        vec![
            Value::Table(Table::empty()),
            opts(vec![("labels", Value::Bool(true))]),
        ],
    )
    .is_err());
}

#[test]
fn direct_and_replayed_pca_are_byte_identical_and_html_has_canvas() {
    let common = vec![
        ("group_col", Value::Str("group".into())),
        ("labels", Value::Bool(true)),
        ("title", Value::Str("Treatment PCA".into())),
        ("width", Value::Int(680)),
        ("height", Value::Int(480)),
        ("raster", Value::Bool(false)),
    ];
    let direct =
        call_bio_plots_builtin("pca_plot", vec![pca_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("pca_plot", vec![pca_input(), opts(spec_options)]).unwrap();
    let replay = call_plot_builtin("render_plot", vec![specification.clone()]).unwrap();
    assert_eq!(direct, replay);
    let Value::Str(html) = call_plot_builtin(
        "render_plot",
        vec![
            specification,
            opts(vec![("format", Value::Str("html".into()))]),
        ],
    )
    .unwrap() else {
        panic!("expected HTML")
    };
    assert!(html.contains("<svg"));
    assert!(html.contains("<canvas"));
}

#[test]
fn volcano_spec_freezes_threshold_classification_and_replays_exactly() {
    let common = vec![
        ("fc", Value::Str("effect".into())),
        ("p", Value::Str("q".into())),
        ("fc_threshold", Value::Float(2.0)),
        ("p_threshold", Value::Float(0.05)),
        ("raster", Value::Bool(false)),
    ];
    let direct = call_plot_builtin(
        "volcano",
        vec![differential_input(18), opts(common.clone())],
    )
    .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_plot_builtin("volcano", vec![differential_input(18), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(
        matches!(map.get("kind"), Some(Value::Str(value)) if value == "differential_expression")
    );
    assert!(matches!(map.get("plot"), Some(Value::Str(value)) if value == "volcano"));
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("volcano options are not a Record")
    };
    assert_eq!(
        options.get("fold_change_threshold"),
        Some(&Value::Float(2.0))
    );
    assert_eq!(options.get("p_value_threshold"), Some(&Value::Float(0.05)));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("volcano data is not a Table")
    };
    assert_eq!(data.rows[0][5], Value::Str("gene-0".into()));
    let statuses = data
        .rows
        .iter()
        .map(|row| row[6].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(statuses.contains(&"up"));
    assert!(statuses.contains(&"down"));
    assert!(statuses.contains(&"not_significant"));
    assert_eq!(
        direct,
        call_plot_builtin("render_plot", vec![specification]).unwrap()
    );
}

#[test]
fn ma_spec_keeps_raw_and_log_coordinates_and_replays_exactly() {
    let common = vec![
        ("a", Value::Str("mean".into())),
        ("m", Value::Str("effect".into())),
        ("raster", Value::Bool(false)),
    ];
    let direct = call_plot_builtin(
        "ma_plot",
        vec![differential_input(12), opts(common.clone())],
    )
    .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_plot_builtin("ma_plot", vec![differential_input(12), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("plot"), Some(Value::Str(value)) if value == "ma"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("MA data is not a Table")
    };
    assert_eq!(data.rows[3][1], Value::Float(4.0));
    assert_eq!(data.rows[3][3], Value::Float(2.0));
    assert_eq!(data.rows[3][4], Value::Float(-1.0));
    assert_eq!(data.rows[3][6], Value::Str("not_changed".into()));
    assert_eq!(
        direct,
        call_plot_builtin("render_plot", vec![specification]).unwrap()
    );
}

#[test]
fn dense_differential_specs_replay_with_a_bounded_raster_layer() {
    for builtin in ["volcano", "ma_plot"] {
        let mut options = vec![("format", Value::Str("spec".into()))];
        if builtin == "volcano" {
            options.extend([
                ("fc", Value::Str("effect".into())),
                ("p", Value::Str("q".into())),
            ]);
        } else {
            options.extend([
                ("a", Value::Str("mean".into())),
                ("m", Value::Str("effect".into())),
            ]);
        }
        let specification =
            call_plot_builtin(builtin, vec![differential_input(20_000), opts(options)]).unwrap();
        let Value::Str(svg) = call_plot_builtin("render_plot", vec![specification]).unwrap() else {
            panic!("expected SVG")
        };
        assert!(
            svg.contains("<image"),
            "{builtin} did not retain raster marks"
        );
        assert!(svg.matches("<circle").count() < 10);
        assert!(svg.len() < 400_000);
    }
}

#[test]
fn malformed_differential_spec_is_rejected() {
    let specification = call_plot_builtin(
        "volcano",
        vec![
            differential_input(4),
            opts(vec![
                ("format", Value::Str("spec".into())),
                ("fc", Value::Str("effect".into())),
                ("p", Value::Str("q".into())),
            ]),
        ],
    )
    .unwrap();
    let mut broken = record(&specification).clone();
    broken.insert("data".into(), Value::Table(Table::empty()));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken.into())]).is_err());

    let pca = call_bio_plots_builtin(
        "pca_plot",
        vec![
            pca_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let mut broken_pca = record(&pca).clone();
    broken_pca.insert("data".into(), Value::Table(Table::empty()));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken_pca.into())]).is_err());
}

#[test]
fn differential_spec_discloses_and_omits_non_finite_geometry() {
    let input = Value::Table(Table::new(
        vec!["gene".into(), "effect".into(), "q".into()],
        vec![
            vec![
                Value::Str("finite".into()),
                Value::Float(2.0),
                Value::Float(0.001),
            ],
            vec![
                Value::Str("invalid".into()),
                Value::Float(f64::INFINITY),
                Value::Float(0.01),
            ],
        ],
    ));
    let specification = call_plot_builtin(
        "volcano",
        vec![
            input,
            opts(vec![
                ("format", Value::Str("spec".into())),
                ("fc", Value::Str("effect".into())),
                ("p", Value::Str("q".into())),
            ]),
        ],
    )
    .unwrap();
    let map = record(&specification);
    let Some(Value::Record(provenance)) = map.get("provenance") else {
        panic!("missing provenance")
    };
    assert_eq!(provenance.get("rendered_points"), Some(&Value::Int(1)));
    assert_eq!(
        provenance.get("non_finite_coordinates"),
        Some(&Value::Int(1))
    );
    assert!(matches!(map.get("warnings"), Some(Value::List(items)) if items.len() == 1));
    let Value::Str(svg) = call_plot_builtin("render_plot", vec![specification]).unwrap() else {
        panic!("expected SVG")
    };
    assert!(!svg.contains("NaN"));
    assert!(!svg.contains("inf"));
}
