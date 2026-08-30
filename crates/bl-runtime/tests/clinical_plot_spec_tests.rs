//! Survival, diagnostic and effect-size plots expose their analytical geometry.

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

fn survival_input() -> Value {
    Value::Table(Table::new(
        vec!["time".into(), "event".into(), "arm".into()],
        vec![
            vec![
                Value::Float(1.0),
                Value::Int(1),
                Value::Str("control".into()),
            ],
            vec![
                Value::Float(2.0),
                Value::Int(0),
                Value::Str("control".into()),
            ],
            vec![
                Value::Float(2.0),
                Value::Int(1),
                Value::Str("control".into()),
            ],
            vec![
                Value::Float(1.0),
                Value::Int(0),
                Value::Str("treated".into()),
            ],
            vec![
                Value::Float(3.0),
                Value::Int(1),
                Value::Str("treated".into()),
            ],
            vec![
                Value::Float(4.0),
                Value::Int(0),
                Value::Str("treated".into()),
            ],
        ],
    ))
}

#[test]
fn kaplan_meier_spec_freezes_tied_risk_sets_greenwood_error_and_exact_replay() {
    let common = vec![
        ("group", Value::Str("arm".into())),
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("Time to recovery".into())),
    ];
    let direct =
        call_bio_plots_builtin("kaplan_meier", vec![survival_input(), opts(common.clone())])
            .unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("kaplan_meier", vec![survival_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "survival"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("survival geometry is not a Table")
    };
    // Control: initial row, event at 1, then one event and one censor tied at 2.
    assert_eq!(data.rows[2][4], Value::Int(2));
    assert_eq!(data.rows[2][5], Value::Int(1));
    assert_eq!(data.rows[2][6], Value::Int(1));
    assert!((data.rows[2][7].as_float().unwrap() - 1.0 / 3.0).abs() < 1e-12);
    assert!(data.rows[2][8].as_float().unwrap().is_finite());
    assert_eq!(data.columns[9], "confidence_lower");
    assert_eq!(data.columns[10], "confidence_upper");
    let event_one = &data.rows[1];
    let expected_lower = (2.0 / 3.0) * (-1.959_963_984_540_054_f64 * (1.0 / 6.0_f64).sqrt()).exp();
    assert!((event_one[9].as_float().unwrap() - expected_lower).abs() < 1e-12);
    assert_eq!(event_one[10], Value::Float(1.0));
    assert_eq!(direct, render(specification));
}

#[test]
fn kaplan_meier_renders_log_confidence_band_risk_table_p_value_and_safe_palette() {
    let specification = call_bio_plots_builtin(
        "kaplan_meier",
        vec![
            survival_input(),
            opts(vec![
                ("group", Value::Str("arm".into())),
                ("confidence", Value::Bool(true)),
                ("risk_table", Value::Bool(true)),
                (
                    "risk_times",
                    Value::List(
                        vec![Value::Float(0.0), Value::Float(2.0), Value::Float(4.0)].into(),
                    ),
                ),
                ("p_value", Value::Float(0.0123)),
                ("legend_title", Value::Str("Treatment".into())),
                (
                    "colors",
                    Value::List(
                        vec![Value::Str("#1C86EE".into()), Value::Str("#EE7AE9".into())].into(),
                    ),
                ),
                ("format", Value::Str("spec".into())),
            ]),
        ],
    )
    .unwrap();
    let Value::Str(svg) = render(specification) else {
        panic!("expected SVG")
    };
    assert!(svg.contains("fill-opacity=\"0.14\""));
    assert!(svg.contains("Number at risk"));
    assert!(svg.contains("Log-rank p = 0.012"));
    assert!(svg.contains("Treatment"));
    assert!(svg.contains("#1C86EE"));
    assert!(svg.contains("#EE7AE9"));
}

