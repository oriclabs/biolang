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

#[test]
fn test_unknown_plot_builtin() {
    let result = call_plot_builtin("nonexistent", vec![]);
    assert!(result.is_err());
}
