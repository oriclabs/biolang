//! Violin, dot and heatmap plots expose resolved geometry for exact replay.

use bl_core::matrix::Matrix;
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

fn render(specification: Value) -> Value {
    call_plot_builtin("render_plot", vec![specification]).expect("render plot specification")
}

fn wide_violin_input() -> Value {
    Value::Table(Table::new(
        vec!["control".into(), "treated".into()],
        vec![
            vec![Value::Float(1.0), Value::Float(4.0)],
            vec![Value::Float(2.0), Value::Float(5.0)],
            vec![Value::Float(2.5), Value::Float(7.0)],
            vec![Value::Float(3.0), Value::Float(9.0)],
        ],
    ))
}

fn long_violin_input() -> Value {
    Value::Table(Table::new(
        vec!["condition".into(), "expression".into()],
        vec![
            vec![Value::Str("control".into()), Value::Float(1.0)],
            vec![Value::Str("control".into()), Value::Float(2.0)],
            vec![Value::Str("control".into()), Value::Float(3.0)],
            vec![Value::Str("treated".into()), Value::Float(4.0)],
            vec![Value::Str("treated".into()), Value::Float(6.0)],
            vec![Value::Str("treated".into()), Value::Float(9.0)],
        ],
    ))
}

fn heatmap_input() -> Value {
    Value::Table(Table::new(
        vec!["sample-a".into(), "sample-b".into()],
        vec![
            vec![Value::Float(8.0), Value::Float(10.0)],
            vec![Value::Float(0.0), Value::Float(2.0)],
            vec![Value::Float(4.0), Value::Float(6.0)],
        ],
    ))
}

#[test]
fn wide_violin_spec_freezes_kde_bandwidth_and_median_and_replays_exactly() {
    let common = vec![
        ("title", Value::Str("Expression distributions".into())),
        ("width", Value::Int(680)),
    ];
    let direct =
        call_bio_plots_builtin("violin", vec![wide_violin_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("violin", vec![wide_violin_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "violin"));
    assert!(matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "wide"));
    let Some(Value::Table(groups)) = map.get("groups") else {
        panic!("violin summaries are not a Table")
    };
    assert_eq!(groups.num_rows(), 2);
    assert_eq!(groups.rows[0][2], Value::Int(4));
    assert_eq!(groups.rows[0][4], Value::Float(2.25));
    assert!(groups.rows[0][3]
        .as_float()
        .is_some_and(|bandwidth| bandwidth > 0.0));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("violin KDE grid is not a Table")
    };
    assert_eq!(data.num_rows(), 100);
    assert_eq!(direct, render(specification));
}

#[test]
fn long_violin_spec_preserves_first_seen_groups_and_exact_publication_replay() {
    let common = vec![
        ("value", Value::Str("expression".into())),
        ("group", Value::Str("condition".into())),
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("Marker expression".into())),
    ];
    let direct = call_bio_plots_builtin(
        "violin_plot",
        vec![long_violin_input(), opts(common.clone())],
    )
    .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("violin_plot", vec![long_violin_input(), opts(spec_options)])
            .unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "long"));
    let Some(Value::Table(groups)) = map.get("groups") else {
        panic!("violin summaries are not a Table")
    };
    assert_eq!(groups.rows[0][1], Value::Str("control".into()));
    assert_eq!(groups.rows[1][1], Value::Str("treated".into()));
    assert_eq!(groups.rows[1][4], Value::Float(6.0));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("violin KDE grid is not a Table")
    };
    assert_eq!(data.num_rows(), 256);
    assert_eq!(direct, render(specification));
}

