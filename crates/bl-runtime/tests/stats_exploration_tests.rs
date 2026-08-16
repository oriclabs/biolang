use bl_core::sparse_matrix::SparseMatrix;
use bl_core::value::{Table, Value};
use bl_runtime::stats::call_stats_builtin;

fn numbers(values: &[f64]) -> Value {
    Value::List(
        values
            .iter()
            .copied()
            .map(Value::Float)
            .collect::<Vec<_>>()
            .into(),
    )
}

fn strings(values: &[&str]) -> Value {
    Value::List(
        values
            .iter()
            .map(|value| Value::Str((*value).into()))
            .collect::<Vec<_>>()
            .into(),
    )
}

fn field<'a>(value: &'a Value, name: &str) -> &'a Value {
    let Value::Record(record) = value else {
        panic!("expected Record, got {value:?}");
    };
    record
        .get(name)
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn float_field(value: &Value, name: &str) -> f64 {
    match field(value, name) {
        Value::Float(value) => *value,
        Value::Int(value) => *value as f64,
        other => panic!("expected numeric field {name}, got {other:?}"),
    }
}

#[test]
fn numeric_exploration_reports_full_data_provenance_and_robust_choice() {
    let input = Value::List(
        vec![
            Value::Float(12.1),
            Value::Float(12.4),
            Value::Nil,
            Value::Float(12.8),
            Value::Float(13.0),
            Value::Float(13.2),
            Value::Float(13.5),
            Value::Float(29.0),
            Value::Float(f64::NAN),
        ]
        .into(),
    );
    let report = call_stats_builtin("stats_explore", vec![input]).unwrap();
    let data = field(&report, "data");
    assert_eq!(field(data, "received"), &Value::Int(9));
    assert_eq!(field(data, "used"), &Value::Int(7));
    assert_eq!(field(data, "missing"), &Value::Int(1));
    assert_eq!(field(data, "non_finite"), &Value::Int(1));

    let suggestion = field(&report, "suggestion");
    assert_eq!(field(suggestion, "center"), &Value::Str("median".into()));
    assert_eq!(field(suggestion, "spread"), &Value::Str("IQR".into()));

    let Value::List(outliers) = field(&report, "outliers") else {
        panic!("outliers should be a List");
    };
    assert_eq!(outliers.len(), 1);
    assert_eq!(field(&outliers[0], "index"), &Value::Int(7));
}

#[test]
fn numeric_exploration_uses_sample_variance_and_type_seven_quantiles() {
    let report = call_stats_builtin("stats_explore", vec![numbers(&[1.0, 2.0, 3.0, 4.0])]).unwrap();
    let summary = field(&report, "summary");
    assert!((float_field(summary, "mean") - 2.5).abs() < 1e-12);
    assert!((float_field(summary, "variance") - 1.666_666_666_666_666_7).abs() < 1e-12);
    assert!((float_field(summary, "q1") - 1.75).abs() < 1e-12);
    assert!((float_field(summary, "q3") - 3.25).abs() < 1e-12);
}

#[test]
fn grouped_exploration_preserves_first_seen_group_order() {
    let report = call_stats_builtin(
        "stats_compare",
        vec![
            numbers(&[4.0, 5.0, 8.0, 9.0]),
            strings(&["B", "A", "B", "A"]),
        ],
    )
    .unwrap();
    let Value::List(names) = field(&report, "group_names") else {
        panic!("group_names should be List");
    };
    assert_eq!(names[0], Value::Str("B".into()));
    assert_eq!(names[1], Value::Str("A".into()));
}

#[test]
fn relationship_uses_pairwise_complete_observations() {
    let x = Value::List(vec![Value::Int(1), Value::Nil, Value::Int(3), Value::Int(4)].into());
    let y = Value::List(vec![Value::Int(2), Value::Int(999), Value::Int(6), Value::Int(8)].into());
    let report = call_stats_builtin("stats_relationship", vec![x, y]).unwrap();
    assert_eq!(field(&report, "complete_pairs"), &Value::Int(3));
    assert_eq!(field(&report, "excluded_pairs"), &Value::Int(1));
    assert!((float_field(&report, "pearson") - 1.0).abs() < 1e-12);
    assert!((float_field(&report, "spearman") - 1.0).abs() < 1e-12);
}

#[test]
fn categorical_exploration_reports_tied_modes() {
    let report = call_stats_builtin(
        "stats_categories",
        vec![strings(&[
            "treated", "control", "treated", "control", "unknown",
        ])],
    )
    .unwrap();
    assert_eq!(field(&report, "n_levels"), &Value::Int(3));
    let Value::List(modes) = field(&report, "modes") else {
        panic!("modes should be List");
    };
    assert_eq!(modes.len(), 2);
    assert_eq!(modes[0], Value::Str("treated".into()));
    assert_eq!(modes[1], Value::Str("control".into()));
}

