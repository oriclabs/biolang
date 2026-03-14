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
    let opts = Value::Record(HashMap::from([("type".into(), Value::Str("line".into()))]));
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
    let opts = Value::Record(HashMap::from([("type".into(), Value::Str("bar".into()))]));
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
    let opts = Value::Record(HashMap::from([("type".into(), Value::Str("box".into()))]));
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
    let opts = Value::Record(HashMap::from([("type".into(), Value::Str("invalid".into()))]));
    let result = call_plot_builtin("plot", vec![table, opts]);
    assert!(result.is_err());
}

// ── Histogram tests ─────────────────────────────────────────────

#[test]
fn test_histogram_returns_svg() {
    let list = Value::List(vec![
        Value::Float(1.0),
        Value::Float(2.0),
        Value::Float(3.0),
        Value::Float(4.0),
        Value::Float(5.0),
        Value::Float(3.0),
    ]);
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
    let list = Value::List(vec![Value::Float(5.0)]);
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
    let list = Value::List(vec![
        Value::Float(3.0),
        Value::Float(3.0),
        Value::Float(3.0),
        Value::Float(3.0),
    ]);
    let result = call_plot_builtin("histogram", vec![list]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_histogram_empty_list_error() {
    let list = Value::List(vec![]);
    let result = call_plot_builtin("histogram", vec![list]);
    assert!(result.is_err());
}

#[test]
fn test_histogram_wrong_type() {
    let result = call_plot_builtin("histogram", vec![Value::Int(42)]);
    assert!(result.is_err());
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
fn test_save_svg_wrong_type_first_arg() {
    let result = call_plot_builtin("save_svg", vec![Value::Int(42), Value::Str("out.svg".into())]);
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
    let opts = Value::Record(HashMap::from([
        ("a".into(), Value::Str("avg_expr".into())),
        ("m".into(), Value::Str("fold_change".into())),
    ]));
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
        vec![
            vec![
                Value::Str("chr1".into()),
                Value::Int(1000),
                Value::Int(2000),
                Value::Str("TP53".into()),
                Value::Str("+".into()),
            ],
        ],
    );
    let opts = Value::Record(HashMap::from([
        ("title".into(), Value::Str("Gene Features".into())),
    ]));
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
        vec![vec![Value::Str("chr1".into()), Value::Int(50), Value::Int(150)]],
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
        vec![vec![Value::Str("chr1".into()), Value::Int(0), Value::Int(1000)]],
    );
    let opts = Value::Record(HashMap::from([
        ("width".into(), Value::Float(1200.0)),
        ("height".into(), Value::Float(400.0)),
    ]));
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
        (0..100).map(|i| Value::Float(i as f64)).collect(),
    );
    let opts = Value::Record(HashMap::from([
        ("bins".into(), Value::Int(5)),
    ]));
    let result = call_plot_builtin("histogram", vec![list, opts]).unwrap();
    if let Value::Str(s) = result {
        assert!(s.contains("<svg"));
        // With 5 bins, should have 5 rects
        let rect_count = s.matches("<rect").count();
        assert!(rect_count >= 5 && rect_count <= 6, "expected ~5 rects for 5 bins, got {rect_count}");
    } else {
        panic!("expected Str");
    }
}

#[test]
fn test_histogram_with_title() {
    let list = Value::List(vec![
        Value::Float(1.0), Value::Float(2.0), Value::Float(3.0),
    ]);
    let opts = Value::Record(HashMap::from([
        ("title".into(), Value::Str("My Histogram".into())),
    ]));
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
    let opts = Value::Record(HashMap::from([
        ("title".into(), Value::Str("Test Plot".into())),
    ]));
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
    let opts = Value::Record(HashMap::from([
        ("width".into(), Value::Float(400.0)),
        ("height".into(), Value::Float(300.0)),
    ]));
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
    let input = Value::Record(HashMap::from([
        ("data".into(), table),
        ("title".into(), Value::Str("From Record".into())),
    ]));
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
    let list = Value::List(vec![
        Value::Float(1.0), Value::Float(2.0), Value::Float(3.0),
        Value::Float(4.0), Value::Float(5.0),
    ]);
    let input = Value::Record(HashMap::from([
        ("values".into(), list),
        ("bins".into(), Value::Int(3)),
    ]));
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
    let opts = Value::Record(HashMap::from([
        ("fc_threshold".into(), Value::Float(2.0)),
        ("p_threshold".into(), Value::Float(0.001)),
    ]));
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
    let opts = Value::Record(HashMap::from([
        ("title".into(), Value::Str("Expression Heatmap".into())),
    ]));
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
    let cols: Vec<&str> = (0..10).map(|_| "c").collect();
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