#[test]
fn kaplan_meier_ggplot_theme_matches_r_hue_palette_and_numeric_strata_order() {
    let input = Value::Table(Table::new(
        vec!["time".into(), "event".into(), "nodes".into()],
        vec![
            vec![Value::Float(1.0), Value::Int(1), Value::Int(10)],
            vec![Value::Float(2.0), Value::Int(0), Value::Int(2)],
            vec![Value::Float(3.0), Value::Int(1), Value::Int(1)],
        ],
    ));
    let specification = call_bio_plots_builtin(
        "kaplan_meier",
        vec![
            input,
            opts(vec![
                ("group", Value::Str("nodes".into())),
                ("theme", Value::Str("ggplot".into())),
                ("format", Value::Str("spec".into())),
            ]),
        ],
    )
    .unwrap();
    let map = record(&specification);
    let Some(Value::Table(groups)) = map.get("groups") else {
        panic!("survival groups are not a Table")
    };
    assert_eq!(groups.rows[0][1], Value::Str("1".into()));
    assert_eq!(groups.rows[1][1], Value::Str("2".into()));
    assert_eq!(groups.rows[2][1], Value::Str("10".into()));

    let Value::Str(svg) = render(specification) else {
        panic!("expected SVG")
    };
    // R: scales::hue_pal()(3), assigned to levels 1, 2 and 10.
    assert!(svg.contains("#f8766d"));
    assert!(svg.contains("#00ba38"));
    assert!(svg.contains("#619cff"));
    let first_curve = svg
        .split("<path")
        .find(|fragment| {
            fragment.contains("stroke=\"#f8766d\"") && fragment.contains("stroke-width=\"2\"")
        })
        .expect("first ggplot survival curve");
    // The ggplot scale expansion keeps S(0)=1 away from both the left and top
    // axes. Without it this path began exactly at M 62.00 52.00.
    assert!(first_curve.contains("M 82.36 67.27"), "{first_curve}");
}

#[test]
fn kaplan_meier_stops_each_curve_at_its_own_last_follow_up() {
    let plotted = call_bio_plots_builtin(
        "kaplan_meier",
        vec![
            survival_input(),
            opts(vec![
                ("group", Value::Str("arm".into())),
                ("confidence", Value::Bool(true)),
                (
                    "colors",
                    Value::List(
                        vec![Value::Str("#112233".into()), Value::Str("#445566".into())].into(),
                    ),
                ),
            ]),
        ],
    )
    .unwrap();
    let Value::Str(svg) = plotted else {
        panic!("expected SVG")
    };
    let control_path = svg
        .split("<path")
        .find(|fragment| {
            fragment.contains("stroke=\"#112233\"") && fragment.contains("stroke-width=\"2\"")
        })
        .expect("control survival path");
    // With the default 640 px canvas, time 2 maps to x=286 and the global
    // maximum time 4 maps to x=510. The shorter control group must stop at 286.
    assert!(control_path.contains("H 286.00"), "{control_path}");
    assert!(!control_path.contains("H 510.00"), "{control_path}");
    let control_band = svg
        .split("<polygon")
        .find(|fragment| fragment.contains("fill=\"#112233\""))
        .expect("control confidence band");
    assert!(!control_band.contains("510.00,"), "{control_band}");
}

#[test]
fn kaplan_meier_rejects_palette_values_that_can_escape_svg_attributes() {
    let result = call_bio_plots_builtin(
        "kaplan_meier",
        vec![
            survival_input(),
            opts(vec![
                ("group", Value::Str("arm".into())),
                (
                    "colors",
                    Value::List(
                        vec![
                            Value::Str("red\" onload=\"alert(1)".into()),
                            Value::Str("#EE7AE9".into()),
                        ]
                        .into(),
                    ),
                ),
            ]),
        ],
    );
    assert!(result.is_err());
}

fn forest_input() -> Value {
    Value::Table(Table::new(
        vec![
            "study".into(),
            "ratio".into(),
            "lower".into(),
            "upper".into(),
            "precision".into(),
        ],
        vec![
            vec![
                Value::Str("Trial A".into()),
                Value::Float(0.72),
                Value::Float(0.51),
                Value::Float(1.01),
                Value::Float(4.0),
            ],
            vec![
                Value::Str("Trial B".into()),
                Value::Float(1.18),
                Value::Float(0.94),
                Value::Float(1.49),
                Value::Float(9.0),
            ],
        ],
    ))
}

