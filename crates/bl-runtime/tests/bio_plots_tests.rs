use bl_core::matrix::Matrix;
use bl_core::value::{Table, Value};
use bl_runtime::bio_plots::call_bio_plots_builtin;
use std::collections::HashMap;

// ── Helpers ──────────────────────────────────────────────────────

fn make_table(cols: Vec<&str>, rows: Vec<Vec<Value>>) -> Value {
    Value::Table(Table::new(
        cols.into_iter().map(|s| s.to_string()).collect(),
        rows,
    ))
}

fn make_opts(pairs: Vec<(&str, Value)>) -> Value {
    Value::Record(
        pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<std::collections::HashMap<_, _>>()
            .into(),
    )
}

fn svg_opts() -> Value {
    make_opts(vec![("format", Value::Str("svg".into()))])
}

fn publication_svg_opts() -> Value {
    make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("Measured result".into())),
        ("subtitle", Value::Str("Independent samples".into())),
        ("caption", Value::Str("BioLang test fixture".into())),
    ])
}

fn assert_svg(val: &Value) {
    if let Value::Str(s) = val {
        assert!(s.contains("<svg"), "output should contain <svg tag");
    } else {
        panic!(
            "expected Value::Str with SVG content, got {:?}",
            val.type_of()
        );
    }
}

fn assert_publication_svg(val: &Value) {
    assert_svg(val);
    let Value::Str(svg) = val else { unreachable!() };
    assert!(svg.contains("data-biolang-theme=\"publication\""), "{svg}");
    assert!(svg.contains(">Measured result<"), "{svg}");
    assert!(svg.contains(">Independent samples<"), "{svg}");
    assert!(svg.contains(">BioLang test fixture<"), "{svg}");
}

// ── 1. manhattan ────────────────────────────────────────────────