#[test]
fn distribution_plot_discloses_visual_sampling_but_uses_full_histogram() {
    let values = (0..100).map(|value| Value::Int(value)).collect::<Vec<_>>();
    let options = Value::Record(HashMap::from([("max_points".to_string(), Value::Int(10))]).into());
    let plot = call_stats_builtin(
        "stats_distribution_plot",
        vec![Value::List(values.into()), options],
    )
    .unwrap();
    let Value::Str(svg) = plot else {
        panic!("plot should be SVG String");
    };
    assert!(svg.starts_with("<svg"));
    assert!(svg.contains("Dots show every 10th observation"));
    assert!(svg.contains("calculations and histogram use all finite values"));
}

#[test]
fn distribution_ascii_is_annotated_and_discloses_exclusions() {
    let input = Value::List(
        vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
            Value::Int(20),
            Value::Nil,
        ]
        .into(),
    );
    let chart = call_stats_builtin("stats_distribution_ascii", vec![input]).unwrap();
    let Value::Str(chart) = chart else {
        panic!("ASCII chart should be a String");
    };
    assert!(chart.contains("Distribution guide (ASCII)"));
    assert!(chart.contains("A=mean M=median"));
    assert!(chart.contains("All 5 finite observations"));
    assert!(chart.contains("1 missing and 0 non-finite excluded"));
    assert!(chart.contains("Tukey review flags 1"));
}

#[test]
fn exploration_rejects_non_numeric_values_instead_of_silently_dropping_them() {
    let error = call_stats_builtin(
        "stats_explore",
        vec![Value::List(
            vec![Value::Int(1), Value::Str("two".into()), Value::Int(3)].into(),
        )],
    )
    .unwrap_err();
    assert!(error.to_string().contains("index 1 is Str"));
}

#[test]
fn preprocessing_distinguishes_observed_zeros_from_a_model_diagnosis() {
    let options = Value::Record(
        HashMap::from([("data_type".to_string(), Value::Str("counts".into()))]).into(),
    );
    let report = call_stats_builtin(
        "stats_preprocess",
        vec![numbers(&[0.0, 0.0, 0.0, 1.0, 2.0, 40.0]), options],
    )
    .unwrap();
    assert_eq!(field(&report, "automatic_changes"), &Value::Bool(false));
    let Value::List(issues) = field(&report, "issues") else {
        panic!("issues should be List");
    };
    let many_zeros = issues
        .iter()
        .find(|item| field(item, "id") == &Value::Str("many_zeros".into()))
        .expect("many-zero clue should be present");
    assert_eq!(field(many_zeros, "is_diagnosis"), &Value::Bool(false));

    let Value::List(suggestions) = field(&report, "suggestions") else {
        panic!("suggestions should be List");
    };
    assert!(suggestions.iter().any(|item| {
        field(item, "name") == &Value::Str("library-size or exposure normalization".into())
            && field(item, "status") == &Value::Str("requires_more_data".into())
    }));
    assert!(suggestions
        .iter()
        .all(|item| field(item, "automatically_applied") == &Value::Bool(false)));
}

#[test]
fn table_profile_combines_integrity_missingness_and_design_clues() {
    let table = Value::Table(Table::new(
        vec![
            "subject".into(),
            "group".into(),
            "batch".into(),
            "age".into(),
        ],
        vec![
            vec![
                Value::Str("s1".into()),
                Value::Str("A".into()),
                Value::Str("b1".into()),
                Value::Int(30),
            ],
            vec![
                Value::Str("s1".into()),
                Value::Str("A".into()),
                Value::Str("b1".into()),
                Value::Nil,
            ],
            vec![
                Value::Str("s2".into()),
                Value::Str("B".into()),
                Value::Str("b2".into()),
                Value::Int(999),
            ],
            vec![
                Value::Str("s2".into()),
                Value::Str("B".into()),
                Value::Str("b2".into()),
                Value::Int(999),
            ],
        ],
    ));
    let ranges = Value::Record(
        HashMap::from([(
            "age".into(),
            Value::Record(
                HashMap::from([
                    ("min".into(), Value::Int(0)),
                    ("max".into(), Value::Int(120)),
                ])
                .into(),
            ),
        )])
        .into(),
    );
    let options = Value::Record(
        HashMap::from([
            ("subject_column".into(), Value::Str("subject".into())),
            ("group_column".into(), Value::Str("group".into())),
            ("batch_column".into(), Value::Str("batch".into())),
            ("ranges".into(), ranges),
        ])
        .into(),
    );
    let report = call_stats_builtin("stats_profile", vec![table, options]).unwrap();
    assert_eq!(field(&report, "duplicate_rows"), &Value::Int(1));
    let design = field(&report, "design");
    assert_eq!(field(design, "repeated_subjects"), &Value::Int(2));
    let Value::List(issues) = field(&report, "issues") else {
        panic!("issues should be List")
    };
    assert!(issues
        .iter()
        .any(|item| field(item, "id") == &Value::Str("expected_range_violation".into())));
    let missingness = field(&report, "missingness");
    assert_eq!(field(missingness, "complete_rows"), &Value::Int(3));
}

