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
            .collect(),
    )
}

fn svg_opts() -> Value {
    make_opts(vec![("format", Value::Str("svg".into()))])
}

fn assert_svg(val: &Value) {
    if let Value::Str(s) = val {
        assert!(s.contains("<svg"), "output should contain <svg tag");
    } else {
        panic!("expected Value::Str with SVG content, got {:?}", val.type_of());
    }
}

// ── 1. manhattan ────────────────────────────────────────────────

#[test]
fn test_manhattan_ascii() {
    let t = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(1000.0), Value::Float(0.001)],
            vec![Value::Str("chr1".into()), Value::Float(2000.0), Value::Float(0.05)],
            vec![Value::Str("chr2".into()), Value::Float(500.0), Value::Float(1e-8)],
        ],
    );
    let r = call_bio_plots_builtin("manhattan", vec![t]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_manhattan_svg() {
    let t = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(1000.0), Value::Float(0.001)],
            vec![Value::Str("chr2".into()), Value::Float(500.0), Value::Float(1e-8)],
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
            vec![Value::Str("chr1".into()), Value::Float(100.0), Value::Float(0.01)],
            vec![Value::Str("chr1".into()), Value::Float(200.0), Value::Float(0.001)],
            vec![Value::Str("chr1".into()), Value::Float(300.0), Value::Float(1e-9)],
        ],
    );
    let r = call_bio_plots_builtin("manhattan", vec![t]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_manhattan_single_chromosome_svg() {
    let t = make_table(
        vec!["chrom", "pos", "pvalue"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(100.0), Value::Float(0.01)],
            vec![Value::Str("chr1".into()), Value::Float(200.0), Value::Float(1e-9)],
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
    let vals = Value::List(vec![
        Value::Float(0.001),
        Value::Float(0.01),
        Value::Float(0.1),
        Value::Float(0.5),
    ]);
    let r = call_bio_plots_builtin("qq_plot", vec![vals]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_qq_plot_svg() {
    let vals = Value::List(vec![
        Value::Float(0.001),
        Value::Float(0.01),
        Value::Float(0.1),
        Value::Float(0.5),
    ]);
    let r = call_bio_plots_builtin("qq_plot", vec![vals, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_qq_plot_all_same_pvalues() {
    let vals = Value::List(vec![
        Value::Float(0.5),
        Value::Float(0.5),
        Value::Float(0.5),
    ]);
    let r = call_bio_plots_builtin("qq_plot", vec![vals]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_qq_plot_wrong_type() {
    let r = call_bio_plots_builtin("qq_plot", vec![Value::Str("bad".into())]);
    assert!(r.is_err());
}

#[test]
fn test_qq_plot_empty_after_filter() {
    // All zero / negative p-values get filtered out
    let vals = Value::List(vec![Value::Float(0.0), Value::Float(-1.0)]);
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
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_ideogram_svg() {
    let t = make_table(
        vec!["chrom", "start", "end"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(0.0), Value::Float(1e6)],
            vec![Value::Str("chr2".into()), Value::Float(0.0), Value::Float(2e6)],
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
    assert_eq!(r, Value::Nil);
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
    // Returns Nil with "insufficient data" message
    let r = call_bio_plots_builtin("rainfall", vec![t]).unwrap();
    assert_eq!(r, Value::Nil);
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
            vec![Value::Str("chr1".into()), Value::Float(0.0), Value::Float(1e6), Value::Float(0.5)],
            vec![Value::Str("chr1".into()), Value::Float(1e6), Value::Float(2e6), Value::Float(-0.3)],
        ],
    );
    let r = call_bio_plots_builtin("cnv_plot", vec![t]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_cnv_plot_svg() {
    let t = make_table(
        vec!["chrom", "start", "end", "log2ratio"],
        vec![
            vec![Value::Str("chr1".into()), Value::Float(0.0), Value::Float(1e6), Value::Float(0.5)],
            vec![Value::Str("chr1".into()), Value::Float(1e6), Value::Float(2e6), Value::Float(-0.3)],
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
    assert_eq!(r, Value::Nil);
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
    let r = call_bio_plots_builtin("violin", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_violin_single_value() {
    let vals = Value::List(vec![Value::Float(42.0)]);
    let r = call_bio_plots_builtin("violin", vec![vals]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_violin_single_value_svg() {
    let vals = Value::List(vec![Value::Float(42.0)]);
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
    let vals = Value::List(vec![
        Value::Float(1.0),
        Value::Float(2.0),
        Value::Float(3.0),
        Value::Float(4.0),
        Value::Float(5.0),
    ]);
    let r = call_bio_plots_builtin("density", vec![vals]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_density_svg() {
    let vals = Value::List(vec![
        Value::Float(1.0),
        Value::Float(2.0),
        Value::Float(3.0),
        Value::Float(4.0),
    ]);
    let r = call_bio_plots_builtin("density", vec![vals, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_density_two_values_minimum() {
    let vals = Value::List(vec![Value::Float(1.0), Value::Float(2.0)]);
    let r = call_bio_plots_builtin("density", vec![vals]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_density_two_values_svg() {
    let vals = Value::List(vec![Value::Float(1.0), Value::Float(2.0)]);
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
    assert_eq!(r, Value::Nil);
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
    let r = call_bio_plots_builtin("kaplan_meier", vec![Value::List(vec![])]);
    assert!(r.is_err());
}

// ── 9. forest_plot ──────────────────────────────────────────────

#[test]
fn test_forest_plot_ascii() {
    let t = make_table(
        vec!["label", "estimate", "lower", "upper"],
        vec![
            vec![Value::Str("Study A".into()), Value::Float(1.5), Value::Float(0.8), Value::Float(2.2)],
            vec![Value::Str("Study B".into()), Value::Float(0.9), Value::Float(0.5), Value::Float(1.3)],
        ],
    );
    let r = call_bio_plots_builtin("forest_plot", vec![t]).unwrap();
    assert_eq!(r, Value::Nil);
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
    assert_eq!(r, Value::Nil);
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
        assert!(s.contains("AUC = 1.000"), "perfect classifier should have AUC=1, got {}", s);
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
    let m = Value::Matrix(Matrix {
        data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        nrow: 2,
        ncol: 3,
        row_names: Some(vec!["r1".into(), "r2".into()]),
        col_names: Some(vec!["c1".into(), "c2".into(), "c3".into()]),
    });
    let r = call_bio_plots_builtin("clustered_heatmap", vec![m]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_clustered_heatmap_svg() {
    let m = Value::Matrix(Matrix {
        data: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        nrow: 2,
        ncol: 3,
        row_names: Some(vec!["r1".into(), "r2".into()]),
        col_names: Some(vec!["c1".into(), "c2".into(), "c3".into()]),
    });
    let r = call_bio_plots_builtin("clustered_heatmap", vec![m, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_clustered_heatmap_wrong_type() {
    let r = call_bio_plots_builtin("clustered_heatmap", vec![Value::Int(0)]);
    assert!(r.is_err());
}

// ── 12. pca_plot ────────────────────────────────────────────────

#[test]
fn test_pca_plot_ascii() {
    let t = make_table(
        vec!["PC1", "PC2", "label"],
        vec![
            vec![Value::Float(1.0), Value::Float(2.0), Value::Str("A".into())],
            vec![Value::Float(-1.0), Value::Float(-0.5), Value::Str("B".into())],
            vec![Value::Float(0.5), Value::Float(1.0), Value::Str("A".into())],
        ],
    );
    let r = call_bio_plots_builtin("pca_plot", vec![t]).unwrap();
    assert_eq!(r, Value::Nil);
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
    let r = call_bio_plots_builtin("pca_plot", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
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
            vec![Value::Str("TP53".into()), Value::Str("S1".into()), Value::Str("missense".into())],
            vec![Value::Str("TP53".into()), Value::Str("S2".into()), Value::Str("nonsense".into())],
            vec![Value::Str("BRCA1".into()), Value::Str("S1".into()), Value::Str("deletion".into())],
        ],
    );
    let r = call_bio_plots_builtin("oncoprint", vec![t]).unwrap();
    assert_eq!(r, Value::Nil);
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
    let r = call_bio_plots_builtin("oncoprint", vec![t, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_oncoprint_wrong_type() {
    let r = call_bio_plots_builtin("oncoprint", vec![Value::Float(1.0)]);
    assert!(r.is_err());
}

// ── 14. venn ────────────────────────────────────────────────────

#[test]
fn test_venn_ascii() {
    let rec = Value::Record(HashMap::from([
        ("A".to_string(), Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
        ("B".to_string(), Value::List(vec![Value::Int(2), Value::Int(3), Value::Int(4)])),
    ]));
    let r = call_bio_plots_builtin("venn", vec![rec]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_venn_svg() {
    let rec = Value::Record(HashMap::from([
        ("A".to_string(), Value::List(vec![Value::Int(1), Value::Int(2)])),
        ("B".to_string(), Value::List(vec![Value::Int(2), Value::Int(3)])),
    ]));
    let r = call_bio_plots_builtin("venn", vec![rec, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_venn_completely_overlapping() {
    let rec = Value::Record(HashMap::from([
        ("A".to_string(), Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
        ("B".to_string(), Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
    ]));
    let r = call_bio_plots_builtin("venn", vec![rec]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_venn_disjoint_sets() {
    let rec = Value::Record(HashMap::from([
        ("A".to_string(), Value::List(vec![Value::Int(1), Value::Int(2)])),
        ("B".to_string(), Value::List(vec![Value::Int(3), Value::Int(4)])),
    ]));
    let r = call_bio_plots_builtin("venn", vec![rec]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_venn_disjoint_svg() {
    let rec = Value::Record(HashMap::from([
        ("X".to_string(), Value::List(vec![Value::Int(10)])),
        ("Y".to_string(), Value::List(vec![Value::Int(20)])),
    ]));
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
    let rec = Value::Record(HashMap::from([(
        "A".to_string(),
        Value::List(vec![Value::Int(1)]),
    )]));
    let r = call_bio_plots_builtin("venn", vec![rec]);
    assert!(r.is_err());
}

// ── 15. upset ───────────────────────────────────────────────────

#[test]
fn test_upset_ascii() {
    let rec = Value::Record(HashMap::from([
        ("A".to_string(), Value::List(vec![Value::Int(1), Value::Int(2), Value::Int(3)])),
        ("B".to_string(), Value::List(vec![Value::Int(2), Value::Int(3), Value::Int(4)])),
        ("C".to_string(), Value::List(vec![Value::Int(3), Value::Int(5)])),
    ]));
    let r = call_bio_plots_builtin("upset", vec![rec]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_upset_svg() {
    let rec = Value::Record(HashMap::from([
        ("A".to_string(), Value::List(vec![Value::Int(1), Value::Int(2)])),
        ("B".to_string(), Value::List(vec![Value::Int(2), Value::Int(3)])),
    ]));
    let r = call_bio_plots_builtin("upset", vec![rec, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_upset_wrong_type() {
    let r = call_bio_plots_builtin("upset", vec![Value::Int(0)]);
    assert!(r.is_err());
}

#[test]
fn test_upset_too_few_sets() {
    let rec = Value::Record(HashMap::from([(
        "A".to_string(),
        Value::List(vec![Value::Int(1)]),
    )]));
    let r = call_bio_plots_builtin("upset", vec![rec]);
    assert!(r.is_err());
}

// ── 16. sequence_logo ───────────────────────────────────────────

#[test]
fn test_sequence_logo_ascii() {
    let seqs = Value::List(vec![
        Value::Str("ACGT".into()),
        Value::Str("ACGT".into()),
        Value::Str("ACGA".into()),
    ]);
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_sequence_logo_svg() {
    let seqs = Value::List(vec![
        Value::Str("ACGT".into()),
        Value::Str("ACGT".into()),
    ]);
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_sequence_logo_single_sequence() {
    let seqs = Value::List(vec![Value::Str("ATCG".into())]);
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_sequence_logo_single_sequence_svg() {
    let seqs = Value::List(vec![Value::Str("ATCGATCG".into())]);
    let r = call_bio_plots_builtin("sequence_logo", vec![seqs, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_sequence_logo_empty_list() {
    let seqs = Value::List(vec![]);
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
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_phylo_tree_svg() {
    let newick = Value::Str("((A:0.1,B:0.2):0.3,C:0.4);".into());
    let r = call_bio_plots_builtin("phylo_tree", vec![newick, svg_opts()]).unwrap();
    assert_svg(&r);
}

#[test]
fn test_phylo_tree_wrong_type() {
    let r = call_bio_plots_builtin("phylo_tree", vec![Value::Int(0)]);
    assert!(r.is_err());
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
    assert_eq!(r, Value::Nil);
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
    assert_eq!(r, Value::Nil);
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
    let m = Value::Matrix(Matrix {
        data: vec![10.0, 5.0, 1.0, 5.0, 8.0, 3.0, 1.0, 3.0, 9.0],
        nrow: 3,
        ncol: 3,
        row_names: Some(vec!["bin1".into(), "bin2".into(), "bin3".into()]),
        col_names: Some(vec!["bin1".into(), "bin2".into(), "bin3".into()]),
    });
    let r = call_bio_plots_builtin("hic_map", vec![m]).unwrap();
    assert_eq!(r, Value::Nil);
}

#[test]
fn test_hic_map_svg() {
    let m = Value::Matrix(Matrix {
        data: vec![10.0, 5.0, 5.0, 8.0],
        nrow: 2,
        ncol: 2,
        row_names: None,
        col_names: None,
    });
    let r = call_bio_plots_builtin("hic_map", vec![m, svg_opts()]).unwrap();
    assert_svg(&r);
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
    assert_eq!(r, Value::Nil);
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
        vec![
            vec![Value::Str("chr1".into()), Value::Float(0.0), Value::Float(1e6)],
        ],
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
        vec![
            vec![Value::Str("chr1".into()), Value::Float(0.0), Value::Float(1e6), Value::Float(0.5)],
        ],
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
