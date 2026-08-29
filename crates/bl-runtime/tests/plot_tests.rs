use bl_core::value::{Table, Value};
use bl_runtime::plot::call_plot_builtin;
use std::collections::HashMap;

fn make_table(cols: Vec<&str>, rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(
        cols.into_iter().map(|s| s.to_string()).collect(),
        rows,
    ))
}

// ── Plot (scatter) tests ────────────────────────────────────────

#[test]
fn test_plot_scatter_returns_svg() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(3.0), Value::Float(4.0)],
        ],
    );
    let result = call_plot_builtin("plot", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<circle"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_plot_single_data_point() {
    let table = make_table(
        vec!["x", "y"],
        vec![vec![Value::Float(5.0), Value::Float(10.0)]],
    );
    let result = call_plot_builtin("plot", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<circle"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_plot_wrong_type() {
    let result = call_plot_builtin("plot", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_plot_line_type() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(3.0), Value::Float(4.0)],
            vec![Value::Float(5.0), Value::Float(1.0)],
        ],
    );
    let opts = Value::Record((HashMap::from([("type".into(), Value::Str("line".into()))])).into());
    let result = call_plot_builtin("plot", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<polyline"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_plot_bar_type() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(10.0)],
            vec![Value::Float(2.0), Value::Float(20.0)],
            vec![Value::Float(3.0), Value::Float(15.0)],
        ],
    );
    let opts = Value::Record((HashMap::from([("type".into(), Value::Str("bar".into()))])).into());
    let result = call_plot_builtin("plot", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_plot_box_type() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(10.0)],
            vec![Value::Float(2.0), Value::Float(20.0)],
            vec![Value::Float(3.0), Value::Float(15.0)],
            vec![Value::Float(4.0), Value::Float(25.0)],
        ],
    );
    let opts = Value::Record((HashMap::from([("type".into(), Value::Str("box".into()))])).into());
    let result = call_plot_builtin("plot", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_plot_unknown_type_error() {
    let table = make_table(
        vec!["x", "y"],
        vec![vec![Value::Float(1.0), Value::Float(2.0)]],
    );
    let opts =
        Value::Record((HashMap::from([("type".into(), Value::Str("invalid".into()))])).into());
    let result = call_plot_builtin("plot", vec![table, opts]);
    assert!(result.is_err());
}

fn record_field<'a>(value: &'a Value, field: &str) -> &'a Value {
    match value {
        Value::Record(record) => record
            .get(field)
            .unwrap_or_else(|| panic!("missing Record field {field}")),
        other => panic!("expected Record, got {other:?}"),
    }
}

#[test]
fn plot_spec_is_versioned_inspectable_and_stable() {
    let table = make_table(
        vec!["time", "mean", "low", "high"],
        vec![
            vec![
                Value::Float(1.0),
                Value::Float(4.0),
                Value::Float(3.0),
                Value::Float(5.0),
            ],
            vec![
                Value::Float(2.0),
                Value::Float(6.0),
                Value::Float(5.0),
                Value::Float(7.0),
            ],
            vec![
                Value::Float(f64::NAN),
                Value::Float(8.0),
                Value::Float(7.0),
                Value::Float(9.0),
            ],
        ],
    );
    let opts = Value::Record(
        HashMap::from([
            ("type".into(), Value::Str("confidence".into())),
            ("x".into(), Value::Str("time".into())),
            ("y".into(), Value::Str("mean".into())),
            ("ymin".into(), Value::Str("low".into())),
            ("ymax".into(), Value::Str("high".into())),
            ("title".into(), Value::Str("Estimate".into())),
        ])
        .into(),
    );
    let spec = call_plot_builtin("plot_spec", vec![table, opts]).unwrap();
    assert!(
        matches!(record_field(&spec, "schema"), Value::Str(value) if value == "biolang.plot.spec/v1")
    );
    assert!(matches!(record_field(&spec, "kind"), Value::Str(value) if value == "confidence"));
    assert!(matches!(
        record_field(&spec, "dropped_non_finite"),
        Value::Int(1)
    ));
    let data = match record_field(&spec, "data") {
        Value::Table(table) => table,
        other => panic!("expected data Table, got {other:?}"),
    };
    assert_eq!(data.num_rows(), 2);
    assert_eq!(
        data.columns,
        vec!["source_row", "series", "colour", "x", "y", "lower", "upper"]
    );
    assert!(matches!(data.rows[1][0], Value::Int(1)));
}

#[test]
fn plot_and_render_plot_use_the_same_specification() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(2.0), Value::Float(3.0)],
            vec![Value::Float(3.0), Value::Float(5.0)],
        ],
    );
    let options = Value::Record(
        HashMap::from([
            ("type".into(), Value::Str("line".into())),
            ("title".into(), Value::Str("Shared".into())),
        ])
        .into(),
    );
    let direct = call_plot_builtin("plot", vec![table.clone(), options.clone()]).unwrap();
    let spec = call_plot_builtin("plot_spec", vec![table, options]).unwrap();
    let rendered = call_plot_builtin("render_plot", vec![spec.clone()]).unwrap();
    assert_eq!(direct, rendered);

    let ascii_options =
        Value::Record(HashMap::from([("format".into(), Value::Str("ascii".into()))]).into());
    let ascii = call_plot_builtin("render_plot", vec![spec.clone(), ascii_options]).unwrap();
    assert!(
        matches!(ascii, Value::Str(text) if !text.contains("<svg") && text.lines().count() > 5)
    );

    let html_options =
        Value::Record(HashMap::from([("format".into(), Value::Str("html".into()))]).into());
    let html = call_plot_builtin("render_plot", vec![spec, html_options]).unwrap();
    assert!(
        matches!(html, Value::Str(text) if text.contains("<canvas") && text.contains("Use canvas") && text.contains("<svg"))
    );
}

