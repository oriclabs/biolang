//! Renderer-independent genomic association and mutation-distance geometry.

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

fn association_input() -> Value {
    Value::Table(Table::new(
        vec![
            "chrom".into(),
            "pos".into(),
            "pvalue".into(),
            "lead".into(),
            "variant".into(),
        ],
        vec![
            vec![
                Value::Str("chr2".into()),
                Value::Float(200.0),
                Value::Float(0.01),
                Value::Bool(false),
                Value::Str("rs1".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(100.0),
                Value::Float(1e-9),
                Value::Bool(true),
                Value::Str("rsLead".into()),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(50.0),
                Value::Float(0.5),
                Value::Bool(false),
                Value::Str("rs3".into()),
            ],
        ],
    ))
}

#[test]
fn manhattan_spec_freezes_first_observed_layout_thresholds_and_exact_replay() {
    let common = vec![
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("Association scan".into())),
        ("highlight", Value::Str("lead".into())),
        ("label", Value::Str("variant".into())),
    ];
    let direct =
        call_bio_plots_builtin("manhattan", vec![association_input(), opts(common.clone())])
            .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("manhattan", vec![association_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "manhattan"));
    let Some(Value::Table(chromosomes)) = map.get("chromosomes") else {
        panic!("chromosome layout is not a Table")
    };
    assert_eq!(chromosomes.rows[0][1], Value::Str("chr2".into()));
    assert_eq!(chromosomes.rows[1][1], Value::Str("chr1".into()));
    assert_eq!(chromosomes.rows[1][2], Value::Float(204.0));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("Manhattan geometry is not a Table")
    };
    assert_eq!(data.rows[1][4], Value::Float(304.0));
    assert_eq!(data.rows[1][7], Value::Bool(true));
    assert_eq!(data.rows[1][8], Value::Bool(true));
    assert_eq!(data.rows[1][9], Value::Str("rsLead".into()));
    assert_eq!(direct, render(specification));
}