#[test]
fn transform_preview_is_non_mutating_and_reports_scale_change() {
    let report = call_stats_builtin(
        "stats_transform_preview",
        vec![numbers(&[0.0, 1.0, 9.0, 99.0]), Value::Str("log1p".into())],
    )
    .unwrap();
    assert_eq!(field(&report, "input_modified"), &Value::Bool(false));
    assert_eq!(field(&report, "rank_order_preserved"), &Value::Bool(true));
    assert!(float_field(&report, "span_ratio") < 0.1);
    let Value::List(values) = field(&report, "values") else {
        panic!("values should be List")
    };
    assert_eq!(values[0], Value::Float(0.0));
}

#[test]
fn uncertainty_is_seeded_and_supports_group_differences() {
    let options = Value::Record(
        HashMap::from([
            ("statistic".into(), Value::Str("difference_mean".into())),
            ("other".into(), numbers(&[4.0, 5.0, 6.0, 7.0])),
            ("repetitions".into(), Value::Int(500)),
            ("seed".into(), Value::Int(7)),
        ])
        .into(),
    );
    let first = call_stats_builtin(
        "stats_uncertainty",
        vec![numbers(&[1.0, 2.0, 3.0, 4.0]), options.clone()],
    )
    .unwrap();
    let second = call_stats_builtin(
        "stats_uncertainty",
        vec![numbers(&[1.0, 2.0, 3.0, 4.0]), options],
    )
    .unwrap();
    assert_eq!(field(&first, "lower"), field(&second, "lower"));
    assert_eq!(field(&first, "upper"), field(&second, "upper"));
    assert_eq!(
        field(&first, "standard_error"),
        field(&second, "standard_error")
    );
    assert!((float_field(&first, "estimate") + 3.0).abs() < 1e-12);
    assert!(float_field(&first, "lower") <= float_field(&first, "estimate"));
    assert!(float_field(&first, "upper") >= float_field(&first, "estimate"));
}

#[test]
fn diagnostic_visuals_have_ascii_and_svg_paths() {
    let ascii_options =
        Value::Record(HashMap::from([("format".into(), Value::Str("ascii".into()))]).into());
    let qq = call_stats_builtin(
        "stats_normal_qq_plot",
        vec![numbers(&[1.0, 2.0, 3.0, 4.0, 5.0]), ascii_options.clone()],
    )
    .unwrap();
    assert!(matches!(qq, Value::Str(ref chart) if chart.contains("Normal Q-Q diagnostic (ASCII)")));
    let relationship = call_stats_builtin(
        "stats_relationship_plot",
        vec![
            numbers(&[1.0, 2.0, 3.0]),
            numbers(&[2.0, 4.0, 6.0]),
            ascii_options.clone(),
        ],
    )
    .unwrap();
    assert!(matches!(relationship, Value::Str(ref chart) if chart.contains("complete pairs")));
    let categories = call_stats_builtin(
        "stats_categorical_plot",
        vec![strings(&["a", "b", "a"]), ascii_options],
    )
    .unwrap();
    assert!(matches!(categories, Value::Str(ref chart) if chart.contains("Categorical frequency")));
    let group_svg = call_stats_builtin(
        "stats_group_plot",
        vec![
            numbers(&[1.0, 2.0, 4.0, 5.0]),
            strings(&["a", "a", "b", "b"]),
        ],
    )
    .unwrap();
    assert!(matches!(group_svg, Value::Str(ref svg) if svg.starts_with("<svg")));
    let alias = call_stats_builtin("normal_qq_plot", vec![numbers(&[1.0, 2.0, 3.0, 4.0])]).unwrap();
    assert!(matches!(alias, Value::Str(ref svg) if svg.starts_with("<svg")));
}

#[test]
fn normalization_guidance_audits_the_complete_matrix_without_applying_it() {
    let matrix = Value::List(
        vec![
            Value::List(vec![Value::Int(0), Value::Int(1), Value::Int(2)].into()),
            Value::List(vec![Value::Int(0), Value::Int(10), Value::Int(20)].into()),
        ]
        .into(),
    );
    let report = call_stats_builtin("stats_normalization_guide", vec![matrix]).unwrap();
    assert_eq!(field(&report, "rows"), &Value::Int(2));
    assert_eq!(field(&report, "cells"), &Value::Int(6));
    assert_eq!(field(&report, "zeros"), &Value::Int(2));
    assert_eq!(field(&report, "automatic_changes"), &Value::Bool(false));
    assert!((float_field(&report, "sample_total_ratio") - 10.0).abs() < 1e-12);
}