#[test]
fn error_bars_and_confidence_bands_render_from_explicit_bounds() {
    let table = make_table(
        vec!["x", "estimate", "lower", "upper"],
        vec![
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Float(1.5),
                Value::Float(2.5),
            ],
            vec![
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Float(2.1),
                Value::Float(3.9),
            ],
        ],
    );
    for (kind, expected) in [("errorbar", "<circle"), ("confidence", "<polygon")] {
        let opts = Value::Record(
            HashMap::from([
                ("type".into(), Value::Str(kind.into())),
                ("y".into(), Value::Str("estimate".into())),
                ("ymin".into(), Value::Str("lower".into())),
                ("ymax".into(), Value::Str("upper".into())),
            ])
            .into(),
        );
        let value = call_plot_builtin("plot", vec![table.clone(), opts]).unwrap();
        assert!(matches!(value, Value::Str(svg) if svg.contains(expected)));
    }
}

#[test]
fn statistical_geometry_exposes_box_ecdf_qq_and_violin_values() {
    let values = Value::List(
        vec![1.0, 2.0, 2.0, 3.0, 4.0, 100.0]
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    );

    let boxes = call_plot_builtin("boxplot_data", vec![values.clone()]).unwrap();
    let groups = match record_field(&boxes, "groups") {
        Value::Table(table) => table,
        other => panic!("expected box groups Table, got {other:?}"),
    };
    assert_eq!(groups.num_rows(), 1);
    let outlier_count = groups.col_index("outlier_count").unwrap();
    assert!(matches!(groups.rows[0][outlier_count], Value::Int(1)));

    let ecdf = call_plot_builtin("ecdf_data", vec![values.clone()]).unwrap();
    let ecdf_table = match record_field(&ecdf, "data") {
        Value::Table(table) => table,
        other => panic!("expected ECDF data Table, got {other:?}"),
    };
    assert_eq!(ecdf_table.num_rows(), 5, "ties share one ECDF jump");
    let count = ecdf_table.col_index("count").unwrap();
    assert!(matches!(ecdf_table.rows[1][count], Value::Int(2)));

    let qq = call_plot_builtin("normal_qq_data", vec![values.clone()]).unwrap();
    assert!(
        matches!(record_field(&qq, "plotting_position"), Value::Str(value) if value == "R_ppoints")
    );
    assert!(matches!(record_field(&qq, "line_slope"), Value::Float(value) if value.is_finite()));

    let violin = call_plot_builtin("violin_data", vec![values]).unwrap();
    let density = match record_field(&violin, "data") {
        Value::Table(table) => table,
        other => panic!("expected violin density Table, got {other:?}"),
    };
    assert_eq!(density.num_rows(), 256);
    let scaled = density.col_index("scaled").unwrap();
    let peak = density
        .rows
        .iter()
        .filter_map(|row| row[scaled].as_float())
        .fold(0.0, f64::max);
    assert!((peak - 1.0).abs() < 1e-12);
}

#[test]
fn linear_fit_geometry_distinguishes_confidence_and_prediction_intervals() {
    let x = Value::List(
        (1..=6)
            .map(|value| Value::Float(value as f64))
            .collect::<Vec<_>>()
            .into(),
    );
    let y = Value::List(
        [2.1, 3.9, 6.2, 7.8, 10.4, 11.7]
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    );
    let result = call_plot_builtin("linear_fit_data", vec![x, y]).unwrap();
    assert!(matches!(record_field(&result, "n"), Value::Int(6)));
    assert!(matches!(
        record_field(&result, "degrees_of_freedom"),
        Value::Int(4)
    ));
    let table = match record_field(&result, "data") {
        Value::Table(table) => table,
        other => panic!("expected linear fit data Table, got {other:?}"),
    };
    let confidence_lower = table.col_index("confidence_lower").unwrap();
    let confidence_upper = table.col_index("confidence_upper").unwrap();
    let prediction_lower = table.col_index("prediction_lower").unwrap();
    let prediction_upper = table.col_index("prediction_upper").unwrap();
    for row in &table.rows {
        let confidence_width =
            row[confidence_upper].as_float().unwrap() - row[confidence_lower].as_float().unwrap();
        let prediction_width =
            row[prediction_upper].as_float().unwrap() - row[prediction_lower].as_float().unwrap();
        assert!(prediction_width > confidence_width);
    }
}