#[test]
fn genetic_qq_spec_freezes_positions_envelope_lambda_and_exact_replay() {
    let input = Value::List(
        vec![
            Value::Float(0.5),
            Value::Float(0.01),
            Value::Float(1e-5),
            Value::Float(0.2),
        ]
        .into(),
    );
    let common = vec![
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("P-value calibration".into())),
        ("envelope", Value::Bool(true)),
    ];
    let direct =
        call_bio_plots_builtin("qq_plot", vec![input.clone(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification = call_bio_plots_builtin("qq_plot", vec![input, opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "genetic_qq"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("genetic Q-Q geometry is not a Table")
    };
    assert_eq!(data.rows[0][0], Value::Int(1));
    assert_eq!(data.rows[0][1], Value::Float(1e-5));
    assert_eq!(data.rows[0][2], Value::Float(0.125));
    assert!(data.rows[0][5].as_float().unwrap() <= data.rows[0][6].as_float().unwrap());
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("genetic Q-Q options are not a Record")
    };
    assert!(options
        .get("lambda_gc")
        .and_then(Value::as_float)
        .is_some_and(|value| value.is_finite() && value > 0.0));
    assert_eq!(direct, render(specification));
}

fn rainfall_input() -> Value {
    Value::Table(Table::new(
        vec!["chrom".into(), "pos".into()],
        vec![
            vec![Value::Str("chr2".into()), Value::Float(100.0)],
            vec![Value::Str("chr1".into()), Value::Float(10.0)],
            vec![Value::Str("chr2".into()), Value::Float(100.0)],
            vec![Value::Str("chr2".into()), Value::Float(400.0)],
            vec![Value::Str("chr1".into()), Value::Float(30.0)],
        ],
    ))
}

#[test]
fn rainfall_spec_freezes_sort_distances_duplicates_and_exact_replay() {
    let common = vec![("theme", Value::Str("publication".into()))];
    let direct =
        call_bio_plots_builtin("rainfall", vec![rainfall_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("rainfall", vec![rainfall_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "rainfall"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("rainfall geometry is not a Table")
    };
    assert_eq!(data.num_rows(), 3);
    assert_eq!(data.rows[0][3], Value::Str("chr2".into()));
    assert_eq!(data.rows[0][7], Value::Float(0.0));
    assert_eq!(data.rows[0][8], Value::Float(1.0));
    assert_eq!(data.rows[0][9], Value::Float(0.0));
    assert_eq!(data.rows[0][10], Value::Bool(true));
    assert_eq!(data.rows[2][3], Value::Str("chr1".into()));
    assert_eq!(data.rows[2][7], Value::Float(20.0));
    assert_eq!(direct, render(specification));
}

#[test]
fn genomic_specs_reject_malformed_geometry_and_html_keeps_canvas_fallback() {
    let manhattan = call_bio_plots_builtin(
        "manhattan",
        vec![
            association_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Value::Str(html) = call_plot_builtin(
        "render_plot",
        vec![
            manhattan.clone(),
            opts(vec![("format", Value::Str("html".into()))]),
        ],
    )
    .unwrap() else {
        panic!("expected standalone HTML")
    };
    assert!(html.contains("<svg"));
    assert!(html.contains("<canvas"));

    let qq = call_bio_plots_builtin(
        "qq_plot",
        vec![
            Value::List(vec![Value::Float(0.1), Value::Float(0.5)].into()),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let rainfall = call_bio_plots_builtin(
        "rainfall",
        vec![
            rainfall_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    for specification in [manhattan, qq, rainfall] {
        let mut broken = record(&specification).clone();
        broken.insert("data".into(), Value::Table(Table::empty()));
        assert!(call_plot_builtin("render_plot", vec![Value::Record(broken.into())]).is_err());
    }
}

#[test]
fn invalid_association_inputs_fail_without_silently_drawing_bad_values() {
    let bad_manhattan = Value::Table(Table::new(
        vec!["chrom".into(), "pos".into(), "pvalue".into()],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(-1.0),
            Value::Float(0.2),
        ]],
    ));
    assert!(call_bio_plots_builtin("manhattan", vec![bad_manhattan]).is_err());
    assert!(call_bio_plots_builtin(
        "qq_plot",
        vec![
            Value::List(vec![Value::Float(0.1)].into()),
            opts(vec![("confidence", Value::Float(1.0))]),
        ],
    )
    .is_err());
    assert!(call_bio_plots_builtin(
        "rainfall",
        vec![
            rainfall_input(),
            opts(vec![("duplicate_floor", Value::Float(0.0))]),
        ],
    )
    .is_err());
}

#[test]
fn dense_genomic_plots_keep_all_spec_rows_and_bound_svg_elements() {
    const POINTS: usize = 25_000;
    let association = Value::Table(Table::new(
        vec!["chrom".into(), "pos".into(), "pvalue".into()],
        (0..POINTS)
            .map(|index| {
                vec![
                    Value::Str(format!("chr{}", index % 4 + 1)),
                    Value::Float((index / 4 + 1) as f64),
                    Value::Float((index as f64 + 1.0) / (POINTS as f64 + 1.0)),
                ]
            })
            .collect(),
    ));
    let manhattan = call_bio_plots_builtin(
        "manhattan",
        vec![
            association,
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Some(Value::Table(manhattan_rows)) = record(&manhattan).get("data") else {
        panic!("Manhattan geometry is not a Table")
    };
    assert_eq!(manhattan_rows.num_rows(), POINTS);
    let Value::Str(manhattan_svg) = render(manhattan) else {
        panic!("expected Manhattan SVG")
    };
    assert_eq!(manhattan_svg.matches("<image").count(), 1);
    assert!(manhattan_svg.matches("<circle").count() < 10);

    let qq = call_bio_plots_builtin(
        "qq_plot",
        vec![
            Value::List(
                (1..=5_000)
                    .map(|index| Value::Float(index as f64 / 5_001.0))
                    .collect::<Vec<_>>()
                    .into(),
            ),
            opts(vec![
                ("format", Value::Str("spec".into())),
                ("raster", Value::Bool(true)),
            ]),
        ],
    )
    .unwrap();
    let Value::Str(qq_svg) = render(qq) else {
        panic!("expected genetic Q-Q SVG")
    };
    assert_eq!(qq_svg.matches("<image").count(), 1);

    let rainfall = Value::Table(Table::new(
        vec!["chrom".into(), "pos".into()],
        (0..POINTS)
            .map(|index| vec![Value::Str("chr1".into()), Value::Float((index + 1) as f64)])
            .collect(),
    ));
    let rainfall_spec = call_bio_plots_builtin(
        "rainfall",
        vec![rainfall, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(rainfall_rows)) = record(&rainfall_spec).get("data") else {
        panic!("rainfall geometry is not a Table")
    };
    assert_eq!(rainfall_rows.num_rows(), POINTS - 1);
    let Value::Str(rainfall_svg) = render(rainfall_spec) else {
        panic!("expected rainfall SVG")
    };
    assert_eq!(rainfall_svg.matches("<image").count(), 1);
}