#[test]
fn forest_spec_freezes_intervals_weights_log_domain_and_exact_replay() {
    let common = vec![
        ("label", Value::Str("study".into())),
        ("estimate", Value::Str("ratio".into())),
        ("weight", Value::Str("precision".into())),
        ("scale", Value::Str("log".into())),
        ("theme", Value::Str("publication".into())),
    ];
    let direct =
        call_bio_plots_builtin("forest_plot", vec![forest_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("forest_plot", vec![forest_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "forest"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("forest geometry is not a Table")
    };
    assert_eq!(data.num_rows(), 2);
    assert_eq!(data.rows[0][3], Value::Float(0.72));
    assert_eq!(data.rows[1][6], Value::Float(9.0));
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("forest options are not a Record")
    };
    assert_eq!(options.get("reference"), Some(&Value::Float(1.0)));
    assert!(options
        .get("display_min")
        .and_then(Value::as_float)
        .is_some_and(f64::is_finite));
    assert_eq!(direct, render(specification));
}

#[test]
fn forest_rejects_reversed_intervals_and_non_positive_log_bounds() {
    let reversed = Value::Table(Table::new(
        vec![
            "label".into(),
            "estimate".into(),
            "lower".into(),
            "upper".into(),
        ],
        vec![vec![
            Value::Str("bad".into()),
            Value::Float(1.0),
            Value::Float(1.2),
            Value::Float(1.4),
        ]],
    ));
    assert!(call_bio_plots_builtin("forest_plot", vec![reversed]).is_err());
    let non_positive = Value::Table(Table::new(
        vec![
            "label".into(),
            "estimate".into(),
            "lower".into(),
            "upper".into(),
        ],
        vec![vec![
            Value::Str("bad".into()),
            Value::Float(1.0),
            Value::Float(0.0),
            Value::Float(1.4),
        ]],
    ));
    assert!(call_bio_plots_builtin(
        "forest_plot",
        vec![
            non_positive,
            opts(vec![("scale", Value::Str("log".into()))])
        ],
    )
    .is_err());
}

fn roc_input() -> Value {
    Value::Table(Table::new(
        vec!["score".into(), "label".into()],
        vec![
            vec![Value::Float(0.9), Value::Int(1)],
            vec![Value::Float(0.8), Value::Int(1)],
            vec![Value::Float(0.8), Value::Int(0)],
            vec![Value::Float(0.1), Value::Int(0)],
        ],
    ))
}

#[test]
fn roc_spec_groups_tied_thresholds_freezes_confusion_counts_and_replays_exactly() {
    let common = vec![
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("Classifier accuracy".into())),
    ];
    let direct =
        call_bio_plots_builtin("roc_curve", vec![roc_input(), opts(common.clone())]).unwrap();
    let mut spec_options = common;
    spec_options.push(("format", Value::Str("spec".into())));
    let specification =
        call_bio_plots_builtin("roc_curve", vec![roc_input(), opts(spec_options)]).unwrap();
    let map = record(&specification);
    assert!(matches!(map.get("kind"), Some(Value::Str(kind)) if kind == "roc"));
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("ROC geometry is not a Table")
    };
    assert_eq!(data.num_rows(), 4); // initial point plus three distinct scores
    assert_eq!(data.rows[2][1], Value::Float(0.8));
    assert_eq!(data.rows[2][4], Value::Int(2));
    assert_eq!(data.rows[2][5], Value::Int(1));
    assert_eq!(data.rows[2][6], Value::Int(1));
    assert_eq!(data.rows[2][7], Value::Int(0));
    let Some(Value::Record(options)) = map.get("options") else {
        panic!("ROC options are not a Record")
    };
    assert!((options.get("auc").and_then(Value::as_float).unwrap() - 0.875).abs() < 1e-12);
    assert_eq!(direct, render(specification));
}