#[test]
fn categorical_geometry_preserves_first_observed_order_and_missing_counts() {
    let values = Value::List(
        vec![
            Value::Str("b".into()),
            Value::Str("a".into()),
            Value::Str("b".into()),
            Value::Nil,
            Value::Bool(true),
        ]
        .into(),
    );
    let result = call_plot_builtin("categorical_data", vec![values]).unwrap();
    assert!(
        matches!(record_field(&result, "schema"), Value::Str(value) if value == "biolang.plot.geometry/v1")
    );
    assert!(
        matches!(record_field(&result, "ordering"), Value::Str(value) if value == "first_observed")
    );
    assert_eq!(record_field(&result, "n_total"), &Value::Int(5));
    assert_eq!(record_field(&result, "n_observed"), &Value::Int(4));
    assert_eq!(record_field(&result, "missing"), &Value::Int(1));

    let data = match record_field(&result, "data") {
        Value::Table(table) => table,
        other => panic!("expected categorical data Table, got {other:?}"),
    };
    let label = data.col_index("label").unwrap();
    let count = data.col_index("count").unwrap();
    let proportion = data.col_index("proportion").unwrap();
    assert_eq!(data.num_rows(), 3);
    assert_eq!(data.rows[0][label], Value::Str("b".into()));
    assert_eq!(data.rows[1][label], Value::Str("a".into()));
    assert_eq!(data.rows[2][label], Value::Str("true".into()));
    assert_eq!(data.rows[0][count], Value::Int(2));
    assert_eq!(data.rows[1][count], Value::Int(1));
    assert_eq!(data.rows[2][count], Value::Int(1));
    let total = data
        .rows
        .iter()
        .map(|row| row[proportion].as_float().unwrap())
        .sum::<f64>();
    assert!((total - 1.0).abs() < 1e-12);
}

#[test]
fn categorical_geometry_rejects_unobserved_and_non_scalar_categories() {
    assert!(call_plot_builtin(
        "categorical_data",
        vec![Value::List(vec![Value::Nil, Value::Nil].into())]
    )
    .is_err());
    assert!(call_plot_builtin(
        "categorical_data",
        vec![Value::List(
            vec![Value::List(vec![Value::Int(1)].into())].into()
        )]
    )
    .is_err());
}

#[test]
fn missingness_geometry_separates_full_counts_from_bounded_display_cells() {
    let table = make_table(
        vec!["a", "b", "c", "d"],
        vec![
            vec![
                Value::Int(1),
                Value::Nil,
                Value::Float(f64::NAN),
                Value::Int(4),
            ],
            vec![
                Value::Nil,
                Value::Int(2),
                Value::Int(3),
                Value::Float(f64::INFINITY),
            ],
            vec![Value::Int(1), Value::Int(2), Value::Int(3), Value::Int(4)],
            vec![Value::Nil, Value::Nil, Value::Int(3), Value::Int(4)],
            vec![Value::Int(1), Value::Int(2), Value::Nil, Value::Int(4)],
        ],
    );
    let options = Value::Record(
        HashMap::from([
            ("max_rows".into(), Value::Int(2)),
            ("max_columns".into(), Value::Int(2)),
        ])
        .into(),
    );
    let result = call_plot_builtin("missingness_data", vec![table, options]).unwrap();
    assert_eq!(record_field(&result, "n_rows"), &Value::Int(5));
    assert_eq!(record_field(&result, "n_columns"), &Value::Int(4));
    assert_eq!(record_field(&result, "missing_cells"), &Value::Int(7));
    assert_eq!(record_field(&result, "row_stride"), &Value::Int(3));
    assert_eq!(record_field(&result, "column_stride"), &Value::Int(2));

    let summary = match record_field(&result, "column_summary") {
        Value::Table(table) => table,
        other => panic!("expected column summary Table, got {other:?}"),
    };
    let missing_count = summary.col_index("missing_count").unwrap();
    assert_eq!(
        summary
            .rows
            .iter()
            .map(|row| row[missing_count].as_int().unwrap())
            .collect::<Vec<_>>(),
        vec![2, 2, 2, 1]
    );

    let cells = match record_field(&result, "cells") {
        Value::Table(table) => table,
        other => panic!("expected missingness cells Table, got {other:?}"),
    };
    assert_eq!(cells.num_rows(), 4);
    let source_row = cells.col_index("source_row").unwrap();
    let source_column = cells.col_index("source_column").unwrap();
    let missing = cells.col_index("missing").unwrap();
    let observed = cells
        .rows
        .iter()
        .map(|row| {
            (
                row[source_row].as_int().unwrap(),
                row[source_column].as_int().unwrap(),
                matches!(row[missing], Value::Bool(true)),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed,
        vec![(0, 0, false), (0, 2, true), (3, 0, true), (3, 2, false)]
    );
}

#[test]
fn missingness_geometry_rejects_invalid_display_limits() {
    let table = make_table(vec!["a"], vec![vec![Value::Nil]]);
    for invalid in [Value::Int(0), Value::Float(1.5), Value::Str("many".into())] {
        let options = Value::Record(HashMap::from([("max_rows".into(), invalid)]).into());
        assert!(call_plot_builtin("missingness_data", vec![table.clone(), options]).is_err());
    }
}

#[test]
fn svg_and_html_renderers_expose_accessible_structure() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Int(1), Value::Int(2)],
            vec![Value::Int(2), Value::Int(4)],
        ],
    );
    let options =
        Value::Record(HashMap::from([("title".into(), Value::Str("A & B".into()))]).into());
    let spec = call_plot_builtin("plot_spec", vec![table, options]).unwrap();
    let svg = call_plot_builtin("render_plot", vec![spec.clone()]).unwrap();
    assert!(matches!(svg, Value::Str(ref text)
        if text.contains("role=\"img\"")
            && text.contains("focusable=\"false\"")
            && text.contains("aria-label=\"A &amp; B\"")
            && text.contains("<title>A &amp; B</title>")
            && text.contains("<desc>scatter plot with 1 series and 2 rendered marks; 0 non-finite rows were excluded.</desc>")));

    let html_options =
        Value::Record(HashMap::from([("format".into(), Value::Str("html".into()))]).into());
    let html = call_plot_builtin("render_plot", vec![spec, html_options]).unwrap();
    assert!(matches!(html, Value::Str(ref text)
        if text.contains("<title>A &amp; B</title>")
            && text.contains("<figure id=\"bl-figure\" aria-labelledby=\"bl-caption\">")
            && text.contains("<figcaption id=\"bl-caption\"")
            && text.contains("type=\"button\" id=\"bl-toggle\"")
            && text.contains("aria-controls=\"bl-svg bl-canvas\"")
            && text.contains("aria-pressed=\"false\"")
            && text.contains("aria-label=\"A &amp; B canvas fallback\"")
            && text.contains("s.id='bl-svg'")));
}