#[test]
fn sparse_normalization_and_missingness_visuals_preserve_sparse_semantics() {
    let sparse = SparseMatrix::from_triplets(&[0, 1], &[1, 2], &[1.0, 2.0], 2, 3);
    let report = call_stats_builtin(
        "stats_normalization_guide",
        vec![Value::SparseMatrix(sparse.into())],
    )
    .unwrap();
    assert_eq!(field(&report, "cells"), &Value::Int(6));
    assert_eq!(field(&report, "zeros"), &Value::Int(4));
    assert!((float_field(&report, "sample_total_ratio") - 2.0).abs() < 1e-12);

    let table = Value::Table(Table::new(
        vec!["a".into(), "b".into()],
        vec![
            vec![Value::Int(1), Value::Nil],
            vec![Value::Nil, Value::Int(2)],
        ],
    ));
    let options =
        Value::Record(HashMap::from([("format".into(), Value::Str("ascii".into()))]).into());
    let plot = call_stats_builtin("stats_missingness_plot", vec![table, options]).unwrap();
    assert!(
        matches!(plot, Value::Str(ref chart) if chart.contains("X=missing/non-finite") && chart.contains("row 1, column 1"))
    );
}

#[test]
fn correlation_uncertainty_and_shape_clues_are_non_diagnostic() {
    let options = Value::Record(
        HashMap::from([
            ("statistic".into(), Value::Str("pearson".into())),
            ("y".into(), numbers(&[2.0, 4.0, 5.5, 8.0, 10.5])),
            ("repetitions".into(), Value::Int(400)),
            ("seed".into(), Value::Int(19)),
        ])
        .into(),
    );
    let uncertainty = call_stats_builtin(
        "stats_uncertainty",
        vec![numbers(&[1.0, 2.0, 3.0, 4.0, 5.0]), options],
    )
    .unwrap();
    assert_eq!(
        field(&uncertainty, "statistic"),
        &Value::Str("pearson".into())
    );
    assert!(float_field(&uncertainty, "lower") <= float_field(&uncertainty, "estimate"));

    let values = (0..20)
        .map(|index| {
            if index < 10 {
                index as f64 * 0.1
            } else {
                10.0 + index as f64 * 0.1
            }
        })
        .collect::<Vec<_>>();
    let shape = call_stats_builtin("stats_shape", vec![numbers(&values)]).unwrap();
    let evidence = field(&shape, "evidence");
    assert_eq!(
        field(evidence, "multimodality_diagnosed"),
        &Value::Bool(false)
    );
    assert_eq!(field(evidence, "normality_diagnosed"), &Value::Bool(false));
}

#[test]
fn association_screen_uses_type_appropriate_effect_sizes_and_bounds_work() {
    let table = Value::Table(Table::new(
        vec!["x".into(), "y".into(), "group".into(), "batch".into()],
        (0..8)
            .map(|index| {
                vec![
                    Value::Int(index + 1),
                    Value::Float(
                        2.0 * (index + 1) as f64 + if index % 2 == 0 { 0.1 } else { -0.1 },
                    ),
                    Value::Str(if index < 4 { "A" } else { "B" }.into()),
                    Value::Str(if index < 4 { "one" } else { "two" }.into()),
                ]
            })
            .collect(),
    ));
    let report = call_stats_builtin("stats_associations", vec![table]).unwrap();
    assert_eq!(
        field(&report, "hypothesis_tests_performed"),
        &Value::Bool(false)
    );
    assert!(record_int_for_test(&report, "pairs_computed") >= 3);
    assert!(record_int_for_test(&report, "high_association_pairs") >= 2);
    let Value::List(pairs) = field(&report, "pairs") else {
        panic!("pairs should be a List")
    };
    assert!(pairs
        .iter()
        .any(|pair| field(pair, "kind") == &Value::Str("numeric_numeric".into())));
    assert!(pairs
        .iter()
        .any(|pair| field(pair, "kind") == &Value::Str("categorical_categorical".into())));
    assert!(pairs
        .iter()
        .any(|pair| field(pair, "kind") == &Value::Str("categorical_numeric".into())));

    let limited_options =
        Value::Record(HashMap::from([("max_pairs".into(), Value::Int(1))]).into());
    let limited = call_stats_builtin(
        "stats_associations",
        vec![
            Value::Table(Table::new(
                vec!["a".into(), "b".into(), "c".into()],
                vec![
                    vec![Value::Int(1), Value::Int(2), Value::Int(3)],
                    vec![Value::Int(2), Value::Int(4), Value::Int(6)],
                    vec![Value::Int(3), Value::Int(6), Value::Int(9)],
                ],
            )),
            limited_options,
        ],
    )
    .unwrap();
    assert_eq!(field(&limited, "pairs_returned"), &Value::Int(1));
    assert_eq!(field(&limited, "pairs_truncated"), &Value::Bool(true));
}

fn record_int_for_test(value: &Value, name: &str) -> i64 {
    match field(value, name) {
        Value::Int(value) => *value,
        other => panic!("expected integer field {name}, got {other:?}"),
    }
}