#[test]
fn dot_plot_spec_freezes_mean_detection_and_scaled_expression() {
    let matrix = || {
        Value::Matrix(
            Matrix {
                data: vec![
                    1.0, 0.0, // A
                    0.0, 2.0, // A
                    3.0, 4.0, // A
                    0.0, 1.0, // B
                    0.0, 1.0, // B
                    6.0, 1.0, // B
                ],
                nrow: 6,
                ncol: 2,
                row_names: None,
                col_names: Some(vec!["G1".into(), "G2".into()]),
            }
            .into(),
        )
    };
    let labels = || {
        Value::List(
            ["A", "A", "A", "B", "B", "B"]
                .into_iter()
                .map(|label| Value::Str(label.into()))
                .collect::<Vec<_>>()
                .into(),
        )
    };
    let common = vec![
        (
            "genes",
            Value::List(vec![Value::Str("G1".into()), Value::Str("G2".into())].into()),
        ),
        ("theme", Value::Str("publication".into())),
    ];
    let direct =
        call_bio_plots_builtin("dot_plot", vec![matrix(), labels(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("dot_plot", vec![matrix(), labels(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "dot_plot"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("dot data is not a Table")
    };
    assert_eq!(data.num_rows(), 4);
    assert!((data.rows[0][4].as_float().unwrap() - 4.0 / 3.0).abs() < 1e-12);
    assert!((data.rows[0][5].as_float().unwrap() - 2.0 / 3.0).abs() < 1e-12);
    assert!((data.rows[1][4].as_float().unwrap() - 2.0).abs() < 1e-12);
    assert!((data.rows[1][5].as_float().unwrap() - 1.0 / 3.0).abs() < 1e-12);
    assert_eq!(direct, render(specification));
}

#[test]
fn generic_heatmap_spec_freezes_mean_sort_and_colour_domain() {
    let common = vec![
        ("cluster", Value::Bool(true)),
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("Sorted heatmap".into())),
    ];
    let direct = call_plot_builtin("heatmap", vec![heatmap_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_plot_builtin("heatmap", vec![heatmap_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "heatmap"));
    assert!(matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "heatmap"));
    let Some(Value::Table(rows)) = map.get("rows") else {
        panic!("heatmap row metadata is not a Table")
    };
    assert_eq!(rows.rows[0][1], Value::Int(1));
    assert_eq!(rows.rows[1][1], Value::Int(2));
    assert_eq!(rows.rows[2][1], Value::Int(0));
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("heatmap options are not a Record")
    };
    assert_eq!(options.get("value_min"), Some(&Value::Float(0.0)));
    assert_eq!(options.get("value_max"), Some(&Value::Float(10.0)));
    assert_eq!(options.get("scale_min"), Some(&Value::Float(0.0)));
    assert_eq!(options.get("scale_max"), Some(&Value::Float(10.0)));
    assert_eq!(direct, render(specification));
}

#[test]
fn clustered_heatmap_spec_freezes_nearest_order_and_replays_exactly() {
    let common = vec![
        ("theme", Value::Str("publication".into())),
        (
            "row_labels",
            Value::List(
                vec![
                    Value::Str("r0".into()),
                    Value::Str("r1".into()),
                    Value::Str("r2".into()),
                ]
                .into(),
            ),
        ),
    ];
    let direct = call_bio_plots_builtin(
        "clustered_heatmap",
        vec![heatmap_input(), opts(common.clone())],
    )
    .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification = call_bio_plots_builtin(
        "clustered_heatmap",
        vec![heatmap_input(), opts(spec_options)],
    )
    .unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("plot"), Some(Value::Str(plot)) if plot == "clustered_heatmap"));
    let Some(Value::Table(rows)) = map.get("rows") else {
        panic!("clustered row metadata is not a Table")
    };
    assert_eq!(rows.num_rows(), 3);
    let mut display_order = rows
        .rows
        .iter()
        .map(|row| {
            (
                row[1].as_float().unwrap() as usize,
                row[0].as_float().unwrap() as usize,
            )
        })
        .collect::<Vec<_>>();
    display_order.sort_unstable();
    assert_eq!(display_order[0].1, 0);
    assert_eq!(direct, render(specification));
}

#[test]
fn hierarchical_heatmap_spec_keeps_merge_heights_and_exact_replay() {
    let common = vec![
        ("order", Value::Str("hierarchical".into())),
        ("linkage", Value::Str("average".into())),
        ("distance", Value::Str("euclidean".into())),
        ("dendrogram", Value::Str("both".into())),
        ("theme", Value::Str("publication".into())),
    ];
    let direct = call_bio_plots_builtin(
        "clustered_heatmap",
        vec![heatmap_input(), opts(common.clone())],
    )
    .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification = call_bio_plots_builtin(
        "clustered_heatmap",
        vec![heatmap_input(), opts(spec_options)],
    )
    .unwrap();
    let map = record(&specification);
    let Some(Value::Table(row_merges)) = map.get("row_merges") else {
        panic!("row merges are not a Table")
    };
    let Some(Value::Table(column_merges)) = map.get("column_merges") else {
        panic!("column merges are not a Table")
    };
    assert_eq!(row_merges.num_rows(), 2);
    assert_eq!(column_merges.num_rows(), 1);
    assert!(row_merges
        .rows
        .iter()
        .all(|row| row[2].as_float().is_some_and(f64::is_finite)));
    assert_eq!(direct, render(specification));
}

#[test]
fn malformed_specs_are_rejected_and_html_contains_canvas_fallback() {
    let specification = call_bio_plots_builtin(
        "violin",
        vec![
            wide_violin_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Value::Str(html) = call_plot_builtin(
        "render_plot",
        vec![
            specification.clone(),
            opts(vec![("format", Value::Str("html".into()))]),
        ],
    )
    .unwrap() else {
        panic!("expected HTML")
    };
    assert!(html.contains("<svg"));
    assert!(html.contains("<canvas"));
    let mut broken = record(&specification).clone();
    broken.insert("groups".into(), Value::Table(Table::empty()));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken.into())]).is_err());

    let heatmap = call_plot_builtin(
        "heatmap",
        vec![
            heatmap_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let mut broken_heatmap = record(&heatmap).clone();
    broken_heatmap.insert("data".into(), Value::Table(Table::empty()));
    assert!(call_plot_builtin("render_plot", vec![Value::Record(broken_heatmap.into())]).is_err());
}

#[test]
fn heatmap_non_finite_cells_are_disclosed_and_replay_as_na_colour() {
    let input = Value::Table(Table::new(
        vec!["a".into(), "b".into()],
        vec![
            vec![Value::Float(1.0), Value::Float(f64::NAN)],
            vec![Value::Float(-2.0), Value::Float(3.0)],
        ],
    ));
    let common = vec![
        ("theme", Value::Str("publication".into())),
        ("na_color", Value::Str("#123456".into())),
    ];
    let direct = call_plot_builtin("heatmap", vec![input.clone(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification = call_plot_builtin("heatmap", vec![input, opts(spec_options)]).unwrap();
    let map = record(&specification);
    let Some(Value::Record(provenance)) = map.get("provenance") else {
        panic!("heatmap provenance is not a Record")
    };
    assert_eq!(provenance.get("non_finite_cells"), Some(&Value::Int(1)));
    let Some(Value::List(warnings)) = map.get("warnings") else {
        panic!("heatmap warnings are not a List")
    };
    assert_eq!(warnings.len(), 1);
    let replay = render(specification);
    assert_eq!(direct, replay);
    let Value::Str(svg) = replay else {
        panic!("expected SVG")
    };
    assert!(svg.contains("#123456"));
    assert!(!svg.contains("x=\"NaN\""));
}

#[test]
fn dense_heatmap_spec_keeps_every_cell_and_replays_without_expanding_the_svg_unreasonably() {
    const ROWS: usize = 100;
    const COLS: usize = 100;
    let matrix = Value::Matrix(
        Matrix {
            data: (0..ROWS * COLS)
                .map(|index| ((index * 37) % 211) as f64 / 20.0 - 5.0)
                .collect(),
            nrow: ROWS,
            ncol: COLS,
            row_names: Some((0..ROWS).map(|index| format!("feature-{index}")).collect()),
            col_names: Some((0..COLS).map(|index| format!("sample-{index}")).collect()),
        }
        .into(),
    );
    let specification = call_plot_builtin(
        "heatmap",
        vec![
            matrix,
            opts(vec![
                ("format", Value::Str("spec".into())),
                ("width", Value::Int(900)),
                ("height", Value::Int(700)),
            ]),
        ],
    )
    .unwrap();
    let map = record(&specification);
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("heatmap cells are not a Table")
    };
    assert_eq!(data.num_rows(), ROWS * COLS);
    let Value::Str(svg) = render(specification) else {
        panic!("expected SVG")
    };
    assert!(svg.contains("feature-99"));
    assert!(svg.contains("sample-99"));
    assert!(
        svg.len() < 2_000_000,
        "dense SVG grew to {} bytes",
        svg.len()
    );
}