#[test]
fn interval_plot_rejects_missing_or_reversed_bounds() {
    let table = make_table(
        vec!["x", "y", "low", "high"],
        vec![vec![
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(1.0),
        ]],
    );
    let missing =
        Value::Record(HashMap::from([("type".into(), Value::Str("errorbar".into()))]).into());
    assert!(call_plot_builtin("plot_spec", vec![table.clone(), missing]).is_err());
    let reversed = Value::Record(
        HashMap::from([
            ("type".into(), Value::Str("errorbar".into())),
            ("ymin".into(), Value::Str("low".into())),
            ("ymax".into(), Value::Str("high".into())),
        ])
        .into(),
    );
    assert!(call_plot_builtin("plot_spec", vec![table, reversed]).is_err());
}

// ── Histogram tests ─────────────────────────────────────────────

#[test]
fn test_histogram_returns_svg() {
    let list = Value::List(
        (vec![
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(4.0),
            Value::Float(5.0),
            Value::Float(3.0),
        ])
        .into(),
    );
    let result = call_plot_builtin("histogram", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_histogram_single_value() {
    let list = Value::List((vec![Value::Float(5.0)]).into());
    let result = call_plot_builtin("histogram", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_histogram_all_same_values() {
    let list = Value::List(
        (vec![
            Value::Float(3.0),
            Value::Float(3.0),
            Value::Float(3.0),
            Value::Float(3.0),
        ])
        .into(),
    );
    let result = call_plot_builtin("histogram", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_histogram_empty_list_error() {
    let list = Value::List((vec![]).into());
    let result = call_plot_builtin("histogram", vec![list]);
    assert!(result.is_err());
}

#[test]
fn test_histogram_wrong_type() {
    let result = call_plot_builtin("histogram", vec![Value::Int(42)]);
    assert!(result.is_err());
}

fn histogram_counts(result: Value) -> Vec<i64> {
    let record = match result {
        Value::Record(record) => record,
        other => panic!("expected histogram geometry Record, got {other:?}"),
    };
    assert!(matches!(
        record.get("schema"),
        Some(Value::Str(schema)) if schema == "biolang.plot.geometry/v1"
    ));
    let bins = match record.get("bins") {
        Some(Value::Table(table)) => table,
        other => panic!("expected bins Table, got {other:?}"),
    };
    assert_eq!(
        bins.columns,
        vec![
            "bin",
            "left",
            "right",
            "left_closed",
            "right_closed",
            "count",
            "density",
            "cumulative_count",
            "cumulative_fraction"
        ]
    );
    let count_column = bins.col_index("count").unwrap();
    bins.rows
        .iter()
        .map(|row| match row[count_column] {
            Value::Int(value) => value,
            ref other => panic!("expected integer count, got {other:?}"),
        })
        .collect()
}

#[test]
fn test_histogram_data_matches_right_closed_endpoint_rules() {
    let values = Value::List(
        vec![0.0, 1.0, 1.5, 2.0, 3.0]
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    );
    let breaks = Value::List(
        vec![0.0, 1.0, 2.0, 3.0]
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    );
    let opts = Value::Record(
        HashMap::from([
            ("breaks".into(), breaks),
            ("right".into(), Value::Bool(true)),
            ("include_lowest".into(), Value::Bool(true)),
        ])
        .into(),
    );
    let result = call_plot_builtin("histogram_data", vec![values, opts]).unwrap();
    assert_eq!(histogram_counts(result), vec![2, 2, 1]);
}

#[test]
fn test_histogram_data_matches_left_closed_endpoint_rules() {
    let values = Value::List(
        vec![0.0, 1.0, 1.5, 2.0, 3.0]
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    );
    let opts = Value::Record(
        HashMap::from([
            (
                "breaks".into(),
                Value::List(
                    vec![0.0, 1.0, 2.0, 3.0]
                        .into_iter()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                ),
            ),
            ("closed".into(), Value::Str("left".into())),
        ])
        .into(),
    );
    let result = call_plot_builtin("histogram_data", vec![values, opts]).unwrap();
    assert_eq!(histogram_counts(result), vec![1, 2, 2]);
}

#[test]
fn test_histogram_data_include_lowest_controls_the_outer_endpoint() {
    let values = Value::List(
        vec![0.0, 1.0, 1.5, 2.0, 3.0]
            .into_iter()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    );
    let breaks = || {
        Value::List(
            vec![0.0, 1.0, 2.0, 3.0]
                .into_iter()
                .map(Value::Float)
                .collect::<Vec<_>>()
                .into(),
        )
    };
    let right = call_plot_builtin(
        "histogram_data",
        vec![
            values.clone(),
            Value::Record(
                HashMap::from([
                    ("breaks".into(), breaks()),
                    ("right".into(), Value::Bool(true)),
                    ("include_lowest".into(), Value::Bool(false)),
                ])
                .into(),
            ),
        ],
    )
    .unwrap();
    let left = call_plot_builtin(
        "histogram_data",
        vec![
            values,
            Value::Record(
                HashMap::from([
                    ("breaks".into(), breaks()),
                    ("closed".into(), Value::Str("left".into())),
                    ("include_lowest".into(), Value::Bool(false)),
                ])
                .into(),
            ),
        ],
    )
    .unwrap();
    assert_eq!(histogram_counts(right), vec![1, 2, 1]);
    assert_eq!(histogram_counts(left), vec![1, 2, 1]);
}

#[test]
fn test_histogram_data_reports_dropped_values() {
    let values = Value::List(
        vec![
            Value::Float(-1.0),
            Value::Float(0.5),
            Value::Str("bad".into()),
            Value::Float(f64::NAN),
            Value::Nil,
            Value::Float(4.0),
        ]
        .into(),
    );
    let opts = Value::Record(
        HashMap::from([(
            "breaks".into(),
            Value::List(vec![Value::Float(0.0), Value::Float(1.0), Value::Float(2.0)].into()),
        )])
        .into(),
    );
    let result = call_plot_builtin("histogram_data", vec![values, opts]).unwrap();
    let record = match result {
        Value::Record(record) => record,
        _ => unreachable!(),
    };
    assert!(matches!(record.get("n_total"), Some(Value::Int(6))));
    assert!(matches!(record.get("n_finite"), Some(Value::Int(3))));
    assert!(matches!(record.get("n_included"), Some(Value::Int(1))));
    assert!(matches!(record.get("dropped_invalid"), Some(Value::Int(2))));
    assert!(matches!(
        record.get("dropped_non_finite"),
        Some(Value::Int(1))
    ));
    assert!(matches!(record.get("dropped_outside"), Some(Value::Int(2))));
}

#[test]
fn test_histogram_data_rejects_invalid_breaks_and_bin_counts() {
    let values = Value::List(vec![Value::Int(1), Value::Int(2)].into());
    for breaks in [
        Value::List(vec![Value::Int(0)].into()),
        Value::List(vec![Value::Int(0), Value::Int(2), Value::Int(1)].into()),
        Value::Int(0),
    ] {
        let opts = Value::Record(HashMap::from([("breaks".into(), breaks)]).into());
        assert!(call_plot_builtin("histogram_data", vec![values.clone(), opts]).is_err());
    }
}

// ── Heatmap tests ───────────────────────────────────────────────

#[test]
fn test_heatmap_returns_svg() {
    let table = make_table(
        vec!["a", "b"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(3.0), Value::Float(4.0)],
        ],
    );
    let result = call_plot_builtin("heatmap", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_heatmap_negative_values() {
    let table = make_table(
        vec!["a", "b"],
        vec![
            vec![Value::Float(-5.0), Value::Float(-1.0)],
            vec![Value::Float(-3.0), Value::Float(-2.0)],
        ],
    );
    let result = call_plot_builtin("heatmap", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_heatmap_wrong_type() {
    let result = call_plot_builtin("heatmap", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn heatmap_row_clustering_keeps_the_documented_mean_order() {
    let table = make_table(
        vec!["a", "b"],
        vec![
            vec![Value::Float(10.0), Value::Float(8.0)],
            vec![Value::Float(1.0), Value::Float(0.0)],
            vec![Value::Float(5.0), Value::Float(4.0)],
        ],
    );
    let opts = Value::Record(
        HashMap::from([
            ("cluster".into(), Value::Bool(true)),
            (
                "row_labels".into(),
                Value::List(
                    vec![
                        Value::Str("high".into()),
                        Value::Str("low".into()),
                        Value::Str("middle".into()),
                    ]
                    .into(),
                ),
            ),
        ])
        .into(),
    );
    let Value::Str(svg) = call_plot_builtin("heatmap", vec![table, opts]).unwrap() else {
        panic!("expected SVG")
    };
    assert!(svg.find(">low<").unwrap() < svg.find(">middle<").unwrap());
    assert!(svg.find(">middle<").unwrap() < svg.find(">high<").unwrap());
}

#[test]
fn publication_heatmap_survives_notebook_and_journal_widths() {
    let table = make_table(
        vec!["control sample", "treated sample", "recovery sample"],
        vec![
            vec![Value::Float(-2.0), Value::Float(0.0), Value::Float(2.0)],
            vec![Value::Float(-1.0), Value::Float(0.5), Value::Float(1.5)],
            vec![Value::Float(-0.5), Value::Float(1.0), Value::Float(2.5)],
        ],
    );
    for width in [321_i64, 680, 800] {
        let opts = Value::Record(
            HashMap::from([
                ("theme".into(), Value::Str("publication".into())),
                ("width".into(), Value::Int(width)),
                ("height".into(), Value::Int(400)),
                ("title".into(), Value::Str("Expression heatmap".into())),
                ("subtitle".into(), Value::Str("Scaled measurements".into())),
                (
                    "caption".into(),
                    Value::Str("Rows retain input order".into()),
                ),
                ("legend_title".into(), Value::Str("z-score".into())),
                (
                    "row_labels".into(),
                    Value::List(
                        vec![
                            Value::Str("TP53".into()),
                            Value::Str("BRCA1".into()),
                            Value::Str("EGFR".into()),
                        ]
                        .into(),
                    ),
                ),
            ])
            .into(),
        );
        let Value::Str(svg) =
            call_plot_builtin("heatmap", vec![table.clone(), opts]).expect("publication heatmap")
        else {
            panic!("expected SVG")
        };
        assert!(svg.contains(&format!("width=\"{width}\"")));
        assert!(svg.contains("data-biolang-theme=\"publication\""));
        assert!(svg.contains(">Scaled measurements<"));
        assert!(svg.contains(">Rows retain input order<"));
        assert!(svg.contains(">TP53<"));
        assert!(svg.contains(">treated sample<"));
        assert!(svg.contains("#3b4cc0"));
        assert!(svg.contains("#f7f7f7"));
        assert!(svg.contains("#b40426"));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }
}

// ── Volcano tests ───────────────────────────────────────────────

#[test]
fn test_volcano_returns_svg() {
    let table = make_table(
        vec!["log2fc", "pvalue"],
        vec![
            vec![Value::Float(2.5), Value::Float(0.001)],
            vec![Value::Float(-1.0), Value::Float(0.1)],
            vec![Value::Float(0.5), Value::Float(0.5)],
        ],
    );
    let result = call_plot_builtin("volcano", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<circle"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_volcano_no_significant_points() {
    let table = make_table(
        vec!["log2fc", "pvalue"],
        vec![
            vec![Value::Float(0.1), Value::Float(0.9)],
            vec![Value::Float(-0.2), Value::Float(0.8)],
            vec![Value::Float(0.05), Value::Float(0.95)],
        ],
    );
    let result = call_plot_builtin("volcano", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        // All points should be gray (#999) since none pass thresholds
        assert!(s.contains("#999"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_volcano_wrong_type() {
    let result = call_plot_builtin("volcano", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── Genome track tests ──────────────────────────────────────────

#[test]
fn test_genome_track_returns_svg() {
    let table = make_table(
        vec!["chrom", "start", "end", "name", "strand"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Int(100),
                Value::Int(200),
                Value::Str("geneA".into()),
                Value::Str("+".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(300),
                Value::Int(400),
                Value::Str("geneB".into()),
                Value::Str("-".into()),
            ],
        ],
    );
    let result = call_plot_builtin("genome_track", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_genome_track_wrong_type() {
    let result = call_plot_builtin("genome_track", vec![Value::Int(42)]);
    assert!(result.is_err());
}

// ── Save SVG tests ──────────────────────────────────────────────

#[test]
fn test_save_svg_roundtrip() {
    let svg = Value::Str("<svg></svg>".into());
    let dir = std::env::temp_dir();
    let path = dir.join("bl_test_save_plot.svg");
    let result = call_plot_builtin(
        "save_svg",
        vec![svg, Value::Str(path.to_string_lossy().into())],
    )
    .unwrap();
    assert!(matches!(result, Value::Str(_)));
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "<svg></svg>");
    let _ = std::fs::remove_file(path);
}

#[test]
fn publication_svg_export_records_physical_size_and_font_profile() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal.svg");
    let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" width="600" height="400" viewBox="0 0 600 400"><text x="10" y="20">figure</text></svg>"#;
    call_plot_builtin(
        "save_svg",
        vec![
            Value::Str(svg.into()),
            Value::Str(path.to_string_lossy().into()),
            Value::Record(
                HashMap::from([
                    ("profile".into(), Value::Str("publication".into())),
                    ("font".into(), Value::Str("serif".into())),
                    ("width_mm".into(), Value::Float(180.0)),
                    ("height_mm".into(), Value::Float(120.0)),
                ])
                .into(),
            ),
        ],
    )
    .unwrap();
    let written = std::fs::read_to_string(path).unwrap();
    assert!(written.contains("data-biolang-export=\"publication\""));
    assert!(written.contains("width=\"180mm\""));
    assert!(written.contains("height=\"120mm\""));
    assert!(written.contains("Times New Roman,Times,serif"));
    assert!(written.contains("<metadata>"));
}

#[test]
fn save_svg_accepts_a_plot_spec_without_manual_replay() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("spec.svg");
    let data = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(2.0), Value::Float(4.0)],
        ],
    );
    let spec = call_plot_builtin("plot_spec", vec![data]).unwrap();
    call_plot_builtin(
        "save_svg",
        vec![spec, Value::Str(path.to_string_lossy().into())],
    )
    .unwrap();
    assert!(std::fs::read_to_string(path).unwrap().starts_with("<svg"));
}

#[test]
fn test_save_svg_wrong_type_first_arg() {
    let result = call_plot_builtin(
        "save_svg",
        vec![Value::Int(42), Value::Str("out.svg".into())],
    );
    assert!(result.is_err());
}

#[test]
fn test_save_svg_wrong_type_second_arg() {
    let result = call_plot_builtin(
        "save_svg",
        vec![Value::Str("<svg></svg>".into()), Value::Int(42)],
    );
    assert!(result.is_err());
}

#[test]
fn test_save_svg_rejects_plain_text_and_profiles_compact_svg() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("bad.svg");
    assert!(call_plot_builtin(
        "save_svg",
        vec![
            Value::Str("not an image".into()),
            Value::Str(bad.to_string_lossy().into()),
        ],
    )
    .is_err());
    assert!(!bad.exists());

    let compact = dir.path().join("compact.svg");
    call_plot_builtin(
        "save_svg",
        vec![
            Value::Str("<svg></svg>".into()),
            Value::Str(compact.to_string_lossy().into()),
            Value::Record(
                HashMap::from([("profile".into(), Value::Str("publication".into()))]).into(),
            ),
        ],
    )
    .unwrap();
    assert!(std::fs::read_to_string(compact)
        .unwrap()
        .contains("<svg data-biolang-export=\"publication\""));
}

// ── SVG output format validation ────────────────────────────────

#[test]
fn test_plot_svg_output_format() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(3.0), Value::Float(4.0)],
        ],
    );
    let result = call_plot_builtin("plot", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.starts_with("<svg"));
        assert!(s.ends_with("</svg>"));
        assert!(s.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    } else {
        panic!("expected Str");
    }
}

// ── Unknown builtin ─────────────────────────────────────────────

// ── MA plot tests ──────────────────────────────────────────────

#[test]
fn test_ma_plot_returns_svg() {
    let table = make_table(
        vec!["baseMean", "log2fc"],
        vec![
            vec![Value::Float(100.0), Value::Float(2.0)],
            vec![Value::Float(200.0), Value::Float(-1.5)],
            vec![Value::Float(50.0), Value::Float(0.3)],
            vec![Value::Float(500.0), Value::Float(3.0)],
        ],
    );
    let result = call_plot_builtin("ma_plot", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<circle"));
        // Should have zero line
        assert!(s.contains("<line"));
    } else {
        panic!("expected Str with SVG");
    }
}

#[test]
fn test_ma_plot_custom_columns() {
    let table = make_table(
        vec!["avg_expr", "fold_change"],
        vec![
            vec![Value::Float(10.0), Value::Float(1.5)],
            vec![Value::Float(20.0), Value::Float(-0.5)],
        ],
    );
    let opts = Value::Record(
        (HashMap::from([
            ("a".into(), Value::Str("avg_expr".into())),
            ("m".into(), Value::Str("fold_change".into())),
        ]))
        .into(),
    );
    let result = call_plot_builtin("ma_plot", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_ma_plot_wrong_type() {
    let result = call_plot_builtin("ma_plot", vec![Value::Int(42)]);
    assert!(result.is_err());
}

#[test]
fn test_ma_plot_single_point() {
    let table = make_table(
        vec!["baseMean", "log2fc"],
        vec![vec![Value::Float(100.0), Value::Float(0.5)]],
    );
    let result = call_plot_builtin("ma_plot", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str");
    }
}

// ── Genome track advanced tests ───────────────────────────────

#[test]
fn test_genome_track_with_title() {
    let table = make_table(
        vec!["chrom", "start", "end", "name", "strand"],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Int(1000),
            Value::Int(2000),
            Value::Str("TP53".into()),
            Value::Str("+".into()),
        ]],
    );
    let opts = Value::Record(
        (HashMap::from([("title".into(), Value::Str("Gene Features".into()))])).into(),
    );
    let result = call_plot_builtin("genome_track", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("Gene Features"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_genome_track_no_name_no_strand() {
    // Minimal table: only chrom, start, end
    let table = make_table(
        vec!["chrom", "start", "end"],
        vec![
            vec![Value::Str("chr1".into()), Value::Int(100), Value::Int(200)],
            vec![Value::Str("chr1".into()), Value::Int(300), Value::Int(500)],
        ],
    );
    let result = call_plot_builtin("genome_track", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("<rect"));
        // No arrows since no strand column
        assert!(!s.contains("<polygon"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_genome_track_many_features() {
    let mut rows = Vec::new();
    for i in 0..20 {
        rows.push(vec![
            Value::Str("chr1".into()),
            Value::Int(i * 100),
            Value::Int(i * 100 + 80),
            Value::Str(format!("gene_{i}")),
            Value::Str(if i % 2 == 0 { "+" } else { "-" }.into()),
        ]);
    }
    let table = make_table(vec!["chrom", "start", "end", "name", "strand"], rows);
    let result = call_plot_builtin("genome_track", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        // Should have 20 features
        let rect_count = s.matches("<rect").count();
        assert!(rect_count >= 20, "expected 20+ rects, got {rect_count}");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_genome_track_single_feature() {
    let table = make_table(
        vec!["chrom", "start", "end"],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Int(50),
            Value::Int(150),
        ]],
    );
    let result = call_plot_builtin("genome_track", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_genome_track_custom_dimensions() {
    let table = make_table(
        vec!["chrom", "start", "end"],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Int(0),
            Value::Int(1000),
        ]],
    );
    let opts = Value::Record(
        (HashMap::from([
            ("width".into(), Value::Float(1200.0)),
            ("height".into(), Value::Float(400.0)),
        ]))
        .into(),
    );
    let result = call_plot_builtin("genome_track", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("1200"));
        assert!(s.contains("400"));
    } else {
        panic!("expected Str");
    }
}

// ── Histogram option tests ────────────────────────────────────

#[test]
fn test_histogram_custom_bins() {
    let list = Value::List(
        (0..100)
            .map(|i| Value::Float(i as f64))
            .collect::<Vec<_>>()
            .into(),
    );
    let opts = Value::Record((HashMap::from([("bins".into(), Value::Int(5))])).into());
    let result = call_plot_builtin("histogram", vec![list, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        // Count the bars themselves. The background and the themed panel are
        // rects too, so a bare `<rect` count measures chrome as well as data.
        let bar_count = s.matches(r##"fill="#595959""##).count();
        assert_eq!(bar_count, 5, "expected 5 bars for 5 bins, got {bar_count}");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_histogram_with_title() {
    let list = Value::List((vec![Value::Float(1.0), Value::Float(2.0), Value::Float(3.0)]).into());
    let opts = Value::Record(
        (HashMap::from([("title".into(), Value::Str("My Histogram".into()))])).into(),
    );
    let result = call_plot_builtin("histogram", vec![list, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("My Histogram"));
    } else {
        panic!("expected Str");
    }
}

// ── Plot option tests ─────────────────────────────────────────

#[test]
fn test_plot_with_title_and_labels() {
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(3.0), Value::Float(4.0)],
        ],
    );
    let opts =
        Value::Record((HashMap::from([("title".into(), Value::Str("Test Plot".into()))])).into());
    let result = call_plot_builtin("plot", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("Test Plot"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_plot_custom_dimensions() {
    let table = make_table(
        vec!["x", "y"],
        vec![vec![Value::Float(1.0), Value::Float(2.0)]],
    );
    let opts = Value::Record(
        (HashMap::from([
            ("width".into(), Value::Float(400.0)),
            ("height".into(), Value::Float(300.0)),
        ]))
        .into(),
    );
    let result = call_plot_builtin("plot", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("400"));
        assert!(s.contains("300"));
    } else {
        panic!("expected Str");
    }
}

// ── normalize_plot_args tests ─────────────────────────────────

#[test]
fn test_plot_with_data_key_record() {
    // {data: table, title: "..."} calling convention
    let table = make_table(
        vec!["x", "y"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(3.0), Value::Float(4.0)],
        ],
    );
    let input = Value::Record(
        (HashMap::from([
            ("data".into(), table),
            ("title".into(), Value::Str("From Record".into())),
        ]))
        .into(),
    );
    let result = call_plot_builtin("plot", vec![input]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        assert!(s.contains("From Record"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_histogram_with_values_key_record() {
    // {values: list, bins: 5} calling convention
    let list = Value::List(
        (vec![
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(4.0),
            Value::Float(5.0),
        ])
        .into(),
    );
    let input = Value::Record(
        (HashMap::from([("values".into(), list), ("bins".into(), Value::Int(3))])).into(),
    );
    let result = call_plot_builtin("histogram", vec![input]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str");
    }
}

// ── save_plot alias test ──────────────────────────────────────

#[test]
fn test_save_plot_alias() {
    let svg = Value::Str("<svg></svg>".into());
    let dir = std::env::temp_dir();
    let path = dir.join("bl_test_save_plot_alias.svg");
    let result = call_plot_builtin(
        "save_plot",
        vec![svg, Value::Str(path.to_string_lossy().into())],
    )
    .unwrap();
    assert!(matches!(result, Value::Str(_)));
    let content = std::fs::read_to_string(&path).unwrap();
    assert_eq!(content, "<svg></svg>");
    let _ = std::fs::remove_file(path);
}

// ── Volcano advanced tests ────────────────────────────────────

#[test]
fn test_volcano_with_custom_thresholds() {
    let table = make_table(
        vec!["log2fc", "pvalue"],
        vec![
            vec![Value::Float(5.0), Value::Float(0.0001)],
            vec![Value::Float(-3.0), Value::Float(0.0005)],
            vec![Value::Float(0.1), Value::Float(0.5)],
        ],
    );
    let opts = Value::Record(
        (HashMap::from([
            ("fc_threshold".into(), Value::Float(2.0)),
            ("p_threshold".into(), Value::Float(0.001)),
        ]))
        .into(),
    );
    let result = call_plot_builtin("volcano", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        // Significant points should be colored (not #999)
        assert!(s.contains("#e15759") || s.contains("#4e79a7"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_volcano_many_points() {
    let mut rows = Vec::new();
    for i in 0..200 {
        rows.push(vec![
            Value::Float((i as f64 - 100.0) * 0.05),
            Value::Float(10.0f64.powf(-(i as f64 * 0.02))),
        ]);
    }
    let table = make_table(vec!["log2fc", "pvalue"], rows);
    let result = call_plot_builtin("volcano", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        let circle_count = s.matches("<circle").count();
        assert_eq!(circle_count, 200, "expected 200 circles");
    } else {
        panic!("expected Str");
    }
}

// ── Heatmap advanced tests ────────────────────────────────────

#[test]
fn test_heatmap_with_title() {
    let table = make_table(
        vec!["a", "b"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(3.0), Value::Float(4.0)],
        ],
    );
    let opts = Value::Record(
        (HashMap::from([("title".into(), Value::Str("Expression Heatmap".into()))])).into(),
    );
    let result = call_plot_builtin("heatmap", vec![table, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("Expression Heatmap"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_heatmap_large_matrix() {
    let mut rows = Vec::new();
    for i in 0..20 {
        let mut row = Vec::new();
        for j in 0..10 {
            row.push(Value::Float((i * 10 + j) as f64));
        }
        rows.push(row);
    }
    let _cols: Vec<&str> = (0..10).map(|_| "c").collect();
    let col_names: Vec<&str> = vec!["c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9"];
    let table = make_table(col_names, rows);
    let result = call_plot_builtin("heatmap", vec![table]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        // 20 rows × 10 cols = 200 cells
        let rect_count = s.matches("<rect").count();
        assert!(rect_count >= 200, "expected 200+ rects, got {rect_count}");
    } else {
        panic!("expected Str");
    }
}

// ── Unknown builtin ─────────────────────────────────────────────

#[test]
fn test_unknown_plot_builtin() {
    let result = call_plot_builtin("nonexistent", vec![]);
    assert!(result.is_err());
}