#[test]
fn guided_scan_and_ascii_overview_link_evidence_to_non_mutating_actions() {
    let table = Value::Table(Table::new(
        vec!["subject".into(), "group".into(), "value".into()],
        vec![
            vec![
                Value::Str("s1".into()),
                Value::Str("A".into()),
                Value::Int(1),
            ],
            vec![Value::Str("s2".into()), Value::Str("A".into()), Value::Nil],
            vec![
                Value::Str("s3".into()),
                Value::Str("B".into()),
                Value::Int(30),
            ],
            vec![
                Value::Str("s3".into()),
                Value::Str("B".into()),
                Value::Int(30),
            ],
        ],
    ));
    let scan_options = Value::Record(
        HashMap::from([("subject_column".into(), Value::Str("subject".into()))]).into(),
    );
    let scan = call_stats_builtin("stats_scan", vec![table.clone(), scan_options]).unwrap();
    assert_eq!(field(&scan, "automatic_changes"), &Value::Bool(false));
    assert_eq!(
        field(&scan, "automatic_test_selection"),
        &Value::Bool(false)
    );
    let Value::List(recommendations) = field(&scan, "recommendations") else {
        panic!("recommendations should be a List")
    };
    assert!(recommendations
        .iter()
        .any(|item| field(item, "id") == &Value::Str("inspect_missingness".into())));
    assert!(recommendations
        .iter()
        .any(|item| field(item, "id") == &Value::Str("verify_duplicates".into())));
    assert!(recommendations
        .iter()
        .all(|item| field(item, "automatically_applied") == &Value::Bool(false)));
    let associations = field(&scan, "associations");
    let Value::List(skipped) = field(associations, "skipped_declared") else {
        panic!("skipped_declared should be a List")
    };
    assert_eq!(skipped.as_ref(), &[Value::Str("subject".into())]);

    let overview = call_stats_builtin("stats_overview_ascii", vec![table]).unwrap();
    assert!(
        matches!(overview, Value::Str(ref output) if output.contains("BioLang dataset overview") && output.contains("median=") && output.contains("top="))
    );
}

#[test]
fn simple_linear_diagnostics_report_residual_clues_and_dual_plots() {
    let x = numbers(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]);
    let y = numbers(&[2.1, 3.9, 6.2, 7.8, 10.4, 11.7, 14.2, 15.8]);
    let report =
        call_stats_builtin("stats_linear_diagnostics", vec![x.clone(), y.clone()]).unwrap();
    assert_eq!(field(&report, "complete_pairs"), &Value::Int(8));
    assert!((float_field(&report, "slope") - 1.98).abs() < 0.05);
    assert_eq!(field(&report, "input_modified"), &Value::Bool(false));
    assert!(matches!(field(&report, "residuals"), Value::List(values) if values.len() == 8));

    let ascii_options = Value::Record(
        HashMap::from([
            ("format".into(), Value::Str("ascii".into())),
            ("view".into(), Value::Str("residuals".into())),
        ])
        .into(),
    );
    let residual_plot = call_stats_builtin(
        "stats_linear_diagnostic_plot",
        vec![x.clone(), y.clone(), ascii_options],
    )
    .unwrap();
    assert!(
        matches!(residual_plot, Value::Str(ref output) if output.contains("Residuals versus fitted (ASCII)"))
    );
    let qq_options =
        Value::Record(HashMap::from([("view".into(), Value::Str("qq".into()))]).into());
    let qq_plot =
        call_stats_builtin("stats_linear_diagnostic_plot", vec![x, y, qq_options]).unwrap();
    assert!(matches!(qq_plot, Value::Str(ref output) if output.starts_with("<svg")));
}

#[test]
fn missingness_and_design_reports_expose_patterns_and_dependence_clues() {
    let table = Value::Table(Table::new(
        vec![
            "subject".into(),
            "group".into(),
            "time".into(),
            "marker".into(),
            "outcome".into(),
        ],
        vec![
            vec![
                Value::Str("s1".into()),
                Value::Str("A".into()),
                Value::Int(0),
                Value::Nil,
                Value::Float(1.0),
            ],
            vec![
                Value::Str("s1".into()),
                Value::Str("B".into()),
                Value::Int(1),
                Value::Nil,
                Value::Float(2.0),
            ],
            vec![
                Value::Str("s2".into()),
                Value::Str("A".into()),
                Value::Int(0),
                Value::Float(5.0),
                Value::Float(10.0),
            ],
            vec![
                Value::Str("s2".into()),
                Value::Str("B".into()),
                Value::Int(1),
                Value::Float(6.0),
                Value::Float(12.0),
            ],
        ],
    ));
    let options = Value::Record(
        HashMap::from([
            ("subject_column".into(), Value::Str("subject".into())),
            ("group_column".into(), Value::Str("group".into())),
            ("time_column".into(), Value::Str("time".into())),
        ])
        .into(),
    );
    let profile = call_stats_builtin("stats_profile", vec![table, options]).unwrap();
    let missingness = field(&profile, "missingness");
    assert!(matches!(field(missingness, "patterns"), Value::List(values) if values.len() == 2));
    assert!(
        matches!(field(missingness, "observed_missing_comparisons"), Value::List(values) if !values.is_empty())
    );
    let design = field(&profile, "design");
    let Value::List(clues) = field(design, "design_clues") else {
        panic!("design_clues should be a List")
    };
    assert!(clues
        .iter()
        .any(|clue| field(clue, "id") == &Value::Str("paired_or_crossover_clue".into())));
    assert!(clues
        .iter()
        .any(|clue| field(clue, "id") == &Value::Str("longitudinal_clue".into())));
    assert_eq!(
        field(design, "independence_established"),
        &Value::Bool(false)
    );
}