#[test]
fn test_manhattan_ascii() {
    let t = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(1000.0),
                Value::Float(0.001),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(2000.0),
                Value::Float(0.05),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(500.0),
                Value::Float(1e-8),
            ],
        ],
    );
    let r = call_bio_plots_builtin("manhattan", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_manhattan_svg() {
    let t = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(1000.0),
                Value::Float(0.001),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(500.0),
                Value::Float(1e-8),
            ],
        ],
    );
    let r = call_bio_plots_builtin("manhattan", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_manhattan_single_chromosome() {
    let t = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(100.0),
                Value::Float(0.01),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(200.0),
                Value::Float(0.001),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(300.0),
                Value::Float(1e-9),
            ],
        ],
    );
    let r = call_bio_plots_builtin("manhattan", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_manhattan_single_chromosome_svg() {
    let t = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(100.0),
                Value::Float(0.01),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(200.0),
                Value::Float(1e-9),
            ],
        ],
    );
    let r = call_bio_plots_builtin("manhattan", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_manhattan_wrong_type() {
    let r = call_bio_plots_builtin("manhattan", vec![Value::Int(42)]);
    assert!(r.is_err());
}

// ── 2. qq_plot ──────────────────────────────────────────────────

#[test]
fn test_qq_plot_ascii() {
    let vals = Value::List(
        (vec![
            Value::Float(0.001),
            Value::Float(0.01),
            Value::Float(0.1),
            Value::Float(0.5),
        ])
        .into(),
    );
    let r = call_bio_plots_builtin("qq_plot", vec![vals]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_qq_plot_svg() {
    let vals = Value::List(
        (vec![
            Value::Float(0.001),
            Value::Float(0.01),
            Value::Float(0.1),
            Value::Float(0.5),
        ])
        .into(),
    );
    let r = call_bio_plots_builtin("qq_plot", vec![vals, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_qq_plot_all_same_pvalues() {
    let vals = Value::List((vec![Value::Float(0.5), Value::Float(0.5), Value::Float(0.5)]).into());
    let r = call_bio_plots_builtin("qq_plot", vec![vals]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_qq_plot_wrong_type() {
    let r = call_bio_plots_builtin("qq_plot", vec![Value::Str("bad".into())]);
    assert!(r.is_err());
}

#[test]
fn test_qq_plot_empty_after_filter() {
    // All zero / negative p-values get filtered out
    let vals = Value::List((vec![Value::Float(0.0), Value::Float(-1.0)]).into());
    let r = call_bio_plots_builtin("qq_plot", vec![vals]);
    assert!(r.is_err());
}

// ── 3. ideogram ─────────────────────────────────────────────────

#[test]
fn test_ideogram_ascii() {
    let t = make_table(
        vec!["chrom", "start", "end", "band", "stain"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(1e6),
                Value::Str("p11".into()),
                Value::Str("gneg".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(1e6),
                Value::Float(3e6),
                Value::Str("p12".into()),
                Value::Str("gpos25".into()),
            ],
        ],
    );
    let r = call_bio_plots_builtin("ideogram", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_ideogram_svg() {
    let t = make_table(
        vec!["chrom", "start", "end"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(1e6),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Float(0.0),
                Value::Float(2e6),
            ],
        ],
    );
    let r = call_bio_plots_builtin("ideogram", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_ideogram_wrong_type() {
    let r = call_bio_plots_builtin("ideogram", vec![Value::Int(1)]);
    assert!(r.is_err());
}

// ── 4. rainfall ─────────────────────────────────────────────────

#[test]
fn test_rainfall_ascii() {
    let t = make_table(
        vec!["chrom", "pos"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(100.0)],
            vec![Value::Str("chr1".into()), Value::Float(200.0)],
            vec![Value::Str("chr1".into()), Value::Float(500.0)],
            vec![Value::Str("chr2".into()), Value::Float(100.0)],
        ],
    );
    let r = call_bio_plots_builtin("rainfall", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_rainfall_svg() {
    let t = make_table(
        vec!["chrom", "pos"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(100.0)],
            vec![Value::Str("chr1".into()), Value::Float(200.0)],
            vec![Value::Str("chr1".into()), Value::Float(500.0)],
        ],
    );
    let r = call_bio_plots_builtin("rainfall", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_rainfall_insufficient_data() {
    // Only one mutation per chrom = no within-chrom distances
    let t = make_table(
        vec!["chrom", "pos"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(100.0)],
            vec![Value::Str("chr2".into()), Value::Float(200.0)],
        ],
    );
    // Returns Nil with "insufficient data" message (early return before rendering)
    let r = call_bio_plots_builtin("rainfall", vec![t]).unwrap();
    // Insufficient data returns Nil (prints message, no plot generated)
    assert!(matches!(r, Value::Nil | Value::Str(_)));
}

#[test]
fn test_rainfall_wrong_type() {
    let r = call_bio_plots_builtin("rainfall", vec![Value::Float(1.0)]);
    assert!(r.is_err());
}

// ── 5. cnv_plot ─────────────────────────────────────────────────

#[test]
fn test_cnv_plot_ascii() {
    let t = make_table(
        vec!["chrom", "start", "end", "log2ratio"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(1e6),
                Value::Float(0.5),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(1e6),
                Value::Float(2e6),
                Value::Float(-0.3),
            ],
        ],
    );
    let r = call_bio_plots_builtin("cnv_plot", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_cnv_plot_svg() {
    let t = make_table(
        vec!["chrom", "start", "end", "log2ratio"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Float(0.0),
                Value::Float(1e6),
                Value::Float(0.5),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Float(1e6),
                Value::Float(2e6),
                Value::Float(-0.3),
            ],
        ],
    );
    let r = call_bio_plots_builtin("cnv_plot", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_cnv_plot_wrong_type() {
    let r = call_bio_plots_builtin("cnv_plot", vec![Value::Nil]);
    assert!(r.is_err());
}

// ── 6. violin ───────────────────────────────────────────────────

#[test]
fn test_violin_ascii() {
    let t = make_table(
        vec!["group", "value"],
        vec![
            vec![Value::Str("A".into()), Value::Float(1.0)],
            vec![Value::Str("A".into()), Value::Float(2.0)],
            vec![Value::Str("A".into()), Value::Float(3.0)],
            vec![Value::Str("B".into()), Value::Float(5.0)],
            vec![Value::Str("B".into()), Value::Float(6.0)],
            vec![Value::Str("B".into()), Value::Float(7.0)],
        ],
    );
    let r = call_bio_plots_builtin("violin", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_violin_svg() {
    let t = make_table(
        vec!["group", "value"],
        vec![
            vec![Value::Str("A".into()), Value::Float(1.0)],
            vec![Value::Str("A".into()), Value::Float(2.0)],
            vec![Value::Str("A".into()), Value::Float(3.0)],
            vec![Value::Str("B".into()), Value::Float(5.0)],
            vec![Value::Str("B".into()), Value::Float(6.0)],
            vec![Value::Str("B".into()), Value::Float(7.0)],
        ],
    );
    let r = call_bio_plots_builtin("violin", vec![t, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn test_violin_single_value() {
    let vals = Value::List((vec![Value::Float(42.0)]).into());
    let r = call_bio_plots_builtin("violin", vec![vals]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_violin_single_value_svg() {
    let vals = Value::List((vec![Value::Float(42.0)]).into());
    let r = call_bio_plots_builtin("violin", vec![vals, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_violin_wrong_type() {
    let r = call_bio_plots_builtin("violin", vec![Value::Int(1)]);
    assert!(r.is_err());
}

// ── 7. density ──────────────────────────────────────────────────

#[test]
fn test_density_list() {
    let vals = Value::List(
        (vec![
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(4.0),
            Value::Float(5.0),
        ])
        .into(),
    );
    let r = call_bio_plots_builtin("density", vec![vals]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_density_svg() {
    let vals = Value::List(
        (vec![
            Value::Float(1.0),
            Value::Float(2.0),
            Value::Float(3.0),
            Value::Float(4.0),
        ])
        .into(),
    );
    let r = call_bio_plots_builtin("density", vec![vals, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn test_density_two_values_minimum() {
    let vals = Value::List((vec![Value::Float(1.0), Value::Float(2.0)]).into());
    let r = call_bio_plots_builtin("density", vec![vals]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_density_two_values_svg() {
    let vals = Value::List((vec![Value::Float(1.0), Value::Float(2.0)]).into());
    let r = call_bio_plots_builtin("density", vec![vals, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_density_wrong_type() {
    let r = call_bio_plots_builtin("density", vec![Value::Int(1)]);
    assert!(r.is_err());
}

// ── 8. kaplan_meier ─────────────────────────────────────────────

#[test]
fn test_kaplan_meier_ascii() {
    let t = make_table(
        vec!["time", "event"],
        vec![
            vec![Value::Float(1.0), Value::Int(1)],
            vec![Value::Float(2.0), Value::Int(0)],
            vec![Value::Float(3.0), Value::Int(1)],
            vec![Value::Float(5.0), Value::Int(1)],
        ],
    );
    let r = call_bio_plots_builtin("kaplan_meier", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_kaplan_meier_svg() {
    let t = make_table(
        vec!["time", "event"],
        vec![
            vec![Value::Float(1.0), Value::Int(1)],
            vec![Value::Float(3.0), Value::Int(1)],
            vec![Value::Float(5.0), Value::Int(0)],
        ],
    );
    let r = call_bio_plots_builtin("kaplan_meier", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_kaplan_meier_wrong_type() {
    let r = call_bio_plots_builtin("kaplan_meier", vec![Value::List((vec![]).into())]);
    assert!(r.is_err());
}

// ── 9. forest_plot ──────────────────────────────────────────────

#[test]
fn test_forest_plot_ascii() {
    let t = make_table(
        vec!["label", "estimate", "lower", "upper"],
        vec![
            vec![
                Value::Str("Study A".into()),
                Value::Float(1.5),
                Value::Float(0.8),
                Value::Float(2.2),
            ],
            vec![
                Value::Str("Study B".into()),
                Value::Float(0.9),
                Value::Float(0.5),
                Value::Float(1.3),
            ],
        ],
    );
    let r = call_bio_plots_builtin("forest_plot", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_forest_plot_svg() {
    let t = make_table(
        vec!["label", "estimate", "lower", "upper"],
        vec![vec![
            Value::Str("Study A".into()),
            Value::Float(1.5),
            Value::Float(0.8),
            Value::Float(2.2),
        ]],
    );
    let r = call_bio_plots_builtin("forest_plot", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_forest_plot_wrong_type() {
    let r = call_bio_plots_builtin("forest_plot", vec![Value::Int(0)]);
    assert!(r.is_err());
}

// ── 10. roc_curve ───────────────────────────────────────────────

#[test]
fn test_roc_curve_ascii() {
    let t = make_table(
        vec!["score", "label"],
        vec![
            vec![Value::Float(0.9), Value::Int(1)],
            vec![Value::Float(0.7), Value::Int(1)],
            vec![Value::Float(0.4), Value::Int(0)],
            vec![Value::Float(0.2), Value::Int(0)],
        ],
    );
    let r = call_bio_plots_builtin("roc_curve", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_roc_curve_svg() {
    let t = make_table(
        vec!["score", "label"],
        vec![
            vec![Value::Float(0.9), Value::Int(1)],
            vec![Value::Float(0.4), Value::Int(0)],
        ],
    );
    let r = call_bio_plots_builtin("roc_curve", vec![t, svg_opts()]).unwrap();
    if let Value::Str(s) = &r {
        assert!(s.contains("<svg") && s.contains("AUC"));
    } else {
        panic!("expected svg str");
    }
}

#[test]
fn test_roc_curve_perfect_classifier() {
    // Perfect classifier: all positives scored higher than negatives => AUC = 1.0
    let t = make_table(
        vec!["score", "label"],
        vec![
            vec![Value::Float(1.0), Value::Int(1)],
            vec![Value::Float(0.9), Value::Int(1)],
            vec![Value::Float(0.8), Value::Int(1)],
            vec![Value::Float(0.3), Value::Int(0)],
            vec![Value::Float(0.2), Value::Int(0)],
            vec![Value::Float(0.1), Value::Int(0)],
        ],
    );
    let r = call_bio_plots_builtin("roc_curve", vec![t, svg_opts()]).unwrap();
    if let Value::Str(s) = &r {
        assert!(
            s.contains("AUC = 1.000"),
            "perfect classifier should have AUC=1, got {}",
            s
        );
    } else {
        panic!("expected svg str");
    }
}

#[test]
fn test_roc_curve_random_classifier() {
    // Alternating labels with uniform scores => AUC near 0.5
    let mut rows = Vec::new();
    for i in 0..100 {
        let score = i as f64 / 100.0;
        let label = if i % 2 == 0 { 1 } else { 0 };
        rows.push(vec![Value::Float(score), Value::Int(label)]);
    }
    let t = make_table(vec!["score", "label"], rows);
    let r = call_bio_plots_builtin("roc_curve", vec![t, svg_opts()]).unwrap();
    if let Value::Str(s) = &r {
        // Extract AUC value from "AUC = X.XXX"
        if let Some(idx) = s.find("AUC = ") {
            let auc_str = &s[idx + 6..idx + 11];
            let auc: f64 = auc_str.parse().unwrap_or(0.0);
            assert!(
                (auc - 0.5).abs() < 0.1,
                "random classifier AUC should be near 0.5, got {auc}"
            );
        }
    } else {
        panic!("expected svg str");
    }
}

#[test]
fn test_roc_curve_wrong_type() {
    let r = call_bio_plots_builtin("roc_curve", vec![Value::Nil]);
    assert!(r.is_err());
}

// ── 11. clustered_heatmap ───────────────────────────────────────

#[test]
fn test_clustered_heatmap_ascii() {
    let m = Value::Matrix(
        Matrix {
            data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            nrow: 2,
            ncol: 3,
            row_names: Some(vec!["r1".into(), "r2".into()]),
            col_names: Some(vec!["c1".into(), "c2".into(), "c3".into()]),
        }
        .into(),
    );
    let r = call_bio_plots_builtin("clustered_heatmap", vec![m]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_clustered_heatmap_svg() {
    let m = Value::Matrix(
        Matrix {
            data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            nrow: 2,
            ncol: 3,
            row_names: Some(vec!["r1".into(), "r2".into()]),
            col_names: Some(vec!["c1".into(), "c2".into(), "c3".into()]),
        }
        .into(),
    );
    let r = call_bio_plots_builtin("clustered_heatmap", vec![m, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_clustered_heatmap_wrong_type() {
    let r = call_bio_plots_builtin("clustered_heatmap", vec![Value::Int(0)]);
    assert!(r.is_err());
}

#[test]
fn clustered_heatmap_freezes_nearest_neighbour_row_and_column_order() {
    let matrix = Value::Matrix(
        Matrix {
            data: vec![0.0, 100.0, 1.0, 10.0, 100.0, 11.0, 1.0, 100.0, 2.0],
            nrow: 3,
            ncol: 3,
            row_names: Some(vec!["r0".into(), "r1".into(), "r2".into()]),
            col_names: Some(vec!["c0".into(), "c1".into(), "c2".into()]),
        }
        .into(),
    );
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("theme", Value::Str("publication".into())),
    ]);
    let Value::Str(svg) =
        call_bio_plots_builtin("clustered_heatmap", vec![matrix, opts]).expect("clustered heatmap")
    else {
        panic!("expected SVG")
    };
    assert!(svg.find(">r0<").unwrap() < svg.find(">r2<").unwrap());
    assert!(svg.find(">r2<").unwrap() < svg.find(">r1<").unwrap());
    assert!(svg.find(">c0<").unwrap() < svg.find(">c2<").unwrap());
    assert!(svg.find(">c2<").unwrap() < svg.find(">c1<").unwrap());
}

#[test]
fn hierarchical_heatmap_matches_base_r_hclust_leaf_order_and_heights() {
    // Oracle: base R 4.5.2, hclust(dist(x), method = ...). These assertions
    // are scale-sensitive: correlation alone cannot hide wrong merge heights.
    let matrix = || {
        Value::Matrix(
            Matrix {
                data: vec![
                    0.0, 0.0, 0.0, 10.0, 10.0, 10.0, 1.0, 1.0, 1.0, 9.0, 9.0, 9.0, 4.0, 5.0, 4.0,
                ],
                nrow: 5,
                ncol: 3,
                row_names: Some((0..5).map(|index| format!("r{index}")).collect()),
                col_names: Some((0..3).map(|index| format!("c{index}")).collect()),
            }
            .into(),
        )
    };
    let expected = [
        (
            "complete",
            "1.732050807569,1.732050807569,7.549834435271,17.320508075689",
        ),
        (
            "average",
            "1.732050807569,1.732050807569,6.690393165058,13.387787546485",
        ),
        (
            "single",
            "1.732050807569,1.732050807569,5.830951894845,8.124038404636",
        ),
        (
            "ward.D2",
            "1.732050807569,1.732050807569,7.724420150838,20.725185966194",
        ),
    ];
    for (linkage, heights) in expected {
        let opts = make_opts(vec![
            ("format", Value::Str("svg".into())),
            ("theme", Value::Str("publication".into())),
            ("order", Value::Str("hierarchical".into())),
            ("distance", Value::Str("euclidean".into())),
            ("linkage", Value::Str(linkage.into())),
            ("dendrogram", Value::Str("both".into())),
        ]);
        let Value::Str(svg) =
            call_bio_plots_builtin("clustered_heatmap", vec![matrix(), opts]).unwrap()
        else {
            panic!("expected SVG")
        };
        assert!(svg.contains("data-biolang-clustering=\"hierarchical\""));
        assert!(svg.contains("data-row-order=\"1,3,4,0,2\""));
        assert!(
            svg.contains(&format!("data-row-heights=\"{heights}\"")),
            "{linkage} heights differed from base R: {svg}"
        );
        assert!(svg.contains(&format!("data-linkage=\"{linkage}\"")));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }
}

#[test]
fn hierarchical_heatmap_validates_options_and_ward_distance() {
    let matrix = || {
        Value::Matrix(
            Matrix {
                data: vec![0.0, 1.0, 2.0, 3.0],
                nrow: 2,
                ncol: 2,
                row_names: None,
                col_names: None,
            }
            .into(),
        )
    };
    for opts in [
        make_opts(vec![("order", Value::Str("mystery".into()))]),
        make_opts(vec![
            ("order", Value::Str("hierarchical".into())),
            ("linkage", Value::Str("centroid".into())),
        ]),
        make_opts(vec![
            ("order", Value::Str("hierarchical".into())),
            ("distance", Value::Str("cosine".into())),
        ]),
        make_opts(vec![
            ("order", Value::Str("hierarchical".into())),
            ("linkage", Value::Str("ward.D2".into())),
            ("distance", Value::Str("manhattan".into())),
        ]),
        make_opts(vec![("dendrogram", Value::Str("both".into()))]),
    ] {
        assert!(call_bio_plots_builtin("clustered_heatmap", vec![matrix(), opts]).is_err());
    }
}

#[test]
fn hierarchical_heatmap_supports_pearson_distance_for_expression_profiles() {
    let matrix = Value::Matrix(
        Matrix {
            data: vec![1.0, 2.0, 3.0, 2.0, 4.0, 6.0, 3.0, 2.0, 1.0],
            nrow: 3,
            ncol: 3,
            row_names: Some(vec!["same-a".into(), "same-b".into(), "opposite".into()]),
            col_names: Some(vec!["s1".into(), "s2".into(), "s3".into()]),
        }
        .into(),
    );
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("order", Value::Str("hierarchical".into())),
        ("distance", Value::Str("pearson".into())),
        ("linkage", Value::Str("complete".into())),
        ("dendrogram", Value::Str("row".into())),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("clustered_heatmap", vec![matrix, opts]).unwrap()
    else {
        panic!("expected SVG")
    };
    assert!(svg.contains("data-distance=\"pearson\""));
    assert!(svg.contains("data-row-heights=\"0.000000000000,2.000000000000\""));
}

#[test]
fn hierarchical_heatmap_can_pin_leaf_orientation_and_annotate_columns() {
    let matrix = Value::Matrix(
        Matrix {
            data: vec![1.0, 2.0, 3.0, 3.0, 2.0, 1.0],
            nrow: 2,
            ncol: 3,
            row_names: Some(vec!["gene-a".into(), "gene-b".into()]),
            col_names: Some(vec![
                "sample-a".into(),
                "sample-b".into(),
                "sample-c".into(),
            ]),
        }
        .into(),
    );
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("theme", Value::Str("publication".into())),
        ("order", Value::Str("hierarchical".into())),
        ("dendrogram", Value::Str("both".into())),
        (
            "row_order",
            Value::List(vec![Value::Str("gene-a".into()), Value::Str("gene-b".into())].into()),
        ),
        (
            "column_order",
            Value::List(
                vec![
                    Value::Str("sample-a".into()),
                    Value::Str("sample-b".into()),
                    Value::Str("sample-c".into()),
                ]
                .into(),
            ),
        ),
        (
            "column_annotations",
            Value::Record(
                HashMap::from([(
                    "condition".into(),
                    Value::List(
                        vec![
                            Value::Str("control".into()),
                            Value::Str("treated".into()),
                            Value::Str("treated".into()),
                        ]
                        .into(),
                    ),
                )])
                .into(),
            ),
        ),
        (
            "annotation_colors",
            Value::Record(
                HashMap::from([(
                    "condition".into(),
                    Value::Record(
                        HashMap::from([
                            ("control".into(), Value::Str("#9999FF".into())),
                            ("treated".into(), Value::Str("#FF66FF".into())),
                        ])
                        .into(),
                    ),
                )])
                .into(),
            ),
        ),
        ("row_labels_side", Value::Str("right".into())),
        ("row_dendrogram_width", Value::Float(58.0)),
        ("column_dendrogram_height", Value::Float(65.0)),
        ("annotation_row_height", Value::Float(10.0)),
        ("annotation_gap", Value::Float(3.0)),
        ("dendrogram_gap", Value::Float(3.0)),
        ("top_padding", Value::Float(0.0)),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("clustered_heatmap", vec![matrix, opts]).unwrap()
    else {
        panic!("expected SVG")
    };
    assert!(svg.contains("data-row-order=\"0,1\""));
    assert!(svg.contains("data-column-order=\"0,1,2\""));
    assert!(svg.contains("data-biolang-column-annotations=\"condition\""));
    assert!(svg.contains("#9999FF") && svg.contains("#FF66FF"));
    assert!(svg.contains(">condition</text>"));
    assert!(
        svg.matches("<line ").count() >= 9,
        "both trees should remain visible"
    );
    let row_label = svg.find(">gene-a</text>").expect("row label");
    assert!(svg[row_label.saturating_sub(180)..row_label].contains("text-anchor=\"start\""));
}

#[test]
fn hierarchical_heatmap_survives_notebook_and_journal_widths() {
    for width in [321_i64, 680, 800] {
        let matrix = Value::Matrix(
            Matrix {
                data: vec![-2.0, 0.0, 1.0, 2.0, -1.0, 0.5, 1.5, 2.5, -0.5],
                nrow: 3,
                ncol: 3,
                row_names: Some(vec!["TP53".into(), "BRCA1".into(), "EGFR".into()]),
                col_names: Some(vec!["control".into(), "treated".into(), "recovery".into()]),
            }
            .into(),
        );
        let opts = make_opts(vec![
            ("format", Value::Str("svg".into())),
            ("theme", Value::Str("publication".into())),
            ("width", Value::Int(width)),
            ("height", Value::Int(400)),
            ("order", Value::Str("hierarchical".into())),
            ("linkage", Value::Str("complete".into())),
            ("dendrogram", Value::Str("both".into())),
        ]);
        let Value::Str(svg) =
            call_bio_plots_builtin("clustered_heatmap", vec![matrix, opts]).unwrap()
        else {
            panic!("expected SVG")
        };
        assert!(svg.contains(&format!("width=\"{width}\"")));
        assert!(svg.contains("data-dendrogram=\"both\""));
        assert!(svg.matches("<line ").count() >= 12);
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }
}

#[test]
fn publication_clustered_heatmap_survives_notebook_and_journal_widths() {
    let matrix = || {
        Value::Matrix(
            Matrix {
                data: vec![-2.0, 0.0, 1.0, 2.0, -1.0, 0.5, 1.5, 2.5, -0.5],
                nrow: 3,
                ncol: 3,
                row_names: Some(vec!["TP53".into(), "BRCA1".into(), "EGFR".into()]),
                col_names: Some(vec!["control".into(), "treated".into(), "recovery".into()]),
            }
            .into(),
        )
    };
    for width in [321_i64, 680, 800] {
        let opts = Value::Record(
            HashMap::from([
                ("format".into(), Value::Str("svg".into())),
                ("theme".into(), Value::Str("publication".into())),
                ("width".into(), Value::Int(width)),
                ("height".into(), Value::Int(400)),
                ("title".into(), Value::Str("Marker panel".into())),
                ("subtitle".into(), Value::Str("Mean expression".into())),
                (
                    "caption".into(),
                    Value::Str("Nearest-neighbour order disclosed".into()),
                ),
                ("legend_title".into(), Value::Str("z-score".into())),
                ("center".into(), Value::Float(0.0)),
            ])
            .into(),
        );
        let Value::Str(svg) = call_bio_plots_builtin("clustered_heatmap", vec![matrix(), opts])
            .expect("publication clustered heatmap")
        else {
            panic!("expected SVG")
        };
        assert!(svg.contains(&format!("width=\"{width}\"")));
        assert!(svg.contains("data-biolang-theme=\"publication\""));
        assert!(svg.contains(">Mean expression<"));
        assert!(svg.contains(">Nearest-neighbour order disclosed<"));
        assert!(svg.contains(">TP53<"));
        assert!(svg.contains(">treated<"));
        assert!(svg.contains("#3b4cc0"));
        assert!(svg.contains("#b40426"));
        assert!(!svg.contains("NaN"));
        assert!(!svg.contains("inf"));
    }
}

#[test]
fn clustered_heatmap_accepts_a_gene_annotation_column() {
    let table = make_table(
        vec!["gene", "cluster 0", "cluster 1"],
        vec![
            vec![
                Value::Str("CD3D".into()),
                Value::Float(3.0),
                Value::Float(0.1),
            ],
            vec![
                Value::Str("MS4A1".into()),
                Value::Float(0.2),
                Value::Float(4.0),
            ],
        ],
    );
    let opts = Value::Record(
        HashMap::from([
            ("format".into(), Value::Str("svg".into())),
            ("theme".into(), Value::Str("publication".into())),
        ])
        .into(),
    );
    let Value::Str(svg) = call_bio_plots_builtin("clustered_heatmap", vec![table, opts]).unwrap()
    else {
        panic!("expected SVG")
    };
    assert!(svg.contains(">CD3D<"));
    assert!(svg.contains(">MS4A1<"));
    assert!(svg.contains(">cluster 0<"));
    assert!(svg.contains(">cluster 1<"));
}

// ── 12. pca_plot ────────────────────────────────────────────────

#[test]
fn test_pca_plot_ascii() {
    let t = make_table(
        vec!["PC1", "PC2", "label"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0), Value::Str("A".into())],
            vec![
                Value::Float(-1.0),
                Value::Float(-0.5),
                Value::Str("B".into()),
            ],
            vec![Value::Float(0.5), Value::Float(1.0), Value::Str("A".into())],
        ],
    );
    let r = call_bio_plots_builtin("pca_plot", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_pca_plot_svg() {
    let t = make_table(
        vec!["PC1", "PC2"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0)],
            vec![Value::Float(-1.0), Value::Float(-0.5)],
        ],
    );
    let r = call_bio_plots_builtin("pca_plot", vec![t, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn pca_plot_can_render_pinned_precomputed_scores_with_ggplot_colours() {
    let table = make_table(
        vec!["sample", "dex", "PC1", "PC2"],
        vec![
            vec![
                Value::Str("S1".into()),
                Value::Str("control".into()),
                Value::Float(-2.0),
                Value::Float(1.0),
            ],
            vec![
                Value::Str("S2".into()),
                Value::Str("treated".into()),
                Value::Float(2.0),
                Value::Float(-1.0),
            ],
        ],
    );
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("precomputed", Value::Bool(true)),
        ("group_col", Value::Str("dex".into())),
        ("palette", Value::Str("ggplot".into())),
        ("legend_title", Value::Str("group".into())),
        ("pc1_variance_percent", Value::Float(32.0)),
        ("pc2_variance_percent", Value::Float(24.0)),
        ("x_label", Value::Str("PC1: 32% variance".into())),
        ("y_label", Value::Str("PC2: 24% variance".into())),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("pca_plot", vec![table, opts]).unwrap() else {
        panic!("expected SVG")
    };
    assert!(svg.contains("#f8766d"));
    assert!(svg.contains("#00bfc4"));
    assert!(svg.contains("PC1: 32% variance"));
    assert!(svg.contains("PC2: 24% variance"));
    assert!(svg.contains(">group</text>"));
}

#[test]
fn pca_plot_can_reproduce_the_white_gridded_plotpca_panel() {
    let table = make_table(
        vec!["sample", "dex", "PC1", "PC2"],
        vec![
            vec![
                Value::Str("S1".into()),
                Value::Str("control".into()),
                Value::Float(-2.0),
                Value::Float(1.0),
            ],
            vec![
                Value::Str("S2".into()),
                Value::Str("treated".into()),
                Value::Float(2.0),
                Value::Float(-1.0),
            ],
        ],
    );
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("precomputed", Value::Bool(true)),
        ("group_col", Value::Str("dex".into())),
        ("theme", Value::Str("publication".into())),
        ("panel_border", Value::Bool(true)),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("pca_plot", vec![table, opts]).unwrap() else {
        panic!("expected SVG")
    };
    assert!(svg.matches("stroke=\"#e5e7eb\"").count() >= 4);
    assert!(svg.contains("fill=\"none\" stroke=\"#303238\""));
}

#[test]
fn test_pca_plot_wrong_type() {
    let r = call_bio_plots_builtin("pca_plot", vec![Value::Int(0)]);
    assert!(r.is_err());
}

#[test]
fn test_pca_plot_too_few_columns() {
    let t = make_table(
        vec!["PC1"],
        vec![vec![Value::Float(1.0)], vec![Value::Float(2.0)]],
    );
    let r = call_bio_plots_builtin("pca_plot", vec![t]);
    assert!(r.is_err());
}

// ── 13. oncoprint ───────────────────────────────────────────────

#[test]
fn test_oncoprint_ascii() {
    let t = make_table(
        vec!["gene", "sample", "type"],
        vec![
            vec![
                Value::Str("TP53".into()),
                Value::Str("S1".into()),
                Value::Str("missense".into()),
            ],
            vec![
                Value::Str("TP53".into()),
                Value::Str("S2".into()),
                Value::Str("nonsense".into()),
            ],
            vec![
                Value::Str("BRCA1".into()),
                Value::Str("S1".into()),
                Value::Str("deletion".into()),
            ],
        ],
    );
    let r = call_bio_plots_builtin("oncoprint", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_oncoprint_svg() {
    let t = make_table(
        vec!["gene", "sample", "type"],
        vec![vec![
            Value::Str("TP53".into()),
            Value::Str("S1".into()),
            Value::Str("missense".into()),
        ]],
    );
    let r = call_bio_plots_builtin("oncoprint", vec![t, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn test_oncoprint_wrong_type() {
    let r = call_bio_plots_builtin("oncoprint", vec![Value::Float(1.0)]);
    assert!(r.is_err());
}

// ── 14. venn ────────────────────────────────────────────────────

#[test]
fn test_venn_ascii() {
    let rec = Value::Record(
        (HashMap::from([
            (
                "A".to_string(),
                Value::List((vec![Value::Int(1), Value::Int(2), Value::Int(3)]).into()),
            ),
            (
                "B".to_string(),
                Value::List((vec![Value::Int(2), Value::Int(3), Value::Int(4)]).into()),
            ),
        ]))
        .into(),
    );
    let r = call_bio_plots_builtin("venn", vec![rec]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_venn_svg() {
    let rec = Value::Record(
        (HashMap::from([
            (
                "A".to_string(),
                Value::List((vec![Value::Int(1), Value::Int(2)]).into()),
            ),
            (
                "B".to_string(),
                Value::List((vec![Value::Int(2), Value::Int(3)]).into()),
            ),
        ]))
        .into(),
    );
    let r = call_bio_plots_builtin("venn", vec![rec, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn test_venn_completely_overlapping() {
    let rec = Value::Record(
        (HashMap::from([
            (
                "A".to_string(),
                Value::List((vec![Value::Int(1), Value::Int(2), Value::Int(3)]).into()),
            ),
            (
                "B".to_string(),
                Value::List((vec![Value::Int(1), Value::Int(2), Value::Int(3)]).into()),
            ),
        ]))
        .into(),
    );
    let r = call_bio_plots_builtin("venn", vec![rec]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_venn_disjoint_sets() {
    let rec = Value::Record(
        (HashMap::from([
            (
                "A".to_string(),
                Value::List((vec![Value::Int(1), Value::Int(2)]).into()),
            ),
            (
                "B".to_string(),
                Value::List((vec![Value::Int(3), Value::Int(4)]).into()),
            ),
        ]))
        .into(),
    );
    let r = call_bio_plots_builtin("venn", vec![rec]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_venn_disjoint_svg() {
    let rec = Value::Record(
        (HashMap::from([
            ("X".to_string(), Value::List((vec![Value::Int(10)]).into())),
            ("Y".to_string(), Value::List((vec![Value::Int(20)]).into())),
        ]))
        .into(),
    );
    let r = call_bio_plots_builtin("venn", vec![rec, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_venn_wrong_type() {
    let r = call_bio_plots_builtin("venn", vec![Value::Int(1)]);
    assert!(r.is_err());
}

#[test]
fn test_venn_too_few_sets() {
    let rec = Value::Record(
        (HashMap::from([("A".to_string(), Value::List((vec![Value::Int(1)]).into()))])).into(),
    );
    let r = call_bio_plots_builtin("venn", vec![rec]);
    assert!(r.is_err());
}

// ── 15. upset ───────────────────────────────────────────────────

#[test]
fn test_upset_ascii() {
    let rec = Value::Record(
        (HashMap::from([
            (
                "A".to_string(),
                Value::List((vec![Value::Int(1), Value::Int(2), Value::Int(3)]).into()),
            ),
            (
                "B".to_string(),
                Value::List((vec![Value::Int(2), Value::Int(3), Value::Int(4)]).into()),
            ),
            (
                "C".to_string(),
                Value::List((vec![Value::Int(3), Value::Int(5)]).into()),
            ),
        ]))
        .into(),
    );
    let r = call_bio_plots_builtin("upset", vec![rec]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_upset_svg() {
    let rec = Value::Record(
        (HashMap::from([
            (
                "A".to_string(),
                Value::List((vec![Value::Int(1), Value::Int(2)]).into()),
            ),
            (
                "B".to_string(),
                Value::List((vec![Value::Int(2), Value::Int(3)]).into()),
            ),
        ]))
        .into(),
    );
    let r = call_bio_plots_builtin("upset", vec![rec, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn test_upset_wrong_type() {
    let r = call_bio_plots_builtin("upset", vec![Value::Int(0)]);
    assert!(r.is_err());
}

#[test]
fn test_upset_too_few_sets() {
    let rec = Value::Record(
        (HashMap::from([("A".to_string(), Value::List((vec![Value::Int(1)]).into()))])).into(),
    );
    let r = call_bio_plots_builtin("upset", vec![rec]);
    assert!(r.is_err());
}

// ── 16. sequence_logo ───────────────────────────────────────────

#[test]
fn test_sequence_logo_ascii() {
    let seqs = Value::List(
        (vec![
            Value::Str("ACGT".into()),
            Value::Str("ACGT".into()),
            Value::Str("ACGA".into()),
        ])
        .into(),
    );
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_sequence_logo_svg() {
    let seqs = Value::List((vec![Value::Str("ACGT".into()), Value::Str("ACGT".into())]).into());
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn test_sequence_logo_single_sequence() {
    let seqs = Value::List((vec![Value::Str("ATCG".into())]).into());
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_sequence_logo_single_sequence_svg() {
    let seqs = Value::List((vec![Value::Str("ATCGATCG".into())]).into());
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_sequence_logo_empty_list() {
    let seqs = Value::List((vec![]).into());
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs]);
    assert!(r.is_err());
}

#[test]
fn test_sequence_logo_wrong_type() {
    let r = call_bio_plots_builtin("sequence_logo", vec![Value::Int(0)]);
    assert!(r.is_err());
}

// ── 17. phylo_tree ──────────────────────────────────────────────

#[test]
fn test_phylo_tree_ascii() {
    let newick = Value::Str("((A:0.1,B:0.2):0.3,C:0.4);".into());
    let r = call_bio_plots_builtin("phylo_tree", vec![newick]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_phylo_tree_svg() {
    let newick = Value::Str("((A:0.1,B:0.2):0.3,C:0.4);".into());
    let r = call_bio_plots_builtin("phylo_tree", vec![newick, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn phylo_tree_rectangular_connectors_join_immediate_children() {
    let newick = Value::Str("((A:1,B:1):1,(C:1,D:1):1);".into());
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("theme", Value::Str("publication".into())),
        ("title", Value::Str("".into())),
        ("width", Value::Int(400)),
        ("height", Value::Int(400)),
        ("show_tip_labels", Value::Bool(false)),
        ("show_tip_points", Value::Bool(false)),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("phylo_tree", vec![newick, opts]).unwrap() else {
        panic!("expected SVG")
    };

    // The root joins the two child-node centres (111 and 289), not the
    // descendant-tip extrema (66.5 and 333.5).
    assert!(svg.contains(r##"<line x1="30.0" y1="111.0" x2="30.0" y2="289.0" stroke="#111111""##));
    assert!(!svg.contains(r#"x1="30.0" y1="66.5" x2="30.0" y2="333.5""#));
}

#[test]
fn phylo_tree_uses_ggtree_dotted_line_rhythm() {
    let newick = Value::Str("(A:1,B:1);".into());
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("line_type", Value::Str("3".into())),
        ("line_width", Value::Float(2.0)),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("phylo_tree", vec![newick, opts]).unwrap() else {
        panic!("expected SVG")
    };
    assert!(svg.contains(r#"stroke-dasharray="2.0,6.0""#));
}

#[test]
fn test_phylo_tree_wrong_type() {
    let r = call_bio_plots_builtin("phylo_tree", vec![Value::Int(0)]);
    assert!(r.is_err());
}

#[test]
fn phylo_tree_reproduces_ggtree_numbering_annotations_and_aesthetics() {
    let newick = Value::Str(
        "(((((((A:4,B:4):6,C:5):8,D:6):3,E:21):10,((F:4,G:12):14,H:8):13):13,((I:5,J:2):30,(K:11,L:11):2):17):4,M:56);".into(),
    );
    let record = |values: Vec<(&str, Value)>| {
        Value::Record(
            values
                .into_iter()
                .map(|(key, value)| (key.to_string(), value))
                .collect::<HashMap<_, _>>()
                .into(),
        )
    };
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("theme", Value::Str("publication".into())),
        ("show_tip_labels", Value::Bool(true)),
        ("show_tip_points", Value::Bool(true)),
        ("show_node_points", Value::Bool(true)),
        ("show_node_labels", Value::Bool(true)),
        ("scale_axis", Value::Bool(true)),
        ("tip_shape", Value::Str("diamond".into())),
        ("tip_color", Value::Str("#9932CC".into())),
        ("tip_label_color", Value::Str("#9932CC".into())),
        (
            "tip_order",
            Value::List(
                [
                    "B", "A", "C", "D", "E", "G", "F", "H", "L", "K", "J", "I", "M",
                ]
                .into_iter()
                .map(|value| Value::Str(value.into()))
                .collect::<Vec<_>>()
                .into(),
            ),
        ),
        (
            "clade_highlights",
            Value::List(
                vec![record(vec![
                    ("node", Value::Int(19)),
                    ("fill", Value::Str("#3333CC".into())),
                ])]
                .into(),
            ),
        ),
        (
            "clade_labels",
            Value::List(
                vec![record(vec![
                    ("node", Value::Int(17)),
                    ("label", Value::Str("Superclade 17".into())),
                    ("color", Value::Str("#CC2200".into())),
                ])]
                .into(),
            ),
        ),
        (
            "taxa_links",
            Value::List(
                vec![record(vec![
                    ("from", Value::Str("C".into())),
                    ("to", Value::Str("E".into())),
                    ("dashed", Value::Bool(true)),
                ])]
                .into(),
            ),
        ),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("phylo_tree", vec![newick, opts]).unwrap() else {
        panic!("expected SVG")
    };
    assert!(svg.contains("data-clade-node=\"19\""));
    assert!(svg.contains("data-taxa-link=\"C,E\""));
    assert!(svg.contains("stroke-dasharray=\"5,5\""));
    assert!(svg.contains(">Superclade 17</text>"));
    assert!(svg.contains(">17</text>") && svg.contains(">21</text>"));
    assert!(svg.contains("#9932CC"));
    assert!(svg.contains(">B</text>") && svg.contains(">M</text>"));
}

#[test]
fn phylo_tree_supports_slanted_circular_and_faceted_layouts() {
    let newick = Value::Str("((A:1,B:1):1,(C:1,D:1):1);".into());
    for layout in ["slanted", "circular"] {
        let opts = make_opts(vec![
            ("format", Value::Str("svg".into())),
            ("layout", Value::Str(layout.into())),
            ("branch_length", Value::Str("none".into())),
            ("show_tip_labels", Value::Bool(false)),
            ("line_color", Value::Str("red".into())),
        ]);
        let Value::Str(svg) =
            call_bio_plots_builtin("phylo_tree", vec![newick.clone(), opts]).unwrap()
        else {
            panic!("expected SVG")
        };
        assert!(svg.contains("stroke=\"red\""));
        assert!(svg.contains(&format!("in {layout} layout")));
    }

    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("columns", Value::Int(2)),
        ("title", Value::Str("Many trees".into())),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin(
        "phylo_tree",
        vec![Value::List(vec![newick.clone(), newick].into()), opts],
    )
    .unwrap() else {
        panic!("expected SVG")
    };
    assert!(svg.contains(">Tree #1</text>") && svg.contains(">Tree #2</text>"));

    // ggtree's multiPhylo facets number tips from the bottom upward.  Keep
    // that orientation separate from the labelled single-tree renderer,
    // whose explicit tip_order is read from top to bottom.
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("columns", Value::Int(1)),
        ("title", Value::Str("Faceted tree".into())),
        ("width", Value::Float(600.0)),
        ("height", Value::Float(400.0)),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin(
        "phylo_tree",
        vec![
            Value::List(vec![Value::Str("(A:1,B:3);".into())].into()),
            opts,
        ],
    )
    .unwrap() else {
        panic!("expected SVG")
    };
    assert!(
        svg.contains(r#"x1="20.5" y1="303.8" x2="207.9" y2="303.8""#),
        "the first traversed tip must occupy the lower faceted-tree row"
    );
}

#[test]
fn phylo_tree_rejects_incomplete_tip_order() {
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        (
            "tip_order",
            Value::List(vec![Value::Str("A".into())].into()),
        ),
    ]);
    let error = call_bio_plots_builtin("phylo_tree", vec![Value::Str("(A:1,B:1);".into()), opts])
        .unwrap_err();
    assert!(error.to_string().contains("tip_order must name every leaf"));
}

#[test]
fn alignment_view_supports_clustal_style_protein_colours() {
    let alignment = make_table(
        vec!["id", "seq"],
        vec![
            vec![Value::Str("one".into()), Value::Str("AKDEC".into())],
            vec![Value::Str("two".into()), Value::Str("AKDEC".into())],
        ],
    );
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("color_by", Value::Str("protein".into())),
    ]);
    let Value::Str(svg) = call_bio_plots_builtin("alignment_view", vec![alignment, opts]).unwrap()
    else {
        panic!("alignment_view should return SVG");
    };
    assert!(
        svg.contains("#80A0F0"),
        "hydrophobic residues need a protein colour"
    );
    assert!(
        svg.contains("#F01505"),
        "positive residues need a protein colour"
    );
    assert!(
        svg.contains("#C048C0"),
        "negative residues need a protein colour"
    );
    assert!(svg.contains("#F08080"), "cysteine needs a protein colour");
}

// ── 18. lollipop ────────────────────────────────────────────────

#[test]
fn test_lollipop_ascii() {
    let t = make_table(
        vec!["position", "count"],
        vec![
            vec![Value::Float(100.0), Value::Float(5.0)],
            vec![Value::Float(200.0), Value::Float(10.0)],
            vec![Value::Float(350.0), Value::Float(3.0)],
        ],
    );
    let r = call_bio_plots_builtin("lollipop", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_lollipop_svg() {
    let t = make_table(
        vec!["position", "count"],
        vec![
            vec![Value::Float(100.0), Value::Float(5.0)],
            vec![Value::Float(200.0), Value::Float(10.0)],
        ],
    );
    let r = call_bio_plots_builtin("lollipop", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_lollipop_wrong_type() {
    let r = call_bio_plots_builtin("lollipop", vec![Value::Nil]);
    assert!(r.is_err());
}

// ── 19. circos ──────────────────────────────────────────────────

#[test]
fn test_circos_ascii() {
    let t = make_table(
        vec!["chrom", "end"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(1e6)],
            vec![Value::Str("chr2".into()), Value::Float(2e6)],
        ],
    );
    let r = call_bio_plots_builtin("circos", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_circos_svg() {
    let t = make_table(
        vec!["chrom", "end"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(1e6)],
            vec![Value::Str("chr2".into()), Value::Float(2e6)],
        ],
    );
    let r = call_bio_plots_builtin("circos", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_circos_wrong_type() {
    let r = call_bio_plots_builtin("circos", vec![Value::Int(0)]);
    assert!(r.is_err());
}

// ── 20. hic_map ─────────────────────────────────────────────────

#[test]
fn test_hic_map_ascii() {
    let m = Value::Matrix(
        Matrix {
            data: vec![10.0, 5.0, 1.0, 5.0, 8.0, 3.0, 1.0, 3.0, 9.0],
            nrow: 3,
            ncol: 3,
            row_names: Some(vec!["bin1".into(), "bin2".into(), "bin3".into()]),
            col_names: Some(vec!["bin1".into(), "bin2".into(), "bin3".into()]),
        }
        .into(),
    );
    let r = call_bio_plots_builtin("hic_map", vec![m]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_hic_map_svg() {
    let m = Value::Matrix(
        Matrix {
            data: vec![10.0, 5.0, 5.0, 8.0],
            nrow: 2,
            ncol: 2,
            row_names: None,
            col_names: None,
        }
        .into(),
    );
    let r = call_bio_plots_builtin("hic_map", vec![m, publication_svg_opts()]).unwrap();
    assert_publication_svg(&r);
}

#[test]
fn test_hic_map_wrong_type() {
    let r = call_bio_plots_builtin("hic_map", vec![Value::Nil]);
    assert!(r.is_err());
}

// ── 21. sashimi ─────────────────────────────────────────────────

#[test]
fn test_sashimi_ascii() {
    let t = make_table(
        vec!["chrom", "start", "end", "junctions"],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(100.0),
            Value::Float(500.0),
            Value::Str("200-400:10".into()),
        ]],
    );
    let r = call_bio_plots_builtin("sashimi", vec![t]).unwrap();
    assert!(matches!(r, Value::Str(_)), "expected Str output, got {r:?}");
}

#[test]
fn test_sashimi_svg() {
    let t = make_table(
        vec!["chrom", "start", "end", "junctions"],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(100.0),
            Value::Float(500.0),
            Value::Str("200-400:10".into()),
        ]],
    );
    let r = call_bio_plots_builtin("sashimi", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_sashimi_wrong_type() {
    let r = call_bio_plots_builtin("sashimi", vec![Value::Int(0)]);
    assert!(r.is_err());
}

// ── Unknown builtin ─────────────────────────────────────────────

#[test]
fn test_unknown_builtin() {
    let r = call_bio_plots_builtin("nonexistent", vec![]);
    assert!(r.is_err());
}

// ── SVG output validation for all SVG-mode plots ────────────────

#[test]
fn test_all_svg_plots_contain_svg_tag() {
    // ideogram svg
    let t = make_table(
        vec!["chrom", "start", "end"],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(0.0),
            Value::Float(1e6),
        ]],
    );
    let r = call_bio_plots_builtin("ideogram", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);

    // rainfall svg
    let t = make_table(
        vec!["chrom", "pos"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(100.0)],
            vec![Value::Str("chr1".into()), Value::Float(200.0)],
            vec![Value::Str("chr1".into()), Value::Float(500.0)],
        ],
    );
    let r = call_bio_plots_builtin("rainfall", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);

    // cnv svg
    let t = make_table(
        vec!["chrom", "start", "end", "log2ratio"],
        vec![vec![
            Value::Str("chr1".into()),
            Value::Float(0.0),
            Value::Float(1e6),
            Value::Float(0.5),
        ]],
    );
    let r = call_bio_plots_builtin("cnv_plot", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);

    // kaplan_meier svg
    let t = make_table(
        vec!["time", "event"],
        vec![
            vec![Value::Float(1.0), Value::Int(1)],
            vec![Value::Float(5.0), Value::Int(0)],
        ],
    );
    let r = call_bio_plots_builtin("kaplan_meier", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);

    // oncoprint svg
    let t = make_table(
        vec!["gene", "sample", "type"],
        vec![vec![
            Value::Str("TP53".into()),
            Value::Str("S1".into()),
            Value::Str("missense".into()),
        ]],
    );
    let r = call_bio_plots_builtin("oncoprint", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);

    // lollipop svg
    let t = make_table(
        vec!["position", "count"],
        vec![vec![Value::Float(100.0), Value::Float(5.0)]],
    );
    let r = call_bio_plots_builtin("lollipop", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

// ── Manhattan plot advanced tests ─────────────────────────────

#[test]
fn test_manhattan_many_chroms() {
    let mut rows = Vec::new();
    for chr in 1..=22 {
        for pos in (0..5).map(|i| i * 10000) {
            rows.push(vec![
                Value::Str(format!("chr{chr}")),
                Value::Int(pos),
                Value::Float(10.0f64.powf(-(pos as f64 / 10000.0 + 1.0))),
            ]);
        }
    }
    let table = make_table(vec!["chrom", "pos", "pvalue"], rows);
    let result = call_bio_plots_builtin("manhattan", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

#[test]
fn test_manhattan_with_title() {
    let table = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Int(1000),
                Value::Float(0.001),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(5000),
                Value::Float(0.05),
            ],
        ],
    );
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("title", Value::Str("GWAS Results".into())),
    ]);
    let r = call_bio_plots_builtin("manhattan", vec![table, opts]).unwrap();
    if let Value::Str(s) = r {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected SVG");
    }
}

// ── QQ plot advanced tests ────────────────────────────────────

#[test]
fn test_qq_plot_many_values() {
    let pvalues = Value::List(
        (1..=100)
            .map(|i| Value::Float(i as f64 / 100.0))
            .collect::<Vec<_>>()
            .into(),
    );
    let result = call_bio_plots_builtin("qq_plot", vec![pvalues]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

#[test]
fn test_qq_plot_extreme_values() {
    let pvalues = Value::List(
        (vec![
            Value::Float(1e-50),
            Value::Float(1e-20),
            Value::Float(1e-10),
            Value::Float(0.5),
            Value::Float(0.99),
        ])
        .into(),
    );
    let result = call_bio_plots_builtin("qq_plot", vec![pvalues]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Violin plot advanced tests ────────────────────────────────

#[test]
fn test_violin_grouped() {
    let table = make_table(
        vec!["group", "value"],
        vec![
            vec![Value::Str("A".into()), Value::Float(1.0)],
            vec![Value::Str("A".into()), Value::Float(2.0)],
            vec![Value::Str("A".into()), Value::Float(3.0)],
            vec![Value::Str("B".into()), Value::Float(5.0)],
            vec![Value::Str("B".into()), Value::Float(6.0)],
            vec![Value::Str("B".into()), Value::Float(7.0)],
        ],
    );
    let result = call_bio_plots_builtin("violin", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Density plot advanced tests ───────────────────────────────

#[test]
fn test_density_many_values() {
    let values = Value::List(
        (0..200)
            .map(|i| Value::Float((i as f64 * 0.1).sin()))
            .collect::<Vec<_>>()
            .into(),
    );
    let result = call_bio_plots_builtin("density", vec![values]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Kaplan-Meier advanced tests ───────────────────────────────

#[test]
fn test_kaplan_meier_all_events() {
    let table = make_table(
        vec!["time", "event"],
        vec![
            vec![Value::Float(1.0), Value::Int(1)],
            vec![Value::Float(2.0), Value::Int(1)],
            vec![Value::Float(3.0), Value::Int(1)],
            vec![Value::Float(5.0), Value::Int(1)],
        ],
    );
    let result = call_bio_plots_builtin("kaplan_meier", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

#[test]
fn test_kaplan_meier_with_censoring() {
    let table = make_table(
        vec!["time", "event"],
        vec![
            vec![Value::Float(1.0), Value::Int(1)],
            vec![Value::Float(2.0), Value::Int(0)],
            vec![Value::Float(4.0), Value::Int(1)],
            vec![Value::Float(6.0), Value::Int(0)],
            vec![Value::Float(8.0), Value::Int(1)],
        ],
    );
    let result = call_bio_plots_builtin("kaplan_meier", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── ROC curve advanced tests ──────────────────────────────────

#[test]
fn test_roc_curve_many_points() {
    let mut rows = Vec::new();
    for i in 0..100 {
        let score = i as f64 / 100.0;
        let label = if i > 50 { 1 } else { 0 };
        rows.push(vec![Value::Float(score), Value::Int(label)]);
    }
    let table = make_table(vec!["score", "label"], rows);
    let result = call_bio_plots_builtin("roc_curve", vec![table, svg_opts()]).unwrap();
    assert_svg(&result);
}

// ── Forest plot advanced tests ────────────────────────────────

#[test]
fn test_forest_plot_many_studies() {
    let mut rows = Vec::new();
    for i in 0..10 {
        let estimate = 1.0 + i as f64 * 0.2;
        let lower = estimate - 0.3;
        let upper = estimate + 0.3;
        rows.push(vec![
            Value::Str(format!("Study {i}")),
            Value::Float(estimate),
            Value::Float(lower),
            Value::Float(upper),
        ]);
    }
    let table = make_table(vec!["label", "estimate", "lower", "upper"], rows);
    let result = call_bio_plots_builtin("forest_plot", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Sequence logo advanced tests ──────────────────────────────

#[test]
fn test_sequence_logo_protein() {
    let seqs = Value::List(
        (vec![
            Value::Str("MVLSPA".into()),
            Value::Str("MVLSAA".into()),
            Value::Str("MVLSGA".into()),
        ])
        .into(),
    );
    let result = call_bio_plots_builtin("sequence_logo", vec![seqs]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

#[test]
fn test_sequence_logo_many_sequences() {
    let seqs = Value::List(
        (0..20)
            .map(|_| Value::Str("ATCGATCG".into()))
            .collect::<Vec<_>>()
            .into(),
    );
    let result = call_bio_plots_builtin("sequence_logo", vec![seqs]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Circos advanced tests ─────────────────────────────────────

#[test]
fn test_circos_multiple_chromosomes() {
    let table = make_table(
        vec!["chrom", "end"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(249e6)],
            vec![Value::Str("chr2".into()), Value::Float(243e6)],
            vec![Value::Str("chr3".into()), Value::Float(198e6)],
            vec![Value::Str("chrX".into()), Value::Float(155e6)],
        ],
    );
    let result = call_bio_plots_builtin("circos", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── PCA plot advanced tests ───────────────────────────────────

#[test]
fn test_pca_plot_with_group() {
    let table = make_table(
        vec!["PC1", "PC2", "group"],
        vec![
            vec![
                Value::Float(1.0),
                Value::Float(2.0),
                Value::Str("control".into()),
            ],
            vec![
                Value::Float(3.0),
                Value::Float(1.0),
                Value::Str("treatment".into()),
            ],
            vec![
                Value::Float(2.0),
                Value::Float(3.0),
                Value::Str("control".into()),
            ],
            vec![
                Value::Float(4.0),
                Value::Float(0.5),
                Value::Str("treatment".into()),
            ],
        ],
    );
    let result = call_bio_plots_builtin("pca_plot", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Lollipop plot advanced tests ──────────────────────────────

#[test]
fn test_lollipop_many_positions() {
    let mut rows = Vec::new();
    for i in 0..50 {
        rows.push(vec![
            Value::Float(i as f64 * 10.0),
            Value::Float((i % 5 + 1) as f64),
        ]);
    }
    let table = make_table(vec!["position", "count"], rows);
    let result = call_bio_plots_builtin("lollipop", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Phylo tree advanced tests ─────────────────────────────────

#[test]
fn test_phylo_tree_complex_newick() {
    let newick = Value::Str("((A:0.1,B:0.2):0.3,(C:0.4,(D:0.5,E:0.6):0.7):0.8);".into());
    let result = call_bio_plots_builtin("phylo_tree", vec![newick]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

#[test]
fn test_phylo_tree_svg_complex() {
    let newick = Value::Str("((human:0.1,chimp:0.15):0.05,(mouse:0.3,rat:0.25):0.2);".into());
    let result = call_bio_plots_builtin("phylo_tree", vec![newick, svg_opts()]).unwrap();
    assert_svg(&result);
}

// ── Venn advanced tests ───────────────────────────────────────

#[test]
fn test_venn_three_sets() {
    let sets = Value::Record(
        (HashMap::from([
            (
                "A".into(),
                Value::List((vec![Value::Int(1), Value::Int(2), Value::Int(3)]).into()),
            ),
            (
                "B".into(),
                Value::List((vec![Value::Int(2), Value::Int(3), Value::Int(4)]).into()),
            ),
            (
                "C".into(),
                Value::List((vec![Value::Int(3), Value::Int(4), Value::Int(5)]).into()),
            ),
        ]))
        .into(),
    );
    let result = call_bio_plots_builtin("venn", vec![sets]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

#[test]
fn test_venn_three_sets_svg() {
    let sets = Value::Record(
        (HashMap::from([
            (
                "SetA".into(),
                Value::List((vec![Value::Int(1), Value::Int(2)]).into()),
            ),
            (
                "SetB".into(),
                Value::List((vec![Value::Int(2), Value::Int(3)]).into()),
            ),
            (
                "SetC".into(),
                Value::List((vec![Value::Int(3), Value::Int(4)]).into()),
            ),
        ]))
        .into(),
    );
    let result = call_bio_plots_builtin("venn", vec![sets, svg_opts()]).unwrap();
    assert_svg(&result);
}

// ── UpSet plot advanced tests ─────────────────────────────────

#[test]
fn test_upset_three_sets() {
    let sets = Value::Record(
        (HashMap::from([
            (
                "A".into(),
                Value::List((vec![Value::Int(1), Value::Int(2), Value::Int(3)]).into()),
            ),
            (
                "B".into(),
                Value::List((vec![Value::Int(2), Value::Int(3), Value::Int(4)]).into()),
            ),
            (
                "C".into(),
                Value::List((vec![Value::Int(1), Value::Int(3), Value::Int(5)]).into()),
            ),
        ]))
        .into(),
    );
    let result = call_bio_plots_builtin("upset", vec![sets]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── CNV plot advanced tests ───────────────────────────────────

#[test]
fn test_cnv_plot_multi_chrom() {
    let table = make_table(
        vec!["chrom", "start", "end", "log2ratio"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Int(0),
                Value::Int(1000000),
                Value::Float(0.5),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(1000000),
                Value::Int(2000000),
                Value::Float(-0.3),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Int(0),
                Value::Int(500000),
                Value::Float(1.2),
            ],
            vec![
                Value::Str("chr2".into()),
                Value::Int(500000),
                Value::Int(1500000),
                Value::Float(-0.8),
            ],
        ],
    );
    let result = call_bio_plots_builtin("cnv_plot", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Sashimi plot advanced tests ───────────────────────────────

#[test]
fn test_sashimi_multi_junctions() {
    let table = make_table(
        vec!["chrom", "start", "end", "junctions"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Int(1000),
                Value::Int(2000),
                Value::Str("1200-1800:50,1500-1900:30".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(2500),
                Value::Int(3500),
                Value::Str("2700-3300:20".into()),
            ],
        ],
    );
    let result = call_bio_plots_builtin("sashimi", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── HiC map advanced tests ───────────────────────────────────

#[test]
fn test_hic_map_large_matrix() {
    let data: Vec<f64> = (0..100)
        .map(|i| {
            let r = i / 10;
            let c = i % 10;
            1.0 / ((r as f64 - c as f64).abs() + 1.0)
        })
        .collect();
    let mat = Matrix::new(data, 10, 10).unwrap();
    let result = call_bio_plots_builtin("hic_map", vec![Value::Matrix(mat.into())]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Ideogram advanced tests ──────────────────────────────────

#[test]
fn test_ideogram_multiple_bands() {
    let table = make_table(
        vec!["chrom", "start", "end", "stain"],
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Int(0),
                Value::Int(2300000),
                Value::Str("gneg".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(2300000),
                Value::Int(5400000),
                Value::Str("gpos25".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(5400000),
                Value::Int(7200000),
                Value::Str("gneg".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(7200000),
                Value::Int(12700000),
                Value::Str("gpos75".into()),
            ],
            vec![
                Value::Str("chr1".into()),
                Value::Int(12700000),
                Value::Int(16200000),
                Value::Str("acen".into()),
            ],
        ],
    );
    let result = call_bio_plots_builtin("ideogram", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Clustered heatmap advanced tests ──────────────────────────

#[test]
fn test_clustered_heatmap_large() {
    let data: Vec<f64> = (0..64).map(|i| (i as f64 * 0.5).sin()).collect();
    let mat = Matrix::new(data, 8, 8).unwrap();
    let result =
        call_bio_plots_builtin("clustered_heatmap", vec![Value::Matrix(mat.into())]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Oncoprint advanced tests ─────────────────────────────────

#[test]
fn test_oncoprint_multi_gene_sample() {
    let table = make_table(
        vec!["gene", "sample", "type"],
        vec![
            vec![
                Value::Str("TP53".into()),
                Value::Str("S1".into()),
                Value::Str("missense".into()),
            ],
            vec![
                Value::Str("TP53".into()),
                Value::Str("S2".into()),
                Value::Str("frameshift".into()),
            ],
            vec![
                Value::Str("BRCA1".into()),
                Value::Str("S1".into()),
                Value::Str("nonsense".into()),
            ],
            vec![
                Value::Str("BRCA1".into()),
                Value::Str("S3".into()),
                Value::Str("missense".into()),
            ],
            vec![
                Value::Str("EGFR".into()),
                Value::Str("S2".into()),
                Value::Str("amplification".into()),
            ],
        ],
    );
    let result = call_bio_plots_builtin("oncoprint", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Rainfall plot advanced tests ──────────────────────────────

#[test]
fn test_rainfall_many_variants() {
    let mut rows = Vec::new();
    for i in 0..100 {
        rows.push(vec![
            Value::Str("chr1".into()),
            Value::Int(i * 1000 + (i * 37 % 500)), // semi-random positions
        ]);
    }
    let table = make_table(vec!["chrom", "pos"], rows);
    let result = call_bio_plots_builtin("rainfall", vec![table]).unwrap();
    assert!(
        matches!(result, Value::Str(_)),
        "expected Str output, got {result:?}"
    );
}

// ── Axis tick labels ────────────────────────────────────────────
//
// A fixed one-decimal tick format collides whenever the axis range is small.
// The scree plot is where it showed: variance ratios spanning 0..0.4 drew
// "0.1" twice, which a reader reads as a broken figure rather than as two
// distinct gridlines.

/// Every tick label an axis draws must be distinguishable from its neighbours.
fn tick_labels_of(svg: &str) -> Vec<String> {
    // Axis labels are the 11px texts; titles and axis names use other sizes.
    svg.split("font-size=\"11\"")
        .skip(1)
        .filter_map(|chunk| {
            let start = chunk.find('>')? + 1;
            let end = chunk[start..].find("</text>")? + start;
            Some(chunk[start..end].to_string())
        })
        .collect()
}

#[test]
fn small_ranges_do_not_repeat_a_tick_label() {
    // Variance-explained ratios: the exact shape that produced a duplicate.
    let values: Vec<Value> = [0.355, 0.299, 0.211, 0.021, 0.018, 0.012, 0.004]
        .iter()
        .map(|v| Value::Float(*v))
        .collect();
    let svg = match call_bio_plots_builtin("elbow_plot", vec![Value::List(values.into())]) {
        Ok(Value::Str(s)) => s,
        other => panic!("elbow_plot returned {other:?}"),
    };

    let labels = tick_labels_of(&svg);
    assert!(!labels.is_empty(), "no tick labels found in the figure");

    let mut seen = labels.clone();
    seen.sort();
    seen.dedup();
    assert_eq!(
        seen.len(),
        labels.len(),
        "an axis drew the same label twice: {labels:?}"
    );
}

#[test]
fn whole_number_axes_do_not_carry_a_pointless_decimal() {
    // The other half of the same rule: asking for the fewest decimals that
    // separate the ticks means an integer axis gets none at all.
    let values: Vec<Value> = (0..40).map(|i| Value::Float(f64::from(i) * 5.0)).collect();
    let svg = match call_bio_plots_builtin("elbow_plot", vec![Value::List(values.into())]) {
        Ok(Value::Str(s)) => s,
        other => panic!("elbow_plot returned {other:?}"),
    };
    let labels = tick_labels_of(&svg);
    assert!(
        labels.iter().any(|l| !l.contains('.')),
        "expected undecorated integer ticks, got {labels:?}"
    );
}

// ── manhattan: thinning ─────────────────────────────────────────
//
// Thinning drops variants. A figure that does that without saying so is a
// figure a reader will misread, so these pin the disclosure as tightly as the
// arithmetic.

fn crowded_gwas(n: usize) -> Value {
    // The axis auto-fits its data, so merely bunching the positions up proves
    // nothing -- the scale would spread them back out across the panel. Real
    // collisions need more variants than the panel has pixels, so these cycle
    // through 50 positions at one p-value: whatever the scale does, 50 columns
    // have to share it. The one genome-wide hit is planted at a known index.
    let rows: Vec<Vec<Value>> = (0..n)
        .map(|i| {
            let p = if i == 7 { 1e-30 } else { 0.5 };
            vec![
                Value::Str("chr1".into()),
                Value::Float(1000.0 + (i % 50) as f64),
                Value::Float(p),
            ]
        })
        .collect();
    make_table(vec!["chrom", "pos", "pvalue"], rows)
}

fn svg_of(value: &Value) -> String {
    match value {
        Value::Str(s) => s.clone(),
        other => panic!("expected SVG, got {:?}", other.type_of()),
    }
}

#[test]
fn manhattan_does_not_thin_unless_asked() {
    let plot = call_bio_plots_builtin("manhattan", vec![crowded_gwas(5000), svg_opts()]).unwrap();
    let svg = svg_of(&plot);
    assert!(
        !svg.contains("thinned"),
        "a plot nobody asked to thin must not announce thinning"
    );
    assert_eq!(
        svg.matches("<circle").count(),
        5000,
        "every variant should still be drawn by default"
    );
}

#[test]
fn manhattan_thinning_says_so_in_the_figure_and_the_description() {
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("thin", Value::Bool(true)),
        ("raster", Value::Str("off".into())),
    ]);
    let svg = svg_of(&call_bio_plots_builtin("manhattan", vec![crowded_gwas(5000), opts]).unwrap());

    let drawn = svg.matches("<circle").count();
    assert!(drawn < 5000, "thinning should have removed something");
    assert!(drawn > 0, "thinning should not empty the plot");

    // Both the visible note and the accessible description carry the counts,
    // so the disclosure survives being read by eye or by screen reader.
    assert!(
        svg.contains(&format!("thinned: {drawn} of 5000 variants drawn")),
        "the visible note should give both counts; got:\n{svg}"
    );
    assert!(
        svg.contains("<desc>Manhattan plot, thinned to one variant per pixel"),
        "the description should record the thinning"
    );
    assert!(
        svg.contains("Point density does not indicate variant count."),
        "the description should warn that density is no longer meaningful"
    );
}

#[test]
fn manhattan_thinning_keeps_the_genome_wide_hit() {
    // The one thing that must never be thinned away. Its y position is far
    // above every other point, so it owns its pixel row outright -- but only
    // if the survivor is chosen by significance rather than by input order.
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("thin", Value::Bool(true)),
        ("raster", Value::Str("off".into())),
    ]);
    let plain =
        svg_of(&call_bio_plots_builtin("manhattan", vec![crowded_gwas(5000), svg_opts()]).unwrap());
    let thinned =
        svg_of(&call_bio_plots_builtin("manhattan", vec![crowded_gwas(5000), opts]).unwrap());

    let highest = |svg: &str| {
        svg.match_indices("<circle")
            .filter_map(|(at, _)| {
                let tag = &svg[at..svg[at..].find("/>").map(|e| at + e).unwrap_or(svg.len())];
                let cy = tag.find("cy=\"")? + 4;
                let end = tag[cy..].find('"')? + cy;
                tag[cy..end].parse::<f64>().ok()
            })
            .fold(f64::MAX, f64::min)
    };
    // Smaller y is higher on the page.
    assert!(
        (highest(&plain) - highest(&thinned)).abs() < 0.05,
        "the top of the plot moved: {} -> {}",
        highest(&plain),
        highest(&thinned)
    );
}

#[test]
fn manhattan_rejects_a_thin_option_it_cannot_read() {
    let opts = make_opts(vec![
        ("format", Value::Str("svg".into())),
        ("thin", Value::Int(3)),
    ]);
    let err = call_bio_plots_builtin("manhattan", vec![crowded_gwas(50), opts]).unwrap_err();
    assert!(
        format!("{err}").contains("'thin' must be"),
        "expected a message naming the option; got {err}"
    );
}