#[test]
fn roc_precomputed_points_are_preserved_and_non_monotone_curves_are_rejected() {
    let curve = Value::Table(Table::new(
        vec!["fpr".into(), "tpr".into()],
        vec![
            vec![Value::Float(0.0), Value::Float(0.0)],
            vec![Value::Float(0.2), Value::Float(0.7)],
            vec![Value::Float(1.0), Value::Float(1.0)],
        ],
    ));
    let specification = call_bio_plots_builtin(
        "roc_curve",
        vec![curve, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let map = record(&specification);
    let Some(Value::Table(data)) = map.get("data") else {
        panic!("ROC geometry is not a Table")
    };
    assert_eq!(data.rows[1][1], Value::Nil);
    assert_eq!(data.rows[1][2], Value::Float(0.2));
    assert_eq!(data.rows[1][3], Value::Float(0.7));
    assert_eq!(data.rows[1][4], Value::Nil);

    let non_monotone = Value::Table(Table::new(
        vec!["fpr".into(), "tpr".into()],
        vec![
            vec![Value::Float(0.0), Value::Float(0.0)],
            vec![Value::Float(0.4), Value::Float(0.8)],
            vec![Value::Float(0.3), Value::Float(0.9)],
        ],
    ));
    assert!(call_bio_plots_builtin("roc_curve", vec![non_monotone]).is_err());
}

#[test]
fn roc_requires_both_classes() {
    let one_class = Value::Table(Table::new(
        vec!["score".into(), "label".into()],
        vec![
            vec![Value::Float(0.9), Value::Int(1)],
            vec![Value::Float(0.2), Value::Int(1)],
        ],
    ));
    assert!(call_bio_plots_builtin("roc_curve", vec![one_class]).is_err());
}

#[test]
fn malformed_clinical_specs_are_rejected_and_html_contains_canvas_fallback() {
    let survival = call_bio_plots_builtin(
        "kaplan_meier",
        vec![
            survival_input(),
            opts(vec![("format", Value::Str("spec".into()))]),
        ],
    )
    .unwrap();
    let Value::Str(html) = call_plot_builtin(
        "render_plot",
        vec![
            survival.clone(),
            opts(vec![("format", Value::Str("html".into()))]),
        ],
    )
    .unwrap() else {
        panic!("expected standalone HTML")
    };
    assert!(html.contains("<svg"));
    assert!(html.contains("<canvas"));

    for specification in [
        survival,
        call_bio_plots_builtin(
            "forest_plot",
            vec![
                forest_input(),
                opts(vec![
                    ("label", Value::Str("study".into())),
                    ("estimate", Value::Str("ratio".into())),
                    ("format", Value::Str("spec".into())),
                ]),
            ],
        )
        .unwrap(),
        call_bio_plots_builtin(
            "roc_curve",
            vec![
                roc_input(),
                opts(vec![("format", Value::Str("spec".into()))]),
            ],
        )
        .unwrap(),
    ] {
        let mut broken = record(&specification).clone();
        broken.insert("data".into(), Value::Table(Table::empty()));
        assert!(call_plot_builtin("render_plot", vec![Value::Record(broken.into())]).is_err());
    }
}

#[test]
fn dense_survival_and_roc_specs_remain_linear_and_use_bounded_svg_elements() {
    const OBSERVATIONS: usize = 5_000;
    let survival = Value::Table(Table::new(
        vec!["time".into(), "event".into()],
        (0..OBSERVATIONS)
            .map(|index| {
                vec![
                    Value::Float(index as f64 + 1.0),
                    Value::Int((index % 3 != 0) as i64),
                ]
            })
            .collect(),
    ));
    let survival_spec = call_bio_plots_builtin(
        "kaplan_meier",
        vec![survival, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(steps)) = record(&survival_spec).get("data") else {
        panic!("survival geometry is not a Table")
    };
    assert_eq!(steps.num_rows(), OBSERVATIONS + 1);
    let Value::Str(survival_svg) = render(survival_spec) else {
        panic!("expected survival SVG")
    };
    assert!(survival_svg.matches("<path").count() < 20);
    assert!(survival_svg.len() < 1_000_000);

    let roc = Value::Table(Table::new(
        vec!["score".into(), "label".into()],
        (0..OBSERVATIONS)
            .map(|index| {
                vec![
                    Value::Float(index as f64 / OBSERVATIONS as f64),
                    Value::Int((index % 2) as i64),
                ]
            })
            .collect(),
    ));
    let roc_spec = call_bio_plots_builtin(
        "roc_curve",
        vec![roc, opts(vec![("format", Value::Str("spec".into()))])],
    )
    .unwrap();
    let Some(Value::Table(points)) = record(&roc_spec).get("data") else {
        panic!("ROC geometry is not a Table")
    };
    assert_eq!(points.num_rows(), OBSERVATIONS + 1);
    let Value::Str(roc_svg) = render(roc_spec) else {
        panic!("expected ROC SVG")
    };
    assert_eq!(roc_svg.matches("<polyline").count(), 1);
    assert!(roc_svg.len() < 1_000_000);
}