#[test]
fn distribution_family_screen_is_scale_sensitive_but_non_selecting() {
    let report = call_stats_builtin(
        "stats_distribution_clues",
        vec![numbers(&[0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 4.0, 9.0, 15.0])],
    )
    .unwrap();
    assert_eq!(field(&report, "nonnegative_integers"), &Value::Bool(true));
    assert_eq!(field(&report, "model_selected"), &Value::Bool(false));
    assert!(float_field(&report, "variance_mean_ratio") > 1.0);
    let Value::List(candidates) = field(&report, "candidates") else {
        panic!("candidates should be a List")
    };
    assert_eq!(candidates.len(), 4);
    assert!(candidates.iter().any(|candidate| {
        field(candidate, "name") == &Value::Str("negative_binomial".into())
            && field(candidate, "available") == &Value::Bool(true)
    }));
}

#[test]
fn multivariable_diagnostics_encode_categories_interactions_and_validation() {
    let predictors = Value::Table(Table::new(
        vec!["age".into(), "group".into()],
        (0..16)
            .map(|index| {
                vec![
                    Value::Float(20.0 + index as f64),
                    Value::Str(if index % 2 == 0 { "A" } else { "B" }.into()),
                ]
            })
            .collect(),
    ));
    let outcome = numbers(
        &(0..16)
            .map(|index| {
                let age = 20.0 + index as f64;
                1.0 + 2.0 * age
                    + if index % 2 == 0 { 0.0 } else { 5.0 + 0.1 * age }
                    + (index % 3) as f64 * 0.05
            })
            .collect::<Vec<_>>(),
    );
    let options = Value::Record(
        HashMap::from([
            (
                "interactions".into(),
                Value::List(vec![Value::Str("age:group".into())].into()),
            ),
            ("validation_folds".into(), Value::Int(4)),
            ("include_values".into(), Value::Bool(true)),
        ])
        .into(),
    );
    let report = call_stats_builtin(
        "stats_multiple_linear_diagnostics",
        vec![predictors, outcome, options],
    )
    .unwrap();
    assert_eq!(field(&report, "complete_rows"), &Value::Int(16));
    assert_eq!(field(&report, "encoded_predictors"), &Value::Int(3));
    assert_eq!(field(&report, "validation_folds"), &Value::Int(4));
    assert!(float_field(&report, "validation_rmse").is_finite());
    assert!(
        matches!(field(&report, "fitted_intervals"), Value::List(values) if values.len() == 16)
    );
    assert_eq!(field(&report, "input_modified"), &Value::Bool(false));
}

#[test]
fn report_and_sparse_omics_profile_are_notebook_and_memory_friendly() {
    let table = Value::Table(Table::new(
        vec!["group".into(), "value".into()],
        vec![
            vec![Value::Str("A".into()), Value::Int(1)],
            vec![Value::Str("A".into()), Value::Nil],
            vec![Value::Str("B".into()), Value::Int(8)],
        ],
    ));
    let options = Value::Record(
        HashMap::from([
            ("format".into(), Value::Str("html".into())),
            ("title".into(), Value::Str("Study <review>".into())),
            ("seed".into(), Value::Int(7)),
        ])
        .into(),
    );
    let report = call_stats_builtin("stats_report", vec![table, options]).unwrap();
    assert_eq!(field(&report, "mime_type"), &Value::Str("text/html".into()));
    assert!(
        matches!(field(&report, "content"), Value::Str(content) if content.starts_with("<!doctype html>") && content.contains("Study &lt;review&gt;") && !content.contains("<script"))
    );
    assert_eq!(field(&report, "automatic_changes"), &Value::Bool(false));

    let sparse =
        SparseMatrix::from_triplets(&[0, 0, 1, 2], &[0, 2, 1, 3], &[2.0, 1.0, 4.0, 9.0], 3, 4);
    let omics_options = Value::Record(
        HashMap::from([
            ("modality".into(), Value::Str("single_cell".into())),
            ("sample_axis".into(), Value::Str("rows".into())),
        ])
        .into(),
    );
    let omics = call_stats_builtin(
        "stats_omics_profile",
        vec![Value::SparseMatrix(sparse.into()), omics_options],
    )
    .unwrap();
    assert_eq!(field(&omics, "samples"), &Value::Int(3));
    assert_eq!(field(&omics, "features"), &Value::Int(4));
    assert_eq!(field(&omics, "zeros"), &Value::Int(8));
    assert!(
        matches!(field(&omics, "memory_behavior"), Value::Str(value) if value.contains("never densify"))
    );
}

