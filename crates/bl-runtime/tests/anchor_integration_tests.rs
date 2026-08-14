use bl_core::value::Value;
use bl_runtime::singlecell::call_singlecell_builtin;
use std::collections::HashMap;

fn matrix(rows: &[&[f64]]) -> Value {
    Value::List(
        rows.iter()
            .map(|row| {
                Value::List(
                    row.iter()
                        .copied()
                        .map(Value::Float)
                        .collect::<Vec<_>>()
                        .into(),
                )
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

fn opts(values: &[(&str, Value)]) -> Value {
    Value::Record(
        values
            .iter()
            .map(|(name, value)| ((*name).to_string(), value.clone()))
            .collect::<HashMap<_, _>>()
            .into(),
    )
}

fn record_list<'a>(value: &'a Value, name: &str) -> &'a [Value] {
    match value {
        Value::Record(fields) => match fields.get(name).unwrap() {
            Value::List(values) => values,
            other => panic!("expected list, got {other:?}"),
        },
        other => panic!("expected record, got {other:?}"),
    }
}

fn anchor_tuples(value: &Value) -> Vec<(i64, i64, f64)> {
    record_list(value, "anchors")
        .iter()
        .map(|anchor| match anchor {
            Value::Record(fields) => {
                let index = |name: &str| match fields.get(name).unwrap() {
                    Value::Int(value) => *value,
                    other => panic!("expected index, got {other:?}"),
                };
                (
                    index("left"),
                    index("right"),
                    fields.get("score").unwrap().as_float().unwrap(),
                )
            }
            other => panic!("expected anchor, got {other:?}"),
        })
        .collect()
}

fn rows(value: &Value) -> Vec<Vec<f64>> {
    match value {
        Value::Matrix(matrix) => (0..matrix.nrow)
            .map(|row| matrix.data[row * matrix.ncol..(row + 1) * matrix.ncol].to_vec())
            .collect(),
        Value::List(values) => values
            .iter()
            .map(|row| match row {
                Value::List(columns) => columns
                    .iter()
                    .map(|value| value.as_float().unwrap())
                    .collect(),
                other => panic!("expected row, got {other:?}"),
            })
            .collect(),
        other => panic!("expected matrix, got {other:?}"),
    }
}

fn centroid(values: &[Vec<f64>]) -> Vec<f64> {
    (0..values[0].len())
        .map(|column| values.iter().map(|row| row[column]).sum::<f64>() / values.len() as f64)
        .collect()
}

fn distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f64>()
        .sqrt()
}

fn record_numbers(value: &Value, name: &str) -> Vec<f64> {
    record_list(value, name)
        .iter()
        .map(|value| value.as_float().unwrap())
        .collect()
}

#[test]
fn paper_lanczos_cca_matches_dense_singular_values() {
    let left = matrix(&[
        &[2.0, 0.1, -0.2],
        &[1.8, -0.1, 0.2],
        &[2.2, 0.0, 0.1],
        &[-2.0, 0.2, 0.1],
        &[-1.8, -0.2, -0.1],
        &[-2.2, 0.0, 0.2],
    ]);
    let right = matrix(&[
        &[2.1, 0.0, -0.1],
        &[1.9, 0.1, 0.1],
        &[2.3, -0.1, 0.0],
        &[-1.9, 0.1, 0.0],
        &[-1.7, -0.1, -0.2],
        &[-2.1, 0.1, 0.1],
    ]);
    let exact = call_singlecell_builtin(
        "cca",
        vec![left.clone(), right.clone(), opts(&[("k", Value::Int(2))])],
    )
    .unwrap();
    let lanczos = call_singlecell_builtin(
        "cca",
        vec![
            left,
            right,
            opts(&[
                ("k", Value::Int(2)),
                ("solver", Value::Str("lanczos".to_string())),
                ("work_extra", Value::Int(2)),
                ("tolerance", Value::Float(1e-8)),
            ]),
        ],
    )
    .unwrap();
    let expected = record_numbers(&exact, "d");
    let actual = record_numbers(&lanczos, "d");
    for (expected, actual) in expected.iter().zip(actual) {
        assert!(
            (expected - actual).abs() / expected.abs() < 1e-7,
            "{actual} != {expected}"
        );
    }
}

#[test]
fn direct_lanczos_pca_matches_default_variance_and_reports_convergence() {
    let input = matrix(&[
        &[2.0, 0.1, -0.2, 0.3],
        &[1.8, -0.1, 0.2, 0.1],
        &[2.2, 0.0, 0.1, -0.2],
        &[-2.0, 0.2, 0.1, 0.4],
        &[-1.8, -0.2, -0.1, -0.3],
        &[-2.2, 0.0, 0.2, -0.1],
    ]);
    let exact = call_singlecell_builtin(
        "sc_pca",
        vec![input.clone(), Value::Int(2), Value::Bool(true)],
    )
    .unwrap();
    let lanczos = call_singlecell_builtin(
        "sc_pca",
        vec![
            input.clone(),
            Value::Int(2),
            Value::Bool(true),
            opts(&[
                ("solver", Value::Str("lanczos".to_string())),
                (
                    "initial",
                    Value::List(
                        vec![0.5, -0.25, 0.75, 0.125]
                            .into_iter()
                            .map(Value::Float)
                            .collect::<Vec<_>>()
                            .into(),
                    ),
                ),
                ("work_extra", Value::Int(2)),
                ("tolerance", Value::Float(1e-9)),
                ("max_iterations", Value::Int(200)),
            ]),
        ],
    )
    .unwrap();
    for (expected, actual) in record_numbers(&exact, "explained_variance")
        .iter()
        .zip(record_numbers(&lanczos, "explained_variance"))
    {
        assert!(
            (expected - actual).abs() / expected.abs().max(1e-12) < 1e-7,
            "{actual} != {expected}"
        );
    }
    match &lanczos {
        Value::Record(fields) => {
            assert_eq!(fields.get("converged"), Some(&Value::Bool(true)));
            assert_eq!(
                fields.get("compute_method"),
                Some(&Value::Str(
                    "direct_matrix_restarted_lanczos_cpu".to_string()
                ))
            );
        }
        other => panic!("expected PCA record, got {other:?}"),
    }

    let limited = call_singlecell_builtin(
        "sc_pca",
        vec![
            input,
            Value::Int(2),
            Value::Bool(true),
            opts(&[
                ("solver", Value::Str("lanczos".to_string())),
                ("work_extra", Value::Int(2)),
                ("max_iterations", Value::Int(1)),
            ]),
        ],
    )
    .unwrap();
    match limited {
        Value::Record(fields) => {
            assert_eq!(fields.get("converged"), Some(&Value::Bool(false)));
        }
        other => panic!("expected PCA record, got {other:?}"),
    }
}

#[test]
fn external_pca_reports_a_missing_provider_clearly() {
    let error = call_singlecell_builtin(
        "sc_pca",
        vec![
            matrix(&[&[1.0, 2.0, 3.0], &[2.0, 1.0, 4.0], &[3.0, 2.0, 1.0]]),
            Value::Int(2),
            Value::Bool(false),
            opts(&[
                ("solver", Value::Str("external".to_string())),
                (
                    "external_provider",
                    Value::Str("definitely-missing-bl-seurat-provider".to_string()),
                ),
            ]),
        ],
    )
    .unwrap_err();
    let message = error.to_string();
    assert!(message.contains("cannot start external provider"));
    assert!(message.contains("BIOLANG_SEURAT_PROVIDER"));
}

#[test]
fn cca_anchors_are_deterministic_mutual_neighbours() {
    let left = matrix(&[
        &[2.0, 0.1, -0.2],
        &[1.8, -0.1, 0.2],
        &[2.2, 0.0, 0.1],
        &[-2.0, 0.2, 0.1],
        &[-1.8, -0.2, -0.1],
        &[-2.2, 0.0, 0.2],
    ]);
    let right = matrix(&[
        &[2.1, 0.0, -0.1],
        &[1.9, 0.1, 0.1],
        &[2.3, -0.1, 0.0],
        &[-1.9, 0.1, 0.0],
        &[-1.7, -0.1, -0.2],
        &[-2.1, 0.1, 0.1],
    ]);
    let options = opts(&[
        ("reduction", Value::Str("cca".to_string())),
        ("dims", Value::Int(2)),
        ("k_anchor", Value::Int(2)),
        ("k_filter", Value::Int(3)),
        ("k_score", Value::Int(3)),
        ("max_features", Value::Int(3)),
        ("cca_sweeps", Value::Int(7)),
        ("cca_oversample", Value::Int(3)),
    ]);
    let first = call_singlecell_builtin(
        "sc_find_anchors",
        vec![left.clone(), right.clone(), options.clone()],
    )
    .unwrap();
    let second = call_singlecell_builtin("sc_find_anchors", vec![left, right, options]).unwrap();
    match &first {
        Value::Record(fields) => {
            assert_eq!(fields.get("cca_sweeps"), Some(&Value::Int(7)));
            assert_eq!(fields.get("cca_oversample"), Some(&Value::Int(3)));
        }
        other => panic!("expected anchor record, got {other:?}"),
    }
    assert_eq!(anchor_tuples(&first), anchor_tuples(&second));
    assert_eq!(
        anchor_tuples(&first),
        vec![
            (0, 0, 1.0),
            (0, 1, 0.0),
            (1, 2, 0.0),
            (2, 2, 1.0),
            (2, 1, 1.0),
            (3, 3, 1.0),
            (3, 4, 0.0),
            (4, 5, 1.0),
            (4, 3, 1.0),
            (5, 5, 1.0),
        ],
        "must match validation/single-cell/seurat_mit_anchor_fixture.R in biolang-workflows"
    );
}

#[test]
fn anchor_correction_reduces_batch_centroid_gap() {
    let left_rows: &[&[f64]] = &[
        &[3.0, 1.0],
        &[2.8, 1.2],
        &[3.2, 0.8],
        &[0.5, 4.0],
        &[0.7, 3.8],
        &[0.3, 4.2],
    ];
    let right_rows: &[&[f64]] = &[
        &[5.0, 0.0],
        &[4.8, 0.2],
        &[5.2, -0.2],
        &[2.5, 3.0],
        &[2.7, 2.8],
        &[2.3, 3.2],
    ];
    let anchors = Value::List(
        (0..6)
            .map(|index| {
                Value::Record(
                    HashMap::from([
                        ("left".to_string(), Value::Int(index)),
                        ("right".to_string(), Value::Int(index)),
                        ("score".to_string(), Value::Float(1.0)),
                    ])
                    .into(),
                )
            })
            .collect::<Vec<_>>()
            .into(),
    );
    let embedding = matrix(&[
        &[1.0, 0.0],
        &[0.9, 0.1],
        &[1.1, -0.1],
        &[0.0, 1.0],
        &[0.1, 0.9],
        &[-0.1, 1.1],
    ]);
    let anchor_set = opts(&[("anchors", anchors)]);
    let integrated = call_singlecell_builtin(
        "sc_integrate_anchors",
        vec![
            matrix(left_rows),
            matrix(right_rows),
            anchor_set,
            opts(&[
                ("k_weight", Value::Int(3)),
                ("sd_weight", Value::Float(0.25)),
                ("weight_reduction", embedding),
            ]),
        ],
    )
    .unwrap();
    let integrated = rows(&integrated);
    let before = distance(
        &centroid(&rows(&matrix(left_rows))),
        &centroid(&rows(&matrix(right_rows))),
    );
    let after = distance(&centroid(&integrated[..6]), &centroid(&integrated[6..]));
    assert!(after < before * 0.05, "centroid gap {before} -> {after}");
    let type_gap = distance(&centroid(&integrated[6..9]), &centroid(&integrated[9..12]));
    assert!(type_gap > 3.0, "biological separation was lost: {type_gap}");
}

#[test]
fn integration_uses_seurat_5_5_1_weight_kernel() {
    let left = matrix(&[&[0.0], &[8.0], &[20.0]]);
    let right = matrix(&[&[2.0], &[12.0], &[25.0], &[8.0]]);
    let anchors = Value::List(
        [(0, 0, 0.5), (1, 1, 1.0), (2, 2, 1.0)]
            .into_iter()
            .map(|(left, right, score)| {
                Value::Record(
                    HashMap::from([
                        ("left".to_string(), Value::Int(left)),
                        ("right".to_string(), Value::Int(right)),
                        ("score".to_string(), Value::Float(score)),
                    ])
                    .into(),
                )
            })
            .collect::<Vec<_>>()
            .into(),
    );
    let anchor_set = opts(&[("anchors", anchors)]);
    let integrated = call_singlecell_builtin(
        "sc_integrate_anchors",
        vec![
            left,
            right,
            anchor_set,
            opts(&[
                ("k_weight", Value::Int(3)),
                ("sd_weight", Value::Float(1.0)),
                ("return_details", Value::Bool(true)),
                (
                    "diagnostic_weight_cells",
                    Value::List(vec![Value::Int(3)].into()),
                ),
                (
                    "weight_reduction",
                    matrix(&[&[0.0], &[1.0], &[3.0], &[0.5]]),
                ),
            ]),
        ],
    )
    .unwrap();
    let integrated = match integrated {
        Value::Record(fields) => {
            assert_eq!(fields.get("effective_k").unwrap(), &Value::Int(3));
            assert_eq!(rows(fields.get("query_weight_embedding").unwrap()).len(), 4);
            let weights = match fields.get("diagnostic_weights").unwrap() {
                Value::List(weights) => weights,
                other => panic!("expected diagnostic weight list, got {other:?}"),
            };
            assert_eq!(weights.len(), 3);
            let total = weights
                .iter()
                .map(|row| match row {
                    Value::Record(values) => values.get("weight").unwrap().as_float().unwrap(),
                    other => panic!("expected diagnostic weight record, got {other:?}"),
                })
                .sum::<f64>();
            assert!((total - 1.0).abs() < 1e-12);
            rows(fields.get("integrated_matrix").unwrap())
        }
        other => panic!("expected detailed integration record, got {other:?}"),
    };

    // Query position 0.5 has normalized similarities 0.8, 0.8, 0.0 to
    // anchor-cell positions 0, 1, 3. This is Seurat's FindWeightsC formula.
    let first_weight = 1.0 - (-0.8_f64 * 0.5 / 4.0).exp();
    let second_weight = 1.0 - (-0.8_f64 / 4.0).exp();
    let expected = 8.0
        + (first_weight * (0.0 - 2.0) + second_weight * (8.0 - 12.0))
            / (first_weight + second_weight);
    assert!(
        (integrated[6][0] - expected).abs() < 1e-12,
        "{} != {expected}",
        integrated[6][0]
    );
}

#[test]
fn integration_caps_expanded_pairs_at_k_weight() {
    let left = matrix(&[&[0.0], &[4.0], &[10.0], &[8.0], &[20.0]]);
    let right = matrix(&[&[2.0], &[12.0], &[25.0], &[8.0]]);
    let anchors = Value::List(
        [
            (0, 0, 0.5),
            (1, 0, 1.0),
            (2, 0, 0.8),
            (3, 1, 1.0),
            (4, 2, 1.0),
        ]
        .into_iter()
        .map(|(left, right, score)| {
            Value::Record(
                HashMap::from([
                    ("left".to_string(), Value::Int(left)),
                    ("right".to_string(), Value::Int(right)),
                    ("score".to_string(), Value::Float(score)),
                ])
                .into(),
            )
        })
        .collect::<Vec<_>>()
        .into(),
    );
    let integrated = call_singlecell_builtin(
        "sc_integrate_anchors",
        vec![
            left,
            right,
            opts(&[("anchors", anchors)]),
            opts(&[
                ("k_weight", Value::Int(3)),
                ("sd_weight", Value::Float(1.0)),
                (
                    "weight_reduction",
                    matrix(&[&[0.0], &[1.0], &[3.0], &[0.5]]),
                ),
            ]),
        ],
    )
    .unwrap();
    let integrated = rows(&integrated);

    // Seurat 5.5.1 FindWeightsC walks the selected anchor cells but stops after
    // k.weight integration-matrix rows. With three anchors on q0, q1 does not
    // contribute. This follows integration.cpp's `k < indices.size()` guard.
    let weights = [0.5_f64, 1.0, 0.8].map(|score| 1.0 - (-0.8 * score / 4.0).exp());
    let differences = [-2.0_f64, 2.0, 8.0];
    let total = weights.iter().sum::<f64>();
    let expected = 8.0
        + weights
            .iter()
            .zip(differences)
            .map(|(weight, difference)| weight * difference)
            .sum::<f64>()
            / total;
    assert!(
        (integrated[8][0] - expected).abs() < 1e-12,
        "{} != {expected}",
        integrated[8][0]
    );
}

#[test]
fn anchors_reject_mismatched_feature_counts() {
    let error = call_singlecell_builtin(
        "sc_find_anchors",
        vec![matrix(&[&[1.0, 2.0]]), matrix(&[&[1.0, 2.0, 3.0]])],
    )
    .unwrap_err();
    assert!(error.to_string().contains("identical feature columns"));
}