#[test]
fn grouped_validation_keeps_subject_rows_in_the_same_fold() {
    let predictors = Value::Table(Table::new(
        vec!["subject".into(), "dose".into()],
        (0..12)
            .map(|index| {
                vec![
                    Value::Str(format!("s{}", index / 2)),
                    Value::Float(index as f64),
                ]
            })
            .collect(),
    ));
    let outcome = numbers(
        &(0..12)
            .map(|index| 3.0 + 1.5 * index as f64 + (index / 2) as f64 * 0.1)
            .collect::<Vec<_>>(),
    );
    let options = Value::Record(
        HashMap::from([
            (
                "validation_group_column".into(),
                Value::Str("subject".into()),
            ),
            ("validation_folds".into(), Value::Int(3)),
        ])
        .into(),
    );
    let report = call_stats_builtin(
        "stats_multiple_linear_diagnostics",
        vec![predictors, outcome, options],
    )
    .unwrap();
    assert_eq!(field(&report, "encoded_predictors"), &Value::Int(1));
    assert_eq!(field(&report, "validation_groups"), &Value::Int(6));
    assert_eq!(field(&report, "validation_folds"), &Value::Int(3));
    assert_eq!(
        field(&report, "validation_method"),
        &Value::Str("group-held-out deterministic folds".into())
    );
}

#[test]
fn robust_weighted_and_time_series_diagnostics_are_explicit_sensitivity_checks() {
    let predictors = Value::Table(Table::new(
        vec!["x".into()],
        (1..=10)
            .map(|value| vec![Value::Float(value as f64)])
            .collect(),
    ));
    let outcome = numbers(&[2.0, 4.1, 5.9, 8.0, 10.1, 12.0, 14.0, 16.1, 18.0, 80.0]);
    let robust =
        call_stats_builtin("stats_robust_linear_diagnostics", vec![predictors, outcome]).unwrap();
    assert_eq!(
        field(&robust, "formal_inference_provided"),
        &Value::Bool(false)
    );
    assert!(float_field(&robust, "minimum_weight") < 0.1);
    assert!(matches!(field(&robust, "coefficients"), Value::List(values) if values.len() == 2));

    let weighted = call_stats_builtin(
        "stats_weighted_summary",
        vec![numbers(&[1.0, 2.0, 10.0]), numbers(&[1.0, 1.0, 8.0])],
    )
    .unwrap();
    assert!((float_field(&weighted, "weighted_mean") - 8.3).abs() < 1e-12);
    assert!(float_field(&weighted, "effective_sample_size") < 2.0);
    assert_eq!(
        field(&weighted, "formal_survey_inference_provided"),
        &Value::Bool(false)
    );

    let series = call_stats_builtin(
        "stats_time_series_diagnostics",
        vec![numbers(&[
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0,
        ])],
    )
    .unwrap();
    assert!(float_field(&series, "order_correlation") > 0.99);
    assert_eq!(field(&series, "model_selected"), &Value::Bool(false));
    assert!(
        matches!(field(&series, "autocorrelations"), Value::List(values) if !values.is_empty())
    );
    assert!(matches!(field(&series, "ascii"), Value::Str(value) if value.contains("Ljung-Box")));
}

#[test]
fn cluster_diagnostics_quantify_declared_non_independence_without_fitting_a_model() {
    let report = call_stats_builtin(
        "stats_cluster_diagnostics",
        vec![
            numbers(&[1.0, 1.2, 5.0, 5.2, 9.0, 9.2, 13.0, 13.2]),
            Value::List(
                ["a", "a", "b", "b", "c", "c", "d", "d"]
                    .into_iter()
                    .map(|value| Value::Str(value.into()))
                    .collect::<Vec<_>>()
                    .into(),
            ),
        ],
    )
    .unwrap();
    assert_eq!(field(&report, "clusters"), &Value::Int(4));
    assert!(float_field(&report, "intraclass_correlation") > 0.9);
    assert!(float_field(&report, "approximate_effective_sample_size") < 5.0);
    assert_eq!(field(&report, "mixed_model_fitted"), &Value::Bool(false));
    assert!(
        matches!(field(&report, "ascii"), Value::Str(value) if value.contains("no mixed model fitted"))
    );
}

#[test]
fn means_guide_pairs_each_centre_with_a_compatible_spread() {
    let options =
        Value::Record(HashMap::from([("trim_fraction".into(), Value::Float(0.25))]).into());
    let report =
        call_stats_builtin("stats_means", vec![numbers(&[1.0, 2.0, 4.0, 8.0]), options]).unwrap();
    assert!((float_field(&report, "arithmetic_mean") - 3.75).abs() < 1e-12);
    assert!((float_field(&report, "geometric_mean") - 8.0_f64.sqrt()).abs() < 1e-12);
    assert!((float_field(&report, "harmonic_mean") - 2.1333333333333333).abs() < 1e-12);
    assert!((float_field(&report, "trimmed_mean") - 3.0).abs() < 1e-12);
    assert_eq!(field(&report, "automatic_choice"), &Value::Bool(false));
    assert!(
        matches!(field(&report, "centre_spread_pairs"), Value::List(values) if values.len() == 7)
    );
}

use std::collections::HashMap;

// ── Degenerate inputs ────────────────────────────────────────────────
//
// The suite had no empty-input test at all, which is how a single-observation
// summary came to report a standard deviation of 0.0 and then recommend
// quoting it. These cover the three shapes that break statistics: nothing,
// one value, and no variance.

/// Every guided entry point refuses an empty input by name, rather than
/// dividing by zero or returning an empty record that reads like a result.
#[test]
fn guided_entry_points_reject_empty_input_by_name() {
    let cases: &[(&str, Vec<Value>)] = &[
        ("stats_explore", vec![numbers(&[])]),
        ("stats_shape", vec![numbers(&[])]),
        ("stats_means", vec![numbers(&[])]),
        ("stats_uncertainty", vec![numbers(&[])]),
        ("stats_distribution_clues", vec![numbers(&[])]),
        ("stats_distribution_ascii", vec![numbers(&[])]),
        ("stats_relationship", vec![numbers(&[]), numbers(&[])]),
        ("stats_compare", vec![numbers(&[]), strings(&[])]),
        ("stats_categories", vec![numbers(&[])]),
    ];
    for (name, args) in cases {
        let error = call_stats_builtin(name, args.clone())
            .expect_err(&format!("{name} should reject an empty input"));
        let rendered = format!("{error:?}");
        assert!(
            rendered.contains(name),
            "{name} should name itself in its error, got {rendered}"
        );
    }
}

/// A single observation has no sample spread. Reporting zero would claim the
/// data has none, when what is true is that it cannot be measured.
#[test]
fn a_single_observation_reports_no_spread_rather_than_zero() {
    let report = call_stats_builtin("stats_explore", vec![numbers(&[5.0])]).unwrap();
    let summary = field(&report, "summary");
    assert_eq!(field(summary, "sd"), &Value::Nil);
    assert_eq!(field(summary, "variance"), &Value::Nil);
    assert!((float_field(summary, "mean") - 5.0).abs() < 1e-12);

    // and it must not recommend quoting a spread that does not exist
    let suggestion = field(&report, "suggestion");
    let spread = field(suggestion, "spread").as_str().unwrap().to_string();
    assert!(
        !spread.contains("standard deviation") && !spread.contains("IQR"),
        "n=1 must not recommend a spread, got {spread:?}"
    );

    // and the reason must be disclosed as a clue, not left silent
    let Value::List(clues) = field(&report, "clues") else {
        panic!("clues should be a List");
    };
    let ids = clues
        .iter()
        .filter_map(|clue| field(clue, "id").as_str().map(str::to_string))
        .collect::<Vec<_>>();
    assert!(
        ids.iter().any(|id| id == "single_observation"),
        "n=1 should be disclosed, got clues {ids:?}"
    );
}

/// Constant input has zero variance, so correlation is undefined. Reporting
/// zero would assert "no relationship" for something unknowable, and the
/// p-value derived from it came out as a confident 1.0.
#[test]
fn correlation_of_a_constant_variable_is_undefined_not_zero() {
    let constant = numbers(&[1.0, 1.0, 1.0]);
    let varying = numbers(&[1.0, 2.0, 3.0]);

    let report = call_stats_builtin(
        "stats_relationship",
        vec![constant.clone(), varying.clone()],
    )
    .unwrap();
    assert_eq!(
        field(&report, "pearson"),
        &Value::Nil,
        "the guided layer reports an undefined correlation as absent"
    );

    // The Float-returning builtins cannot express absence, so they agree on NaN.
    let bare = call_stats_builtin("cor", vec![constant, varying]).unwrap();
    let Value::Float(value) = bare else {
        panic!("cor should return a Float, got {bare:?}");
    };
    assert!(value.is_nan(), "cor of a constant variable should be NaN");
}

/// A constant variable still has a defined mean, median and IQR: only the
/// spread-based quantities are affected, and the summary should say so.
#[test]
fn constant_input_keeps_the_quantities_that_are_still_defined() {
    let report = call_stats_builtin("stats_explore", vec![numbers(&[5.0, 5.0, 5.0, 5.0])]).unwrap();
    let summary = field(&report, "summary");
    assert!((float_field(summary, "mean") - 5.0).abs() < 1e-12);
    assert!((float_field(summary, "median") - 5.0).abs() < 1e-12);
    assert!((float_field(summary, "iqr")).abs() < 1e-12);
    // four observations, so the sample variance is defined and is genuinely zero
    assert!((float_field(summary, "sd")).abs() < 1e-12);
    assert!((float_field(summary, "variance")).abs() < 1e-12);
    // zero spread is a measured result here, so it is not disclosed as absent
    let Value::List(clues) = field(&report, "clues") else {
        panic!("clues should be a List");
    };
    assert!(
        !clues
            .iter()
            .any(|clue| field(clue, "id").as_str() == Some("single_observation")),
        "four observations is not the single-observation case"
    );
}
